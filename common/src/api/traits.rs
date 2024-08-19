use std::collections::BTreeMap;

use crate::types::{
    DatasetFullMountState, DatasetList, DatasetMountedResponse, DatasetsFullMountState,
    KeyLoadedResponse,
};
use async_trait::async_trait;
use reqwasm::http;

#[async_trait(?Send)]
pub trait ZfsRemoteAPI: Clone {
    type Error: std::error::Error + Send + Sync + Clone + 'static;

    async fn encrypted_locked_datasets(&self) -> Result<DatasetList, Self::Error>;
    async fn encrypted_unmounted_datasets(&self) -> Result<DatasetsFullMountState, Self::Error>;
    async fn encrypted_dataset_state(
        &self,
        dataset_name: &str,
    ) -> Result<DatasetFullMountState, Self::Error>;
    async fn load_key(
        &mut self,
        dataset_name: &str,
        password: &str,
    ) -> Result<KeyLoadedResponse, Self::Error>;
    async fn mount_dataset(
        &mut self,
        dataset_name: &str,
    ) -> Result<DatasetMountedResponse, Self::Error>;
    async fn unload_key(&mut self, dataset_name: &str) -> Result<KeyLoadedResponse, Self::Error>;
    async fn unmount_dataset(
        &mut self,
        dataset_name: &str,
    ) -> Result<DatasetMountedResponse, Self::Error>;
}

#[async_trait(?Send)]
pub trait ZfsRemoteHighLevel: ZfsRemoteAPI {}

impl<T: ZfsRemoteAPI> ZfsRemoteHighLevel for T {}

#[async_trait(?Send)]
pub(crate) trait HttpRequest {
    type Error: std::error::Error + 'static;

    async fn get(&self, url: &str) -> Result<http::Response, Self::Error>;
    async fn post<T: serde::Serialize>(
        &self,
        url: &str,
        body: Option<T>,
        extra_headers: BTreeMap<String, String>,
    ) -> Result<http::Response, Self::Error>;
}
