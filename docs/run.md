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
command = "/usr/bin/env II_GREETD_DEFAULT_USER=YOUR_USER II_GREETD_LOCK_USER=1 II_GREETD_SESSION_JSON='[\"niri\"]' /usr/bin/ii-greetd-launcher"
user = "greeter"
```

Restart greetd:

```bash
sudo systemctl restart greetd
```

Note: `ii-greetd-launcher` picks `LIBSEAT_BACKEND=seatd` when `/run/seatd.sock`
exists, otherwise `logind`. Override by setting `LIBSEAT_BACKEND` explicitly.

Config load order (last wins):

1) `/etc/ii-greetd/config.toml`
2) `~/.config/ii-greetd/config.toml`
3) Environment variables

Example schema:

- `docs/ii-greetd-config.toml.example`

Theme selection:

- `II_GREETD_QML_FILE`: absolute path to a `Main.qml`.
- `II_GREETD_THEME_DIR`: directory containing `Main.qml`.
- `II_GREETD_THEME`: theme name (searched in standard theme dirs).

Session override:

```bash
env II_GREETD_SESSION_JSON='[\"niri\"]' ii-greetd-backend
```

Note: The backend connects to the `GREETD_SOCK` socket. Running the UI outside
greetd will fail unless that variable is set to a valid greetd socket path.

Logs (default):

- `/tmp/ii-greetd-<uid>/ii-greetd-ui.log`
- `/tmp/ii-greetd-<uid>/ii-greetd-backend.log`

Override log directory:

```bash
env II_GREETD_LOG_DIR=/tmp/ii-greetd-logs ii-greetd-launcher
```

Launcher overrides:

- `II_GREETD_BACKEND`: absolute path to `ii-greetd-backend`.
- `II_GREETD_QML_FILE`: absolute path to `Main.qml`.
- `II_GREETD_CAGE_ARGS`: extra args passed to `cage` (e.g. `"-d"`).
