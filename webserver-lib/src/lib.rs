mod command_caller;
mod custom_commands;
pub mod run_options;
pub mod state;
mod zfs;

use std::sync::Arc;

use axum::{
    response::{IntoResponse, Response},
    routing::{get, IntoMakeService},
    serve::Serve,
    Json, Router,
};
use common::types::HelloResponse;
use custom_commands::{
    commands_to_routables, custom_commands_list_route_handler, routes_from_config,
};
use hyper::{Method, StatusCode};
use run_options::{config::ApiServerConfig, server_run_options::ServerRunOptions};
use sam_zfs_unlocker::ZfsError;
use serde_json::json;
use state::ServerState;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http_axum::cors::{AllowMethods, CorsLayer};
use zfs::zfs_routes;

type StateType = Arc<Mutex<ServerState>>;

const ZFS_DIR: &str = "/zfs";
const CUSTOM_COMMANDS_DIR: &str = "/custom-commands";
const CUSTOM_COMMANDS_LIST_ENDPOINT: &str = "/custom-commands-list";

async fn handler_404() -> impl IntoResponse {
    (StatusCode::BAD_REQUEST, "Bad request")
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("ZFS error: {0}")]
    Zfs(#[from] ZfsError),
    #[error("ZFS dataset {0} not found")]
    DatasetNotFound(String),
    #[error("ZFS dataset {0} key is not loaded")]
    KeyNotLoadedForDataset(String),
    #[error("ZFS passphrase for dataset {0} is not provided")]
    PassphraseNotProvided(String),
    #[error("ZFS passphrase for dataset {1} is not printable. Error: {0}")]
    NonPrintablePassphrase(String, String),
    #[error("The commands chain is empty")]
    NoCommandsProvided,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Zfs(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::DatasetNotFound(ds) => (StatusCode::NOT_FOUND, ds.to_string()),
            Error::KeyNotLoadedForDataset(_) => (StatusCode::METHOD_NOT_ALLOWED, self.to_string()),
            Error::PassphraseNotProvided(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            Error::NonPrintablePassphrase(_, _) => (StatusCode::BAD_REQUEST, self.to_string()),
            Error::NoCommandsProvided => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

async fn hello() -> Result<impl IntoResponse, Error> {
    Ok(Json::from(HelloResponse::default()))
}

fn web_server(
    socket: TcpListener,
    config: Option<ApiServerConfig>,
) -> Serve<IntoMakeService<Router>, Router> {
    let state = ServerState::new();
    // Placeholder state, for future need
    let state = Arc::new(Mutex::new(state));

    let cors_layer = CorsLayer::new()
        .allow_methods(AllowMethods::list([Method::GET, Method::POST]))
        .allow_headers(tower_http_axum::cors::Any)
        .allow_origin(tower_http_axum::cors::Any);

    let custom_cmds_data = config.map(|cfg| commands_to_routables(cfg.custom_commands));
    let custom_cmds_routes = custom_cmds_data
        .as_ref()
        .map(|cmds| routes_from_config(cmds.clone()))
        .unwrap_or_default()
        .route(
            CUSTOM_COMMANDS_LIST_ENDPOINT,
            get(|s| custom_commands_list_route_handler(s, custom_cmds_data.unwrap_or_default())),
        );

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

    web_server(listener_socket, Some(config))
        .await
        .map_err(Into::into)
}
