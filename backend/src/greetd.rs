use anyhow::{anyhow, Context, Result};
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

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AuthMessageType {
    Visible,
    Secret,
    Info,
    Error,
}

fn write_request(stream: &mut UnixStream, req: &Request) -> Result<()> {
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

fn read_response(stream: &mut UnixStream) -> Result<Response> {
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
    password: &str,
    command: &[String],
    env: &[String],
    log: &mut Logger,
) -> Result<()> {
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
    let password = password.to_string();

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
                        last_info = Some(auth_message);
                    }
                    write_request(
                        &mut stream,
                        &Request::PostAuthMessageResponse { response: None },
                    )?;
                }
                AuthMessageType::Visible | AuthMessageType::Secret => {
                    let prompt = auth_message.trim().to_lowercase();
                    if !auth_message.trim().is_empty() {
                        log.log(&format!("pam prompt: {:?}: {}", auth_message_type, auth_message.trim()));
                    } else {
                        log.log(&format!("pam prompt: {:?} (empty)", auth_message_type));
                    }
                    if password.is_empty() {
                        cancel_session(&mut stream);
                        return Err(anyhow!("empty password"));
                    }
                    let response = if auth_message_type == AuthMessageType::Visible {
                        if prompt.contains("user") || prompt.contains("login") || prompt.contains("name") {
                            username.to_string()
                        } else if prompt.contains("pass") {
                            password.clone()
                        } else {
                            username.to_string()
                        }
                    } else {
                        password.clone()
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
                if detail.is_empty() {
                    return Err(anyhow!("{}: {}", error_type, description));
                }
                return Err(anyhow!("{}: {} ({})", error_type, description, detail));
            }
        }
    }

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
        } => Err(anyhow!("{}: {}", error_type, description)),
        Response::AuthMessage { auth_message, .. } => Err(anyhow!(
            "unexpected auth message during start_session: {}",
            auth_message
        )),
    }
}
