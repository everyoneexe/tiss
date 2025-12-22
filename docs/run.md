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
command = "/usr/bin/env LIBSEAT_BACKEND=logind II_GREETD_QML_FILE=/usr/share/ii-greetd/qml/Main.qml II_GREETD_BACKEND=/usr/lib/ii-greetd/ii-greetd-backend II_GREETD_DEFAULT_USER=YOUR_USER II_GREETD_LOCK_USER=1 II_GREETD_LOG_DIR=/tmp/ii-greetd-logs II_GREETD_SESSION_CMD=niri /usr/bin/cage -s -- /usr/bin/ii-greetd-ui"
user = "greeter"
```

Restart greetd:

```bash
sudo systemctl restart greetd
```

Session override:

```bash
env II_GREETD_SESSION_CMD="niri" ii-greetd-backend
```

Note: The backend connects to the `GREETD_SOCK` socket. Running the UI outside
greetd will fail unless that variable is set to a valid greetd socket path.

Logs (default):

- `/tmp/ii-greetd-<uid>/ii-greetd-ui.log`
- `/tmp/ii-greetd-<uid>/ii-greetd-backend.log`

Override log directory:

```bash
env II_GREETD_LOG_DIR=/tmp/ii-greetd-logs ii-greetd-ui
```
