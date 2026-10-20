# Packaging and release

Run `npm run release:test` before releasing. It checks release metadata, formatting, lint, types, unit/component/browser tests, Rust formatting, Clippy and tests, then builds the archive and runs its smoke test. See [Development](DEVELOPMENT.md) for prerequisites.

`npm run nas:package` builds the frontend and release backend, then creates `release-assets/nas-system-terminal-<version>-linux-<arch>.tar.gz` and its SHA256 sidecar. Packaging supports native Linux x64 and arm64; the GitHub Build workflow uses x64. `npm run nas:test` checks the current archive. The package copies `src/assets/logo.svg` to the `icon.svg` path declared in the manifest.

Keep `package.json`, `package-lock.json`, `nas-app.json`, `backend/axum/Cargo.toml`, `backend/Cargo.lock` and `CHANGELOG.txt` aligned. `npm run release:check` verifies the package, manifest and Cargo versions and the matching changelog entry. `sonar.properties` records the analysis version separately and must be updated when it changes.

From a clean working tree, `npm run release:merge -- --dry-run` reports the next version. Without `--dry-run`, the command updates version metadata and the changelog, formats and runs release checks, commits, merges into `main` when necessary, pushes `main`, creates the version tag and pushes it. CI runs on pull requests and manual dispatch; the Build workflow publishes archives for version tags whose commit is already reachable from `main`.

The launcher catalogue declares this application as a preinstalled system application. Install the package before opening the external terminal shortcut. Interactive sessions also require the launcher-installed terminal socket, service template and configured Linux account described in [Application contract](APPS.md).
