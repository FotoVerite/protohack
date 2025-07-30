use std::collections::{BTreeMap};

use anyhow;
use std::ops::Bound::Included;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::error;

pub async fn handle_is_mte(stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut buf = [0u8; 9];
    let mut store = BTreeMap::new();
    loop {
        match stream.read_exact(&mut buf).await {
            Ok(_req) => {
                let method_type = buf[0];
                let arg_1 = i32::from_be_bytes(buf[1..5].try_into()?);
                let arg_2 = i32::from_be_bytes(buf[5..9].try_into()?);
                match method_type {
                    0x49 => {
                        store.insert(arg_1, arg_2);
                    }
                    0x51 => {
                        if arg_1 > arg_2  {
                           stream.write_all(&0i32.to_be_bytes()).await?;
                           continue
                        }
                        let range = store.range((Included(arg_1), Included(arg_2)));
                        let (sum, count) = range
                            .fold((0i64, 0), |(sum, count), (_timestamp, price)| {
                                (sum + *price as i64, count + 1)
                            });

                        let mean = if count > 0 { (sum / count) as i32 } else { 0 };
                        stream.write_all(&mean.to_be_bytes()).await?;
                    }
                    _ => {
                        error!("Unknown request");
                        return Err(anyhow::anyhow!("Unknown request"));
                    }
                }
            }
            Err(e) => {
                error!(?e, "error reading response")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        net::TcpStream,
    };

    // Builds a 9-byte insert message
    fn build_insert(timestamp: i32, price: i32) -> [u8; 9] {
        let mut buf = [0u8; 9];
        buf[0] = b'I';
        buf[1..5].copy_from_slice(&timestamp.to_be_bytes());
        buf[5..9].copy_from_slice(&price.to_be_bytes());
        buf
    }

    // Builds a 9-byte query message
    fn build_query(mintime: i32, maxtime: i32) -> [u8; 9] {
        let mut buf = [0u8; 9];
        buf[0] = b'Q';
        buf[1..5].copy_from_slice(&mintime.to_be_bytes());
        buf[5..9].copy_from_slice(&maxtime.to_be_bytes());
        buf
    }

    // Parse the 4-byte response as i32
    fn parse_response(resp: &[u8]) -> i32 {
        let arr: [u8; 4] = resp.try_into().unwrap();
        i32::from_be_bytes(arr)
    }

    // Helper to start the server in background on ephemeral port and return address
    async fn start_server() -> anyhow::Result<std::net::SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    if let Err(e) = handle_is_mte(&mut socket).await {
                        eprintln!("server error: {:?}", e);
                    }
                });
            }
        });

        // Give server a tiny moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(addr)
    }

    #[tokio::test]
    async fn test_insert_and_query_basic() -> anyhow::Result<()> {
        let addr = start_server().await?;

        let mut stream = TcpStream::connect(addr).await?;

        // Insert timestamp=12345 price=101
        stream.write_all(&build_insert(12345, 101)).await?;

        // Insert timestamp=12346 price=102
        stream.write_all(&build_insert(12346, 102)).await?;

        // Query average from 12345 to 12346 inclusive
        stream.write_all(&build_query(12345, 12346)).await?;

        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await?;

        let avg = parse_response(&buf);

        assert!(avg == 101 || avg == 102);

        Ok(())
    }

    #[tokio::test]
    async fn test_query_empty_returns_zero() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let mut stream = TcpStream::connect(addr).await?;

        // Query with no inserts should return 0
        stream.write_all(&build_query(0, 1000)).await?;

        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await?;

        let avg = parse_response(&buf);

        assert_eq!(avg, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_query_invalid_range_returns_zero() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let mut stream = TcpStream::connect(addr).await?;

        // Insert one data point
        stream.write_all(&build_insert(10, 50)).await?;

        // Query where mintime > maxtime
        stream.write_all(&build_query(100, 50)).await?;

        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await?;

        let avg = parse_response(&buf);
        assert_eq!(avg, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_negative_price_and_query() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let mut stream = TcpStream::connect(addr).await?;

        stream.write_all(&build_insert(500, -20)).await?;
        stream.write_all(&build_insert(600, 40)).await?;

        stream.write_all(&build_query(400, 700)).await?;

        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await?;

        let avg = parse_response(&buf);
        assert_eq!(avg, 10);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_queries_after_inserts() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let mut stream = TcpStream::connect(addr).await?;

        for (ts, price) in &[(1, 100), (2, 200), (3, 300)] {
            stream.write_all(&build_insert(*ts, *price)).await?;
        }

        // Query full range
        stream.write_all(&build_query(1, 3)).await?;
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await?;
        let avg = parse_response(&buf);
        assert_eq!(avg, 200);

        // Query partial range
        stream.write_all(&build_query(2, 2)).await?;
        stream.read_exact(&mut buf).await?;
        let avg2 = parse_response(&buf);
        assert_eq!(avg2, 200);

        // Query out of range
        stream.write_all(&build_query(10, 20)).await?;
        stream.read_exact(&mut buf).await?;
        let avg3 = parse_response(&buf);
        assert_eq!(avg3, 0);

        Ok(())
    }
}
