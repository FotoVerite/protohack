use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc::Receiver, oneshot::Sender};
use tracing::{debug, warn};

use crate::job_center::actor_scheduler::{
    actor::JobCommand,
    job::{Job, JobState},
    queue::{Entry, Queue},
};

pub struct JobManager {
    pub jobs: HashMap<usize, Job>,
    queues: HashMap<String, Queue>,
    pub rx: Receiver<JobCommand>,
    next_id: usize,
}

impl JobManager {
    pub fn new(rx: Receiver<JobCommand>) -> Self {
        Self {
            jobs: HashMap::new(),
            queues: HashMap::new(),
            rx,
            next_id: 0,
        }
    }
    pub fn add_job(&mut self, queue: &str, job: Job) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let pri = job.priority;
        self.jobs.insert(id, job);
        self.add_to_queue(queue, id, pri);
        id
    }

    pub fn add_to_queue(&mut self, queue: &str, id: usize, priority: usize) {
        let queue = self.queues.entry(queue.to_string()).or_default();
        while let Some((addr, resp)) = queue.senders.pop() {
            let resp_guard = resp.lock().unwrap().take().unwrap();
            if let Some(job) = self.jobs.get_mut(&id) {
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
        match self.jobs.get(&id) {
            None => None,
            Some(job) => match job.state {
                JobState::Ready => Some(job),
                _ => None,
            },
        }
    }

    pub fn mark_job(&mut self, id: usize) {
        if let Some(job) = self.jobs.get_mut(&id) {
            debug!("Marking job {:?}", job);
            job.state = JobState::Deleted
        };
    }

    pub fn give_job(&mut self, id: usize, addr: SocketAddr) {
        if let Some(job) = self.jobs.get_mut(&id) {
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
        let mut top_job_entry: Option<(String, Entry)> = None;

        for queue_name in queues {
            if let Some(queue) = self.queues.get_mut(queue_name) {
                // Lazily remove invalid jobs from the top of the heap.
                while let Some(entry) = queue.jobs.peek() {
                    if self
                        .jobs
                        .get(&entry.id)
                        .map_or(true, |j| j.state != JobState::Ready)
                    {
                        // Job is gone or not ready, pop it.
                        queue.jobs.pop();
                    } else {
                        // Found a valid, ready job.
                        break;
                    }
                }

                // If a valid job is now at the top, consider it.
                if let Some(entry) = queue.jobs.peek() {
                    if top_job_entry
                        .as_ref()
                        .map_or(true, |(_, top)| entry.priority > top.priority)
                    {
                        top_job_entry = Some((queue_name.clone(), entry.clone()));
                    }
                }
            }
        }

        if let Some((queue_name, _)) = top_job_entry {
            let queue = self.queues.get_mut(&queue_name).unwrap();
            let entry = queue.jobs.pop().unwrap();
            let job = self.jobs.get_mut(&entry.id).unwrap(); // Should exist
            job.state = JobState::Given(addr);
            respond(resp, Ok(Some((entry.id, job.clone()))));
            return;
        }

        if wait {
            let resp_arc = Arc::new(Mutex::new(Some(resp)));
            for queue_name in queues {
                let queue = self.queues.entry(queue_name.to_string()).or_default();
                queue.senders.push((addr, Arc::clone(&resp_arc)));
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
