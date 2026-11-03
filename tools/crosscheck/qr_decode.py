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

import io
import os
import subprocess
import tempfile

from PIL import Image

from common import Mismatch

ZBAR = "seedqr.zbar"
ZXING = "seedqr.zxing"
READERS = (ZBAR, ZXING)


class Unreadable(Exception):
    pass


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
    import zxingcpp  # noqa: PLC0415 (a decoder PyPy may lack: the area then runs under CPython)
    image = Image.open(io.BytesIO(png)).convert("L")
    found = zxingcpp.read_barcodes(image, formats=zxingcpp.BarcodeFormat.QRCode)
    if len(found) != 1:
        raise Unreadable(f"{len(found)} symbols")
    return bytes(found[0].bytes)


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
