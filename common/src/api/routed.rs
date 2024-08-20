use std::collections::BTreeMap;

use super::traits::HttpRequest;
use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    config::LiveSettings,
    types::{
        DatasetBody, DatasetFullMountState, DatasetList, DatasetMountedResponse,
        DatasetsFullMountState, KeyLoadedResponse,
    },
};

use super::{traits::ZfsRemoteAPI, wasm_request::WasmRequest};

#[derive(thiserror::Error, Debug, Clone)]
pub enum ApiError {
    #[error("Request error: {0}")]
    Request(String),
    #[error("Json conversion error for URL `{0}`: {1}")]
    JsonConversion(String, String),
}

#[derive(Debug, Clone)]
pub struct ApiRouteImpl {
    base_url: String,
}

impl ApiRouteImpl {
    pub fn new_from_config(settings: LiveSettings) -> Self {
        Self {
            base_url: settings.base_url,
        }
    }
}

#[async_trait(?Send)]
impl ZfsRemoteAPI for ApiRouteImpl {
    type Error = ApiError;

    async fn encrypted_locked_datasets(&self) -> Result<DatasetList, Self::Error> {
        let url = format!("{}/zfs/encrypted_locked_datasets", self.base_url);
        do_get_request(&url).await
    }

    async fn encrypted_unmounted_datasets(&self) -> Result<DatasetsFullMountState, Self::Error> {
        let url = format!("{}/zfs/encrypted_unmounted_datasets", self.base_url);
        do_get_request(&url).await
    }

    async fn encrypted_dataset_state(
        &self,
        dataset_name: &str,
    ) -> Result<DatasetFullMountState, Self::Error> {
        let url = format!("{}/zfs/encrypted_dataset_state", self.base_url);
        do_post_request(
            &url,
            Some(DatasetBody {
                dataset_name: dataset_name.to_string(),
            }),
            [].into(),
        )
        .await
    }

    async fn load_key(
        &mut self,
        dataset_name: &str,
        password: &str,
    ) -> Result<KeyLoadedResponse, Self::Error> {
        let url = format!("{}/zfs/load_key", self.base_url);
        do_post_request(
            &url,
            Some(DatasetBody {
                dataset_name: dataset_name.to_string(),
            }),
            [("Authorization".to_string(), password.to_string())]
                .into_iter()
                .collect(),
        )
        .await
    }

    async fn mount_dataset(
        &mut self,
        dataset_name: &str,
    ) -> Result<DatasetMountedResponse, Self::Error> {
        let url = format!("{}/zfs/mount", self.base_url);
        do_post_request(
            &url,
            Some(DatasetBody {
                dataset_name: dataset_name.to_string(),
            }),
            [].into_iter().collect(),
        )
        .await
    }

    async fn unload_key(&mut self, dataset_name: &str) -> Result<KeyLoadedResponse, Self::Error> {
        let url = format!("{}/zfs/unload_key", self.base_url);
        do_post_request(
            &url,
            Some(DatasetBody {
                dataset_name: dataset_name.to_string(),
            }),
            [].into_iter().collect(),
        )
        .await
    }

    async fn unmount_dataset(
        &mut self,
        dataset_name: &str,
    ) -> Result<DatasetMountedResponse, Self::Error> {
        let url = format!("{}/unmount", self.base_url);
        do_post_request(
            &url,
            Some(DatasetBody {
                dataset_name: dataset_name.to_string(),
            }),
            [].into_iter().collect(),
        )
        .await
    }
}

async fn do_get_request<J: for<'de> Deserialize<'de>>(url: &str) -> Result<J, ApiError> {
    let response = WasmRequest::new()
        .get(&url)
        .await
        .map_err(|e| ApiError::Request(e.to_string()))?;
    let response_json = response
        .json::<J>()
        .await
        .map_err(|e| ApiError::JsonConversion(url.to_string(), e.to_string()))?;

    Ok(response_json)
}

async fn do_post_request<J: for<'de> Deserialize<'de>, T: serde::Serialize>(
    url: &str,
    body: Option<T>,
    extra_headers: BTreeMap<String, String>,
) -> Result<J, ApiError> {
    let response = WasmRequest::new()
        .post(&url, body, extra_headers)
        .await
        .map_err(|e| ApiError::Request(e.to_string()))?;
    let response_json = response
        .json::<J>()
        .await
        .map_err(|e| ApiError::JsonConversion(url.to_string(), e.to_string()))?;

    Ok(response_json)
}
