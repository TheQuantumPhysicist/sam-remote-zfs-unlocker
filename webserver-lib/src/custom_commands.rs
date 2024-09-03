use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use common::types::CustomCommandRunOptions;
use tokio::sync::Mutex;

use crate::{
    backend::traits::ExecutionBackend, state::ServerState, StateType, CUSTOM_COMMANDS_DIR,
};

async fn route_handler_from_command<B: ExecutionBackend>(
    State(state): State<Arc<Mutex<ServerState<B>>>>,
    json_body: Option<Json<CustomCommandRunOptions>>,
    url_endpoint: String,
) -> Result<impl IntoResponse, B::Error> {
    let state = &*state.lock().await;

    let cmd = state
        .backend
        .custom_cmds_routables()
        .get(&url_endpoint)
        .unwrap();

    let stdin = json_body.and_then(|b| b.stdin.clone());

    let result = state
        .backend
        .custom_cmd_call(&cmd.url_endpoint, stdin)
        .await?;

    Ok(Json::from(result))
}

fn route_from_command<B: ExecutionBackend>(
    router: Router<StateType<B>>,
    url_endpoint: impl Into<String>,
) -> Router<StateType<B>> {
    let url_endpoint = url_endpoint.into();

    router.route(
        &format!("/{}", url_endpoint),
        post(move |state, json| route_handler_from_command(state, json, url_endpoint)),
    )
}

pub async fn custom_commands_list_route_handler<B: ExecutionBackend>(
    State(state): State<StateType<B>>,
) -> Result<impl IntoResponse, B::Error> {
    let state = &*state.lock().await;

    let result = state.backend.custom_cmds_list()?;

    Ok(Json::from(result))
}

pub fn make_custom_commands_routes<B: ExecutionBackend>(
    state: &ServerState<B>,
) -> Router<StateType<B>> {
    let inner_routes = state
        .backend
        .custom_cmds_routables()
        .values()
        .fold(Router::new(), |router, cmd| {
            route_from_command(router, &cmd.url_endpoint)
        });

    Router::new().nest(CUSTOM_COMMANDS_DIR, inner_routes)
}
