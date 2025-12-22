use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum UiRequest {
    #[serde(rename = "hello")]
    Hello { ui_version: u32 },
    #[serde(rename = "auth")]
    Auth {
        username: String,
        #[serde(default)]
        command: Vec<String>,
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        profile_id: Option<String>,
        #[serde(default)]
        locale: Option<String>,
    },
    #[serde(rename = "prompt_response")]
    PromptResponse {
        id: u64,
        #[serde(default)]
        response: Option<String>,
    },
    #[serde(rename = "cancel")]
    Cancel,
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
    #[serde(rename = "prompt")]
    Prompt {
        id: u64,
        kind: String,
        message: String,
        echo: bool,
    },
    #[serde(rename = "error")]
    Error { code: String, message: String },
    #[serde(rename = "success")]
    Success,
}
