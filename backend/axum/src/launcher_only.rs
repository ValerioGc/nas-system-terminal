//! Restricts the app socket to the launcher. Every app shares the group of the sockets folder, so
//! file permissions alone would admit any of them. A connection is accepted only from root, the
//! owner of the sockets folder (the launcher) or the owner of the socket (the app itself).

use axum::serve::Listener;
use std::{io, os::unix::fs::MetadataExt};
use tokio::net::{unix::SocketAddr, UnixListener, UnixStream};

pub struct LauncherOnly {
    listener: UnixListener,
    launcher: Option<u32>,
    own: Option<u32>,
}

impl LauncherOnly {
    pub fn new(listener: UnixListener) -> Self {
        let socket = listener
            .local_addr()
            .ok()
            .and_then(|address| address.as_pathname().map(ToOwned::to_owned));
        let owner =
            |path: &std::path::Path| std::fs::metadata(path).ok().map(|metadata| metadata.uid());
        let launcher = socket
            .as_deref()
            .map(folder_of)
            .and_then(|folder| owner(&folder));
        let own = socket.as_deref().and_then(owner);
        Self {
            listener,
            launcher,
            own,
        }
    }

    fn admits(&self, uid: u32) -> bool {
        uid == 0 || Some(uid) == self.launcher || Some(uid) == self.own
    }
}

/// Returns the folder of a socket path, or `.` for a bare file name.
fn folder_of(socket: &std::path::Path) -> std::path::PathBuf {
    match socket.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder.to_owned(),
        _ => std::path::PathBuf::from("."),
    }
}

impl Listener for LauncherOnly {
    type Io = UnixStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, address) = Listener::accept(&mut self.listener).await;
            match stream.peer_cred() {
                Ok(peer) if self.admits(peer.uid()) => return (stream, address),
                Ok(peer) => eprintln!(
                    "<4>refused a connection of the user {}: only the launcher speaks here",
                    peer.uid()
                ),
                Err(error) => eprintln!("<4>refused a connection without credentials: {error}"),
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

#[cfg(test)]
#[path = "../tests/unit/axum/launcher_only.rs"]
mod tests;
