use tokio::{sync::mpsc::Sender, time::Duration};
use tokio_util::sync::CancellationToken;

use crate::road::codec::RespValue;
pub fn spawn_heartbeat_task(interval: u32, tx: Sender<RespValue>, cancel_token: CancellationToken) {
    println!("Spawning hearbeat at interval {interval}");
    if interval == 0 {
        return;
    }
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis((interval as u64) * 100));

        interval.tick().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if tx.send(RespValue::Heartbeat).await.is_err() {
                        break;
                    }
                }
                _ = cancel_token.cancelled() => {
                    break;
                }
            }
        }
    });
}
