use anyhow::Context;
use futures::{stream::SplitSink, SinkExt};
use tokio::{net::TcpStream, sync::mpsc::Receiver};
use tokio_util::codec::Framed;
use tracing::info;

use crate::version_control::codec::codec::{Codec, RespValue};


pub async fn response_handler(
    mut writer: SplitSink<Framed<TcpStream, Codec>, RespValue>,
    mut rx: Receiver<RespValue>,
) -> anyhow::Result<()> {
    while let Some(msg) = rx.recv().await {
        info!("Sending {:?}", msg);
        if let Err(e) = writer.send(msg.clone()).await.context("Failed to send response") {
            eprintln!("Writer error ({}), shutting down response handler for msg: {:?}", e, msg);
            break;
        }
    }
    println!("Response handler exiting");
    Ok(())
}