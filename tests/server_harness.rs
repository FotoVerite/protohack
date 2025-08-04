use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

// Generic server trait
pub trait Server {
    fn run(listener: TcpListener) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub struct ServerHarness<T: Server> {
    pub addr: SocketAddr,
    handle: JoinHandle<anyhow::Result<()>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Server + Send + 'static> ServerHarness<T> {
    pub async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
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

impl<T: Server> Drop for ServerHarness<T> {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
