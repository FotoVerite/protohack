use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::io::split;

pub struct BinaryTestClient {
    writer: WriteHalf<TcpStream>,
    reader: ReadHalf<TcpStream>,
}

impl BinaryTestClient {
    pub async fn connect(addr: &str) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = split(stream);
        Ok(Self {
            writer: write_half,
            reader: read_half,
        })
    }

    pub async fn send_bytes(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(buf).await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// Read exactly `n` bytes.
    pub async fn read_exact(&mut self, n: usize) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.reader.read_exact(&mut buf).await?;
        Ok(buf)
    }

    /// Read up to `n` bytes (can return fewer).
    pub async fn read_some(&mut self, n: usize) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        let read = self.reader.read(&mut buf).await?;
        buf.truncate(read);
        Ok(buf)
    }
}