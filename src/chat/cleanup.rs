use std::{net::SocketAddr, sync::Arc};

use tokio::sync::broadcast::Sender;

use crate::chat::handle_chat::UserStorage;

pub async fn cleanup(
    users: UserStorage,
    peer_address: SocketAddr,
    tx: &Arc<Sender<(SocketAddr, String)>>,
) -> anyhow::Result<()> {
    let mut guard = users.lock().await;
    if let Some(user) = guard.remove(&peer_address) {
        let left_message = format!("* {} has left the room", user.name);
        // Use a special system address, not the leaving user's address
        let system_addr = SocketAddr::from(([0, 0, 0, 0], 0));
        let _ = tx.send((system_addr, left_message));
    }
    Ok(())
}
