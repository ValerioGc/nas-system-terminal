# Application contract

Identity: `nas-system-terminal`, UI protocol 1, system category. The package contains `nas-app.json`, `icon.svg`, `bin/nas-system-terminal-axum` and the built interface under `dist/`.

The launcher supplies theme, palette, locale and visibility through the `nas-base-app` bridge. The HTTP backend listens on the Unix socket passed through `--listen` or `NAS_APP_SOCKET`; the packaged manifest declares `/run/nas-os/apps/nas-system-terminal.sock`. It serves `/health`, static interface assets, the WebSocket endpoint `/api/terminal/session`, journal SSE at `/api/terminal/log`, and GET/PUT appearance settings at `/api/settings/terminal`.

The app checks Unix peer credentials before accepting HTTP connections: root, the owner of the socket directory and the owner of the socket file are admitted. Journal and appearance requests are forwarded to `/run/nas-os/launcher/apps.sock`, overridable through `NAS_LAUNCHER_SOCKET`. The launcher identifies the application by its cgroup and restricts these routes to `nas-system-terminal`.

The app implements the WebSocket bridge, PTY and login shell. It connects to `/run/nas-os-terminal.sock`, overridable through `NAS_TERMINAL_SOCKET`; systemd starts the app binary in `--shell` mode for each connection. The launcher installs the socket and service, configures the dedicated Linux account and sets the systemd session policy. The current socket permits up to four simultaneous connections. Shell commands run under ordinary Linux permissions; the app validates transport frames and terminal dimensions rather than applying a command allowlist.

The launcher owns the journal reader and stored appearance settings, including settings validation and configuration export/import. Factory reset and uninstall remove the launcher-managed terminal services and account configuration. The app manifest declares no additional permission grants.
