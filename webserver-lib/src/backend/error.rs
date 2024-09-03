use axum::{
    response::{IntoResponse, Response},
    Json,
};
use hyper::StatusCode;
use sam_zfs_unlocker::ZfsError;
use serde_json::json;

#[derive(thiserror::Error, Debug)]
pub enum Error {
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
    #[error("ZFS control is disabled in API server")]
    ZfsDisabled,
    #[error("Attempted to mutate the state of a blacklisted dataset {0}")]
    BlacklistedDataset(String),
    #[error("Internal invariant error: A registered command was not found: {0}")]
    RegisteredCmdMissing(String),
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
            Error::ZfsDisabled => (StatusCode::UNAUTHORIZED, self.to_string()),
            Error::BlacklistedDataset(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            Error::RegisteredCmdMissing(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
