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
                eprintln!(
                    "packet {} @ offset {}: type={} (0x{:02x}) payload_len={} checksum=0x{:02x}",
                    packet.packet_index,
                    packet.offset,
                    packet.header.packet_type.name(),
                    packet.header.packet_type.as_byte(),
                    packet.header.payload_len,
                    packet.checksum
                );
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