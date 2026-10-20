use super::*;
use std::time::Duration;
use tokio::{net::UnixListener, time::timeout};

#[test]
fn resizes_require_bounded_nonzero_integer_dimensions() {
    assert_eq!(
        resize_frame(r#"{"cols":120,"rows":40}"#),
        Some(vec![1, 0, 120, 0, 40])
    );
    for text in [
        "broken",
        "null",
        "{}",
        r#"{"cols":0,"rows":24}"#,
        r#"{"cols":80,"rows":1001}"#,
        r#"{"cols":65536,"rows":24}"#,
        r#"{"cols":80.5,"rows":24}"#,
    ] {
        assert_eq!(resize_frame(text), None, "{text}");
    }
}

async fn frame(socket: &mut UnixStream, bytes: &[u8]) {
    socket.write_u32(bytes.len() as u32).await.unwrap();
    socket.write_all(bytes).await.unwrap();
}

async fn command(socket: &mut UnixStream, text: &str) {
    let mut bytes = vec![0];
    bytes.extend_from_slice(text.as_bytes());
    frame(socket, &bytes).await;
}

async fn until(socket: &mut UnixStream, marker: &str) -> String {
    let mut output = Vec::new();
    timeout(Duration::from_secs(10), async {
        let mut bytes = [0; 8192];
        loop {
            let length = socket.read(&mut bytes).await.unwrap();
            assert_ne!(
                length,
                0,
                "closed before {marker}: {}",
                String::from_utf8_lossy(&output)
            );
            output.extend_from_slice(&bytes[..length]);
            let text = String::from_utf8_lossy(&output);
            if text.contains(marker) {
                return text.into_owned();
            }
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "interactive shell timed out waiting for {marker:?}: {}",
            String::from_utf8_lossy(&output)
        )
    })
}

#[tokio::test]
async fn a_real_controlling_terminal_handles_input_resize_passwords_interrupts_and_exit() {
    let (pty, shell) = spawn_shell().unwrap();
    let (worker, browser) = std::os::unix::net::UnixStream::pair().unwrap();
    browser.set_nonblocking(true).unwrap();
    let mut browser = UnixStream::from_std(browser).unwrap();
    let task = tokio::spawn(async move { session(pty, worker, shell).await });
    command(
        &mut browser,
        "bind 'set enable-bracketed-paste on'; stty -echo; printf '\\nREADY\\n'\r",
    )
    .await;
    until(&mut browser, "\r\nREADY\r\n").await;
    command(
        &mut browser,
        "test -t 0 && test -t 1 && test -t 2 && printf '\\nTTY:%s\\n' \"$TERM\"\r",
    )
    .await;
    until(&mut browser, "\r\nTTY:xterm-256color\r\n").await;
    command(&mut browser, "cd /tmp; printf '\\nDIR:%s\\n' \"$PWD\"\r").await;
    until(&mut browser, "\r\nDIR:/tmp\r\n").await;
    frame(
        &mut browser,
        &resize_frame(r#"{"cols":132,"rows":42}"#).unwrap(),
    )
    .await;
    command(&mut browser, "printf '\\n'; stty size\r").await;
    until(&mut browser, "\r\n42 132\r\n").await;
    command(&mut browser, "stty echo; read -s -p $'\\nPassword:' secret; stty -echo; printf '\\nACCEPTED:%s\\n' \"${#secret}\"\r").await;
    until(&mut browser, "\r\nPassword:").await;
    command(&mut browser, "never-show-this\r").await;
    let output = until(&mut browser, "\r\nACCEPTED:15\r\n").await;
    assert!(!output.contains("never-show-this"));
    command(
        &mut browser,
        "PS1=$'\\nINTERRUPT_READY\\n'; PROMPT_COMMAND=; bash -c 'trap \"exit 130\" INT; printf \"\\nWAITING_INPUT\\n\"; read -r -t 30'\r",
    )
    .await;
    until(&mut browser, "\r\nWAITING_INPUT\r\n").await;
    command(&mut browser, "\u{3}").await;
    until(&mut browser, "\r\nINTERRUPT_READY\r\n").await;
    command(&mut browser, "printf '\\nINTERRUPTED\\n'\r").await;
    until(&mut browser, "\r\nINTERRUPTED\r\n").await;
    command(&mut browser, "exit\r").await;
    let result = timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_ok() || result.unwrap_err().raw_os_error() == Some(nix::libc::EIO));
}

#[tokio::test]
async fn closing_the_connection_ends_the_shell_and_malformed_frames_are_refused() {
    for invalid in [vec![2], vec![1, 0], vec![], vec![0; MAX_INPUT + 2]] {
        let (pty, shell) = spawn_shell().unwrap();
        let (worker, browser) = std::os::unix::net::UnixStream::pair().unwrap();
        browser.set_nonblocking(true).unwrap();
        let mut browser = UnixStream::from_std(browser).unwrap();
        let task = tokio::spawn(async move { session(pty, worker, shell).await });
        frame(&mut browser, &invalid).await;
        assert!(timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .is_err());
    }
    let (pty, shell) = spawn_shell().unwrap();
    let (worker, browser) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(browser);
    assert!(session(pty, worker, shell).await.is_err());
}

async fn websocket(shell: std::path::PathBuf) -> (UnixStream, tokio::task::JoinHandle<()>) {
    let dir = crate::tests::temp_dir();
    let path = dir.join("http.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let app = axum::Router::new()
        .route("/session", axum::routing::get(open))
        .with_state(crate::AppState {
            shell,
            launcher: "/no/launcher".into(),
            ui_dir: dir,
        });
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut client = UnixStream::connect(&path).await.unwrap();
    client.write_all(b"GET /session HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").await.unwrap();
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        header.push(client.read_u8().await.unwrap());
    }
    assert!(String::from_utf8_lossy(&header).starts_with("HTTP/1.1 101"));
    (client, server)
}

async fn send_ws(socket: &mut UnixStream, opcode: u8, payload: &[u8]) {
    assert!(payload.len() < 126);
    let mut bytes = vec![0x80 | opcode, 0x80 | payload.len() as u8, 1, 2, 3, 4];
    bytes.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ [1, 2, 3, 4][index % 4]),
    );
    socket.write_all(&bytes).await.unwrap();
}

#[tokio::test]
async fn websocket_transports_raw_input_output_resize_and_closes_the_worker() {
    let dir = crate::tests::temp_dir();
    let path = dir.join("worker.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (mut browser, server) = websocket(path).await;
    let (mut worker, _) = listener.accept().await.unwrap();
    send_ws(&mut browser, 2, b"ls\r\x03").await;
    assert_eq!(worker.read_u32().await.unwrap(), 5);
    let mut input = [0; 5];
    worker.read_exact(&mut input).await.unwrap();
    assert_eq!(&input, b"\0ls\r\x03");
    send_ws(&mut browser, 1, br#"{"cols":100,"rows":30}"#).await;
    assert_eq!(worker.read_u32().await.unwrap(), 5);
    worker.read_exact(&mut input).await.unwrap();
    assert_eq!(input, [1, 0, 100, 0, 30]);
    worker.write_all(b"\x1b[31mhello\r\n").await.unwrap();
    assert_eq!(browser.read_u8().await.unwrap(), 0x82);
    let length = browser.read_u8().await.unwrap();
    let mut output = vec![0; length as usize];
    browser.read_exact(&mut output).await.unwrap();
    assert_eq!(output, b"\x1b[31mhello\r\n");
    send_ws(&mut browser, 9, b"ping").await;
    send_ws(&mut browser, 8, b"").await;
    assert_eq!(
        timeout(Duration::from_secs(3), worker.read_u8())
            .await
            .unwrap()
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
    server.abort();
}

#[tokio::test]
async fn invalid_websocket_resize_and_worker_eof_end_the_connection() {
    for invalid in [true, false] {
        let dir = crate::tests::temp_dir();
        let path = dir.join("worker.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (mut browser, server) = websocket(path).await;
        let (mut worker, _) = listener.accept().await.unwrap();
        if invalid {
            send_ws(&mut browser, 1, b"invalid").await;
            assert_eq!(
                timeout(Duration::from_secs(3), worker.read_u8())
                    .await
                    .unwrap()
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::UnexpectedEof
            );
        } else {
            drop(worker);
            assert_eq!(
                timeout(Duration::from_secs(3), browser.read_u8())
                    .await
                    .unwrap()
                    .unwrap(),
                0x88
            );
        }
        server.abort();
    }
}
