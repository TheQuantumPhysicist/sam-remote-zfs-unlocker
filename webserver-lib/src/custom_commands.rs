use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use common::types::{AvailableCustomCommands, CustomCommandInfo, CustomCommandRunOptions};
use tokio::sync::Mutex;

use crate::{
    run_options::config::CustomCommand, state::ServerState, Error, StateType, CUSTOM_COMMANDS_DIR,
};

async fn route_handler_from_command(
    State(_state): State<Arc<Mutex<ServerState>>>,
    json_body: Option<Json<CustomCommandRunOptions>>,
    cmd: RoutableCommand,
) -> Result<impl IntoResponse, Error> {
    let stdin = json_body.map(|b| b.stdin.clone()).flatten();
    let result = crate::command_caller::run_command(&cmd.run_cmd, stdin)
        .await
        .map_err(|e| Error::CommandExecution(e.to_string()))?;

    Ok(Json::from(result))
}

fn route_from_command(router: Router<StateType>, cmd: &RoutableCommand) -> Router<StateType> {
    let cmd = cmd.clone();
    router.route(
        &format!("/{}", cmd.url_endpoint),
        post(move |state, json| route_handler_from_command(state, json, cmd)),
    )
}

pub async fn custom_commands_list_route_handler(
    State(_state): State<StateType>,
    cmds: Vec<RoutableCommand>,
) -> Result<impl IntoResponse, Error> {
    let commands = cmds
        .iter()
        .map(|c| CustomCommandInfo {
            label: c.label.to_string(),
            endpoint: c.url_endpoint.to_string(),
            allow_stdin: c.allow_stdin,
        })
        .collect::<Vec<_>>();

    let result = AvailableCustomCommands { commands };

    Ok(Json::from(result))
}

pub fn routes_from_config(cmds: Vec<RoutableCommand>) -> Router<StateType> {
    let inner_routes = cmds.iter().fold(Router::new(), route_from_command);

    Router::new().nest(CUSTOM_COMMANDS_DIR, inner_routes)
}

fn hash_string(s: impl AsRef<str>) -> String {
    use blake2::{Blake2b512, Digest};

    let mut hasher = Blake2b512::new();
    hasher.update(s.as_ref().as_bytes());
    let res = hasher.finalize();

    hex::encode(res).to_ascii_lowercase()
}

#[derive(Clone, Debug)]
pub struct RoutableCommand {
    pub label: String,
    pub url_endpoint: String,
    pub run_cmd: Vec<String>,
    pub allow_stdin: bool,
}

fn endpoint_from_custom_command(cmd: &CustomCommand) -> String {
    cmd.url_endpoint
        .clone()
        .unwrap_or_else(|| hash_string(cmd.label.to_string() + &cmd.run_cmd.join(" ")))
}

impl From<CustomCommand> for RoutableCommand {
    fn from(cmd: CustomCommand) -> Self {
        RoutableCommand {
            url_endpoint: endpoint_from_custom_command(&cmd),
            label: cmd.label,
            run_cmd: cmd.run_cmd,
            allow_stdin: cmd.allow_stdin,
        }
    }
}

pub fn commands_to_routables(cmds: Vec<CustomCommand>) -> Vec<RoutableCommand> {
    cmds.into_iter()
        .filter(|cmd| cmd.enabled)
        .map(Into::into)
        .collect()
}
