use std::result;

use anyhow::bail;
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder},
};

use crate::road::ticket::Ticket;

#[derive(Debug, PartialEq)]
pub enum RespValue {
    Error(String),
    Ticket(Ticket),
    Heartbeat,
}

#[derive(Debug, PartialEq)]
pub enum ReqValue {
    Plate(String, u32),
    WantHeartbeat(u32),
    IAmCamera(u16, u16, u16),
    IAmDispatcher(Vec<u16>),
    ReqError(String),
}

pub struct Codec;

impl Decoder for Codec {
    type Item = ReqValue; // Include raw bytes
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, anyhow::Error> {
        // Check if we have a full line ending in \r\n
        if src.is_empty() {
            return Ok(None);
        }
        println!("Decoding {:?}", src);
        if !frame_finished(src) {
            return Ok(None);
        }
        let first_byte: u8 = src[0];
        src.advance(1);

        match first_byte {
            0x20 => return Ok(parse_plate(src)?),
            0x40 => return Ok(Some(parse_heartbeat(src)?)),
            0x80 => return Ok(Some(parse_camera(src)?)),
            0x81 => return Ok(parse_dispatcher(src)?),
            other => {
                return Err(anyhow::anyhow!(format!("Unknown Req type {}", other)));
            }
        }
    }
}

fn frame_finished(src: &BytesMut) -> bool {
    if src.is_empty() {
        return false;
    }

    let opt_code = src[0];

    match opt_code {
        0x20 => {
            if src.len() < 2 {
                return false;
            }
            let length = src[1] as usize;
            let total_len = 2 + length + 4;
            src.len() >= total_len
        }
        0x40 => src.len() >= 5,
        0x80 => src.len() >= 7,
        0x81 => {
            if src.len() < 2 {
                return false;
            }
            let count = src[1] as usize;
            let total_len = 2 + count * 2;
            src.len() >= total_len
        }
        _ => true, // or false if unknown opcodes should be treated as errors
    }
}
fn parse_length_prefixed_str(src: &mut BytesMut) -> anyhow::Result<String> {
    if src.len() < 1 {
        // Not enough bytes for the length prefix
        bail!("Insufficient data for string length");
    }

    // Peek at the length without advancing yet
    let len = src[0] as usize;

    if src.len() < 1 + len {
        // Wait for more data
        bail!("Incomplete string data");
    }

    // Consume the length prefix
    src.advance(1);

    // Split out the string bytes
    let str_bytes = src.split_to(len);

    // Convert to &str
    let s = std::str::from_utf8(&str_bytes)?;
    Ok(s.to_owned())
}

fn parse_u16_slice(src: &mut BytesMut, count: usize) -> anyhow::Result<Vec<u16>> {
    let needed = count * 2;
    if src.len() < needed {
        bail!("Not enough bytes to read {} u16 values", count);
    }

    let raw = src.split_to(needed);

    // Convert to u16s
    let result = raw
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]])) // or from_le_bytes
        .collect();

    Ok(result)
}

fn parse_dispatcher(src: &mut BytesMut) -> anyhow::Result<Option<ReqValue>> {
    let count = src[0] as usize;
    let total_len = 2 + count * 2;
    if src.len() < total_len {
        return Ok(None); // not enough bytes yet
    }
    src.advance(1);

    let roads = parse_u16_slice(src, count)?;
    Ok(Some(ReqValue::IAmDispatcher(roads)))
}

fn parse_plate(src: &mut BytesMut) -> anyhow::Result<Option<ReqValue>> {
    let plate = parse_length_prefixed_str(src)?;
    let timestamp = src.get_u32();

    Ok(Some(ReqValue::Plate(plate, timestamp)))
}

fn parse_camera(src: &mut BytesMut) -> anyhow::Result<ReqValue> {
    let road = src.get_u16();
    let mile = src.get_u16();
    let limit = src.get_u16();

    Ok(ReqValue::IAmCamera(road, mile, limit))
}

fn parse_heartbeat(src: &mut BytesMut) -> anyhow::Result<ReqValue> {
    if src.len() >= 4 {
        let val = src.get_u32(); // automatically advances the cursor
        return Ok(ReqValue::WantHeartbeat(val));
        // `header` now holds the first 4 bytes, and they're gone from `src`
    }
    return Err(anyhow::anyhow!("Empty Heartbeat Request"));
}

impl Encoder<RespValue> for Codec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: RespValue, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            RespValue::Error(e) => write_str(dst, 0x10, e.as_bytes()),
            RespValue::Heartbeat => {
                dst.put_u8(0x41);
                Ok(())
            }
            RespValue::Ticket(ticket) => write_ticket(dst, ticket),
        }
    }
}
fn write_ticket(dst: &mut BytesMut, ticket: Ticket) -> anyhow::Result<()> {
    write_str(dst, 0x21, ticket.plate.as_bytes())?;
    dst.put_u16(ticket.road);
    dst.put_u16(ticket.mile1);
    dst.put_u32(ticket.timestamp1);
    dst.put_u16(ticket.mile2);
    dst.put_u32(ticket.timestamp2);
    dst.put_u16(ticket.speed);
    Ok(())
}
fn write_str(dst: &mut BytesMut, opcode: u8, payload: &[u8]) -> anyhow::Result<()> {
    let len = payload.len();
    if len > 255 {
        anyhow::bail!("Error Catch for too long message")
    }
    dst.put_u8(opcode);
    dst.put_u8(len as u8); // opcode as a single byte
    dst.extend_from_slice(payload);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use tokio_util::{bytes, codec::Decoder};

    #[test]
    fn test_decode_heartbeat() {
        let mut codec = Codec;
        let mut buf = BytesMut::with_capacity(5);
        buf.put_u8(0x40);
        buf.put_u32(123456);

        let val = codec.decode(&mut buf).unwrap();
        assert_eq!(val, Some(ReqValue::WantHeartbeat(123456)));
        assert_eq!(buf.len(), 0); // buffer should be consumed
    }

    #[test]
    fn test_incomplete_heartbeat_returns_none() {
        let mut codec = Codec;
        let mut buf = BytesMut::from(&[0x40, 0x00, 0x01][..]);

        let err = codec.decode(&mut buf).unwrap_err();
        assert!(err.to_string().contains("Empty Heartbeat Request"));
    }

    #[test]
    fn test_unknown_opcode_errors() {
        let mut codec = Codec;
        let mut buf = BytesMut::from(&[0x99][..]);

        let err = codec.decode(&mut buf).unwrap_err();
        assert!(err.to_string().contains("Unknown Req type"));
        //assert_eq!(err.kind(), anyhow::Error);
    }

    #[test]
    fn test_multiple_heartbeat_messages() {
        let mut codec = Codec;
        let mut buf = BytesMut::with_capacity(10);
        buf.put_u8(0x40);
        buf.put_u32(42);
        buf.put_u8(0x40);
        buf.put_u32(99);

        let first = codec.decode(&mut buf).unwrap();
        assert_eq!(first, Some(ReqValue::WantHeartbeat(42)));

        let second = codec.decode(&mut buf).unwrap();
        assert_eq!(second, Some(ReqValue::WantHeartbeat(99)));

        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_empty_buffer() {
        let mut codec = Codec;
        let mut buf = BytesMut::new();

        let val = codec.decode(&mut buf).unwrap();
        assert_eq!(val, None);
    }

    fn assert_opcode(buf: &[u8], expected: u8) {
        assert_eq!(buf[0], expected, "Unexpected opcode");
    }

    fn assert_length(buf: &[u8], expected_len: usize) {
        assert_eq!(buf[1] as usize, expected_len, "Unexpected length byte");
    }

    fn assert_be_bytes(actual: &[u8], expected: &[u8], field_name: &str) {
        assert_eq!(actual, expected, "Mismatch in field {}", field_name);
    }

    #[test]
    fn encode_error() -> anyhow::Result<()> {
        let mut codec = Codec;
        let mut buf = BytesMut::new();

        let err_msg = "some error occurred";
        let item = RespValue::Error(err_msg.to_string());

        codec.encode(item, &mut buf)?;

        assert_opcode(&buf, 0x10);
        assert_length(&buf, err_msg.len());
        assert_eq!(&buf[2..], err_msg.as_bytes());

        Ok(())
    }

    #[test]
    fn encode_heartbeat() -> anyhow::Result<()> {
        let mut codec = Codec;
        let mut buf = BytesMut::new();

        codec.encode(RespValue::Heartbeat, &mut buf)?;

        assert_opcode(&buf, 0x41);
        assert_length(&buf, 0);
        assert_eq!(buf.len(), 2);

        Ok(())
    }

    #[test]
    fn encode_ticket() -> anyhow::Result<()> {
        let mut codec = Codec;
        let mut buf = BytesMut::new();

        let plate = "ABC123".to_string();
        let road = 123;
        let mile1 = 456;
        let timestamp1 = 789;
        let mile2 = 1011;
        let timestamp2 = 1213;
        let speed = 88;
        let ticket = Ticket {
            plate: plate.clone(),
            road,
            mile1,
            timestamp1,
            mile2,
            timestamp2,
            speed,
        };

        codec.encode(RespValue::Ticket(ticket), &mut buf)?;

        assert_opcode(&buf, 0x21);
        assert_length(&buf, plate.len());
        assert_eq!(&buf[2..2 + plate.len()], plate.as_bytes());

        let rem = &buf[(2 + plate.len())..];
        assert_be_bytes(&rem[0..2], &road.to_be_bytes(), "road");
        assert_be_bytes(&rem[2..4], &mile1.to_be_bytes(), "mile1");
        assert_be_bytes(&rem[4..8], &timestamp1.to_be_bytes(), "timestamp1");
        assert_be_bytes(&rem[8..10], &mile2.to_be_bytes(), "mile2");
        assert_be_bytes(&rem[10..14], &timestamp2.to_be_bytes(), "timestamp2");
        assert_be_bytes(&rem[14..16], &speed.to_be_bytes(), "speed");

        Ok(())
    }

    #[test]
    fn encode_error_too_long() {
        let mut codec = Codec;
        let mut buf = BytesMut::new();

        let long_err = "a".repeat(300);

        let item = RespValue::Error(long_err);

        let result = codec.encode(item, &mut buf);

        assert!(
            result.is_err(),
            "Encoding should fail on too long error message"
        );
    }

    #[test]
    fn test_frame_finished_and_consumption_consistency() {
        let mut codec = Codec;

        // Helper to build plate frame: opcode(1) + length(1) + plate + timestamp(4)
        fn build_plate_frame(plate: &str, timestamp: u32) -> BytesMut {
            let mut buf = BytesMut::new();
            buf.put_u8(0x20);
            buf.put_u8(plate.len() as u8);
            buf.extend_from_slice(plate.as_bytes());
            buf.put_u32(timestamp);
            buf
        }

        let plate = "EZ24TEG";
        let timestamp = 0x08_4B_A4; // arbitrary

        let mut buf = build_plate_frame(plate, timestamp);

        // Check frame_finished returns true for full frame
        assert!(
            frame_finished(&buf),
            "frame_finished should be true for complete plate frame"
        );

        // Decode and check buffer fully consumed
        let before_len = buf.len();
        let decoded = codec.decode(&mut buf).unwrap();
        let after_len = buf.len();
        assert!(decoded.is_some(), "decode should return Some on full frame");
        assert_eq!(
            decoded.unwrap(),
            Some(ReqValue::Plate(plate.to_string(), timestamp)).unwrap()
        );
        assert_eq!(
            after_len, 0,
            "Buffer should be fully consumed, got {} bytes leftover",
            after_len
        );
        assert_eq!(
            before_len - after_len,
            before_len,
            "Consumed bytes must equal initial buffer length"
        );
    }

    #[test]
    fn test_incomplete_plate_frame_returns_none() {
        let mut codec = Codec;
        // Build frame but chop off last byte to simulate incomplete frame
        let mut buf = BytesMut::new();
        buf.put_u8(0x20);
        buf.put_u8(3);
        buf.extend_from_slice(b"ABC");
        buf.put_u32(123456);
        buf.truncate(buf.len() - 1); // remove last byte

        assert!(
            !frame_finished(&buf),
            "frame_finished should be false for incomplete plate frame"
        );
        let decoded = codec.decode(&mut buf).unwrap();
        assert!(
            decoded.is_none(),
            "decode should return None for incomplete plate frame"
        );
    }

    #[test]
    fn test_extra_bytes_after_frame_error() {
        let mut codec = Codec;

        // Build a plate frame but add extra garbage bytes at the end
        let mut buf = BytesMut::new();
        buf.put_u8(0x20);
        buf.put_u8(3);
        buf.extend_from_slice(b"XYZ");
        buf.put_u32(999);
        buf.extend_from_slice(b"extra");

        assert!(
            frame_finished(&buf),
            "frame_finished should be true even with extra bytes"
        );

        // Decode should error because bytes remaining after expected frame
        let res = codec.decode(&mut buf);
        assert!(
            res.is_err(),
            "decode should error due to bytes remaining on stream"
        );
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("bytes remaining on stream"),
            "error should mention bytes remaining"
        );
    }

    #[test]
    fn test_unknown_opcode_error() {
        let mut codec = Codec;
        let mut buf = BytesMut::from(&[0x99, 0x00, 0x00][..]);

        let res = codec.decode(&mut buf);
        assert!(res.is_err(), "decode should error on unknown opcode");
        assert!(
            res.unwrap_err().to_string().contains("Unknown Req type"),
            "error should mention unknown req type"
        );
    }

    // Extend this if you support strings or tickets
}
