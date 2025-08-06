use anyhow::{Result, bail};
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder},
};
use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    Text(String),
}

#[derive(Debug, PartialEq)]
pub enum VersionControlCommand {
    Help,
    List(Option<String>),
    Get(Option<String>),
    Put(Option<(String, String)>),
}

pub struct Codec;

impl Decoder for Codec {
    type Item = VersionControlCommand;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        info!("Decoding {:?}", src);

        // Clone src for parsing, so we don't consume original unless successful
        let mut buf = src.clone();

        match buf.windows(1).position(|ch| ch == b"\n") {
            None => return Ok(None),
            Some(position) => {
                let line = buf.split_to(position + 1);
                if let Some((consume, command)) = self.parse_command(&line, &mut buf)? {
                    src.advance(consume);
                    return Ok(Some(command));
                } else {
                    return Ok(None);
                }
            }
        };
    }
}

impl Codec {
    fn parse_command(
        &mut self,
        line: &BytesMut,
        buf: &mut BytesMut,
    ) -> anyhow::Result<Option<(usize, VersionControlCommand)>> {
        let input_string = String::from_utf8(line.to_vec())?;
        let to_consume = line.len();
        let parts = input_string
            .trim()
            .split_ascii_whitespace()
            .collect::<Vec<&str>>();
        match parts.as_slice() {
            [cmd, args @ ..] => {
                let cmd = cmd.to_ascii_uppercase();
                match cmd.as_str() {
                    "HELP" => Ok(Some((to_consume, VersionControlCommand::Help))),
                    "LIST" => Ok(Some((to_consume, VersionControlCommand::List(Some(args[0].to_string()))))),
                    "GET" => Ok(Some((to_consume, VersionControlCommand::Get(Some(args[0].to_string()))))),
                    "PUT" => {
                        let file = args.get(0).expect("Filename not given");
                        let length = args.get(1).expect("Length not given").parse::<usize>()?;
                        if buf.len() < length {
                            Ok(None)
                        } else {
                            let data = String::from_utf8(buf[..length].to_vec())?;
                            let command = VersionControlCommand::Put(Some((file.to_string(), data)));
                            return Ok(Some((to_consume + length, command)));
                        }
                    }
                    _ => bail!("Illegal Method"),
                }
            }
            [] => bail!("Illegal Method"),
        }
    }
}

impl Encoder<RespValue> for Codec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: RespValue, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            RespValue::Text(string) => {
                dst.put(string.as_bytes());
                dst.put("\n".as_bytes());
            }
        }
        Ok(())
    }
}
