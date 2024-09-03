use std::collections::BTreeMap;

use axum::async_trait;
use common::types::{
    AvailableCustomCommands, CustomCommandPublicInfo, DatasetFullMountState,
    DatasetMountedResponse, DatasetsFullMountState, KeyLoadedResponse, RunCommandOutput,
};
use sam_zfs_unlocker::{
    zfs_is_dataset_mounted, zfs_is_key_loaded, zfs_load_key, zfs_mount_dataset,
};

use crate::run_options::config::ApiServerConfig;

use super::{
    command_caller::chain_commands, error::Error, routable_command::RoutableCommand,
    traits::ExecutionBackend,
};

pub struct LiveExecutionBackend {
    config: ApiServerConfig,
    custom_commands_routables: BTreeMap<String, RoutableCommand>,
}

impl LiveExecutionBackend {
    pub fn new(config: ApiServerConfig) -> Self {
        let custom_commands_routables = config
            .custom_commands_config
            .custom_commands
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(RoutableCommand::from)
            .map(|cmd| (cmd.url_endpoint.clone(), cmd))
            .collect::<BTreeMap<_, _>>();

        Self {
            custom_commands_routables,
            config,
        }
    }

    pub fn zfs_enabled(&self) -> bool {
        self.config.zfs_config.zfs_enabled
    }

    pub fn zfs_enabled_or_error(&self) -> Result<(), Error> {
        if !self.zfs_enabled() {
            return Err(Error::ZfsDisabled);
        }
        Ok(())
    }

    pub fn zfs_dataset_blacklisted(&self, dataset_name: impl AsRef<str>) -> bool {
        self.config
            .zfs_config
            .blacklisted_zfs_datasets
            .as_ref()
            .map(|bl| bl.iter().any(|s| s == dataset_name.as_ref()))
            .unwrap_or(false)
    }

    pub fn zfs_dataset_not_blacklisted_or_error(
        &self,
        dataset_name: impl AsRef<str>,
    ) -> Result<(), Error> {
        if self.zfs_dataset_blacklisted(dataset_name.as_ref()) {
            return Err(Error::BlacklistedDataset(dataset_name.as_ref().to_string()));
        }
        Ok(())
    }

    fn internal_get_encrypted_datasets_state(&self) -> Result<DatasetsFullMountState, Error> {
        let config = &self.config.zfs_config;
        if !config.zfs_enabled {
            return Ok(DatasetsFullMountState {
                states: Default::default(),
            });
        }

        let mount_states = sam_zfs_unlocker::zfs_list_encrypted_datasets()?;

        let mount_states = mount_states
            .into_iter()
            .map(|(ds_name, m)| {
                (
                    ds_name,
                    DatasetFullMountState {
                        dataset_name: m.dataset_name,
                        key_loaded: m.is_key_loaded,
                        is_mounted: m.is_mounted,
                    },
                )
            })
            .filter(|(ds_name, _m)| !self.zfs_dataset_blacklisted(ds_name))
            .collect::<BTreeMap<_, _>>();

        Ok(DatasetsFullMountState {
            states: mount_states,
        })
    }
}

#[async_trait]
impl ExecutionBackend for LiveExecutionBackend {
    type Error = super::error::Error;

    fn zfs_encrypted_datasets_state(&self) -> Result<DatasetsFullMountState, Self::Error> {
        let result = self.internal_get_encrypted_datasets_state()?;

        Ok(result)
    }

    fn zfs_encrypted_dataset_state(
        &self,
        dataset_name: impl AsRef<str>,
    ) -> Result<DatasetFullMountState, Self::Error> {
        let dataset_name = dataset_name.as_ref();

        let mut all_datasets_states = self.internal_get_encrypted_datasets_state()?;

        let result = all_datasets_states
            .states
            .remove(dataset_name)
            .ok_or(Error::DatasetNotFound(dataset_name.to_string()))?;

        Ok(result)
    }

    fn zfs_load_key(
        &self,
        dataset_name: impl AsRef<str>,
        passphrase: impl AsRef<str>,
    ) -> Result<KeyLoadedResponse, Self::Error> {
        self.zfs_enabled_or_error()?;

        let dataset_name = dataset_name.as_ref();

        self.zfs_dataset_not_blacklisted_or_error(dataset_name)?;

        if zfs_is_key_loaded(dataset_name)?
            .ok_or(Error::DatasetNotFound(dataset_name.to_string()))?
        {
            return Ok(KeyLoadedResponse {
                dataset_name: dataset_name.to_string(),
                key_loaded: true,
            });
        }

        zfs_load_key(dataset_name, passphrase)?;

        Ok(KeyLoadedResponse {
            dataset_name: dataset_name.to_string(),
            key_loaded: true,
        })
    }

    fn zfs_mount_dataset(
        &self,
        dataset_name: impl AsRef<str>,
    ) -> Result<DatasetMountedResponse, Self::Error> {
        self.zfs_enabled_or_error()?;

        let dataset_name = dataset_name.as_ref();

        self.zfs_dataset_not_blacklisted_or_error(dataset_name)?;

        if zfs_is_dataset_mounted(dataset_name)?
            .ok_or(Error::DatasetNotFound(dataset_name.to_string()))?
        {
            return Ok(DatasetMountedResponse {
                dataset_name: dataset_name.to_string(),
                is_mounted: true,
            });
        }

        if !zfs_is_key_loaded(dataset_name)?
            .ok_or(Error::DatasetNotFound(dataset_name.to_string()))?
        {
            return Err(Error::KeyNotLoadedForDataset(dataset_name.to_string()));
        }

        zfs_mount_dataset(dataset_name)?;

        Ok(DatasetMountedResponse {
            dataset_name: dataset_name.to_string(),
            is_mounted: true,
        })
    }

    fn custom_cmds_list(&self) -> Result<AvailableCustomCommands, Self::Error> {
        let commands = self
            .custom_commands_routables
            .values()
            .map(|c| CustomCommandPublicInfo {
                label: c.label.to_string(),
                endpoint: c.url_endpoint.to_string(),
                stdin_allow: c.stdin_allow,
                stdin_text_placeholder: c.stdin_placeholder_text.to_string(),
                stdin_is_password: c.stdin_is_password,
            })
            .collect::<Vec<_>>();

        let result = AvailableCustomCommands { commands };

        Ok(result)
    }

    fn custom_cmds_routables(&self) -> &BTreeMap<String, RoutableCommand> {
        &self.custom_commands_routables
    }

    async fn custom_cmd_call(
        &self,
        endpoint: &str,
        initial_stdin_input: Option<String>,
    ) -> Result<RunCommandOutput, Self::Error> {
        let cmd = self.custom_commands_routables.get(endpoint).unwrap();

        let result = chain_commands(&cmd.run_cmd, initial_stdin_input).await?;

        Ok(result)
    }

    fn make_error_passphrase_missing(dataset_name: impl Into<String>) -> Self::Error {
        Error::PassphraseNotProvided(dataset_name.into())
    }

    fn make_error_passphrase_non_printable(
        error: impl std::error::Error,
        dataset_name: impl Into<String>,
    ) -> Self::Error {
        Error::NonPrintablePassphrase(error.to_string(), dataset_name.into())
    }

    fn make_error_internetl_custom_command_error(url_endpoint: String) -> Self::Error {
        Error::RegisteredCmdMissing(url_endpoint)
    }
}
