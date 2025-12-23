# tiss-greetd-qml

QML-based greeter for greetd. This is a new, standalone app (not the existing
Quickshell lockscreen) intended to provide a tiss-login-like login feel.

## Why not SDDM

TISS is not an SDDM clone. It is a Wayland-first greetd greeter with a strict
QML UI boundary and a stable, typed auth contract. That keeps the core minimal,
theme-agnostic, and animation-friendly while avoiding SDDM’s X11 legacy and
theme-plugin coupling. See `docs/WHY_NOT_SDDM.md`.

## Architecture

- `backend/` (Rust): speaks to greetd over GREETD_SOCK and exposes a JSON-lines
  protocol over stdin/stdout for the UI.
- `ui/` (Qt/QML): renders the login screen and talks to the backend process.
- `launcher/` (Rust): resolves config, sanitizes env, and starts cage + UI.

## Build (dev)

Backend:

```bash
cd backend
cargo build
```

Launcher:

```bash
cd launcher
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
./ui/build/tiss-greetd-ui
```

Note: config files are applied by `tiss-greetd-launcher`; running the UI
directly only uses environment variables.

If `~/.local/state/tiss-greetd/appearance.json` (or `XDG_STATE_HOME`) exists,
the launcher exports it as `TISS_GREETD_APPEARANCE_JSON` for themes to consume.

## Config

Config files are optional and layered in this order (last wins):

1) `/etc/tiss-greetd/config.toml`
2) `~/.config/tiss-greetd/config.toml`
3) environment variables (highest priority)

See `docs/tiss-greetd-config.toml.example` for the full schema (packaging installs
to `/usr/share/tiss-greetd/config.toml.example`).

## Docs

- `docs/APPEARANCE.md`: optional appearance exporter workflow.
- `docs/CORE_HARDENING.md`: core hardening summary and validation checklist.
- `docs/ENV.md`: environment variables and theme resolution.
- `docs/PAM.md`: prompt/message contract, typed errors, and distro policy.
- `docs/THEME_AUTHORING.md`: how to build themes and handle prompts.
- `docs/run.md`: greetd setup and dev run notes.
- `docs/WHY_NOT_SDDM.md`: design intent vs SDDM.

## Status

Greetd IPC is implemented using the `greetd-ipc` wire format (length-prefixed
JSON). UI/backend protocol details live in `protocol/README.md`.
