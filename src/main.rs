use std::{
    net::{SocketAddr, SocketAddrV4},
    str::FromStr,
};

use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // TODO: Fill this from clap
    let listener_socket = TcpListener::bind(SocketAddr::V4(
        SocketAddrV4::from_str("127.0.0.1:6666").unwrap(),
    ))
    .await
    .expect("Valid listening address");

    // TODO: set permissive from clap
    api_server::web_server(listener_socket, true)
        .await
        .expect("Failed to start web server")
}
