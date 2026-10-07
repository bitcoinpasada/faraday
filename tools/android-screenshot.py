#!/usr/bin/env python3
"""Capture the running emulator's screen to a PNG.

`adb exec-out screencap` returns a black image for this app: the window
carries FLAG_SECURE, and the platform excludes secure layers from every
screenshot API (docs/PLANNING.md §4.6). That is the point of the flag, and
it is not relaxed for development.

The emulator's own console captures the host-side framebuffer instead,
below the layer where the platform enforces that rule, so a review pass can
still see what the core drew. It works on an emulator and nowhere else,
which is exactly the right limit.

    tools/android-screenshot.py out/android/home.png [--console-port 5554]

The console port is the number in the `emulator-NNNN` serial that `adb
devices` prints. The auth token is the one the emulator writes to
`$HOME/.emulator_console_auth_token`.
"""

import argparse
import os
import pathlib
import shutil
import socket
import sys
import tempfile
import time


def console(port: int, token: str, command: str, timeout: float) -> str:
    """Runs one console command and returns everything the console said."""
    with socket.create_connection(("127.0.0.1", port), timeout) as sock:
        sock.settimeout(timeout)
        transcript = []

        def drain(seconds: float) -> None:
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                try:
                    chunk = sock.recv(65536)
                except socket.timeout:
                    return
                if not chunk:
                    return
                transcript.append(chunk.decode(errors="replace"))

        drain(0.5)
        sock.sendall(f"auth {token}\n".encode())
        drain(0.5)
        sock.sendall(f"{command}\n".encode())
        drain(timeout)
        return "".join(transcript)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", help="where to write the PNG")
    parser.add_argument("--console-port", type=int, default=5554)
    parser.add_argument("--timeout", type=float, default=10.0)
    args = parser.parse_args()

    token_path = pathlib.Path(os.environ["HOME"]) / ".emulator_console_auth_token"
    if not token_path.is_file():
        print(f"error: no emulator console token at {token_path}", file=sys.stderr)
        return 1
    token = token_path.read_text().strip()

    # The console writes into a directory under a name of its own choosing.
    with tempfile.TemporaryDirectory() as staging:
        reply = console(
            args.console_port, token, f"screenrecord screenshot {staging}", args.timeout
        )
        shots = sorted(pathlib.Path(staging).glob("*.png"))
        if not shots:
            print("error: the emulator console wrote no image", file=sys.stderr)
            print(reply.strip(), file=sys.stderr)
            return 1
        output = pathlib.Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(shots[-1]), output)
    print(output)
    return 0


if __name__ == "__main__":
    sys.exit(main())
