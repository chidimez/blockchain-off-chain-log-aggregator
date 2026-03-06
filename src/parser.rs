#![allow(dead_code)]
use std::io::{self, Read};

use crate::protocol::{PacketType, MAGIC_BYTE};

#[derive(Debug, Clone)]
pub struct PacketHeader {
    pub packet_type: PacketType,
    pub payload_len: u16,
}

#[derive(Debug, Clone)]
pub struct RawPacket {
    pub packet_index: u64,
    pub offset: u64, // byte offset of the packet start (magic byte)
    pub header: PacketHeader,
    pub payload: Vec<u8>,
    pub checksum: u8,
}

#[derive(Debug)]
pub enum ParseError {
    Io(io::Error),
    TruncatedHeader { at_offset: u64 },
    TruncatedPayload { at_offset: u64, expected: u16 },
    MissingChecksum { at_offset: u64 },
}

impl std::convert::From<io::Error> for ParseError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct StreamParser<R: Read> {
    reader: R,
    offset: u64,
    packet_index: u64,
}

impl<R: Read> StreamParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            offset: 0,
            packet_index: 0,
        }
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Attempt to read the next packet.
    ///
    /// Returns:
    /// - Ok(Some(packet)) when a full packet is framed
    /// - Ok(None) when EOF is reached cleanly before starting a new packet
    /// - Err(ParseError) for truncation or I/O errors
    ///
    /// Bad magic is handled defensively by consuming bytes until MAGIC_BYTE is found.
    pub fn next_packet(&mut self) -> Result<Option<RawPacket>, ParseError> {
        loop {
            let packet_start = self.offset;

            let magic = match read_u8_eof_ok(&mut self.reader, &mut self.offset)? {
                None => return Ok(None), // clean EOF before starting a packet
                Some(b) => b,
            };

            if magic != MAGIC_BYTE {
                // Desync: consume bytes one-by-one until we find MAGIC_BYTE.
                // We do not treat this as a fatal error at framing level.
                eprintln!(
                    "desync @ offset {}: bad magic 0x{:02x}, scanning forward",
                    packet_start, magic
                );
                continue;
            }

            // Packet type (1 byte)
            let raw_type = read_u8_required(&mut self.reader, &mut self.offset)
                .map_err(|_| ParseError::TruncatedHeader { at_offset: self.offset })?;
            let packet_type = PacketType::from_byte(raw_type);

            // Payload length (2 bytes, big-endian)
            let payload_len = read_be_u16_required(&mut self.reader, &mut self.offset)
                .map_err(|_| ParseError::TruncatedHeader { at_offset: self.offset })?;

            // Payload (N bytes)
            let mut payload = vec![0u8; payload_len as usize];
            if read_exact_required(&mut self.reader, payload.as_mut_slice(), &mut self.offset).is_err() {
                return Err(ParseError::TruncatedPayload {
                    at_offset: self.offset,
                    expected: payload_len,
                });
            }

            // Checksum (1 byte)
            let checksum = read_u8_required(&mut self.reader, &mut self.offset)
                .map_err(|_| ParseError::MissingChecksum { at_offset: self.offset })?;

            let header = PacketHeader {
                packet_type,
                payload_len,
            };

            let packet = RawPacket {
                packet_index: self.packet_index,
                offset: packet_start,
                header,
                payload,
                checksum,
            };

            self.packet_index += 1;
            return Ok(Some(packet));
        }
    }
}

fn read_u8_eof_ok<R: Read>(reader: &mut R, offset: &mut u64) -> Result<Option<u8>, io::Error> {
    let mut buf = [0u8; 1];
    match reader.read(&mut buf) {
        Ok(0) => Ok(None),
        Ok(1) => {
            *offset += 1;
            Ok(Some(buf[0]))
        }
        Ok(_) => unreachable!("reading into a 1-byte buffer cannot yield >1 bytes"),
        Err(e) => Err(e),
    }
}

fn read_u8_required<R: Read>(reader: &mut R, offset: &mut u64) -> Result<u8, io::Error> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    *offset += 1;
    Ok(buf[0])
}

fn read_be_u16_required<R: Read>(reader: &mut R, offset: &mut u64) -> Result<u16, io::Error> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    *offset += 2;
    Ok(u16::from_be_bytes(buf))
}

fn read_exact_required<R: Read>(reader: &mut R, out: &mut [u8], offset: &mut u64) -> Result<(), io::Error> {
    reader.read_exact(out)?;
    *offset += out.len() as u64;
    Ok(())
}