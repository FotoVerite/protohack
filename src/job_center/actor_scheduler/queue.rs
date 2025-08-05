use std::{cmp::Ordering, collections::BinaryHeap, net::SocketAddr, sync::{Arc, Mutex}};

use tokio::sync::oneshot::Sender;

use crate::job_center::actor_scheduler::job::Job;

#[derive(Debug, Clone)]
pub struct Entry {
    pub priority: usize,
    pub id: usize,
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

#[derive(Debug, Default)]
pub struct Queue {
    pub jobs: BinaryHeap<Entry>,
    pub senders: Vec<(SocketAddr, WaitResponder)>,
}

pub type WaitResponder = Arc<Mutex<Option<Sender<anyhow::Result<Option<(usize, Job)>>>>>>;