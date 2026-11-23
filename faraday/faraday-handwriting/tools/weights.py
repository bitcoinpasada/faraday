#!/usr/bin/env python3
"""Writes the files faraday-handwriting compiles in or tests against, from the
published files of the Haller handwriting recognizer (stdlib only; read-only
on the sources).

    python3 -I -B weights.py <resources dir> <handwriting.c> <crate dir>

<resources dir> holds haller-1-capitals.npz, haller-1-detector.npz and their
-vectors.json files; <handwriting.c> is the C file with the log-add table.
Both weight files are checked against the SHA-256 the README names. Writes,
under <crate dir>: weights/word.bin, weights/detector.bin, weights/logadd.bin
and tests/data/capitals.bin, tests/data/detector.bin (layouts in the Rust
sources that read them). Everything is little-endian.
"""
import base64
import hashlib
import json
import re
import struct
import sys
import zipfile
import zlib

WORD_SHA = "2e9275f1acd0d347437e856160430e96a283ec484a48ab96a2ea1ca91531fe14"
DETECTOR_SHA = "b33f6d02f1af87a4aff1cecc73368cc2230c2fa68d0eda977a112df437d501cc"

WORD_LAYERS = [("b1", (16, 9)), ("b2", (32, 3, 3, 16)), ("b3", (64, 3, 3, 32)),
               ("b4", (64, 3, 3, 64)), ("t1", (128, 3, 128)), ("t2", (128, 3, 128))]
DETECTOR_LAYERS = [("e1", (8, 3, 3, 1)), ("e2", (16, 3, 3, 8)), ("e3", (32, 3, 3, 16)),
                   ("e4", (48, 3, 3, 32)), ("e5", (64, 3, 3, 48)), ("e6", (64, 3, 3, 64)),
                   ("e7", (64, 3, 3, 64)), ("e8", (64, 3, 3, 64)), ("d4", (32, 3, 3, 128)),
                   ("d3", (32, 3, 3, 64)), ("d2", (16, 3, 3, 48)), ("d1", (8, 3, 3, 24))]


def npy(z, name, descr, shape):
    """The data of one array of the archive, its header checked."""
    raw = z.read(name + ".npy")
    assert raw[:8] == b"\x93NUMPY\x01\x00", name
    hl = struct.unpack("<H", raw[8:10])[0]
    head = raw[10:10 + hl].decode()
    m = re.fullmatch(r"\{'descr': '(\S+)', 'fortran_order': False, 'shape': \(([^)]*)\), \}\s*", head)
    assert m, (name, head)
    assert m.group(1) == descr, (name, head)
    got = tuple(int(v) for v in m.group(2).split(",") if v.strip())
    assert got == tuple(shape), (name, got, shape)
    data = raw[10 + hl:]
    n = 1
    for s in shape:
        n *= s
    assert len(data) == n * {"<i2": 2, "<i4": 4, "<f4": 4}[descr], name
    return data


def flat(path, expected, layers, head):
    raw = open(path, "rb").read()
    assert hashlib.sha256(raw).hexdigest() == expected, path
    z = zipfile.ZipFile(path)
    out = b""
    for name, shape in layers:
        out += npy(z, name + ".weight", "<i2", shape)
        out += npy(z, name + ".bias", "<i4", shape[:1])
        out += npy(z, name + ".mul", "<i4", shape[:1])
    hname, hshape, sname = head
    out += npy(z, hname + ".weight", "<i2", hshape)
    out += npy(z, hname + "." + sname, "<f4", hshape[:1])
    out += npy(z, hname + ".bias", "<f4", hshape[:1])
    return out


def table(c_path):
    text = open(c_path).read()
    body = text.split("static const float lut[LUT_SIZE + 1] = {", 1)[1].split("};", 1)[0]
    vals = re.findall(r"0x[0-9a-f.]+p[-+]?\d+f", body)
    assert len(vals) == 1025, len(vals)
    out = b""
    for v in vals:
        f = float.fromhex(v[:-1])
        bits = struct.pack("<f", f)
        assert struct.unpack("<f", bits)[0] == f, v
        out += bits
    return out


def capitals(path, expected):
    v = json.load(open(path))
    assert v["weights_sha256"] == expected and v["beam"] == 30.0
    strips = v["strips"]
    out = struct.pack("<I", len(strips))
    for s in strips:
        width = s["width"]
        ink = zlib.decompress(base64.b64decode(s["ink"]))
        assert len(ink) == 32 * width, s["label"]
        assert len(s["words"]) == 5 and len(s["scores"]) == 5
        out += struct.pack("<I", width) + ink
        out += bytes.fromhex(s["logits_sha256"]) + bytes.fromhex(s["scores_sha256"])
        for w, sc in zip(s["words"], s["scores"]):
            wb = w.encode()
            assert len(wb) <= 12
            out += wb.ljust(12, b"\0") + struct.pack("<f", sc)
    # After the last strip, one label per strip as the strip was written
    # (12 bytes, zero-padded; "" for a strip of noise; a line number may
    # stand before the word), for the tests that check a clear reading
    # against what was written, not against Haller's top answer.
    for s in strips:
        lb = s["label"].encode()
        assert len(lb) <= 12, lb
        out += lb.ljust(12, b"\0")
    return out


def detector(path, expected):
    v = json.load(open(path))
    assert v["weights_sha256"] == expected
    images = v["images"]
    out = struct.pack("<I", len(images))
    for im in images:
        pix = zlib.decompress(base64.b64decode(im["image"]))
        assert len(pix) == 384 * 288
        assert len(im["maps_max"]) == 11
        out += pix + bytes.fromhex(im["maps_sha256"])
        out += b"".join(struct.pack("<f", m) for m in im["maps_max"])
    return out


def main(res, c_path, crate):
    files = {
        "weights/word.bin": flat(res + "/haller-1-capitals.npz", WORD_SHA, WORD_LAYERS, ("out", (37, 128), "scale")),
        "weights/detector.bin": flat(res + "/haller-1-detector.npz", DETECTOR_SHA, DETECTOR_LAYERS,
                                     ("head", (11, 3, 3, 8), "scale")),
        "weights/logadd.bin": table(c_path),
        "tests/data/capitals.bin": capitals(res + "/haller-1-capitals-vectors.json", WORD_SHA),
        "tests/data/detector.bin": detector(res + "/haller-1-detector-vectors.json", DETECTOR_SHA),
    }
    for name, data in files.items():
        assert len(data) < 2 * 1024 * 1024, name
        with open(crate + "/" + name, "wb") as f:
            f.write(data)
        print(name, len(data), hashlib.sha256(data).hexdigest())


if len(sys.argv) != 4:
    sys.exit(__doc__)
main(*sys.argv[1:])
