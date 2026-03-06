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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::PacketType;

    #[test]
    fn xor_checksum_empty_payload_is_zero() {
        let payload = [];
        assert_eq!(xor_checksum(&payload), 0x00);
    }

    #[test]
    fn xor_checksum_computes_expected_value() {
        let payload = [0x10, 0x20, 0x30];
        // 0x10 ^ 0x20 ^ 0x30 = 0x00
        assert_eq!(xor_checksum(&payload), 0x00);
    }

    #[test]
    fn validate_transaction_accepts_minimum_valid_payload() {
        let payload = vec![0u8; 40];
        let checksum = xor_checksum(&payload);

        let result = validate_packet(PacketType::Transaction, &payload, checksum);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_transaction_rejects_short_payload() {
        let payload = vec![0u8; 39];
        let checksum = xor_checksum(&payload);

        let result = validate_packet(PacketType::Transaction, &payload, checksum);

        match result {
            Err(ValidationError::InvalidTxPayloadLen { payload_len }) => {
                assert_eq!(payload_len, 39);
            }
            other => panic!("expected InvalidTxPayloadLen, got {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_checksum_mismatch() {
        let payload = vec![0xAA, 0xBB, 0xCC];
        let wrong_checksum = 0x00;

        let result = validate_packet(PacketType::KeepAlive, &payload, wrong_checksum);

        match result {
            Err(ValidationError::ChecksumMismatch { expected, got }) => {
                assert_eq!(expected, xor_checksum(&payload));
                assert_eq!(got, wrong_checksum);
            }
            other => panic!("expected ChecksumMismatch, got {:?}", other),
        }
    }

    #[test]
    fn validate_state_update_accepts_exact_length() {
        let payload = vec![0u8; 33];
        let checksum = xor_checksum(&payload);

        let result = validate_packet(PacketType::StateUpdate, &payload, checksum);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_state_update_rejects_wrong_length() {
        let payload = vec![0u8; 32];
        let checksum = xor_checksum(&payload);

        let result = validate_packet(PacketType::StateUpdate, &payload, checksum);

        match result {
            Err(ValidationError::InvalidStateUpdateLen { payload_len }) => {
                assert_eq!(payload_len, 32);
            }
            other => panic!("expected InvalidStateUpdateLen, got {:?}", other),
        }
    }

    #[test]
    fn validate_unknown_packet_type_is_allowed_if_checksum_matches() {
        let payload = vec![1u8, 2u8, 3u8, 4u8];
        let checksum = xor_checksum(&payload);

        let result = validate_packet(PacketType::Unknown(0x77), &payload, checksum);
        assert!(result.is_ok());
    }
}