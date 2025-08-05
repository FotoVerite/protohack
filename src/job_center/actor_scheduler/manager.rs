use slab::Slab;
use tracing::{debug, warn};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc::Receiver, oneshot::Sender};

use crate::job_center::actor_scheduler::{
    actor::JobCommand,
    job::{Job, JobState},
    queue::{Entry, Queue},
};

pub struct JobManager {
    pub jobs: Slab<Job>,
    queues: HashMap<String, Queue>,
    pub rx: Receiver<JobCommand>,
}

impl JobManager {
    pub fn new(rx: Receiver<JobCommand>) -> Self {
        Self {
            jobs: Slab::new(),
            queues: HashMap::new(),
            rx,
        }
    }
    pub fn add_job(&mut self, queue: &str, job: Job) -> usize {
        let pri = job.priority.clone();
        let id = self.jobs.insert(job);
        self.add_to_queue(queue, id, pri);
        id
    }

    pub fn add_to_queue(&mut self, queue: &str, id: usize, priority: usize) {
        let queue = self.queues.entry(queue.to_string()).or_default();
        while let Some((addr, resp)) = queue.senders.pop() {
            let resp_guard = resp.lock().unwrap().take().unwrap();
            if let Some(job) = self.jobs.get_mut(id) {
                if let Err(_) = resp_guard.send(Ok(Some((id, job.clone())))) {
                    continue;
                }
                job.state = JobState::Given(addr);
                return;
            }
        }
        let entry = Entry {
            id,
            priority: priority,
        };
        warn!("ADDING ENTRY, {:?}", entry);
        queue.jobs.push(entry);
    }

    pub fn get_job(&self, id: usize) -> Option<&Job> {
        match self.jobs.get(id) {
            None => None,
            Some(job) => match job.state {
                JobState::Ready => Some(job),
                _ => None,
            },
        }
    }

    pub fn mark_job(&mut self, id: usize) {
        if let Some(job) = self.jobs.get_mut(id) {
            debug!("Marking job {:?}", job);
            job.state = JobState::Deleted
        };
    }

    pub fn give_job(&mut self, id: usize, addr: SocketAddr) {
        if let Some(job) = self.jobs.get_mut(id) {
            job.state = JobState::Given(addr)
        };
    }

    pub fn get_from_queues(
        &mut self,
        queues: &Vec<String>,
        wait: bool,
        addr: SocketAddr,
        resp: Sender<anyhow::Result<Option<(usize, Job)>>>,
    ) {
        let mut top_job: Option<(&String, &Job)> = None;
        let jobs = &self.jobs;
        for queue_name in queues {
            if let Some(queue) = self.queues.get_mut(queue_name) {
                while let Some(entry) = queue.jobs.peek() {
                    if let Some(job) = jobs.get(entry.id) {
                        match job.state {
                            JobState::Deleted => {
                                queue.jobs.pop();
                                continue;
                            }
                            JobState::Given(_) => {
                                queue.jobs.pop();
                                continue;
                            }
                            JobState::Ready => match top_job {
                                None => {
                                    top_job = Some((queue_name, job));
                                    break;
                                }
                                Some((_, candidate)) => {
                                    if candidate.priority < job.priority {
                                        top_job = Some((queue_name, job));
                                    }
                                    break;
                                }
                            },
                        }
                    }
                }
            }
        }
        if let Some((queue, _)) = top_job {
            let entry = self.queues.get_mut(queue).unwrap().jobs.pop().unwrap();
            if let Some(job) = self.jobs.get_mut(entry.id) {
                job.state = JobState::Given(addr);
                respond(resp, Ok(Some((entry.id.clone(), job.clone()))));
            } else {
                respond(resp, Ok(None));
            }
            return;
        }
        if wait {
            let resp_arc = Arc::new(Mutex::new(Some(resp)));
            for queue_name in queues {
                if let Some(queue) = self.queues.get_mut(queue_name) {
                    queue.senders.push((addr, Arc::clone(&resp_arc)));
                }
            }
        } else {
            respond(resp, Ok(None));
        }
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
