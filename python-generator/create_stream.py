#!/usr/bin/env python3

from __future__ import annotations

import argparse
import random
import socket
import struct
import sys
import time
from dataclasses import dataclass
from typing import BinaryIO, Callable, Iterable, List, Optional


MAGIC = 0xA5

PT_TX = 0x01
PT_STATE = 0x02
PT_KEEP = 0xFF

DEFAULT_SEED = 1337


@dataclass
class ManifestEntry:
    index: int
    label: str
    pkt_type: str
    payload_len: int
    valid: bool
    reason: str
    amount: Optional[int] = None
    pass_filter: Optional[bool] = None


def checksum(payload: bytes) -> int:
    c = 0
    for b in payload:
        c ^= b
    return c & 0xFF


def be_u16(value: int) -> bytes:
    return struct.pack(">H", value)


def be_u64(value: int) -> bytes:
    return struct.pack(">Q", value)


def rand_bytes(rng: random.Random, n: int) -> bytes:
    return bytes(rng.getrandbits(8) for _ in range(n))


def build_packet(
    pkt_type: int,
    payload: bytes,
    *,
    magic: int = MAGIC,
    bad_checksum: bool = False,
) -> bytes:
    if len(payload) > 0xFFFF:
        raise ValueError("payload length exceeds u16 maximum")

    packet_checksum = checksum(payload)
    if bad_checksum:
        packet_checksum ^= 0xFF

    return bytes([magic, pkt_type]) + be_u16(len(payload)) + payload + bytes([packet_checksum])


def build_tx_payload(
    rng: random.Random,
    amount: int,
    memo_bytes: bytes,
    *,
    tx_hash: Optional[bytes] = None,
) -> bytes:
    if tx_hash is None:
        tx_hash = rand_bytes(rng, 32)

    if len(tx_hash) != 32:
        raise ValueError("transaction hash must be exactly 32 bytes")

    payload = tx_hash + be_u64(amount) + memo_bytes
    if len(payload) < 40:
        raise ValueError("valid transaction payload must be at least 40 bytes")

    if len(memo_bytes) > 200:
        raise ValueError("memo length should stay reasonable for this generator")

    return payload


def build_state_payload(rng: random.Random, status: int) -> bytes:
    if not 0 <= status <= 255:
        raise ValueError("status must be in range 0..255")
    payload = rand_bytes(rng, 32) + bytes([status])
    if len(payload) != 33:
        raise AssertionError("state payload must be exactly 33 bytes")
    return payload


def build_keepalive_payload(rng: random.Random, length: int) -> bytes:
    if not 0 <= length <= 0xFFFF:
        raise ValueError("keepalive payload length must fit u16")
    return rand_bytes(rng, length)


def ensure_non_magic_garbage(rng: random.Random, length: int) -> bytes:
    if length < 1:
        raise ValueError("garbage length must be at least 1")
    first = rng.randrange(0, 256)
    while first == MAGIC:
        first = rng.randrange(0, 256)
    return bytes([first]) + rand_bytes(rng, length - 1)


class Emitter:
    def __init__(
        self,
        sink: Callable[[bytes], None],
        *,
        chunked: bool = False,
        chunk_sleep_ms: int = 0,
        rng: Optional[random.Random] = None,
    ) -> None:
        self.sink = sink
        self.chunked = chunked
        self.chunk_sleep_ms = max(0, chunk_sleep_ms)
        self.rng = rng or random.Random(DEFAULT_SEED)

    def send(self, data: bytes) -> None:
        if not self.chunked or len(data) <= 1:
            self.sink(data)
            return

        remaining = len(data)
        offset = 0

        while remaining > 0:
            chunk_size = self.rng.randint(1, min(8, remaining))
            chunk = data[offset : offset + chunk_size]
            self.sink(chunk)
            offset += chunk_size
            remaining -= chunk_size

            if remaining > 0 and self.chunk_sleep_ms > 0:
                time.sleep(self.chunk_sleep_ms / 1000.0)


def build_required_scenarios(rng: random.Random) -> tuple[list[bytes], list[ManifestEntry]]:
    packets: List[bytes] = []
    manifest: List[ManifestEntry] = []

    def add(
        *,
        label: str,
        pkt_type: str,
        payload_len: int,
        valid: bool,
        reason: str,
        data: bytes,
        amount: Optional[int] = None,
        pass_filter: Optional[bool] = None,
    ) -> None:
        manifest.append(
            ManifestEntry(
                index=len(manifest) + 1,
                label=label,
                pkt_type=pkt_type,
                payload_len=payload_len,
                valid=valid,
                reason=reason,
                amount=amount,
                pass_filter=pass_filter,
            )
        )
        packets.append(data)

    # A1
    memo = "below".encode("utf-8")
    payload = build_tx_payload(rng, 999, memo)
    add(
        label="A1 valid tx below threshold",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=True,
        reason="amount=999 memo='below'",
        data=build_packet(PT_TX, payload),
        amount=999,
        pass_filter=False,
    )

    # A2
    memo = "edge".encode("utf-8")
    payload = build_tx_payload(rng, 1000, memo)
    add(
        label="A2 valid tx at threshold",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=True,
        reason="amount=1000 memo='edge'",
        data=build_packet(PT_TX, payload),
        amount=1000,
        pass_filter=False,
    )

    # A3
    memo = "above".encode("utf-8")
    payload = build_tx_payload(rng, 1001, memo)
    add(
        label="A3 valid tx above threshold",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=True,
        reason="amount=1001 memo='above'",
        data=build_packet(PT_TX, payload),
        amount=1001,
        pass_filter=True,
    )

    # A4 multi-byte UTF-8
    memo = "café naïve résumé".encode("utf-8")
    payload = build_tx_payload(rng, 50000, memo)
    add(
        label="A4 valid tx large amount multibyte utf8",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=True,
        reason="amount=50000 memo has multibyte UTF-8 characters",
        data=build_packet(PT_TX, payload),
        amount=50000,
        pass_filter=True,
    )

    # A5
    memo = b""
    payload = build_tx_payload(rng, 2000, memo)
    if len(payload) != 40:
        raise AssertionError("empty-memo transaction payload must be exactly 40 bytes")
    add(
        label="A5 valid tx empty memo",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=True,
        reason="amount=2000 empty memo payload_len=40",
        data=build_packet(PT_TX, payload),
        amount=2000,
        pass_filter=True,
    )

    # A6
    status = rng.randint(0, 255)
    payload = build_state_payload(rng, status)
    add(
        label="A6 valid state update",
        pkt_type="0x02",
        payload_len=len(payload),
        valid=True,
        reason=f"status={status}",
        data=build_packet(PT_STATE, payload),
    )

    # A7
    payload = build_keepalive_payload(rng, 4)
    add(
        label="A7 valid keep-alive",
        pkt_type="0xFF",
        payload_len=len(payload),
        valid=True,
        reason="legacy payload len=4",
        data=build_packet(PT_KEEP, payload),
    )

    # B8 invalid UTF-8 but checksum-valid
    memo = bytes([0xFF, 0xFE, 0xC3, 0x28, 0x80])
    payload = build_tx_payload(rng, 2001, memo)
    add(
        label="B8 tx invalid utf8 memo checksum-valid",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=False,
        reason="amount=2001 memo bytes are invalid UTF-8 but framing and checksum are valid",
        data=build_packet(PT_TX, payload),
        amount=2001,
        pass_filter=True,
    )

    # C9 bad checksum only
    memo = "checksum failure".encode("utf-8")
    payload = build_tx_payload(rng, 6000, memo)
    add(
        label="C9 tx bad checksum",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=False,
        reason="amount=6000 payload is valid but checksum is intentionally wrong",
        data=build_packet(PT_TX, payload, bad_checksum=True),
        amount=6000,
        pass_filter=True,
    )

    # D10 bad magic only
    memo = "bad magic".encode("utf-8")
    payload = build_tx_payload(rng, 7000, memo)
    add(
        label="D10 packet bad magic",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=False,
        reason="bad magic only, rest of packet is structurally consistent",
        data=build_packet(PT_TX, payload, magic=0x00),
        amount=7000,
        pass_filter=True,
    )

    # E11 unknown type but otherwise valid
    payload = rand_bytes(rng, 6)
    add(
        label="E11 unknown packet type",
        pkt_type="0x10",
        payload_len=len(payload),
        valid=False,
        reason="unknown packet type with valid length and checksum, should be skipped safely",
        data=build_packet(0x10, payload),
    )

    # F12 garbage bytes
    garbage_len = rng.randint(5, 20)
    garbage = ensure_non_magic_garbage(rng, garbage_len)
    add(
        label="F12 garbage bytes between packets",
        pkt_type="garbage",
        payload_len=len(garbage),
        valid=False,
        reason=f"{garbage_len} random bytes inserted to force resync, first byte is not 0xA5",
        data=garbage,
    )

    # F12b valid packet after garbage
    memo = "after garbage".encode("utf-8")
    payload = build_tx_payload(rng, 3000, memo)
    add(
        label="F12b valid tx after garbage",
        pkt_type="0x01",
        payload_len=len(payload),
        valid=True,
        reason="amount=3000 valid packet placed after garbage for resync recovery",
        data=build_packet(PT_TX, payload),
        amount=3000,
        pass_filter=True,
    )

    # G13 truncated final packet
    claimed_len = 50
    partial_payload = rand_bytes(rng, 10)
    truncated = bytes([MAGIC, PT_STATE]) + be_u16(claimed_len) + partial_payload
    add(
        label="G13 truncated final packet",
        pkt_type="0x02",
        payload_len=claimed_len,
        valid=False,
        reason="header claims larger payload than bytes written, checksum omitted",
        data=truncated,
    )

    return packets, manifest


def print_manifest(manifest: Iterable[ManifestEntry]) -> None:
    print("\nPacket Manifest\n")
    for entry in manifest:
        amount_text = "None" if entry.amount is None else str(entry.amount)
        pass_text = "None" if entry.pass_filter is None else str(entry.pass_filter)
        print(
            f"{entry.index:>2} | "
            f"{entry.label:<36} "
            f"type={entry.pkt_type:<7} "
            f"len={entry.payload_len:<5} "
            f"amount={amount_text:<6} "
            f"pass_filter={pass_text:<5} "
            f"valid={entry.valid!s:<5} "
            f"{entry.reason}"
        )


def build_live_packet(rng: random.Random) -> tuple[bytes, str]:
    """
    Mostly valid traffic, with occasional safe-to-test corruption.
    No truncation here because abrupt termination is better simulated
    by stopping the sender mid-stream.
    """
    roll = rng.random()

    if roll < 0.55:
        memo_options = [
            "live alpha",
            "live beta",
            "café",
            "naïve",
            "résumé",
            "",
            "steady stream",
        ]
        memo = rng.choice(memo_options).encode("utf-8")
        amount = rng.choice([200, 999, 1000, 1001, 1500, 3200, 70000])
        payload = build_tx_payload(rng, amount, memo)
        return build_packet(PT_TX, payload), f"live valid tx amount={amount}"

    if roll < 0.70:
        payload = build_state_payload(rng, rng.randint(0, 255))
        return build_packet(PT_STATE, payload), "live valid state update"

    if roll < 0.82:
        payload = build_keepalive_payload(rng, rng.randint(0, 8))
        return build_packet(PT_KEEP, payload), "live valid keep-alive"

    if roll < 0.88:
        payload = rand_bytes(rng, rng.randint(1, 8))
        unknown_type = rng.choice([0x10, 0x20, 0x7E])
        return build_packet(unknown_type, payload), f"live unknown packet type=0x{unknown_type:02X}"

    if roll < 0.94:
        payload = build_tx_payload(rng, 4000, b"bad checksum live")
        return build_packet(PT_TX, payload, bad_checksum=True), "live tx bad checksum"

    if roll < 0.97:
        payload = build_tx_payload(rng, 5000, b"bad magic live")
        return build_packet(PT_TX, payload, magic=0x00), "live tx bad magic"

    garbage = ensure_non_magic_garbage(rng, rng.randint(5, 12))
    return garbage, "live garbage bytes"


def write_file_mode(
    out_path: str,
    packets: list[bytes],
) -> None:
    with open(out_path, "wb") as f:
        for pkt in packets:
            f.write(pkt)


def stdout_sink() -> Callable[[bytes], None]:
    out = sys.stdout.buffer

    def sink(data: bytes) -> None:
        out.write(data)
        out.flush()

    return sink


def file_append_sink(path: str) -> Callable[[bytes], None]:
    f = open(path, "ab")

    def sink(data: bytes) -> None:
        f.write(data)
        f.flush()

    sink._file = f  # type: ignore[attr-defined]
    return sink


def tcp_server_sink(host: str, port: int) -> Callable[[bytes], None]:
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind((host, port))
    server.listen(1)

    print(f"TCP stream server listening on {host}:{port}", file=sys.stderr)
    conn, addr = server.accept()
    print(f"Client connected from {addr[0]}:{addr[1]}", file=sys.stderr)

    def sink(data: bytes) -> None:
        conn.sendall(data)

    sink._conn = conn  # type: ignore[attr-defined]
    sink._server = server  # type: ignore[attr-defined]
    return sink


def close_sink(sink: Callable[[bytes], None]) -> None:
    for attr in ("_file", "_conn", "_server"):
        obj = getattr(sink, attr, None)
        if obj is not None:
            try:
                obj.close()
            except Exception:
                pass


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate or stream a deterministic legacy-format blockchain byte stream."
    )
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED, help="deterministic random seed")
    parser.add_argument("--out", default="../stream.bin", help="output file path for file modes")
    parser.add_argument(
        "--mode",
        choices=["file", "append", "stdout", "tcp"],
        default="file",
        help="file=write once, append=append/stream to file, stdout=write to stdout, tcp=serve over TCP",
    )
    parser.add_argument("--host", default="127.0.0.1", help="TCP bind host")
    parser.add_argument("--port", type=int, default=9000, help="TCP bind port")
    parser.add_argument(
        "--loop",
        action="store_true",
        help="after required scenarios, continue emitting live packets forever",
    )
    parser.add_argument(
        "--rate",
        type=float,
        default=5.0,
        help="live packet rate per second after required scenarios, used with --loop",
    )
    parser.add_argument(
        "--chunked",
        action="store_true",
        help="split writes into smaller chunks to simulate partial TCP/file writes",
    )
    parser.add_argument(
        "--chunk-sleep-ms",
        type=int,
        default=0,
        help="sleep between chunks in milliseconds when --chunked is enabled",
    )
    parser.add_argument(
        "--no-manifest",
        action="store_true",
        help="suppress manifest printing",
    )

    args = parser.parse_args()

    if args.rate <= 0:
        raise SystemExit("--rate must be greater than 0")

    rng = random.Random(args.seed)
    packets, manifest = build_required_scenarios(rng)

    if not args.no_manifest:
        print_manifest(manifest)

    # One-shot file mode is special because it should overwrite by default
    if args.mode == "file" and not args.loop:
        write_file_mode(args.out, packets)
        print(f"\nWrote {args.out}", file=sys.stderr)
        return 0

    if args.mode == "file" and args.loop:
        # Start clean, then switch to append-style writes for live mode
        write_file_mode(args.out, [])
        sink = file_append_sink(args.out)
    elif args.mode == "append":
        sink = file_append_sink(args.out)
    elif args.mode == "stdout":
        sink = stdout_sink()
    else:
        sink = tcp_server_sink(args.host, args.port)

    emitter = Emitter(
        sink,
        chunked=args.chunked,
        chunk_sleep_ms=args.chunk_sleep_ms,
        rng=random.Random(args.seed ^ 0xABCDEF),
    )

    try:
        for pkt in packets:
            emitter.send(pkt)

        if not args.loop:
            return 0

        interval = 1.0 / args.rate
        while True:
            pkt, _description = build_live_packet(rng)
            emitter.send(pkt)
            time.sleep(interval)

    except BrokenPipeError:
        print("output consumer disconnected", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 0
    finally:
        close_sink(sink)


if __name__ == "__main__":
    raise SystemExit(main())