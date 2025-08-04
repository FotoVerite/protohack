use prime_time::job_center::{
    Request, Response, handle_jobs_center::handle_job_center, scheduler::manager::JobManager,
};
use tracing_test::traced_test;
use std::sync::{Arc, atomic::AtomicUsize};
use tokio::net::TcpListener;

mod job_center_harness;
mod test_util;
use job_center_harness::{JobCenterHarness, JobServer};
use test_util::TestClient;

struct TestJobServer;

impl JobServer for TestJobServer {
    async fn run(listener: TcpListener) -> anyhow::Result<()> {
        let job_manager = JobManager::new();
        let id_counter = Arc::new(AtomicUsize::new(0));
        loop {
            let (socket, _addr) = listener.accept().await?;
            let job_manager = job_manager.clone();
            let id_counter = id_counter.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_job_center(socket, job_manager, id_counter).await {
                    eprintln!("Error handling job center connection: {}", e);
                }
            });
        }
    }
}

async fn send_request(client: &mut TestClient, request: Request) -> anyhow::Result<()> {
    let mut request_str = serde_json::to_string(&request)?;
    request_str.push('\n');
    client.send(&request_str).await?;
    Ok(())
}

async fn recv_response(client: &mut TestClient) -> anyhow::Result<Response> {
    let response_str = client.read_line().await?;
    let response: Response = serde_json::from_str(&response_str)?;
    Ok(response)
}

// #[tokio::test]
// async fn test_job_center_startup() {
//     let _harness = JobCenterHarness::<TestJobServer>::new().await;
// }

#[tokio::test]
async fn test_put_job() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client = TestClient::connect(&harness.endpoint()).await.unwrap();

    send_request(
        &mut client,
        Request::Put {
            queue: "test".to_string(),
            job: serde_json::json!({"data": "hello"}),
            pri: 1,
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client).await.unwrap();

    assert_eq!(
        response,
        Response::Ok {
            id: Some(0),
            job: None
        }
    );
}

#[tokio::test]
async fn test_get_no_job() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client = TestClient::connect(&harness.endpoint()).await.unwrap();

    send_request(
        &mut client,
        Request::Get {
            queues: vec!["test".to_string()],
            wait: None,
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client).await.unwrap();
    assert_eq!(response, Response::NoJob {});
}

#[tokio::test]
async fn test_get_job() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client = TestClient::connect(&harness.endpoint()).await.unwrap();

    send_request(
        &mut client,
        Request::Put {
            queue: "test".to_string(),
            job: serde_json::json!({"data": "hello"}),
            pri: 1,
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client).await.unwrap();

    assert_eq!(
        response,
        Response::Ok {
            id: Some(0),
            job: None
        }
    );

    send_request(
        &mut client,
        Request::Get {
            queues: vec!["test".to_string()],
            wait: None,
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client).await.unwrap();
   // assert_eq!(response, Response::Ok {id: Some(0), job: Some()});
}

#[tokio::test]
async fn test_abort_job() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client = TestClient::connect(&harness.endpoint()).await.unwrap();

    // Put a job
    send_request(
        &mut client,
        Request::Put {
            queue: "test".to_string(),
            job: serde_json::json!({"data": "job_to_abort"}),
            pri: 1,
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client).await.unwrap();
    let job_id = match response {
        Response::Ok { id: Some(id), .. } => id,
        _ => panic!("Unexpected response for put job: {:?}", response),
    };

    // Abort the job
    send_request(
        &mut client,
        Request::Abort {
            id: job_id,
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client).await.unwrap();
    assert_eq!(response, Response::Ok { id: None, job: None });

    // Try to get the job again, should be NoJob
    send_request(
        &mut client,
        Request::Get {
            queues: vec!["test".to_string()],
            wait: None,
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client).await.unwrap();
    assert_eq!(response, Response::NoJob {});
}


