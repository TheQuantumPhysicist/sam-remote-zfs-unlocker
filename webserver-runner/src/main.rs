use clap::Parser;

use api_server::{run_options::RunOptions, start_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = RunOptions::parse();

    log::info!("Starting server...");

    match args.command {
        api_server::run_options::RunCommand::Server(s) => start_server(s).await,
    }
}
