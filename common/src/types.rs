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

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetsMountState {
    pub datasets_mounted: BTreeMap<String, bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetFullMountState {
    pub dataset_name: String,
    pub key_loaded: bool,
    pub is_mounted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetsFullMountState {
    pub states: BTreeMap<String, DatasetFullMountState>,
}
