use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use futures::{ StreamExt, stream::SplitStream};
use tokio::{
    net::TcpStream,
    sync::{mpsc::Sender},
};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{info, warn};

use crate::job_center::{scheduler::manager::JobManager, Request, Response};

pub async fn handle_request(
    mut reader: SplitStream<Framed<TcpStream, LinesCodec>>,
    peer_address: &SocketAddr,
    mut job_manager: JobManager,
    job_counter: Arc<AtomicUsize>,
    tx: &mut Sender<Response>,
) -> anyhow::Result<()> {
    while let Some(line) = reader.next().await.transpose()? {
        info!("Received {}", line);
        let req: Request = serde_json::from_str(&line)?;
        let result = match &req {
            Request::Put { queue, job, pri } => {
                let uuid = job_counter.fetch_add(1, Ordering::Relaxed);
                job_manager.add_job(uuid, queue.clone(), pri, job).await;
                tx.send(Response::Ok {
                    id: Some(uuid),
                    job: None,
                })
                .await
            }
            Request::Get { queues, wait } => {
                let mut waiter = None;
                if let Some(job) = job_manager
                    .get_job(queues.clone(), waiter, peer_address.clone())
                    .await?
                {
                    info!("Manager return {:?}", job);
                    tx
                        .send(Response::Ok {
                            id: Some(job.id),
                            job: Some(job.to_serd()),
                        })
                        .await
                }
                else {
                    tx.send(Response::NoJob {}).await
                }
            }
            Request::Delete { id } => {
                match job_manager.delete_job(id.clone()).await? {
                    true => tx.send(Response::Ok {
                        id: None,
                        job: None,
                    }),
                    false => tx.send(Response::NoJob {}),
                }
                .await
            }
            Request::Abort { id } => {
                warn!("Called Abort");
                match job_manager
                    .abort_job(id.clone(), peer_address.clone())
                    .await?
                {
                    true => tx.send(Response::Ok {
                        id: None,
                        job: None,
                    }),
                    false => tx.send(Response::NoJob {}),
                }
                .await
            }
        };
        _ = result.map_err(|err| {
            info!("Received Request that errored {:?}", err);
            _ = tx.send(Response::Error {
                error: err.to_string(),
            });
        });
    }
    Ok(())
}
