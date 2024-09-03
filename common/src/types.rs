use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const HELLO_RESPONSE: &str = "WelcomeToTheUltimateUnlocker!";

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetMountedResponse {
    pub dataset_name: String,
    pub is_mounted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyLoadedResponse {
    pub dataset_name: String,
    pub key_loaded: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetList {
    pub datasets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetsMountState {
    pub datasets_mounted: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DatasetFullMountState {
    pub dataset_name: String,
    pub key_loaded: bool,
    pub is_mounted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetsFullMountState {
    pub states: BTreeMap<String, DatasetFullMountState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetBody {
    pub dataset_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RunCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub error_code: i32,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct AvailableCustomCommands {
    pub commands: Vec<CustomCommandPublicInfo>,
}

/// The response about a custom command when commands are queried
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct CustomCommandPublicInfo {
    pub label: String,
    pub endpoint: String,
    pub stdin_allow: bool,
    pub stdin_text_placeholder: String,
    pub stdin_is_password: bool,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct CustomCommandRunOptions {
    pub stdin: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct HelloResponse {
    pub result: String,
}

impl Default for HelloResponse {
    fn default() -> Self {
        Self {
            result: HELLO_RESPONSE.to_string(),
        }
    }
}
