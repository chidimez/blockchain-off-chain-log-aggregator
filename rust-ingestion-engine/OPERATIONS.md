# Operations Guide

This document lists **all operational commands** for running the generator and ingestion engine.

---

# Build Commands

Debug build

````

cargo build

```

Release build

```

cargo build --release

```

---

# Test Commands

Run all tests

```

cargo test

```

Run integration test

```

cargo test --test ingestion

```

Run tests with logs

```

cargo test -- --nocapture

```

---

# File Mode

Consume a binary file.

Default file:

```

cargo run --release

```

Custom file:

```

cargo run --release -- stream.bin

```

File in parent directory:

```

cargo run --release -- ../stream.bin

```

Capture JSON output

```

cargo run --release -- stream.bin > out.jsonl

```

Capture logs separately

```

cargo run --release -- stream.bin > out.jsonl 2> err.log

```

---

# TCP Streaming Mode

Start generator

```

python create_stream.py --mode tcp --port 9000 --loop --rate 10 --chunked --chunk-sleep-ms 20

```

Run ingestion engine

```

cargo run --release -- --mode tcp --host 127.0.0.1 --port 9000

```

Save output

```

cargo run --release -- --mode tcp --host 127.0.0.1 --port 9000 > live.jsonl

```

---

# STDIN Pipeline Mode

Pipe generator output directly into the engine.

```

python create_stream.py --mode stdout --loop --rate 10 | cargo run --release -- --mode stdin

```

Save results

```

python create_stream.py --mode stdout --loop --rate 10 | cargo run --release -- --mode stdin > out.jsonl

```

---

# File Follow Mode

Follow a continuously written binary file.

```

stdbuf -o0 tail -c +1 -f ../stream.bin | cargo run --release -- --mode stdin

```

If the file may not exist yet

```

stdbuf -o0 tail --retry -c +1 -f ../stream.bin | cargo run --release -- --mode stdin

```

---

# Recommended Continuous Stream Setup

Best option:

TCP streaming

Terminal 1:

```

python create_stream.py --mode tcp --port 9000 --loop --rate 10 --chunked --chunk-sleep-ms 20

```

Terminal 2:

```

cargo run --release -- --mode tcp --host 127.0.0.1 --port 9000

```

---

# Output Behavior

stdout:

```

JSON transaction records

```

stderr:

```

diagnostics
parser logs
summary statistics

```

Example summary:

```

summary: framed=544 valid=511 emitted=182 skipped=329 discarded=33

```

---

# Common Debug Commands

View JSON output

```

cat out.jsonl

```

Pretty print JSON

```

cat out.jsonl | jq

```

View only emitted transactions

```

cargo run --release -- stream.bin | jq

```

---

# End-to-End Example

Generate test stream

```

python create_stream.py --mode file

```

Run ingestion

```

cargo run --release -- stream.bin > out.jsonl

```

Inspect results

```

cat out.jsonl | jq

```
