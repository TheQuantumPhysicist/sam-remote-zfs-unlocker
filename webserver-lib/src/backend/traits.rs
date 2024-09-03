use std::collections::BTreeMap;

use axum::{async_trait, response::IntoResponse};
use common::types::{
    AvailableCustomCommands, DatasetFullMountState, DatasetMountedResponse, DatasetsFullMountState,
    KeyLoadedResponse, RunCommandOutput,
};

use super::routable_command::RoutableCommand;

#[async_trait]
pub trait ExecutionBackend: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static + IntoResponse + ExtraRequestErrors<Self>;

    fn zfs_encrypted_datasets_state(&self) -> Result<DatasetsFullMountState, Self::Error>;
    fn zfs_encrypted_dataset_state(
        &self,
        dataset_name: impl AsRef<str>,
    ) -> Result<DatasetFullMountState, Self::Error>;
    fn zfs_load_key(
        &self,
        dataset_name: impl AsRef<str>,
        passphrase: impl AsRef<str>,
    ) -> Result<KeyLoadedResponse, Self::Error>;
    fn zfs_mount_dataset(
        &self,
        dataset_name: impl AsRef<str>,
    ) -> Result<DatasetMountedResponse, Self::Error>;

    fn custom_cmds_list(&self) -> Result<AvailableCustomCommands, Self::Error>;

    fn custom_cmds_routables(&self) -> &BTreeMap<String, RoutableCommand>;

    async fn custom_cmd_call(
        &self,
        endpoint: &str,
        initial_stdin_input: Option<String>,
    ) -> Result<RunCommandOutput, Self::Error>;
}

/// Errors that come from API requests details, instead of from the implementation
pub trait ExtraRequestErrors<B: ExecutionBackend + ?Sized> {
    fn make_error_passphrase_missing(dataset_name: impl Into<String>) -> B::Error;
    fn make_error_passphrase_non_printable(
        error: impl std::error::Error,
        dataset_name: impl Into<String>,
    ) -> B::Error;
    fn make_error_internetl_custom_command_error(url_endpoint: String) -> B::Error;
}
