use crate::run_options::config::CustomCommand;

fn hash_string(s: impl AsRef<str>) -> String {
    use blake2::{Blake2b512, Digest};

    let mut hasher = Blake2b512::new();
    hasher.update(s.as_ref().as_bytes());
    let res = hasher.finalize();

    hex::encode(res).to_ascii_lowercase()
}

/// All the information needed for API calls to be made to run a command
#[derive(Clone, Debug)]
pub struct RoutableCommand {
    pub label: String,
    pub url_endpoint: String,
    pub run_cmd: Vec<Vec<String>>,
    pub stdin_allow: bool,
    pub stdin_placeholder_text: String,
    pub stdin_is_password: bool,
}

fn endpoint_from_custom_command(cmd: &CustomCommand) -> String {
    cmd.url_endpoint
        .clone()
        .unwrap_or_else(|| hash_string(cmd.label.to_string() + &cmd.run_cmd.as_string()))
}

impl From<CustomCommand> for RoutableCommand {
    fn from(cmd: CustomCommand) -> Self {
        RoutableCommand {
            url_endpoint: endpoint_from_custom_command(&cmd),
            label: cmd.label,
            run_cmd: cmd.run_cmd.take_commands(),
            stdin_allow: cmd.stdin_allow,
            stdin_placeholder_text: cmd.stdin_placeholder_text,
            stdin_is_password: cmd.stdin_is_password,
        }
    }
}
