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
./ui/build/ii-greetd-ui
```

Note: config files are applied by `ii-greetd-launcher`; running the UI
directly only uses environment variables.

## Config

Config files are optional and layered in this order (last wins):

1) `/etc/ii-greetd/config.toml`
2) `~/.config/ii-greetd/config.toml`
3) environment variables (highest priority)

See `docs/ii-greetd-config.toml.example` for the full schema (packaging installs
to `/usr/share/ii-greetd/config.toml.example`).

## Environment variables

- `II_GREETD_BACKEND`: absolute path to `ii-greetd-backend` (overrides search).
- `II_GREETD_DEFAULT_USER`: prefill username field.
- `II_GREETD_LOCK_USER`: if set, hide username input and force default user.
- `II_GREETD_SESSION_JSON`: override session command as a JSON array (defaults to `["niri"]`).
- `II_GREETD_SESSION_ENV_JSON`: override session env as a JSON object.
- `II_GREETD_SESSIONS_JSON`: discovered sessions list as JSON (for themes).
- `II_GREETD_LAST_SESSION_ID`: last selected session id (for themes).
- `II_GREETD_QML_URI`: override QML module URI (defaults to `IIGreetd`).
- `II_GREETD_QML_FILE`: absolute path to a QML file to load directly.
- `II_GREETD_THEME_DIR`: directory containing a theme `Main.qml`.
- `II_GREETD_THEME`: theme name (searched in theme roots).
- `II_GREETD_LOG_DIR`: directory for log files (defaults to `/tmp/ii-greetd-<uid>`).
- `II_GREETD_SHOW_PASSWORD_TOGGLE`: set to `0` to hide the "show password" toggle.

Launcher environment variables (used by `ii-greetd-launcher`):

- `II_GREETD_CAGE_ARGS`: extra args passed to `cage`.
- `II_GREETD_CAGE_BIN`: override the cage binary path.
- `II_GREETD_UI_BIN`: override the UI binary path.

## Themes

Theme resolution order (first match wins):

1) `II_GREETD_QML_FILE` / `paths.qml_file`
2) `II_GREETD_THEME_DIR` / `paths.theme_dir`
3) `II_GREETD_THEME` / `paths.theme`
4) Built-in QML module or system fallback

If an explicit theme path/name fails to load, the greeter shows a hard error
screen and does not silently fall back.

Theme search roots (for `II_GREETD_THEME`):

- `~/.local/share/ii-greetd/themes/<name>/Main.qml`
- `/usr/local/share/ii-greetd/themes/<name>/Main.qml`
- `/usr/share/ii-greetd/themes/<name>/Main.qml`
- `../themes/<name>/Main.qml` relative to the binary (dev)

Example theme:

- `themes/lockscreen-vibe`

Sessions are discovered from:

- `/usr/share/wayland-sessions/*.desktop`
- `/usr/share/xsessions/*.desktop`

Last-selected session is persisted and restored on the next login.

## How to write a theme

Create a directory with a `Main.qml` entry point. Your QML can use the
following context properties:

- `iiDefaultUser` (string)
- `iiLockUser` (bool)
- `iiShowPasswordToggle` (bool)
- `iiSessionCommand` (string list)
- `iiSessionEnv` (map)
- `iiSessions` (list of session objects)
- `iiLastSessionId` (string)

`iiSessions` entries include:

- `id` (string)
- `name` (string)
- `exec` (string list)
- `type` (`"wayland"` or `"x11"`)
- `desktop_file` (string)

`BackendProcess` emits:

- `promptReceived(id, kind, message, echo)`
- `errorReceived(code, message)` where code is `auth_failed`, `account_locked`,
  `password_expired`, `pam_error`, or `backend_crash`.
- `success()` / `backendCrashed(message)`

`BackendProcess` properties:

- `sessionCommand`, `sessionEnv`
- `selectedSessionId` (optional; persisted on success)

Minimal example:

```qml
import QtQuick 2.15
import QtQuick.Controls 2.15
import IIGreetd 1.0

ApplicationWindow {
    id: root
    visible: true
    property string defaultUser: iiDefaultUser
    property bool lockUser: iiLockUser

    property int promptId: -1
    property string promptKind: ""
    property string promptMessage: ""

    BackendProcess {
        id: backend
        sessionCommand: iiSessionCommand
        sessionEnv: iiSessionEnv
        selectedSessionId: iiLastSessionId
        onPromptReceived: (id, kind, message, echo) => {
            promptId = id
            promptKind = kind
            promptMessage = message
        }
    }

    function doLogin() {
        backend.authenticate(lockUser ? defaultUser : userField.text)
    }

    function submitPrompt() {
        if (promptKind === "secret" || promptKind === "visible") {
            backend.respondPrompt(promptId, promptField.text)
        } else {
            backend.ackPrompt(promptId)
        }
    }
}
```

## greetd example

See `docs/greetd-config.toml.example` for a greetd setup using
`ii-greetd-launcher`.

## Arch packaging

PKGBUILD: `packaging/arch/PKGBUILD`
Update `_repo_url` inside the PKGBUILD before running `makepkg`.

## Status

Greetd IPC is implemented using the `greetd-ipc` wire format (length-prefixed
JSON). UI/backend protocol details live in `protocol/README.md`. UI/UX still
needs refinement to match ii-niri visuals.
