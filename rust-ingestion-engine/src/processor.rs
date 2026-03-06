use crate::decoder::{decode_transaction, DecodeError};
use crate::parser::RawPacket;
use crate::protocol::PacketType;
use crate::validate::{validate_packet, ValidationError};

#[derive(Debug)]
pub enum PacketDecision {
    EmitJson(String),
    Skip(String),
    Discard(String),
}

pub fn process_packet(packet: &RawPacket) -> PacketDecision {
    match validate_packet(
        packet.header.packet_type,
        &packet.payload,
        packet.checksum,
    ) {
        Ok(()) => match packet.header.packet_type {
            PacketType::Transaction => match decode_transaction(&packet.payload) {
                Ok(tx) => {
                    if tx.amount > 1000 {
                        match serde_json::to_string(&tx) {
                            Ok(line) => PacketDecision::EmitJson(line),
                            Err(e) => PacketDecision::Discard(format!(
                                "packet {} @ offset {}: JSON serialization error: {}, discarded",
                                packet.packet_index, packet.offset, e
                            )),
                        }
                    } else {
                        PacketDecision::Skip(format!(
                            "packet {} @ offset {}: transaction amount {} <= 1000, skipped",
                            packet.packet_index, packet.offset, tx.amount
                        ))
                    }
                }

                Err(DecodeError::PayloadTooShort { payload_len }) => PacketDecision::Discard(
                    format!(
                        "packet {} @ offset {}: transaction payload too short ({}), discarded",
                        packet.packet_index, packet.offset, payload_len
                    ),
                ),

                Err(DecodeError::InvalidUtf8Memo) => PacketDecision::Discard(format!(
                    "packet {} @ offset {}: invalid UTF-8 memo, discarded",
                    packet.packet_index, packet.offset
                )),
            },

            PacketType::StateUpdate => PacketDecision::Skip(format!(
                "packet {} @ offset {}: state update valid, skipped",
                packet.packet_index, packet.offset
            )),

            PacketType::KeepAlive => PacketDecision::Skip(format!(
                "packet {} @ offset {}: keep-alive valid, skipped",
                packet.packet_index, packet.offset
            )),

            PacketType::Unknown(t) => PacketDecision::Skip(format!(
                "packet {} @ offset {}: unknown packet type 0x{:02x} valid, skipped",
                packet.packet_index, packet.offset, t
            )),
        },

        Err(ValidationError::ChecksumMismatch { expected, got }) => PacketDecision::Discard(
            format!(
                "packet {} @ offset {}: checksum mismatch, expected 0x{:02x} got 0x{:02x}, discarded",
                packet.packet_index, packet.offset, expected, got
            ),
        ),

        Err(ValidationError::InvalidTxPayloadLen { payload_len }) => PacketDecision::Discard(
            format!(
                "packet {} @ offset {}: invalid transaction payload length {}, discarded",
                packet.packet_index, packet.offset, payload_len
            ),
        ),

        Err(ValidationError::InvalidStateUpdateLen { payload_len }) => PacketDecision::Discard(
            format!(
                "packet {} @ offset {}: invalid state update payload length {}, discarded",
                packet.packet_index, packet.offset, payload_len
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{PacketHeader, RawPacket};
    use crate::protocol::PacketType;

    fn xor_checksum(payload: &[u8]) -> u8 {
        payload.iter().fold(0u8, |acc, &b| acc ^ b)
    }

    fn build_raw_packet(
        packet_index: u64,
        offset: u64,
        packet_type: PacketType,
        payload: Vec<u8>,
        checksum: u8,
    ) -> RawPacket {
        RawPacket {
            packet_index,
            offset,
            header: PacketHeader {
                packet_type,
                payload_len: payload.len() as u16,
            },
            payload,
            checksum,
        }
    }

    fn build_tx_payload(hash_byte: u8, amount: u64, memo: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&[hash_byte; 32]);
        payload.extend_from_slice(&amount.to_be_bytes());
        payload.extend_from_slice(memo);
        payload
    }

    #[test]
    fn process_packet_emits_json_for_valid_high_value_transaction() {
        let payload = build_tx_payload(0xAA, 5000, b"big transfer");
        let checksum = xor_checksum(&payload);
        let packet = build_raw_packet(2, 99, PacketType::Transaction, payload, checksum);

        let decision = process_packet(&packet);

        match decision {
            PacketDecision::EmitJson(line) => {
                assert!(line.contains(r#""amount":5000"#));
                assert!(line.contains(r#""memo":"big transfer""#));
                assert!(line.contains(
                    r#""tx_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#
                ));
            }
            other => panic!("expected EmitJson, got {:?}", other),
        }
    }

    #[test]
    fn process_packet_skips_low_value_transaction() {
        let payload = build_tx_payload(0x01, 1000, b"threshold");
        let checksum = xor_checksum(&payload);
        let packet = build_raw_packet(1, 50, PacketType::Transaction, payload, checksum);

        let decision = process_packet(&packet);

        match decision {
            PacketDecision::Skip(message) => {
                assert!(message.contains("transaction amount 1000 <= 1000, skipped"));
                assert!(message.contains("packet 1 @ offset 50"));
            }
            other => panic!("expected Skip, got {:?}", other),
        }
    }

    #[test]
    fn process_packet_discards_invalid_utf8_transaction() {
        let payload = build_tx_payload(0x02, 3000, &[0xFF, 0xFE]);
        let checksum = xor_checksum(&payload);
        let packet = build_raw_packet(7, 307, PacketType::Transaction, payload, checksum);

        let decision = process_packet(&packet);

        match decision {
            PacketDecision::Discard(message) => {
                assert!(message.contains("invalid UTF-8 memo, discarded"));
                assert!(message.contains("packet 7 @ offset 307"));
            }
            other => panic!("expected Discard, got {:?}", other),
        }
    }

    #[test]
    fn process_packet_discards_checksum_mismatch() {
        let payload = build_tx_payload(0x03, 99999, b"bad checksum");
        let wrong_checksum = 0x00;
        let packet = build_raw_packet(8, 357, PacketType::Transaction, payload, wrong_checksum);

        let decision = process_packet(&packet);

        match decision {
            PacketDecision::Discard(message) => {
                assert!(message.contains("checksum mismatch"));
                assert!(message.contains("packet 8 @ offset 357"));
            }
            other => panic!("expected Discard, got {:?}", other),
        }
    }

    #[test]
    fn process_packet_skips_valid_state_update() {
        let mut payload = vec![0x11; 32];
        payload.push(0x01);
        let checksum = xor_checksum(&payload);
        let packet = build_raw_packet(5, 260, PacketType::StateUpdate, payload, checksum);

        let decision = process_packet(&packet);

        match decision {
            PacketDecision::Skip(message) => {
                assert!(message.contains("state update valid, skipped"));
                assert!(message.contains("packet 5 @ offset 260"));
            }
            other => panic!("expected Skip, got {:?}", other),
        }
    }

    #[test]
    fn process_packet_skips_valid_keep_alive() {
        let payload = vec![1, 2, 3, 4];
        let checksum = xor_checksum(&payload);
        let packet = build_raw_packet(6, 298, PacketType::KeepAlive, payload, checksum);

        let decision = process_packet(&packet);

        match decision {
            PacketDecision::Skip(message) => {
                assert!(message.contains("keep-alive valid, skipped"));
                assert!(message.contains("packet 6 @ offset 298"));
            }
            other => panic!("expected Skip, got {:?}", other),
        }
    }

    #[test]
    fn process_packet_skips_valid_unknown_type() {
        let payload = b"abcdef".to_vec();
        let checksum = xor_checksum(&payload);
        let packet = build_raw_packet(9, 472, PacketType::Unknown(0x10), payload, checksum);

        let decision = process_packet(&packet);

        match decision {
            PacketDecision::Skip(message) => {
                assert!(message.contains("unknown packet type 0x10 valid, skipped"));
                assert!(message.contains("packet 9 @ offset 472"));
            }
            other => panic!("expected Skip, got {:?}", other),
        }
    }
}