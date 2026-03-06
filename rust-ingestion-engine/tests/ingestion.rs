use std::io::Cursor;

use rust_ingestion_engine::decoder::decode_transaction;
use rust_ingestion_engine::output::FilteredTransaction;
use rust_ingestion_engine::parser::StreamParser;
use rust_ingestion_engine::protocol::PacketType;
use rust_ingestion_engine::validate::validate_packet;

fn xor_checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0u8, |acc, &b| acc ^ b)
}

fn build_packet(packet_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut packet = Vec::new();
    packet.push(0xA5);
    packet.push(packet_type);
    packet.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    packet.extend_from_slice(payload);
    packet.push(xor_checksum(payload));
    packet
}

fn build_packet_with_checksum(packet_type: u8, payload: &[u8], checksum: u8) -> Vec<u8> {
    let mut packet = Vec::new();
    packet.push(0xA5);
    packet.push(packet_type);
    packet.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    packet.extend_from_slice(payload);
    packet.push(checksum);
    packet
}

fn build_tx_payload(hash_byte: u8, amount: u64, memo: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&[hash_byte; 32]);
    payload.extend_from_slice(&amount.to_be_bytes());
    payload.extend_from_slice(memo);
    payload
}

fn collect_filtered_transactions(stream_bytes: Vec<u8>) -> Vec<FilteredTransaction> {
    let cursor = Cursor::new(stream_bytes);
    let mut parser = StreamParser::new(cursor);
    let mut out = Vec::new();

    loop {
        match parser.next_packet() {
            Ok(None) => break,
            Ok(Some(packet)) => {
                if validate_packet(packet.header.packet_type, &packet.payload, packet.checksum).is_err() {
                    continue;
                }

                if packet.header.packet_type != PacketType::Transaction {
                    continue;
                }

                let decoded = match decode_transaction(&packet.payload) {
                    Ok(tx) => tx,
                    Err(_) => continue,
                };

                if decoded.amount > 1000 {
                    out.push(decoded);
                }
            }
            Err(_) => break,
        }
    }

    out
}

#[test]
fn ingestion_pipeline_emits_only_valid_high_value_transactions() {
    let tx_999 = build_packet(0x01, &build_tx_payload(0x01, 999, b"low"));
    let tx_1001 = build_packet(0x01, &build_tx_payload(0x02, 1001, b"above"));
    let keep_alive = build_packet(0xFF, &[1, 2, 3, 4]);
    let invalid_utf8_tx = build_packet(0x01, &build_tx_payload(0x03, 5000, &[0xFF, 0xFE]));
    let unknown_valid = build_packet(0x10, b"abcdef");
    let tx_after_garbage = build_packet(0x01, &build_tx_payload(0x04, 3000, b"after garbage"));
    let bad_checksum_tx = build_packet_with_checksum(0x01, &build_tx_payload(0x05, 99999, b"bad checksum"), 0x00);

    let mut stream = Vec::new();
    stream.extend_from_slice(&tx_999);
    stream.extend_from_slice(&tx_1001);
    stream.extend_from_slice(&keep_alive);
    stream.extend_from_slice(&invalid_utf8_tx);
    stream.extend_from_slice(&unknown_valid);
    stream.extend_from_slice(&[0x00, 0x12, 0x34, 0x56]); // garbage for resync
    stream.extend_from_slice(&tx_after_garbage);
    stream.extend_from_slice(&bad_checksum_tx);

    let out = collect_filtered_transactions(stream);

    assert_eq!(out.len(), 2);

    assert_eq!(out[0].amount, 1001);
    assert_eq!(out[0].memo, "above");
    assert_eq!(
        out[0].tx_hash,
        "0202020202020202020202020202020202020202020202020202020202020202"
    );

    assert_eq!(out[1].amount, 3000);
    assert_eq!(out[1].memo, "after garbage");
    assert_eq!(
        out[1].tx_hash,
        "0404040404040404040404040404040404040404040404040404040404040404"
    );
}