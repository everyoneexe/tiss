# Protocol (JSON Lines)

One JSON object per line. UTF-8 encoded text.

## UI -> Backend

```json
{ "type": "hello", "ui_version": 2 }
{ "type": "auth", "username": "alice", "command": ["niri"], "env": {"XDG_SESSION_TYPE": "wayland"}, "session_id": "niri", "profile_id": "work", "locale": "en_US.UTF-8" }
{ "type": "prompt_response", "id": 1, "response": "secret" }
{ "type": "prompt_response", "id": 2, "response": null }
{ "type": "start", "command": ["niri"], "env": {"XDG_SESSION_TYPE": "wayland"} }
{ "type": "power", "action": "reboot" }
```

## Backend -> UI

```json
{ "type": "state", "phase": "idle" }
{ "type": "state", "phase": "auth" }
{ "type": "state", "phase": "waiting" }
{ "type": "state", "phase": "error" }
{ "type": "state", "phase": "success" }
{ "type": "prompt", "id": 1, "kind": "secret", "message": "Password:", "echo": false }
{ "type": "prompt", "id": 2, "kind": "info", "message": "Password expired", "echo": true }
{ "type": "error", "code": "auth_failed", "message": "Authentication failed" }
{ "type": "success" }
```

## Runtime states

- `idle`: UI is ready for input, no active authentication request.
- `auth`: authentication in progress (PAM conversation).
- `waiting`: session start requested and waiting for greetd to respond.
- `error`: last authentication failed; error message follows via `error` payload.
- `success`: authentication succeeded and session will start.

Auth errors are normalized with explicit codes:

- `auth_failed`: bad credentials or generic auth failure.
- `account_locked`: account locked or disabled.
- `password_expired`: account or password expired.
- `pam_error`: other PAM/system errors.
- `power_denied`: power action denied by policy.
- `power_error`: power action failed.
