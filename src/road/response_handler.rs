use anyhow::Context;
use futures::{stream::SplitSink, SinkExt};
use tokio::{net::TcpStream, sync::mpsc::Receiver};
use tokio_util::codec::Framed;

use crate::road::codec::{Codec, RespValue};

pub async fn response_handler(
    mut writer: SplitSink<Framed<TcpStream, Codec>, RespValue>,
    mut rx: Receiver<RespValue>,
) -> anyhow::Result<()> {
    while let Some(msg) = rx.recv().await {
        println!("sending {:?}", msg);
        if let Err(e) = writer.send(msg).await.context("Failed to send response") {
            eprintln!("Writer error ({}), shutting down response handler", e);
            break;
        }
    }
    println!("Response handler exiting");
    Ok(())
}