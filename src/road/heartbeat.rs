use crate::road::codec::RespValue;
use tokio::sync::mpsc::Sender;
use tokio::time::{Duration, sleep};

pub fn spawn_heartbeat_task(interval: u32, tx: Sender<RespValue>) {
    println!("Spawning hearbeat at interval {interval}");
    if interval == 0 {
        return;
    }
    let delay = Duration::from_millis(interval as u64 * 100);

    tokio::spawn(async move {
        loop {
            sleep(delay).await;
            if tx.send(RespValue::Heartbeat).await.is_err() {
                break; // client likely disconnected
            }
        }
    });
}
