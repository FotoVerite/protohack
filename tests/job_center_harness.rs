use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tracing_subscriber::{EnvFilter, fmt::TestWriter};
use std::sync::Once;

static TRACING_INIT: Once = Once::new();

pub trait JobServer {
    fn run(listener: TcpListener) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub struct JobCenterHarness<T: JobServer> {
    pub addr: SocketAddr,
    handle: JoinHandle<anyhow::Result<()>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: JobServer + Send + 'static> JobCenterHarness<T> {
    pub async fn new() -> Self {
        // Initialize tracing once
        TRACING_INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::new("trace"))
                .with_writer(TestWriter::new())
                .init();
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            tracing::info!("Starting test server on {}", addr);
            T::run(listener).await
        });

        Self {
            addr,
            handle,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn endpoint(&self) -> String {
        format!("localhost:{}", self.addr.port())
    }
}