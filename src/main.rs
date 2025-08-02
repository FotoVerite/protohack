pub mod chat;
pub mod database_server;
pub mod handle_is_prime;
pub mod handle_mte;

use std::sync::atomic::{AtomicUsize, Ordering};

use prime_time::crypto::handle_crypto::handle_cipher;
use tokio::net::TcpListener;
use tracing::{Instrument, info, info_span};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener: TcpListener = TcpListener::bind("0.0.0.0:3030").await?;

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()) // allows dynamic filtering
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE) // logs when spans open/close
        .pretty() // optional: makes output human-friendly
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    static CONN_COUNTER: AtomicUsize = AtomicUsize::new(1);

    loop {
        let (socket, addr) = listener.accept().await?;
        let conn_id = CONN_COUNTER.fetch_add(1, Ordering::Relaxed);

        let span = info_span!("conn", %addr, conn_id);

        // Spawn a new async task to handle the connection
        let _ = tokio::spawn(async move {
            info!("Connection started for");
            if let Err(e) = handle_cipher(socket).await {
                eprintln!("Connection {} ended with error: {:?}", addr, e);
            } else {
                println!("Connection {} ended cleanly", addr);
            }
        })
        .instrument(span);
    }
}
