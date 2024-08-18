use crate::types::{
    DatasetList, DatasetMountedResponse, DatasetsFullMountState, KeyLoadedResponse,
};
use async_trait::async_trait;

#[async_trait]
pub trait ZfsRemoteAPI: Clone {
    type Error: std::error::Error + Send + Sync + Clone + 'static;

    async fn encrypted_locked_datasets(&self) -> Result<DatasetList, Self::Error>;
    async fn encrypted_unmounted_datasets(&self) -> Result<DatasetsFullMountState, Self::Error>;
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
