"""A BOLT #12 offer writer for the `lightning` area: random offers with every
field the reader knows, and the same offers spoiled in one way each. Written
here from BOLT #12's text (the offer's TLV types, BOLT #1's BigSize and
truncated integers, the blinded path of BOLT #4, the text form: Bech32
characters without a checksum, `+` joins) and BOLT #1's rules for text. It
shares nothing with Faraday's reader or with Core Lightning's.

An offer is a dict: `chains` (32-byte hashes), `metadata`, `currency`,
`amount`, `description`, `features` (bit numbers), `expiry`, `paths` (a list
of paths, each a dict), `issuer`, `quantity_max`, `issuer_id`, and `unknown`
(odd types the reader does not define, as (type, bytes)). `tlvs_of` turns it
into (type, value) records, `stream` into bytes and `text_of` into the text.
Points are valid compressed keys unless a spoil changes one; btclib (on its
own curve) says whether a given byte string is one (`point_ok`).
"""

import xml.etree.ElementTree as ET

import bip32_ref
from common import ROOT
from ecc.curves import point_from_octets_var, secp256k1

CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
# The genesis hashes, as BOLT #12 writes them (the block hash's bytes reversed).
GENESIS = {
    "bitcoin": "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
    "testnet3": "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943",
    "testnet4": "00000000da84f2bafbbc53dee25a72ae507ff4914b867c565be350b0da8bf043",
    "signet": "00000008819873e925422c1ff0f99f7cc9bbb232af63a077a480a3633bee1ef6",
    "regtest": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
}
KNOWN_CHAINS = [bytes.fromhex(h)[::-1] for h in GENESIS.values()]
# Types the reader defines, and the three odd ones Core Lightning also reads
# (its recurrence fields, which BOLT #12 no longer has) that the writer avoids.
DEFINED = {2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22}
RECURRENCE = {1000000024, 1000000025, 1000000026, 1000000027, 1000000028, 1000000029}
# Text that is valid and shows something: ASCII, Latin, Greek, Cyrillic, CJK,
# an emoji outside the BMP. No control, format, private-use or unassigned
# character.
ALPHABETS = [
    "abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 .,:;-_/()",
    "éèçñüößåøæ ÀÉÎÕÜ",
    "αβγδεζηθικλμνξοπρστυφχψω",
    "абвгдежзийклмнопрстуфхцчшщ",
    "日本語中文한국어",
    "😀🚀⚡",
]


def currencies():
    """Every ISO 4217 code of the pinned list with its minor unit digits,
    read from the XML here (codes with no numeric minor unit left out)."""
    root = ET.parse(ROOT / "tools/vectors/lightning-bolt12/iso4217-list-one.xml").getroot()
    out = {}
    for e in root.iter("CcyNtry"):
        code, minor = e.findtext("Ccy"), e.findtext("CcyMnrUnts")
        if code and minor and minor.isdigit():
            out[code] = int(minor)
    return out


CURRENCIES = currencies()


def bigsize(n):
    if n < 0xFD:
        return bytes([n])
    if n <= 0xFFFF:
        return b"\xfd" + n.to_bytes(2, "big")
    if n <= 0xFFFFFFFF:
        return b"\xfe" + n.to_bytes(4, "big")
    return b"\xff" + n.to_bytes(8, "big")


def tu(n):
    """A truncated integer: the fewest bytes, none for 0."""
    return n.to_bytes((n.bit_length() + 7) // 8, "big")


def record(t, value):
    return bigsize(t) + bigsize(len(value)) + value


def point_ok(b):
    """Whether btclib reads these bytes as a compressed curve point."""
    if len(b) != 33 or b[0] not in (2, 3):
        return False
    try:
        point_from_octets_var(b, secp256k1)
    except Exception:  # noqa: BLE001 - any refusal is one
        return False
    return True


def key(rng):
    """Costs an elliptic-curve multiplication in btclib."""
    return bip32_ref.point(rng.randrange(1, secp256k1.n))


def off_curve(rng):
    """33 bytes, 02 or 03 and an x with no point on the curve (btclib says)."""
    while True:
        b = bytes([rng.choice([2, 3])]) + rng.randbytes(32)
        if not point_ok(b):
            return b


def text(rng, lo=1, hi=30):
    alphabet = "".join(rng.sample(ALPHABETS, rng.randint(1, 3)))
    return "".join(rng.choice(alphabet) for _ in range(rng.randint(lo, hi)))


def path(rng, hops=None):
    """A blinded path: a first node (a key, or a channel and a direction),
    the first path key, and hops of a blinded key and encrypted data."""
    if rng.random() < 0.5:
        first = key(rng)
    else:
        first = bytes([rng.randint(0, 1)]) + rng.randbytes(8)
    return {"first": first, "path_key": key(rng),
            "hops": [(key(rng), rng.randbytes(rng.randint(0, 50)))
                     for _ in range(hops or rng.randint(1, 3))]}


def path_bytes(p):
    return (p["first"] + p["path_key"] + bytes([len(p["hops"])])
            + b"".join(k + len(d).to_bytes(2, "big") + d for k, d in p["hops"]))


def feature_bytes(bits):
    """BOLT #9's bit numbers as bytes, bit 0 the lowest bit of the last."""
    if not bits:
        return b""
    out = bytearray((max(bits) // 8) + 1)
    for b in bits:
        out[len(out) - 1 - b // 8] |= 1 << (b % 8)
    return bytes(out)


def unknown_type(rng):
    """An odd type outside the ones either reader defines, in a range an
    offer may use."""
    while True:
        t = rng.choice([rng.randrange(1, 80), rng.randrange(1_000_000_000, 2_000_000_000)]) | 1
        if t not in DEFINED | RECURRENCE and (1 <= t <= 79 or 1_000_000_000 <= t <= 1_999_999_999):
            return t


def random_offer(rng):
    o = {"chains": [], "metadata": None, "currency": None, "amount": None, "description": None,
         "features": [], "expiry": None, "paths": None, "issuer": None, "quantity_max": None,
         "issuer_id": None, "unknown": []}
    if rng.random() < 0.5:
        # A known chain first: Core Lightning reads an amount in bitcoin by the
        # first chain alone.
        o["chains"] = rng.sample(KNOWN_CHAINS, rng.randint(1, 3))
        if rng.random() < 0.3:
            o["chains"].append(rng.randbytes(32))
    if rng.random() < 0.4:
        o["metadata"] = rng.randbytes(rng.randint(0, 40))
    if rng.random() < 0.6:
        if rng.random() < 0.4:
            o["currency"] = rng.choice(sorted(CURRENCIES))
            o["amount"] = rng.choice([1, rng.randint(1, 10**6), rng.randint(1, 2**40)])
        else:
            o["amount"] = rng.choice([1, 1000, rng.randint(1, 10**6),
                                      rng.randint(1, 2100000000000000000)])
        o["description"] = text(rng)
    elif rng.random() < 0.5:
        o["description"] = text(rng, 0, 40)
    if rng.random() < 0.5:
        o["features"] = sorted(rng.sample(range(1, 120, 2), rng.randint(1, 4)))
    if rng.random() < 0.5:
        o["expiry"] = rng.choice([0, 1, rng.randint(0, 2**33), 253402300799])
    if rng.random() < 0.5:
        o["issuer"] = text(rng)
    if rng.random() < 0.4:
        o["quantity_max"] = rng.choice([0, 1, rng.randint(1, 2**32), rng.randint(1, 2**64 - 1)])
    paths = rng.random() < 0.5
    if paths:
        o["paths"] = [path(rng) for _ in range(rng.randint(1, 3))]
    if not paths or rng.random() < 0.5:
        o["issuer_id"] = key(rng)
    for _ in range(rng.choice([0, 0, 1, 2, 3])):
        t = unknown_type(rng)
        if all(t != u for u, _ in o["unknown"]):
            o["unknown"].append((t, rng.randbytes(rng.randint(0, 40))))
    return o


def tlvs_of(o):
    """The (type, value) records of an offer, in increasing type."""
    r = []
    if o["chains"]:
        r.append((2, b"".join(o["chains"])))
    if o["metadata"] is not None:
        r.append((4, o["metadata"]))
    if o["currency"] is not None:
        r.append((6, o["currency"].encode()))
    if o["amount"] is not None:
        r.append((8, tu(o["amount"])))
    if o["description"] is not None:
        r.append((10, o["description"].encode()))
    if o["features"]:
        r.append((12, feature_bytes(o["features"])))
    if o["expiry"] is not None:
        r.append((14, tu(o["expiry"])))
    if o["paths"] is not None:
        r.append((16, b"".join(path_bytes(p) for p in o["paths"])))
    if o["issuer"] is not None:
        r.append((18, o["issuer"].encode()))
    if o["quantity_max"] is not None:
        r.append((20, tu(o["quantity_max"])))
    if o["issuer_id"] is not None:
        r.append((22, o["issuer_id"]))
    r.extend(o["unknown"])
    return sorted(r, key=lambda x: x[0])


def stream(records):
    return b"".join(record(t, v) for t, v in records)


def to5(data, pad=None):
    """Bytes as 5-bit values, zero padded (`pad` gives the padding bits)."""
    bits = "".join(f"{b:08b}" for b in data)
    n = -len(bits) % 5
    bits += pad if pad is not None else "0" * n
    return [int(bits[i:i + 5], 2) for i in range(0, len(bits), 5)]


def text_of(data, rng=None, hrp="lno", pad=None):
    """The offer's text, from its bytes. With `rng`, it is split by `+` and
    whitespace in places between Bech32 characters, and in upper case a
    third of the time."""
    body = "".join(CHARSET[v] for v in to5(data, pad))
    s = hrp + "1" + body
    if rng is not None:
        for _ in range(rng.choice([0, 0, 1, 2, 3])):
            i = rng.randrange(1, len(s) - 1)
            while s[i - 1] not in CHARSET or s[i] not in CHARSET:
                i = rng.randrange(1, len(s) - 1)
            s = s[:i] + "+" + "".join(rng.choice(" \t\r\n") for _ in range(rng.randint(0, 3))) + s[i:]
        if rng.random() < 1 / 3:
            s = s.upper()
    return s


def mixed_case(t, rng):
    """One letter of the data part in the other case from the rest."""
    i = rng.choice([k for k, c in enumerate(t) if c.isalpha() and k > 3])
    return t[:i] + t[i].swapcase() + t[i + 1:]


# --- Spoils ---------------------------------------------------------------
# Each returns the spoiled text. KINDS lists them with who refuses: "both"
# (BOLT #12 or BOLT #1 makes it a refusal: Faraday and Core Lightning must
# both refuse it) or "faraday" (BOLT #12 leaves it open and Faraday refuses
# by decision, F232; Core Lightning is expected to read it).

def with_changes(o, **changes):
    return dict(o, **changes)


def base(rng, **changes):
    """A valid offer with a description and an issuer id, changed."""
    o = random_offer(rng)
    o.update({"description": text(rng), "issuer_id": key(rng)})
    o.update(changes)
    return o


def raw(records):
    return text_of(stream(records))


def s_order(rng):
    r = tlvs_of(base(rng, metadata=b"ab", issuer="x"))
    i = rng.randrange(len(r) - 1)
    r[i], r[i + 1] = r[i + 1], r[i]
    return raw(r)


def s_repeat(rng):
    r = tlvs_of(base(rng))
    i = rng.randrange(len(r))
    return raw(r[:i + 1] + [r[i]] + r[i + 1:])


def s_truncated(rng):
    """The data cut short by a byte, in the last record's value (the issuer
    id, last in an offer without a field of a type above 22)."""
    data = stream(tlvs_of(base(rng, unknown=[])))
    return text_of(data[:-1])


def s_length_past_end(rng):
    r = tlvs_of(base(rng))
    t, v = r[-1]
    return text_of(stream(r[:-1]) + bigsize(t) + bigsize(len(v) + rng.randint(1, 5)) + v)


def s_bigsize_nonminimal(rng):
    r = tlvs_of(base(rng))
    i = rng.randrange(len(r))
    t, v = r[i]
    # The next longer form of the type: never the shortest one for its value.
    if t < 0xFD:
        wide = b"\xfd" + t.to_bytes(2, "big")
    elif t <= 0xFFFF:
        wide = b"\xfe" + t.to_bytes(4, "big")
    else:
        wide = b"\xff" + t.to_bytes(8, "big")
    if rng.random() < 0.5 and len(v) < 0xFD:
        return raw_parts(r, i, bigsize(t) + b"\xfd" + len(v).to_bytes(2, "big") + v)
    return raw_parts(r, i, wide + bigsize(len(v)) + v)


def raw_parts(r, i, replaced):
    return text_of(stream(r[:i]) + replaced + stream(r[i + 1:]))


def s_tu_leading_zero(rng):
    kind = rng.choice([8, 14, 20])
    r = [x for x in tlvs_of(base(rng)) if x[0] != kind]
    r.append((kind, b"\x00" + tu(rng.randint(1, 2**30))))
    return raw(sorted(r, key=lambda x: x[0]))


def s_tu_overlong(rng):
    """A truncated integer longer than eight bytes."""
    kind = rng.choice([8, 14, 20])
    o = base(rng, description="d")
    r = [x for x in tlvs_of(o) if x[0] != kind] + [(kind, b"\x01" + bytes(8))]
    return raw(sorted(r, key=lambda x: x[0]))


def s_type_out_of_range(rng):
    t = rng.choice([0, 80, 81, 999_999_999, 2_000_000_000, 2_000_000_001, 2**32 + 1])
    r = tlvs_of(base(rng)) + [(t, b"")]
    return raw(sorted(r, key=lambda x: x[0]))


def s_unknown_even(rng):
    t = rng.choice([24, 30, 78, 1_000_000_030, 1_000_000_100, 1_999_999_998])
    r = tlvs_of(base(rng)) + [(t, rng.randbytes(rng.randint(0, 5)))]
    return raw(sorted(r, key=lambda x: x[0]))


def s_features_leading_zero(rng):
    o = base(rng)
    r = [x for x in tlvs_of(o) if x[0] != 12] + [(12, b"\x00" + feature_bytes([rng.choice([1, 3, 9])]))]
    return raw(sorted(r, key=lambda x: x[0]))


def s_features_even(rng):
    bit = rng.choice([0, 2, 8, 10, 34, 60, 100])
    o = base(rng, features=sorted({bit, rng.choice([1, 3, 5, 99])}))
    return raw(tlvs_of(o))


def bad_text(rng):
    """Text field bytes that are not BOLT #1's text: invalid UTF-8, or valid
    with a character BOLT #1 refuses."""
    choice = rng.choice(["utf8", "cc", "cf", "co", "zl", "zp", "nul"])
    if choice == "utf8":
        return rng.choice([b"\xff", b"\xc0\xaf", b"\xe2\x82", b"\xed\xa0\x80", b"\xf8\x88\x80\x80\x80"])
    char = {"cc": rng.choice(["\x07", "\x1b", "\x7f", "\x85", "\n", "\t", "\x00"]),
            "nul": "\x00", "cf": rng.choice(["\u200b", "\u202e", "\ufeff", "\u00ad"]),
            "co": rng.choice(["\ue000", "\U000f0000"]), "zl": "\u2028", "zp": "\u2029"}[choice]
    return ("a" + char + "b").encode()


def s_bad_text(rng):
    o = base(rng)
    field = rng.choice(["description", "issuer", "currency"])
    r = [x for x in tlvs_of(o) if x[0] != {"description": 10, "issuer": 18, "currency": 6}[field]]
    v = bad_text(rng)
    if field == "currency":
        # A currency is three capital letters, so an invalid one is the length
        # and the bad byte both: also a refusal of the currency's own.
        v = b"U" + v[:1] + b"D"
        r = r + [(8, b"\x01")]
    r = r + [({"description": 10, "issuer": 18, "currency": 6}[field], v)]
    if not any(x[0] == 10 for x in r):
        r.append((10, b"d"))
    return raw(sorted(r, key=lambda x: x[0]))


def s_amount_no_description(rng):
    o = base(rng, description=None, amount=rng.randint(1, 10**6))
    return raw(tlvs_of(o))


def s_amount_zero(rng):
    o = base(rng, amount=1)
    r = [(8, b"") if t == 8 else (t, v) for t, v in tlvs_of(o)]
    return raw(r)


def s_currency_no_amount(rng):
    o = base(rng, currency=rng.choice(sorted(CURRENCIES)), amount=None)
    return raw(tlvs_of(o))


def s_no_issuer(rng):
    o = base(rng, issuer_id=None, paths=None)
    return raw(tlvs_of(o))


def s_path_no_hops(rng):
    o = base(rng, paths=[path(rng)])
    p = dict(o["paths"][0], hops=[])
    return raw(tlvs_of(dict(o, paths=[p])))


def s_empty_paths(rng):
    o = base(rng, paths=[])
    return raw(tlvs_of(o))


def s_bad_point(rng):
    """A point that is not a compressed curve key, in the issuer id or a path."""
    bad = rng.choice([off_curve(rng), b"\x04" + rng.randbytes(32), b"\x00" * 33,
                      b"\x05" + rng.randbytes(32), rng.randbytes(33)])
    if point_ok(bad):
        bad = off_curve(rng)
    where = rng.choice(["issuer_id", "path_key", "hop", "first_node"])
    if where == "issuer_id":
        return raw(tlvs_of(base(rng, issuer_id=bad)))
    p = path(rng)
    if where == "path_key":
        p["path_key"] = bad
    elif where == "hop":
        p["hops"][0] = (bad, p["hops"][0][1])
    else:
        p["first"] = bad if bad[0] in (2, 3) else b"\x02" + bad[1:]
        if point_ok(p["first"]):
            p["first"] = off_curve(rng)
    return raw(tlvs_of(base(rng, paths=[p])))


def s_point_length(rng):
    n = rng.choice([32, 34, 0, 65])
    o = base(rng)
    r = [(22, key(rng)[:n] if n < 33 else key(rng) + bytes(n - 33)) if t == 22 else (t, v)
         for t, v in tlvs_of(o)]
    return raw(r)


def s_chains_empty(rng):
    r = tlvs_of(base(rng, chains=[])) + [(2, b"")]
    return raw(sorted(r, key=lambda x: x[0]))


def s_chains_length(rng):
    r = [x for x in tlvs_of(base(rng)) if x[0] != 2] + [(2, rng.randbytes(rng.choice([1, 31, 33, 63])))]
    return raw(sorted(r, key=lambda x: x[0]))


def s_padding_over_four(rng):
    """A fifth padding bit: a trailing character of zero bits, which makes
    the data end five bits too long to be whole bytes."""
    data = stream(tlvs_of(base(rng)))
    return text_of(data, pad="0" * ((-8 * len(data)) % 5) + "00000")


def s_mixed_case(rng):
    return mixed_case(text_of(stream(tlvs_of(base(rng)))), rng)


def s_plus_edge(rng):
    t = text_of(stream(tlvs_of(base(rng))))
    return rng.choice(["+" + t, t + "+", t + "+ ", "+ " + t])


def s_hrp(rng):
    return text_of(stream(tlvs_of(base(rng))), hrp=rng.choice(["lnr", "lni", "lnx", "ln", "lno2", "bc"]))


def s_not_bech32(rng):
    t = text_of(stream(tlvs_of(base(rng))))
    i = rng.randrange(5, len(t))
    return t[:i] + rng.choice("bio1\x00 !-") + t[i + 1:]


# Faraday only (F232's stricter list).

def s_too_long(rng):
    """Over 4,296 characters, with every field valid: one odd field long enough."""
    r = tlvs_of(base(rng))
    r.append((unknown_type(rng), rng.randbytes(rng.randint(2700, 3000))))
    return raw(sorted(r, key=lambda x: x[0]))


def s_padding_nonzero(rng):
    """Padding bits that are not zero (they carry no data)."""
    data = stream(tlvs_of(base(rng)))
    n = (-8 * len(data)) % 5
    if n == 0:
        data += record(unknown_type(rng), b"")
        n = (-8 * len(data)) % 5
    # A trailing record of bytes keeps the data whole; the padding is the last n bits.
    pad = "".join(rng.choice("01") for _ in range(n))
    if "1" not in pad:
        pad = "1" + pad[1:]
    return text_of(data, pad=pad)


def s_plus_whitespace(rng):
    t = text_of(stream(tlvs_of(base(rng))))
    i = rng.randrange(5, len(t) - 1)
    while t[i - 1] not in CHARSET or t[i] not in CHARSET:
        i = rng.randrange(5, len(t) - 1)
    return t[:i] + "+" + rng.choice(["\x0b", "\x0c", "\u00a0", "\u2003", "\x1c", "\x85"]) + t[i:]


def s_plus_not_bech32(rng):
    t = text_of(stream(tlvs_of(base(rng))))
    return t.replace("lno1", rng.choice(["ln+o1", "lno+1", "l+no+1"]), 1)


def s_chain_twice(rng):
    c = rng.choice(KNOWN_CHAINS)
    chains = [c] + rng.sample(KNOWN_CHAINS, rng.randint(0, 2)) + [c]
    return raw(tlvs_of(base(rng, chains=chains, amount=None, currency=None)))


def s_chains_unknown(rng):
    chains = [rng.randbytes(32) for _ in range(rng.randint(1, 3))]
    return raw(tlvs_of(base(rng, chains=chains, amount=None, currency=None)))


def s_currency(rng):
    """A currency that is not three capital letters of the list."""
    code = rng.choice(["usd", "Usd", "US", "USDX", "AAA", "ZZZ", "XAU", "XXX", "BTC", "U$D", "ÜSD"])
    if code in CURRENCIES:
        code = "ZZZ"
    o = base(rng, currency=code, amount=rng.randint(1, 10**6))
    return raw(tlvs_of(o))


def s_amount_over_21m(rng):
    return raw(tlvs_of(base(rng, currency=None, chains=[], amount=rng.randint(2100000000000000001, 2**64 - 1))))


def s_expiry_late(rng):
    return raw(tlvs_of(base(rng, expiry=rng.choice([253402300800, 2**40, 2**49]))))


def s_first_node_byte(rng):
    p = path(rng)
    p["first"] = bytes([rng.choice([4, 5, 0x10, 0x80, 0xFF])]) + rng.randbytes(32)
    return raw(tlvs_of(base(rng, paths=[p])))


def s_unassigned_char(rng):
    o = base(rng)
    field = rng.choice(["description", "issuer"])
    o[field] = "a" + rng.choice(["\u0378", "\u0ffe", "\U000e0080", "\u0380", "\U00040000"]) + "b"
    return raw(tlvs_of(o))


KINDS = {
    "order": (s_order, "both"), "repeat": (s_repeat, "both"),
    "truncated": (s_truncated, "both"), "length_past_end": (s_length_past_end, "both"),
    "bigsize_nonminimal": (s_bigsize_nonminimal, "both"),
    "tu_leading_zero": (s_tu_leading_zero, "both"), "tu_overlong": (s_tu_overlong, "both"),
    "type_out_of_range": (s_type_out_of_range, "both"), "unknown_even": (s_unknown_even, "both"),
    "features_leading_zero": (s_features_leading_zero, "both"),
    "features_even": (s_features_even, "both"), "bad_text": (s_bad_text, "both"),
    "amount_no_description": (s_amount_no_description, "both"),
    "amount_zero": (s_amount_zero, "both"), "currency_no_amount": (s_currency_no_amount, "both"),
    "no_issuer": (s_no_issuer, "both"), "path_no_hops": (s_path_no_hops, "both"),
    "empty_paths": (s_empty_paths, "both"), "bad_point": (s_bad_point, "both"),
    "point_length": (s_point_length, "both"), "chains_empty": (s_chains_empty, "both"),
    "chains_length": (s_chains_length, "both"), "padding_over_four": (s_padding_over_four, "both"),
    "mixed_case": (s_mixed_case, "both"), "plus_edge": (s_plus_edge, "both"),
    "hrp": (s_hrp, "both"), "not_bech32": (s_not_bech32, "both"),
    "too_long": (s_too_long, "faraday"), "padding_nonzero": (s_padding_nonzero, "faraday"),
    "plus_whitespace": (s_plus_whitespace, "faraday"),
    "plus_not_bech32": (s_plus_not_bech32, "faraday"), "chain_twice": (s_chain_twice, "faraday"),
    "chains_unknown": (s_chains_unknown, "faraday"), "currency": (s_currency, "faraday"),
    "amount_over_21m": (s_amount_over_21m, "faraday"), "expiry_late": (s_expiry_late, "faraday"),
    "first_node_byte": (s_first_node_byte, "faraday"),
    "unassigned_char": (s_unassigned_char, "faraday"),
}


def expected_view(o):
    """What an offer says, in the shape of the area's comparison, from the
    writer's own record (for a valid offer)."""
    amount = None
    if o["amount"] is not None:
        if o["currency"] is None:
            amount = {"msat": o["amount"]}
        else:
            amount = {"currency": o["currency"], "exponent": CURRENCIES[o["currency"]],
                      "units": o["amount"]}
    paths = []
    for p in o["paths"] or []:
        f = p["first"]
        if f[0] in (2, 3):
            paths.append({"node": f.hex(), "hops": len(p["hops"])})
        else:
            paths.append({"scid": int.from_bytes(f[1:], "big"), "direction": f[0],
                          "hops": len(p["hops"])})
    return {
        "chains": [c.hex() for c in o["chains"]],
        "metadata": None if o["metadata"] is None else o["metadata"].hex(),
        "amount": amount,
        "description": o["description"],
        "features": sorted(o["features"]),
        "expiry": o["expiry"],
        "paths": paths,
        "issuer": o["issuer"],
        "quantity_max": o["quantity_max"],
        "issuer_id": None if o["issuer_id"] is None else o["issuer_id"].hex(),
        "unknown": [{"type": t, "data": v.hex()} for t, v in sorted(o["unknown"])],
    }


def damaged(t, rng):
    """One Bech32 character of the data part changed to another one, or a
    character removed or added (the TLV or the padding then breaks, or the
    offer says something else)."""
    i = rng.randrange(4, len(t))
    mode = rng.choice(["change", "drop", "add"])
    if mode == "change":
        c = rng.choice([c for c in CHARSET if c != t[i].lower()])
        return t[:i] + c + t[i + 1:]
    if mode == "drop":
        return t[:i] + t[i + 1:]
    return t[:i] + rng.choice(CHARSET) + t[i:]

