mod launcher_only;
mod terminal;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::{
    env,
    os::unix::fs::FileTypeExt,
    path::{Component, Path, PathBuf},
};
use tokio::net::{UnixListener, UnixStream};

#[derive(Clone)]
struct AppState {
    ui_dir: PathBuf,
    launcher: PathBuf,
    shell: PathBuf,
}

fn unavailable(error: impl std::fmt::Display) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error": format!("Launcher unavailable: {error}")})),
    )
        .into_response()
}

async fn terminal(State(state): State<AppState>, request: Request) -> Response {
    let stream = match UnixStream::connect(&state.launcher).await {
        Ok(stream) => stream,
        Err(error) => return unavailable(error),
    };

    let (mut sender, connection) =
        match hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream)).await {
            Ok(connection) => connection,
            Err(error) => return unavailable(error),
        };
    tokio::spawn(async move {
        let _ = connection.await;
    });

    let (mut parts, body) = request.into_parts();
    parts.headers.remove(header::HOST);
    parts.headers.remove(header::COOKIE);
    parts
        .headers
        .insert(header::HOST, "localhost".parse().expect("static host"));
    match sender.send_request(Request::from_parts(parts, body)).await {
        Ok(response) => response.map(Body::new),
        Err(error) => unavailable(error),
    }
}

async fn health() -> impl IntoResponse {
    Json(json!({"status":"ok","version":env!("CARGO_PKG_VERSION")}))
}

fn ui_dir_for(
    override_dir: Option<PathBuf>,
    executable: Result<PathBuf, String>,
) -> Result<PathBuf, String> {
    if let Some(path) = override_dir {
        return Ok(path);
    }

    let executable = executable?;
    let binary_dir = executable.parent().ok_or("Invalid executable path")?;
    let packaged = binary_dir
        .parent()
        .ok_or("Invalid executable path")?
        .join("dist");

    if packaged.join("index.html").is_file() {
        Ok(packaged)
    } else {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dist/axum"))
    }
}

fn socket_path(
    args: Vec<std::ffi::OsString>,
    configured: Option<std::ffi::OsString>,
) -> Result<PathBuf, String> {
    let mut listen = configured;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        if arg == "--listen" {
            listen = Some(args.next().ok_or("--listen requires a socket path")?);
        } else {
            return Err(format!("Unrecognized argument: {}", arg.to_string_lossy()));
        }
    }

    let path = PathBuf::from(listen.ok_or("Provide --listen or NAS_APP_SOCKET")?);
    if path.as_os_str().is_empty() {
        return Err("Empty socket path".into());
    }
    Ok(path)
}

async fn static_file(State(state): State<AppState>, uri: Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/api/") || path == "/api" {
        return StatusCode::NOT_FOUND.into_response();
    }

    let requested = if path == "/" {
        "index.html"
    } else {
        &path[1..]
    };

    let relative = Path::new(requested);
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let file = state.ui_dir.join(relative);
    let mime = match file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    };

    let cache = if requested.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };

    match tokio::fs::read(file).await {
        Ok(bytes) => (
            [(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache)],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn prepare_socket(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = tokio::fs::symlink_metadata(path).await {
        if !metadata.file_type().is_socket() {
            return Err(format!("Socket path is occupied: {}", path.display()));
        }
        match UnixStream::connect(path).await {
            Ok(_) => return Err(format!("Socket is already listening: {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {}
            Err(error) => return Err(format!("Unable to check socket: {error}")),
        }
        tokio::fs::remove_file(path)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn run(
    args: Vec<std::ffi::OsString>,
    configured_socket: Option<std::ffi::OsString>,
    configured_ui: Option<PathBuf>,
    shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = socket_path(args, configured_socket)?;
    let state = AppState {
        shell: env::var_os("NAS_TERMINAL_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/run/nas-os-terminal.sock")),
        launcher: env::var_os("NAS_LAUNCHER_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/run/nas-os/launcher/apps.sock")),
        ui_dir: ui_dir_for(
            configured_ui,
            env::current_exe().map_err(|error| error.to_string()),
        )?,
    };
    if !state.ui_dir.join("index.html").is_file() {
        return Err(format!("Frontend not found: {}", state.ui_dir.display()).into());
    }

    prepare_socket(&path).await?;

    let listener = UnixListener::bind(&path)?;
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/terminal/session", get(terminal::open))
        .route("/api/terminal/log", get(terminal))
        .route("/api/settings/terminal", get(terminal).put(terminal))
        .fallback(get(static_file))
        .with_state(state);

    println!("nas-system-terminal listening on {}", path.display());
    let result = axum::serve(launcher_only::LauncherOnly::new(listener), app)
        .with_graceful_shutdown(shutdown_signal)
        .await;

    if let Err(error) = tokio::fs::remove_file(&path).await {
        eprintln!("Unable to remove socket: {error}");
    }

    result?;
    Ok(())
}

async fn serve() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    run(
        env::args_os().skip(1).collect(),
        env::var_os("NAS_APP_SOCKET"),
        env::var_os("NAS_UI_DIR").map(PathBuf::from),
        async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
        },
    )
    .await
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if env::args_os().nth(1).is_some_and(|arg| arg == "--shell") {
        terminal::worker()?;
        return Ok(());
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve())
}

#[cfg(test)]
#[path = "../tests/unit/axum/main.rs"]
mod tests;
