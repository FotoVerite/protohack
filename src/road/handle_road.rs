use futures::StreamExt;
use tokio::{net::TcpStream, sync::mpsc};
use tokio_util::codec::Framed;

use crate::road::{
    Plates, RoadDispatchers, codec::Codec, request_handler::handle_request,
    response_handler::response_handler,
};

pub async fn handle_road(
    socket: TcpStream,
    mut dispatchers: RoadDispatchers,
    plate_storage: Plates,
) -> anyhow::Result<()> {
    let addr = socket.peer_addr()?;

    let framed: Framed<TcpStream, Codec> = Framed::new(socket, Codec);
    let (writer, reader) = framed.split();

    let (tx, rx) = mpsc::channel(1_000);

    let writer_task = { tokio::spawn(async move { response_handler(writer, rx).await }) };

    let reader_task = {
        tokio::spawn(async move {
            handle_request(reader, addr, &mut dispatchers, plate_storage, tx).await
        })
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
