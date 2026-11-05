"""The CPython helper of the QR decoders (`qr_decode.py`): Pillow and
zxing-cpp have no build for PyPy, so a worker under PyPy keeps them here
and asks. One request a line on the standard input, one answer a line:

    {"op": "zxing" | "pixels", "png": <hex>}
    -> {"ok": <hex>}  (pixels: also "size": [width, height])
    -> {"err": <text>}    the picture is unreadable
    -> {"crash": <text>}  anything else; the caller stops the run

It ends when its input does. Nothing is decided here: the same decoders
read the same pictures as when they run in the worker itself.
"""

import io
import json
import sys
import traceback

from PIL import Image


class Unreadable(Exception):
    pass


def zxing(png):
    import zxingcpp
    image = Image.open(io.BytesIO(png)).convert("L")
    found = zxingcpp.read_barcodes(image, formats=zxingcpp.BarcodeFormat.QRCode)
    if len(found) != 1:
        raise Unreadable(f"{len(found)} symbols")
    return {"ok": bytes(found[0].bytes).hex()}


def pixels(png):
    image = Image.open(io.BytesIO(png))
    image.load()
    image = image.convert("L")
    return {"ok": image.tobytes().hex(), "size": list(image.size)}


def main():
    out = sys.stdout
    sys.stdout = sys.stderr
    for line in sys.stdin:
        req = json.loads(line)
        try:
            png = bytes.fromhex(req["png"])
            answer = {"zxing": zxing, "pixels": pixels}[req["op"]](png)
        except (Unreadable, ValueError, OSError) as e:
            answer = {"err": str(e)}
        except Exception:  # noqa: BLE001 - reported, and the run stops
            answer = {"crash": traceback.format_exc()}
        out.write(json.dumps(answer) + "\n")
        out.flush()


if __name__ == "__main__":
    main()
