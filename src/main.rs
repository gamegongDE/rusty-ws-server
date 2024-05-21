use log::LevelFilter::Debug;
#[allow(unused_imports)]
use log::{debug, error, info, warn};

use core::server::GameServer;
use std::net::SocketAddr;

use crate::core::args::{Args, Parser};

mod core;
mod handler;
mod msg;
mod object;
mod system;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logging
    env_logger::builder().filter_level(Debug).init();

    // Parse args
    let args: Args = Args::parse();
    let addr = format!("0.0.0.0:{}", args.port);
    let socket_addr = addr.parse::<SocketAddr>().expect("Can't parse address");

    let mut game_server: GameServer;
    let game_server_result = GameServer::new(socket_addr).await;
    match game_server_result {
        Ok(server) => {
            game_server = server;
        }
        Err(e) => {
            error!("Error creating server: {}", e);
            return Ok(());
        }
    }
    let game_server_result = game_server.run().await;
    match game_server_result {
        Ok(()) => {
            info!("Server ended");
        }
        Err(e) => {
            error!("Error running server: {}", e);
            return Ok(());
        }
    }

    Ok(())
}
