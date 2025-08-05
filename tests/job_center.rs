use prime_time::job_center::{
    actor_scheduler::{actor::JobCommand, manager::JobManager},
    handle_jobs_center::handle_job_center,
    Request, Response,
};
use tokio::{net::TcpListener, sync::mpsc};

mod job_center_harness;
mod test_util;
use job_center_harness::{JobCenterHarness, JobServer};
use test_util::TestClient;

struct TestJobServer;

impl JobServer for TestJobServer {
    async fn run(listener: TcpListener) -> anyhow::Result<()> {
        let (tx, rx) = mpsc::channel::<JobCommand>(32);
        let mut job_manager = JobManager::new(rx);
        tokio::spawn(async move {
            if let Err(e) = job_manager.job_actor().await {
                tracing::error!("JobManager exited with error: {:?}", e);
            }
        });

        loop {
            let (socket, _addr) = listener.accept().await?;
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_job_center(socket, tx_clone).await {
                    tracing::error!("Error handling job center connection: {}", e);
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

async fn put_job_and_get_id(
    client: &mut TestClient,
    queue: String,
    job: serde_json::Value,
    pri: usize,
) -> anyhow::Result<usize> {
    send_request(
        client,
        Request::Put {
            queue,
            job,
            pri,
        },
    )
    .await?;

    let response = recv_response(client).await?;
    match response {
        Response::Ok { id: Some(id), .. } => Ok(id),
        _ => Err(anyhow::anyhow!("Unexpected response for put job: {:?}", response)),
    }
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

    let _ = recv_response(&mut client).await.unwrap();
   // assert_eq!(response, Response::Ok {id: Some(0), job: Some()});
}

#[tokio::test]
async fn test_abort_job() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client = TestClient::connect(&harness.endpoint()).await.unwrap();

    let job_id = put_job_and_get_id(
        &mut client,
        "test".to_string(),
        serde_json::json!({"data": "job_to_abort"}),
        1,
    )
    .await
    .unwrap();

    // Get the job to change its state to Given
    send_request(
        &mut client,
        Request::Get {
            queues: vec!["test".to_string()],
            wait: None,
        },
    )
    .await
    .unwrap();

    let _response = recv_response(&mut client).await.unwrap();

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

    // Try to get the job again, should be the re-queued job
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
    // Assert that the job is retrieved and its ID matches
    match response {
        Response::Ok { id: Some(retrieved_id), job: Some(_) } => {
            assert_eq!(retrieved_id, job_id);
        },
        _ => panic!("Expected re-queued job, but got: {:?}", response),
    }
}

#[tokio::test]
async fn test_abort_job_different_client() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client1 = TestClient::connect(&harness.endpoint()).await.unwrap();
    let mut client2 = TestClient::connect(&harness.endpoint()).await.unwrap();

    // Client 1 puts a job
    let job_id = put_job_and_get_id(
        &mut client1,
        "test".to_string(),
        serde_json::json!({"data": "job_for_client1"}),
        1,
    )
    .await
    .unwrap();

    // Client 1 gets the job (changes state to Given)
    send_request(
        &mut client1,
        Request::Get {
            queues: vec!["test".to_string()],
            wait: None,
        },
    )
    .await
    .unwrap();
    let _response = recv_response(&mut client1).await.unwrap();

    // Client 2 attempts to abort the job (should fail)
    send_request(
        &mut client2,
        Request::Abort {
            id: job_id,
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client2).await.unwrap();
    // Expect an error because it's a different client
    match response {
        Response::Error { .. } => {},
        _ => panic!("Expected an Error response, but got: {:?}", response),
    }

    // Verify the job is still with client1 (or re-queued if client1 disconnects)
    // For this test, we just check that client2 didn't successfully abort it.
}

#[tokio::test]
async fn test_abort_non_existent_job() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client = TestClient::connect(&harness.endpoint()).await.unwrap();

    // Attempt to abort a non-existent job
    send_request(
        &mut client,
        Request::Abort {
            id: 9999, // A non-existent ID
        },
    )
    .await
    .unwrap();

    let response = recv_response(&mut client).await.unwrap();
    assert_eq!(response, Response::NoJob {});
}

#[tokio::test]
async fn test_delete_job() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client = TestClient::connect(&harness.endpoint()).await.unwrap();

    let job_id = put_job_and_get_id(
        &mut client,
        "test".to_string(),
        serde_json::json!({"data": "job_to_delete"}),
        1,
    )
    .await
    .unwrap();

    // Delete the job
    send_request(&mut client, Request::Delete { id: job_id })
        .await
        .unwrap();

    let response = recv_response(&mut client).await.unwrap();
    assert_eq!(response, Response::Ok { id: None, job: None });

    // Attempt to delete the same job again
    send_request(&mut client, Request::Delete { id: job_id })
        .await
        .unwrap();

    let response = recv_response(&mut client).await.unwrap();
    assert_eq!(response, Response::NoJob {});
}

#[tokio::test]
async fn test_delete_then_get() {
    let harness = JobCenterHarness::<TestJobServer>::new().await;
    let mut client = TestClient::connect(&harness.endpoint()).await.unwrap();

    // Put a high-priority job
    let job_id_high = put_job_and_get_id(
        &mut client,
        "test".to_string(),
        serde_json::json!({"data": "high_pri"}),
        10,
    )
    .await
    .unwrap();

    // Put a low-priority job
    let job_id_low = put_job_and_get_id(
        &mut client,
        "test".to_string(),
        serde_json::json!({"data": "low_pri"}),
        5,
    )
    .await
    .unwrap();

    // Delete the high-priority job
    send_request(&mut client, Request::Delete { id: job_id_high })
        .await
        .unwrap();
    let response = recv_response(&mut client).await.unwrap();
    assert_eq!(response, Response::Ok { id: None, job: None });

    // Get a job from the queue
    send_request(
        &mut client,
        Request::Get {
            queues: vec!["test".to_string()],
            wait: None,
        },
    )
    .await
    .unwrap();

    // We should receive the low-priority job
    let response = recv_response(&mut client).await.unwrap();
    match response {
        Response::Ok { id: Some(id), .. } => {
            assert_eq!(id, job_id_low);
        }
        _ => panic!("Expected to get the low-priority job, but got {:?}", response),
    }
}


