use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum UiRequest {
    #[serde(rename = "hello")]
    Hello { ui_version: u32 },
    #[serde(rename = "auth")]
    Auth {
        username: String,
        password: String,
        #[serde(default)]
        command: Vec<String>,
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
    },
    #[serde(rename = "start")]
    Start { command: Vec<String>, #[serde(default)] env: std::collections::HashMap<String, String> },
    #[serde(rename = "power")]
    Power { action: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum BackendResponse {
    #[serde(rename = "state")]
    State { phase: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "success")]
    Success,
}
