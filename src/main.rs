use std::fs::File;
use std::io::BufReader;

use rust_ingestion_engine::cli::CliConfig;
use rust_ingestion_engine::error::AppError;
use rust_ingestion_engine::parser::StreamParser;
use rust_ingestion_engine::processor::{process_packet, PacketDecision};

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

                match process_packet(&packet) {
                    PacketDecision::EmitJson(line) => {
                        println!("{}", line);
                        stats.valid_packets += 1;
                        stats.emitted_transactions += 1;
                    }

                    PacketDecision::Skip(message) => {
                        eprintln!("{}", message);
                        stats.valid_packets += 1;
                        stats.skipped_packets += 1;
                    }

                    PacketDecision::Discard(message) => {
                        eprintln!("{}", message);
                        stats.discarded_packets += 1;
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