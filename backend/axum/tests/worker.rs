use std::{
    io::{Read, Write},
    os::{fd::OwnedFd, unix::net::UnixStream},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn the_packaged_worker_opens_an_interactive_shell_and_ends_with_its_connection() {
    let (worker, mut client) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_nas-system-terminal-axum"))
        .arg("--shell")
        .stdin(Stdio::from(OwnedFd::from(worker)))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut frame = vec![0];
    frame.extend_from_slice(b"test -t 0 && printf '\\nWORKER_READY\\n'\r");
    client
        .write_all(&(frame.len() as u32).to_be_bytes())
        .unwrap();
    client.write_all(&frame).unwrap();
    let mut output = Vec::new();
    let mut buffer = [0; 8192];
    while !String::from_utf8_lossy(&output).contains("\r\nWORKER_READY\r\n") {
        let length = client.read(&mut buffer).unwrap();
        assert_ne!(length, 0);
        output.extend_from_slice(&buffer[..length]);
    }
    drop(client);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "{}",
                String::from_utf8_lossy(&child.wait_with_output().unwrap().stderr)
            );
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("worker kept running after disconnect");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn the_packaged_http_server_reports_invalid_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_nas-system-terminal-axum"))
        .arg("--unexpected")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unrecognized argument"));
}
