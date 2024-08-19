use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
    config::MockSettings,
    types::{
        DatasetFullMountState, DatasetList, DatasetMountedResponse, DatasetsFullMountState,
        KeyLoadedResponse,
    },
};

use super::{sleeper::Sleepr, traits::ZfsRemoteAPI};

#[derive(thiserror::Error, Debug, Clone)]
pub enum ApiMockError {
    #[error("Wrong password")]
    InvalidEncryptionPassword,
    #[error("Dataset not found {0}")]
    DatasetNotFound(String),
    #[error("Attempted unload key for a busy dataset: {0}")]
    CannotUnlockKeyForMountDataset(String),
    #[error("Simulated error for dataset: {0}")]
    SimulatedError(String),
}

#[derive(Debug, Clone)]
pub struct MockDatasetDetails {
    state: DatasetFullMountState,
    unlock_password: String,
}
struct ApiMockInner {
    state: BTreeMap<String, MockDatasetDetails>,
    /// Datasets that produce errors on requests
    erring_datasets: BTreeSet<String>,
}

#[derive(Clone)]
pub struct ApiMock {
    inner: Arc<Mutex<ApiMockInner>>,
}

impl ApiMock {
    pub fn new(
        dataset_names_and_password: Vec<(String, String)>,
        erring_datasets: Vec<String>,
    ) -> Self {
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

        let erring_datasets = erring_datasets.into_iter().collect();

        let result = ApiMockInner {
            state,
            erring_datasets,
        };

        Self {
            inner: Arc::new(result.into()),
        }
    }

    pub fn new_from_config(config: MockSettings) -> Self {
        let state = config
            .datasets_and_passwords
            .unwrap_or_default()
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

        let erring_datasets = config
            .erring_datasets
            .unwrap_or_default()
            .into_iter()
            .collect();

        let result = ApiMockInner {
            state,
            erring_datasets,
        };

        Self {
            inner: Arc::new(result.into()),
        }
    }
}

#[async_trait(?Send)]
impl ZfsRemoteAPI for ApiMock {
    type Error = ApiMockError;

    async fn encrypted_locked_datasets(&self) -> Result<DatasetList, Self::Error> {
        sleep_for_dramatic_effect().await;

        let inner = self.inner.lock().expect("Poisoned mutex");

        let datasets = inner
            .state
            .iter()
            .filter(|(_ds_name, m)| !m.state.key_loaded)
            .map(|(ds_name, _m)| ds_name.to_string())
            .chain(inner.erring_datasets.clone().into_iter())
            .collect();
        Ok(DatasetList { datasets })
    }

    async fn encrypted_unmounted_datasets(&self) -> Result<DatasetsFullMountState, Self::Error> {
        sleep_for_dramatic_effect().await;

        let inner = self.inner.lock().expect("Poisoned mutex");

        let datasets_mounted = inner
            .state
            .iter()
            .map(|(ds_name, m)| (ds_name.to_string(), m.state.clone()))
            .chain(inner.erring_datasets.iter().map(|ds_name| {
                (
                    ds_name.to_string(),
                    DatasetFullMountState {
                        dataset_name: ds_name.to_string(),
                        key_loaded: false,
                        is_mounted: false,
                    },
                )
            }))
            .collect();

        Ok(DatasetsFullMountState {
            states: datasets_mounted,
        })
    }

    async fn load_key(
        &mut self,
        dataset_name: &str,
        password: &str,
    ) -> Result<KeyLoadedResponse, Self::Error> {
        sleep_for_dramatic_effect().await;

        let mut inner = self.inner.lock().expect("Poisoned mutex");

        if let Some(ds_name) = inner.erring_datasets.get(dataset_name) {
            return Err(ApiMockError::SimulatedError(ds_name.to_string()));
        }

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

        if let Some(ds_name) = inner.erring_datasets.get(dataset_name) {
            return Err(ApiMockError::SimulatedError(ds_name.to_string()));
        }

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

        if let Some(ds_name) = inner.erring_datasets.get(dataset_name) {
            return Err(ApiMockError::SimulatedError(ds_name.to_string()));
        }

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

        if let Some(ds_name) = inner.erring_datasets.get(dataset_name) {
            return Err(ApiMockError::SimulatedError(ds_name.to_string()));
        }

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

    async fn encrypted_dataset_state(
        &self,
        dataset_name: &str,
    ) -> Result<DatasetFullMountState, Self::Error> {
        sleep_for_dramatic_effect().await;

        let inner = self.inner.lock().expect("Poisoned mutex");

        if let Some(ds_name) = inner.erring_datasets.get(dataset_name) {
            return Err(ApiMockError::SimulatedError(ds_name.to_string()));
        }

        let dataset_details = inner
            .state
            .get(dataset_name)
            .ok_or(ApiMockError::DatasetNotFound(dataset_name.to_string()))?;

        Ok(dataset_details.state.clone())
    }
}

async fn sleep_for_dramatic_effect() {
    const SLEEP_DURATION: u32 = 1000;
    Sleepr::new(SLEEP_DURATION).sleep().await;
}
