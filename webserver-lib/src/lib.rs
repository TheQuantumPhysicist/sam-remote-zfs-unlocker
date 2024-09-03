mod backend;
mod custom_commands;
pub mod run_options;
pub mod state;
mod zfs;

use std::sync::Arc;

use axum::{
    response::IntoResponse,
    routing::{get, IntoMakeService},
    serve::Serve,
    Json, Router,
};
use backend::error::Error;
use backend::{live::LiveExecutionBackend, traits::ExecutionBackend};
use common::types::HelloResponse;
use custom_commands::{custom_commands_list_route_handler, make_custom_commands_routes};
use hyper::{Method, StatusCode};
use run_options::{config::ApiServerConfig, server_run_options::ServerRunOptions};
use state::ServerState;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http_axum::cors::{AllowMethods, CorsLayer};
use zfs::zfs_routes;

type StateType<B> = Arc<Mutex<ServerState<B>>>;

const ZFS_DIR: &str = "/zfs";
const CUSTOM_COMMANDS_DIR: &str = "/custom-commands";
const CUSTOM_COMMANDS_LIST_ENDPOINT: &str = "/custom-commands-list";

async fn handler_404() -> impl IntoResponse {
    (StatusCode::BAD_REQUEST, "Bad request")
}

async fn hello() -> Result<impl IntoResponse, Error> {
    Ok(Json::from(HelloResponse::default()))
}

fn web_server<B: ExecutionBackend>(
    socket: TcpListener,
    config: Option<ApiServerConfig>,
    backend: B,
) -> Serve<IntoMakeService<Router>, Router> {
    let cors_layer = CorsLayer::new()
        .allow_methods(AllowMethods::list([Method::GET, Method::POST]))
        .allow_headers(tower_http_axum::cors::Any)
        .allow_origin(tower_http_axum::cors::Any);

    let (zfs_config, custom_cmds_config) = config
        .map(|c| (c.zfs_config, c.custom_commands_config))
        .unwrap_or_default();

    let state = ServerState::new(zfs_config, custom_cmds_config.clone(), backend);

    let custom_cmds_routes = make_custom_commands_routes(&state).route(
        CUSTOM_COMMANDS_LIST_ENDPOINT,
        get(custom_commands_list_route_handler),
    );

    let state = Arc::new(Mutex::new(state));

    let routes = Router::new()
        .route("/hello", get(hello))
        .merge(zfs_routes())
        .merge(custom_cmds_routes)
        .with_state(state)
        .layer(cors_layer)
        .layer(tower_http_axum::trace::TraceLayer::new_for_http())
        .fallback(handler_404);

    axum::serve(socket, routes.into_make_service())
}

pub async fn start_server(options: ServerRunOptions) -> Result<(), Box<dyn std::error::Error>> {
    let bind_address = options.bind_address();
    let listener_socket = TcpListener::bind(bind_address).await?;

    let config = ApiServerConfig::from_file(options.config_path())?;

    log::info!("Server socket binding to {}", bind_address);

    web_server(
        listener_socket,
        Some(config.clone()),
        LiveExecutionBackend::new(config),
    )
    .await
    .map_err(Into::into)
}
