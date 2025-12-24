# Changelog

## v0.1.1 - 2025-12-25

- Core stabilized with clippy/qmllint clean builds.
- Backend sends hello handshake on startup.
- CMake Qt6 policy warnings resolved.
- PAM edge cases handled best-effort.
- Session/profile/locale selection complete.
- No breaking changes from v0.1.0.

## v0.1.0 - 2025-12-22

- greetd backend with PAM prompt forwarding and structured error codes.
- QML UI supports prompt-driven auth and explicit theme load failures.
- Rust launcher handles config, session discovery, and last-session persistence.
- Session selection metadata exposed to themes with QML-first control.
