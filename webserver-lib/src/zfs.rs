use std::{collections::BTreeMap, sync::Arc};

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use common::types::{
    DatasetBody, DatasetFullMountState, DatasetMountedResponse, DatasetsFullMountState,
    KeyLoadedResponse,
};
use hyper::HeaderMap;
use sam_zfs_unlocker::{
    zfs_is_dataset_mounted, zfs_is_key_loaded, zfs_load_key, zfs_mount_dataset,
};
use tokio::sync::Mutex;

use crate::{state::ServerState, Error, StateType, ZFS_DIR};

async fn mount_dataset(
    State(state): State<Arc<Mutex<ServerState>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let config = &state.lock().await.zfs_config;
    if !config.zfs_enabled {
        return Err(Error::ZfsDisabled);
    }

    let dataset_name = &json_body.dataset_name;
    if config
        .blacklisted_zfs_datasets
        .as_ref()
        .map(|bl| bl.contains(dataset_name))
        .unwrap_or(false)
    {
        return Err(Error::BlacklistedDataset(dataset_name.clone()));
    }

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
    State(state): State<Arc<Mutex<ServerState>>>,
    headers: HeaderMap,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let config = &state.lock().await.zfs_config;
    if !config.zfs_enabled {
        return Err(Error::ZfsDisabled);
    }

    let dataset_name = &json_body.dataset_name;
    if config
        .blacklisted_zfs_datasets
        .as_ref()
        .map(|bl| bl.contains(dataset_name))
        .unwrap_or(false)
    {
        return Err(Error::BlacklistedDataset(dataset_name.clone()));
    }

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

fn internal_get_encrypted_datasets_state(
    state: &ServerState,
) -> Result<DatasetsFullMountState, Error> {
    let config = &state.zfs_config;
    if !config.zfs_enabled {
        return Ok(DatasetsFullMountState {
            states: Default::default(),
        });
    }

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
        .filter(|(ds_name, _m)| {
            !config
                .blacklisted_zfs_datasets
                .as_ref()
                .cloned()
                .unwrap_or_default()
                .iter()
                .any(|bl| ds_name == bl)
        })
        .collect::<BTreeMap<_, _>>();

    Ok(DatasetsFullMountState {
        states: mount_states,
    })
}

/// Returns a list of the encrypted datasets, and whether they're mounted, and whether their keys are loaded.
async fn encrypted_datasets_state(
    State(state): State<Arc<Mutex<ServerState>>>,
) -> Result<impl IntoResponse, Error> {
    let state = &state.lock().await;

    let result = internal_get_encrypted_datasets_state(state)?;

    Ok(Json::from(result))
}

/// Returns the given encrypted dataset state, and whether it's mounted, and whether their keys is loaded.
async fn encrypted_dataset_state(
    State(state): State<Arc<Mutex<ServerState>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
    let state = &state.lock().await;

    let dataset_name = &json_body.dataset_name;
    let all_datasets_states = internal_get_encrypted_datasets_state(state)?;

    let result = all_datasets_states
        .states
        .get(dataset_name)
        .ok_or(Error::DatasetNotFound(dataset_name.to_string()))?;

    Ok(Json::from(result.clone()))
}

pub fn zfs_routes() -> Router<StateType> {
    let inner_routes = Router::new()
        .route("/encrypted-datasets-state", get(encrypted_datasets_state))
        .route("/encrypted-dataset-state", post(encrypted_dataset_state))
        .route("/load-key", post(load_key))
        .route("/mount-dataset", post(mount_dataset));

    Router::new().nest(ZFS_DIR, inner_routes)
}
