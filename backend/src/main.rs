use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::time::{Duration, Instant};

mod greetd;
mod logging;
mod protocol;

fn default_command(log: &mut logging::Logger) -> Vec<String> {
    if let Ok(cmd_json) = env::var("TISS_GREETD_SESSION_JSON") {
        if let Ok(cmd) = serde_json::from_str::<Vec<String>>(&cmd_json) {
            if !cmd.is_empty() {
                return cmd;
            }
        } else {
            log.log("invalid TISS_GREETD_SESSION_JSON; falling back to default command");
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

fn auth_timeout(log: &mut logging::Logger) -> Option<Duration> {
    let value = env::var("TISS_GREETD_AUTH_TIMEOUT_SECS").unwrap_or_default();
    if value.trim().is_empty() {
        return None;
    }
    match value.trim().parse::<u64>() {
        Ok(0) => None,
        Ok(secs) => Some(Duration::from_secs(secs)),
        Err(err) => {
            log.log(&format!(
                "invalid TISS_GREETD_AUTH_TIMEOUT_SECS='{}': {}",
                value, err
            ));
            None
        }
    }
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

fn set_phase(out: &mut dyn Write, current: &Cell<&'static str>, phase: &'static str) -> Result<()> {
    current.set(phase);
    send_response(
        out,
        protocol::BackendResponse::State {
            phase: phase.to_string(),
        },
    )
}

#[derive(Debug, Deserialize)]
struct SessionListEntry {
    id: String,
    #[serde(default)]
    exec: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileEntry {
    id: String,
    #[serde(default)]
    session: String,
    #[serde(default)]
    env: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PersistedState {
    #[serde(default)]
    last_session_id: Option<String>,
    #[serde(default)]
    last_profile_id: Option<String>,
    #[serde(default)]
    last_locale: Option<String>,
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
        greetd::AuthMessageType::Info | greetd::AuthMessageType::Error => {
            unreachable!("info/error prompts do not require responses")
        }
    }
}

fn message_kind(kind: greetd::AuthMessageType) -> &'static str {
    match kind {
        greetd::AuthMessageType::Info => "info",
        greetd::AuthMessageType::Error => "error",
        greetd::AuthMessageType::Visible | greetd::AuthMessageType::Secret => {
            unreachable!("visible/secret messages require responses")
        }
    }
}

fn wait_prompt_response(
    prompt_id: u64,
    reader: &mut dyn BufRead,
    stdin_fd: std::os::unix::io::RawFd,
    out: &mut dyn Write,
    timeout: Option<Duration>,
) -> greetd::AuthResult<Option<String>> {
    let start = Instant::now();
    loop {
        if let Some(timeout) = timeout {
            let elapsed = start.elapsed();
            let remaining = timeout.saturating_sub(elapsed);
            if remaining.is_zero() {
                return Err(greetd::AuthError::timeout());
            }
            if !poll_readable(stdin_fd, remaining).map_err(greetd::AuthError::from)? {
                return Err(greetd::AuthError::timeout());
            }
        }
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
                return Err(greetd::AuthError::cancelled());
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

fn poll_readable(fd: std::os::unix::io::RawFd, timeout: Duration) -> Result<bool> {
    let millis = timeout
        .as_millis()
        .min(i32::MAX as u128)
        .try_into()
        .unwrap_or(i32::MAX);
    let mut pollfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let res = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, millis) };
    if res < 0 {
        Err(std::io::Error::last_os_error()).context("poll stdin")?
    }
    Ok(res > 0)
}

fn state_path() -> std::path::PathBuf {
    if let Ok(path) = env::var("XDG_STATE_HOME") {
        if !path.trim().is_empty() {
            return std::path::PathBuf::from(path).join("tiss-greetd/state.json");
        }
    }
    std::path::PathBuf::from("/var/lib/tiss-greetd/state.json")
}

fn load_sessions(log: &mut logging::Logger) -> HashMap<String, Vec<String>> {
    let raw = env::var("TISS_GREETD_SESSIONS_JSON").unwrap_or_default();
    if raw.trim().is_empty() {
        return HashMap::new();
    }
    let entries: Vec<SessionListEntry> = match serde_json::from_str(&raw) {
        Ok(entries) => entries,
        Err(err) => {
            log.log(&format!("invalid TISS_GREETD_SESSIONS_JSON: {}", err));
            return HashMap::new();
        }
    };
    let mut sessions = HashMap::new();
    for entry in entries {
        if entry.id.trim().is_empty() || entry.exec.is_empty() {
            continue;
        }
        sessions.insert(entry.id, entry.exec);
    }
    sessions
}

fn load_profiles(log: &mut logging::Logger) -> HashMap<String, ProfileEntry> {
    let raw = env::var("TISS_GREETD_PROFILES_JSON").unwrap_or_default();
    if raw.trim().is_empty() {
        return HashMap::new();
    }
    let entries: Vec<ProfileEntry> = match serde_json::from_str(&raw) {
        Ok(entries) => entries,
        Err(err) => {
            log.log(&format!("invalid TISS_GREETD_PROFILES_JSON: {}", err));
            return HashMap::new();
        }
    };
    let mut profiles = HashMap::new();
    for entry in entries {
        if entry.id.trim().is_empty() {
            continue;
        }
        profiles.insert(entry.id.clone(), entry);
    }
    profiles
}

fn load_power_actions(log: &mut logging::Logger) -> HashSet<String> {
    let raw = env::var("TISS_GREETD_POWER_ACTIONS_JSON").unwrap_or_default();
    if raw.trim().is_empty() {
        return HashSet::new();
    }
    let entries: Vec<String> = match serde_json::from_str(&raw) {
        Ok(entries) => entries,
        Err(err) => {
            log.log(&format!("invalid TISS_GREETD_POWER_ACTIONS_JSON: {}", err));
            return HashSet::new();
        }
    };
    entries
        .into_iter()
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn load_power_allowed_states(log: &mut logging::Logger) -> HashSet<String> {
    let raw = env::var("TISS_GREETD_POWER_ALLOWED_STATES_JSON").unwrap_or_default();
    if raw.trim().is_empty() {
        return ["idle".to_string()].into_iter().collect();
    }
    let entries: Vec<String> = match serde_json::from_str(&raw) {
        Ok(entries) => entries,
        Err(err) => {
            log.log(&format!("invalid TISS_GREETD_POWER_ALLOWED_STATES_JSON: {}", err));
            return ["idle".to_string()].into_iter().collect();
        }
    };
    entries
        .into_iter()
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn read_state(log: &mut logging::Logger) -> PersistedState {
    let path = state_path();
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return PersistedState::default(),
    };
    let mut state: PersistedState = match serde_json::from_str(&content) {
        Ok(state) => state,
        Err(err) => {
            log.log(&format!("failed to parse state {}: {}", path.display(), err));
            return PersistedState::default();
        }
    };
    state.last_session_id = state.last_session_id.filter(|value| !value.trim().is_empty());
    state.last_profile_id = state.last_profile_id.filter(|value| !value.trim().is_empty());
    state.last_locale = state.last_locale.filter(|value| !value.trim().is_empty());
    state
}

fn write_state(state: &PersistedState, log: &mut logging::Logger) {
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
    match serde_json::to_vec(state) {
        Ok(payload) => {
            let tmp_path = path.with_extension("json.tmp");
            match File::create(&tmp_path) {
                Ok(mut file) => {
                    if let Err(err) = file.write_all(&payload) {
                        log.log(&format!("failed to write state {}: {}", tmp_path.display(), err));
                        return;
                    }
                    if let Err(err) = file.sync_all() {
                        log.log(&format!("failed to fsync state {}: {}", tmp_path.display(), err));
                        return;
                    }
                    if let Err(err) = fs::rename(&tmp_path, &path) {
                        log.log(&format!("failed to replace state {}: {}", path.display(), err));
                    }
                }
                Err(err) => {
                    log.log(&format!("failed to create state {}: {}", tmp_path.display(), err));
                }
            }
        }
        Err(err) => {
            log.log(&format!("failed to serialize state: {}", err));
        }
    }
}

fn wait_for_ack(reader: &mut dyn BufRead, log: &mut logging::Logger) -> Result<()> {
    loop {
        let line = match read_line(reader)? {
            Some(line) => line,
            None => {
                log.log("ui disconnected while waiting for success ack");
                return Ok(());
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let req = match parse_request(&line) {
            Ok(req) => req,
            Err(err) => {
                log.log(&format!("invalid json while waiting for ack: {}", err));
                continue;
            }
        };
        match req {
            protocol::UiRequest::Ack { kind } => {
                if kind == "success" {
                    log.log("received success ack");
                    return Ok(());
                }
                log.log(&format!("unexpected ack kind: {}", kind));
            }
            _ => {
                log.log("ignoring request while waiting for success ack");
            }
        }
    }
}

fn persist_state_update(
    session_id: Option<&str>,
    profile_id: Option<&str>,
    locale: Option<&str>,
    log: &mut logging::Logger,
) {
    if session_id.is_none() && profile_id.is_none() && locale.is_none() {
        return;
    }
    let mut state = read_state(log);
    if let Some(session_id) = session_id.map(str::trim).filter(|value| !value.is_empty()) {
        state.last_session_id = Some(session_id.to_string());
    }
    if let Some(profile_id) = profile_id.map(str::trim).filter(|value| !value.is_empty()) {
        state.last_profile_id = Some(profile_id.to_string());
    }
    if let Some(locale) = locale.map(str::trim).filter(|value| !value.is_empty()) {
        state.last_locale = Some(locale.to_string());
    }
    write_state(&state, log);
}

fn power_error_code(message: &str) -> &'static str {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("accessdenied")
        || lowered.contains("notauthorized")
        || lowered.contains("not authorized")
        || lowered.contains("permission")
        || lowered.contains("polkit")
    {
        "power_denied"
    } else {
        "power_error"
    }
}

fn request_power_action(action: &str) -> std::result::Result<(), String> {
    let conn = zbus::blocking::Connection::system().map_err(|err| err.to_string())?;
    let proxy = zbus::blocking::Proxy::new(
        &conn,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .map_err(|err| err.to_string())?;
    match action {
        "poweroff" => proxy.call("PowerOff", &(false)).map_err(|err| err.to_string()),
        "reboot" => proxy.call("Reboot", &(false)).map_err(|err| err.to_string()),
        "suspend" => proxy.call("Suspend", &(false)).map_err(|err| err.to_string()),
        _ => Err(format!("unknown power action: {}", action)),
    }
}

fn main() -> Result<()> {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdin_fd = stdin.as_raw_fd();
    let stdout = Rc::new(RefCell::new(io::stdout()));
    let mut log = logging::Logger::new("backend");
    let current_phase = Cell::new("idle");

    log.log("backend start");
    let sessions = load_sessions(&mut log);
    let profiles = load_profiles(&mut log);
    let power_actions = load_power_actions(&mut log);
    let power_allowed_states = load_power_allowed_states(&mut log);
    let auth_timeout = auth_timeout(&mut log);
    let mut auth_attempts: u64 = 0;
    set_phase(&mut *stdout.borrow_mut(), &current_phase, "idle")?;

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
                set_phase(&mut *stdout.borrow_mut(), &current_phase, "idle")?;
            }
            protocol::UiRequest::Auth {
                username,
                command,
                env,
                session_id,
                profile_id,
                locale,
            } => {
                auth_attempts += 1;
                let auth_started = Instant::now();
                log.log(&format!(
                    "request: auth attempt={} user={}",
                    auth_attempts, username
                ));
                set_phase(&mut *stdout.borrow_mut(), &current_phase, "auth")?;
                let username = username.trim().to_string();
                if username.is_empty() {
                    set_phase(&mut *stdout.borrow_mut(), &current_phase, "error")?;
                    send_error(
                        &mut *stdout.borrow_mut(),
                        "pam_error",
                        "username is required",
                    )?;
                    continue;
                }
                let session_id = session_id.and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });
                let profile_id = profile_id.and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });
                let locale = locale.and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });
                let mut effective_session_id = session_id.clone();
                let profile = profile_id.as_ref().and_then(|id| profiles.get(id));
                if effective_session_id.is_none() {
                    if let Some(profile) = profile {
                        let value = profile.session.trim();
                        if !value.is_empty() {
                            effective_session_id = Some(value.to_string());
                        }
                    }
                }
                let mut cmd = if !command.is_empty() {
                    command
                } else if let Some(id) = effective_session_id.as_ref() {
                    sessions.get(id).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                };
                if cmd.is_empty() {
                    cmd = default_command(&mut log);
                }

                let mut env_map = env;
                if let Some(profile) = profile {
                    for (key, value) in profile.env.iter() {
                        env_map.insert(key.clone(), value.clone());
                    }
                }
                if let Some(locale) = locale.as_ref() {
                    env_map.insert("LANG".to_string(), locale.clone());
                    env_map.insert("LC_ALL".to_string(), locale.clone());
                }
                let env_vec = build_env(env_map);
                let mut prompt_id = 0u64;
                let stdout_for_prompt = Rc::clone(&stdout);
                let stdout_for_wait = Rc::clone(&stdout);
                let mut prompt_handler =
                    |kind: greetd::AuthMessageType,
                     message: &str|
                     -> greetd::AuthResult<Option<String>> {
                        match kind {
                            greetd::AuthMessageType::Visible | greetd::AuthMessageType::Secret => {
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
                                wait_prompt_response(
                                    prompt_id,
                                    &mut stdin_lock,
                                    stdin_fd,
                                    &mut *out,
                                    auth_timeout,
                                )
                            }
                            greetd::AuthMessageType::Info | greetd::AuthMessageType::Error => {
                                let kind_str = message_kind(kind);
                                let mut out = stdout_for_prompt.borrow_mut();
                                send_response(
                                    &mut *out,
                                    protocol::BackendResponse::Message {
                                        kind: kind_str.to_string(),
                                        message: message.to_string(),
                                    },
                                )?;
                                Ok(None)
                            }
                        }
                    };
                let mut on_waiting = || {
                    let _ =
                        set_phase(&mut *stdout_for_wait.borrow_mut(), &current_phase, "waiting");
                };
                match greetd::authenticate_and_start(
                    &username,
                    &cmd,
                    &env_vec,
                    &mut log,
                    &mut prompt_handler,
                    &mut on_waiting,
                ) {
                    Ok(()) => {
                        log.log("auth success; start_session ok");
                        persist_state_update(
                            effective_session_id.as_deref(),
                            profile_id.as_deref(),
                            locale.as_deref(),
                            &mut log,
                        );
                        set_phase(&mut *stdout.borrow_mut(), &current_phase, "success")?;
                        send_response(&mut *stdout.borrow_mut(), protocol::BackendResponse::Success)?;
                        wait_for_ack(&mut stdin_lock, &mut log)?;
                        log.log(&format!(
                            "auth attempt={} success in {}ms",
                            auth_attempts,
                            auth_started.elapsed().as_millis()
                        ));
                        return Ok(());
                    }
                    Err(err) => {
                        log.log(&format!("auth failed: {}", err));
                        log.log(&format!(
                            "auth attempt={} failed in {}ms",
                            auth_attempts,
                            auth_started.elapsed().as_millis()
                        ));
                        if err.return_to_idle() {
                            send_error(
                                &mut *stdout.borrow_mut(),
                                err.code().as_str(),
                                err.message(),
                            )?;
                            set_phase(&mut *stdout.borrow_mut(), &current_phase, "idle")?;
                        } else {
                            set_phase(&mut *stdout.borrow_mut(), &current_phase, "error")?;
                            send_error(
                                &mut *stdout.borrow_mut(),
                                err.code().as_str(),
                                err.message(),
                            )?;
                        }
                    }
                }
            }
            protocol::UiRequest::Start { command, env } => {
                log.log(&format!(
                    "request: start {:?} env_len={}",
                    command,
                    env.len()
                ));
                set_phase(&mut *stdout.borrow_mut(), &current_phase, "waiting")?;
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
            protocol::UiRequest::Ack { .. } => {
                send_error(&mut *stdout.borrow_mut(), "pam_error", "unexpected ack")?;
            }
            protocol::UiRequest::Power { action } => {
                let action = action.trim().to_ascii_lowercase();
                log.log(&format!("request: power {}", action));
                if action.is_empty() {
                    send_error(&mut *stdout.borrow_mut(), "power_error", "power action missing")?;
                    continue;
                }
                if !power_allowed_states.contains(current_phase.get()) {
                    send_error(
                        &mut *stdout.borrow_mut(),
                        "power_denied",
                        format!("power action not allowed during {}", current_phase.get()),
                    )?;
                    continue;
                }
                if !power_actions.contains(&action) {
                    send_error(
                        &mut *stdout.borrow_mut(),
                        "power_denied",
                        format!("power action not allowed: {}", action),
                    )?;
                    continue;
                }
                match request_power_action(&action) {
                    Ok(()) => {
                        log.log(&format!("power action dispatched: {}", action));
                    }
                    Err(err) => {
                        let code = power_error_code(&err);
                        send_error(&mut *stdout.borrow_mut(), code, err)?;
                    }
                }
            }
        }
    }

    Ok(())
}
