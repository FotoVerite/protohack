use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;

// Generic server trait
pub trait UdpServer {
    fn run(socket: UdpSocket) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub struct UdpServerHarness<T: UdpServer> {
    pub addr: SocketAddr,
    handle: JoinHandle<anyhow::Result<()>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: UdpServer + Send + 'static> UdpServerHarness<T> {
    pub async fn new() -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            T::run(socket).await
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

impl<T: UdpServer> Drop for UdpServerHarness<T> {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
