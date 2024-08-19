use std::collections::BTreeMap;

use super::traits::HttpRequest;
use async_trait::async_trait;
use serde::Deserialize;

use crate::types::{
    DatasetBody, DatasetFullMountState, DatasetList, DatasetMountedResponse,
    DatasetsFullMountState, KeyLoadedResponse,
};

use super::{traits::ZfsRemoteAPI, wasm_request::WasmRequest};

#[derive(thiserror::Error, Debug, Clone)]
pub enum ApiError {
    #[error("Request error: {0}")]
    Request(String),
    #[error("Json conversion error")]
    JsonConversion(String),
}

#[derive(Debug, Clone)]
pub struct ApiRouteImpl {
    base_url: String,
}

#[async_trait(?Send)]
impl ZfsRemoteAPI for ApiRouteImpl {
    type Error = ApiError;

    async fn encrypted_locked_datasets(&self) -> Result<DatasetList, Self::Error> {
        let url = format!("{}/encrypted_locked_datasets", self.base_url);
        do_get_request(&url).await
    }

    async fn encrypted_unmounted_datasets(&self) -> Result<DatasetsFullMountState, Self::Error> {
        let url = format!("{}/encrypted_unmounted_datasets", self.base_url);
        do_get_request(&url).await
    }

    async fn encrypted_dataset_state(
        &self,
        dataset_name: &str,
    ) -> Result<DatasetFullMountState, Self::Error> {
        let url = format!(
            "{}/encrypted_unmounted_datasets/{dataset_name}",
            self.base_url
        );
        do_get_request(&url).await
    }

    async fn load_key(
        &mut self,
        dataset_name: &str,
        password: &str,
    ) -> Result<KeyLoadedResponse, Self::Error> {
        let url = format!("{}/load_key", self.base_url);
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
        let url = format!("{}/mount", self.base_url);
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
        let url = format!("{}/unload_key", self.base_url);
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
        let url = format!("{}/is_permissive", self.base_url);
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
        .map_err(|e| ApiError::JsonConversion(e.to_string()))?;

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
        .map_err(|e| ApiError::JsonConversion(e.to_string()))?;

    Ok(response_json)
}
