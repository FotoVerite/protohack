pub mod chat;
pub mod handle_is_prime;
pub mod handle_mte;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::chat::handle_chat::handle_chat;
use tokio::{
    net::TcpListener,
    sync::{Mutex, broadcast},
};
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener: TcpListener = TcpListener::bind("0.0.0.0:3030").await?;
    tracing_subscriber::fmt()
        .with_env_filter("my_server=debug,tokio=info")
        .init();
    let (tx, _rx) = broadcast::channel::<(SocketAddr, String)>(100);
    let sender = Arc::new(tx);
    let users = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (socket, addr) = listener.accept().await?;
        let handler_tx = sender.clone();
        let handler_users = users.clone();
        info!("new connection from {}", addr);
        // Spawn a new async task to handle the connection
        tokio::spawn(async move {
            println!("Connection started for {}", addr);
            if let Err(e) = handle_chat(socket, &handler_tx, handler_users).await {
                eprintln!("Connection {} ended with error: {:?}", addr, e);
            } else {
                println!("Connection {} ended cleanly", addr);
            }
        });
    }
}
