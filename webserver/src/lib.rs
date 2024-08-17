use std::{collections::BTreeMap, sync::Arc};

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::{get, post, IntoMakeService},
    serve::Serve,
    Json, Router,
};
use common::types::{
    DatasetFullMountState, DatasetList, DatasetMountedResponse, DatasetsFullMountState,
    DatasetsMountState, KeyLoadedResponse,
};
use hyper::{HeaderMap, StatusCode};
use sam_zfs_unlocker::{
    zfs_is_dataset_mounted, zfs_is_key_loaded, zfs_load_key, zfs_mount_dataset, zfs_unload_key,
    zfs_unmount_dataset, ZfsError,
};
use serde::Deserialize;
use serde_json::json;
use tokio::{net::TcpListener, sync::Mutex};

#[derive(Debug, Deserialize)]
struct DatasetBody {
    dataset_name: String,
}

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
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::Zfs(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::DatasetNotFound(ds) => (StatusCode::NOT_FOUND, ds.to_string()),
            Error::KeyNotLoadedForDataset(_) => (StatusCode::METHOD_NOT_ALLOWED, self.to_string()),
            Error::PassphraseNotProvided(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            Error::NonPrintablePassphrase(_, _) => (StatusCode::BAD_REQUEST, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

async fn mount_dataset(
    State(_): State<Arc<Mutex<()>>>,
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
    State(_): State<Arc<Mutex<()>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
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
    State(_): State<Arc<Mutex<()>>>,
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
    State(_): State<Arc<Mutex<()>>>,
    json_body: Json<DatasetBody>,
) -> Result<impl IntoResponse, Error> {
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

async fn locked_datasets(State(_): State<Arc<Mutex<()>>>) -> Result<impl IntoResponse, Error> {
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

async fn all_datasets_mount_state(
    State(_): State<Arc<Mutex<()>>>,
) -> Result<impl IntoResponse, Error> {
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

async fn unmounted_datasets(State(_): State<Arc<Mutex<()>>>) -> Result<impl IntoResponse, Error> {
    let mount_states = sam_zfs_unlocker::zfs_list_unmounted_datasets()?;

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

    Ok(Json::from(DatasetsFullMountState {
        states: mount_states,
    }))
}

fn routes(permissive: bool) -> Router<Arc<Mutex<()>>> {
    let router = Router::new();

    let router = router
        .route("/locked_datasets", get(locked_datasets))
        .route("/unmounted_datasets", get(unmounted_datasets))
        .route("/load_key", post(load_key))
        .route("/mount", post(mount_dataset));

    // Permissive mode reveals more information about datasets that are not encrypted or locked
    if permissive {
        router
            .route("/datasets_mount_state", get(all_datasets_mount_state))
            .route("/unload_key", post(unload_key))
            .route("/unmount", post(unmount_dataset))
    } else {
        router
    }
}

pub fn web_server(socket: TcpListener, permissive: bool) -> Serve<IntoMakeService<Router>, Router> {
    let state = Arc::new(Mutex::new(()));

    let routes = Router::new()
        .nest("/zfs", routes(permissive))
        .with_state(state)
        .fallback(handler_404);

    axum::serve(socket, routes.into_make_service())
}
