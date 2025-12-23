# Core Hardening

This document summarizes the stabilization work for the greetd core and defines
the validation checklist used to consider the core "stable".

## Patch Set 1: PAM flow + typed errors

Goals:
- Forward PAM_TEXT_INFO / PAM_ERROR_MSG as message events.
- Prompt only for response-requiring challenges.
- Provide typed errors: `auth_failed`, `account_locked`, `password_expired`,
  `pam_error`.
- Cancel/timeout returns the backend to `idle`.

Key behaviors:
- OTP and password-change are handled as multi-step prompts.
- Themes must not parse prompt strings.
- PAM errors fall back to `pam_error` when unknown.

## Patch Set 2: session start + state persistence

Goals:
- Deterministic start-session exit: backend waits for UI ack.
- Atomic state writes for session/profile/locale.

Key behaviors:
- UI sends an `ack` after `success`.
- State file writes are `write -> fsync -> rename`.
- State path defaults to `/var/lib/tiss-greetd/state.json` when `XDG_STATE_HOME`
  is missing.

## Patch Set 3: environment + logging + output retry

Goals:
- Minimal env pass-through.
- Explicit auth timing logs.
- Basic Wayland output retry.

Key behaviors:
- Launcher sanitizes environment to a strict allowlist.
- Backend logs auth attempt count + duration.
- UI retries outputs (100ms -> 300ms -> 1s) and logs failure.

## Validation Checklist

Manual tests (greetd + cage):
- Normal login succeeds.
- Wrong password returns `auth_failed`.
- `pam_faillock` lockout returns `account_locked`.
- Password expired / change-required returns `password_expired`.
- OTP flows require multi-step prompts.
- Cancel/timeout returns to `idle`.
- Success emits `success` + `ack` and exits deterministically.

Expected logs:
- Backend logs attempt count + timing.
- UI logs output retry and theme load failures.

