use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{IntoResponse, Response},
};
use nix::{
    fcntl::{fcntl, FcntlArg, FdFlag, OFlag},
    pty::{openpty, Winsize},
    unistd::setsid,
};
use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, BorrowedFd},
        unix::process::CommandExt,
    },
    process::{Command, Stdio},
};
use tokio::{
    io::{unix::AsyncFd, AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

const MAX_INPUT: usize = 65536;
nix::ioctl_write_int_bad!(controlling_terminal, nix::libc::TIOCSCTTY);
nix::ioctl_write_ptr_bad!(set_size, nix::libc::TIOCSWINSZ, Winsize);

pub async fn open(State(state): State<crate::AppState>, upgrade: WebSocketUpgrade) -> Response {
    match UnixStream::connect(&state.shell).await {
        Ok(shell) => upgrade
            .max_message_size(MAX_INPUT)
            .on_upgrade(|socket| bridge(socket, shell))
            .into_response(),
        Err(error) => crate::unavailable(error),
    }
}

async fn bridge(mut socket: WebSocket, shell: UnixStream) {
    let (mut output, mut input) = shell.into_split();
    let mut buffer = [0; 8192];
    loop {
        tokio::select! {
            read = output.read(&mut buffer) => {
                let Ok(length) = read else { break };
                if length == 0 || socket.send(Message::Binary(buffer[..length].to_vec().into())).await.is_err() {
                    break;
                }
            }
            message = socket.recv() => {
                let frame = match message {
                    Some(Ok(Message::Binary(data))) => {
                        let mut frame = vec![0];
                        frame.extend_from_slice(&data);
                        frame
                    }
                    Some(Ok(Message::Text(text))) => match resize_frame(&text) {
                        Some(frame) => frame,
                        None => break,
                    },
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
                    _ => break,
                };

                if input.write_u32(frame.len() as u32).await.is_err() || input.write_all(&frame).await.is_err() {
                    break;
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

fn resize_frame(text: &str) -> Option<Vec<u8>> {
    let size: serde_json::Value = serde_json::from_str(text).ok()?;
    let cols = u16::try_from(size["cols"].as_u64()?).ok()?;
    let rows = u16::try_from(size["rows"].as_u64()?).ok()?;
    if !(1..=1000).contains(&cols) || !(1..=1000).contains(&rows) {
        return None;
    }

    let mut frame = vec![1];
    frame.extend_from_slice(&cols.to_be_bytes());
    frame.extend_from_slice(&rows.to_be_bytes());
    Some(frame)
}

fn spawn_shell() -> io::Result<(File, tokio::process::Child)> {
    let pair = openpty(
        Some(&Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }),
        None,
    )?;

    fcntl(&pair.master, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC))?;
    fcntl(&pair.slave, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC))?;
    let slave = File::from(pair.slave);
    let mut command = Command::new("/bin/bash");
    command
        .args(["--login", "-i"])
        .env("TERM", "xterm-256color")
        .stdin(Stdio::from(slave.try_clone()?))
        .stdout(Stdio::from(slave.try_clone()?))
        .stderr(Stdio::from(slave));

    // SAFETY: the hook runs in the forked child and only calls the async-signal-safe `setsid` and
    // `ioctl`.
    unsafe {
        command.pre_exec(|| {
            setsid()?;
            controlling_terminal(0, 0)?;
            Ok(())
        });
    }
    let child = tokio::process::Command::from(command)
        .kill_on_drop(true)
        .spawn()?;
    fcntl(&pair.master, FcntlArg::F_SETFL(OFlag::O_NONBLOCK))?;
    Ok((File::from(pair.master), child))
}

async fn read_pty(pty: &AsyncFd<File>, buffer: &mut [u8]) -> io::Result<usize> {
    loop {
        let mut ready = pty.readable().await?;
        if let Ok(result) = ready.try_io(|fd| fd.get_ref().read(buffer)) {
            return result;
        }
    }
}

async fn write_pty(pty: &AsyncFd<File>, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        let mut ready = pty.writable().await?;
        if let Ok(result) = ready.try_io(|fd| fd.get_ref().write(bytes)) {
            let length = result?;
            if length == 0 {
                return Err(io::ErrorKind::WriteZero.into());
            }
            bytes = &bytes[length..];
        }
    }
    Ok(())
}

async fn input(pty: &AsyncFd<File>, mut socket: tokio::net::unix::OwnedReadHalf) -> io::Result<()> {
    loop {
        let length = socket.read_u32().await? as usize;
        if !(1..=MAX_INPUT + 1).contains(&length) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut frame = vec![0; length];
        socket.read_exact(&mut frame).await?;
        match frame.as_slice() {
            [0, bytes @ ..] => write_pty(pty, bytes).await?,
            [1, a, b, c, d] => {
                let size = Winsize {
                    ws_col: u16::from_be_bytes([*a, *b]),
                    ws_row: u16::from_be_bytes([*c, *d]),
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                };

                // SAFETY: the descriptor is the open pty and `size` outlives the call.
                unsafe {
                    set_size(pty.get_ref().as_raw_fd(), &size)?;
                }
            }
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
    }
}

async fn output(
    pty: &AsyncFd<File>,
    mut socket: tokio::net::unix::OwnedWriteHalf,
    mut shell: tokio::process::Child,
) -> io::Result<()> {
    let mut buffer = [0; 8192];
    let mut exited = false;
    loop {
        let length = if exited {
            match pty.get_ref().read(&mut buffer) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                result => result?,
            }
        } else {
            tokio::select! {
                result = read_pty(pty, &mut buffer) => result?,
                status = shell.wait() => {
                    status?;
                    exited = true;
                    continue;
                }
            }
        };

        if length == 0 {
            return Ok(());
        }

        socket.write_all(&buffer[..length]).await?;
    }
}

async fn session(
    pty: File,
    socket: std::os::unix::net::UnixStream,
    shell: tokio::process::Child,
) -> io::Result<()> {
    socket.set_nonblocking(true)?;
    let (read, write) = UnixStream::from_std(socket)?.into_split();
    // SAFETY: the `AsyncFd` owns the pty file, so its descriptor stays open until it is dropped.
    let pty = unsafe { AsyncFd::register(pty) }?;
    tokio::select! {
        result = input(&pty, read) => result,
        result = output(&pty, write, shell) => result,
    }
}

pub fn worker() -> io::Result<()> {
    // SAFETY: systemd passes the accepted connection as stdin, open for the whole process.
    let socket = unsafe { BorrowedFd::borrow_raw(0) }.try_clone_to_owned()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    match runtime.block_on(async {
        let (pty, shell) = spawn_shell()?;
        session(pty, socket.into(), shell).await
    }) {
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::UnexpectedEof
                    | io::ErrorKind::BrokenPipe
                    | io::ErrorKind::ConnectionReset
            ) || error.raw_os_error() == Some(nix::libc::EIO) =>
        {
            Ok(())
        }
        result => result,
    }
}

#[cfg(test)]
#[path = "../tests/unit/axum/terminal.rs"]
mod tests;
