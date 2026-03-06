use std::fs::File;
use std::io::{self, BufReader, Read};
use std::net::TcpStream;
use std::path::Path;

use crate::error::AppError;
use crate::parser::StreamParser;
use crate::processor::{process_packet, PacketDecision};

#[derive(Default, Debug)]
struct RunStats {
    framed_packets: u64,
    valid_packets: u64,
    emitted_transactions: u64,
    skipped_packets: u64,
    discarded_packets: u64,
}

pub fn run_file(path: &Path) -> Result<(), AppError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    process_stream(reader, &format!("file: {}", path.display()))
}

pub fn run_tcp(host: &str, port: u16) -> Result<(), AppError> {
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr)?;
    let reader = BufReader::new(stream);

    process_stream(reader, &format!("tcp://{}", addr))
}

pub fn run_stdin() -> Result<(), AppError> {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    process_stream(reader, "stdin")
}

pub fn process_stream<R: Read>(reader: R, source_label: &str) -> Result<(), AppError> {
    eprintln!("ingestion engine started, input source: {}", source_label);

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