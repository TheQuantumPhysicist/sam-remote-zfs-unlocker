use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:6677";

#[derive(Parser, Clone, Debug, Default)]
pub struct ServerRunOptions {
    /// The address, at which the server will bind to
    #[clap(long, value_name = "ADDR")]
    bind_address: Option<SocketAddr>,

    /// Config file path
    #[clap(long, value_name = "PATH")]
    config_path: PathBuf,
}

impl ServerRunOptions {
    pub fn bind_address(&self) -> SocketAddr {
        self.bind_address.unwrap_or(
            DEFAULT_BIND_ADDRESS
                .parse::<SocketAddr>()
                .expect("Must succeed"),
        )
    }

    pub fn config_path(&self) -> PathBuf {
        self.config_path.clone()
    }
}
