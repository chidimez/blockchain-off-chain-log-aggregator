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