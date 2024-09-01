use async_trait::async_trait;

use crate::types::{
    AvailableCustomCommands, DatasetFullMountState, DatasetMountedResponse, DatasetsFullMountState,
    KeyLoadedResponse, RunCommandOutput,
};

use super::{
    mock::{ApiMock, ApiMockError},
    routed::{ApiError, ApiRouteImpl},
    traits::ZfsRemoteAPI,
};

/// This is a manual `dyn` solution because the API cannot go into a vtable
#[derive(Clone)]
pub enum ApiAny {
    Live(ApiRouteImpl),
    Mock(ApiMock),
}

#[derive(Clone, Debug)]
pub enum ApiAnyError {
    Live(ApiError),
    Mock(ApiMockError),
}

impl std::error::Error for ApiAnyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ApiAnyError::Live(e) => e.source(),
            ApiAnyError::Mock(e) => e.source(),
        }
    }
}

#[async_trait(?Send)]
impl ZfsRemoteAPI for ApiAny {
    type Error = ApiAnyError;

    async fn encrypted_datasets_state(&self) -> Result<DatasetsFullMountState, Self::Error> {
        match self {
            ApiAny::Live(e) => e.encrypted_datasets_state().await.map_err(Into::into),
            ApiAny::Mock(e) => e.encrypted_datasets_state().await.map_err(Into::into),
        }
    }

    async fn encrypted_dataset_state(
        &self,
        dataset_name: &str,
    ) -> Result<DatasetFullMountState, Self::Error> {
        match self {
            ApiAny::Live(e) => e
                .encrypted_dataset_state(dataset_name)
                .await
                .map_err(Into::into),
            ApiAny::Mock(e) => e
                .encrypted_dataset_state(dataset_name)
                .await
                .map_err(Into::into),
        }
    }

    async fn load_key(
        &mut self,
        dataset_name: &str,
        password: &str,
    ) -> Result<KeyLoadedResponse, Self::Error> {
        match self {
            ApiAny::Live(e) => e.load_key(dataset_name, password).await.map_err(Into::into),
            ApiAny::Mock(e) => e.load_key(dataset_name, password).await.map_err(Into::into),
        }
    }

    async fn mount_dataset(
        &mut self,
        dataset_name: &str,
    ) -> Result<DatasetMountedResponse, Self::Error> {
        match self {
            ApiAny::Live(e) => e.mount_dataset(dataset_name).await.map_err(Into::into),
            ApiAny::Mock(e) => e.mount_dataset(dataset_name).await.map_err(Into::into),
        }
    }

    async fn list_available_commands(&self) -> Result<AvailableCustomCommands, Self::Error> {
        match self {
            ApiAny::Live(e) => e.list_available_commands().await.map_err(Into::into),
            ApiAny::Mock(e) => e.list_available_commands().await.map_err(Into::into),
        }
    }

    async fn call_custom_command(
        &mut self,
        endpoint: &str,
        stdin: Option<&str>,
    ) -> Result<RunCommandOutput, Self::Error> {
        match self {
            ApiAny::Live(e) => e
                .call_custom_command(endpoint, stdin)
                .await
                .map_err(Into::into),
            ApiAny::Mock(e) => e
                .call_custom_command(endpoint, stdin)
                .await
                .map_err(Into::into),
        }
    }
}

impl From<ApiRouteImpl> for ApiAny {
    fn from(value: ApiRouteImpl) -> Self {
        ApiAny::Live(value)
    }
}

impl From<ApiMock> for ApiAny {
    fn from(value: ApiMock) -> Self {
        ApiAny::Mock(value)
    }
}

impl std::fmt::Display for ApiAnyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiAnyError::Live(e) => e.fmt(f),
            ApiAnyError::Mock(e) => e.fmt(f),
        }
    }
}

impl From<ApiError> for ApiAnyError {
    fn from(value: ApiError) -> Self {
        ApiAnyError::Live(value)
    }
}

impl From<ApiMockError> for ApiAnyError {
    fn from(value: ApiMockError) -> Self {
        ApiAnyError::Mock(value)
    }
}
