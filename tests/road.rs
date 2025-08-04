use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use prime_time::road::{handle_road::handle_road, plate::PlateStorage, Plates, RoadDispatchers};
use tokio_util::bytes::BufMut;
use crate::server_harness::{Server, ServerHarness};

mod server_harness;
mod test_util;

struct RoadServer;

impl Server for RoadServer {
    fn run(listener: TcpListener) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async move {
            let dispatchers: RoadDispatchers = Arc::new(Mutex::new(HashMap::new()));
            let plate_storage: Plates = Arc::new(Mutex::new(PlateStorage::new()));
    
            loop {
                let (socket, _addr) = listener.accept().await?;
                let d = dispatchers.clone();
                let p = plate_storage.clone();
                tokio::spawn(async move {
                    handle_road(socket, d, p).await
                });
            }
        }
    }
}

#[tokio::test]
async fn test_road_server() {
    let harness = ServerHarness::<RoadServer>::new().await;
    let mut client = test_util::TestClient::connect(&harness.endpoint()).await.unwrap();

    // Send a WantHeartbeat message
    let mut buf = [0u8; 5];
    buf[0] = 0x40;
    (&mut buf[1..]).put_u32(123456);
    client.send_bytes(&buf).await.unwrap();

    // Read the response
    // let response = client.read_exact(1).await.unwrap();
    // assert_eq!(response[0], 0x41);

    // Close the connection
    drop(client);
}
