
use futures::StreamExt;
use tokio::{net::TcpStream, sync::mpsc::{self, Sender}};
use tokio_util::codec::{Framed, LinesCodec};

use crate::job_center::{actor_scheduler::actor::JobCommand, handle_request::handle_request, handle_response::response_handler};


pub async fn handle_job_center(
    socket: TcpStream,
    job_command_sender: Sender<JobCommand>
) -> anyhow::Result<()> {
    let addr = socket.peer_addr()?;

    let framed: Framed<TcpStream, LinesCodec> = Framed::new(socket, LinesCodec::new());
    let (writer, reader) = framed.split();

    let (mut tx, rx) = mpsc::channel(1_000);

    let writer_task = { tokio::spawn(async move { response_handler(writer,  rx).await }) };

    let reader_task = {
        tokio::spawn(async move {
            handle_request(reader, &addr, job_command_sender, &mut tx).await
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
