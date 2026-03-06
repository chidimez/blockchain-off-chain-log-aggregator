mod cli;
mod decoder;
mod error;
mod output;
mod parser;
mod protocol;
mod validate;

use std::fs::File;
use std::io::BufReader;

use cli::CliConfig;
use decoder::{decode_transaction, DecodeError};
use error::AppError;
use parser::StreamParser;
use protocol::PacketType;
use validate::{validate_packet, ValidationError};

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

    loop {
        match parser.next_packet() {
            Ok(None) => {
                eprintln!("EOF reached cleanly @ offset {}", parser.offset());
                break;
            }

            Ok(Some(packet)) => {
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
                                        Ok(line) => println!("{}", line),
                                        Err(e) => eprintln!(
                                            "packet {} @ offset {}: JSON serialization error: {}, discarded",
                                            packet.packet_index, packet.offset, e
                                        ),
                                    }
                                } else {
                                    eprintln!(
                                        "packet {} @ offset {}: transaction amount {} <= 1000, skipped",
                                        packet.packet_index, packet.offset, tx.amount
                                    );
                                }
                            }
                            Err(DecodeError::PayloadTooShort { payload_len }) => {
                                eprintln!(
                                    "packet {} @ offset {}: transaction payload too short ({}), discarded",
                                    packet.packet_index, packet.offset, payload_len
                                );
                            }
                            Err(DecodeError::InvalidUtf8Memo) => {
                                eprintln!(
                                    "packet {} @ offset {}: invalid UTF-8 memo, discarded",
                                    packet.packet_index, packet.offset
                                );
                            }
                        },

                        PacketType::StateUpdate => {
                            eprintln!(
                                "packet {} @ offset {}: state update valid, skipped",
                                packet.packet_index, packet.offset
                            );
                        }

                        PacketType::KeepAlive => {
                            eprintln!(
                                "packet {} @ offset {}: keep-alive valid, skipped",
                                packet.packet_index, packet.offset
                            );
                        }

                        PacketType::Unknown(t) => {
                            eprintln!(
                                "packet {} @ offset {}: unknown packet type 0x{:02x} valid, skipped",
                                packet.packet_index, packet.offset, t
                            );
                        }
                    },

                    Err(ValidationError::ChecksumMismatch { expected, got }) => {
                        eprintln!(
                            "packet {} @ offset {}: checksum mismatch, expected 0x{:02x} got 0x{:02x}, discarded",
                            packet.packet_index, packet.offset, expected, got
                        );
                    }

                    Err(ValidationError::InvalidTxPayloadLen { payload_len }) => {
                        eprintln!(
                            "packet {} @ offset {}: invalid transaction payload length {}, discarded",
                            packet.packet_index, packet.offset, payload_len
                        );
                    }

                    Err(ValidationError::InvalidStateUpdateLen { payload_len }) => {
                        eprintln!(
                            "packet {} @ offset {}: invalid state update payload length {}, discarded",
                            packet.packet_index, packet.offset, payload_len
                        );
                    }
                }
            }

            Err(err) => {
                eprintln!("parse error @ offset {}: {:?}. stopping.", parser.offset(), err);
                break;
            }
        }
    }

    Ok(())
}