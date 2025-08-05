
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::job_center::actor_scheduler::job::JobPayload;


pub mod handle_jobs_center;
mod handle_request;
mod handle_response;
pub mod actor_scheduler;


#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "status")]
pub enum Response {
    #[serde(rename = "ok")]
    Ok {
        // other fields for success response
        id: Option<usize>,
        #[serde(flatten)]
        job: Option<JobPayload>,
    },

    #[serde(rename = "error")]
    Error {
        // error details
        error: String,
    },

    #[serde(rename = "no-job")]
    NoJob {
        // maybe empty or some other fields
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "request")]
pub enum Request {
    #[serde(rename = "put", alias = "PUT", alias = "Put")]
    Put {
        queue: String,
        job: Value,
        pri: usize,
    },

    #[serde(rename = "get", alias = "GET", alias = "Get")]
    Get {
        queues: Vec<String>,
        wait: Option<bool>,
    },

    #[serde(rename = "delete", alias = "DELETE", alias = "Delete")]
    Delete { id: usize },

    #[serde(rename = "abort", alias = "ABORT", alias = "Abort")]
    Abort { id: usize },
}
