use std::{cmp::Ordering, collections::BinaryHeap};

use tracing::warn;

use crate::job_center::scheduler::{job::JobRef, manager::ClientWaiter};

#[derive(Debug, Clone)]
pub struct Entry {
    pub priority: usize,
    pub id: usize,
    pub job_ref: JobRef,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}
impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.priority.cmp(&other.priority))
    }
}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        // max‐heap: higher priority pops first
        self.priority.cmp(&other.priority)
    }
}

#[derive(Debug, Clone)]
pub struct Queue {
    senders: Vec<ClientWaiter>,
    pub jobs: BinaryHeap<Entry>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            senders: vec![],
            jobs: BinaryHeap::new(),
        }
    }
    pub async fn enqueue_job(&mut self, id: usize, priority: usize, job_ref: JobRef) {
        if let Some(sender_ref) = self.senders.pop() {
            let ref_copy = sender_ref.clone();
            let mut sender_guard = ref_copy.lock().await;
            if let Some(sender) = sender_guard.take() {
                match &sender.send(id) {
                    Ok(_) => {}
                    Err(_) => {
                        warn!("Listener did not Respond");
                        self.jobs.push(Entry {
                            priority,
                            id,
                            job_ref,
                        })
                    }
                }
            }
        } else {
            self.jobs.push(Entry {
                priority,
                id,
                job_ref,
            })
        }
    }

    pub async fn register_waiter(&mut self, sender_ref: ClientWaiter) {
        if let Some(job) = self.jobs.pop() {
            let ref_copy = sender_ref.clone();
            let mut sender_guard = ref_copy.lock().await;
            if let Some(sender) = sender_guard.take() {
                match sender.send(job.id) {
                    Ok(_) => {}
                    Err(_) => {
                        warn!("Listener did not Respond");
                        self.jobs.push(job)
                    }
                }
            }
        } else {
            self.senders.push(sender_ref)
        }
    }
}
