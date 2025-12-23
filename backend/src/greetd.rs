use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use crate::logging::Logger;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    CreateSession { username: String },
    PostAuthMessageResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<String>,
    },
    StartSession { cmd: Vec<String>, env: Vec<String> },
    CancelSession,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Success,
    Error { error_type: String, description: String },
    AuthMessage {
        auth_message_type: AuthMessageType,
        auth_message: String,
    },
}

#[derive(Debug, Deserialize, PartialEq, Eq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AuthMessageType {
    Visible,
    Secret,
    Info,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub enum AuthErrorCode {
    AuthFailed,
    AccountLocked,
    PasswordExpired,
    PamError,
}

impl AuthErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthErrorCode::AuthFailed => "auth_failed",
            AuthErrorCode::AccountLocked => "account_locked",
            AuthErrorCode::PasswordExpired => "password_expired",
            AuthErrorCode::PamError => "pam_error",
        }
    }
}

#[derive(Debug)]
pub struct AuthError {
    code: AuthErrorCode,
    message: String,
    return_to_idle: bool,
}

impl AuthError {
    fn auth_failure(kind: AuthFailureKind, detail: &str) -> Self {
        AuthError {
            code: kind.code(),
            message: format_auth_error(kind, detail),
            return_to_idle: false,
        }
    }

    pub fn pam_error(message: impl Into<String>) -> Self {
        AuthError {
            code: AuthErrorCode::PamError,
            message: message.into(),
            return_to_idle: false,
        }
    }

    pub fn cancelled() -> Self {
        AuthError {
            code: AuthErrorCode::PamError,
            message: "authentication cancelled".to_string(),
            return_to_idle: true,
        }
    }

    pub fn timeout() -> Self {
        AuthError {
            code: AuthErrorCode::PamError,
            message: "authentication timed out".to_string(),
            return_to_idle: true,
        }
    }

    pub fn code(&self) -> AuthErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn return_to_idle(&self) -> bool {
        self.return_to_idle
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AuthError {}

impl From<anyhow::Error> for AuthError {
    fn from(err: anyhow::Error) -> Self {
        AuthError::pam_error(err.to_string())
    }
}

pub type AuthResult<T> = std::result::Result<T, AuthError>;

fn write_request(stream: &mut UnixStream, mut req: Request) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(&req).context("serialize greetd request")?;
    zero_request(&mut req);
    let len = u32::try_from(payload.len()).context("payload too large")?;
    stream
        .write_all(&len.to_ne_bytes())
        .context("write greetd length")?;
    stream
        .write_all(&payload)
        .context("write greetd payload")?;
    stream.flush().ok();
    Ok(())
}

fn read_response(stream: &mut UnixStream) -> anyhow::Result<Response> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).context("read greetd length")?;
    let len = u32::from_ne_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    stream
        .read_exact(&mut payload)
        .context("read greetd payload")?;
    serde_json::from_slice(&payload).context("parse greetd response")
}

fn cancel_session(stream: &mut UnixStream) {
    let _ = write_request(stream, Request::CancelSession);
}

pub fn authenticate_and_start(
    username: &str,
    command: &[String],
    env: &[String],
    log: &mut Logger,
    mut prompt: impl FnMut(AuthMessageType, &str) -> AuthResult<Option<String>>,
    on_waiting: &mut dyn FnMut(),
) -> AuthResult<()> {
    let sock = std::env::var("GREETD_SOCK").context("GREETD_SOCK not set")?;
    let mut stream = UnixStream::connect(sock).context("connect greetd socket")?;

    log.log(&format!("create_session {}", username));
    write_request(
        &mut stream,
        Request::CreateSession {
            username: username.to_string(),
        },
    )?;

    let mut last_info: Option<String> = None;

    loop {
        match read_response(&mut stream)? {
            Response::Success => break,
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => match auth_message_type {
                AuthMessageType::Info | AuthMessageType::Error => {
                    let msg = auth_message.trim();
                    if !msg.is_empty() {
                        log.log(&format!("pam message: {:?}: {}", auth_message_type, msg));
                    } else {
                        log.log(&format!("pam message: {:?} (empty)", auth_message_type));
                    }
                    if !auth_message.trim().is_empty() {
                        last_info = Some(auth_message.clone());
                    }
                    let response = match prompt(auth_message_type, msg) {
                        Ok(resp) => resp,
                        Err(err) => {
                            cancel_session(&mut stream);
                            return Err(err);
                        }
                    };
                    write_request(
                        &mut stream,
                        Request::PostAuthMessageResponse { response },
                    )?;
                }
                AuthMessageType::Visible | AuthMessageType::Secret => {
                    let prompt_text = auth_message.trim();
                    if !prompt_text.is_empty() {
                        log.log(&format!("pam prompt: {:?}: {}", auth_message_type, prompt_text));
                    } else {
                        log.log(&format!("pam prompt: {:?} (empty)", auth_message_type));
                    }
                    let response = match prompt(auth_message_type, prompt_text) {
                        Ok(Some(resp)) => resp,
                        Ok(None) => {
                            cancel_session(&mut stream);
                            return Err(AuthError::pam_error("prompt response missing"));
                        }
                        Err(err) => {
                            cancel_session(&mut stream);
                            return Err(err);
                        }
                    };
                    write_request(
                        &mut stream,
                        Request::PostAuthMessageResponse {
                            response: Some(response),
                        },
                    )?;
                }
            },
            Response::Error {
                error_type,
                description,
            } => {
                log.log(&format!("greetd error: {} {}", error_type, description));
                cancel_session(&mut stream);
                let detail = last_info.unwrap_or_default();
                if error_type == "auth_error" {
                    let kind = classify_auth_failure(&description, &detail);
                    return Err(AuthError::auth_failure(kind, &detail));
                }
                let message = if detail.is_empty() {
                    format!("{}: {}", error_type, description)
                } else {
                    format!("{}: {} ({})", error_type, description, detail)
                };
                return Err(AuthError::pam_error(message));
            }
        }
    }

    log.log("start_session");
    on_waiting();
    write_request(
        &mut stream,
        Request::StartSession {
            cmd: command.to_vec(),
            env: env.to_vec(),
        },
    )?;

    match read_response(&mut stream)? {
        Response::Success => Ok(()),
        Response::Error {
            error_type,
            description,
        } => Err(AuthError::pam_error(format!("{}: {}", error_type, description))),
        Response::AuthMessage { auth_message, .. } => Err(AuthError::pam_error(format!(
            "unexpected auth message during start_session: {}",
            auth_message
        ))),
    }
}

#[derive(Debug, Clone, Copy)]
enum AuthFailureKind {
    AuthErr,
    AccountLocked,
    Expired,
}

impl AuthFailureKind {
    fn code(self) -> AuthErrorCode {
        match self {
            AuthFailureKind::AuthErr => AuthErrorCode::AuthFailed,
            AuthFailureKind::AccountLocked => AuthErrorCode::AccountLocked,
            AuthFailureKind::Expired => AuthErrorCode::PasswordExpired,
        }
    }
}

fn classify_auth_failure(description: &str, detail: &str) -> AuthFailureKind {
    let mut joined = String::new();
    joined.push_str(description);
    joined.push(' ');
    joined.push_str(detail);

    if let Some(token) = extract_pam_error_token(&joined) {
        if let Some(kind) = map_pam_token(token) {
            return kind;
        }
    }

    let lowered = joined.to_lowercase();
    if lowered.contains("account locked")
        || lowered.contains("too many failed")
        || lowered.contains("maximum number of retries")
        || lowered.contains("faillock")
    {
        return AuthFailureKind::AccountLocked;
    }

    if lowered.contains("password expired")
        || lowered.contains("authentication token is no longer valid")
        || lowered.contains("new password required")
        || lowered.contains("password change required")
    {
        return AuthFailureKind::Expired;
    }

    AuthFailureKind::AuthErr
}

fn extract_pam_error_token(haystack: &str) -> Option<&str> {
    haystack
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .find(|token| token.starts_with("PAM_"))
}

fn map_pam_token(token: &str) -> Option<AuthFailureKind> {
    match token {
        "PAM_ACCT_EXPIRED"
        | "PAM_CRED_EXPIRED"
        | "PAM_AUTHTOK_EXPIRED"
        | "PAM_NEW_AUTHTOK_REQD" => Some(AuthFailureKind::Expired),
        "PAM_MAXTRIES" => Some(AuthFailureKind::AccountLocked),
        "PAM_PERM_DENIED" => Some(AuthFailureKind::AccountLocked),
        "PAM_AUTH_ERR" | "PAM_USER_UNKNOWN" | "PAM_CRED_INSUFFICIENT" => {
            Some(AuthFailureKind::AuthErr)
        }
        _ => None,
    }
}

fn zero_request(req: &mut Request) {
    if let Request::PostAuthMessageResponse { response } = req {
        if let Some(ref mut value) = response {
            zero_string(value);
        }
    }
}

fn zero_string(value: &mut String) {
    unsafe {
        value.as_mut_vec().fill(0);
    }
}

fn format_auth_error(kind: AuthFailureKind, detail: &str) -> String {
    let suffix = detail.trim();
    let message = match kind {
        AuthFailureKind::AuthErr => "Authentication failed",
        AuthFailureKind::AccountLocked => "Account locked or disabled",
        AuthFailureKind::Expired => "Account or password expired",
    };

    if suffix.is_empty() {
        message.to_string()
    } else {
        format!("{} ({})", message, suffix)
    }
}
