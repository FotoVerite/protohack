use std::net::SocketAddr;

use futures::{StreamExt, stream::SplitStream};
use tokio::{
    net::TcpStream,
    sync::{mpsc::Sender, oneshot},
};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{info, warn};

use crate::job_center::{
    Request, Response,
    actor_scheduler::{actor::JobCommand, job::Job},
};

pub async fn handle_request(
    mut reader: SplitStream<Framed<TcpStream, LinesCodec>>,
    peer_address: &SocketAddr,
    job_command_sender: Sender<JobCommand>,
    writer_tx: &mut Sender<Response>,
) -> anyhow::Result<()> {
    let mut working_on = vec![];
    while let Some(line) = reader.next().await.transpose()? {
        info!("Received {}", line);
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to parse request from {}: {}", peer_address, e);
                writer_tx
                    .send(Response::Error {
                        error: format!("Invalid request: {}", e),
                    })
                    .await?;
                continue; // skip to next line
            }
        };
        match &req {
            Request::Get { queues, wait } => {
                let (tx, rx) = oneshot::channel::<anyhow::Result<Option<(usize, Job)>>>();
                job_command_sender
                    .send(JobCommand::Get {
                        queues: queues.clone(),
                        addr: *peer_address,
                        wait: wait.unwrap_or(false),
                        resp: tx,
                    })
                    .await
                    .expect("Failed to send job command");
                if let Some((id, job)) = rx.await?? {
                    working_on.push(id);
                    writer_tx
                        .send(Response::Ok {
                            id: Some(id),
                            job: Some(job.to_serd()),
                        })
                        .await?;
                } else {
                    writer_tx.send(Response::NoJob {}).await?
                }
            }
            Request::Put { queue, job, pri } => {
                let (tx, rx) = oneshot::channel::<anyhow::Result<usize>>();
                job_command_sender
                    .send(JobCommand::Put {
                        queue: queue.clone(),
                        job: job.clone(),
                        pri: pri.clone(),
                        resp: tx,
                    })
                    .await
                    .expect("Failed to send job command");
                let id = rx.await??;
                writer_tx
                    .send(Response::Ok {
                        id: Some(id),
                        job: None,
                    })
                    .await?;
            }

            Request::Delete { id } => {
                let (tx, rx) = oneshot::channel::<anyhow::Result<bool>>();

                job_command_sender
                    .send(JobCommand::Delete {
                        id: id.clone(),
                        resp: tx,
                    })
                    .await
                    .expect("Failed to send job command");

                if rx.await?? {
                    writer_tx
                        .send(Response::Ok {
                            id: None,
                            job: None,
                        })
                        .await?;
                } else {
                    writer_tx.send(Response::NoJob {}).await?;
                }
            }

            Request::Abort { id } => {
                let (tx, rx) = oneshot::channel::<anyhow::Result<bool>>();

                job_command_sender
                    .send(JobCommand::Abort {
                        id: id.clone(),
                        addr: *peer_address,
                        resp: tx,
                    })
                    .await
                    .expect("Failed to send job command");
                let result = rx.await?;
                match result {
                    Ok(response) => {
                        if response {
                            writer_tx
                                .send(Response::Ok {
                                    id: None,
                                    job: None,
                                })
                                .await?;
                        } else {
                            writer_tx.send(Response::NoJob {}).await?;
                        }
                    }
                    Err(e) => {
                        writer_tx
                            .send(Response::Error {
                                error: e.to_string(),
                            })
                            .await?;
                    }
                }
            }
        }
    }
    for id in working_on {
        let (tx, _) = oneshot::channel::<anyhow::Result<bool>>();

        job_command_sender
            .send(JobCommand::Abort {
                id: id,
                addr: *peer_address,
                resp: tx,
            })
            .await?;
    }
    Ok(())
}
