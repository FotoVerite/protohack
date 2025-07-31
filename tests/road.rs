mod test_util;

use tokio::time::{sleep, timeout, Duration};

use crate::test_util::TestClient;

const SERVER_ADDR: &str = "127.0.0.1:5000"; // or spawn dynamically

#[tokio::test]
async fn test_iam_and_location() -> anyhow::Result<()> {
    let mut client = TestClient::connect("127.0.0.1:4000").await?;

    // Simulate IAm camera
    client.send("IAm:camera:128:244\n").await?;
    sleep(Duration::from_millis(10)).await;

    // Send plate at time
    client.send("Plate:ABC123:1000\n").await?;
    sleep(Duration::from_millis(10)).await;

    // No immediate response expected
    let res = client.read_line().await;
    assert!(res.is_err()); // timeout or closed
    Ok(())
}

#[tokio::test]
async fn test_ticket_issued() -> anyhow::Result<()> {
    let mut client = TestClient::connect("127.0.0.1:4000").await?;

    client.send("IAm:camera:128:244\n").await?;
    client.send("Plate:ABC123:1000\n").await?;
    sleep(Duration::from_millis(10)).await;

    client.send("Plate:ABC123:1100\n").await?;
    sleep(Duration::from_millis(10)).await;

    let resp = client.read_line().await?;
    assert!(resp.contains("Ticket"));
    Ok(())
}

#[tokio::test]
async fn test_multiple_clients() -> anyhow::Result<()> {
    let mut car = TestClient::connect("127.0.0.1:4000").await?;
    let mut dispatcher = TestClient::connect("127.0.0.1:4000").await?;

    dispatcher.send("IAm:dispatcher:128:129\n").await?;
    car.send("IAm:camera:128:244\n").await?;
    car.send("Plate:ZZZ999:1234\n").await?;

    let notice = dispatcher.read_line().await?;
    assert!(notice.contains("Plate"));
    Ok(())
}
