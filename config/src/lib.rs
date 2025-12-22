use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub login: Login,
    #[serde(default)]
    pub session: Session,
    #[serde(default)]
    pub sessions: Vec<SessionEntry>,
    #[serde(default)]
    pub logging: Logging,
    #[serde(default)]
    pub seat: Seat,
    #[serde(default)]
    pub ui: Ui,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Paths {
    pub backend: Option<PathBuf>,
    pub qml_file: Option<PathBuf>,
    pub qml_uri: Option<String>,
    pub theme_dir: Option<PathBuf>,
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Login {
    pub default_user: Option<String>,
    pub lock_user: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Session {
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SessionEntry {
    pub name: String,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Logging {
    pub dir: Option<PathBuf>,
    pub level: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Seat {
    pub backend: Option<String>,
    pub cage_bin: Option<PathBuf>,
    #[serde(default)]
    pub cage_args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Ui {
    pub show_password_toggle: Option<bool>,
}

impl Config {
    pub fn merge(self, other: Config) -> Config {
        Config {
            paths: self.paths.merge(other.paths),
            login: self.login.merge(other.login),
            session: self.session.merge(other.session),
            sessions: if other.sessions.is_empty() { self.sessions } else { other.sessions },
            logging: self.logging.merge(other.logging),
            seat: self.seat.merge(other.seat),
            ui: self.ui.merge(other.ui),
        }
    }

    pub fn load_from_path(path: &Path) -> Result<Config, String> {
        let content = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
        toml::from_str(&content).map_err(|err| err.to_string())
    }
}

impl Paths {
    fn merge(self, other: Paths) -> Paths {
        Paths {
            backend: other.backend.or(self.backend),
            qml_file: other.qml_file.or(self.qml_file),
            qml_uri: other.qml_uri.or(self.qml_uri),
            theme_dir: other.theme_dir.or(self.theme_dir),
            theme: other.theme.or(self.theme),
        }
    }
}

impl Login {
    fn merge(self, other: Login) -> Login {
        Login {
            default_user: other.default_user.or(self.default_user),
            lock_user: other.lock_user.or(self.lock_user),
        }
    }
}

impl Session {
    fn merge(self, other: Session) -> Session {
        let mut env = self.env;
        env.extend(other.env);
        let command = if other.command.is_empty() {
            self.command
        } else {
            other.command
        };
        Session { command, env }
    }
}

impl Logging {
    fn merge(self, other: Logging) -> Logging {
        Logging {
            dir: other.dir.or(self.dir),
            level: other.level.or(self.level),
        }
    }
}

impl Seat {
    fn merge(self, other: Seat) -> Seat {
        Seat {
            backend: other.backend.or(self.backend),
            cage_bin: other.cage_bin.or(self.cage_bin),
            cage_args: if other.cage_args.is_empty() { self.cage_args } else { other.cage_args },
        }
    }
}

impl Ui {
    fn merge(self, other: Ui) -> Ui {
        Ui {
            show_password_toggle: other.show_password_toggle.or(self.show_password_toggle),
        }
    }
}
