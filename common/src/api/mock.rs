use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;

use crate::types::{
    DatasetFullMountState, DatasetList, DatasetMountedResponse, DatasetsMountState,
    KeyLoadedResponse,
};

use super::traits::ZfsRemoteAPI;

#[derive(thiserror::Error, Debug)]
pub enum ApiMockError {
    #[error("Wrong password")]
    InvalidEncryptionPassword,
    #[error("Dataset not found {0}")]
    DatasetNotFound(String),
    #[error("Attempted unload key for a busy dataset: {0}")]
    CannotUnlockKeyForMountDataset(String),
}

pub struct MockDatasetDetails {
    state: DatasetFullMountState,
    unlock_password: String,
}

pub struct ApiMock {
    state: BTreeMap<String, MockDatasetDetails>,
    /// Permissive is true when all functionalities are allowed in the API server
    /// when false, only limited functionality exists
    permissive: bool,
}

impl ApiMock {
    pub fn new(permissive: bool, dataset_names_and_password: Vec<(String, String)>) -> Self {
        let state = dataset_names_and_password
            .into_iter()
            .map(|(ds_name, password)| {
                (
                    ds_name.to_string(),
                    MockDatasetDetails {
                        state: DatasetFullMountState {
                            dataset_name: ds_name,
                            key_loaded: false,
                            is_mounted: false,
                        },
                        unlock_password: password,
                    },
                )
            })
            .collect();

        Self { state, permissive }
    }
}

#[async_trait]
impl ZfsRemoteAPI for ApiMock {
    type Error = ApiMockError;

    async fn locked_datasets(&self) -> Result<DatasetList, Self::Error> {
        sleep_for_dramatic_effect().await;

        let datasets = self
            .state
            .iter()
            .filter(|(_ds_name, m)| !m.state.key_loaded)
            .map(|(ds_name, _m)| ds_name.to_string())
            .collect();
        Ok(DatasetList { datasets })
    }
    async fn unmounted_datasets(&self) -> Result<DatasetsMountState, Self::Error> {
        sleep_for_dramatic_effect().await;

        let datasets_mounted = self
            .state
            .iter()
            .map(|(ds_name, m)| (ds_name.to_string(), m.state.is_mounted))
            .collect();
        Ok(DatasetsMountState { datasets_mounted })
    }
    async fn load_key(
        &mut self,
        dataset_name: &str,
        password: &str,
    ) -> Result<KeyLoadedResponse, Self::Error> {
        sleep_for_dramatic_effect().await;

        let dataset_details = self
            .state
            .get_mut(dataset_name)
            .ok_or(ApiMockError::DatasetNotFound(dataset_name.to_string()))?;

        if password == dataset_details.unlock_password {
            dataset_details.state.key_loaded = true;
            Ok(KeyLoadedResponse {
                dataset_name: dataset_name.to_string(),
                key_loaded: true,
            })
        } else {
            Err(ApiMockError::InvalidEncryptionPassword)
        }
    }
    async fn mount_dataset(
        &mut self,
        dataset_name: &str,
    ) -> Result<DatasetMountedResponse, Self::Error> {
        sleep_for_dramatic_effect().await;

        let dataset_details = self
            .state
            .get_mut(dataset_name)
            .ok_or(ApiMockError::DatasetNotFound(dataset_name.to_string()))?;

        dataset_details.state.is_mounted = true;
        Ok(DatasetMountedResponse {
            dataset_name: dataset_name.to_string(),
            is_mounted: true,
        })
    }
    async fn unload_key(&mut self, dataset_name: &str) -> Result<KeyLoadedResponse, Self::Error> {
        sleep_for_dramatic_effect().await;

        let dataset_details = self
            .state
            .get_mut(dataset_name)
            .ok_or(ApiMockError::DatasetNotFound(dataset_name.to_string()))?;

        if !dataset_details.state.is_mounted {
            dataset_details.state.key_loaded = false;
            Ok(KeyLoadedResponse {
                dataset_name: dataset_name.to_string(),
                key_loaded: false,
            })
        } else {
            Err(ApiMockError::CannotUnlockKeyForMountDataset(
                dataset_name.to_string(),
            ))
        }
    }
    async fn unmount_dataset(
        &mut self,
        dataset_name: &str,
    ) -> Result<DatasetMountedResponse, Self::Error> {
        sleep_for_dramatic_effect().await;

        let dataset_details = self
            .state
            .get_mut(dataset_name)
            .ok_or(ApiMockError::DatasetNotFound(dataset_name.to_string()))?;

        dataset_details.state.is_mounted = false;
        Ok(DatasetMountedResponse {
            dataset_name: dataset_name.to_string(),
            is_mounted: false,
        })
    }

    async fn is_permissive(&self) -> Result<bool, Self::Error> {
        Ok(self.permissive)
    }
}

async fn sleep_for_dramatic_effect() {
    const SLEEP_TIME: Duration = Duration::from_secs(2);
    tokio::time::sleep(SLEEP_TIME).await;
}
