use std::{path::Path, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MockSettings {
    // Dataset name, password, probability of failure
    pub datasets_and_passwords: Option<Vec<(String, String, f32)>>,
    #[allow(clippy::type_complexity)]
    #[serde(rename = "custom_command")]
    pub custom_commands: Option<Vec<MockedCustomCommandConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveSettings {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "lowercase")]
pub enum LiveOrMock {
    Live(LiveSettings),
    Mock(MockSettings),
}

#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
        let config: WebPageConfig = toml::from_str(s)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MockedCustomCommandConfig {
    pub unique_label: String,
    pub expected_stdout: String,
    pub expected_stderr: String,
    pub expected_error_code: i32,
    pub stdin: MockedCustomCommandStdinConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum MockedCustomCommandStdinConfig {
    Simple(bool),
    AllSettings(MockedCustomCommandStdinSettings),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MockedCustomCommandStdinSettings {
    pub allow: bool,
    pub placeholder: String,
    #[serde(default = "default_true")]
    pub is_password: bool,
}

fn default_true() -> bool {
    true
}

impl MockedCustomCommandStdinConfig {
    pub fn is_stdin_enabled(&self) -> bool {
        match self {
            MockedCustomCommandStdinConfig::Simple(b) => *b,
            MockedCustomCommandStdinConfig::AllSettings(s) => s.allow,
        }
    }

    pub fn stdin_placeholder_text(&self) -> String {
        match self {
            MockedCustomCommandStdinConfig::Simple(_) => "".to_string(),
            MockedCustomCommandStdinConfig::AllSettings(s) => s.placeholder.to_string(),
        }
    }

    pub fn is_password(&self) -> bool {
        match self {
            MockedCustomCommandStdinConfig::Simple(b) => *b,
            MockedCustomCommandStdinConfig::AllSettings(s) => s.is_password,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_file() {
        // println!("{}", std::env::current_dir().unwrap().display());
        let _config = WebPageConfig::from_file("../frontend/public/web.toml").unwrap();
        // println!("{_config:?}");
        // println!("{}", toml::to_string_pretty(&_config).unwrap());
    }
}
