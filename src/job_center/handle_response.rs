use anyhow::Context;
use futures::SinkExt;
use futures::stream::SplitSink;
use tokio::{net::TcpStream, sync::mpsc::Receiver}; 
use tokio_util::codec::{Framed, LinesCodec};
use tracing::info;

use crate::job_center::Response;

pub async fn response_handler(
    mut writer: SplitSink<Framed<TcpStream, LinesCodec>, String>,
    mut rx: Receiver<Response>,
) -> anyhow::Result<()> {
    while let Some(msg) = rx.recv().await {
        info!("Sending {:?}", msg);
        let string_message = serde_json::to_string(&msg)?;

        if let Err(e) = writer
            .send(string_message)
            .await
            .context("Failed to send response")
        {
            eprintln!(
                "Writer error ({}), shutting down response handler for msg: {:?}",
                e, msg
            );
            break;
        }
    }

    println!("Response handler exiting");
    Ok(())
}
