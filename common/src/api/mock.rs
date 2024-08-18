use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;

use crate::types::{
    DatasetFullMountState, DatasetList, DatasetMountedResponse, DatasetsMountState,
    KeyLoadedResponse,
};

use super::traits::ZfsRemoteAPI;

#[derive(thiserror::Error, Debug, Clone)]
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
struct ApiMockInner {
    state: BTreeMap<String, MockDatasetDetails>,
    /// Permissive is true when all functionalities are allowed in the API server
    /// when false, only limited functionality exists
    permissive: bool,
}

#[derive(Clone)]
pub struct ApiMock {
    inner: Arc<Mutex<ApiMockInner>>,
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

        let result = ApiMockInner { state, permissive };

        Self {
            inner: Arc::new(result.into()),
        }
    }
}

#[async_trait]
impl ZfsRemoteAPI for ApiMock {
    type Error = ApiMockError;

    async fn locked_datasets(&self) -> Result<DatasetList, Self::Error> {
        sleep_for_dramatic_effect().await;

        let inner = self.inner.lock().expect("Poisoned mutex");

        let datasets = inner
            .state
            .iter()
            .filter(|(_ds_name, m)| !m.state.key_loaded)
            .map(|(ds_name, _m)| ds_name.to_string())
            .collect();
        Ok(DatasetList { datasets })
    }
    async fn unmounted_datasets(&self) -> Result<DatasetsMountState, Self::Error> {
        sleep_for_dramatic_effect().await;

        let inner = self.inner.lock().expect("Poisoned mutex");

        let datasets_mounted = inner
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

        let mut inner = self.inner.lock().expect("Poisoned mutex");

        let dataset_details = inner
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

        let mut inner = self.inner.lock().expect("Poisoned mutex");

        let dataset_details = inner
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

        let mut inner = self.inner.lock().expect("Poisoned mutex");

        let dataset_details = inner
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

        let mut inner = self.inner.lock().expect("Poisoned mutex");

        let dataset_details = inner
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
        let inner = self.inner.lock().expect("Poisoned mutex");

        Ok(inner.permissive)
    }
}

async fn sleep_for_dramatic_effect() {
    const SLEEP_TIME: Duration = Duration::from_secs(5);
    tokio::time::sleep(SLEEP_TIME).await;
}
