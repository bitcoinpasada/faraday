"""The outside judges of the `ur` area's CRC-32 and CBOR: CPython's
`zlib.crc32` and cbor2 (package python3-cbor2), neither written by
Faraday. Used by `area_ur.py`; the expected answers for the registry's
types are worked out here from what was put in, with btclib's curve and
base58 (`bip32_ref.py`), never from Faraday's code.
"""

import json
import zlib

import cbor2

from bip32_ref import b58check_encode, point
from common import Mismatch

# Classes of input Faraday's CBOR reader refuses and cbor2 takes (the
# area's docstring names each).
REFUSED = ("indefinite", "float", "simple", "deep", "duplicate", "break")
MAX_DEPTH = 16  # `faraday_codec::cbor`'s bound on nesting
# Tags below 300 and from 1000 on are the ones cbor2 reads as types of
# its own (dates, bignums, sets, ...) and so may refuse or read as other
# values; the registry's lie between.


def own_tag(tag):
    return tag < 300 or tag >= 1000


def check_crc(area, data):
    """Faraday's CRC-32 of `data`, against `zlib.crc32`."""
    area.compared += 1
    got = area.rust.call("ur.crc32", data=bytes(data).hex())
    want = ("ok", {"crc": zlib.crc32(bytes(data))})
    if got != want:
        raise Mismatch(area.name, "ur.crc32", {"data": bytes(data).hex()}, got, want,
                       "zlib.crc32 differs")


def cbor_loads(data):
    """The one item `data` holds, nothing after it, as cbor2 reads it.
    cbor2's decoder reads ahead of the item, so the item's end is the
    shortest prefix it reads."""
    v = cbor2.loads(data)
    for k in range(1, len(data)):
        try:
            cbor2.loads(data[:k])
        except Exception:  # noqa: BLE001 - a short read is the expected case
            continue
        raise ValueError("bytes after the item")
    return v


def canonical(v):
    return cbor2.dumps(v, canonical=True)


def check_part(area, part, message_cbor):
    """The CBOR of one fountain part Faraday wrote: cbor2 reads it as
    `[seq, count, message length, checksum, data]`, the checksum is
    `zlib.crc32` of the message, and cbor2 writes the same bytes."""
    from ur.bytewords import Bytewords, Bytewords_Style_minimal
    body = Bytewords.decode(Bytewords_Style_minimal, part.lower().split("/")[-1])
    area.compared += 1
    try:
        v = cbor_loads(bytes(body))
        ok = (isinstance(v, list) and len(v) == 5 and v[2] == len(message_cbor)
              and v[3] == zlib.crc32(message_cbor) and isinstance(v[4], bytes)
              and canonical(v) == bytes(body))
    except Exception:  # noqa: BLE001 - any refusal is the failure
        ok = False
    if not ok:
        raise Mismatch(area.name, "ur.encode", {"part": part}, ("ok", bytes(body).hex()),
                       ("ok", f"checksum {zlib.crc32(message_cbor)} of {len(message_cbor)} bytes"),
                       "cbor2 or zlib.crc32 differs on a part's CBOR")


def check_single(area, text, data):
    """A single-part `crypto-psbt` or `bytes` UR: its CBOR is the byte
    string of what was sent, and cbor2 writes the same bytes."""
    from ur.ur_decoder import URDecoder
    cbor = bytes(URDecoder.decode(text).cbor)
    area.compared += 1
    try:
        ok = cbor_loads(cbor) == bytes(data) and canonical(bytes(data)) == cbor
    except Exception:  # noqa: BLE001
        ok = False
    if not ok:
        raise Mismatch(area.name, "ur.encode", {"single": text}, ("ok", cbor.hex()),
                       ("ok", bytes(data).hex()), "cbor2 reads another value")


# ---- the reader against cbor2 ------------------------------------------

def head(major, n, width=None):
    """A CBOR head; `width` 0 to 4 forces the argument's form (inline, 1,
    2, 4, 8 bytes) where it fits, even when longer than needed."""
    if width is None:
        width = 0 if n < 24 else 1 if n < 256 else 2 if n < 65536 else 3 if n < 2**32 else 4
    if width == 0 and n < 24:
        return bytes([major << 5 | n])
    width = max(width, 1)
    size = {1: 1, 2: 2, 3: 4, 4: 8}[width]
    if n >= 256 ** size:
        return head(major, n)
    return bytes([major << 5 | (23 + width)]) + n.to_bytes(size, "big")


def gen(rng, depth=0):
    """Random CBOR: mostly what Faraday reads, some of every class it
    refuses."""
    kinds = ["u", "u", "n", "b", "t", "a", "m", "g", "f"]
    if depth > 3:
        kinds = ["u", "n", "b", "t", "f"]
    k = rng.choice(kinds + ["x"] * (1 if rng.random() < 0.3 else 0))
    width = rng.choice([None, None, None, 1, 2, 3, 4])
    if k in "un":
        n = rng.choice([rng.randrange(24), rng.randrange(256), rng.randrange(2**16),
                        rng.randrange(2**32), rng.randrange(2**64)])
        return head(0 if k == "u" else 1, n, width)
    if k == "b":
        d = rng.randbytes(rng.randrange(40))
        return head(2, len(d), width) + d
    if k == "t":
        d = "".join(rng.choice("abc é€𝄞") for _ in range(rng.randrange(12))).encode()
        return head(3, len(d), width) + d
    if k == "a":
        items = [gen(rng, depth + 1) for _ in range(rng.randrange(5))]
        return head(4, len(items), width) + b"".join(items)
    if k == "m":
        n = rng.randrange(4)
        out = head(5, n, width)
        for _ in range(n):
            key = gen(rng, 9) if rng.random() < 0.9 else gen(rng, depth + 1)
            out += key + gen(rng, depth + 1)
        return out
    if k == "g":
        return head(6, rng.choice([303, 304, 308, 400, 401, 404, rng.randrange(300, 1000)]),
                    width) + gen(rng, depth + 1)
    if k == "f":
        return bytes([rng.choice([0xf4, 0xf5, 0xf6])])
    # A class Faraday refuses.
    return rng.choice([
        lambda: b"\x9f" + gen(rng, depth + 1) + b"\xff",
        lambda: b"\xbf" + gen(rng, 9) + gen(rng, 9) + b"\xff",
        lambda: b"\x5f" + head(2, 2) + b"ab" + b"\xff",
        lambda: b"\x7f" + head(3, 2) + b"ab" + b"\xff",
        lambda: b"\xf9" + rng.randbytes(2),
        lambda: b"\xfa" + rng.randbytes(4),
        lambda: b"\xfb" + rng.randbytes(8),
        lambda: b"\xf7",
        lambda: bytes([0xe0 | rng.randrange(20)]),
        lambda: b"\xf8" + bytes([rng.randrange(32, 256)]),
        lambda: b"\x81" * rng.randrange(14, 22) + b"\x00",
        lambda: duplicate(rng),
    ])()


def duplicate(rng):
    """A map that gives a key twice, the second time in the same bytes or
    (a number) in a longer form."""
    key = gen(rng, 9)
    again = key
    if rng.random() < 0.5:
        n = rng.randrange(24)
        key, again = head(0, n), head(0, n, rng.choice([1, 2, 3, 4]))
    return head(5, 2) + key + gen(rng, 9) + again + gen(rng, 9)


def mutate(rng, data):
    data = bytearray(data)
    for _ in range(rng.randrange(1, 4)):
        r = rng.random()
        if r < 0.4 and data:
            data[rng.randrange(len(data))] = rng.randrange(256)
        elif r < 0.6 and data:
            del data[rng.randrange(len(data)):]
        elif r < 0.8:
            data += rng.randbytes(rng.randrange(1, 4))
        elif data:
            del data[rng.randrange(len(data))]
    return bytes(data)


def features(data):
    """The classes of input in `data` that decide a difference: read by
    a walk of the bytes' own, as far as they parse."""
    found = set()

    def key_seen(key, seen):
        try:
            value = repr(cbor2.loads(key))
        except Exception:  # noqa: BLE001 - a key cbor2 refuses is no duplicate of its own
            return
        if value in seen:
            found.add("duplicate")
        seen.add(value)

    def walk(i, depth):
        if depth > MAX_DEPTH:
            found.add("deep")
        h = data[i]
        major, info = h >> 5, h & 31
        i += 1
        if major == 7:
            if info in (25, 26, 27):
                found.add("float")
                return i + {25: 2, 26: 4, 27: 8}[info]
            if info == 31:
                found.add("break")  # an indefinite item's own is read below
            if info == 31 or info >= 28:
                raise ValueError("break or reserved")
            found.add("simple") if info not in (20, 21, 22) else None
            return i + (1 if info == 24 else 0)
        if info == 31:
            if major < 2 or major > 5:
                raise ValueError("indefinite head of another type")
            found.add("indefinite")
            seen = set()
            while data[i] != 0xff:
                at = i
                i = walk(i, depth + 1)
                if major == 5:
                    key_seen(data[at:i], seen)
                    i = walk(i, depth + 1)
            return i + 1
        if info >= 28:
            raise ValueError("reserved")
        size = {24: 1, 25: 2, 26: 4, 27: 8}.get(info, 0)
        n = info if info < 24 else int.from_bytes(data[i:i + size], "big")
        i += size
        if major in (2, 3):
            return i + n
        if major == 4:
            for _ in range(n):
                i = walk(i, depth + 1)
        elif major == 5:
            seen = set()
            for _ in range(min(n, len(data))):
                at = i
                i = walk(i, depth + 1)
                key_seen(data[at:i], seen)
                i = walk(i, depth + 1)
        elif major == 6:
            if own_tag(n):
                found.add("own-tag")
            i = walk(i, depth + 1)
        return i

    try:
        walk(0, 0)
    except (IndexError, ValueError, RecursionError):
        pass
    return found


SCALAR = ("u", "n", "b", "t", "f", "z")


def py_of(t):
    """Faraday's tree as the Python value cbor2 would read it, or None
    where a map has a key that is not a scalar."""
    kind = t[0]
    if kind == "u":
        return t[1]
    if kind == "n":
        return -1 - t[1]
    if kind == "b":
        return bytes.fromhex(t[1])
    if kind in ("t", "f"):
        return t[1]
    if kind == "z":
        return None
    if kind == "a":
        return [py_of(x) for x in t[1]]
    if kind == "g":
        return cbor2.CBORTag(t[1], py_of(t[2]))
    if kind == "m":
        out = {}
        seen = set()
        for k, v in t[1]:
            if k[0] not in SCALAR:
                raise TypeError("composite key")
            if json.dumps(k) in seen:
                raise ValueError("a key twice")
            seen.add(json.dumps(k))
            out[py_of(k)] = py_of(v)
        return out
    raise ValueError(kind)


def norm(v):
    if v is None:
        return ["z"]
    if isinstance(v, bool):
        return ["f", v]
    if isinstance(v, int):
        return ["u", v] if v >= 0 else ["n", v]
    if isinstance(v, bytes):
        return ["b", v.hex()]
    if isinstance(v, str):
        return ["t", v]
    if isinstance(v, (list, tuple)):
        return ["a", [norm(x) for x in v]]
    if isinstance(v, cbor2.CBORTag):
        return ["g", v.tag, norm(v.value)]
    if isinstance(v, dict):
        pairs = sorted(([norm(k), norm(x)] for k, x in v.items()), key=json.dumps)
        return ["m", pairs]
    raise ValueError(type(v))


def check_cbor(area, data):
    """Faraday's reader and cbor2 on one input: both refuse it or both
    read the same values. Differences that are known are counted."""
    data = bytes(data)
    inputs = {"cbor": data.hex()}
    got = area.rust.call("ur.cbor", **inputs)
    try:
        theirs = ("ok", cbor_loads(data))
    except Exception as e:  # noqa: BLE001 - any refusal
        theirs = ("err", f"{type(e).__name__}: {e}")
    area.compared += 1
    feats = features(data)
    if got[0] == "err" and theirs[0] == "err":
        return
    if got[0] == "ok" and theirs[0] == "err":
        if "own-tag" in feats:
            area.known += 1
            return
        raise Mismatch(area.name, "ur.cbor", inputs, got, theirs,
                       "Faraday reads what cbor2 refuses")
    if got[0] == "err" and theirs[0] == "ok":
        if feats & set(REFUSED):
            area.known += 1
            return
        raise Mismatch(area.name, "ur.cbor", inputs, got, ("ok", norm_safe(theirs[1])),
                       "Faraday refuses what cbor2 reads, of no class named")
    try:
        mine = norm(py_of(got[1]))
    except TypeError:
        return  # a map under a composite key: acceptance only
    except (ValueError, IndexError, KeyError):
        raise Mismatch(area.name, "ur.cbor", inputs, got, ("ok", norm_safe(theirs[1])),
                       "Faraday's answer is not a CBOR tree")
    if mine != norm_safe(theirs[1]):
        if "own-tag" in feats:
            area.known += 1
            return
        raise Mismatch(area.name, "ur.cbor", inputs, ("ok", mine), ("ok", norm_safe(theirs[1])),
                       "Faraday and cbor2 read different values")


def norm_safe(v):
    try:
        return norm(v)
    except ValueError:
        return repr(v)


# ---- the registry's types, built with cbor2 ----------------------------

def xpub_text(key, chain, depth, parent, child, test):
    version = bytes.fromhex("043587cf" if test else "0488b21e")
    return b58check_encode(version + bytes([depth]) + parent.to_bytes(4, "big")
                           + child.to_bytes(4, "big") + chain + key)


_KEYS = []


def public_keys():
    """A dozen public keys, made once (the curve is slow in Python)."""
    if not _KEYS:
        _KEYS.extend(point(0x1234567 * (i + 1) ** 5 + i) for i in range(12))
    return _KEYS


class Hd:
    """One extended public key as the registry states it, with the text
    it stands for worked out from the fields alone."""

    def __init__(self, rng, children=None, test=None):
        self.key = rng.choice(public_keys())
        self.chain = rng.randbytes(32)
        self.test = rng.random() < 0.3 if test is None else test
        self.path = [(rng.randrange(2**31), rng.random() < 0.7)
                     for _ in range(rng.randrange(1, 6))]
        # A source fingerprint is never 0 (the specification says `.ne 0`),
        # and with one step the parent is the master.
        self.source = rng.randrange(1, 2**32)
        self.parent = self.source if len(self.path) == 1 else rng.randrange(2**32)
        # `ur:hdkey`'s own numbers for the key, key path and use-info tags.
        self.tags = (40303, 40304, 40305) if rng.random() < 0.3 else (303, 304, 305)
        self.children = children
        self.with_depth = rng.random() < 0.7

    def map(self):
        keypath = {1: [x for c in self.path for x in c], 2: self.source}
        if self.with_depth:
            keypath[3] = len(self.path)
        m = {3: self.key, 4: self.chain}
        if self.test:
            m[5] = cbor2.CBORTag(self.tags[2], {2: 1})
        m[6] = cbor2.CBORTag(self.tags[1], keypath)
        if self.children is not None:
            m[7] = cbor2.CBORTag(self.tags[1], {1: self.children})
        m[8] = self.parent
        return m

    def tagged(self):
        return cbor2.CBORTag(self.tags[0], self.map())

    def text(self):
        index, hardened = self.path[-1]
        child = index + 2**31 * hardened
        xpub = xpub_text(self.key, self.chain, len(self.path), self.parent, child, self.test)
        path = "".join(f"/{i}{'h' if h else ''}" for i, h in self.path)
        return f"[{self.source:08x}{path}]{xpub}"


def expect_keys(area, ur_type, value, lines):
    """Faraday's reading of a registry UR: its key lines are `lines`."""
    cbor = cbor2.dumps(value)
    got = area.rust.call("ur.registry", type=ur_type, cbor=cbor.hex())
    area.compared += 1
    seen = [] if got[0] != "ok" else [
        ln for ln in (got[1] or {}).get("keys", "").splitlines() if not ln.startswith("#")]
    if got[0] != "ok" or seen != lines:
        raise Mismatch(area.name, "ur.registry", {"type": ur_type, "cbor": cbor.hex()}, got,
                       ("ok", lines), "the key lines worked out from what was put in")


def expect_descriptor(area, value, text):
    cbor = cbor2.dumps(value)
    got = area.rust.call("ur.registry", type="crypto-output", cbor=cbor.hex())
    area.compared += 1
    if got != ("ok", {"descriptor": text + "\n"}):
        raise Mismatch(area.name, "ur.registry", {"type": "crypto-output", "cbor": cbor.hex()},
                       got, ("ok", text), "the descriptor worked out from what was put in")


def expect_refused(area, ur_type, value):
    expect_refused_bytes(area, ur_type, cbor2.dumps(value))


def expect_refused_bytes(area, ur_type, cbor):
    got = area.rust.call("ur.registry", type=ur_type, cbor=cbor.hex())
    area.compared += 1
    if got[0] != "err":
        raise Mismatch(area.name, "ur.registry", {"type": ur_type, "cbor": cbor.hex()}, got,
                       ("err", None), "a seed, a private key or a key given twice must stay refused")


SCRIPTS = [("wsh", [401], 0), ("sh-wsh", [400, 401], 1), ("wpkh", [404], 2),
           ("sh-wpkh", [400, 404], 3), ("tr", [409], 4), ("pkh", [403], 5)]


def wrap(tags, inner):
    for t in reversed(tags):
        inner = cbor2.CBORTag(t, inner)
    return inner


def registry_case(area):
    rng = area.rng
    r = rng.randrange(6)
    if r == 0:
        hd = Hd(rng)
        value = hd.tagged() if rng.random() < 0.7 else hd.map()
        expect_keys(area, rng.choice(["crypto-hdkey", "hdkey"]), value, [hd.text()])
    elif r == 1:
        hd = Hd(rng)
        value = hd.map()
        value[2] = True
        expect_refused(area, "crypto-hdkey", value)
        expect_refused(area, "crypto-seed", cbor2.CBORTag(300, {1: rng.randbytes(16)}))
        one, two = rng.sample(public_keys(), 2)
        expect_refused_bytes(area, "crypto-hdkey", b"".join([
            head(5, 3), head(0, 3), cbor2.dumps(one), head(0, 3), cbor2.dumps(two),
            head(0, 4), cbor2.dumps(rng.randbytes(32))]))
    elif r == 2:
        chosen = [s for s in SCRIPTS if rng.random() < 0.5] or [SCRIPTS[2]]
        keys = [(s, Hd(rng)) for s in chosen]
        outputs = []
        for s, hd in keys:
            o = wrap(s[1], hd.tagged())
            outputs.append(cbor2.CBORTag(308, o) if rng.random() < 0.3 else o)
        account = {1: rng.randrange(2**32), 2: outputs}
        value = cbor2.CBORTag(311, account) if rng.random() < 0.5 else account
        keys.sort(key=lambda sk: sk[0][2])
        expect_keys(area, "crypto-account", value, [hd.text() for _, hd in keys])
    elif r == 3:
        children = rng.choice([None, [0, False, [], False], [1, False, [], False]])
        hd = Hd(rng, children)
        shown = "/1/*" if children and children[0] == 1 else "/<0;1>/*"
        name, tags, _ = rng.choice(SCRIPTS[2:])
        value = wrap(tags, hd.tagged())
        if rng.random() < 0.3:
            value = cbor2.CBORTag(308, value)
        expect_descriptor(area, value, f"{name.split('-')[-1]}({hd.text()}{shown})"
                          if name != "sh-wpkh" else f"sh(wpkh({hd.text()}{shown}))")
    elif r == 4:
        n = rng.randrange(1, 4)
        hds = [Hd(rng) for _ in range(n)]
        m = rng.randrange(1, n + 1)
        sorted_ = rng.random() < 0.5
        body = cbor2.CBORTag(407 if sorted_ else 406, {1: m, 2: [h.tagged() for h in hds]})
        name = "sortedmulti" if sorted_ else "multi"
        inner = f"{name}({m},{','.join(h.text() + '/<0;1>/*' for h in hds)})"
        form = rng.choice(["wsh", "sh-wsh", "sh"])
        value = wrap({"wsh": [401], "sh-wsh": [400, 401], "sh": [400]}[form], body)
        text = {"wsh": f"wsh({inner})", "sh-wsh": f"sh(wsh({inner}))", "sh": f"sh({inner})"}[form]
        expect_descriptor(area, value, text)
    else:
        account_case(area)


def account_case(area):
    """`account_cbor`'s output as cbor2 reads it: what was given."""
    rng = area.rng
    hd = Hd(rng)
    hd.children = None
    wpkh = rng.random() < 0.5
    master = rng.randrange(1, 2**32)
    hd.source = master
    if len(hd.path) == 1:
        hd.parent = master  # with one step the parent is the master
    xpub = xpub_text(hd.key, hd.chain, len(hd.path), hd.parent,
                     hd.path[-1][0] + 2**31 * hd.path[-1][1], hd.test)
    hd.with_depth = True
    hd.tags = (303, 304, 305)  # the numbers Faraday writes
    path = [f"{i}{'h' if h else ''}" for i, h in hd.path]
    inputs = {"master": master.to_bytes(4, "big").hex(), "path": path, "xpub": xpub,
              "wpkh": wpkh}
    got = area.rust.call("ur.account", **inputs)
    area.compared += 1
    want = {1: master, 2: [cbor2.CBORTag(404 if wpkh else 401, hd.tagged())]}
    try:
        cbor = bytes.fromhex(got[1]["cbor"])
        ok = got[0] == "ok" and cbor_loads(cbor) == want and canonical(want) == cbor
    except Exception:  # noqa: BLE001
        ok = False
    if not ok:
        raise Mismatch(area.name, "ur.account", inputs, got, ("ok", norm_safe(want)),
                       "cbor2 reads another account than was given")
    expect_keys(area, "crypto-account", want, [hd.text()])
