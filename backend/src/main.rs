use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, Write};

mod greetd;
mod logging;
mod protocol;

fn default_command() -> Vec<String> {
    if let Ok(cmd) = std::env::var("II_GREETD_SESSION_CMD") {
        let parts: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
        if !parts.is_empty() {
            return parts;
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

fn main() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut log = logging::Logger::new("backend");

    log.log("backend start");
    send_response(&mut stdout, protocol::BackendResponse::State { phase: "idle".into() })?;

    for line in stdin.lock().lines() {
        let line = line.context("read line")?;
        if line.trim().is_empty() {
            continue;
        }

        let req: protocol::UiRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(err) => {
                let _ = send_response(
                    &mut stdout,
                    protocol::BackendResponse::Error { message: format!("invalid json: {}", err) },
                );
                continue;
            }
        };

        match req {
            protocol::UiRequest::Hello { .. } => {
                log.log("request: hello");
                send_response(&mut stdout, protocol::BackendResponse::State { phase: "idle".into() })?;
            }
            protocol::UiRequest::Auth { username, password, command, env } => {
                log.log(&format!("request: auth user={}", username));
                send_response(&mut stdout, protocol::BackendResponse::State { phase: "authenticating".into() })?;
                let username = username.trim().to_string();
                if username.is_empty() {
                    send_response(&mut stdout, protocol::BackendResponse::State { phase: "failed".into() })?;
                    send_response(
                        &mut stdout,
                        protocol::BackendResponse::Error { message: "username is required".to_string() },
                    )?;
                    continue;
                }
                if password.is_empty() {
                    send_response(&mut stdout, protocol::BackendResponse::State { phase: "failed".into() })?;
                    send_response(
                        &mut stdout,
                        protocol::BackendResponse::Error { message: "password is required".to_string() },
                    )?;
                    continue;
                }
                let cmd = if command.is_empty() { default_command() } else { command };
                let env_vec = build_env(env);
                match greetd::authenticate_and_start(&username, &password, &cmd, &env_vec, &mut log) {
                    Ok(()) => {
                        log.log("auth success; start_session ok");
                        send_response(&mut stdout, protocol::BackendResponse::State { phase: "starting".into() })?;
                        send_response(&mut stdout, protocol::BackendResponse::Success)?;
                        return Ok(());
                    }
                    Err(err) => {
                        log.log(&format!("auth failed: {}", err));
                        send_response(&mut stdout, protocol::BackendResponse::State { phase: "failed".into() })?;
                        send_response(&mut stdout, protocol::BackendResponse::Error { message: err.to_string() })?;
                    }
                }
            }
            protocol::UiRequest::Start { command, env: _ } => {
                log.log(&format!("request: start {:?}", command));
                send_response(&mut stdout, protocol::BackendResponse::State { phase: "starting".into() })?;
                send_response(
                    &mut stdout,
                    protocol::BackendResponse::Error { message: format!("start not implemented: {:?}", command) },
                )?;
            }
            protocol::UiRequest::Power { action } => {
                log.log(&format!("request: power {}", action));
                send_response(&mut stdout, protocol::BackendResponse::Error { message: format!("power action not implemented: {}", action) })?;
            }
        }
    }

    Ok(())
}
