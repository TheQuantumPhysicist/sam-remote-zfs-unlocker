use std::{path::Path, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize};

#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiServerConfig {
    pub custom_commands: Vec<CustomCommand>,
}

impl ApiServerConfig {
    pub fn from_file(
        path: impl AsRef<Path>,
    ) -> Result<ApiServerConfig, Box<dyn std::error::Error>> {
        let config_content = std::fs::read_to_string(path)?;
        Self::from_str(&config_content)
    }

    pub fn custom_commands(&self) -> &[CustomCommand] {
        &self.custom_commands
    }
}

impl FromStr for ApiServerConfig {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config: ApiServerConfig = toml::from_str(s)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    /// The label of the command, to show in the UI
    pub label: String,
    /// The url endpoint, which will be used for the URL. If left empty, it will be automatically generated.
    #[serde(default, deserialize_with = "validate_url_endpoint")]
    pub url_endpoint: Option<String>,
    /// Command to run to activate something
    pub run_cmd: Vec<String>,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

// Custom deserialization function to validate the label field
fn validate_url_endpoint<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Deserialize::deserialize(deserializer)?;

    if let Some(ref label) = s {
        // Define a set of valid characters
        let is_valid = label.chars().all(|c| {
            (c.is_lowercase() && c.is_alphabetic())
                || c.is_numeric()
                || c == '-'
                || c == '_'
                || c == '.'
        });

        if !is_valid {
            return Err(serde::de::Error::custom(format!(
            "Invalid label: '{}'. Labels must only contain alphanumeric characters, hyphens, underscores, and periods.",
            label
        )));
        }
    }

    Ok(s)
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::ApiServerConfig;

    #[test]
    fn basic() {
        let _config = ApiServerConfig::from_file("api-config.toml").unwrap();
        println!("{_config:?}");
    }
}
