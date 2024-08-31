use std::str::FromStr;

use common::config::WebPageConfig;

use super::log;

const CONFIG_URL: &str = "/public/web.toml";

#[derive(thiserror::Error, Debug, Clone)]
pub enum ConfigurationLoadError {
    #[error("Configuration retrieval error. Configuration is expected to be found in path {1}. Error: {0}")]
    Retrieval(String, String),
    #[error("Failed to get configuration file as text to parse. Error: {0}")]
    TextRetrieval(String),
    #[error("Config file parse error. Make sure the config file is in the path: {1}. Error: {0}")]
    FileParse(String, String),
}

pub async fn retrieve_config() -> Result<WebPageConfig, ConfigurationLoadError> {
    let url = CONFIG_URL;

    log(&format!("Retrieving config from URL: {url}"));

    let config_file = reqwasm::http::Request::get(url)
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        .send()
        .await
        .map_err(|e| ConfigurationLoadError::Retrieval(e.to_string(), url.to_string()))?
        .text()
        .await
        .map_err(|e| ConfigurationLoadError::TextRetrieval(e.to_string()))?;

    let webpage_config = WebPageConfig::from_str(&config_file)
        .map_err(|e| ConfigurationLoadError::FileParse(url.to_string(), e.to_string()))?;

    log("Done retrieving config");

    Ok(webpage_config)
}
