use std::net::SocketAddr;

use futures::{StreamExt, stream::SplitStream};
use tokio::{
    net::TcpStream,
    sync::{mpsc::Sender, oneshot},
};
use tokio_util::codec::Framed;
use tracing::{info, warn};

use crate::version_control::{codec::codec::{Codec, RespValue, VersionControlCommand}, handler_version_control::FileManagerSender};

pub async fn handle_request(
    mut reader: SplitStream<Framed<TcpStream, Codec>>,
    version_control_sender: FileManagerSender,
    writer_tx: &mut Sender<RespValue>,
) -> anyhow::Result<()> {
    while let Some(command) = reader.next().await.transpose()? {
        info!("Received {:?}", command);
        match command {
            VersionControlCommand::Help => {
                let (tx, rx) = oneshot::channel::<RespValue>();

                version_control_sender.send((command, tx)).await?;
                let resp = rx.await?;
                writer_tx.send(resp).await?;
                writer_tx.send(RespValue::Text("READY".to_string())).await?
            }
            other => {
                let (tx, rx) = oneshot::channel::<RespValue>();

                version_control_sender.send((other, tx)).await?;
                let resp = rx.await?;
                writer_tx.send(resp).await?;
                writer_tx.send(RespValue::Text("READY".to_string())).await?
            }
        }
    }
    Ok(())
}
