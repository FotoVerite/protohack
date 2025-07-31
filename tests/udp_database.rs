use prime_time::database_server;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::UdpSocket, sync::Mutex, time::timeout};

type Storage = HashMap<String, String>;

async fn start_server(addr: &str, storage: Arc<Mutex<Storage>>) {
    let storage = storage.clone();
    let addr = addr.to_string();
    tokio::spawn(async move {
        database_server::run_udp_server(&addr, storage)
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
}

async fn setup_client(addr: &str) -> anyhow::Result<UdpSocket> {
    let client = UdpSocket::bind("0.0.0.0:0").await?;
    client.connect(addr).await?;
    Ok(client)
}

async fn recv_with_timeout(socket: &UdpSocket) -> anyhow::Result<Option<String>> {
    let mut buf = [0u8; 1000];
    match timeout(Duration::from_millis(200), socket.recv(&mut buf)).await {
        Ok(Ok(len)) => Ok(Some(std::str::from_utf8(&buf[..len])?.to_string())),
        _ => Ok(None),
    }
}

#[tokio::test]
async fn test_basic_set_and_get() -> anyhow::Result<()> {
    let addr = "127.0.0.1:4000";
    let storage = Arc::new(Mutex::new(HashMap::new()));
    start_server(addr, storage.clone()).await;

    let client = setup_client(addr).await?;
    client.send(b"foo=bar").await?;

    // No response expected from SET
    assert!(
        recv_with_timeout(&client).await?.is_none(),
        "SET should not return a response"
    );

    client.send(b"foo").await?;
    let resp = recv_with_timeout(&client).await?.unwrap();
    assert_eq!(resp, "foo=bar");

    Ok(())
}

#[tokio::test]
async fn test_unknown_key() -> anyhow::Result<()> {
    let addr = "127.0.0.1:4001";
    let storage = Arc::new(Mutex::new(HashMap::new()));
    start_server(addr, storage.clone()).await;

    let client = setup_client(addr).await?;
    client.send(b"missingkey").await?;
    let resp = recv_with_timeout(&client).await?.unwrap();
    assert_eq!(resp, "missingkey");

    Ok(())
}

#[tokio::test]
async fn test_version() -> anyhow::Result<()> {
    let addr = "127.0.0.1:4002";
    let storage = Arc::new(Mutex::new(HashMap::new()));
    start_server(addr, storage.clone()).await;

    let client = setup_client(addr).await?;
    client.send(b"version").await?;
    let resp = recv_with_timeout(&client).await?.unwrap();
    assert_eq!(resp, "version=Ken's Key-Value Store 1.0");

    Ok(())
}

#[tokio::test]
async fn test_insert_json_string() -> anyhow::Result<()> {
    let addr = "127.0.0.1:4002";
    let storage = Arc::new(Mutex::new(HashMap::new()));
    start_server(addr, storage.clone()).await;

    let client = setup_client(addr).await?;

    let key = "product.68620";
    let val = r#"{"price":"197267.76","delivery":"8 business days","name":"23 GB spirit level bubble (M6 thread) (415-pack)","stock":374}"#;

    let insert = format!("{key}={val}");
    client.send(insert.as_bytes()).await?;

    // No response expected
    let mut buf = [0u8; 1024];
    let recv = timeout(Duration::from_millis(200), client.recv(&mut buf)).await;
    assert!(recv.is_err(), "SET should not return a response");

    // Retrieve
    client.send(key.as_bytes()).await?;
    let len = timeout(Duration::from_millis(200), client.recv(&mut buf)).await??;
    let resp = std::str::from_utf8(&buf[..len])?;

    assert_eq!(resp, insert);

    Ok(())
}
