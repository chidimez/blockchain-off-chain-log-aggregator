use std::env;
use std::path::PathBuf;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct CliConfig {
    pub input_path: PathBuf,
}

impl CliConfig {
    pub fn from_args() -> Result<Self, AppError> {
        let args: Vec<String> = env::args().collect();

        match args.len() {
            1 => Ok(Self {
                input_path: PathBuf::from("stream.bin"),
            }),
            2 => Ok(Self {
                input_path: PathBuf::from(&args[1]),
            }),
            _ => Err(AppError::InvalidArguments(
                "usage: cargo run --release -- [stream-file]".to_string(),
            )),
        }
    }
}