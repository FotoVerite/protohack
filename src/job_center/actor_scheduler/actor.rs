use std::net::SocketAddr;

use anyhow::{Ok};
use serde_json::Value;
use tokio::sync::oneshot::Sender;
use tracing::warn;

use crate::job_center::actor_scheduler::{
    job::{Job, JobState},
    manager::JobManager,
};

pub enum JobCommand {
    Put {
        queue: String,
        job: Value,
        pri: usize,
        resp: Sender<anyhow::Result<usize>>,
    },
    Get {
        queues: Vec<String>,
        wait: bool,
        addr: SocketAddr,
        resp: Sender<anyhow::Result<Option<(usize, Job)>>>,
    },
    Delete {
        id: usize,
        resp: Sender<anyhow::Result<bool>>,
    },
    Abort {
        id: usize,
        addr: SocketAddr,
        resp: Sender<anyhow::Result<bool>>,
    },
}

impl JobManager {
    pub async fn job_actor(&mut self) -> anyhow::Result<()> {
        while let Some(command) = self.rx.recv().await {
            match command {
                JobCommand::Put {
                    queue,
                    job,
                    pri,
                    resp,
                } => {
                    let job = Job::new(queue.clone(), job, pri);
                    let id = self.add_job(&queue, job);
                    respond(resp, Ok(id));
                }
                JobCommand::Get {
                    queues,
                    wait,
                    addr,
                    resp,
                } => self.get_from_queues(&queues, wait, addr, resp),
                JobCommand::Delete { id, resp } => {
                    if self.jobs.remove(&id).is_some() {
                        respond(resp, Ok(true));
                    } else {
                        respond(resp, Ok(false));
                    }
                },
                JobCommand::Abort { id, addr, resp } => {
                    let job_to_add_back = match self.jobs.get_mut(&id) {
                        None => {
                            respond(resp, Ok(false));
                            None
                        }
                        Some(job) => match job.state {
                            JobState::Given(given_to) => {
                                warn!("Found Job {:?}", job);
                                if given_to == addr {
                                    job.state = JobState::Ready;
                                    respond(resp, Ok(true));

                                    Some(job.clone())
                                } else {
                                    respond(resp, Err(anyhow::anyhow!("Has not been given job")));
                                    None
                                }
                            }
                            _ => {
                                warn!("Found a job {:?}", job);
                                respond(resp, Ok(false));
                                None
                            }
                        },
                    };
                    if let Some(mut job) = job_to_add_back {
                        warn!("ADDING BACK {:?}", job);
                        job.state = JobState::Ready;
                        self.add_to_queue(&job.queue, id, job.priority);
                    }
                }
            }
        }
        Ok(())
    }
}

fn respond<T>(tx: Sender<T>, val: T)
where
    T: std::fmt::Debug,
{
    if let Err(e) = tx.send(val) {
        tracing::warn!("Failed to send response: {:?}", e);
    }
}
