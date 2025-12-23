use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Logger {
    file: Option<std::fs::File>,
    component: String,
}

impl Logger {
    pub fn new(component: &str) -> Self {
        let component = component.to_string();
        let dir = default_log_dir();
        let file = fs::create_dir_all(&dir)
            .ok()
            .and_then(|_| OpenOptions::new().create(true).append(true).open(dir.join(format!("tiss-greetd-{}.log", component))).ok());

        if file.is_none() {
            eprintln!("tiss-greetd-{}: failed to open log file", component);
        }

        Logger { file, component }
    }

    pub fn log(&mut self, message: &str) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(file) = self.file.as_mut() {
            let _ = writeln!(file, "[{}] {}: {}", ts, self.component, message);
            let _ = file.flush();
        } else {
            eprintln!("[{}] {}: {}", ts, self.component, message);
        }
    }
}

fn default_log_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TISS_GREETD_LOG_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }

    if let Some(uid) = read_uid_from_proc() {
        return PathBuf::from(format!("/tmp/tiss-greetd-{}", uid));
    }

    if let Ok(user) = std::env::var("USER") {
        if !user.trim().is_empty() {
            return PathBuf::from(format!("/tmp/tiss-greetd-{}", user));
        }
    }

    PathBuf::from("/tmp/tiss-greetd")
}

fn read_uid_from_proc() -> Option<String> {
    let content = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let uid = rest.split_whitespace().next()?;
            return Some(uid.to_string());
        }
    }
    None
}
