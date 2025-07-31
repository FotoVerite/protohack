use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::{net::UdpSocket, sync::Mutex};
use tracing::{error, info};

pub type Storage = HashMap<String, String>;

pub async fn run_udp_server(addr: &str, storage: Arc<Mutex<Storage>>) -> anyhow::Result<()> {
    let socket = UdpSocket::bind(addr).await?;

    loop {
        // recv_from returns (len, sender addr)
        let mut buf = [0u8; 1000];
        let (len, addr) = socket.recv_from(&mut buf).await?;
        if let Ok(text) = std::str::from_utf8(&buf[..len]) {
            println!("Text: {}", text);
            if text.contains("=") {
                if let Some((key, value)) = text.split_once("=") {
                    info!("Storing key: {} with value len: {}", key, value.len());
                    let mut guard = storage.lock().await; // Use key and value here
                    let _ = guard.insert(key.to_string(), value.to_string());
                    drop(guard)
                }
            } else {
                if text == "version" {
                    socket
                        .send_to("version=Ken's Key-Value Store 1.0".as_bytes(), &addr)
                        .await?;
                } else {
                    let guard = storage.lock().await; // Use key and value here
                    match guard.get(text) {
                        Some(value) => {
                            socket
                                .send_to(format!("{}={}", text, value).as_bytes(), &addr)
                                .await?;
                        }
                        None => {
                            socket.send_to(text.as_bytes(), &addr).await?;
                        }
                    }
                }
            }
        } else {
            println!("Non-UTF8 data received");
        }
    }
}
