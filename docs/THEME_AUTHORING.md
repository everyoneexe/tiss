# Theme Authoring Guide

This document describes how to write a theme for TISS and what guarantees the
core provides.

## Theme Entry Point

Your theme must provide a `Main.qml` entry point. Configure one of:

- `paths.qml_file` (absolute path to a QML file)
- `paths.theme_dir` (directory containing `Main.qml`)
- `paths.theme` (theme name searched in theme roots)

Theme search roots:

- `~/.local/share/tiss-greetd/themes/<name>/Main.qml`
- `/usr/local/share/tiss-greetd/themes/<name>/Main.qml`
- `/usr/share/tiss-greetd/themes/<name>/Main.qml`
- `../themes/<name>/Main.qml` relative to the UI binary (dev)

## Required Imports

At minimum:

```qml
import QtQuick 2.15
import QtQuick.Controls 2.15
import TissGreetd 1.0
```

## Core API (Theme-Safe)

Context properties:

- `tissDefaultUser` (string)
- `tissLockUser` (bool)
- `tissShowPasswordToggle` (bool)
- `tissSessionCommand` (string list)
- `tissSessionEnv` (map)
- `tissSessions` (list of session objects)
- `tissLastSessionId` (string)
- `tissProfiles` (list of profile objects)
- `tissLastProfileId` (string)
- `tissLocales` (map: `default`, `available`)
- `tissLastLocale` (string)
- `tissPowerActions` (list of strings)
- `tissAppearance` (map of appearance tokens)

Signals (`BackendProcess`):

- `promptReceived(id, kind, message, echo)`
- `messageReceived(kind, message)`
- `errorReceived(code, message)` where `code` is:
  `auth_failed`, `account_locked`, `password_expired`, `pam_error`,
  `power_denied`, `power_error`, `backend_crash`.
- `success()`, `backendCrashed(message)`

Methods (`BackendProcess`):

- `authenticate(username)`
- `respondPrompt(id, text)`
- `ackPrompt(id)`
- `requestPower(action)` where action is `poweroff`, `reboot`, `suspend`

## Prompt vs Message (Do Not Mix)

PAM_TEXT_INFO / PAM_ERROR_MSG map to `messageReceived`. These are informational
and do not require a response.

Only `promptReceived` requires a response. Treat prompt flows as multi-step:

1) Password prompt
2) OTP prompt (if enabled)
3) New password / confirm prompts (if password expired)

Never parse the prompt string to detect OTP/fingerprint. Render the message as
display text and keep your flow state generic.

## Typed Errors

Use `errorReceived(code, message)` without string matching. The core guarantees
stable error codes; message text is for user display only.

## Appearance Tokens

If `tissAppearance` is present, use it as a style map (colors, radius, scale,
wallpaper path). Always provide fallbacks in the theme for missing tokens.

Common keys:

- `accent`, `bg`, `fg`, `subfg`, `card_bg`, `card_border`, `shadow`
- `radius` (number), `scale` (number), `wallpaper_path` (string)

## Stability Guidelines

- Keep UI responsive during `auth` and `waiting` phases.
- Always return to `idle` state after cancel/timeout.
- Avoid long-running JS loops; keep animations lightweight.
- Do not rely on QML disk cache (disabled by launcher).

## Minimal Skeleton

```qml
import QtQuick 2.15
import QtQuick.Controls 2.15
import TissGreetd 1.0

ApplicationWindow {
    id: root
    visible: true

    property int promptId: -1
    property string promptKind: ""
    property string promptMessage: ""

    BackendProcess {
        id: backend
        sessionCommand: tissSessionCommand
        sessionEnv: tissSessionEnv
        onPromptReceived: (id, kind, message, echo) => {
            promptId = id
            promptKind = kind
            promptMessage = message
        }
    }

    function doLogin(user, pass) {
        backend.authenticate(user)
    }

    function submitPrompt(text) {
        if (promptKind === "secret" || promptKind === "visible") {
            backend.respondPrompt(promptId, text)
        } else {
            backend.ackPrompt(promptId)
        }
    }
}
```

