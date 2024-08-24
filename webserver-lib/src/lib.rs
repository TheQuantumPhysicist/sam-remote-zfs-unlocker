mod commands;
pub mod run_options;
pub mod state;

use std::{collections::BTreeMap, sync::Arc};

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::{get, post, IntoMakeService},
    serve::Serve,
    Json, Router,
};
use common::types::{
    AvailableCustomCommands, CustomCommandInfo, DatasetBody, DatasetFullMountState,
    DatasetMountedResponse, DatasetsFullMountState, KeyLoadedResponse,
};
use hyper::{HeaderMap, Method, StatusCode};
use run_options::{
    config::{ApiServerConfig, CustomCommand},
    server_run_options::ServerRunOptions,
};
use sam_zfs_unlocker::{
    zfs_is_dataset_mounted, zfs_is_key_loaded, zfs_load_key, zfs_mount_dataset, ZfsError,
};
use serde_json::json;
use state::ServerState;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http_axum::cors::{AllowMethods, CorsLayer};

type StateType = Arc<Mutex<ServerState>>;

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
    #[error("Command execution error: {0}")]
    CommandExecution(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Zfs(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::DatasetNotFound(ds) => (StatusCode::NOT_FOUND, ds.to_string()),
            Error::KeyNotLoadedForDataset(_) => (StatusCode::METHOD_NOT_ALLOWED, self.to_string()),
            Error::PassphraseNotProvided(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            Error::NonPrintablePassphrase(_, _) => (StatusCode::BAD_REQUEST, self.to_string()),
            Error::CommandExecution(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

/// Waits for a certain check for the dataset to be satisfied, or an error to be returned.
/// The function succeeds in both cases, whether the state is satisfied or not. But if the
/// check isn't satisfied, it'll keep attempting until timeout_duration is passed.
#[allow(dead_code)]
async fn await_state(
    dataset_name: impl AsRef<str>,
    check: impl for<'a> Fn(&'a DatasetFullMountState) -> bool,
    timeout_duration: std::time::Duration,
) -> Result<(), Error> {
    for _ in 0..timeout_duration.as_secs() {
        let new_datasets_state = get_encrypted_datasets_state().await?;
        if let Some(dataset_state) = new_datasets_state.states.get(dataset_name.as_ref()) {
            if check(dataset_state) {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    Ok(())
}

async fn mount_dataset(
    State(_): State<Arc<Mutex<ServerState>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let dataset_name = &json_body.dataset_name;
    if zfs_is_dataset_mounted(dataset_name)?.ok_or(Error::DatasetNotFound(dataset_name.clone()))? {
        return Ok(Json::from(DatasetMountedResponse {
            dataset_name: dataset_name.to_string(),
            is_mounted: true,
        }));
    }

    if !zfs_is_key_loaded(dataset_name)?.ok_or(Error::DatasetNotFound(dataset_name.clone()))? {
        return Err(Error::KeyNotLoadedForDataset(dataset_name.clone()));
    }

    zfs_mount_dataset(dataset_name)?;

    Ok(Json::from(DatasetMountedResponse {
        dataset_name: dataset_name.to_string(),
        is_mounted: true,
    }))
}

async fn load_key(
    State(_state): State<Arc<Mutex<ServerState>>>,
    headers: HeaderMap,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let dataset_name = &json_body.dataset_name;

    if zfs_is_key_loaded(dataset_name)?.ok_or(Error::DatasetNotFound(dataset_name.clone()))? {
        return Ok(Json::from(KeyLoadedResponse {
            dataset_name: dataset_name.to_string(),
            key_loaded: true,
        }));
    }

    let passphrase = match headers.get("Authorization") {
        Some(pp) => pp,
        None => return Err(Error::PassphraseNotProvided(dataset_name.clone())),
    };

    let passphrase = passphrase
        .to_str()
        .map_err(|e| Error::NonPrintablePassphrase(e.to_string(), dataset_name.clone()))?;

    zfs_load_key(dataset_name, passphrase)?;

    Ok(Json::from(KeyLoadedResponse {
        dataset_name: dataset_name.to_string(),
        key_loaded: true,
    }))
}

async fn get_encrypted_datasets_state() -> Result<DatasetsFullMountState, Error> {
    let mount_states = sam_zfs_unlocker::zfs_list_encrypted_datasets()?;

    let mount_states = mount_states
        .into_iter()
        .map(|(ds_name, m)| {
            (
                ds_name,
                DatasetFullMountState {
                    dataset_name: m.dataset_name,
                    key_loaded: m.is_key_loaded,
                    is_mounted: m.is_mounted,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    Ok(DatasetsFullMountState {
        states: mount_states,
    })
}

/// Returns a list of the encrypted datasets, and whether they're mounted, and whether their keys are loaded.
async fn encrypted_datasets_state(
    State(_state): State<Arc<Mutex<ServerState>>>,
) -> Result<impl IntoResponse, Error> {
    let result = get_encrypted_datasets_state().await?;

    Ok(Json::from(result))
}

/// Returns the given encrypted dataset state, and whether it's mounted, and whether their keys is loaded.
async fn encrypted_dataset_state(
    State(_state): State<Arc<Mutex<ServerState>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let dataset_name = &json_body.dataset_name;
    let all_datasets_states = get_encrypted_datasets_state().await?;

    let result = all_datasets_states
        .states
        .get(dataset_name)
        .ok_or(Error::DatasetNotFound(dataset_name.to_string()))?;

    Ok(Json::from(result.clone()))
}

fn zfs_routes() -> Router<StateType> {
    let inner_routes = Router::new()
        .route("/encrypted-datasets-state", get(encrypted_datasets_state))
        .route("/encrypted-dataset-state", post(encrypted_dataset_state))
        .route("/load-key", post(load_key))
        .route("/mount-dataset", post(mount_dataset));

    Router::new().nest("/zfs", inner_routes)
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

    let custom_commands_data = config.map(|cfg| commands_to_routables(cfg.custom_commands));
    let custom_routes = custom_commands_data
        .as_ref()
        .map(|cmds| routes_from_config(cmds.clone()))
        .unwrap_or_default()
        .route(
            "/custom-commands-list",
            get(|s| {
                custom_commands_list_route_handler(s, custom_commands_data.unwrap_or_default())
            }),
        );

    let routes = Router::new()
        .merge(zfs_routes())
        .merge(custom_routes)
        .with_state(state)
        .layer(cors_layer)
        .layer(tower_http_axum::trace::TraceLayer::new_for_http())
        .fallback(handler_404);

    axum::serve(socket, routes.into_make_service())
}

async fn route_handler_from_command(
    State(_state): State<Arc<Mutex<ServerState>>>,
    cmd: RoutableCommand,
) -> Result<impl IntoResponse, Error> {
    let result = commands::run_command(&cmd.run_cmd)
        .await
        .map_err(|e| Error::CommandExecution(e.to_string()))?;

    Ok(Json::from(result))
}

fn route_from_command(router: Router<StateType>, cmd: &RoutableCommand) -> Router<StateType> {
    let cmd = cmd.clone();
    router.route(
        &format!("/{}", cmd.url_endpoint),
        post(move |state| route_handler_from_command(state, cmd)),
    )
}

async fn custom_commands_list_route_handler(
    State(_state): State<Arc<Mutex<ServerState>>>,
    cmds: Vec<RoutableCommand>,
) -> Result<impl IntoResponse, Error> {
    let commands = cmds
        .iter()
        .map(|c| CustomCommandInfo {
            label: c.label.to_string(),
            endpoint: c.url_endpoint.to_string(),
        })
        .collect::<Vec<_>>();

    let result = AvailableCustomCommands { commands };

    Ok(Json::from(result))
}

fn routes_from_config(cmds: Vec<RoutableCommand>) -> Router<StateType> {
    let inner_routes = cmds.iter().fold(Router::new(), route_from_command);

    Router::new().nest("/custom-commands", inner_routes)
}

pub async fn start_server(options: ServerRunOptions) -> Result<(), Box<dyn std::error::Error>> {
    let bind_address = options.bind_address();
    let listener_socket = TcpListener::bind(bind_address).await?;

    let config = options
        .config_path()
        .map(ApiServerConfig::from_file)
        .transpose()?;

    log::info!("Server socket binding to {}", bind_address);

    web_server(listener_socket, config)
        .await
        .map_err(Into::into)
}

pub fn hash_string(s: impl AsRef<str>) -> String {
    use blake2::{Blake2b512, Digest};

    let mut hasher = Blake2b512::new();
    hasher.update(s.as_ref().as_bytes());
    let res = hasher.finalize();

    hex::encode(res).to_ascii_lowercase()
}

#[derive(Clone, Debug)]
struct RoutableCommand {
    pub label: String,
    pub url_endpoint: String,
    pub run_cmd: Vec<String>,
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
        }
    }
}

fn commands_to_routables(cmds: Vec<CustomCommand>) -> Vec<RoutableCommand> {
    cmds.into_iter()
        .filter(|cmd| cmd.enabled)
        .map(Into::into)
        .collect()
}
