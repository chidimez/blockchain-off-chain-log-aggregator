mod cli;
mod error;
mod output;
mod protocol;
mod parser;
mod validate;

use std::fs::File;
use std::io::BufReader;

use cli::CliConfig;
use error::AppError;
use parser::StreamParser;
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

    // Sprint 2: frame packets and print framing diagnostics (stderr only).
    let mut parser = StreamParser::new(reader);

    loop {
        match parser.next_packet() {
            Ok(None) => {
                eprintln!("EOF reached cleanly @ offset {}", parser.offset());
                break;
            }
            Ok(Some(packet)) => {
                match validate_packet(packet.header.packet_type, &packet.payload, packet.checksum) {
                    Ok(()) => {
                        eprintln!(
                            "packet {} @ offset {}: type={} (0x{:02x}) payload_len={} checksum=0x{:02x} [valid]",
                            packet.packet_index,
                            packet.offset,
                            packet.header.packet_type.name(),
                            packet.header.packet_type.as_byte(),
                            packet.header.payload_len,
                            packet.checksum
                        );
                    }
                    Err(err) => {
                        match err {
                            ValidationError::ChecksumMismatch { expected, got } => {
                                eprintln!(
                                    "packet {} @ offset {}: checksum mismatch, expected 0x{:02x} got 0x{:02x}, discarded",
                                    packet.packet_index,
                                    packet.offset,
                                    expected,
                                    got
                                );
                            }
                            ValidationError::InvalidTxPayloadLen { payload_len } => {
                                eprintln!(
                                    "packet {} @ offset {}: invalid transaction payload length {}, discarded",
                                    packet.packet_index,
                                    packet.offset,
                                    payload_len
                                );
                            }
                            ValidationError::InvalidStateUpdateLen { payload_len } => {
                                eprintln!(
                                    "packet {} @ offset {}: invalid state update payload length {}, discarded",
                                    packet.packet_index,
                                    packet.offset,
                                    payload_len
                                );
                            }
                        }
                    }
                }
            }
            Err(err) => {
                // Sprint 2 rule: truncation means stop cleanly; other I/O errors stop too.
                eprintln!("parse error @ offset {}: {:?}. stopping.", parser.offset(), err);
                break;
            }
        }
    }

    Ok(())
}