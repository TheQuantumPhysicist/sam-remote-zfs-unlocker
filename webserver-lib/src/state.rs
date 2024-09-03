use crate::{
    backend::traits::ExecutionBackend,
    run_options::config::{CustomCommandsConfig, ZfsConfig},
};

pub struct ServerState<B: ExecutionBackend> {
    pub zfs_config: ZfsConfig,
    pub custom_commands_config: CustomCommandsConfig,
    pub backend: B,
}

#[allow(clippy::new_without_default)]
impl<B: ExecutionBackend> ServerState<B> {
    pub fn new(
        zfs_config: ZfsConfig,
        custom_commands_config: CustomCommandsConfig,
        backend: B,
    ) -> Self {
        Self {
            zfs_config,
            custom_commands_config,
            backend,
        }
    }
}
