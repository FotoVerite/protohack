use futures::{SinkExt, stream::SplitSink};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpStream, sync::broadcast::Sender};
use tokio_util::codec::{Framed, LinesCodec};

use crate::chat::{handle_chat::UserStorage, user::User};

pub async fn add_user(
    name: &str,
    peer_address: SocketAddr,
    users: &UserStorage,
    writer: &mut SplitSink<Framed<TcpStream, LinesCodec>, String>,
    tx: &Arc<Sender<(SocketAddr, String)>>,
) -> anyhow::Result<()> {
    let user = User::new(name)?;
    let current_users = get_usernames(&users).await;

    if current_users.contains(&name.to_string()) {
        return Err(anyhow::anyhow!("User already exists"));
    }

    let announcement = format!("* The room contains: {}", current_users.join(", "));

    match writer.send(announcement).await {
        Ok(()) => println!("Sent announcement to Alice successfully"),
        Err(e) => {
            println!("Failed to send announcement to Alice: {:?}", e);
            return Err(e.into());
        }
    }
    let joined_message = format!("* {} has entered the room", &user.name);
    let mut guard = users.lock().await;

    guard.insert(peer_address, user);
    drop(guard);

    let system_addr = SocketAddr::from(([0, 0, 0, 0], 0));

    let _ = tx.send((system_addr, joined_message));
    Ok(())
}

async fn get_usernames(users: &UserStorage) -> Vec<String> {
    let guard = users.lock().await;
    guard.values().map(|u| u.name.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, net::SocketAddr, sync::Arc};
    use tokio::{net::TcpStream, sync::{broadcast, Mutex}};
    use tokio_util::codec::{Framed, LinesCodec};
    use futures::{stream::SplitStream, StreamExt};
    use crate::chat::user::User;

    type TestUserStorage = Arc<Mutex<HashMap<SocketAddr, User>>>;

    struct TestSetup {
        users: TestUserStorage,
        tx: Arc<broadcast::Sender<(SocketAddr, String)>>,
        rx: broadcast::Receiver<(SocketAddr, String)>,
        writer: SplitSink<Framed<TcpStream, LinesCodec>, String>,
        reader: SplitStream<Framed<TcpStream, LinesCodec>>,
        peer_addr: SocketAddr,
    }

    impl TestSetup {
        async fn new() -> Self {
            let users = Arc::new(Mutex::new(HashMap::new()));
            let (tx, rx) = broadcast::channel(10);
            let tx = Arc::new(tx);
            let (writer, reader, peer_addr) = Self::create_mock_connection().await;
            
            Self { users, tx, rx, writer, reader, peer_addr }
        }

        async fn create_mock_connection() -> (
            SplitSink<Framed<TcpStream, LinesCodec>, String>,
            SplitStream<Framed<TcpStream, LinesCodec>>,
            SocketAddr,
        ) {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            
            let (client_stream, _server_stream) = tokio::join!(
                TcpStream::connect(addr),
                listener.accept()
            );
            
            let client_stream = client_stream.unwrap();
            let peer_addr = client_stream.peer_addr().unwrap();
            let framed = Framed::new(client_stream, LinesCodec::new());
            let (writer, reader) = framed.split();
            
            (writer, reader, peer_addr)
        }

        async fn add_existing_user(&self, name: &str, addr: SocketAddr) {
            let mut guard = self.users.lock().await;
            guard.insert(addr, User::new(name).unwrap());
        }

        async fn user_count(&self) -> usize {
            self.users.lock().await.len()
        }

        async fn has_user(&self, addr: &SocketAddr) -> bool {
            self.users.lock().await.contains_key(addr)
        }

        async fn get_user_name(&self, addr: &SocketAddr) -> Option<String> {
            self.users.lock().await.get(addr).map(|u| u.name.clone())
        }

        fn try_recv_broadcast(&mut self) -> Result<(SocketAddr, String), broadcast::error::TryRecvError> {
            self.rx.try_recv()
        }

        async fn read_announcement(&mut self) -> String {
            self.reader.next().await.unwrap().unwrap()
        }

        fn system_addr() -> SocketAddr {
            SocketAddr::from(([0, 0, 0, 0], 0))
        }

        fn test_addr(port: u16) -> SocketAddr {
            SocketAddr::from(([127, 0, 0, 1], port))
        }
    }

    #[tokio::test]
    async fn test_add_user_success_empty_room() {
        let mut setup = TestSetup::new().await;

        let result = add_user("alice", setup.peer_addr, &setup.users, &mut setup.writer, &setup.tx).await;
        
        assert!(result.is_ok());
        assert_eq!(setup.user_count().await, 1);
        assert!(setup.has_user(&setup.peer_addr).await);
        assert_eq!(setup.get_user_name(&setup.peer_addr).await, Some("alice".to_string()));
        
        let (sender, msg) = setup.try_recv_broadcast().unwrap();
        assert_eq!(msg, "* alice has entered the room");
        assert_eq!(sender, TestSetup::system_addr());
    }

    #[tokio::test]
    async fn test_add_user_success_with_existing_users() {
        let mut setup = TestSetup::new().await;
        
        setup.add_existing_user("bob", TestSetup::test_addr(8000)).await;

        let result = add_user("alice", setup.peer_addr, &setup.users, &mut setup.writer, &setup.tx).await;
        
        assert!(result.is_ok());
        assert_eq!(setup.user_count().await, 2);
        
        let (_, msg) = setup.try_recv_broadcast().unwrap();
        assert_eq!(msg, "* alice has entered the room");
    }

    #[tokio::test]
    async fn test_add_user_duplicate_name_error() {
        let mut setup = TestSetup::new().await;
        
        setup.add_existing_user("alice", TestSetup::test_addr(8000)).await;

        let result = add_user("alice", setup.peer_addr, &setup.users, &mut setup.writer, &setup.tx).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("User already exists"));
        assert_eq!(setup.user_count().await, 1);
        assert!(!setup.has_user(&setup.peer_addr).await);
    }

    #[tokio::test]
    async fn test_add_user_invalid_username() {
        let mut setup = TestSetup::new().await;
        
        let result = add_user("", setup.peer_addr, &setup.users, &mut setup.writer, &setup.tx).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_user_no_broadcast_receivers() {
        let users = Arc::new(Mutex::new(HashMap::new()));
        let (tx, _rx) = broadcast::channel(10);
        drop(_rx); // No receivers
        
        let tx = Arc::new(tx);
        let (mut writer, _reader, peer_addr) = TestSetup::create_mock_connection().await;
        
        let result = add_user("alice", peer_addr, &users, &mut writer, &tx).await;
        
        assert!(result.is_ok());
        assert_eq!(users.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn test_get_usernames_empty() {
        let users = Arc::new(Mutex::new(HashMap::new()));
        
        let usernames = get_usernames(&users).await;
        
        assert!(usernames.is_empty());
    }

    #[tokio::test]
    async fn test_get_usernames_multiple_users() {
        let users = Arc::new(Mutex::new(HashMap::new()));
        
        {
            let mut guard = users.lock().await;
            guard.insert(TestSetup::test_addr(8000), User::new("alice").unwrap());
            guard.insert(TestSetup::test_addr(8001), User::new("bob").unwrap());
            guard.insert(TestSetup::test_addr(8002), User::new("charlie").unwrap());
        }
        
        let mut usernames = get_usernames(&users).await;
        usernames.sort();
        
        assert_eq!(usernames, vec!["alice", "bob", "charlie"]);
    }

   
}