pub mod server_run_options;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct RunOptions {
    #[clap(subcommand)]
    pub command: RunCommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum RunCommand {
    /// Run the server
    Server(server_run_options::ServerRunOptions),
}
