#[warn(dead_code)]

use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, WriteHalf, ReadHalf, AsyncReadExt};
use tokio::io::split;
pub struct TestClient {
    writer: WriteHalf<TcpStream>,
    reader: BufReader<ReadHalf<TcpStream>>,
}

impl TestClient {
    pub async fn connect(addr: &str) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = split(stream);
        Ok(Self {
            writer: write_half,
            reader: BufReader::new(read_half),
        })
    }
    
    pub async fn send(&mut self, msg: &str) -> anyhow::Result<()> {
        self.writer.write_all(msg.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn send_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn read_line(&mut self) -> anyhow::Result<String> {
        let mut buf = String::new();
        let n = self.reader.read_line(&mut buf).await?;
        if n == 0 {
            anyhow::bail!("Connection closed");
        }
        Ok(buf)
    }

    pub async fn read_exact(&mut self, n: usize) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.reader.read_exact(&mut buf).await?;
        Ok(buf)
    }
}