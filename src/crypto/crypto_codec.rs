use anyhow::{Result, bail};
use tokio_util::{
    bytes::{Buf, BytesMut},
    codec::{Decoder, Encoder},
};

use crate::crypto::ops::Op;

#[derive(Debug, Clone)]
pub enum DecodeState {
    Secure(Vec<Op>),
    Insecure,
}

pub enum Message {
    Cipher(Vec<Op>),
    Text(String),
}

pub struct CryptoCodec {
    state: DecodeState,
    client_pos: usize,
    server_pos: usize,
}
impl CryptoCodec {
    pub fn new() -> Self {
        Self {
            state: DecodeState::Insecure,
            client_pos: 0,
            server_pos: 0,
        }
    }
}

impl Decoder for CryptoCodec {
    type Item = Message;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }
        let mut buf = src.clone();

        match &self.state {
            DecodeState::Insecure => match parse_buf(&mut buf) {
                Ok((cipher, consume)) => {
                    let test_str = b"Test123!@\n";
                    if encrypt(&cipher, test_str, 0).as_slice() == test_str {
                        bail!("No Op");
                    }
                    src.advance(consume);
                    self.state = DecodeState::Secure(cipher.clone());
                    println!("{:?}", cipher);
                    return Ok(Some(Message::Cipher(cipher)));
                }
                Err(_) => return Ok(None),
            },
            DecodeState::Secure(cipher) => {
                let result = decrypt(&cipher, &buf, self.client_pos);
                if let Some(pos) = result.iter().position(|&b| b == b'\n') {
                    let to_decrypt = &result[..=pos]; // include newline
                    src.advance(to_decrypt.len());
                    self.client_pos += to_decrypt.len();
                    return Ok(Some(Message::Text(String::from_utf8(to_decrypt.to_vec())?)));
                } else {
                    // Wait for more data to arrive
                    return Ok(None);
                }
            }
        }
    }
}
fn parse_buf(src: &mut BytesMut) -> anyhow::Result<(Vec<Op>, usize)> {
    let mut idx = 0usize;
    let mut cipher = vec![];

    while idx < src.len() {
        match src[idx] {
            0x00 => {
                idx += 1;
                break;
            }
            0x01 => {
                cipher.push(Op::Reverse);
                idx += 1;
            }
            0x02 => {
                let n = *src
                    .get(idx + 1)
                    .ok_or_else(|| anyhow::anyhow!("Incomplete Xor at idx {}", idx))?;
                cipher.push(Op::Xor(n));
                idx += 2;
            }
            0x03 => {
                cipher.push(Op::XorPos);
                idx += 1;
            }

            0x04 => {
                if let Some(&n) = src.get(idx + 1) {
                    cipher.push(Op::Add(n));
                    idx += 2;
                } else {
                    anyhow::bail!("Parse error in cipher")
                }
            }
            0x05 => {
                cipher.push(Op::AddPos);
                idx += 1;
            }
            b => anyhow::bail!("Invalid opcode 0x{:02x} at idx {}", b, idx),
        }
    }
    return Ok((cipher, idx));
}

fn decrypt(cipher: &[Op], input: &[u8], start_pos: usize) -> Vec<u8> {
    input
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            let pos = start_pos + i;
            cipher
                .iter()
                .rev()
                .fold(b, |acc, op| op.inverse().apply(acc, pos))
        })
        .collect()
}

fn encrypt(cipher: &[Op], input: &[u8], start_pos: usize) -> Vec<u8> {
    input
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            let pos = start_pos + i;
            cipher.iter().fold(b, |acc, op| op.apply(acc, pos))
        })
        .collect()
}

impl Encoder<Message> for CryptoCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match &self.state {
            DecodeState::Secure(cipher) => match item {
                Message::Text(raw_sting) => {
                    let result = encrypt(&cipher, raw_sting.as_bytes(), self.server_pos);
                    self.server_pos += result.len();
                    dst.extend(result);
                    Ok(())
                }
                _ => {
                    bail!("Cannot parse")
                }
            },
            _ => bail!("Have not setup cipher"),
        }
    }
}
