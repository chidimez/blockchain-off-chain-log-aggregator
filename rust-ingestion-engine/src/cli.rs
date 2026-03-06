use std::env;
use std::path::PathBuf;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub enum InputMode {
    File(PathBuf),
    Tcp { host: String, port: u16 },
    Stdin,
}

#[derive(Debug, Clone)]
pub struct CliConfig {
    pub input_mode: InputMode,
}

impl CliConfig {
    pub fn from_args() -> Result<Self, AppError> {
        let mut args = env::args().skip(1);

        let mut mode = String::from("file");
        let mut path: Option<PathBuf> = None;
        let mut host = String::from("127.0.0.1");
        let mut port: u16 = 9000;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--mode" => {
                    mode = args.next().ok_or_else(|| {
                        AppError::InvalidArguments("missing value for --mode".to_string())
                    })?;
                }

                "--path" => {
                    let value = args.next().ok_or_else(|| {
                        AppError::InvalidArguments("missing value for --path".to_string())
                    })?;
                    path = Some(PathBuf::from(value));
                }

                "--host" => {
                    host = args.next().ok_or_else(|| {
                        AppError::InvalidArguments("missing value for --host".to_string())
                    })?;
                }

                "--port" => {
                    let raw = args.next().ok_or_else(|| {
                        AppError::InvalidArguments("missing value for --port".to_string())
                    })?;

                    port = raw.parse::<u16>().map_err(|_| {
                        AppError::InvalidArguments(format!("invalid port: {}", raw))
                    })?;
                }

                "--help" | "-h" => {
                    return Err(AppError::InvalidArguments(Self::usage()));
                }

                other if !other.starts_with('-') && path.is_none() && mode == "file" => {
                    path = Some(PathBuf::from(other));
                }

                other => {
                    return Err(AppError::InvalidArguments(format!(
                        "unrecognized argument: {}\n\n{}",
                        other,
                        Self::usage()
                    )));
                }
            }
        }

        let input_mode = match mode.as_str() {
            "file" => InputMode::File(path.unwrap_or_else(|| PathBuf::from("stream.bin"))),
            "tcp" => InputMode::Tcp { host, port },
            "stdin" => InputMode::Stdin,
            other => {
                return Err(AppError::InvalidArguments(format!(
                    "unsupported mode: {}\n\n{}",
                    other,
                    Self::usage()
                )));
            }
        };

        Ok(Self { input_mode })
    }

    fn usage() -> String {
        [
            "usage:",
            "  cargo run --release -- [stream-file]",
            "  cargo run --release -- --mode file [--path stream.bin]",
            "  cargo run --release -- --mode tcp [--host 127.0.0.1] [--port 9000]",
            "  cargo run --release -- --mode stdin",
        ]
            .join("\n")
    }
}