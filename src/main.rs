pub mod chat;
pub mod database_server;
pub mod handle_is_prime;
pub mod handle_mte;
pub mod job_center;

use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::{net::TcpListener, sync::mpsc};
use tracing::{Instrument, info, info_span};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

use crate::job_center::{
    actor_scheduler::manager::JobManager, handle_jobs_center::handle_job_center,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener: TcpListener = TcpListener::bind("0.0.0.0:3030").await?;

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")), // Default to info level
        )
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .pretty()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    static CONN_COUNTER: AtomicUsize = AtomicUsize::new(1);
    let (tx, rx) = mpsc::channel(32);
    let mut job_manager = JobManager::new(rx);
    tokio::spawn(async move {
        if let Err(e) = job_manager.job_actor().await {
            tracing::error!("JobManager exited with error: {:?}", e);
        }
    });

    loop {
        let (socket, addr) = listener.accept().await?;
        let conn_id = CONN_COUNTER.fetch_add(1, Ordering::Relaxed);

        let span = info_span!("conn", %addr, conn_id);
        let tx_clone = tx.clone();

        // Spawn a new async task to handle the connection
        let _ = tokio::spawn(async move {
            info!("New connection established");
            if let Err(e) = handle_job_center(socket, tx_clone).await {
                tracing::error!("Connection {} ended with error: {}", addr, e);
            } else {
                println!("Connection {} ended cleanly", addr);
            }
        })
        .instrument(span);
    }
}
