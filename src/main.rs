use clap::Parser;
use log::LevelFilter::Debug;
#[allow(unused_imports)]
use log::{debug, error, info, warn};

use core::server::{GameServer, ServerArgs};

mod core;
mod game_object;
mod game_system;
mod network_handler;
mod network_message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logging
    env_logger::builder().filter_level(Debug).init();

    let args: ServerArgs = ServerArgs::parse();
    info!("ServerArgs: {:?}", args);

    let mut game_server: GameServer;
    let game_server_result = GameServer::new(args).await;
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
