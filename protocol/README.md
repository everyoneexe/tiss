# Protocol (JSON Lines)

One JSON object per line. UTF-8 encoded text.

## UI -> Backend

```json
{ "type": "hello", "ui_version": 1 }
{ "type": "auth", "username": "alice", "password": "secret", "command": ["niri"], "env": {"XDG_SESSION_TYPE": "wayland"} }
{ "type": "start", "command": ["niri"], "env": {"XDG_SESSION_TYPE": "wayland"} }
{ "type": "power", "action": "reboot" }
```

## Backend -> UI

```json
{ "type": "state", "phase": "idle" }
{ "type": "state", "phase": "authenticating" }
{ "type": "state", "phase": "failed" }
{ "type": "error", "message": "bad credentials" }
{ "type": "success" }
```
