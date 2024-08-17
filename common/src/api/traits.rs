use crate::types::{DatasetList, DatasetMountedResponse, DatasetsMountState, KeyLoadedResponse};
use async_trait::async_trait;

#[async_trait]
pub trait ZfsRemoteAPI {
    type Error: std::error::Error;

    async fn locked_datasets(&self) -> Result<DatasetList, Self::Error>;
    async fn unmounted_datasets(&self) -> Result<DatasetsMountState, Self::Error>;
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
    async fn is_permissive(&self) -> Result<bool, Self::Error>;
}

#[async_trait]
pub trait ZfsRemoteHighLevel: ZfsRemoteAPI {}

impl<T: ZfsRemoteAPI> ZfsRemoteHighLevel for T {}
