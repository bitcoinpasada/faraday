#!/usr/bin/env python3
"""Writes the test vectors of the Argon QR Vault (AQRV) (docs/AQRV.md §11).

    python3 -I -B tools/vectors/aqrv/gen.py            # writes aqrv.json, qr-53.png, qr-62.png
    python3 -I -B tools/vectors/aqrv/gen.py --check    # compares, writes nothing

Libraries Faraday's authors did not write: argon2-cffi (Argon2id), Python's
hmac and hashlib (HMAC-SHA-256, HKDF-Expand, SHA-256, SHAKE-256), PyCA
cryptography (ChaCha20, AES-GCM, and an HKDF-Expand to cross-check the
hand-written one), and Nayuki's qrcodegen (tools/reference/qrcodegen) for the
QR code. Nothing here is random: every passphrase, salt, item and padding is
fixed by the script, so a second run writes the same bytes.

Every case is also read back by `read` below, a plain reading of §6 and §9, and
the script stops if any answer differs from the one it writes. That reader is
the script's own check, not an implementation to ship.
"""
import argparse
import hashlib
import hmac
import json
import os
import struct
import sys
import time
import unicodedata
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
sys.path.insert(0, os.path.join(ROOT, "tools", "reference", "qrcodegen"))

from argon2.low_level import Type, hash_secret_raw  # noqa: E402
from cryptography.hazmat.primitives import hashes  # noqa: E402
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms  # noqa: E402
from cryptography.hazmat.primitives.ciphers.aead import AESGCM  # noqa: E402
from cryptography.hazmat.primitives.kdf.hkdf import HKDFExpand  # noqa: E402
from qrcodegen import QrCode, QrSegment  # noqa: E402

# ---------------------------------------------------------------- the format

STEPS = {0: 65536, 1: 131072, 2: 262144, 3: 524288, 4: 1048576}  # KiB
LABEL_ENC = b"argon-qr-vault-1 enc"
LABEL_MAC = b"argon-qr-vault-1 mac"
SIZES = {53: (32, 4), 62: (37, 8)}  # N -> (room r, tag length t)

KINDS = [  # kind, name, item bytes or None
    (0x10, "BIP-39 entropy, 12 words", 16),
    (0x14, "BIP-39 entropy, 15 words", 20),
    (0x18, "BIP-39 entropy, 18 words", 24),
    (0x1C, "BIP-39 entropy, 21 words", 28),
    (0x20, "BIP-39 entropy, 24 words", 32),
    (0x30, "a codex32 string", None),
    (0x40, "a BIP-39 passphrase", None),
    (0x41, "a note", None),
    (0x50, "a raw 32-byte key", 32),
]
KIND_NAME = {k: n for k, n, _ in KINDS}
ENTROPY_LEN = {k: b for k, _, b in KINDS if k in (0x10, 0x14, 0x18, 0x1C, 0x20)}

CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
CODEX_BYTES = {0: 16, 1: 20, 2: 24, 3: 28, 4: 32}
CODEX_C = {0: 26, 1: 32, 2: 39, 3: 45, 4: 52}
THRESHOLDS = {CHARSET.index(ch): ch for ch in "023456789"}

MS32_CONST = 0x10CE0795C2FD1E62A
MS32_GEN = [
    0x19DC500CE73FDE210,
    0x1BFAE00DEF77FE529,
    0x1FBD920FFFE7BEE52,
    0x1739640BDEEE3FDAD,
    0x07729A039CFC75F5A,
]


def ms32_polymod(values):  # BIP 93, copied
    residue = 0x23181B3
    for v in values:
        b = residue >> 60
        residue = (residue & 0x0FFFFFFFFFFFFFFF) << 5 ^ v
        for i in range(5):
            residue ^= MS32_GEN[i] if ((b >> i) & 1) else 0
    return residue


def ms32_checksum(data):  # BIP 93 ms32_create_regular_checksum, copied
    polymod = ms32_polymod(list(data) + [0] * 13) ^ MS32_CONST
    return [(polymod >> 5 * (12 - i)) & 31 for i in range(13)]


def hx(b):
    return bytes(b).hex()


def nfc(text):
    return unicodedata.normalize("NFC", text)


# ------------------------------------------------------------- primitives

_derive_cache = {}


def hkdf_expand(prk, info):
    out = hmac.new(prk, info + b"\x01", hashlib.sha256).digest()  # RFC 5869 §2.3, L = 32
    check = HKDFExpand(hashes.SHA256(), 32, info).derive(prk)
    assert out == check
    return out


def chacha20_stream(key, n):  # RFC 8439 §2.4: counter 0, nonce twelve zero bytes
    enc = Cipher(algorithms.ChaCha20(key, b"\x00" * 16), mode=None).encryptor()
    return enc.update(b"\x00" * n)


def derive(passphrase, salt, step):
    pw = nfc(passphrase).encode("utf-8")
    key = (pw, bytes(salt), step)
    if key not in _derive_cache:
        prk = hash_secret_raw(pw, bytes(salt), time_cost=3, memory_cost=STEPS[step],
                              parallelism=4, hash_len=32, type=Type.ID, version=19)
        k_enc = hkdf_expand(prk, LABEL_ENC)
        k_mac = hkdf_expand(prk, LABEL_MAC)
        _derive_cache[key] = dict(pw=pw, prk=prk, k_enc=k_enc, k_mac=k_mac,
                                  keystream=chacha20_stream(k_enc, 37))
    return _derive_cache[key]


def tag_message(header, n, salt, kind, c):
    return bytes([header, n]) + bytes(salt) + bytes([kind]) + bytes(c)


def tag(k_mac, header, n, salt, kind, c, t):
    return hmac.new(k_mac, tag_message(header, n, salt, kind, c), hashlib.sha256).digest()[:t]


def xor(a, b):
    return bytes(x ^ y for x, y in zip(a, b))


# ------------------------------------------------------------- the reader


def parse_item(kind, p, r):
    """Reads P by kind's layout. Returns (item dict, None) or (None, why)."""
    if kind in ENTROPY_LEN:
        return {"type": "entropy", "hex": hx(p[:ENTROPY_LEN[kind]])}, None
    if kind == 0x50:
        return {"type": "raw_key", "hex": hx(p[:32])}, None
    if kind in (0x40, 0x41):
        n = p[0]
        if n < 1 or n > r - 1:
            return None, "len %d is outside 1 to %d" % (n, r - 1)
        try:
            text = bytes(p[1:1 + n]).decode("utf-8")
        except UnicodeDecodeError:
            return None, "the text is not valid UTF-8"
        return {"type": "passphrase" if kind == 0x40 else "note", "text": text}, None
    assert kind == 0x30
    size = p[0] >> 5
    if size > 4:
        return None, "codex32 size %d is reserved" % size
    c = CODEX_C[size]
    nbits = 34 + 5 * c
    nbytes = (nbits + 7) // 8
    if nbytes > r:
        return None, "codex32 size %d needs %d bytes, the room is %d" % (size, nbytes, r)
    bits = int.from_bytes(p[:nbytes], "big") >> (nbytes * 8 - nbits)
    upper = (bits >> (nbits - 4)) & 1
    pos = nbits - 4
    pos -= 5
    threshold = (bits >> pos) & 31
    ident = []
    for _ in range(4):
        pos -= 5
        ident.append((bits >> pos) & 31)
    pos -= 5
    index = (bits >> pos) & 31
    payload = []
    for _ in range(c):
        pos -= 5
        payload.append((bits >> pos) & 31)
    assert pos == 0
    if threshold not in THRESHOLDS and threshold != CHARSET.index("0"):
        return None, "codex32 threshold value %d is not 0 or 2 to 9" % threshold
    if threshold == CHARSET.index("0") and index != CHARSET.index("s"):
        return None, "codex32 threshold 0 with an index other than s"
    data = [threshold] + ident + [index] + payload
    s = "ms1" + "".join(CHARSET[v] for v in data + ms32_checksum(data))
    if upper:
        s = s.upper()
    return {"type": "codex32", "string": s, "is_secret": index == CHARSET.index("s")}, None


def read(env, passphrase):
    """§9 then §6 reading. Returns a dict with outcome and, when it opens, kind and item."""
    n = len(env)
    if n not in SIZES or not (0xF8 <= env[0] <= 0xFC):
        return {"outcome": "not_recognised"}
    r, t = SIZES[n]
    step = env[0] & 7
    salt, c, stored = env[1:17], env[17:17 + r], env[17 + r:]
    d = derive(passphrase, salt, step)
    hits = [k for k, _, _ in KINDS if tag(d["k_mac"], env[0], n, salt, k, c, t) == stored]
    if not hits:
        return {"outcome": "does_not_open", "why": "no_tag_matches"}
    assert len(hits) == 1
    p = xor(c, d["keystream"][:r])
    item, why = parse_item(hits[0], p, r)
    if item is None:
        return {"outcome": "does_not_open", "why": "format", "kind": hits[0], "detail": why}
    return {"outcome": "opens", "kind": hits[0], "item": item}


# ------------------------------------------------------------ case building


def shake(label, n):
    return hashlib.shake_256(b"AQRV vector " + label.encode()).digest(n)


def salt_of(name):
    return hashlib.sha256(b"AQRV vector salt " + name.encode()).digest()[:16]


def kh(k):
    return "0x%02X" % k


def make_p(name, item, r, unused_mask=0):
    """P = item bytes (unused low bits of the last byte filled), then padding."""
    stream = shake("padding " + name, r + 1)
    fill = stream[0] & unused_mask
    body = bytearray(item)
    if unused_mask:
        assert body[-1] & unused_mask == 0
        body[-1] |= fill
    padding = stream[1:1 + r - len(item)]
    return bytes(body) + padding, fill, padding


def seal(name, group, n, step, passphrase, kind, p, item_hex, expect_item, extra=None, salt=None):
    r, t = SIZES[n]
    assert len(p) == r
    salt = salt or salt_of(name)
    header = 0xF8 | step
    d = derive(passphrase, salt, step)
    c = xor(p, d["keystream"][:r])
    tags = {kh(k): hx(tag(d["k_mac"], header, n, salt, k, c, t)) for k, _, _ in KINDS}
    assert len(set(tags.values())) == len(tags), "two kinds share a tag: choose another salt"
    env = bytes([header]) + salt + c + bytes.fromhex(tags[kh(kind)])
    assert len(env) == n
    case = {
        "name": name,
        "group": group,
        "size": n,
        "step": step,
        "memory_kib": STEPS[step],
        "passphrase": passphrase,
        "kind": kh(kind),
        "kind_name": KIND_NAME[kind],
        "item_bytes": item_hex,
        "salt": hx(salt),
        "header": "0x%02X" % header,
        "values": {
            "pw": hx(d["pw"]),
            "PRK": hx(d["prk"]),
            "K_enc": hx(d["k_enc"]),
            "K_mac": hx(d["k_mac"]),
            "keystream": hx(d["keystream"][:r]),
            "P": hx(p),
            "C": hx(c),
            "hmac_message": hx(tag_message(header, n, salt, kind, c)),
            "tags": tags,
            "tag": tags[kh(kind)],
        },
        "envelope": hx(env),
    }
    if extra:
        case.update(extra)
    return case, env


def valid_case(name, group, n, step, passphrase, kind, item, unused_mask=0, expect_item=None,
               extra=None, salt=None, pad_name=None):
    r, _ = SIZES[n]
    p, fill, padding = make_p(pad_name or name, item, r, unused_mask)
    extra = dict(extra or {})
    extra["padding"] = hx(padding)
    if unused_mask:
        extra["last_byte_fill"] = "0x%02X" % fill
    case, env = seal(name, group, n, step, passphrase, kind, p, hx(item), expect_item, extra, salt)
    got = read(env, passphrase)
    assert got["outcome"] == "opens" and got["kind"] == kind, (name, got)
    assert got["item"] == expect_item, (name, got["item"], expect_item)
    case["expect"] = {"outcome": "opens", "kind": kh(kind), "item": expect_item}
    return case, env


def fail_case(name, group, env, passphrase, outcome, why, source=None, note=""):
    got = read(env, passphrase)
    assert got["outcome"] == outcome, (name, got)
    if outcome == "does_not_open":
        assert got["why"] == "no_tag_matches", (name, got)
    expect = {"outcome": outcome}
    if outcome == "does_not_open":
        expect["why"] = "no kind's tag matches"
    expect["reason"] = why
    case = {"name": name, "group": group, "size": len(env), "passphrase": passphrase,
            "envelope": hx(env), "expect": expect}
    if source:
        case["made_from"] = source
    if note:
        case["note"] = note
    return case


def crafted_case(name, kind, n, p, passphrase, detail, step=0):
    """A right tag over a P the format refuses: made with the key."""
    case, env = seal(name, "fail-right-tag", n, step, passphrase, kind, p, hx(p), None)
    got = read(env, passphrase)
    assert got["outcome"] == "does_not_open" and got["why"] == "format" and got["kind"] == kind, (name, got)
    case["expect"] = {"outcome": "does_not_open", "why": "the tag of kind %s matches and the item fails its layout" % kh(kind),
                      "reason": detail}
    return case


# ------------------------------------------------------------ codex32 input


def codex_item(s):
    """Item bytes, unused-bit mask and the five fields of a codex32 string."""
    upper = s.isupper()
    assert s.islower() or upper
    s = s.lower()
    assert s.startswith("ms1")
    body = s[3:-13]
    c = len(body) - 6
    size = [k for k, v in CODEX_C.items() if v == c][0]
    vals = [CHARSET.index(ch) for ch in body]
    threshold, ident, index, payload = vals[0], vals[1:5], vals[5], vals[6:]
    bits = (size << 1) | int(upper)
    for v in [threshold] + ident + [index] + payload:
        bits = (bits << 5) | v
    nbits = 4 + 5 * (1 + 4 + 1 + c)
    nbytes = (nbits + 7) // 8
    pad = nbytes * 8 - nbits
    return (bits << pad).to_bytes(nbytes, "big"), (1 << pad) - 1, size, nbytes


def codex_p(head_size, upper, threshold, ident, index, payload):
    bits = (head_size << 1) | upper
    for v in [threshold] + ident + [index] + payload:
        bits = (bits << 5) | v
    return bits


# -------------------------------------------------------------------- QR


def qr_of(env):
    n = len(env)
    version, ecc = (3, QrCode.Ecc.LOW) if n == 53 else (4, QrCode.Ecc.MEDIUM)
    qr = QrCode.encode_segments([QrSegment.make_bytes(env)], ecc, version, version, -1, False)
    return qr


def png_of(qr, scale=8, quiet=4):
    """A 1-bit greyscale PNG with the three required chunks and nothing else."""
    size = qr.get_size()
    side = (size + 2 * quiet) * scale
    rows = bytearray()
    for y in range(side):
        my = y // scale - quiet
        row = bytearray((side + 7) // 8)
        for x in range(side):
            mx = x // scale - quiet
            dark = 0 <= mx < size and 0 <= my < size and qr.get_module(mx, my)
            if not dark:
                row[x // 8] |= 0x80 >> (x % 8)
        rows += b"\x00" + row

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", side, side, 1, 0, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(bytes(rows), 9)) + chunk(b"IEND", b""))


# ------------------------------------------------------------------ cases

PASS = "aqrv vector passphrase"
UNI = "café ✨ Ångström"
UNI_NFD = unicodedata.normalize("NFD", UNI)
assert UNI != UNI_NFD and nfc(UNI_NFD) == UNI


def build():
    cases = []

    def add(pair):
        cases.append(pair[0])
        return pair[1]

    # Steps: a 24-word seed at every step, both sizes.
    base = {}
    for n in (53, 62):
        for step in range(5):
            nm = "step-%d-24w-%d" % (step, n)
            ent = shake("entropy " + nm, 32)
            env = add(valid_case(nm, "step", n, step, PASS, 0x20, ent,
                                 expect_item={"type": "entropy", "hex": hx(ent)}))
            if step == 0:
                base[n] = (env, PASS, nm)

    # Every kind. Entropy of all five lengths, both sizes (the 24-word one is the step 0 case).
    for n in (53, 62):
        for kind, ln in sorted(ENTROPY_LEN.items()):
            if kind == 0x20:
                continue
            nm = "entropy-%dw-%d" % (ln * 3 // 4, n)
            ent = shake("entropy " + nm, ln)
            add(valid_case(nm, "kind", n, 0, PASS, kind, ent,
                           expect_item={"type": "entropy", "hex": hx(ent)}))
        nm = "raw-key-%d" % n
        key = shake("key " + nm, 32)
        add(valid_case(nm, "kind", n, 0, PASS, 0x50, key, expect_item={"type": "raw_key", "hex": hx(key)}))
        r = SIZES[n][0]
        longest = r - 1
        for kind, label, tname in ((0x40, "passphrase", "passphrase"), (0x41, "note", "note")):
            for how, text in (
                ("shortest", "x"),
                ("longest", ("0123456789abcdefghijklmnopqrstuvwxyz"[:longest])),
                ("non-ascii", "Zoë café 日本語 ✓"),
            ):
                b = text.encode("utf-8")
                assert 1 <= len(b) <= longest
                nm = "%s-%s-%d" % (label, how, n)
                add(valid_case(nm, "kind", n, 0, PASS, kind, bytes([len(b)]) + b,
                               expect_item={"type": tname, "text": text},
                               extra={"text_bytes": len(b)}))
        # A longest text that is multi-byte, ending exactly at the room.
        text = "é" * (longest // 2) + ("a" if longest % 2 else "")
        b = text.encode("utf-8")
        assert len(b) == longest
        nm = "note-longest-multibyte-%d" % n
        add(valid_case(nm, "kind", n, 0, PASS, 0x41, bytes([len(b)]) + b,
                       expect_item={"type": "note", "text": text}, extra={"text_bytes": len(b)}))

    # All-zero and all-one seeds.
    for n, ln, kind in ((53, 16, 0x10), (62, 16, 0x10), (53, 32, 0x20), (62, 32, 0x20)):
        for label, byte in (("zero", 0x00), ("ones", 0xFF)):
            ent = bytes([byte]) * ln
            nm = "%s-seed-%dw-%d" % (label, ln * 3 // 4, n)
            add(valid_case(nm, "seed", n, 0, PASS, kind, ent,
                           expect_item={"type": "entropy", "hex": hx(ent)}))

    # A passphrase that is not ASCII, in NFC and in NFD: one envelope.
    for n in (53, 62):
        ent = shake("entropy unicode-%d" % n, 24)
        salt = salt_of("unicode-%d" % n)
        envs = []
        for form, pp in (("nfc", UNI), ("nfd", UNI_NFD)):
            nm = "unicode-%s-%d" % (form, n)
            envs.append(add(valid_case(nm, "unicode", n, 0, pp, 0x18, ent,
                                       expect_item={"type": "entropy", "hex": hx(ent)}, salt=salt,
                                       pad_name="unicode-%d" % n, extra={"passphrase_utf8": hx(pp.encode())})))
        assert envs[0] == envs[1]
    # A compatibility character is kept: "fine" and "ﬁne" are two passphrases.
    n = 53
    nm = "unicode-compat-fine-53"
    ent = shake("entropy " + nm, 16)
    env = add(valid_case(nm, "unicode", n, 0, "fine", 0x10, ent,
                         expect_item={"type": "entropy", "hex": hx(ent)}))
    cases.append(fail_case("unicode-compat-ligature-53", "fail", env, "ﬁne", "does_not_open",
                           "NFC keeps the ligature, so it is a different passphrase from fine",
                           source=nm))

    # codex32 strings from BIP 93's vectors.
    bip = json.load(open(os.path.join(ROOT, "tools", "vectors", "bip93", "vectors.json")))
    strings = []

    def take(group, s):
        if len(s) <= 93 and s not in [x[1] for x in strings]:
            strings.append((group, s))

    for e in bip["secrets"]:
        if e["vector"] != "5":  # 64 bytes: does not fit
            take("vector %s" % e["vector"], e["string"])
    for e in bip["sets"]:
        for s in e["shares"]:
            take("vector %s" % e["vector"], s)
        for s in e["derived"].values():
            take("vector %s" % e["vector"], s)
        take("vector %s" % e["vector"], e["secret"])
    for e in bip["alternates"]:
        for s in e["strings"]:
            take("vector %s" % e["vector"], s)
    firsts = set()
    for i, (grp, s) in enumerate(strings):
        item, mask, size, nbytes = codex_item(s)
        sizes = [53, 62] if nbytes <= 32 else [62]
        if nbytes <= 32 and grp not in firsts:
            firsts.add(grp)
        else:
            sizes = sizes[:1]
        for n in sizes:
            nm = "codex32-%02d-%s-%d" % (i, grp.replace(" ", ""), n)
            add(valid_case(nm, "codex32", n, 0, PASS, 0x30, item, unused_mask=mask,
                           expect_item={"type": "codex32", "string": s,
                                        "is_secret": s.lower()[3 + 5] == "s"},
                           extra={"source": "BIP 93 " + grp, "codex32_size": size,
                                  "item_length": nbytes}))
    writer_refuses = [
        {"string": [e for e in bip["secrets"] if e["vector"] == "5"][0]["string"],
         "reason": "BIP 93 vector 5, a 64-byte seed: 103 characters of payload, over the largest form"},
        {"string": bip["invalid"]["strings"][0] if isinstance(bip["invalid"], dict) else bip["invalid"][0]["strings"][0],
         "reason": "BIP 93's first invalid string: the checksum is wrong"},
    ]

    # Must fail: the base 24-word envelopes changed one way at a time.
    for n in (53, 62):
        env, pp, src = base[n]
        r, t = SIZES[n]

        def flip(i, bit=0x01):
            b = bytearray(env)
            b[i] ^= bit
            return bytes(b)

        cases.append(fail_case("fail-wrong-passphrase-%d" % n, "fail", env, PASS + "!", "does_not_open",
                               "a wrong passphrase", src))
        cases.append(fail_case("fail-header-step-bit-%d" % n, "fail", flip(0), PASS, "does_not_open",
                               "the step changed from 0 to 1: a different memory, a different key and tag", src))
        cases.append(fail_case("fail-header-marker-bit-%d" % n, "fail", flip(0, 0x80), PASS, "not_recognised",
                               "a bit of the marker changed: the first byte is 0x78", src))
        cases.append(fail_case("fail-salt-bit-%d" % n, "fail", flip(1), PASS, "does_not_open",
                               "a bit of the salt changed", src))
        cases.append(fail_case("fail-c-bit-%d" % n, "fail", flip(17), PASS, "does_not_open",
                               "a bit of the encrypted item changed", src))
        cases.append(fail_case("fail-tag-bit-%d" % n, "fail", flip(n - 1), PASS, "does_not_open",
                               "a bit of the tag changed", src))
        for h in (0xFD, 0xFE, 0xFF):
            cases.append(fail_case("fail-reserved-step-%d-%d" % (h & 7, n), "fail", bytes([h]) + env[1:], PASS,
                                   "not_recognised", "step %d is reserved: the first byte is 0x%02X" % (h & 7, h), src))
        cases.append(fail_case("fail-marker-11110-%d" % n, "fail", bytes([0xF0]) + env[1:], PASS, "not_recognised",
                               "marker 0b11110, which a later version would use: the first byte is 0xF0", src))
    e53, e62 = base[53][0], base[62][0]
    cases.append(fail_case("fail-length-52", "fail", e53[:52], PASS, "not_recognised",
                           "52 bytes: the 53-byte envelope with its last byte removed", base[53][2]))
    cases.append(fail_case("fail-length-54", "fail", e53 + b"\x00", PASS, "not_recognised",
                           "54 bytes: the 53-byte envelope with a zero byte added", base[53][2]))
    cases.append(fail_case("fail-length-61", "fail", e62[:61], PASS, "not_recognised",
                           "61 bytes: the 62-byte envelope with its last byte removed", base[62][2]))
    cases.append(fail_case("fail-length-63", "fail", e62 + b"\x00", PASS, "not_recognised",
                           "63 bytes: the 62-byte envelope with a zero byte added", base[62][2]))
    cases.append(fail_case("fail-62-cut-to-53", "fail", e62[:53], PASS, "does_not_open",
                           "the first 53 bytes of a 62-byte envelope: the length byte N is 53, and the bytes read as C and tag are not the ones sealed",
                           base[62][2]))
    # A tag made with the other size's length byte N, on the right fields.
    for n, other in ((62, 53), (53, 62)):
        env, pp, src = base[n]
        r, t = SIZES[n]
        d = derive(pp, env[1:17], 0)
        wrong = tag(d["k_mac"], env[0], other, env[1:17], 0x20, env[17:17 + r], t)
        mutated = env[:17 + r] + wrong
        cases.append(fail_case("fail-tag-with-n-%d-on-%d" % (other, n), "fail", mutated, pp, "does_not_open",
                               "the tag is HMAC over the message with N = %d instead of %d, cut to %d bytes: a tag made at one size does not open the other" % (other, n, t),
                               src))

    # Must fail with the right tag: made with the key.
    for n in (53, 62):
        r, _ = SIZES[n]
        pad = lambda label, head: head + shake("padding crafted-%s-%d" % (label, n), r - len(head))
        # len of 0 and one past the room (len = room is one past the largest, room - 1).
        cases.append(crafted_case("tag-ok-note-len-0-%d" % n, 0x41, n, pad("len0", b"\x00"), PASS, "len is 0"))
        cases.append(crafted_case("tag-ok-passphrase-len-0-%d" % n, 0x40, n, pad("plen0", b"\x00"), PASS, "len is 0"))
        cases.append(crafted_case("tag-ok-note-len-past-room-%d" % n, 0x41, n, pad("lenr", bytes([r])), PASS,
                                  "len is %d, one past the longest allowed, room - 1 = %d" % (r, r - 1)))
        cases.append(crafted_case("tag-ok-passphrase-len-past-room-%d" % n, 0x40, n, pad("plenr", bytes([r])), PASS,
                                  "len is %d, one past the longest allowed, room - 1 = %d" % (r, r - 1)))
        cases.append(crafted_case("tag-ok-note-not-utf8-%d" % n, 0x41, n, pad("utf8", b"\x03\xff\xfe\xfd"), PASS,
                                  "the 3 bytes of text 0xFF 0xFE 0xFD are not UTF-8"))
        cases.append(crafted_case("tag-ok-passphrase-truncated-utf8-%d" % n, 0x40, n, pad("utf8b", b"\x02\xc3\x28"), PASS,
                                  "the 2 bytes of text 0xC3 0x28 are not UTF-8"))
        ident = [CHARSET.index(ch) for ch in "test"]
        payload = [CHARSET.index("x")] * 26
        s_idx, a_idx = CHARSET.index("s"), CHARSET.index("a")

        def cx(size, threshold, index):
            nbits = 4 + 5 * (1 + 4 + 1 + 26)
            v = codex_p(size, 0, threshold, ident, index, payload)
            nb = (nbits + 7) // 8
            head = (v << (nb * 8 - nbits)).to_bytes(nb, "big")
            return head + shake("padding crafted-cx-%d-%d-%d-%d" % (size, threshold, index, n), r - nb)

        for size in (5, 6, 7):
            cases.append(crafted_case("tag-ok-codex32-size-%d-%d" % (size, n), 0x30, n, cx(size, 15, s_idx), PASS,
                                      "codex32 size %d is reserved" % size))
        if n == 53:
            for size, ln in ((3, 33), (4, 37)):
                p = bytes([(size << 5) | 0x05]) + shake("padding crafted-sz%d" % size, 31)
                cases.append(crafted_case("tag-ok-codex32-size-%d-%d" % (size, n), 0x30, n, p, PASS,
                                          "codex32 size %d needs %d bytes, over the 32-byte room" % (size, ln)))
        cases.append(crafted_case("tag-ok-codex32-threshold-1-%d" % n, 0x30, n, cx(0, 1, s_idx), PASS,
                                  "the threshold field holds the value 1 (the character p), which is neither 0 nor 2 to 9"))
        cases.append(crafted_case("tag-ok-codex32-threshold-0-index-a-%d" % n, 0x30, n, cx(0, CHARSET.index("0"), a_idx), PASS,
                                  "threshold 0 with the index a, not s"))
        # The same string with threshold 0 and index s is valid: it is vector 1's shape.

    # Must not be recognised: KEF version 20 envelopes, built from docs/KEF.md.
    kef = []
    for nm, id_len in (("kef-v20-empty-id-53", 0), ("kef-v20-9-byte-id-62", 9)):
        kid = shake("kef id " + nm, id_len) if id_len else b""
        kpass = "kef vector passphrase"
        count_field = 10  # 10 * 10,000 = 100,000 iterations
        key = hashlib.pbkdf2_hmac("sha256", kpass.encode(), kid, count_field * 10000, 32)
        iv = shake("kef iv " + nm, 12)
        plain = shake("kef plaintext " + nm, 32)
        ct_tag = AESGCM(key).encrypt(iv, plain, None)
        payload = iv + ct_tag[:32] + ct_tag[32:36]
        env = bytes([id_len]) + kid + bytes([20]) + count_field.to_bytes(3, "big") + payload
        assert len(env) == (53 if id_len == 0 else 62)
        kef.append({
            "name": nm, "group": "not-recognised", "size": len(env), "envelope": hx(env),
            "expect": {"outcome": "not_recognised",
                       "reason": "a KEF version 20 envelope: its first byte is the id length, 0x%02X, below 0xF8" % id_len},
            "kef": {"id": hx(kid), "version": 20, "iterations_field": count_field, "iterations": count_field * 10000,
                    "passphrase": kpass, "plaintext": hx(plain), "iv": hx(iv), "tag_bytes": 4},
        })
        assert read(env, "x")["outcome"] == "not_recognised"
    cases += kef

    # The QR code of one vector at each size.
    qrs = []
    pngs = {}
    for n in (53, 62):
        env, pp, nm = base[n]
        qr = qr_of(env)
        size = qr.get_size()
        pngs["qr-%d.png" % n] = png_of(qr)
        qrs.append({
            "case": nm, "envelope": hx(env), "bytes": n,
            "version": qr.get_version(), "modules": size, "level": "L" if n == 53 else "M",
            "mask": qr.get_mask(), "segment": "one byte-mode segment, no ECI",
            "png": "qr-%d.png" % n, "png_sha256": hashlib.sha256(pngs["qr-%d.png" % n]).hexdigest(),
            "matrix": ["".join("1" if qr.get_module(x, y) else "0" for x in range(size)) for y in range(size)],
            "matrix_note": "one string per row, top to bottom, 1 is a dark module, no quiet zone",
        })
        assert size == (29 if n == 53 else 33)

    doc = {
        "about": "Test vectors for the Argon QR Vault (AQRV), docs/AQRV.md section 11. See README.md. Written by gen.py.",
        "parameters": {
            "argon2id": {"passes": 3, "lanes": 4, "output_bytes": 32, "version": "0x13"},
            "steps": [{"step": s, "header": "0x%02X" % (0xF8 | s), "memory_kib": m} for s, m in STEPS.items()],
            "labels": {"enc": LABEL_ENC.decode(), "mac": LABEL_MAC.decode()},
            "sizes": [{"bytes": n, "room": r, "tag_bytes": t} for n, (r, t) in SIZES.items()],
            "kinds": [{"kind": kh(k), "name": nme} for k, nme, _ in KINDS],
            "outcomes": {
                "opens": "recognised, one kind's tag matches, the item reads: expect.kind and expect.item are what a reader shows",
                "does_not_open": "recognised, asks for the passphrase, and reports that the code does not open",
                "not_recognised": "not treated as AQRV: no passphrase is asked for",
            },
        },
        "cases": cases,
        "writer_refuses": writer_refuses,
        "qr": qrs,
    }
    text = json.dumps(doc, indent=1, ensure_ascii=True) + "\n"
    out = {"aqrv.json": text.encode()}
    out.update(pngs)
    return out, len(cases)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="compare with the files, write nothing")
    args = ap.parse_args()
    t0 = time.time()
    files, n = build()
    bad = 0
    for name, data in files.items():
        path = os.path.join(HERE, name)
        if args.check:
            have = open(path, "rb").read() if os.path.exists(path) else None
            if have != data:
                print("differs: " + name)
                bad += 1
        else:
            with open(path, "wb") as f:
                f.write(data)
    print("%d cases, %d files, %.1f s%s" % (n, len(files), time.time() - t0, ", all identical" if args.check and not bad else ""))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
