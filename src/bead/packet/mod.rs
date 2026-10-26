//! A simple framed request and response protocol.

use std::convert::Infallible;
use std::iter::repeat;

use tokio::io::{self, AsyncReadExt};

mod crc;
pub use crc::*;

pub mod incoming;
pub mod outgoing;

pub const MAGIC: [u8; 2] = [b'H', b'B'];

pub const MAX_PAYLOAD_SIZE: usize = u8::MAX as usize;

/// In big endian:               | magic       | len | payload          | checksum |
pub const MAX_PACKET_SIZE: usize = MAGIC.len() + 1   + MAX_PAYLOAD_SIZE + 2        ;

pub enum FrameError<T = Infallible> {
    NotMagic,
    ChecksumMismatch,
    FromRead(T),
}

impl<T> From<T> for FrameError<T> {
    fn from(value: T) -> Self { FrameError::FromRead(value) }
}

pub async fn read_frame<'a, R: io::AsyncRead + Unpin>(
    reader: &mut R,
    buffer: &'a mut heapless::Deque<u8, MAX_PACKET_SIZE>)
    -> Result<&'a mut [u8], FrameError<io::Error>> {
    let target_buffer_len = MAGIC.len() + 1;

    if buffer.len() < target_buffer_len {
        let start = buffer.len();
        buffer.extend(repeat(0).take(target_buffer_len - buffer.len()));
        reader.read_exact(&mut buffer.make_contiguous()[start..target_buffer_len]).await?;
    }

    if !MAGIC.iter().eq(buffer.iter().take(MAGIC.len())) {
        return Err(FrameError::NotMagic);
    }
    let len = *buffer.get(MAGIC.len()).unwrap() as usize;

    let target_buffer_len = target_buffer_len + len + 2;

    if buffer.len() < target_buffer_len {
        let start = buffer.len();
        buffer.extend(repeat(0).take(target_buffer_len - buffer.len()));
        reader.read_exact(&mut buffer.make_contiguous()[start..target_buffer_len]).await?;
    }

    let checksum = u16::from_be_bytes([
        *buffer.get(target_buffer_len - 2).unwrap(),
        *buffer.get(target_buffer_len - 1).unwrap(),
    ]);

    let payload = &mut buffer.make_contiguous()[MAGIC.len() + 1..][..len as usize];

    let mut crc = CRC16::default();
    for byte in payload.iter() {
        crc.resume(*byte);
    }

    if crc.complete() == checksum {
        Ok(payload)
    } else {
        Err(FrameError::ChecksumMismatch)
    }
}

pub fn surround_frame(buffer: &mut heapless::Deque<u8, MAX_PACKET_SIZE>) {
    let mut crc = CRC16::default();
    for byte in buffer.iter() {
        crc.resume(*byte);
    }

    buffer.push_front(buffer.len().try_into().map_err(drop).unwrap()).map_err(drop).unwrap();
    for byte in MAGIC.into_iter().rev() {
        buffer.push_front(byte).map_err(drop).unwrap();
    }
    for byte in crc.complete().to_be_bytes() {
        buffer.push_back(byte).map_err(drop).unwrap();
    }
}
