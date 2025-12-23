# PAM Contract

TISS keeps PAM handling strict and theme-agnostic. The UI must not parse PAM
strings to infer flow types.

## Message vs Prompt

- PAM_TEXT_INFO / PAM_ERROR_MSG -> `messageReceived(kind, message)`
- Response-required prompts -> `promptReceived(id, kind, message, echo)`

Only `promptReceived` requires a response. `messageReceived` is informational.

## Typed Errors

The backend maps PAM failures into stable, typed error codes:

- `auth_failed`: wrong password or generic failure.
- `account_locked`: `pam_faillock` or account lock.
- `password_expired`: expired or change-required tokens.
- `pam_error`: unknown or unsupported flows.

## Multi-step Flows

OTP and password-change flows appear as multiple prompt events:

1) OTP prompt -> respond.
2) New password prompt -> respond.
3) Confirmation prompt -> respond.

Themes must support multiple prompt rounds without resetting state.

## Distro Policy

Supported targets: Arch, Fedora, Debian.  
Other distributions are best-effort; unknown PAM responses degrade to
`pam_error` without breaking the UI contract.

