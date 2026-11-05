"""Two outside decoders read every QR picture Faraday writes
(docs/ESTABLISHED.md §9.4, batch X21): zbar (`zbarimg`, zbar-tools) and
zxing-cpp (`zxingcpp`, python3-zxing-cpp). Neither shares code with
Faraday's encoder, which is a port of Nayuki's, nor with rqrr.

What each returns is bytes, compared as bytes with the payload the
picture is meant to hold:

- zbar: `zbarimg --raw -Sbinary` writes the symbol's data as it is, with no
  character-set guess and no newline added; without `-Sbinary` it
  converts bytes of 0x80 and over to UTF-8 and ends each symbol with a
  newline. Only the QR code symbology is on, so the label drawn above a
  picture is not read as anything.
- zxing-cpp: `Barcode.bytes` is the data as it is, in any mode (`text` is
  its decoding, which this module does not use).

A picture either decoder cannot read, reads as no symbol or as more than
one, or reads as other bytes than the payload, is a disagreement
(`common.Mismatch`) naming the picture; its `op` is `seedqr.zbar` or
`seedqr.zxing`, the operations the self-test makes wrong (`swapped`: the
decoder's last byte is changed, as a decoder reading wrongly would).
"""

import atexit
import json
import os
import pathlib
import subprocess
import sys
import tempfile

from common import Mismatch

ZBAR = "seedqr.zbar"
ZXING = "seedqr.zxing"
READERS = (ZBAR, ZXING)


class Unreadable(Exception):
    pass


class HelperFailed(RuntimeError):
    """The CPython helper died or crashed: the run stops, it is never a
    picture a decoder could not read."""


_helper = None


def _cpython():
    """CPython, for the helper: this interpreter when it is one, else the
    one the driver passed down."""
    if sys.implementation.name == "cpython":
        return sys.executable
    return os.environ.get("FARADAY_CROSSCHECK_CPYTHON") or "python3"


def _stop_helper():
    global _helper
    if _helper is not None:
        try:
            _helper.stdin.close()
            _helper.wait(timeout=10)
        except (OSError, subprocess.TimeoutExpired):
            _helper.kill()
        _helper = None


def ask(op, png):
    """The helper's answer to `op` on a PNG. The helper (`qr_helper.py`:
    Pillow and zxing-cpp, which PyPy cannot import) starts on the first
    question and lives as long as this process."""
    global _helper
    if _helper is not None and _helper.poll() is not None:
        raise HelperFailed(f"the QR helper exited with {_helper.returncode}")
    if _helper is None:
        _helper = subprocess.Popen(
            [_cpython(), "-I", str(pathlib.Path(__file__).with_name("qr_helper.py"))],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        atexit.register(_stop_helper)
    try:
        _helper.stdin.write(json.dumps({"op": op, "png": png.hex()}) + "\n")
        _helper.stdin.flush()
        line = _helper.stdout.readline()
    except OSError as e:
        raise HelperFailed(f"the QR helper cannot be reached: {e}") from e
    if not line:
        raise HelperFailed("the QR helper ended without an answer")
    answer = json.loads(line)
    if "crash" in answer:
        raise HelperFailed(f"the QR helper failed:\n{answer['crash']}")
    return answer


def pixels(png):
    """Pillow's reading of a PNG: (luma bytes, (width, height))."""
    a = ask("pixels", png)
    if "err" in a:
        raise AssertionError(f"Pillow cannot read the picture: {a['err']}")
    return bytes.fromhex(a["ok"]), tuple(a["size"])


def zbar_read(png):
    """The one QR symbol zbar finds in a PNG, as bytes."""
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "picture.png")
        with open(path, "wb") as f:
            f.write(png)
        run = subprocess.run(
            ["zbarimg", "--raw", "-q", "-Sdisable", "-Sqrcode.enable", "-Sbinary", path],
            capture_output=True, timeout=120)
    if run.returncode != 0:
        raise Unreadable(f"zbarimg exits {run.returncode}: {run.stderr.decode(errors='replace')[:80]}")
    return run.stdout


def zxing_read(png):
    """The one QR symbol zxing-cpp finds in a PNG, as bytes."""
    a = ask("zxing", png)
    if "err" in a:
        raise Unreadable(a["err"])
    return bytes.fromhex(a["ok"])


def swapped(data):
    """A wrong answer: the last byte changed."""
    return data[:-1] + bytes([(data[-1] ^ 1) if data else 0])


def check(area, picture, png, payload, sabotage=None):
    """Both decoders read `png`, named `picture`, as `payload` (bytes)."""
    for op, read in ((ZBAR, zbar_read), (ZXING, zxing_read)):
        try:
            got = read(png)
        except (Unreadable, subprocess.TimeoutExpired, OSError, ValueError) as e:
            raise Mismatch(area, op, {"picture": picture}, ("ok", payload.hex()),
                           ("err", str(e)), f"{op[len('seedqr.'):]} cannot read {picture}")
        if sabotage == op:
            got = swapped(got)
        if got != payload:
            raise Mismatch(area, op, {"picture": picture}, ("ok", payload.hex()),
                           ("ok", got.hex()),
                           f"{op[len('seedqr.'):]} reads {picture} as other bytes than it holds")
