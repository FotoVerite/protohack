use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{net::UdpSocket, sync::Mutex, time::timeout};
use prime_time::database_server::{self, run_udp_server_with_socket};
use crate::udp_server_harness::{UdpServer, UdpServerHarness};

mod udp_server_harness;

struct UdpDatabaseServer;

impl UdpServer for UdpDatabaseServer {
    fn run(socket: UdpSocket) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async move {
            let storage = Arc::new(Mutex::new(HashMap::new()));
            run_udp_server_with_socket(socket, storage).await
        }
    }
}

struct UdpTestClient {
    socket: UdpSocket,
}

impl UdpTestClient {
    async fn new(addr: &str) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(addr).await?;
        Ok(Self { socket })
    }

    async fn send(&self, data: &[u8]) -> anyhow::Result<()> {
        self.socket.send(data).await?;
        Ok(())
    }

    async fn recv_with_timeout(&self) -> anyhow::Result<Option<String>> {
        let mut buf = [0u8; 1000];
        match timeout(Duration::from_millis(200), self.socket.recv(&mut buf)).await {
            Ok(Ok(len)) => Ok(Some(std::str::from_utf8(&buf[..len])?.to_string())),
            _ => Ok(None),
        }
    }
}

#[tokio::test]
async fn test_basic_set_and_get() -> anyhow::Result<()> {
    let harness = UdpServerHarness::<UdpDatabaseServer>::new().await;
    let client = UdpTestClient::new(&harness.endpoint()).await?;

    client.send(b"foo=bar").await?;

    // No response expected from SET
    assert!(
        client.recv_with_timeout().await?.is_none(),
        "SET should not return a response"
    );

    client.send(b"foo").await?;
    let resp = client.recv_with_timeout().await?.unwrap();
    assert_eq!(resp, "foo=bar");

    Ok(())
}

#[tokio::test]
async fn test_unknown_key() -> anyhow::Result<()> {
    let harness = UdpServerHarness::<UdpDatabaseServer>::new().await;
    let client = UdpTestClient::new(&harness.endpoint()).await?;

    client.send(b"missingkey").await?;
    let resp = client.recv_with_timeout().await?.unwrap();
    assert_eq!(resp, "missingkey");

    Ok(())
}

#[tokio::test]
async fn test_version() -> anyhow::Result<()> {
    let harness = UdpServerHarness::<UdpDatabaseServer>::new().await;
    let client = UdpTestClient::new(&harness.endpoint()).await?;

    client.send(b"version").await?;
    let resp = client.recv_with_timeout().await?.unwrap();
    assert_eq!(resp, "version=Ken's Key-Value Store 1.0");

    Ok(())
}