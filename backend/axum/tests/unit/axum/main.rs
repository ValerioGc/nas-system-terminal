use super::*;
use axum::body::to_bytes;
use std::{
    fs,
    os::unix::net::UnixListener as StdUnixListener,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::UnixListener;

#[tokio::test]
async fn forwards_streams_without_buffering_and_closes_them_with_the_browser() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let dir = temp_dir();
    let path = dir.join("launcher.sock");
    let listener = UnixListener::bind(&path).expect("launcher socket");
    let upstream = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("request");
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            socket.read_exact(&mut byte).await.expect("header");
            request.push(byte[0]);
        }
        assert!(String::from_utf8_lossy(&request).starts_with("GET /api/terminal/log HTTP/1.1"));
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").await.expect("headers");
        let event = b"event: session\ndata: session-1\n\n";
        socket
            .write_all(format!("{:x}\r\n", event.len()).as_bytes())
            .await
            .expect("chunk size");
        socket.write_all(event).await.expect("event");
        socket.write_all(b"\r\n").await.expect("chunk end");
        let mut rest = Vec::new();
        socket.read_to_end(&mut rest).await.expect("closed");
    });

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        terminal(
            State(AppState {
                ui_dir: dir.clone(),
                shell: PathBuf::from("/no/shell.sock"),
                launcher: path,
            }),
            Request::builder()
                .uri("/api/terminal/log")
                .body(Body::empty())
                .expect("request"),
        ),
    )
    .await
    .expect("headers arrive before the stream ends");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/event-stream"
    );
    drop(response);

    tokio::time::timeout(std::time::Duration::from_secs(2), upstream)
        .await
        .expect("upstream closes")
        .expect("server");
    fs::remove_dir_all(dir).expect("cleanup");
}

#[tokio::test]
async fn reports_an_unavailable_launcher_without_falling_back_to_another_transport() {
    let response = terminal(
        State(AppState {
            ui_dir: PathBuf::new(),
            shell: PathBuf::from("/no/shell.sock"),
            launcher: PathBuf::from("/no/launcher.sock"),
        }),
        Request::builder()
            .uri("/api/settings/terminal")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

pub(super) fn temp_dir() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "nas-system-terminal-test-{}-{nonce}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[tokio::test]
async fn health_endpoint_returns_version() {
    let response = health().await.into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["status"],
        "ok"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["version"],
        env!("CARGO_PKG_VERSION")
    );
}

#[tokio::test]
async fn static_file_blocks_api_and_traversal_and_serves_known_mime_types() {
    let dir = temp_dir();
    fs::write(dir.join("index.html"), "home").unwrap();
    fs::write(dir.join("app.js"), "script").unwrap();
    let state = State(AppState {
        ui_dir: dir.clone(),
        shell: PathBuf::from("/no/shell.sock"),
        launcher: PathBuf::from("/no/launcher.sock"),
    });

    let api = static_file(state.clone(), "/api/entries".parse().unwrap()).await;
    assert_eq!(api.status(), StatusCode::NOT_FOUND);
    let api_root = static_file(state.clone(), "/api".parse().unwrap()).await;
    assert_eq!(api_root.status(), StatusCode::NOT_FOUND);
    let traversal = static_file(state.clone(), "/../secret".parse().unwrap()).await;
    assert_eq!(traversal.status(), StatusCode::BAD_REQUEST);
    let home = static_file(state.clone(), "/".parse().unwrap()).await;
    assert_eq!(home.headers()[header::CACHE_CONTROL], "no-cache");
    fs::create_dir_all(dir.join("assets")).unwrap();
    fs::write(dir.join("assets/index-abc.js"), "script").unwrap();
    let asset = static_file(state.clone(), "/assets/index-abc.js".parse().unwrap()).await;
    assert_eq!(
        asset.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert_eq!(
        home.headers()[header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );
    assert_eq!(
        to_bytes(home.into_body(), usize::MAX).await.unwrap(),
        "home"
    );
    let js = static_file(state.clone(), "/app.js".parse().unwrap()).await;
    assert_eq!(
        js.headers()[header::CONTENT_TYPE],
        "text/javascript; charset=utf-8"
    );
    let missing = static_file(state, "/missing.bin".parse().unwrap()).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn static_file_uses_fallback_mime_for_unknown_extensions() {
    let dir = temp_dir();
    fs::write(dir.join("data.bin"), "data").unwrap();
    let response = static_file(
        State(AppState {
            ui_dir: dir.clone(),
            shell: PathBuf::from("/no/shell.sock"),
            launcher: PathBuf::from("/no/launcher.sock"),
        }),
        "/data.bin".parse().unwrap(),
    )
    .await;
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/octet-stream"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn socket_preparation_rejects_regular_files_and_live_sockets() {
    let dir = temp_dir();
    let file = dir.join("occupied");
    fs::write(&file, "file").unwrap();
    assert!(prepare_socket(&file)
        .await
        .unwrap_err()
        .contains("occupied"));

    let socket = dir.join("live.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    assert!(prepare_socket(&socket)
        .await
        .unwrap_err()
        .contains("already listening"));
    drop(listener);
    fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn socket_preparation_removes_a_stale_socket() {
    let dir = temp_dir();
    let socket = dir.join("stale.sock");
    let listener = StdUnixListener::bind(&socket).unwrap();
    drop(listener);
    assert!(prepare_socket(&socket).await.is_ok());
    assert!(!socket.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn finds_the_configured_ui_directory_and_package_fallbacks() {
    let override_path = PathBuf::from("/custom/ui");
    assert_eq!(
        ui_dir_for(
            Some(override_path.clone()),
            Ok(PathBuf::from("/ignored/bin"))
        )
        .unwrap(),
        override_path
    );

    let dir = temp_dir();
    let executable = dir.join("package/bin/app");
    let packaged = dir.join("package/dist");
    fs::create_dir_all(&packaged).unwrap();
    fs::write(packaged.join("index.html"), "packaged").unwrap();
    assert_eq!(ui_dir_for(None, Ok(executable.clone())).unwrap(), packaged);

    let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dist/axum");
    assert_eq!(ui_dir_for(None, Ok(dir.join("bin/app"))).unwrap(), fallback);
    assert!(ui_dir_for(None, Err("no executable".to_owned())).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_bad_command_line_arguments() {
    assert!(socket_path(vec!["--listen".into()], None)
        .unwrap_err()
        .contains("requires a socket path"));
    assert!(socket_path(vec!["--other".into()], None)
        .unwrap_err()
        .contains("Unrecognized argument"));
    assert!(socket_path(vec![], None).unwrap_err().contains("Provide"));
    assert!(socket_path(vec!["--listen".into(), "".into()], None)
        .unwrap_err()
        .contains("Empty socket path"));
    assert_eq!(
        socket_path(vec![], Some("/tmp/app.sock".into())).unwrap(),
        PathBuf::from("/tmp/app.sock")
    );
}

#[tokio::test]
async fn starts_and_shuts_down_cleanly_with_a_configured_socket_and_ui() {
    let dir = temp_dir();
    let ui = dir.join("ui");
    fs::create_dir_all(&ui).unwrap();
    fs::write(ui.join("index.html"), "home").unwrap();
    let socket = dir.join("app.sock");
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(run(
        vec![],
        Some(socket.clone().into_os_string()),
        Some(ui),
        async {
            let _ = stopped.await;
        },
    ));
    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(socket.exists());
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert!(!socket.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn run_reports_missing_ui_and_socket_setup_errors() {
    let dir = temp_dir();
    let missing = run(
        vec![],
        Some(dir.join("missing.sock").into_os_string()),
        Some(dir.join("ui")),
        std::future::pending(),
    )
    .await;
    assert!(missing
        .unwrap_err()
        .to_string()
        .contains("Frontend not found"));

    let ui = dir.join("ui");
    fs::create_dir_all(&ui).unwrap();
    fs::write(ui.join("index.html"), "home").unwrap();
    let occupied = dir.join("occupied.sock");
    fs::write(&occupied, "file").unwrap();
    let error = run(
        vec![],
        Some(occupied.into_os_string()),
        Some(ui),
        std::future::pending(),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("occupied"));
    fs::remove_dir_all(dir).unwrap();
}
