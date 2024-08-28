use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RunCommandOutput {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AvailableCustomCommands {
    pub commands: Vec<CustomCommandInfo>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct CustomCommandInfo {
    pub label: String,
    pub endpoint: String,
}
