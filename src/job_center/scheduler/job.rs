use std::{cmp::Ordering, net::SocketAddr, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use serde_json::{Value, to_value};

#[derive(Debug, Clone)]
pub struct Job {
    pub id: usize,
    pub priority: usize,
    pub queue: String,
    pub state: JobState,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobPayload {
    job: Value,
    #[serde(rename = "pri")]
    priority: usize,
    queue: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum JobState {
    Ready,
    Given(SocketAddr),
    Deleted,
}

pub type JobRef = Arc<Mutex<Job>>;

impl Job {
    pub fn new(id: usize, queue: String, value: Value, priority: usize) -> Self {
        Self {
            id,
            priority,
            queue,
            state: JobState::Ready,
            value,
        }
    }

    pub fn to_serd(&self) -> JobPayload {
        JobPayload {
            job: self.value.clone(),
            priority: self.priority,
            queue: self.queue.clone(),
        }
    }
}
