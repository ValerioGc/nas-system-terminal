# NAS Terminal

Interactive Linux terminal and system journal for the NAS launcher.

The console runs `/bin/bash --login -i` with a real controlling PTY. It supports interactive applications, ANSI output, shell history, completion, control keys and resizing. The browser uses xterm.js and WebSocket; the Rust backend uses nix for the PTY. xterm.js, its fit addon and nix use the MIT license.

Each window starts as a dedicated Linux account named after the wizard account, with its password locked and without sudo membership. Run `su - nas` and enter the existing Linux password to switch to `nas`; sudo then follows that account's system permissions. The terminal does not change the `nas` password or grant privileges. Closing the window ends the dedicated systemd session, including its remaining processes. Switching to the log tab keeps the shell open.

The launcher installs the terminal socket and service, configures the dedicated account and retains appearance settings in its database. The app implements the PTY and WebSocket session; journal and appearance requests are forwarded to the launcher. The log tab starts with the last 200 journal lines, follows new entries while open and retains up to 2,000 lines in the browser.

The app uses `nas-base-app` for the launcher context, palette, localization, frame guard, modal, Sass helpers and shared tooling. Icons use local SVG assets. The local decimal stepper supports fractional line heights.

See [Development](DEVELOPMENT.md), [NAS packaging](NAS.md) and [Application contract](APPS.md).
