use std::{collections::HashMap, net::SocketAddr};

use tokio::sync::mpsc::Sender;

use crate::road::codec::RespValue;

pub struct RoadDispatcher {
    senders: HashMap<SocketAddr, Sender<RespValue>>,
    queue: Vec<RespValue>,
}

impl RoadDispatcher {
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
            queue: vec![],
        }
    }

    pub async fn add_ticket(&mut self, ticket: RespValue) {
        let maybe_key = self.senders.keys().next().cloned(); // avoid borrowing the whole map

        match maybe_key {
            Some(key) => {
                if let Some(tx) = self.senders.get(&key) {
                    if tx.send(ticket.clone()).await.is_err() {
                        // Receiver gone, re-queue and clear sender
                        self.queue.push(ticket);
                        self.senders.remove(&key); // safe, no active borrow
                    }
                }
            }
            None => {
                self.queue.push(ticket);
            }
        }
    }

    pub async fn add_sender(&mut self, address: SocketAddr, tx: Sender<RespValue>) {
        // Try to send all queued tickets
        for resp in self.queue.drain(..) {
            if tx.send(resp).await.is_err() {
                // Receiver died while draining
                return;
            }
        }
        self.senders.insert(address, tx);
    }

    pub async fn remove_sender(&mut self, key: SocketAddr) {
        self.senders.remove(&key);
    }
}
