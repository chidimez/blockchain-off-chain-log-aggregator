use rust_ingestion_engine::cli::{CliConfig, InputMode};
use rust_ingestion_engine::error::AppError;
use rust_ingestion_engine::transport::{run_file, run_stdin, run_tcp};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let config = CliConfig::from_args()?;

    match config.input_mode {
        InputMode::File(path) => run_file(&path),
        InputMode::Tcp { host, port } => run_tcp(&host, port),
        InputMode::Stdin => run_stdin(),
    }
}