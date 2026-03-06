use std::fs::File;
use std::io::BufReader;

use rust_ingestion_engine::cli::CliConfig;
use rust_ingestion_engine::decoder::{decode_transaction, DecodeError};
use rust_ingestion_engine::error::AppError;
use rust_ingestion_engine::parser::StreamParser;
use rust_ingestion_engine::protocol::PacketType;
use rust_ingestion_engine::validate::{validate_packet, ValidationError};

#[derive(Default, Debug)]
struct RunStats {
    framed_packets: u64,
    valid_packets: u64,
    emitted_transactions: u64,
    skipped_packets: u64,
    discarded_packets: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let config = CliConfig::from_args()?;

    let file = File::open(&config.input_path)?;
    let reader = BufReader::new(file);

    eprintln!(
        "ingestion engine started, input file: {}",
        config.input_path.display()
    );

    let mut parser = StreamParser::new(reader);
    let mut stats = RunStats::default();

    loop {
        match parser.next_packet() {
            Ok(None) => {
                eprintln!("EOF reached cleanly @ offset {}", parser.offset());
                break;
            }

            Ok(Some(packet)) => {
                stats.framed_packets += 1;

                match validate_packet(
                    packet.header.packet_type,
                    &packet.payload,
                    packet.checksum,
                ) {
                    Ok(()) => {
                        stats.valid_packets += 1;

                        match packet.header.packet_type {
                            PacketType::Transaction => match decode_transaction(&packet.payload) {
                                Ok(tx) => {
                                    if tx.amount > 1000 {
                                        match serde_json::to_string(&tx) {
                                            Ok(line) => {
                                                println!("{}", line);
                                                stats.emitted_transactions += 1;
                                            }
                                            Err(e) => {
                                                stats.discarded_packets += 1;
                                                eprintln!(
                                                    "packet {} @ offset {}: JSON serialization error: {}, discarded",
                                                    packet.packet_index, packet.offset, e
                                                );
                                            }
                                        }
                                    } else {
                                        stats.skipped_packets += 1;
                                        eprintln!(
                                            "packet {} @ offset {}: transaction amount {} <= 1000, skipped",
                                            packet.packet_index, packet.offset, tx.amount
                                        );
                                    }
                                }

                                Err(DecodeError::PayloadTooShort { payload_len }) => {
                                    stats.discarded_packets += 1;
                                    eprintln!(
                                        "packet {} @ offset {}: transaction payload too short ({}), discarded",
                                        packet.packet_index, packet.offset, payload_len
                                    );
                                }

                                Err(DecodeError::InvalidUtf8Memo) => {
                                    stats.discarded_packets += 1;
                                    eprintln!(
                                        "packet {} @ offset {}: invalid UTF-8 memo, discarded",
                                        packet.packet_index, packet.offset
                                    );
                                }
                            },

                            PacketType::StateUpdate => {
                                stats.skipped_packets += 1;
                                eprintln!(
                                    "packet {} @ offset {}: state update valid, skipped",
                                    packet.packet_index, packet.offset
                                );
                            }

                            PacketType::KeepAlive => {
                                stats.skipped_packets += 1;
                                eprintln!(
                                    "packet {} @ offset {}: keep-alive valid, skipped",
                                    packet.packet_index, packet.offset
                                );
                            }

                            PacketType::Unknown(t) => {
                                stats.skipped_packets += 1;
                                eprintln!(
                                    "packet {} @ offset {}: unknown packet type 0x{:02x} valid, skipped",
                                    packet.packet_index, packet.offset, t
                                );
                            }
                        }
                    }

                    Err(ValidationError::ChecksumMismatch { expected, got }) => {
                        stats.discarded_packets += 1;
                        eprintln!(
                            "packet {} @ offset {}: checksum mismatch, expected 0x{:02x} got 0x{:02x}, discarded",
                            packet.packet_index, packet.offset, expected, got
                        );
                    }

                    Err(ValidationError::InvalidTxPayloadLen { payload_len }) => {
                        stats.discarded_packets += 1;
                        eprintln!(
                            "packet {} @ offset {}: invalid transaction payload length {}, discarded",
                            packet.packet_index, packet.offset, payload_len
                        );
                    }

                    Err(ValidationError::InvalidStateUpdateLen { payload_len }) => {
                        stats.discarded_packets += 1;
                        eprintln!(
                            "packet {} @ offset {}: invalid state update payload length {}, discarded",
                            packet.packet_index, packet.offset, payload_len
                        );
                    }
                }
            }

            Err(err) => {
                eprintln!(
                    "parse error @ offset {}: {}. stopping cleanly.",
                    parser.offset(),
                    err
                );
                break;
            }
        }
    }

    eprintln!(
        "summary: framed={} valid={} emitted={} skipped={} discarded={}",
        stats.framed_packets,
        stats.valid_packets,
        stats.emitted_transactions,
        stats.skipped_packets,
        stats.discarded_packets
    );

    Ok(())
}