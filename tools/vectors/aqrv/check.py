#!/usr/bin/env python3
"""A second reader and writer for the Argon QR Vault (AQRV), written from
docs/AQRV.md section 2 to 10 alone, and run over every case of aqrv.json.

It takes an envelope and a passphrase and answers: not recognised; refused,
and why; or opened, with the fields. It also seals: for each case that opens,
the same inputs (passphrase, step, size, kind, item, salt, padding) must give
the same envelope.

Libraries other than gen.py's: PyCA cryptography's Argon2id; HKDF-Expand and
HMAC-SHA-256 over Python's hmac and hashlib; ChaCha20 written out below from
RFC 8439 and checked against its section 2.4.2 vector before anything else.

    python3 -I -B tools/vectors/aqrv/check.py [--no-qr]

Prints a one-line count and every disagreement; exits 1 on any. Written by
Faraday's authors, so it is a second reader and not the outside
implementation docs/AQRV.md section 11 asks for.
"""

import functools
import hashlib
import hmac
import json
import os
import re
import struct
import subprocess
import sys
import tempfile
import unicodedata
import zlib

from cryptography.hazmat.primitives.kdf.argon2 import Argon2id

HERE = os.path.dirname(os.path.abspath(__file__))

# ---------------------------------------------------------------- ChaCha20


def _rotl(x, n):
    return ((x << n) & 0xFFFFFFFF) | (x >> (32 - n))


def _quarter(s, a, b, c, d):
    s[a] = (s[a] + s[b]) & 0xFFFFFFFF
    s[d] = _rotl(s[d] ^ s[a], 16)
    s[c] = (s[c] + s[d]) & 0xFFFFFFFF
    s[b] = _rotl(s[b] ^ s[c], 12)
    s[a] = (s[a] + s[b]) & 0xFFFFFFFF
    s[d] = _rotl(s[d] ^ s[a], 8)
    s[c] = (s[c] + s[d]) & 0xFFFFFFFF
    s[b] = _rotl(s[b] ^ s[c], 7)


def chacha20_block(key, counter, nonce):
    init = (
        [0x61707865, 0x3320646E, 0x79622D32, 0x6B206574]
        + list(struct.unpack("<8I", key))
        + [counter & 0xFFFFFFFF]
        + list(struct.unpack("<3I", nonce))
    )
    s = list(init)
    for _ in range(10):
        _quarter(s, 0, 4, 8, 12)
        _quarter(s, 1, 5, 9, 13)
        _quarter(s, 2, 6, 10, 14)
        _quarter(s, 3, 7, 11, 15)
        _quarter(s, 0, 5, 10, 15)
        _quarter(s, 1, 6, 11, 12)
        _quarter(s, 2, 7, 8, 13)
        _quarter(s, 3, 4, 9, 14)
    return struct.pack("<16I", *[(a + b) & 0xFFFFFFFF for a, b in zip(s, init)])


def chacha20_stream(key, counter, nonce, n):
    out = b""
    while len(out) < n:
        out += chacha20_block(key, counter, nonce)
        counter += 1
    return out[:n]


def xor(a, b):
    return bytes(x ^ y for x, y in zip(a, b))


def chacha20_self_test():
    key = bytes(range(32))
    nonce = bytes.fromhex("000000000000004a00000000")
    pt = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it."
    want = bytes.fromhex(
        "6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0b"
        "f91b65c5524733ab8f593dabcd62b3571639d624e65152ab8f530c359f0861d8"
        "07ca0dbf500d6a6156a38e088a22b65e52bc514d16ccf806818ce91ab7793736"
        "5af90bbf74a35be6b40b8eedf2785e42874d"
    )
    return xor(pt, chacha20_stream(key, 1, nonce, len(pt))) == want


# ---------------------------------------------------------------- the format

STEP_KIB = {0: 65536, 1: 131072, 2: 262144, 3: 524288, 4: 1048576}
SIZES = {53: (32, 4), 62: (37, 8)}  # envelope length -> room, tag length
KINDS = [0x10, 0x14, 0x18, 0x1C, 0x20, 0x30, 0x40, 0x41, 0x50]
ENTROPY = {0x10: 16, 0x14: 20, 0x18: 24, 0x1C: 28, 0x20: 32}
LABEL_ENC = b"argon-qr-vault-1 enc"
LABEL_MAC = b"argon-qr-vault-1 mac"

CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"  # BIP-173 values
CODEX_SIZE = {0: (26, 21), 1: (32, 25), 2: (39, 29), 3: (45, 33), 4: (52, 37)}  # c, item bytes
CODEX_LEN = {22 + c: s for s, (c, _) in CODEX_SIZE.items()}  # string length -> size
GEN = [0x19DC500CE73FDE210, 0x1BFAE00DEF77FE529, 0x1FBD920FFFE7BEE52, 0x1739640BDEEE3FDAD, 0x07729A039CFC75F5A]
MS32_CONST = 0x10CE0795C2FD1E62A


def ms32_polymod(values):
    residue = 0x23181B3
    for v in values:
        b = residue >> 60
        residue = ((residue & 0x0FFFFFFFFFFFFFFF) << 5) ^ v
        for i in range(5):
            if (b >> i) & 1:
                residue ^= GEN[i]
    return residue


def ms32_checksum(data):
    p = ms32_polymod(list(data) + [0] * 13) ^ MS32_CONST
    return [(p >> (5 * (12 - i))) & 31 for i in range(13)]


def hkdf_expand_one(prk, info):
    """RFC 5869 section 2.3, one block: T(1) = HMAC(PRK, info || 0x01)."""
    return hmac.new(prk, info + b"\x01", hashlib.sha256).digest()


@functools.lru_cache(maxsize=None)
def argon(pw, salt, kib):
    return Argon2id(salt=salt, length=32, iterations=3, lanes=4, memory_cost=kib).derive(pw)


def derive(passphrase, salt, kib):
    pw = unicodedata.normalize("NFC", passphrase).encode("utf-8")
    prk = argon(pw, salt, kib)
    return pw, prk, hkdf_expand_one(prk, LABEL_ENC), hkdf_expand_one(prk, LABEL_MAC)


def tags_for(k_mac, header, n, salt, c, t):
    out = {}
    for kind in KINDS:
        msg = bytes([header, n]) + salt + bytes([kind]) + c
        out[kind] = hmac.new(k_mac, msg, hashlib.sha256).digest()[:t]
    return out


# ---------------------------------------------------------------- reading


class Refused(Exception):
    """The item fails its kind's layout (section 6, reading, step 4)."""

    def __init__(self, rule):
        super().__init__(rule)
        self.rule = rule


def read_codex32(p):
    size = p[0] >> 5
    if size > 4:
        raise Refused("size")
    c, nbytes = CODEX_SIZE[size]
    if nbytes > len(p):
        raise Refused("room")
    upper = (p[0] >> 4) & 1
    nbits = 4 + 5 + 20 + 5 + 5 * c
    big = int.from_bytes(p[:nbytes], "big")
    chars = []
    for i in range(1 + 4 + 1 + c):
        shift = nbytes * 8 - 4 - 5 * (i + 1)
        chars.append((big >> shift) & 31)
    assert 4 + 5 * len(chars) == nbits
    threshold, index = chars[0], chars[5]
    if threshold not in [CHARSET.index(d) for d in "023456789"]:
        raise Refused("threshold")
    if threshold == CHARSET.index("0") and index != CHARSET.index("s"):
        raise Refused("threshold-index")
    text = "ms1" + "".join(CHARSET[v] for v in chars) + "".join(CHARSET[v] for v in ms32_checksum(chars))
    if upper:
        text = text.upper()
    return {"type": "codex32", "string": text, "is_secret": index == CHARSET.index("s")}


def read_text(p, typ):
    n = p[0]
    if n < 1 or n > len(p) - 1:
        raise Refused("len")
    try:
        text = bytes(p[1 : 1 + n]).decode("utf-8")
    except UnicodeDecodeError:
        raise Refused("utf8") from None
    return {"type": typ, "text": text}


def read_item(kind, p):
    if kind in ENTROPY:
        return {"type": "entropy", "hex": p[: ENTROPY[kind]].hex()}
    if kind == 0x50:
        return {"type": "raw_key", "hex": p[:32].hex()}
    if kind == 0x30:
        return read_codex32(p)
    if kind == 0x40:
        return read_text(p, "passphrase")
    return read_text(p, "note")


def recognised(env):
    return len(env) in SIZES and 0xF8 <= env[0] <= 0xFC


def read(env, passphrase):
    """Returns (answer, values). answer is ("not_recognised",), ("no_match",),
    ("ambiguous",), ("layout", kind, rule) or ("opens", kind, item)."""
    if not recognised(env):
        return ("not_recognised",), {}
    header, n = env[0], len(env)
    room, t = SIZES[n]
    salt, c, tag = env[1:17], env[17 : 17 + room], env[17 + room :]
    pw, prk, k_enc, k_mac = derive(passphrase, salt, STEP_KIB[header & 7])
    ks = chacha20_stream(k_enc, 0, bytes(12), room)
    p = xor(c, ks)
    tags = tags_for(k_mac, header, n, salt, c, t)
    values = {
        "pw": pw.hex(), "PRK": prk.hex(), "K_enc": k_enc.hex(), "K_mac": k_mac.hex(),
        "keystream": ks.hex(), "P": p.hex(), "C": c.hex(), "tag": tag.hex(),
        "tags": {"0x%02X" % k: v.hex() for k, v in tags.items()},
        "hmac_message": {k: bytes([header, n]) + salt + bytes([k]) + c for k in KINDS},
    }
    hits = [k for k in KINDS if hmac.compare_digest(tags[k], tag)]
    if not hits:
        return ("no_match",), values
    if len(hits) > 1:
        return ("ambiguous",), values
    try:
        return ("opens", hits[0], read_item(hits[0], p)), values
    except Refused as e:
        return ("layout", hits[0], e.rule), values


# ---------------------------------------------------------------- writing


class WriterRefuses(Exception):
    pass


def codex32_item(s, fill):
    """The item bytes of a codex32 string (section 3.1); fill is the value
    that goes into the unused low bits of the last byte."""
    if s != s.lower() and s != s.upper():
        raise WriterRefuses("mixed case")
    low = s.lower()
    if len(s) not in CODEX_LEN or not low.startswith("ms1"):
        raise WriterRefuses("prefix or length")
    if any(ch not in CHARSET for ch in low[3:]):
        raise WriterRefuses("character")
    vals = [CHARSET.index(ch) for ch in low[3:]]
    if ms32_polymod(vals) != MS32_CONST:
        raise WriterRefuses("checksum")
    data = vals[:-13]
    if low[3] not in "023456789":
        raise WriterRefuses("threshold")
    if low[3] == "0" and low[3 + 5] != "s":
        raise WriterRefuses("threshold 0 and an index other than s")
    size = CODEX_LEN[len(s)]
    nbytes = CODEX_SIZE[size][1]
    bits = (size << 1) | (1 if s == s.upper() and s != s.lower() else 0)
    nb = 4
    for v in data:
        bits = (bits << 5) | v
        nb += 5
    spare = nbytes * 8 - nb
    if not 0 <= fill < (1 << spare) and fill != 0:
        raise WriterRefuses("fill")
    bits = (bits << spare) | fill
    return bits.to_bytes(nbytes, "big")


def item_bytes_for(kind, item, fill):
    if kind in ENTROPY or kind == 0x50:
        b = bytes.fromhex(item["hex"])
        want = ENTROPY.get(kind, 32)
        if len(b) != want:
            raise WriterRefuses("length")
        return b
    if kind == 0x30:
        return codex32_item(item["string"], fill)
    text = item["text"].encode("utf-8")
    if not text:
        raise WriterRefuses("empty text")
    return bytes([len(text)]) + text


def seal(passphrase, step, size, kind, item, salt, padding, fill=0):
    if not passphrase:
        raise WriterRefuses("empty passphrase")
    room, t = SIZES[size]
    ib = item_bytes_for(kind, item, fill)
    if len(ib) > room:
        raise WriterRefuses("does not fit")
    if len(padding) != room - len(ib):
        raise WriterRefuses("padding length")
    header = 0xF8 | step
    _, _, k_enc, k_mac = derive(passphrase, salt, STEP_KIB[step])
    c = xor(ib + padding, chacha20_stream(k_enc, 0, bytes(12), room))
    tags = tags_for(k_mac, header, size, salt, c, t)
    if any(tags[k] == tags[kind] for k in KINDS if k != kind):
        raise WriterRefuses("two kinds' tags agree")
    return bytes([header]) + salt + c + tags[kind]


# ---------------------------------------------------------------- PNG and QR


def png_modules(path):
    """Reads the picture with no image library; returns (chunk names, rows of
    modules without the quiet zone, 1 is dark)."""
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, names, idat, ihdr = 8, [], b"", None
    while pos < len(data):
        ln, name = struct.unpack(">I4s", data[pos : pos + 8])
        body = data[pos + 8 : pos + 8 + ln]
        names.append(name.decode())
        if name == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", body)
        elif name == b"IDAT":
            idat += body
        pos += 12 + ln
    w, h, depth, ctype, _, _, interlace = ihdr
    if (depth, ctype, interlace) != (1, 0, 0):
        raise ValueError("not 1-bit greyscale without interlace")
    raw = zlib.decompress(idat)
    stride = (w + 7) // 8
    rows, prev = [], bytes(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)])
        if f == 0:
            pass
        elif f == 1:
            for i in range(1, stride):
                line[i] = (line[i] + line[i - 1]) & 255
        elif f == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                a = line[i - 1] if i else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 255
        elif f == 4:
            for i in range(stride):
                a = line[i - 1] if i else 0
                b, c = prev[i], prev[i - 1] if i else 0
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 255
        else:
            raise ValueError("filter")
        prev = bytes(line)
        rows.append(prev)
    pix = [[(rows[y][x // 8] >> (7 - x % 8)) & 1 for x in range(w)] for y in range(h)]
    return names, pix


def matrix_from_png(path, modules):
    names, pix = png_modules(path)
    scale, quiet = 8, 4
    total = (modules + 2 * quiet) * scale
    if len(pix) != total or len(pix[0]) != total:
        raise ValueError("size %dx%d, expected %d" % (len(pix[0]), len(pix), total))
    rows = []
    for my in range(-quiet, modules + quiet):
        row = ""
        for mx in range(-quiet, modules + quiet):
            cell = {pix[(my + quiet) * scale + dy][(mx + quiet) * scale + dx] for dy in range(scale) for dx in range(scale)}
            if len(cell) != 1:
                raise ValueError("a module is not one colour")
            row += "1" if cell == {0} else "0"  # 0 is black in greyscale
        rows.append(row)
    inner = [r[quiet : quiet + modules] for r in rows[quiet : quiet + modules]]
    quiet_ok = all("1" not in r for r in rows[:quiet] + rows[-quiet:]) and all("1" not in r[:quiet] + r[-quiet:] for r in rows)
    return names, inner, quiet_ok


def qr_checks(data, fails):
    notes = []
    for q in data["qr"]:
        path = os.path.join(HERE, q["png"])
        try:
            names, inner, quiet_ok = matrix_from_png(path, q["modules"])
        except Exception as e:  # noqa: BLE001
            fails.append("qr %s: %s" % (q["png"], e))
            continue
        if inner != q["matrix"]:
            fails.append("qr %s: the picture's modules differ from the matrix in the JSON" % q["png"])
        if not quiet_ok:
            fails.append("qr %s: the quiet zone is not light" % q["png"])
        if sorted(names) != sorted(["IHDR", "IDAT", "IEND"]):
            fails.append("qr %s: chunks %s" % (q["png"], names))
        if hashlib.sha256(open(path, "rb").read()).hexdigest() != q["png_sha256"]:
            fails.append("qr %s: sha256 differs from the JSON" % q["png"])
        want = bytes.fromhex(q["envelope"])
        notes.append(decode_qr(path, want, q["png"], fails))
    return notes


def decode_qr(path, want, label, fails):
    out = []
    try:
        import zxingcpp
        from PIL import Image

        found = zxingcpp.read_barcodes(Image.open(path).convert("L"), formats=zxingcpp.BarcodeFormat.QRCode)
        if len(found) != 1:
            fails.append("qr %s: zxing-cpp finds %d symbols" % (label, len(found)))
        elif bytes(found[0].bytes) != want:
            fails.append("qr %s: zxing-cpp reads other bytes" % label)
        else:
            out.append("zxing-cpp")
    except ImportError as e:
        out.append("zxing-cpp not available (%s)" % e)
    try:
        run = subprocess.run(
            ["zbarimg", "--raw", "-q", "-Sdisable", "-Sqrcode.enable", "-Sbinary", path],
            capture_output=True, timeout=60,
        )
        if run.returncode != 0:
            fails.append("qr %s: zbarimg exits %d" % (label, run.returncode))
        elif run.stdout != want:
            fails.append("qr %s: zbar reads other bytes (%d)" % (label, len(run.stdout)))
        else:
            out.append("zbar")
    except FileNotFoundError:
        out.append("zbarimg not available")
    return "%s: %s" % (label, ", ".join(out))


# ---------------------------------------------------------------- the run

RULE_WORDS = {
    "size": "reserved", "room": "over the", "threshold": "neither 0 nor 2 to 9",
    "threshold-index": "not s", "len": "len is", "utf8": "not UTF-8",
}


def kind_in(text):
    m = re.search(r"kind (0x[0-9A-Fa-f]{2})", text)
    return int(m.group(1), 16) if m else None


def main():
    fails = []
    if not chacha20_self_test():
        print("ChaCha20 does not give RFC 8439 section 2.4.2")
        return 1
    data = json.load(open(os.path.join(HERE, "aqrv.json")))
    cases = data["cases"]
    opened = refused = unrec = sealed = values_n = 0
    for case in cases:
        name, exp = case["name"], case["expect"]
        env = bytes.fromhex(case["envelope"])
        ans, vals = read(env, case.get("passphrase") or case["kef"]["passphrase"])
        out = exp["outcome"]
        if out == "not_recognised":
            unrec += 1
            if ans[0] != "not_recognised":
                fails.append("%s: expected not recognised, got %s" % (name, ans[0]))
            continue
        if out == "does_not_open":
            refused += 1
            if "no kind's tag" in exp["why"]:
                if ans[0] != "no_match":
                    fails.append("%s: expected no tag to match, got %s" % (name, ans))
            else:
                k = kind_in(exp["why"])
                if ans[0] != "layout" or ans[1] != k:
                    fails.append("%s: expected a layout refusal of kind 0x%02X, got %s" % (name, k or 0, ans))
                elif RULE_WORDS[ans[2]] not in exp["reason"]:
                    fails.append("%s: refused by rule %r, the case says %r" % (name, ans[2], exp["reason"]))
        elif out == "opens":
            opened += 1
            if ans[0] != "opens":
                fails.append("%s: expected to open, got %s" % (name, ans))
            else:
                if ans[1] != int(exp["kind"], 16):
                    fails.append("%s: kind 0x%02X, expected %s" % (name, ans[1], exp["kind"]))
                if ans[2] != exp["item"]:
                    fails.append("%s: item %s, expected %s" % (name, ans[2], exp["item"]))
        # the intermediate values the case gives
        for key, want in case.get("values", {}).items():
            if key not in vals:
                continue
            got = vals[key]
            if key == "tags":
                for k, w in want.items():
                    values_n += 1
                    if got.get(k) != w:
                        fails.append("%s: tags[%s] %s, expected %s" % (name, k, got.get(k), w))
            elif key == "hmac_message":
                values_n += 1
                if got[kind_of(case)].hex() != want:
                    fails.append("%s: hmac_message differs" % name)
            else:
                values_n += 1
                if got != want:
                    fails.append("%s: %s %s, expected %s" % (name, key, got, want))
        if "header" in case and case["header"] != "0x%02X" % env[0]:
            fails.append("%s: header" % name)
        if "item_bytes" in case and "P" in vals and ans[0] in ("opens", "layout"):
            ib = case["item_bytes"]
            if case["kind"] == "0x30":
                ib = ib[:-2]  # the last byte's low bits are padding bits
            if not vals["P"].startswith(ib):
                fails.append("%s: P does not start with item_bytes" % name)
        # sealing
        if out == "opens":
            sealed += 1
            fill = int(case["last_byte_fill"], 16) if "last_byte_fill" in case else 0
            try:
                got = seal(case["passphrase"], case["step"], case["size"], int(case["kind"], 16),
                           exp["item"], bytes.fromhex(case["salt"]), bytes.fromhex(case["padding"]), fill)
            except WriterRefuses as e:
                fails.append("%s: the writer refuses: %s" % (name, e))
                continue
            if got != env:
                fails.append("%s: sealed envelope differs" % name)
    # what a writer refuses
    for r in data["writer_refuses"]:
        try:
            codex32_item(r["string"], 0)
            fails.append("writer takes %r (%s)" % (r["string"], r["reason"]))
        except WriterRefuses:
            pass
    for what, call in (
        ("empty passphrase", lambda: seal("", 0, 53, 0x41, {"text": "x"}, bytes(16), bytes(30))),
        ("empty note", lambda: item_bytes_for(0x41, {"text": ""}, 0)),
        ("note over the room at 53", lambda: seal("p", 0, 53, 0x41, {"text": "x" * 32}, bytes(16), b"")),
        ("37 bytes of codex32 at 53", lambda: seal("p", 0, 53, 0x30, {"string": "ms1" + "q" * 71}, bytes(16), b"")),
    ):
        try:
            call()
            fails.append("writer takes: %s" % what)
        except WriterRefuses:
            pass
    qr_notes = [] if "--no-qr" in sys.argv else qr_checks(data, fails)
    print("%d cases: %d open, %d do not open, %d not recognised; %d sealed again; %d values compared; %d disagreements"
          % (len(cases), opened, refused, unrec, sealed, values_n, len(fails)))
    for n in qr_notes:
        print("qr " + n)
    for f in fails:
        print("DISAGREES " + f)
    return 1 if fails else 0


def kind_of(case):
    return int(case["kind"], 16)


if __name__ == "__main__":
    sys.exit(main())
