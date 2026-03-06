#!/usr/bin/env bash
set -euo pipefail

GEN_DIR="python-generator"
RUST_DIR="rust-ingestion-engine"

STREAM_FILE="../stream.bin"
OUT_FILE="out.jsonl"
ERR_FILE="err.log"

echo "Generating stream.bin..."
(
  cd "$GEN_DIR"
  python3 create_stream.py --mode file --loop --rate 5
)

echo "Running Rust ingestion engine..."
(
  cd "$RUST_DIR"
  cargo run --release -- --mode file --input "$STREAM_FILE"
) > "../$OUT_FILE" 2> "../$ERR_FILE"

echo "Done."
echo "Output written to: $OUT_FILE"
echo "Errors written to: $ERR_FILE"