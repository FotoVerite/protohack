pub mod chat;
pub mod handle_is_prime;
pub mod handle_mte;
pub mod database_server;

use std::{collections::HashMap, sync::Arc};

use prime_time::road::{handle_road::handle_road, plate::PlateStorage};
use tokio::{
    net::TcpListener,
    sync::Mutex,
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener: TcpListener = TcpListener::bind("0.0.0.0:3030").await?;
    tracing_subscriber::fmt()
        .with_env_filter("my_server=debug,tokio=info")
        .init();
    let plate_storage = Arc::new(Mutex::new(PlateStorage::new()));
        let dispatchers = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, addr) = listener.accept().await?;
        let plate_storage_clone = plate_storage.clone();
        let dispatchers_clone = dispatchers.clone();
        info!("new connection from {}", addr);
        // Spawn a new async task to handle the connection
        tokio::spawn(async move {
            println!("Connection started for {}", addr);
            if let Err(e) = handle_road(socket, dispatchers_clone, plate_storage_clone, ).await {
                eprintln!("Connection {} ended with error: {:?}", addr, e);
            } else {
                println!("Connection {} ended cleanly", addr);
            }
        });
    }
}
// async fn main() -> anyhow::Result<()> {
//     tracing_subscriber::fmt()
//         .with_env_filter("my_server=debug,tokio=info")
//         .init();
//     let storage: Arc<Mutex<Storage>> = Arc::new(Mutex::new(HashMap::new()));
//     database_server::run_udp_server("0.0.0.0:3030", storage).await?;
//     Ok(())
    
// }


