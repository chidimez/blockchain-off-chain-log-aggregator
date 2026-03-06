use crate::protocol::{PacketType, STATE_UPDATE_PAYLOAD_LEN, TX_MIN_PAYLOAD_LEN};

#[derive(Debug)]
pub enum ValidationError {
    ChecksumMismatch { expected: u8, got: u8 },
    InvalidTxPayloadLen { payload_len: u16 },
    InvalidStateUpdateLen { payload_len: u16 },
}

pub fn xor_checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0u8, |acc, &b| acc ^ b)
}

pub fn validate_packet(packet_type: PacketType, payload: &[u8], checksum: u8) -> Result<(), ValidationError> {
    let expected = xor_checksum(payload);
    if expected != checksum {
        return Err(ValidationError::ChecksumMismatch {
            expected,
            got: checksum,
        });
    }

    let payload_len = payload.len() as u16;

    match packet_type {
        PacketType::Transaction => {
            if payload.len() < TX_MIN_PAYLOAD_LEN {
                return Err(ValidationError::InvalidTxPayloadLen { payload_len });
            }
        }
        PacketType::StateUpdate => {
            if payload.len() != STATE_UPDATE_PAYLOAD_LEN {
                return Err(ValidationError::InvalidStateUpdateLen { payload_len });
            }
        }
        PacketType::KeepAlive => {
            // Opaque, any length allowed, checksum already validated
        }
        PacketType::Unknown(_) => {
            // Forward compatible: accept structure if checksum is valid
        }
    }

    Ok(())
}