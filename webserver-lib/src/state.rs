use crate::run_options::config::ZfsConfig;

pub struct ServerState {
    pub zfs_config: ZfsConfig,
}

#[allow(clippy::new_without_default)]
impl ServerState {
    pub fn new(zfs_config: ZfsConfig) -> Self {
        Self { zfs_config }
    }
}
