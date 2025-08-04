pub mod chat;
pub mod database_server;
pub mod handle_is_prime;
pub mod handle_mte;

use std::sync::atomic::{AtomicUsize, Ordering};

use prime_time::job_center::{
    handle_jobs_center::handle_job_center, scheduler::manager::JobManager,
};
use tokio::net::TcpListener;
use tracing::{Instrument, info, info_span};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

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
    let id_counter = std::sync::Arc::new(AtomicUsize::new(1));
    let job_manager = JobManager::new();

    loop {
        let (socket, addr) = listener.accept().await?;
        let conn_id = CONN_COUNTER.fetch_add(1, Ordering::Relaxed);

        let span = info_span!("conn", %addr, conn_id);
        let id_counter_clone = id_counter.clone();

        let job_manager_clone = job_manager.clone();

        // Spawn a new async task to handle the connection
        let _ = tokio::spawn(async move {
            info!("Connection started for");
            if let Err(e) = handle_job_center(socket, job_manager_clone, id_counter_clone).await {
                tracing::error!("Connection {} ended with error: {}", addr, e);
            } else {
                println!("Connection {} ended cleanly", addr);
            }
        })
        .instrument(span);
    }
}
