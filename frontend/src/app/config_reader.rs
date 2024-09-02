use std::str::FromStr;

use common::config::WebPageConfig;

use super::log;

const CONFIG_URL: &str = "/public/app-config.toml";

fn is_html(file: &str) -> bool {
    file.trim().starts_with("<!DOCTYPE html>") || file.to_lowercase().trim().starts_with("<html>")
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum ConfigurationLoadError {
    #[error("Configuration retrieval error. Configuration is expected to be found in path {1}. Error: {0}")]
    Retrieval(String, String),
    #[error("Failed to get configuration file as text to parse. Error: {0}")]
    TextRetrieval(String),
    #[error("Config file parse error. Make sure the config file is in the path: {1}. Error: {0}")]
    FileParse(String, String),
    #[error("Config file not found in path: {0}")]
    NotFoundInPath(String),
}

pub async fn retrieve_config() -> Result<WebPageConfig, ConfigurationLoadError> {
    let url = CONFIG_URL;

    log(&format!("Retrieving config from URL: {url}"));

    let config_file = reqwasm::http::Request::get(url)
        .send()
        .await
        .map_err(|e| ConfigurationLoadError::Retrieval(e.to_string(), url.to_string()))?
        .text()
        .await
        .map_err(|e| ConfigurationLoadError::TextRetrieval(e.to_string()))?;

    let webpage_config = WebPageConfig::from_str(&config_file).map_err(|e| {
        if is_html(&config_file) {
            // If the file is an HTML file, this means that the file is not found, because `trunk` returns
            // an HTML page with error if the config file is not found.
            ConfigurationLoadError::NotFoundInPath(CONFIG_URL.to_string())
        } else {
            ConfigurationLoadError::FileParse(url.to_string(), e.to_string())
        }
    })?;

    log("Done retrieving config");

    Ok(webpage_config)
}
