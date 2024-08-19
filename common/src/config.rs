use std::{path::Path, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSettings {
    pub datasets_and_passwords: Option<Vec<(String, String)>>,
    pub erring_datasets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSettings {
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiveOrMock {
    Live(LiveSettings),
    Mock(MockSettings),
}

#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPageConfig {
    pub mode: LiveOrMock,
}

impl WebPageConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<WebPageConfig, Box<dyn std::error::Error>> {
        let config_content = std::fs::read_to_string(path)?;
        Self::from_str(&config_content)
    }
}

impl FromStr for WebPageConfig {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config: WebPageConfig = toml::from_str(&s)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_file() {
        // println!("{}", std::env::current_dir().unwrap().display());
        let config = WebPageConfig::from_file("../frontend/web.toml").unwrap();
        println!("{config:?}");
    }
}
