# Dev run notes

## Dependencies (Arch)

- greetd
- cage
- qt6-base
- qt6-declarative
- rust (for backend build)

## greetd

Example config:

```toml
[terminal]
vt = 1

[default_session]
command = "/usr/bin/env TISS_GREETD_DEFAULT_USER=YOUR_USER TISS_GREETD_LOCK_USER=1 TISS_GREETD_SESSION_JSON='[\"niri\"]' /usr/bin/tiss-greetd-launcher"
user = "greeter"
```

Restart greetd:

```bash
sudo systemctl restart greetd
```

Note: `tiss-greetd-launcher` picks `LIBSEAT_BACKEND=seatd` when `/run/seatd.sock`
exists, otherwise `logind`. Override by setting `LIBSEAT_BACKEND` explicitly.

Config load order (last wins):

1) `/etc/tiss-greetd/config.toml`
2) `~/.config/tiss-greetd/config.toml`
3) Environment variables

Example schema:

- `docs/tiss-greetd-config.toml.example`

Theme selection:

- `TISS_GREETD_QML_FILE`: absolute path to a `Main.qml`.
- `TISS_GREETD_THEME_DIR`: directory containing `Main.qml`.
- `TISS_GREETD_THEME`: theme name (searched in standard theme dirs).

Session override:

```bash
env TISS_GREETD_SESSION_JSON='[\"niri\"]' tiss-greetd-backend
```

Note: The backend connects to the `GREETD_SOCK` socket. Running the UI outside
greetd will fail unless that variable is set to a valid greetd socket path.

Logs (default):

- `/tmp/tiss-greetd-<uid>/tiss-greetd-ui.log`
- `/tmp/tiss-greetd-<uid>/tiss-greetd-backend.log`

Override log directory:

```bash
env TISS_GREETD_LOG_DIR=/tmp/tiss-greetd-logs tiss-greetd-launcher
```

Launcher overrides:

- `TISS_GREETD_BACKEND`: absolute path to `tiss-greetd-backend`.
- `TISS_GREETD_QML_FILE`: absolute path to `Main.qml`.
- `TISS_GREETD_CAGE_ARGS`: extra args passed to `cage` (e.g. `"-d"`).
