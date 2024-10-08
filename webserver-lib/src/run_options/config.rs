use std::{collections::BTreeSet, path::Path, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize};

#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiServerConfig {
    #[serde(flatten)]
    pub custom_commands_config: CustomCommandsConfig,

    #[serde(flatten)]
    pub zfs_config: ZfsConfig,
}

impl ApiServerConfig {
    pub fn from_file(
        path: impl AsRef<Path>,
    ) -> Result<ApiServerConfig, Box<dyn std::error::Error>> {
        let config_content = std::fs::read_to_string(path)?;
        Self::from_str(&config_content)
    }

    pub fn custom_commands(&self) -> Option<&[CustomCommand]> {
        self.custom_commands_config.custom_commands.as_deref()
    }
}

impl FromStr for ApiServerConfig {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config: ApiServerConfig = toml::from_str(s)?;
        Ok(config)
    }
}

#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomCommandsConfig {
    #[serde(
        default,
        deserialize_with = "validate_commands_list",
        rename = "custom_command"
    )]
    pub custom_commands: Option<Vec<CustomCommand>>,
}

#[allow(clippy::derivable_impls)]
impl Default for CustomCommandsConfig {
    fn default() -> Self {
        Self {
            custom_commands: None,
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZfsConfig {
    #[serde(default = "default_zfs_enabled")]
    /// If disabled, listing ZFS datasets will return an empty set, and no operations will work
    pub zfs_enabled: bool,

    #[serde(default)]
    /// ZFS datasets that won't be reachable with the API
    pub blacklisted_zfs_datasets: Option<Vec<String>>,
}

impl Default for ZfsConfig {
    fn default() -> Self {
        Self {
            zfs_enabled: default_zfs_enabled(),
            blacklisted_zfs_datasets: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomCommand {
    /// The label of the command, to show in the UI
    pub label: String,
    /// The url endpoint, which will be used for the URL. If left empty, it will be automatically generated.
    #[serde(default, deserialize_with = "validate_url_endpoint")]
    pub url_endpoint: Option<String>,
    /// Commands to run to activate something. Multiple commands will result in commands being executed in order,
    /// and every command's result get piped to the next command.
    /// Foe example, `systemctl status docker | grep Active` becomes `[["systemctl","status","docker"],["grep","Active"]]`
    pub run_cmd: SingleOrChainedCommands,
    /// Whether to enable piping some input string into the command
    #[serde(default)]
    pub stdin_allow: bool,
    /// The definition of the text to be input in stdin... something like "Email address", or "Password", etc.
    #[serde(default)]
    pub stdin_placeholder_text: String,
    /// The definition of the text to be input in stdin... something like "Email address", or "Password", etc.
    #[serde(default = "default_true")]
    pub stdin_is_password: bool,

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum SingleOrChainedCommands {
    Single(Vec<String>),
    Chained(Vec<Vec<String>>),
}

impl SingleOrChainedCommands {
    pub fn commands(&self) -> Vec<Vec<String>> {
        match self {
            SingleOrChainedCommands::Single(cmd) => vec![cmd.clone()],
            SingleOrChainedCommands::Chained(cmds) => cmds.clone(),
        }
    }

    pub fn take_commands(self) -> Vec<Vec<String>> {
        match self {
            SingleOrChainedCommands::Single(cmd) => vec![cmd],
            SingleOrChainedCommands::Chained(cmds) => cmds,
        }
    }

    pub fn as_string(&self) -> String {
        self.commands()
            .iter()
            .map(|v| v.join(" "))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

// Custom deserialization function to validate the label field
fn validate_commands_list<'de, D>(deserializer: D) -> Result<Option<Vec<CustomCommand>>, D::Error>
where
    D: Deserializer<'de>,
{
    let cmds: Option<Vec<CustomCommand>> = Deserialize::deserialize(deserializer)?;

    let cmds = match cmds {
        Some(cmds) => cmds,
        None => return Ok(None),
    };

    // Find duplicates in commands
    {
        let mut seen: BTreeSet<Vec<Vec<String>>> = BTreeSet::new();
        for item in cmds.iter().filter(|cmd| cmd.enabled) {
            if !seen.insert(item.run_cmd.commands().clone()) {
                return Err(serde::de::Error::custom(format!(
                    "Failed to load config. Item with command `{}`, as a duplicate was found",
                    &item.run_cmd.as_string()
                )));
            }
        }
    }

    // Find duplicates in endpoints
    {
        let mut seen = BTreeSet::new();
        for endpoint in cmds
            .iter()
            .filter(|cmd| cmd.enabled)
            .filter_map(|cmd| cmd.url_endpoint.as_ref())
        {
            if !seen.insert(endpoint) {
                return Err(serde::de::Error::custom(format!(
                    "Failed to load config. Item with url_endpoint `{}`, as a duplicate was found",
                    endpoint
                )));
            }
        }
    }

    Ok(Some(cmds))
}

fn default_true() -> bool {
    true
}

pub fn default_zfs_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::ApiServerConfig;

    #[test]
    fn basic() {
        let _config = ApiServerConfig::from_file("api-config.toml").unwrap();
        // println!("{_config:?}");
        // println!("{}", toml::to_string_pretty(&_config).unwrap());
    }
}
