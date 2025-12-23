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

## Appearance exporter (optional)

`tiss-greetd-appearance` generates `appearance.json` for themes. Example:

```bash
tiss-greetd-appearance --wallpaper /path/to/wall.jpg --scale 1.0 --radius 18
```

It also accepts `TISS_GREETD_WALLPAPER` if `--wallpaper` is not provided. The
output path defaults to `~/.local/state/tiss-greetd/appearance.json`.

For continuous updates (polling), use:

```bash
tiss-greetd-appearance --wallpaper-file /path/to/wallpaper.txt --watch --interval 2
```

Note: the greeter reads `appearance.json` on startup; live UI reload would need
theme-side file watching if you want mid-session updates.

## Config

Config files are optional and layered in this order (last wins):

1) `/etc/tiss-greetd/config.toml`
2) `~/.config/tiss-greetd/config.toml`
3) environment variables (highest priority)

See `docs/tiss-greetd-config.toml.example` for the full schema (packaging installs
to `/usr/share/tiss-greetd/config.toml.example`).

## Docs

- `docs/CORE_HARDENING.md`: core hardening summary and validation checklist.
- `docs/PAM.md`: prompt/message contract, typed errors, and distro policy.
- `docs/WHY_NOT_SDDM.md`: design intent vs SDDM.

## Environment variables

- `TISS_GREETD_BACKEND`: absolute path to `tiss-greetd-backend` (overrides search).
- `TISS_GREETD_DEFAULT_USER`: prefill username field.
- `TISS_GREETD_LOCK_USER`: if set, hide username input and force default user.
- `TISS_GREETD_SESSION_JSON`: override session command as a JSON array (defaults to `["niri"]`).
- `TISS_GREETD_SESSION_ENV_JSON`: override session env as a JSON object.
- `TISS_GREETD_SESSIONS_JSON`: discovered sessions list as JSON (for themes).
- `TISS_GREETD_LAST_SESSION_ID`: last selected session id (for themes).
- `TISS_GREETD_PROFILES_JSON`: profiles list as JSON (for themes).
- `TISS_GREETD_LAST_PROFILE_ID`: last selected profile id (for themes).
- `TISS_GREETD_LOCALES_JSON`: locale config as JSON (for themes).
- `TISS_GREETD_LAST_LOCALE`: last selected locale (for themes).
- `TISS_GREETD_POWER_ACTIONS_JSON`: enabled power actions as JSON (for themes).
- `TISS_GREETD_POWER_ALLOWED_STATES_JSON`: allowed power states as JSON (backend policy).
- `TISS_GREETD_APPEARANCE_JSON`: appearance tokens JSON (from optional exporter).
- `TISS_GREETD_QML_URI`: override QML module URI (defaults to `TissGreetd`).
- `TISS_GREETD_QML_FILE`: absolute path to a QML file to load directly.
- `TISS_GREETD_THEME_DIR`: directory containing a theme `Main.qml`.
- `TISS_GREETD_THEME`: theme name (searched in theme roots).
- `TISS_GREETD_LOG_DIR`: directory for log files (defaults to `/tmp/tiss-greetd-<uid>`).
- `TISS_GREETD_SHOW_PASSWORD_TOGGLE`: set to `0` to hide the "show password" toggle.

Launcher environment variables (used by `tiss-greetd-launcher`):

- `TISS_GREETD_CAGE_ARGS`: extra args passed to `cage`.
- `TISS_GREETD_CAGE_BIN`: override the cage binary path.
- `TISS_GREETD_UI_BIN`: override the UI binary path.

## Themes

Theme resolution order (first match wins):

1) `TISS_GREETD_QML_FILE` / `paths.qml_file`
2) `TISS_GREETD_THEME_DIR` / `paths.theme_dir`
3) `TISS_GREETD_THEME` / `paths.theme`
4) Built-in QML module or system fallback

If an explicit theme path/name fails to load, the greeter shows a hard error
screen and does not silently fall back.

Theme search roots (for `TISS_GREETD_THEME`):

- `~/.local/share/tiss-greetd/themes/<name>/Main.qml`
- `/usr/local/share/tiss-greetd/themes/<name>/Main.qml`
- `/usr/share/tiss-greetd/themes/<name>/Main.qml`
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

- `tissDefaultUser` (string)
- `tissLockUser` (bool)
- `tissShowPasswordToggle` (bool)
- `tissSessionCommand` (string list)
- `tissSessionEnv` (map)
- `tissSessions` (list of session objects)
- `tissLastSessionId` (string)
- `tissProfiles` (list of profile objects)
- `tissLastProfileId` (string)
- `tissLocales` (map with `default` and `available`)
- `tissLastLocale` (string)
- `tissPowerActions` (list of strings)
- `tissAppearance` (map of appearance tokens)

`tissAppearance` common keys (optional, theme-defined):

- `accent`, `bg`, `fg`, `subfg`, `card_bg`, `card_border`, `shadow`
- `radius` (number), `scale` (number), `wallpaper_path` (string)

`tissSessions` entries include:

- `id` (string)
- `name` (string)
- `exec` (string list)
- `type` (`"wayland"` or `"x11"`)
- `desktop_file` (string)

`BackendProcess` emits:

- `promptReceived(id, kind, message, echo)`
- `messageReceived(kind, message)`
- `errorReceived(code, message)` where code is `auth_failed`, `account_locked`,
  `password_expired`, `pam_error`, `power_denied`, `power_error`, or `backend_crash`.
- `success()` / `backendCrashed(message)`

`BackendProcess` properties:

- `sessionCommand`, `sessionEnv`
- `selectedSessionId` (optional; persisted on success)
- `selectedProfileId` (optional; persisted on success)
- `selectedLocale` (optional; persisted on success)

`BackendProcess` methods:

- `requestPower(action)` where action is `poweroff`, `reboot`, or `suspend`

## PAM edge cases (OTP / password change / faillock)

TISS forwards PAM_TEXT_INFO / PAM_ERROR_MSG as `messageReceived(kind, message)` and only
uses `promptReceived(...)` for responses that require user input. Themes should never
string-match PAM messages.

Typed error codes are stable and theme-safe:

- `auth_failed`: wrong password or generic auth failure
- `account_locked`: faillock/lockout or explicit account lock
- `password_expired`: expired or change-required tokens
- `pam_error`: anything else (stack-specific or unsupported flows)

OTP and password-change flows appear as multiple `promptReceived(...)` events. Themes must
handle multi-step prompts (e.g., OTP → new password → confirm).

## Distro support policy

Supported targets: Arch, Fedora, Debian. Other distributions are best-effort.  
If the PAM stack is exotic or returns unknown errors, TISS degrades to `pam_error` without
breaking the UI contract.

Minimal example:

```qml
import QtQuick 2.15
import QtQuick.Controls 2.15
import TissGreetd 1.0

ApplicationWindow {
    id: root
    visible: true
    property string defaultUser: tissDefaultUser
    property bool lockUser: tissLockUser

    property int promptId: -1
    property string promptKind: ""
    property string promptMessage: ""

    BackendProcess {
        id: backend
        sessionCommand: tissSessionCommand
        sessionEnv: tissSessionEnv
        selectedSessionId: tissLastSessionId
        selectedProfileId: tissLastProfileId
        selectedLocale: tissLastLocale
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
`tiss-greetd-launcher`.

## Arch packaging

PKGBUILD: `packaging/arch/PKGBUILD`
Update `_repo_url` inside the PKGBUILD before running `makepkg`.

## Status

Greetd IPC is implemented using the `greetd-ipc` wire format (length-prefixed
JSON). UI/backend protocol details live in `protocol/README.md`. UI/UX still
needs refinement to match tiss-login visuals.
