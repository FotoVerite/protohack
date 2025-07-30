use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::{
    net::TcpStream,
    sync::{Mutex, broadcast::Sender},
};
use tokio_util::codec::{Framed, LinesCodec};

use crate::chat::{ChatSession, user::User};

pub type Users = HashMap<SocketAddr, User>;
pub type UserStorage = Arc<Mutex<Users>>;

pub async fn handle_chat(
    socket: TcpStream,
    tx: &Arc<Sender<(SocketAddr, String)>>,
    users: UserStorage,
) -> anyhow::Result<()> {
    let framed: Framed<TcpStream, LinesCodec> = Framed::new(socket, LinesCodec::new());

    let session = ChatSession::handshake(framed, &users, tx).await?;
    let result = session.run(users).await?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::{SinkExt, StreamExt};
    use tokio::{net::TcpListener, sync::broadcast, time::timeout};
    use tokio_util::codec::LinesCodecError;

    use super::*;

    type SharedUsers = Arc<Mutex<HashMap<SocketAddr, User>>>;

    pub async fn spawn_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, _rx) = broadcast::channel::<(SocketAddr, String)>(100);
        let tx = Arc::new(tx);

        let users: SharedUsers = Arc::new(Mutex::new(HashMap::new()));

        tokio::spawn(async move {
            loop {
                let (stream, peer_addr) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(_) => continue,
                };

                let users = Arc::clone(&users);
                let tx = Arc::clone(&tx);
                tokio::spawn(async move {
                    if let Err(e) = handle_chat(stream, &tx, users).await {
                        eprintln!("error in connection from {}: {:?}", peer_addr, e);
                    }
                });
            }
        });

        addr
    }

    async fn connect_and_name(
        addr: SocketAddr,
        name: &str,
    ) -> Result<Framed<TcpStream, LinesCodec>, LinesCodecError> {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut framed = Framed::new(stream, LinesCodec::new());
        framed.send(name.to_string()).await?;
        Ok(framed)
    }

    #[tokio::test]
    async fn test_chat() {
        let addr = spawn_server().await;

        let mut bob = connect_and_name(addr, "bob").await.unwrap();
        let mut alice = connect_and_name(addr, "alice").await.unwrap();
        let mut trent = connect_and_name(addr, "trent").await.unwrap();

        let _ = expect_messages(
            &mut alice,
            &[
                "Welcome to budgetchat! What shall I call you?",
                "* The room contains: bob",
                "* trent has entered the room",
            ],
        )
        .await;

        let _ = expect_messages(
            &mut bob,
            &[
                "Welcome to budgetchat! What shall I call you?",
                "* The room contains: ",
                "* alice has entered the room",
                "* trent has entered the room",
            ],
        )
        .await;

        trent.send("hello everyone".to_string()).await.unwrap();

        let _ = expect_messages(&mut alice, &["[trent] hello everyone"]).await;

        let _ = expect_messages(&mut bob, &["[trent] hello everyone"]).await;

        // // Bob receives it
        // let msg = bob.next().await.unwrap().unwrap();
        // assert_eq!(msg, "[trent]: hello everyone");
    }

    #[tokio::test]
    async fn test_user_leave() -> anyhow::Result<()> {
        let addr = spawn_server().await;

        let bob = connect_and_name(addr, "bob").await?;
        let mut alice = connect_and_name(addr, "alice").await?;

        // Consume join messages so alice knows about bob
        expect_messages(
            &mut alice,
            &[
                "Welcome to budgetchat! What shall I call you?",
                "* The room contains: bob",
            ],
        )
        .await?;

        // Drop bob's connection to simulate leaving
        println!("About to drop bob");
        drop(bob);
        println!("Bob dropped");

        // Alice should receive leave notification
        expect_messages(&mut alice, &["* bob has left the room"]).await?;

        Ok(())
    }

    fn assert_str_diff(expected: &str, actual: &str) {
        use similar::{ChangeTag, TextDiff};

        if expected != actual {
            let diff = TextDiff::from_lines(expected, actual);
            println!("--- Expected\n+++ Actual\n");
            for change in diff.iter_all_changes() {
                match change.tag() {
                    ChangeTag::Delete => print!("-{}", change),
                    ChangeTag::Insert => print!("+{}", change),
                    ChangeTag::Equal => print!(" {}", change),
                }
            }
            panic!("Strings did not match!");
        }
    }

    pub async fn expect_messages(
    framed: &mut Framed<TcpStream, LinesCodec>,
    expected: &[&str],
) -> anyhow::Result<()> {
    for &exp in expected {
        let result = timeout(Duration::from_secs(1), framed.next())
            .await
            .map_err(|_| anyhow::anyhow!("Timeout waiting for message: '{}'", exp))?;

        let actual = match result {
            Some(Ok(msg)) => msg,  // msg is a String
            Some(Err(e)) => {
                return Err(anyhow::anyhow!(
                    "Stream error while waiting for: '{}', error: {:?}",
                    exp,
                    e
                ));
            }
            None => {
                return Err(anyhow::anyhow!(
                    "Stream closed unexpectedly while waiting for: '{}' (framed.next() returned None)",
                    exp
                ));
            }
        };


        // Don't forget to actually check the message!
        assert_str_diff(exp, &actual);
    }
    Ok(())
}
}
