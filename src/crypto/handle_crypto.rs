use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tracing::info;

use crate::crypto::{
    crypto_codec::{CryptoCodec, Message},
    toys::Toy,
};

pub async fn handle_cipher(socket: TcpStream) -> anyhow::Result<()> {

    let mut framed: Framed<TcpStream, CryptoCodec> = Framed::new(socket, CryptoCodec::new());
    while let Some(message) = framed.next().await.transpose()? {
        match message {
            Message::Cipher(_) => {}
            Message::Text(text) => {    
                info!("{text}");            
                let mut toys: Vec<Toy> = text
                    .split(',')
                    .filter_map(|entry| {
                        let (amount_str, name) = entry.split_once('x')?;
                        let amount = amount_str.parse::<u32>().ok()?;
                        Some(Toy {
                            amount,
                            name: name.to_string(),
                        })
                    })
                    .collect();

                toys.sort_by(|a, b| b.amount.cmp(&a.amount));
                
                if let Some(largest) = toys.first() {
                    info!("sending largest toy {}", largest);
                    let reply = Message::Text(largest.to_string());
                    framed.send(reply).await?;
                }

            }
        }
    }

    Ok(())
}
