use std::sync::Arc;

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use common::types::DatasetBody;
use hyper::HeaderMap;
use tokio::sync::Mutex;

use crate::{
    backend::traits::{ExecutionBackend, ExtraRequestErrors},
    state::ServerState,
    StateType, ZFS_DIR,
};

async fn mount_dataset<B: ExecutionBackend>(
    State(state): State<Arc<Mutex<ServerState<B>>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, <B as ExecutionBackend>::Error> {
    let state = &*state.lock().await;

    let dataset_name = &json_body.dataset_name;

    let result = state.backend.zfs_mount_dataset(dataset_name)?;

    Ok(Json::from(result))
}

async fn load_key<B: ExecutionBackend>(
    State(state): State<Arc<Mutex<ServerState<B>>>>,
    headers: HeaderMap,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, <B as ExecutionBackend>::Error> {
    let dataset_name = &json_body.dataset_name;

    let state = &*state.lock().await;

    let passphrase = match headers.get("Authorization") {
        Some(pp) => pp,
        None => return Err(B::Error::make_error_passphrase_missing(dataset_name)),
    };

    let passphrase = passphrase
        .to_str()
        .map_err(|e| B::Error::make_error_passphrase_non_printable(e, dataset_name.clone()))?;

    let result = state.backend.zfs_load_key(dataset_name, passphrase)?;

    Ok(Json::from(result))
}

/// Returns a list of the encrypted datasets, and whether they're mounted, and whether their keys are loaded.
async fn encrypted_datasets_state<B: ExecutionBackend>(
    State(state): State<Arc<Mutex<ServerState<B>>>>,
) -> Result<impl IntoResponse, <B as ExecutionBackend>::Error> {
    let state = &*state.lock().await;

    let result = state.backend.zfs_encrypted_datasets_state()?;

    Ok(Json::from(result))
}

/// Returns the given encrypted dataset state, and whether it's mounted, and whether their keys is loaded.
async fn encrypted_dataset_state<B: ExecutionBackend>(
    State(state): State<Arc<Mutex<ServerState<B>>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, <B as ExecutionBackend>::Error> {
    let state = &state.lock().await;

    let dataset_name = &json_body.dataset_name;
    let result = state.backend.zfs_encrypted_dataset_state(dataset_name)?;

    Ok(Json::from(result))
}

pub fn zfs_routes<B: ExecutionBackend>() -> Router<StateType<B>> {
    let inner_routes = Router::new()
        .route("/encrypted-datasets-state", get(encrypted_datasets_state))
        .route("/encrypted-dataset-state", post(encrypted_dataset_state))
        .route("/load-key", post(load_key))
        .route("/mount-dataset", post(mount_dataset));

    Router::new().nest(ZFS_DIR, inner_routes)
}
