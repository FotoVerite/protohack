use std::{
    cmp::Reverse,
    collections::{HashMap, btree_set},
    net::SocketAddr,
    sync::Arc,
};

use anyhow::bail;
use serde_json::Value;
use tokio::sync::{Mutex, oneshot::Sender};
use tracing::info;

use crate::job_center::scheduler::{
    job::{Job, JobRef, JobState},
    queue::{self, Entry, Queue},
};

#[derive(Debug, Clone)]
pub struct JobManager {
    jobs: HashMap<usize, JobRef>,
    queues: HashMap<String, Queue>,
}
pub type ClientWaiter = Arc<Mutex<Option<Sender<usize>>>>;

impl JobManager {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            queues: HashMap::new(),
        }
    }

    pub async fn add_job(
        &mut self,
        id: usize,
        queue_name: String,
        priority: &usize,
        value: &Value,
    ) {
        let job = Job::new(id, queue_name.clone(), value.clone(), priority.clone());
        let arc_job = Arc::new(Mutex::new(job));
        self.jobs.insert(id, Arc::clone(&arc_job));
        let queue = self.queues.entry(queue_name).or_insert(Queue::new());
        queue
            .enqueue_job(id, priority.clone(), Arc::clone(&arc_job))
            .await;
    }

    pub async fn delete_job(&mut self, id: usize) -> anyhow::Result<bool> {
        if let Some(job_ref) = self.jobs.remove(&id) {
            let job_ref = job_ref.clone(); // Clone the Arc before locking
            let mut job = job_ref.lock().await;
            match job.state {
                JobState::Deleted => {
                    bail!("Job {} was already deleted (race condition?)", id)
                }
                _ => {
                    job.state = JobState::Deleted;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub async fn abort_job(
        &mut self,
        id: usize,
        client_address: SocketAddr,
    ) -> anyhow::Result<bool> {
        info!("Trying to abort Job {} from peer {}", id, client_address);
        let should_remove = if let Some(job_ref) = self.jobs.get(&id) {
            let job_ref = job_ref.clone();
            let mut job = job_ref.lock().await;
            info!("Found Job {:?}", job);
            match job.state {
                JobState::Deleted => true,
                JobState::Ready => return Ok(false),
                JobState::Given(given_address) => {
                    if client_address == given_address {
                        job.state = JobState::Ready;
                        if let Some(queue) = self.queues.get_mut(&job.queue) {
                            queue.enqueue_job(job.id, job.priority, Arc::clone(&job_ref)).await;
                        }
                        true // Should remove
                    } else {
                        bail!("Is not working on this job")
                    }
                }
            }
        } else {
            return Ok(false);
        };

        // Lock is dropped here, now we can mutate self.jobs
        if should_remove {
            self.jobs.remove(&id);
            Ok(false)
        } else {
            Ok(false)
        }
    }

    pub async fn get_job(
        &mut self,
        queues: Vec<String>,
        waiter: Option<ClientWaiter>,
        client_address: SocketAddr,
    ) -> anyhow::Result<Option<Job>> {
        let mut best_entry: Option<Entry> = None;
        let mut candidates: Vec<(String, Entry)> = Vec::new();

        for queue_name in &queues {
            if let Some(queue) = self.queues.get_mut(queue_name) {
                if let Some(candidate) = queue.jobs.peek() {
                    let should_take = match &best_entry {
                        None => true,                                               // First candidate
                        Some(entry) if candidate.priority > entry.priority => true, // Better candidate
                        _ => false, // Current best is still better
                    };
                    if should_take {
                        let entry = queue.jobs.pop();
                        best_entry = entry.clone();
                        candidates.push((queue_name.clone(), entry.unwrap())); // Take this one
                    }
                }
            }
        }

        candidates.sort_by_key(|(_, e)| e.priority);
        let candidate = candidates.pop();
        for (queue, entry) in candidates {
            if let Some(queue) = self.queues.get_mut(&queue) {
                queue.jobs.push(entry);
            }
        }
        if let Some((_, entry)) = candidate {
            let job_ref_clone = entry.job_ref;
            let mut job_guard = job_ref_clone.lock().await;
            match job_guard.state {
                JobState::Ready => {
                    job_guard.state = JobState::Given(client_address);
                    return Ok(Some(job_guard.clone()));
                }
                JobState::Deleted | JobState::Given(_) => {
                    // Skip invalid jobs
                }
            }
        }
        if let Some(waiter) = &waiter {
            for queue_name in &queues {
                if let Some(queue) = self.queues.get_mut(queue_name) {
                    queue.register_waiter(Arc::clone(waiter));
                }
            }
        }

        return Ok(None);
    }
}
