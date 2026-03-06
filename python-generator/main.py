#!/usr/bin/env python3

import subprocess
import sys


TCP_MODE = [
    sys.executable,
    "create_stream.py",
    "--mode", "tcp",
    "--port", "9000",
    "--loop",
    "--rate", "10",
    "--chunked",
    "--chunk-sleep-ms", "20",
]

FILE_MODE = [
    sys.executable,
    "create_stream.py",
    "--mode", "file",
    "--loop",
    "--rate", "5",
]

DEFAULT_MODE = [
    sys.executable,
    "create_stream.py",
]


def main():
    cmd = TCP_MODE
    # cmd = FILE_MODE
    # cmd = DEFAULT_MODE

    subprocess.run(cmd, check=True)


if __name__ == "__main__":
    main()