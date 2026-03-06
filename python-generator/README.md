# Blockchain Event Stream Parser

This project simulates and processes a **raw binary blockchain event stream** using a custom legacy packet protocol.

It consists of two main components:

* **Python stream generator (`create_stream.py`)**
  Produces deterministic test streams and continuous simulated network traffic.

* **Rust backend parser**
  Connects to the stream, reconstructs packets, validates them, and outputs filtered transaction events as JSON.

The system is designed to test **stream framing recovery, checksum validation, forward compatibility, and UTF-8 safety** in a realistic blockchain-style log pipeline.

---

# Architecture Overview

```
             +------------------------+
             |  Python Stream Source  |
             |  create_stream.py      |
             +-----------+------------+
                         |
              raw TCP / file / stdout
                         |
                         v
             +------------------------+
             |   Rust Stream Parser   |
             |                        |
             |  - framing recovery   |
             |  - checksum validation|
             |  - packet decoding    |
             |  - filtering logic    |
             +-----------+------------+
                         |
                         v
                  JSON output
```

---

# Protocol Specification

All multi-byte integers are **big-endian**.

Each packet has the following structure:

| Field          | Size    | Description                           |
| -------------- | ------- | ------------------------------------- |
| Magic Byte     | 1 byte  | Always `0xA5` for valid packets       |
| Packet Type    | 1 byte  | Identifies payload schema             |
| Payload Length | 2 bytes | Unsigned big-endian length of payload |
| Payload        | N bytes | Packet data                           |
| Checksum       | 1 byte  | XOR of all payload bytes              |

Checksum is computed as:

```
checksum = 0
for byte in payload:
    checksum ^= byte
```

The checksum covers **payload only**, not header fields.

---

# Packet Types

## Transaction Event (`0x01`)

Payload layout:

| Field  | Size                     |
| ------ | ------------------------ |
| TxHash | 32 bytes                 |
| Amount | 8 bytes (u64 big-endian) |
| Memo   | Remaining bytes          |

Memo length is derived as:

```
memo_len = payload_len - 40
```

Valid transaction payloads must have:

```
payload_len >= 40
```

---

## State Update (`0x02`)

Payload layout:

| Field     | Size     |
| --------- | -------- |
| BlockHash | 32 bytes |
| Status    | 1 byte   |

Payload length must be **exactly 33 bytes**.

---

## Keep-Alive (`0xFF`)

Payload is **opaque legacy data**.
The parser only validates framing and checksum.

---

# Stream Behaviour

The input is a **continuous byte stream**, not packet-aligned.

The parser must handle:

* partial packet reads
* multiple packets in one read
* corrupted bytes between packets
* unknown packet types
* abrupt stream termination

Framing recovery must resynchronize on the **magic byte `0xA5`**.

---

# Output Behaviour

The Rust backend prints JSON for **valid transaction events where**

```
amount > 1000
```

Example output:

```json
{
  "tx_hash": "8a1f5c4d...e7",
  "amount": 50000,
  "memo": "café naïve résumé"
}
```

Transactions that fail validation or filtering are ignored.

---

# Stream Generator

The Python generator produces deterministic test streams and continuous simulated traffic.

## Features

* deterministic output with fixed seed
* valid and invalid packets
* corrupted framing
* unknown packet types
* checksum failures
* invalid UTF-8 memo bytes
* garbage bytes between packets
* truncated end-of-stream packet
* optional TCP streaming mode
* optional chunked writes to simulate partial reads

---

# Mandatory Test Scenarios

The generator writes the following scenarios:

### Valid baseline packets

1. Transaction amount `999`
2. Transaction amount `1000`
3. Transaction amount `1001`
4. Transaction amount `50000` with multi-byte UTF-8 memo
5. Transaction amount `2000` with empty memo
6. State update packet
7. Keep-alive packet

### Edge and failure cases

8. Transaction with invalid UTF-8 memo but valid checksum
9. Transaction with intentionally wrong checksum
10. Packet with incorrect magic byte
11. Packet with unknown type
12. Garbage bytes inserted between packets
13. Final truncated packet

These cases test parser robustness.

---

# Running the Generator

## Deterministic File Mode

Writes a test stream to `../stream.bin`.

```
python create_stream.py
```

---

## Continuous TCP Stream

Starts a TCP server that emits packets continuously.

```
python create_stream.py \
--mode tcp \
--port 9000 \
--loop \
--rate 10 \
--chunked \
--chunk-sleep-ms 20
```

Rust backend should connect to:

```
127.0.0.1:9000
```

---

## Continuous Stdout Stream

Useful for piping directly into the Rust program.

```
python create_stream.py --mode stdout --loop --rate 10 | cargo run
```

---

## Continuous File Stream

Appends packets continuously to a file.

```
python create_stream.py --mode file --loop --rate 5
```

---


# Example Development Workflow

Start the stream generator:

```
python create_stream.py --mode tcp --port 9000 --loop
```

Then run the Rust parser:

```
cargo run
```

The parser will output JSON for qualifying transactions.

---

# Repository Structure

```
project/
│
├── create_stream.py
├── main.py
│
└── README.md
```


