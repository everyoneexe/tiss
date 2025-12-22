use ii_greetd_config::Config;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::process::CommandExt;

fn main() {
    if let Err(err) = run() {
        eprintln!("ii-greetd-launcher: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = load_config();
    let session_json_explicit = !env_missing("II_GREETD_SESSION_JSON");
    apply_config_env(&config)?;
    configure_sessions(session_json_explicit);
    ensure_seat_backend(&config);
    ensure_log_dir();
    ensure_cache_env();
    ensure_backend_path();
    ensure_qml_path();

    let cage_bin = resolve_cage_bin(&config)?;
    let ui_bin = resolve_ui_bin()?;
    let cage_args = build_cage_args(&config);

    let err = Command::new(&cage_bin)
        .args(&cage_args)
        .arg("--")
        .arg(&ui_bin)
        .exec();

    Err(format!(
        "failed to exec cage {}: {}",
        cage_bin.display(),
        err
    ))
}

#[derive(Debug, Serialize)]
struct SessionEntry {
    id: String,
    name: String,
    exec: Vec<String>,
    #[serde(rename = "type")]
    session_type: String,
    desktop_file: String,
}

#[derive(Debug, Deserialize)]
struct PersistedState {
    #[serde(default)]
    last_session_id: Option<String>,
}

fn load_config() -> Config {
    let mut config = Config::default();
    let system_path = Path::new("/etc/ii-greetd/config.toml");
    if system_path.exists() {
        match Config::load_from_path(system_path) {
            Ok(cfg) => config = config.merge(cfg),
            Err(err) => eprintln!(
                "ii-greetd-launcher: failed to read {}: {}",
                system_path.display(),
                err
            ),
        }
    }

    if let Some(home) = env::var_os("HOME") {
        let user_path = PathBuf::from(home).join(".config/ii-greetd/config.toml");
        if user_path.exists() {
            match Config::load_from_path(&user_path) {
                Ok(cfg) => config = config.merge(cfg),
                Err(err) => eprintln!(
                    "ii-greetd-launcher: failed to read {}: {}",
                    user_path.display(),
                    err
                ),
            }
        }
    }

    config
}

fn env_missing(key: &str) -> bool {
    match env::var(key) {
        Ok(value) => value.trim().is_empty(),
        Err(_) => true,
    }
}

fn set_env_if_missing(key: &str, value: Option<String>) {
    if !env_missing(key) {
        return;
    }
    if let Some(value) = value {
        if !value.trim().is_empty() {
            env::set_var(key, value);
        }
    }
}

fn apply_config_env(config: &Config) -> Result<(), String> {
    set_env_if_missing(
        "II_GREETD_BACKEND",
        config
            .paths
            .backend
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
    );
    set_env_if_missing(
        "II_GREETD_QML_FILE",
        config
            .paths
            .qml_file
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
    );
    set_env_if_missing("II_GREETD_QML_URI", config.paths.qml_uri.clone());
    set_env_if_missing(
        "II_GREETD_THEME_DIR",
        config
            .paths
            .theme_dir
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
    );
    set_env_if_missing("II_GREETD_THEME", config.paths.theme.clone());

    set_env_if_missing("II_GREETD_DEFAULT_USER", config.login.default_user.clone());
    set_env_if_missing(
        "II_GREETD_LOCK_USER",
        config
            .login
            .lock_user
            .map(|value| if value { "1".to_string() } else { "0".to_string() }),
    );

    if env_missing("II_GREETD_SESSION_JSON") && !config.session.command.is_empty() {
        let json = serde_json::to_string(&config.session.command)
            .map_err(|err| format!("invalid session.command: {}", err))?;
        env::set_var("II_GREETD_SESSION_JSON", json);
    }

    if env_missing("II_GREETD_SESSION_ENV_JSON") && !config.session.env.is_empty() {
        let json = serde_json::to_string(&config.session.env)
            .map_err(|err| format!("invalid session.env: {}", err))?;
        env::set_var("II_GREETD_SESSION_ENV_JSON", json);
    }

    set_env_if_missing(
        "II_GREETD_LOG_DIR",
        config
            .logging
            .dir
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
    );
    set_env_if_missing("II_GREETD_LOG_LEVEL", config.logging.level.clone());

    set_env_if_missing(
        "II_GREETD_SHOW_PASSWORD_TOGGLE",
        config
            .ui
            .show_password_toggle
            .map(|value| if value { "1".to_string() } else { "0".to_string() }),
    );

    Ok(())
}

fn configure_sessions(session_json_explicit: bool) {
    let sessions = discover_sessions();
    if let Ok(json) = serde_json::to_string(&sessions) {
        set_env_if_missing("II_GREETD_SESSIONS_JSON", Some(json));
    } else {
        eprintln!("ii-greetd-launcher: failed to serialize session list");
    }

    let last_session = load_last_session_id();
    if let Some(last_session_id) = last_session.as_ref() {
        if sessions.iter().any(|session| session.id == *last_session_id) {
            env::set_var("II_GREETD_LAST_SESSION_ID", last_session_id);
            if !session_json_explicit {
                if let Some(session) = sessions.iter().find(|session| session.id == *last_session_id) {
                    if let Ok(json) = serde_json::to_string(&session.exec) {
                        env::set_var("II_GREETD_SESSION_JSON", json);
                    }
                }
            }
        }
    }
}

fn discover_sessions() -> Vec<SessionEntry> {
    let mut sessions = Vec::new();
    sessions.extend(scan_sessions_dir("/usr/share/wayland-sessions", "wayland"));
    sessions.extend(scan_sessions_dir("/usr/share/xsessions", "x11"));
    sessions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    sessions
}

fn scan_sessions_dir(dir: &str, session_type: &str) -> Vec<SessionEntry> {
    let mut sessions = Vec::new();
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return sessions,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
            continue;
        }
        if let Some(session) = parse_desktop_entry(&path, session_type) {
            sessions.push(session);
        }
    }
    sessions
}

fn parse_desktop_entry(path: &Path, session_type: &str) -> Option<SessionEntry> {
    let content = fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut hidden = false;
    let mut nodisplay = false;
    let mut try_exec = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let (key, value) = match line.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "Name" => name = Some(value.to_string()),
            "Exec" => exec = Some(value.to_string()),
            "Hidden" => hidden = parse_bool(value),
            "NoDisplay" => nodisplay = parse_bool(value),
            "TryExec" => try_exec = Some(value.to_string()),
            _ => {}
        }
    }

    if hidden || nodisplay {
        return None;
    }

    let exec = exec?;
    if let Some(try_exec) = try_exec {
        let token = parse_exec(&try_exec).into_iter().next();
        if let Some(token) = token {
            if !try_exec_exists(&token) {
                return None;
            }
        }
    }

    let argv = parse_exec(&exec);
    if argv.is_empty() {
        return None;
    }

    let id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();
    let name = name.unwrap_or_else(|| id.clone());

    Some(SessionEntry {
        id,
        name,
        exec: argv,
        session_type: session_type.to_string(),
        desktop_file: path.to_string_lossy().to_string(),
    })
}

fn parse_bool(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes")
}

fn parse_exec(raw: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;

    for ch in raw.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' && !in_single {
            escape = true;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if ch.is_whitespace() && !in_single && !in_double {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            continue;
        }
        current.push(ch);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
        .into_iter()
        .filter_map(|token| {
            let cleaned = strip_field_codes(&token);
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
        .collect()
}

fn strip_field_codes(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.peek() {
                Some('%') => {
                    out.push('%');
                    chars.next();
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn try_exec_exists(token: &str) -> bool {
    if token.contains('/') {
        return is_executable(Path::new(token));
    }
    find_executable(token).is_some()
}

fn load_last_session_id() -> Option<String> {
    let path = state_path();
    let content = fs::read_to_string(path).ok()?;
    let state: PersistedState = serde_json::from_str(&content).ok()?;
    state.last_session_id.and_then(|id| {
        if id.trim().is_empty() {
            None
        } else {
            Some(id)
        }
    })
}

fn state_path() -> PathBuf {
    if let Ok(path) = env::var("XDG_STATE_HOME") {
        if !path.trim().is_empty() {
            return PathBuf::from(path).join("ii-greetd/state.json");
        }
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".local/state/ii-greetd/state.json");
    }
    PathBuf::from("/tmp/ii-greetd-state.json")
}

fn ensure_seat_backend(config: &Config) {
    if !env_missing("LIBSEAT_BACKEND") {
        return;
    }

    if let Some(backend) = config.seat.backend.as_ref() {
        if !backend.trim().is_empty() {
            env::set_var("LIBSEAT_BACKEND", backend);
            return;
        }
    }

    let seatd_sock = Path::new("/run/seatd.sock");
    let backend = match fs::metadata(seatd_sock) {
        Ok(meta) if meta.file_type().is_socket() => "seatd",
        _ => "logind",
    };
    env::set_var("LIBSEAT_BACKEND", backend);
}

fn ensure_log_dir() {
    if env_missing("II_GREETD_LOG_DIR") {
        env::set_var("II_GREETD_LOG_DIR", default_log_dir());
    }

    let current = env::var("II_GREETD_LOG_DIR").unwrap_or_else(|_| "/tmp/ii-greetd".to_string());
    if fs::create_dir_all(&current).is_err() {
        env::set_var("II_GREETD_LOG_DIR", "/tmp/ii-greetd");
        let _ = fs::create_dir_all("/tmp/ii-greetd");
    }
}

fn ensure_cache_env() {
    if env_missing("QML_DISABLE_DISK_CACHE") {
        env::set_var("QML_DISABLE_DISK_CACHE", "1");
    }

    if env_missing("XDG_CACHE_HOME") {
        let path = format!("/tmp/ii-greetd-cache-{}", uid_string());
        env::set_var("XDG_CACHE_HOME", &path);
    }

    if let Ok(cache_home) = env::var("XDG_CACHE_HOME") {
        let _ = fs::create_dir_all(&cache_home);
        if env_missing("MESA_SHADER_CACHE_DIR") {
            let mesa = Path::new(&cache_home).join("mesa");
            env::set_var("MESA_SHADER_CACHE_DIR", mesa.to_string_lossy().to_string());
        }
    }

    if let Ok(mesa) = env::var("MESA_SHADER_CACHE_DIR") {
        let _ = fs::create_dir_all(mesa);
    }
}

fn default_log_dir() -> String {
    format!("/tmp/ii-greetd-{}", uid_string())
}

fn uid_string() -> String {
    unsafe { libc::geteuid().to_string() }
}

fn ensure_backend_path() {
    if !env_missing("II_GREETD_BACKEND") {
        return;
    }

    let candidates = [
        "/usr/lib/ii-greetd/ii-greetd-backend",
        "/usr/local/lib/ii-greetd/ii-greetd-backend",
    ];
    for candidate in candidates {
        let path = Path::new(candidate);
        if is_executable(path) {
            env::set_var("II_GREETD_BACKEND", candidate);
            return;
        }
    }

    if let Some(path) = find_executable("ii-greetd-backend") {
        env::set_var("II_GREETD_BACKEND", path.to_string_lossy().to_string());
    }
}

fn ensure_qml_path() {
    if !env_missing("II_GREETD_QML_FILE") {
        return;
    }

    let candidates = [
        "/usr/share/ii-greetd/qml/Main.qml",
        "/usr/local/share/ii-greetd/qml/Main.qml",
    ];
    for candidate in candidates {
        let path = Path::new(candidate);
        if path.exists() {
            env::set_var("II_GREETD_QML_FILE", candidate);
            return;
        }
    }
}

fn resolve_cage_bin(config: &Config) -> Result<PathBuf, String> {
    if let Ok(path) = env::var("II_GREETD_CAGE_BIN") {
        if !path.trim().is_empty() {
            let path = PathBuf::from(path);
            if is_executable(&path) {
                return Ok(path);
            }
            return Err("cage not found (II_GREETD_CAGE_BIN)".to_string());
        }
    }

    if let Some(path) = config.seat.cage_bin.as_ref() {
        if is_executable(path) {
            return Ok(path.clone());
        }
        return Err("cage not found (seat.cage_bin)".to_string());
    }

    if let Some(path) = find_executable("cage") {
        return Ok(path);
    }

    Err("cage not found".to_string())
}

fn resolve_ui_bin() -> Result<PathBuf, String> {
    if let Ok(path) = env::var("II_GREETD_UI_BIN") {
        if !path.trim().is_empty() {
            let path = PathBuf::from(path);
            if is_executable(&path) {
                return Ok(path);
            }
            return Err("ii-greetd-ui not found (II_GREETD_UI_BIN)".to_string());
        }
    }

    if let Some(path) = find_executable("ii-greetd-ui") {
        return Ok(path);
    }

    Err("ii-greetd-ui not found".to_string())
}

fn build_cage_args(config: &Config) -> Vec<String> {
    let mut args = vec!["-s".to_string()];
    if let Ok(raw) = env::var("II_GREETD_CAGE_ARGS") {
        let extra = split_args(&raw);
        if !extra.is_empty() {
            args.extend(extra);
            return args;
        }
    }
    if !config.seat.cage_args.is_empty() {
        args.extend(config.seat.cage_args.iter().cloned());
    }
    args
}

fn split_args(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(|part| part.to_string()).collect()
}

fn find_executable(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let path = PathBuf::from(name);
        if is_executable(&path) {
            return Some(path);
        }
        return None;
    }

    let path_var = env::var("PATH").ok()?;
    for dir in path_var.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = Path::new(dir).join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    if let Ok(meta) = fs::metadata(path) {
        if !meta.is_file() {
            return false;
        }
        let mode = meta.permissions().mode();
        return mode & 0o111 != 0;
    }
    false
}
