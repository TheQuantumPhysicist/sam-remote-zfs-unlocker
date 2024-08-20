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
    DatasetBody, DatasetFullMountState, DatasetList, DatasetMountedResponse,
    DatasetsFullMountState, DatasetsMountState, KeyLoadedResponse,
};
use hyper::{HeaderMap, Method, StatusCode};
use sam_zfs_unlocker::{
    zfs_is_dataset_mounted, zfs_is_key_loaded, zfs_load_key, zfs_mount_dataset, zfs_unload_key,
    zfs_unmount_dataset, ZfsError,
};
use serde_json::json;
use state::ServerState;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http_axum::cors::{AllowMethods, CorsLayer};

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
    #[error("Server running in non-permissive. This action is disallowed.")]
    NonPermissiveMode,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Zfs(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::DatasetNotFound(ds) => (StatusCode::NOT_FOUND, ds.to_string()),
            Error::KeyNotLoadedForDataset(_) => (StatusCode::METHOD_NOT_ALLOWED, self.to_string()),
            Error::PassphraseNotProvided(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            Error::NonPrintablePassphrase(_, _) => (StatusCode::BAD_REQUEST, self.to_string()),
            Error::NonPermissiveMode => (StatusCode::UNAUTHORIZED, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
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

async fn unmount_dataset(
    State(state): State<Arc<Mutex<ServerState>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let is_permissive = state.lock().await.is_permissive();
    if !is_permissive {
        return Err(Error::NonPermissiveMode);
    }

    let dataset_name = &json_body.dataset_name;
    if !zfs_is_dataset_mounted(dataset_name)?.ok_or(Error::DatasetNotFound(dataset_name.clone()))? {
        return Ok(Json::from(DatasetMountedResponse {
            dataset_name: dataset_name.to_string(),
            is_mounted: false,
        }));
    }

    zfs_unmount_dataset(dataset_name)?;

    Ok(Json::from(DatasetMountedResponse {
        dataset_name: dataset_name.to_string(),
        is_mounted: false,
    }))
}

async fn load_key(
    State(_): State<Arc<Mutex<ServerState>>>,
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

async fn unload_key(
    State(state): State<Arc<Mutex<ServerState>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let is_permissive = state.lock().await.is_permissive();
    if !is_permissive {
        return Err(Error::NonPermissiveMode);
    }

    let dataset_name = &json_body.dataset_name;

    if !zfs_is_key_loaded(dataset_name)?.ok_or(Error::DatasetNotFound(dataset_name.clone()))? {
        return Ok(Json::from(KeyLoadedResponse {
            dataset_name: dataset_name.to_string(),
            key_loaded: false,
        }));
    }

    zfs_unload_key(dataset_name)?;

    Ok(Json::from(KeyLoadedResponse {
        dataset_name: dataset_name.to_string(),
        key_loaded: false,
    }))
}

async fn encrypted_locked_datasets(
    State(_): State<Arc<Mutex<ServerState>>>,
) -> Result<impl IntoResponse, Error> {
    let mounts = sam_zfs_unlocker::zfs_list_datasets_mountpoints()?;

    let key_loaded_all = mounts
        .keys()
        .map(|m| zfs_is_key_loaded(m).map(|v| (m, v)))
        .collect::<Result<Vec<_>, _>>()?;

    let key_not_loaded = key_loaded_all
        .into_iter()
        .filter_map(|(ds, key_loaded)| key_loaded.map(|kl| (ds.clone(), kl)))
        .filter(|(_, key_loaded)| !(*key_loaded))
        .map(|(ds, _)| ds)
        .collect::<Vec<_>>();

    Ok(Json::from(DatasetList {
        datasets: key_not_loaded,
    }))
}

/// Returns a list of ALL datasets (whether encrypted or not) and whether they're mounted.
/// This function only works in permissive mode.
async fn all_datasets_mount_state(
    State(state): State<Arc<Mutex<ServerState>>>,
) -> Result<impl IntoResponse, Error> {
    let is_permissive = state.lock().await.is_permissive();
    if !is_permissive {
        return Err(Error::NonPermissiveMode);
    }

    let mounts = sam_zfs_unlocker::zfs_list_datasets_mountpoints()?;

    let mount_states = mounts
        .keys()
        .map(|ds| zfs_is_dataset_mounted(ds).map(|m| (ds, m)))
        .collect::<Result<Vec<_>, _>>()?;

    let mount_states = mount_states
        .into_iter()
        .filter_map(|(ds, m)| m.map(|v| (ds.to_string(), v)))
        .collect::<BTreeMap<_, _>>();

    Ok(Json::from(DatasetsMountState {
        datasets_mounted: mount_states,
    }))
}

async fn get_encrypted_datasets_state(
    state: Arc<Mutex<ServerState>>,
) -> Result<DatasetsFullMountState, Error> {
    let mount_states = sam_zfs_unlocker::zfs_list_encrypted_datasets()?;

    let permissive = state.lock().await.is_permissive();

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
        .filter(|(_ds_name, m)| if permissive { true } else { !m.is_mounted })
        .collect::<BTreeMap<_, _>>();

    Ok(DatasetsFullMountState {
        states: mount_states,
    })
}

/// Returns a list of the encrypted datasets, and whether they're mounted, and whether their keys are loaded.
/// In permissive mode, all encrypted datasets are returns. In non-permissive mode, only unmounted are returned.
async fn encrypted_unmounted_datasets(
    State(state): State<Arc<Mutex<ServerState>>>,
) -> Result<impl IntoResponse, Error> {
    let result = get_encrypted_datasets_state(state).await?;

    Ok(Json::from(result))
}

/// Returns the given encrypted dataset state, and whether it's mounted, and whether their keys is loaded.
async fn encrypted_dataset_state(
    State(state): State<Arc<Mutex<ServerState>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let dataset_name = &json_body.dataset_name;
    let all_datasets_states = get_encrypted_datasets_state(state).await?;

    let result = all_datasets_states
        .states
        .get(dataset_name)
        .ok_or(Error::DatasetNotFound(dataset_name.to_string()))?;

    Ok(Json::from(result.clone()))
}

/// Returns true if permissive mode is enabled
async fn is_permissive(
    State(state): State<Arc<Mutex<ServerState>>>,
) -> Result<impl IntoResponse, Error> {
    Ok(Json::from(state.lock().await.is_permissive()))
}

fn routes() -> Router<Arc<Mutex<ServerState>>> {
    let router = Router::new();

    router
        .route("/encrypted_locked_datasets", get(encrypted_locked_datasets))
        .route(
            "/encrypted_unmounted_datasets",
            get(encrypted_unmounted_datasets),
        )
        .route("/encrypted_dataset_state", post(encrypted_dataset_state))
        .route("/load_key", post(load_key))
        .route("/mount", post(mount_dataset))
        .route("/is_permissive", get(is_permissive))
        .route("/all_datasets_mount_state", get(all_datasets_mount_state))
        .route("/unload_key", post(unload_key))
        .route("/unmount", post(unmount_dataset))
}

pub fn web_server(
    socket: TcpListener,
    is_permissive: bool,
) -> Serve<IntoMakeService<Router>, Router> {
    let state = ServerState::new(is_permissive);
    // Placeholder state, for future need
    let state = Arc::new(Mutex::new(state));

    let cors_layer = CorsLayer::new()
        .allow_methods(AllowMethods::list([Method::GET, Method::POST]))
        .allow_headers(tower_http_axum::cors::Any)
        .allow_origin(tower_http_axum::cors::Any);

    let routes = Router::new()
        .nest("/zfs", routes())
        .with_state(state)
        .layer(cors_layer)
        .fallback(handler_404);

    axum::serve(socket, routes.into_make_service())
}
