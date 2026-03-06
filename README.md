# Binary Stream Ingestion Engine

This project simulates and processes a raw blockchain-style binary event stream. It demonstrates how to build a resilient ingestion pipeline that can parse a hostile binary stream, recover from corruption, and extract meaningful events.

The system has two components:

* **Python Generator** – simulates a blockchain node emitting binary packets.
* **Rust Ingestion Engine** – consumes the stream, validates packets, and outputs filtered transactions as JSON.

---

# Architecture

```
Mock Blockchain Node (Python)
        │
        │  TCP / File Binary Stream
        ▼
Rust Ingestion Engine
        │
        ▼
Filtered JSON Events
```

The generator produces a continuous stream of packets using a custom legacy binary protocol.
The Rust service reads that stream, validates it, and emits only transactions we care about.

---

# Protocol Overview

Each packet follows this structure:

```
Magic Byte (1 byte)      -> 0xA5
Packet Type (1 byte)     -> 0x01, 0x02, 0xFF, etc.
Payload Length (2 bytes) -> big-endian
Payload (N bytes)
Checksum (1 byte)        -> XOR of payload bytes
```

### Packet Types

| Type  | Description                  |
| ----- | ---------------------------- |
| 0x01  | Transaction Event            |
| 0x02  | State Update                 |
| 0xFF  | Keep Alive                   |
| other | Unknown / forward-compatible |

### Transaction Payload

```
TxHash (32 bytes)
Amount (8 bytes, u64 big-endian)
Memo (UTF-8 string, remainder of payload)
```

Only transactions where **amount > 1000** are emitted.

---

# Features

The generator intentionally produces edge cases to stress test the parser:

* corrupted checksums
* invalid UTF-8 memos
* unknown packet types
* garbage bytes between packets
* truncated packets
* random chunked TCP transmission

The Rust ingestion engine handles these safely by:

* validating checksums
* rejecting malformed packets
* recovering from desynchronization using the magic byte
* skipping unknown packet types
* continuing processing after errors

---

# Project Structure

```
.
├── python-generator/
│   └── create_stream.py
│    ── ......
│
├── rust-ingestion-engine/
│   ├── Cargo.toml
│   └── src/
│
├── run_tcp_demo.sh
│
├── out.jsonl
└── err.log
```

---

# Running the Demo

The easiest way to test the system is using the provided shell script.

Make it executable:

```bash
chmod +x run_file_demo.sh
chmod +x run_tcp_demo.sh
```

Run the demo:

```bash
./run_file_demo.sh
./run_tcp_demo.sh
```

This will:

1. Start the Python generator as a TCP server
2. Start the Rust ingestion engine
3. Stream packets into the engine
4. Save results

Outputs:

```
out.jsonl   -> filtered transactions
err.log     -> parser diagnostics
```

---

# Manual Run

### Start the generator

```
cd python-generator

python3 create_stream.py \
  --mode tcp \
  --port 9000 \
  --loop \
  --rate 10 \
  --chunked \
  --chunk-sleep-ms 20
```

### Run the Rust ingestion engine

```
cd rust-ingestion-engine

cargo run --release -- \
  --mode tcp \
  --host 127.0.0.1 \
  --port 9000
```

Or redirect output to files:

```
cargo run --release -- \
  --mode tcp \
  --host 127.0.0.1 \
  --port 9000 \
  > ../out.jsonl 2> ../err.log
```

---

# Example Output

```
{"tx_hash":"a9c3...","amount":50000,"memo":"big transfer"}
{"tx_hash":"2f71...","amount":12000,"memo":"payment"}
```

---

# Summary Output

At shutdown the engine prints a summary such as:

```
summary: framed=544 valid=511 emitted=182 skipped=329 discarded=33
```

Where:

| Metric    | Meaning                    |
| --------- | -------------------------- |
| framed    | packets detected in stream |
| valid     | packets passing checksum   |
| emitted   | transactions output        |
| skipped   | valid but filtered         |
| discarded | malformed packets          |

---

# Design Goals

This project demonstrates:

* safe binary protocol parsing
* stream framing recovery
* defensive input validation
* forward-compatible packet handling
* efficient event filtering

It models the kind of ingestion middleware often used in blockchain nodes, SIEM pipelines, or high-throughput event systems.
