use std::{net::SocketAddr};

use serde::{Deserialize, Serialize};

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Job {
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

impl Job {
    pub fn new(queue: String, value: Value, priority: usize) -> Self {
        Self {
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
