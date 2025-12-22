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
}

impl AuthError {
    fn auth_failure(kind: AuthFailureKind, detail: &str) -> Self {
        AuthError {
            code: kind.code(),
            message: format_auth_error(kind, detail),
        }
    }

    pub fn pam_error(message: impl Into<String>) -> Self {
        AuthError {
            code: AuthErrorCode::PamError,
            message: message.into(),
        }
    }

    pub fn code(&self) -> AuthErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
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

fn write_request(stream: &mut UnixStream, req: &Request) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(req).context("serialize greetd request")?;
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
    let _ = write_request(stream, &Request::CancelSession);
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
        &Request::CreateSession {
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
                        &Request::PostAuthMessageResponse { response },
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
                        &Request::PostAuthMessageResponse {
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
        &Request::StartSession {
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
    let mut haystack = String::new();
    haystack.push_str(description);
    haystack.push(' ');
    haystack.push_str(detail);
    let haystack = haystack.to_lowercase();

    if haystack.contains("acct_expired")
        || haystack.contains("authtok_expired")
        || haystack.contains("new_authtok_reqd")
        || haystack.contains("expired")
    {
        return AuthFailureKind::Expired;
    }

    if haystack.contains("account locked")
        || haystack.contains("locked")
        || haystack.contains("maxtries")
        || haystack.contains("perm_denied")
    {
        return AuthFailureKind::AccountLocked;
    }

    AuthFailureKind::AuthErr
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
