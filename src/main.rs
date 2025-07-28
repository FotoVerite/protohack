
pub mod handle_is_prime;
use crate::handle_is_prime::handle_is_prime;
use tokio::{io::AsyncWriteExt, net::TcpListener};
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener: TcpListener = TcpListener::bind("127.0.0.1:3030").await?;
    tracing_subscriber::fmt()
        .with_env_filter("my_server=debug,tokio=info")
        .init();
    loop {
        let (mut socket, addr) = listener.accept().await?;
        info!("new connection from {}", addr);
        // Spawn a new async task to handle the connection
        tokio::spawn(async move {
            if let Err(e) = handle_is_prime(&mut socket).await {
                error!("Error handling {}: {:?}", addr, e);
                let _ = socket.shutdown().await;
            }
            // Use `socket` to read/write asynchronously here
        });
    }
}
