use anyhow::{Context, Result};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::rc::Rc;

mod greetd;
mod logging;
mod protocol;

fn default_command(log: &mut logging::Logger) -> Vec<String> {
    if let Ok(cmd_json) = env::var("II_GREETD_SESSION_JSON") {
        if let Ok(cmd) = serde_json::from_str::<Vec<String>>(&cmd_json) {
            if !cmd.is_empty() {
                return cmd;
            }
        } else {
            log.log("invalid II_GREETD_SESSION_JSON; falling back to default command");
        }
    }
    vec!["niri".to_string()]
}

fn default_env() -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
    env.insert("XDG_SESSION_CLASS".to_string(), "user".to_string());
    env.insert("XDG_CURRENT_DESKTOP".to_string(), "niri".to_string());
    env.insert("XDG_SESSION_DESKTOP".to_string(), "niri".to_string());
    env
}

fn build_env(overrides: HashMap<String, String>) -> Vec<String> {
    let mut env_map = default_env();
    for (key, value) in overrides {
        if key.trim().is_empty() {
            continue;
        }
        env_map.insert(key, value);
    }
    env_map
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect()
}

fn send_response(out: &mut dyn Write, resp: protocol::BackendResponse) -> Result<()> {
    let line = serde_json::to_string(&resp).context("serialize response")?;
    writeln!(out, "{}", line).context("write response")?;
    out.flush().ok();
    Ok(())
}

fn send_error(out: &mut dyn Write, code: &str, message: impl Into<String>) -> Result<()> {
    send_response(
        out,
        protocol::BackendResponse::Error {
            code: code.to_string(),
            message: message.into(),
        },
    )
}

#[derive(Debug, Serialize)]
struct PersistedState {
    last_session_id: String,
}

fn read_line(reader: &mut dyn BufRead) -> Result<Option<String>> {
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).context("read line")?;
    if bytes == 0 {
        return Ok(None);
    }
    Ok(Some(line))
}

fn parse_request(line: &str) -> Result<protocol::UiRequest> {
    serde_json::from_str(line).context("parse request json")
}

fn prompt_kind(kind: greetd::AuthMessageType) -> (&'static str, bool) {
    match kind {
        greetd::AuthMessageType::Visible => ("visible", true),
        greetd::AuthMessageType::Secret => ("secret", false),
        greetd::AuthMessageType::Info => ("info", true),
        greetd::AuthMessageType::Error => ("error", true),
    }
}

fn wait_prompt_response(
    prompt_id: u64,
    reader: &mut dyn BufRead,
    out: &mut dyn Write,
) -> greetd::AuthResult<Option<String>> {
    loop {
        let line = match read_line(reader)? {
            Some(line) => line,
            None => return Err(greetd::AuthError::pam_error("ui disconnected during auth")),
        };
        if line.trim().is_empty() {
            continue;
        }
        let req = match parse_request(&line) {
            Ok(req) => req,
            Err(err) => {
                let _ = send_error(out, "pam_error", format!("invalid json: {}", err));
                continue;
            }
        };
        match req {
            protocol::UiRequest::PromptResponse { id, response } => {
                if id == prompt_id {
                    return Ok(response);
                }
                let _ = send_error(out, "pam_error", format!("unexpected prompt id: {}", id));
            }
            protocol::UiRequest::Cancel => {
                return Err(greetd::AuthError::pam_error("auth cancelled"));
            }
            protocol::UiRequest::Hello { .. } => {
                continue;
            }
            _ => {
                let _ = send_error(out, "pam_error", "auth in progress");
            }
        }
    }
}

fn state_path() -> std::path::PathBuf {
    if let Ok(path) = env::var("XDG_STATE_HOME") {
        if !path.trim().is_empty() {
            return std::path::PathBuf::from(path).join("ii-greetd/state.json");
        }
    }
    if let Ok(home) = env::var("HOME") {
        if !home.trim().is_empty() {
            return std::path::PathBuf::from(home).join(".local/state/ii-greetd/state.json");
        }
    }
    std::path::PathBuf::from("/tmp/ii-greetd-state.json")
}

fn persist_last_session(session_id: &str, log: &mut logging::Logger) {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return;
    }
    let path = state_path();
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            log.log(&format!(
                "failed to create state dir {}: {}",
                parent.display(),
                err
            ));
            return;
        }
    }
    let state = PersistedState {
        last_session_id: session_id.to_string(),
    };
    match serde_json::to_vec(&state) {
        Ok(payload) => {
            if let Err(err) = fs::write(&path, payload) {
                log.log(&format!("failed to write state {}: {}", path.display(), err));
            }
        }
        Err(err) => {
            log.log(&format!("failed to serialize state: {}", err));
        }
    }
}

fn main() -> Result<()> {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = Rc::new(RefCell::new(io::stdout()));
    let mut log = logging::Logger::new("backend");

    log.log("backend start");
    send_response(
        &mut *stdout.borrow_mut(),
        protocol::BackendResponse::State { phase: "idle".into() },
    )?;

    loop {
        let line = match read_line(&mut stdin_lock)? {
            Some(line) => line,
            None => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let req: protocol::UiRequest = match parse_request(&line) {
            Ok(req) => req,
            Err(err) => {
                let _ = send_error(&mut *stdout.borrow_mut(), "pam_error", format!("invalid json: {}", err));
                continue;
            }
        };

        match req {
            protocol::UiRequest::Hello { ui_version } => {
                log.log(&format!("request: hello ui_version={}", ui_version));
                send_response(
                    &mut *stdout.borrow_mut(),
                    protocol::BackendResponse::State { phase: "idle".into() },
                )?;
            }
            protocol::UiRequest::Auth {
                username,
                command,
                env,
                session_id,
            } => {
                log.log(&format!("request: auth user={}", username));
                send_response(
                    &mut *stdout.borrow_mut(),
                    protocol::BackendResponse::State { phase: "auth".into() },
                )?;
                let username = username.trim().to_string();
                if username.is_empty() {
                    send_response(
                        &mut *stdout.borrow_mut(),
                        protocol::BackendResponse::State { phase: "error".into() },
                    )?;
                    send_error(
                        &mut *stdout.borrow_mut(),
                        "pam_error",
                        "username is required",
                    )?;
                    continue;
                }
                let cmd = if command.is_empty() { default_command(&mut log) } else { command };
                let env_vec = build_env(env);
                let mut prompt_id = 0u64;
                let stdout_for_prompt = Rc::clone(&stdout);
                let stdout_for_wait = Rc::clone(&stdout);
                let mut prompt_handler = |kind: greetd::AuthMessageType, message: &str| -> greetd::AuthResult<Option<String>> {
                    prompt_id += 1;
                    let (kind_str, echo) = prompt_kind(kind);
                    {
                        let mut out = stdout_for_prompt.borrow_mut();
                        send_response(
                            &mut *out,
                            protocol::BackendResponse::Prompt {
                                id: prompt_id,
                                kind: kind_str.to_string(),
                                message: message.to_string(),
                                echo,
                            },
                        )?;
                    }
                    let mut out = stdout_for_prompt.borrow_mut();
                    wait_prompt_response(prompt_id, &mut stdin_lock, &mut *out)
                };
                let mut on_waiting = || {
                    let _ = send_response(
                        &mut *stdout_for_wait.borrow_mut(),
                        protocol::BackendResponse::State { phase: "waiting".into() },
                    );
                };
                match greetd::authenticate_and_start(&username, &cmd, &env_vec, &mut log, &mut prompt_handler, &mut on_waiting) {
                    Ok(()) => {
                        log.log("auth success; start_session ok");
                        if let Some(session_id) = session_id.as_ref() {
                            persist_last_session(session_id, &mut log);
                        }
                        send_response(
                            &mut *stdout.borrow_mut(),
                            protocol::BackendResponse::State { phase: "success".into() },
                        )?;
                        send_response(&mut *stdout.borrow_mut(), protocol::BackendResponse::Success)?;
                        return Ok(());
                    }
                    Err(err) => {
                        log.log(&format!("auth failed: {}", err));
                        send_response(
                            &mut *stdout.borrow_mut(),
                            protocol::BackendResponse::State { phase: "error".into() },
                        )?;
                        send_error(
                            &mut *stdout.borrow_mut(),
                            err.code().as_str(),
                            err.message(),
                        )?;
                    }
                }
            }
            protocol::UiRequest::Start { command, env } => {
                log.log(&format!(
                    "request: start {:?} env_len={}",
                    command,
                    env.len()
                ));
                send_response(
                    &mut *stdout.borrow_mut(),
                    protocol::BackendResponse::State { phase: "waiting".into() },
                )?;
                send_error(
                    &mut *stdout.borrow_mut(),
                    "pam_error",
                    format!("start not implemented: {:?}", command),
                )?;
            }
            protocol::UiRequest::PromptResponse { .. } => {
                send_error(&mut *stdout.borrow_mut(), "pam_error", "no active prompt")?;
            }
            protocol::UiRequest::Cancel => {
                send_error(
                    &mut *stdout.borrow_mut(),
                    "pam_error",
                    "no active auth session",
                )?;
            }
            protocol::UiRequest::Power { action } => {
                log.log(&format!("request: power {}", action));
                send_error(
                    &mut *stdout.borrow_mut(),
                    "pam_error",
                    format!("power action not implemented: {}", action),
                )?;
            }
        }
    }

    Ok(())
}
