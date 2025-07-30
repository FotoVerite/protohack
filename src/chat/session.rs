use crate::chat::{add_user::add_user, cleanup::cleanup, handle_chat::UserStorage};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpStream,
    sync::broadcast::{Receiver, Sender},
};
use tokio_util::{
    codec::{Framed, LinesCodec},
    sync::CancellationToken,
};

pub struct ChatSession {
    name: String,
    peer: SocketAddr,
    tx: Arc<Sender<(SocketAddr, String)>>,
    rx: Receiver<(SocketAddr, String)>,
    writer: SplitSink<Framed<TcpStream, LinesCodec>, String>,
    reader: SplitStream<Framed<TcpStream, LinesCodec>>,
}

impl ChatSession {
    pub async fn handshake(
        framed: Framed<TcpStream, LinesCodec>,
        users: &UserStorage,
        tx: &Arc<Sender<(SocketAddr, String)>>,
    ) -> anyhow::Result<Self> {
        let peer = framed.get_ref().peer_addr()?;
        let (mut writer, mut reader) = framed.split();

        writer
            .send("Welcome to budgetchat! What shall I call you?".into())
            .await?;
        let name = reader
            .next()
            .await
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("disconnected before naming"))?;

        add_user(&name, peer, users, &mut writer, tx).await?;

        Ok(Self {
            name,
            peer,
            tx: Arc::clone(tx),
            rx: tx.subscribe(),
            writer,
            reader,
        })
    }

    pub async fn run(self, users: UserStorage) -> anyhow::Result<()> {
        let ChatSession {
            name,
            peer,
            tx,
            rx,
            writer,
            reader,
        } = self;

        let shutdown = CancellationToken::new();

        let reader_task = {
            let tx_clone = tx.clone();
            let shutdown_clone = shutdown.clone();
            let users_clone = users.clone();
            let name_clone = name.clone();
            tokio::spawn(async move {
                read_task(
                    reader,
                    tx_clone,
                    peer,
                    name_clone,
                    users_clone,
                    shutdown_clone,
                )
                .await
            })
        };

        let writer_task = {
            let shutdown_clone = shutdown.clone();
            tokio::spawn(async move { write_task(writer, rx, peer, shutdown_clone).await })
        };

        // FIXED: Wait for BOTH tasks to complete before exiting
        let (reader_result, writer_result) = tokio::join!(reader_task, writer_task);

        // Handle results properly
        match reader_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("Reader error: {:?}", e),
            Err(e) => eprintln!("Reader task panicked: {:?}", e),
        }

        match writer_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("Writer error: {:?}", e),
            Err(e) => eprintln!("Writer task panicked: {:?}", e),
        }

        Ok(())
    }
}

async fn read_task(
    mut reader: SplitStream<Framed<TcpStream, LinesCodec>>,
    tx: Arc<Sender<(SocketAddr, String)>>,
    peer: SocketAddr,
    name: String,
    users: UserStorage,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    println!("Read task started for {} ({})", name, peer);

    loop {
        match reader.next().await {
            Some(Ok(line)) => {
                // Process message
                let _ = tx.send((peer, format!("[{}] {}", name, line)));
            }
            Some(Err(e)) => {
                eprintln!("Read error: {:?}", e);
                break; // Connection error
            }
            None => {
                break; // Connection closed
            }
        }
    }

    cleanup(users, peer, &tx).await?;
    shutdown.cancel();
    Ok(())
}

async fn write_task(
    mut writer: SplitSink<Framed<TcpStream, LinesCodec>, String>,
    mut rx: Receiver<(SocketAddr, String)>,
    peer: SocketAddr,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    println!("Writer task started for {}", peer);

    loop {
        tokio::select! {
            recv = rx.recv() => match recv {
                Ok((src, msg)) if src != peer => {
                    if writer.send(msg).await.is_err() {
                        break;
                    }
                }
                // Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                //     continue;
                // }
                // Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                //     break;
                // }
                _ => {}
            },
            // _ = shutdown.cancelled() => {
            //     break;
            // }
        }
    }
    Ok(())
}
