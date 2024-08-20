use clap::Parser;

use api_server::{run_options::RunOptions, start_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = RunOptions::parse();

    match args.command {
        api_server::run_options::RunCommand::Server(s) => start_server(s).await,
    }
}
