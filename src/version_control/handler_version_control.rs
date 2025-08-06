use futures::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, Sender},
    sync::oneshot
};
use tokio_util::codec::{Framed};

use crate::version_control::{
    codec::codec::{Codec, RespValue, VersionControlCommand}, request_handler::handle_request, response_handler::response_handler,
};

pub type  FileManagerSender = Sender<(VersionControlCommand, oneshot::Sender<RespValue>)>;

pub async fn handle_version_control(
    socket: TcpStream,
    version_control_manager: FileManagerSender,
) -> anyhow::Result<()> {
    let addr = socket.peer_addr()?;

    let mut framed = Framed::new(socket, Codec);
    framed.send(RespValue::Text("READY".to_string())).await?;
    let (writer, reader) = framed.split();

    let (mut tx, rx) = mpsc::channel(1_000);

    let writer_task = { tokio::spawn(async move { response_handler(writer, rx).await }) };

    let reader_task = {
        tokio::spawn(
            async move { handle_request(reader, version_control_manager, &mut tx).await },
        )
    };

    // FIXED: Wait for BOTH tasks to complete before exiting
    let (reader_result, writer_result) = tokio::join!(reader_task, writer_task);

    // Handle results properly
    match reader_result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("Reader error: {:?} for {}", e, addr),
        Err(e) => eprintln!("Reader task panicked: {:?}", e),
    }

    match writer_result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("Writer error: {:?} for {}", e, addr),
        Err(e) => eprintln!("Writer task panicked: {:?}", e),
    }

    Ok(())
}
