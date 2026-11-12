#!/usr/bin/env python3
"""Differential test of the Argon QR Vault's two readers and writers.

It makes valid codes with check.py's `seal` (the lowest memory step, every
kind at every size, random items, passphrases, salts and padding from a seeded
generator), makes corrupted ones from them, and gives each to check.py (`read`,
`seal`, the recognition of docs/AQRV.md section 9) and to the Rust library
through `aqrv-cli`'s example `diff_batch`, which reads JSON lines and runs the
library as it is, Argon2id included. It compares what each says: not
recognised, or the header, then no tag matches, a layout refusal (the kind and
the rule) or the item; and, for a writer case, the envelope or the refusal.

    python3 -I -B tools/vectors/aqrv/diff.py --seed 1 --rounds 3

Prints the seed, the cases run per kind and per corruption, and every
disagreement; exits 1 on any. Written by Faraday's authors on both sides, so
it is robustness testing and not the outside implementation docs/AQRV.md
section 11 asks for.
"""

import argparse
import collections
import importlib.util
import json
import multiprocessing
import os
import random
import subprocess
import sys
import tempfile
import time
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("aqrv_check", os.path.join(HERE, "check.py"))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)

KIND_NAME = {
    0x10: "words12", 0x14: "words15", 0x18: "words18", 0x1C: "words21", 0x20: "words24",
    0x30: "codex32", 0x40: "passphrase", 0x41: "note", 0x50: "raw_key",
}
TABLE = check.QR_ALPHABET
RANGES = [
    (0x20, 0x7E), (0xA0, 0x24F), (0x300, 0x36F), (0x370, 0x3FF), (0x400, 0x4FF),
    (0x900, 0x97F), (0x1100, 0x11FF), (0xAC00, 0xD7A3), (0x3040, 0x30FF),
    (0x4E00, 0x4FFF), (0xFB00, 0xFB4F), (0xFF00, 0xFFEF), (0x1F300, 0x1F64F),
    (0x2100, 0x214F), (0x1E00, 0x1EFF), (0x212A, 0x212B),
]
SPECIAL = ["é", "ﬁ", "Å", "가", "क़", "ạ̀", "\t", " ", "\n", "é", "\u0000"]


def rand_text(rng, max_bytes, nbytes=None):
    """A string of at most max_bytes of UTF-8 (exactly nbytes if it can be)."""
    want = nbytes if nbytes is not None else rng.choice([1, max_bytes, rng.randint(1, max_bytes)])
    out, used = [], 0
    style = rng.choice(["ascii", "mixed", "mixed", "special"])
    while used < want:
        if style == "ascii" or (style == "special" and rng.random() < 0.3):
            ch = chr(rng.randint(0x20, 0x7E))
        elif style == "special":
            ch = rng.choice(SPECIAL)
        else:
            lo, hi = rng.choice(RANGES)
            ch = chr(rng.randint(lo, hi))
        n = len(ch.encode())
        if used + n > want:
            if nbytes is None:
                break
            ch, n = "x", 1
            if used + n > want:
                break
        out.append(ch)
        used += n
    return "".join(out) or "x"


def rand_pass(rng):
    s = rand_text(rng, rng.choice([4, 12, 40, 100]))
    return s


# ------------------------------------------------------------------ items


def codex_string(rng, size, upper=False, threshold=None, index=None):
    c = check.CODEX_SIZE[size][0]
    t = threshold if threshold is not None else rng.choice("023456789")
    if index is None:
        index = "s" if t == "0" else rng.choice(check.CHARSET)
    ch = lambda n: "".join(rng.choice(check.CHARSET) for _ in range(n))
    body = t + ch(4) + index + ch(c)
    vals = [check.CHARSET.index(x) for x in body]
    s = "ms1" + body + "".join(check.CHARSET[v] for v in check.ms32_checksum(vals))
    return s.upper() if upper else s


def codex_spare(size):
    c, nbytes = check.CODEX_SIZE[size]
    return nbytes * 8 - 4 - 5 * (6 + c)


def make_item(rng, size, kind):
    """(item, fill): a random item of the kind that fits the size."""
    room = check.SIZES[size][0]
    if kind in check.ENTROPY:
        return {"type": "entropy", "hex": rng.randbytes(check.ENTROPY[kind]).hex()}, 0
    if kind == 0x50:
        return {"type": "raw_key", "hex": rng.randbytes(32).hex()}, 0
    if kind == 0x30:
        fits = [s for s, (_, n) in check.CODEX_SIZE.items() if n <= room]
        cs = rng.choice(fits)
        return {"type": "codex32", "string": codex_string(rng, cs, rng.random() < 0.3)}, rng.getrandbits(codex_spare(cs)) if codex_spare(cs) else 0
    typ = "passphrase" if kind == 0x40 else "note"
    return {"type": typ, "text": rand_text(rng, room - 1)}, 0


def kind_of_item(item):
    t = item["type"]
    if t == "entropy":
        n = len(item["hex"]) // 2
        for k, v in check.ENTROPY.items():
            if v == n:
                return k
        return None
    return {"raw_key": 0x50, "codex32": 0x30, "passphrase": 0x40, "note": 0x41}[t]


def seal_op(rng, size, item, fill, pw, step=0, salt=None, padding=None, **tag):
    """A writer case: the inputs both sides are given, the draws included."""
    room = check.SIZES[size][0]
    salt = salt if salt is not None else rng.randbytes(check.SALT[size])
    if padding is None:
        k = kind_of_item(item)
        try:
            n = len(check.item_bytes_for(k, item, 0)) if k is not None else room
        except check.WriterRefuses:
            n = room
        padding = rng.randbytes(max(0, room - n))
    op = {"op": "seal", "size": size, "step": step, "passphrase": pw, "fill": fill,
          "salt": salt.hex(), "padding": padding.hex()}
    op.update(item)
    op.update(tag)
    return op


# ------------------------------------------------------------- recognition


def py_recognise(mode, payload):
    """docs/AQRV.md section 9 and 9.1, on the payload's bytes."""
    if mode == "file":
        return payload if check.recognised(payload) else None
    if len(payload) in (53, 62) and 0xF8 <= payload[0] <= 0xFC:
        return payload
    try:
        s = payload.decode("utf-8")
    except UnicodeDecodeError:
        return None
    env = check.text_to_envelope(s)
    return env if env is not None and check.recognised(env) else None


def render(env, mode):
    """An envelope as the payload a code of that form carries."""
    if mode == "qr" and len(env) == 32:
        return check.envelope_to_text(env).encode()
    return bytes(env)


def read_op(mode, payload, pw, **tag):
    op = {"op": "read", "mode": mode, "payload": payload.hex(), "passphrase": pw}
    op.update(tag)
    return op


# -------------------------------------------------------------- the python side


def py_run(op):
    try:
        if op["op"] == "read":
            return py_read(op)
        return py_seal(op)
    except Exception as e:  # a crash is an outcome that differs
        return {"py_error": repr(e)}


def py_read(op):
    env = py_recognise(op["mode"], bytes.fromhex(op["payload"]))
    if env is None:
        return {"recognised": False}
    r = {"recognised": True, "env": env.hex(), "size": len(env), "step": env[0] & 7}
    ans, _ = check.read(env, op["passphrase"])
    r["answer"] = {"no_match": "no_match", "ambiguous": "ambiguous", "layout": "layout", "opens": "opens"}[ans[0]]
    if ans[0] == "layout":
        r["kind"], r["rule"] = ans[1], ans[2]
    if ans[0] == "opens":
        r["item"] = ans[2]
    return r


def py_seal(op):
    item = {k: op[k] for k in ("type", "hex", "text", "string") if k in op}
    kind = kind_of_item(item)
    if kind is None:
        return {"refused": "entropy length"}
    try:
        env = check.seal(op["passphrase"], op["step"], op["size"], kind, item,
                         bytes.fromhex(op["salt"]), bytes.fromhex(op["padding"]), op["fill"])
    except check.WriterRefuses as e:
        return {"refused": str(e)}
    return {"envelope": env.hex()}


def same(a, b):
    """Whether the Python and the Rust outcome agree."""
    if "refused" in a or "refused" in b:
        return "refused" in a and "refused" in b
    return a == b


# ----------------------------------------------------------------- the cases


def forge(pw, step, size, kind, p, salt):
    """An envelope with the right tag of `kind` over the plaintext p."""
    room, t = check.SIZES[size]
    header = 0xF8 | step
    _, _, k_enc, k_mac = check.derive(pw, salt, check.STEP_KIB[step])
    c = check.xor(p, check.chacha20_stream(k_enc, 0, bytes(12), room))
    return bytes([header]) + salt + c + check.tags_for(k_mac, header, size, salt, c, t)[kind]


def forged_plain(rng, size, kind):
    room = check.SIZES[size][0]
    if kind in (0x40, 0x41):
        n = rng.choice([0, 1, 2, room - 2, room - 1, room, room + 1 if room < 255 else 255, 255, rng.randint(0, 255)])
        n = min(n, 255)
        body = rng.choice(["utf8", "utf8", "raw", "bad", "cut"])
        text = rand_text(rng, max(1, min(n, room - 1)), None).encode() if body != "raw" else rng.randbytes(room)
        if body == "bad":
            text = bytearray(text)
            text[rng.randrange(len(text))] = rng.choice([0xFF, 0xC0, 0x80, 0xED, 0xF8])
            text = bytes(text)
        if body == "cut":
            text = ("é" * room).encode()  # a two-byte character cut at n
        b = bytes([n]) + text + rng.randbytes(room)
        return b[:room]
    if kind == 0x30:
        size_field = rng.randint(0, 7)
        if size_field > 4:
            b = bytearray(rng.randbytes(room))
            b[0] = (b[0] & 0x1F) | (size_field << 5)
            return bytes(b)
        c, nbytes = check.CODEX_SIZE[size_field]
        t = rng.choice([15, 15, 5, 7, 2, 0, 1, 31, rng.randrange(32)])  # 0 is q; 15 is 0
        idx = rng.choice([check.CHARSET.index("s"), rng.randrange(32)])
        chars = [t] + [rng.randrange(32) for _ in range(4)] + [idx] + [rng.randrange(32) for _ in range(c)]
        bits = (size_field << 1) | rng.getrandbits(1)
        for v in chars:
            bits = (bits << 5) | v
        nb = 4 + 5 * len(chars)
        spare = nbytes * 8 - nb
        bits = (bits << spare) | rng.getrandbits(spare)
        return (bits.to_bytes(nbytes, "big") + rng.randbytes(room))[:room]
    return rng.randbytes(room)


class Maker:
    def __init__(self, rng):
        self.rng = rng
        self.ops = []

    def add(self, op, kind, corruption, size):
        op["group"] = "%s/%d" % (KIND_NAME.get(kind, "none"), size)
        op["corruption"] = corruption
        self.ops.append(op)

    def mode(self):
        return self.rng.choice(["qr", "file"])

    def round(self, size, kind):
        rng, add = self.rng, self.add
        room, tlen = check.SIZES[size]
        item, fill = make_item(rng, size, kind)
        pw = rand_pass(rng)
        salt = rng.randbytes(check.SALT[size])
        ib = check.item_bytes_for(kind, item, fill)
        padding = rng.randbytes(room - len(ib))
        env = check.seal(pw, 0, size, kind, item, salt, padding, fill)
        header_len = 1 + check.SALT[size]

        add(seal_op(rng, size, item, fill, pw, 0, salt, padding), kind, "seal-valid", size)
        for m in ("qr", "file"):
            add(read_op(m, render(env, m), pw), kind, "valid-" + m, size)
        for name, pad in (("zeros", b"\x00"), ("ones", b"\xff"), ("pattern", b"\xa5\x5a")):
            padding2 = (pad * room)[: len(padding)]
            e2 = check.seal(pw, 0, size, kind, item, salt, padding2, fill)
            m = self.mode()
            add(read_op(m, render(e2, m), pw), kind, "padding-" + name, size)

        def flip(name, lo, hi, nbits=1):
            e = bytearray(env)
            for _ in range(nbits):
                pos = rng.randrange(lo, hi)
                e[pos] ^= 1 << rng.randrange(8)
            m = self.mode()
            add(read_op(m, render(bytes(e), m), pw), kind, name, size)

        e = bytearray(env)
        e[0] ^= 1 << rng.randrange(3, 8)
        m = self.mode()
        add(read_op(m, render(bytes(e), m), pw), kind, "flip-marker", size)
        flip("flip-salt", 1, 1 + check.SALT[size], rng.choice([1, 1, 2]))
        flip("flip-ciphertext", header_len, header_len + room, rng.choice([1, 1, 3]))
        flip("flip-tag", header_len + room, len(env), rng.choice([1, 1, 2]))
        flip("flip-any", 1, len(env), rng.randint(1, 4))

        step = rng.choice([1, 5, 6, 7, 5, 6, 7])
        e = bytes([0xF8 | step]) + env[1:]
        m = self.mode()
        add(read_op(m, render(e, m), pw), kind, "header-step", size)
        b = rng.randrange(256)
        b = (b & ~7) | rng.choice([0, 5, 6, 7])
        e = bytes([b]) + env[1:]
        m = self.mode()
        add(read_op(m, render(e, m), pw), kind, "header-byte", size)

        cut = rng.choice([0, 1, len(env) - 1, len(env) - 1, rng.randrange(len(env))])
        add(read_op(self.mode(), env[:cut], pw), kind, "truncate", size)
        if size == 62:
            add(read_op(self.mode(), env[:53], pw), kind, "truncate", size)
        ext = env + rng.randbytes(rng.choice([1, 1, 2, 9, 31]))
        add(read_op(self.mode(), ext, pw), kind, "extend", size)

        # the other kinds' tags over the same bytes: the reader finds that
        # kind, and its layout decides
        _, _, k_enc, k_mac = check.derive(pw, salt, check.STEP_KIB[0])
        c = env[header_len : header_len + room]
        tags = check.tags_for(k_mac, env[0], size, salt, c, tlen)
        for other in rng.sample([k for k in check.KINDS_OF[size] if k != kind], min(2, len(check.KINDS_OF[size]) - 1)):
            e = env[: header_len + room] + tags[other]
            m = self.mode()
            add(read_op(m, render(e, m), pw), kind, "kind-swap", size)

        for other in rng.sample(check.KINDS_OF[size], 2):
            e = forge(pw, 0, size, other, forged_plain(rng, size, other), salt)
            m = self.mode()
            add(read_op(m, render(e, m), pw), other, "forged-layout", size)

        # passphrases
        wrong = rng.choice([pw + rng.choice("a ?́"), rand_pass(rng), pw.swapcase(), pw[:-1] or "x", pw + pw])
        if wrong == pw:
            wrong += "!"
        for name, other in (
            ("wrong-passphrase", wrong),
            ("empty-passphrase", ""),
            ("passphrase-nfd", unicodedata.normalize("NFD", pw)),
            ("passphrase-nfkd", unicodedata.normalize("NFKD", pw)),
            ("passphrase-nfkc", unicodedata.normalize("NFKC", pw)),
        ):
            m = self.mode()
            add(read_op(m, render(env, m), other), kind, name, size)

        if size == 32:
            self.text_forms(env, pw, kind)

    def text_forms(self, env, pw, kind):
        rng, add = self.rng, self.add
        s = check.envelope_to_text(env)

        def t(name, text, mode="qr"):
            b = text if isinstance(text, bytes) else text.encode()
            add(read_op(mode, b, pw), kind, name, 32)

        i = rng.randrange(47)
        low = [j for j, ch in enumerate(s) if ch.isalpha()]
        j = rng.choice(low)
        t("text-lowercase", s[:j] + s[j].lower() + s[j + 1 :])
        t("text-lowercase", s.lower())
        for ch in rng.sample(["#", "a", "é", "\x00", "_", "@", "€", "Ａ", "\n", "あ", "\U0001F600", "́"], 2):
            t("text-badchar", s[:i] + ch + s[i + 1 :])
        t("text-badchar", s[:i].encode() + bytes([rng.choice([0xFF, 0xC0, 0x80])]) + s[i + 1 :].encode())
        t("text-shorter", s[: rng.choice([46, 46, rng.randrange(47)])])
        t("text-shorter", s[1:])
        t("text-longer", s + rng.choice(TABLE))
        t("text-longer", "0" + s)
        t("text-longer", s + rng.choice([" ", "\n", "\r\n", "\x00"]))
        t("text-longer", rng.choice([" ", "\n"]) + s)
        t("text-first-digit", rng.choice(TABLE) + s[1:])
        t("text-too-big", rng.choice(TABLE[12:]) + "".join(rng.choice(TABLE) for _ in range(46)))
        t("text-swap", s[:i] + s[i + 1] + s[i] + s[i + 2 :] if i < 45 else s[::-1])
        t("text-binary32", env)
        t("text-in-file", s, "file")
        t("text-in-file", s + "\n", "file")
        t("text-fullwidth", s.translate({ord(c): ord(c) + 0xFEE0 for c in "0123456789"}))


def other_payloads(rng, ops):
    """Payloads that are no valid code's corruption: random bytes and texts."""
    def add(op, corruption):
        op["group"] = "none/%d" % op["_size"]
        del op["_size"]
        op["corruption"] = corruption
        ops.append(op)

    for size in (32, 53, 62):
        pw = rand_pass(rng)
        b = bytearray(rng.randbytes(size))
        b[0] = rng.choice([0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF, 0xF7, 0x00, 0x7F])
        b[0] = (b[0] & ~7) | rng.choice([0, 5, 6, 7]) if b[0] >= 0xF8 else b[0]
        add(dict(read_op(rng.choice(["qr", "file"]), bytes(b), pw), _size=size), "random-bytes")
    pw = rand_pass(rng)
    v = rng.randrange(0xF8 << 248, 0xFD << 248)
    add(dict(read_op("qr", check.envelope_to_text(v.to_bytes(32, "big")).encode(), pw), _size=32), "random-text-in-range")
    first = rng.choice(TABLE)
    add(dict(read_op("qr", (first + "".join(rng.choice(TABLE) for _ in range(46))).encode(), pw), _size=32), "random-text")
    add(dict(read_op("qr", ("A" + "".join(rng.choice(TABLE) for _ in range(46))).encode(), pw), _size=32), "random-text")


def writer_refusals(rng, ops):
    def add(op, corruption, size):
        op["group"] = "writer/%d" % size
        op["corruption"] = corruption
        ops.append(op)

    for size in (32, 53, 62):
        room = check.SIZES[size][0]
        pw = rand_pass(rng)
        text = lambda typ, n: {"type": typ, "text": rand_text(rng, n, n)}
        for typ in ("passphrase", "note"):
            add(seal_op(rng, size, text(typ, 5), 0, ""), "writer-empty-passphrase", size)
            add(seal_op(rng, size, {"type": typ, "text": ""}, 0, pw), "writer-empty-text", size)
            for extra in (1, 2, 7):
                add(seal_op(rng, size, text(typ, room - 1 + extra), 0, pw, padding=b""), "writer-text-too-long", size)
            add(seal_op(rng, size, text(typ, room - 1), 0, pw), "writer-text-longest", size)
            add(seal_op(rng, size, {"type": typ, "text": "é" * (room // 2)}, 0, pw, padding=b""), "writer-text-too-long", size)
        for n in (0, 1, 15, 17, 19, 33, 64):
            add(seal_op(rng, size, {"type": "entropy", "hex": rng.randbytes(n).hex()}, 0, pw), "writer-entropy-length", size)
        for n in (16, 20, 24, 28, 32):
            add(seal_op(rng, size, {"type": "entropy", "hex": rng.randbytes(n).hex()}, 0, pw), "writer-entropy-held", size)
        add(seal_op(rng, size, {"type": "raw_key", "hex": rng.randbytes(32).hex()}, 0, pw), "writer-raw-key", size)
        for cs in range(5):
            fill = rng.getrandbits(codex_spare(cs)) if codex_spare(cs) else 0
            ok = codex_string(rng, cs, rng.random() < 0.5)
            add(seal_op(rng, size, {"type": "codex32", "string": ok}, fill, pw), "writer-codex32-valid", size)
            j = rng.randrange(3, len(ok))
            repl = rng.choice([c for c in check.CHARSET if c != ok[j].lower()])
            add(seal_op(rng, size, {"type": "codex32", "string": ok[:j] + (repl.upper() if ok.isupper() else repl) + ok[j + 1 :]}, fill, pw), "writer-codex32-checksum", size)
            mixed = ok.lower()
            k = rng.randrange(3, len(ok))
            add(seal_op(rng, size, {"type": "codex32", "string": mixed[:k] + mixed[k].upper() + mixed[k + 1 :]}, fill, pw), "writer-codex32-case", size)
            add(seal_op(rng, size, {"type": "codex32", "string": "ms2" + ok[3:]}, fill, pw), "writer-codex32-prefix", size)
            add(seal_op(rng, size, {"type": "codex32", "string": ok + rng.choice(check.CHARSET)}, fill, pw), "writer-codex32-length", size)
            add(seal_op(rng, size, {"type": "codex32", "string": ok[:-1]}, fill, pw), "writer-codex32-length", size)
            add(seal_op(rng, size, {"type": "codex32", "string": codex_string(rng, cs, False, threshold=rng.choice([ch for ch in check.CHARSET if ch not in "023456789"]))}, fill, pw), "writer-codex32-threshold", size)
            add(seal_op(rng, size, {"type": "codex32", "string": codex_string(rng, cs, False, threshold="0", index=rng.choice([ch for ch in check.CHARSET if ch != "s"]))}, fill, pw), "writer-codex32-threshold-index", size)
            bad = ok[:5] + ("b" if ok[5] != "b" else "i") + ok[6:]
            add(seal_op(rng, size, {"type": "codex32", "string": bad}, fill, pw), "writer-codex32-character", size)


def build(seed, rounds):
    rng = random.Random(seed)
    mk = Maker(rng)
    for _ in range(rounds):
        for size in (32, 53, 62):
            for kind in check.KINDS_OF[size]:
                mk.round(size, kind)
        other_payloads(rng, mk.ops)
        writer_refusals(rng, mk.ops)
    return mk.ops


# ------------------------------------------------------------------ the run


def rust_command(arg):
    if arg:
        return arg.split()
    return ["cargo", "run", "--locked", "--release", "-q", "-p", "aqrv-cli", "--example", "diff_batch"]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--seed", type=int, required=True, help="the seed of the generator")
    ap.add_argument("--rounds", type=int, default=1, help="a code of every kind and size, corrupted, a round")
    ap.add_argument("--jobs", type=int, default=min(4, os.cpu_count() or 1), help="Python workers")
    ap.add_argument("--rust", help="the command of the Rust batch reader (default: cargo run the example)")
    ap.add_argument("--show", type=int, default=40, help="disagreements to print in full")
    a = ap.parse_args()

    print("seed", a.seed, "rounds", a.rounds, flush=True)
    t0 = time.time()
    ops = build(a.seed, a.rounds)
    print("cases", len(ops), flush=True)
    with tempfile.TemporaryDirectory() as tmp:
        src, dst = os.path.join(tmp, "in.jsonl"), os.path.join(tmp, "out.jsonl")
        with open(src, "w", encoding="utf-8") as f:
            for op in ops:
                f.write(json.dumps({k: v for k, v in op.items() if k not in ("group", "corruption")}, ensure_ascii=False) + "\n")
        with open(src, "rb") as fi, open(dst, "wb") as fo:
            rust = subprocess.Popen(rust_command(a.rust), stdin=fi, stdout=fo, cwd=os.path.join(HERE, "..", "..", ".."))
            with multiprocessing.Pool(a.jobs) as pool:
                py = pool.map(py_run, ops, chunksize=2)
            if rust.wait() != 0:
                sys.exit("the Rust batch reader failed: %d" % rust.returncode)
        with open(dst, encoding="utf-8") as f:
            rs = [json.loads(line) for line in f]
    if len(rs) != len(ops):
        sys.exit("the Rust side answered %d of %d cases" % (len(rs), len(ops)))

    per_group, per_corruption = collections.Counter(), collections.Counter()
    outcomes = collections.Counter()
    bad = []
    for op, p, r in zip(ops, py, rs):
        per_group[op["group"]] += 1
        per_corruption[op["corruption"]] += 1
        outcomes[(op["corruption"], p.get("answer", "recognised" if p.get("recognised") else p.get("envelope") and "sealed" or ("refused" if "refused" in p else "not_recognised")))] += 1
        if not same(p, r):
            bad.append((op, p, r))
    print("\ncases per kind and size")
    for k, n in sorted(per_group.items()):
        print("  %-18s %d" % (k, n))
    print("\ncases per corruption (python outcomes in brackets)")
    for k, n in sorted(per_corruption.items()):
        o = ", ".join("%s %d" % (w, c) for (kk, w), c in sorted(outcomes.items()) if kk == k)
        print("  %-30s %5d  [%s]" % (k, n, o))
    print("\ndisagreements: %d of %d cases" % (len(bad), len(ops)))
    by = collections.Counter(op["corruption"] for op, _, _ in bad)
    for k, n in sorted(by.items()):
        print("  %-30s %d" % (k, n))
    for op, p, r in bad[: a.show]:
        print("\n--", op["group"], op["corruption"])
        print("   input ", json.dumps({k: v for k, v in op.items() if k not in ("group", "corruption")}, ensure_ascii=False))
        print("   python", json.dumps(p, ensure_ascii=False))
        print("   rust  ", json.dumps(r, ensure_ascii=False))
    print("\nrun time %.0f s, seed %d" % (time.time() - t0, a.seed))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
