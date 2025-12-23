# Environment Variables

This file lists the environment variables used by TISS.

## Core

- `TISS_GREETD_BACKEND`: absolute path to `tiss-greetd-backend` (overrides search).
- `TISS_GREETD_DEFAULT_USER`: prefill username field.
- `TISS_GREETD_LOCK_USER`: if set, hide username input and force default user.
- `TISS_GREETD_SESSION_JSON`: override session command as a JSON array.
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
- `TISS_GREETD_LOG_DIR`: directory for log files.
- `TISS_GREETD_SHOW_PASSWORD_TOGGLE`: set to `0` to hide the "show password" toggle.

## Launcher

- `TISS_GREETD_CAGE_ARGS`: extra args passed to `cage`.
- `TISS_GREETD_CAGE_BIN`: override the cage binary path.
- `TISS_GREETD_UI_BIN`: override the UI binary path.

## Theme Resolution

Resolution order (first match wins):

1) `TISS_GREETD_QML_FILE` / `paths.qml_file`
2) `TISS_GREETD_THEME_DIR` / `paths.theme_dir`
3) `TISS_GREETD_THEME` / `paths.theme`
4) Built-in QML module or system fallback

Search roots for `TISS_GREETD_THEME`:

- `~/.local/share/tiss-greetd/themes/<name>/Main.qml`
- `/usr/local/share/tiss-greetd/themes/<name>/Main.qml`
- `/usr/share/tiss-greetd/themes/<name>/Main.qml`
- `../themes/<name>/Main.qml` relative to the binary (dev)

