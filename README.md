# ii-greetd-qml

QML-based greeter for greetd. This is a new, standalone app (not the existing
Quickshell lockscreen) intended to provide an ii-niri-like login feel.

## Architecture

- `backend/` (Rust): speaks to greetd over GREETD_SOCK and exposes a JSON-lines
  protocol over stdin/stdout for the UI.
- `ui/` (Qt/QML): renders the login screen and talks to the backend process.

## Build (dev)

Backend:

```bash
cd backend
cargo build
```

UI:

```bash
cd ui
cmake -S . -B build
cmake --build build
```

## Run (dev)

Run the UI directly (it will spawn the backend via PATH). This requires a
running greetd and a valid `GREETD_SOCK` environment variable (normally set
by greetd itself):

```bash
./ui/build/ii-greetd-ui
```

## Environment variables

- `II_GREETD_BACKEND`: absolute path to `ii-greetd-backend` (overrides search).
- `II_GREETD_DEFAULT_USER`: prefill username field.
- `II_GREETD_LOCK_USER`: if set, hide username input and force default user.
- `II_GREETD_SESSION_CMD`: override session command (defaults to `niri`).
- `II_GREETD_QML_URI`: override QML module URI (defaults to `IIGreetd`).
- `II_GREETD_QML_FILE`: absolute path to a QML file to load directly.
- `II_GREETD_LOG_DIR`: directory for log files (defaults to `/tmp/ii-greetd-<uid>`).

## greetd example

See `docs/greetd-config.toml.example` for a cage + greetd setup.

## Arch packaging

PKGBUILD: `packaging/arch/PKGBUILD`
Update `_repo_url` inside the PKGBUILD before running `makepkg`.

## Status

Greetd IPC is implemented using the `greetd-ipc` wire format (length-prefixed
JSON). UI/UX still needs refinement to match ii-niri visuals.
