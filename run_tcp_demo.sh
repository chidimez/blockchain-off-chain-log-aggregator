#!/usr/bin/env bash
set -euo pipefail

GEN_DIR="python-generator"
RUST_DIR="rust-ingestion-engine"

HOST="127.0.0.1"
PORT="9000"

OUT_FILE="out.jsonl"
ERR_FILE="err.log"

cleanup() {
  if [[ -n "${GEN_PID:-}" ]] && kill -0 "$GEN_PID" 2>/dev/null; then
    echo "Stopping Python generator (PID: $GEN_PID)..."
    kill "$GEN_PID" 2>/dev/null || true
    wait "$GEN_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

echo "Starting Python TCP generator..."
(
  cd "$GEN_DIR"
  python3 create_stream.py \
    --mode tcp \
    --port "$PORT" \
    --loop \
    --rate 10 \
    --chunked \
    --chunk-sleep-ms 20
) &
GEN_PID=$!

# Give the TCP server a moment to start
sleep 2

echo "Starting Rust ingestion engine..."
(
  cd "$RUST_DIR"
  cargo run --release -- \
    --mode tcp \
    --host "$HOST" \
    --port "$PORT"
) > "../$OUT_FILE" 2> "../$ERR_FILE"

echo "Rust engine exited."
echo "Output written to: $OUT_FILE"
echo "Errors written to: $ERR_FILE"