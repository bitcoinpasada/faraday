"""A BOLT #11 invoice writer for the `lightning` area: random invoices of
every field kind, and the same invoices spoiled in one way each. Written
here from BOLT #11's text (Bech32, the 35-bit timestamp, the tagged fields,
the 65-byte recoverable signature over SHA-256 of the human-readable part
and the zero-padded data), signing with btclib on its own curve
(`bip32_ref` sets that up). It shares nothing with Faraday's decoder or
the `bolt11` package it is compared with.

An invoice is a dict: `net` (the human-readable part's network letters),
`amount` (millisatoshis or None), `amount_text` (overrides the amount's
spelling), `ts`, `fields` (a list of (tag character, 5-bit values)), `key`
(the signing scalar) and `sign` (how to spoil the signature, or None).
"""

import bip32_ref  # noqa: F401 - btclib on its own curve, before ecc is imported
from ecc.curves import bytes_from_point, mult, secp256k1
from ecc.ecc import dsa

CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
NETWORKS = ["bc", "tb", "tbs", "bcrt"]
# The even feature bits Faraday knows how to require.
KNOWN_REQUIRED = [0, 8, 12, 14, 16, 24, 44, 48]
# Tag characters BOLT #11 does not define.
UNDEFINED_TAGS = "qzy8g2tvw03j54ke6ua7l"
N = secp256k1.n


def _polymod(values):
    gen = [0x3B6A57B2, 0x26508E6D, 0x1EA119FA, 0x3D4233DD, 0x2A1462B3]
    chk = 1
    for v in values:
        top = chk >> 25
        chk = (chk & 0x1FFFFFF) << 5 ^ v
        for i in range(5):
            chk ^= gen[i] if (top >> i) & 1 else 0
    return chk


def bech32(hrp, values):
    expanded = [ord(c) >> 5 for c in hrp] + [0] + [ord(c) & 31 for c in hrp]
    mod = _polymod(expanded + values + [0] * 6) ^ 1
    checksum = [(mod >> 5 * (5 - i)) & 31 for i in range(6)]
    return hrp + "1" + "".join(CHARSET[v] for v in values + checksum)


def to5(data):
    """Bytes as 5-bit values, zero padded."""
    bits = "".join(f"{b:08b}" for b in data)
    bits += "0" * (-len(bits) % 5)
    return [int(bits[i:i + 5], 2) for i in range(0, len(bits), 5)]


def from5_bytes(values):
    """5-bit values as whole bytes, the padding dropped."""
    bits = "".join(f"{v:05b}" for v in values)
    return bytes(int(bits[i:i + 8], 2) for i in range(0, len(bits) - len(bits) % 8, 8))


def num5(n):
    """A number as the fewest 5-bit values (none for 0)."""
    out = []
    while n:
        out.append(n & 31)
        n >>= 5
    return out[::-1]


def field(tag, values):
    return [CHARSET.index(tag), len(values) >> 5, len(values) & 31] + values


def amount_text(msat, rng):
    """An amount's text: any multiplier that leaves a whole number and
    puts no digit past a millisatoshi, without a leading zero."""
    pico = msat * 10
    options = [(str(pico), "p")]
    for unit, div in (("n", 10**3), ("u", 10**6), ("m", 10**9), ("", 10**12)):
        if pico % div == 0:
            options.append((str(pico // div), unit))
    digits, unit = rng.choice(options)
    return digits + unit


def features_values(bits):
    """Feature bits as a field: bit n is the n-th from the end."""
    length = max(bits) // 5 + 1
    n = sum(1 << b for b in bits)
    return [(n >> 5 * (length - 1 - i)) & 31 for i in range(length)]


def public_key(key):
    return bytes_from_point(mult(key), secp256k1, True)


def text_of(inv, rng=None):
    """The invoice as text, signed with `inv["key"]`."""
    hrp = "ln" + inv["net"] + (inv.get("amount_text") or "")
    ts = [(inv["ts"] >> 5 * (6 - i)) & 31 for i in range(7)]
    data = ts + [v for f in inv["fields"] for v in f]
    bits = "".join(f"{v:05b}" for v in data)
    bits += "0" * (-len(bits) % 8)
    signed = hrp.encode() + bytes(int(bits[i:i + 8], 2) for i in range(0, len(bits), 8))
    sig, key_id = dsa.sign_recoverable(signed, inv["key"], lower_s=True)
    r, s = sig.r, sig.s
    if inv.get("sign") == "high_s":
        s, key_id = N - s, key_id ^ 1
    raw = r.to_bytes(32, "big") + s.to_bytes(32, "big") + bytes([key_id])
    return bech32(hrp, data + to5(raw))


def random_text(rng):
    pools = ["abcdefghij KLMNOP 0123456789 .,:-", "éèüñ ßç", "コーヒー 一杯",
             "\U0001f680☕", "\u0000\u007f\n\t"]
    pool = "".join(rng.sample(pools, rng.randint(1, 3)))
    out = ""
    for _ in range(rng.choice([0, 1, 5, 20, 100, 200])):
        c = rng.choice(pool)
        if len((out + c).encode()) > 500:  # a field's length is two 5-bit values
            break
        out += c
    return out


def random_fallback(rng):
    kind = rng.choice(["p2pkh", "p2sh", "v0_20", "v0_32", "v1", "vn", "unknown"])
    if kind == "p2pkh":
        return [17] + to5(rng.randbytes(20))
    if kind == "p2sh":
        return [18] + to5(rng.randbytes(20))
    if kind == "v0_20":
        return [0] + to5(rng.randbytes(20))
    if kind == "v0_32":
        return [0] + to5(rng.randbytes(32))
    if kind == "v1":
        return [1] + to5(rng.randbytes(32))
    if kind == "vn":
        return [rng.randint(2, 16)] + to5(rng.randbytes(rng.randint(2, 40)))
    return [rng.randint(19, 31)] + to5(rng.randbytes(rng.randint(0, 30)))


def random_hop(rng):
    return public_key(rng.randrange(1, N)) + (
        rng.getrandbits(64).to_bytes(8, "big") + rng.getrandbits(32).to_bytes(4, "big")
        + rng.getrandbits(32).to_bytes(4, "big") + rng.getrandbits(16).to_bytes(2, "big"))


def random_invoice(rng):
    """An invoice with a random choice of every field kind. Route hints
    stay at three hops or fewer, the `bolt11` package drops the last hop
    of a hint of a multiple of five."""
    key = rng.randrange(1, N)
    fields = [("p", to5(rng.randbytes(32))), ("s", to5(rng.randbytes(32)))]
    if rng.random() < 0.7:
        fields.append(("d", to5(random_text(rng).encode())))
    else:
        fields.append(("h", to5(rng.randbytes(32))))
    if rng.random() < 0.3:
        fields.append(("n", to5(public_key(key))))
    if rng.random() < 0.4:
        # The package cannot read an empty field, so a valid invoice never states 0.
        fields.append(("x", num5(rng.getrandbits(rng.randint(1, 40)) or 1)))
    if rng.random() < 0.4:
        fields.append(("c", num5(rng.getrandbits(rng.randint(1, 16)) or 1)))
    for _ in range(rng.choice([0, 0, 1, 2])):
        fields.append(("f", random_fallback(rng)))
    for _ in range(rng.choice([0, 0, 1, 2])):
        hops = b"".join(random_hop(rng) for _ in range(rng.randint(1, 3)))
        fields.append(("r", to5(hops)))
    if rng.random() < 0.5:
        bits = set(rng.sample(KNOWN_REQUIRED, rng.randint(0, 3)))
        bits |= {rng.randrange(1, 200) | 1 for _ in range(rng.randint(0, 4))}
        # The package keeps one state per feature: both bits of a pair read as supported.
        bits = {b for b in bits if b % 2 == 0 or b - 1 not in bits}
        if bits:
            fields.append(("9", features_values(bits)))
    if rng.random() < 0.25:
        fields.append(("m", to5(rng.randbytes(rng.randint(0, 40)))))
    if rng.random() < 0.2:
        fields.append((rng.choice(UNDEFINED_TAGS), [rng.randrange(32) for _ in range(rng.randint(0, 30))]))
    rng.shuffle(fields)
    inv = {"net": rng.choice(NETWORKS), "amount_text": None, "ts": rng.getrandbits(rng.choice([20, 33, 35])),
           "fields": [field(t, v) for t, v in fields], "tagged": fields, "key": key, "sign": None}
    if rng.random() < 0.7:
        inv["amount_text"] = amount_text(rng.randrange(1, 10 ** rng.randint(1, 17)), rng)
    return inv


def replaced(inv, **changes):
    out = dict(inv, **changes)
    out["fields"] = [field(t, v) for t, v in out["tagged"]]
    return out


def without(inv, tags):
    return replaced(inv, tagged=[f for f in inv["tagged"] if f[0] not in tags])


# Each spoiling: one thing wrong with a signed invoice. Faraday refuses
# every one but the high-S signature without an `n` field (a key recovered
# from it is the same key); the `bolt11` package refuses some and accepts
# others (the area says which are Faraday's stricter rules).
def spoil(inv, kind, rng):
    tagged = list(inv["tagged"])
    if kind == "repeat_p":
        return replaced(inv, tagged=tagged + [("p", to5(rng.randbytes(32)))])
    if kind == "repeat_x":
        return replaced(inv, tagged=tagged + [("x", num5(60)), ("x", num5(61))])
    if kind in ("short_p", "short_s", "short_h", "short_n"):
        tag = kind[-1]
        return replaced(inv, tagged=without(inv, tag)["tagged"] + [(tag, to5(rng.randbytes(31)))])
    if kind == "no_p":
        return without(inv, "p")
    if kind == "no_s":
        return without(inv, "s")
    if kind == "both_d_h":
        return replaced(inv, tagged=without(inv, "dh")["tagged"] + [("d", to5(b"x")), ("h", to5(rng.randbytes(32)))])
    if kind == "no_d_h":
        return without(inv, "dh")
    if kind == "zero_field":
        tag = rng.choice("xc9")
        rest = without(inv, tag)["tagged"]
        values = features_values({rng.choice([8, 14, 99])}) if tag == "9" else num5(rng.randrange(0, 10**6))
        return replaced(inv, tagged=rest + [(tag, [0] + values)])
    if kind == "required_feature":
        bit = rng.choice([b for b in range(2, 120, 2) if b not in KNOWN_REQUIRED])
        return replaced(without(inv, "9"), tagged=without(inv, "9")["tagged"] + [("9", features_values({bit}))])
    if kind == "leading_zero":
        return replaced(inv, amount_text="0" + (amount_text(rng.randrange(1, 10**6), rng)))
    if kind == "sub_msat":
        return replaced(inv, amount_text=str(rng.randrange(1, 10**8) * 10 + rng.randint(1, 9)) + "p")
    if kind == "bad_multiplier":
        return replaced(inv, amount_text=str(rng.randrange(1, 10**6)) + "k")
    if kind == "unknown_network":
        return replaced(inv, net="ltc")
    if kind == "high_s":
        return dict(inv, sign="high_s")
    if kind == "cut_field":
        out = replaced(inv)
        out["fields"] = out["fields"] + [[CHARSET.index("m"), 0, 31] + to5(rng.randbytes(10))]
        return out
    if kind == "too_long":
        return replaced(without(inv, "dh"), tagged=without(inv, "dh")["tagged"]
                        + [("d", to5(b"a" * 600))])
    if kind == "bad_utf8":
        return replaced(without(inv, "dh"), tagged=without(inv, "dh")["tagged"] + [("d", to5(b"ab\xff\xfe"))])
    raise AssertionError(kind)


KINDS = ["repeat_p", "repeat_x", "short_p", "short_s", "short_h", "short_n", "no_p", "no_s", "both_d_h",
         "no_d_h", "required_feature", "leading_zero", "sub_msat", "bad_multiplier", "unknown_network",
         "high_s", "cut_field", "too_long", "bad_utf8", "zero_field"]


def damaged(text, rng):
    """The text with one character changed: the Bech32 checksum fails."""
    at = rng.randrange(text.rindex("1") + 1, len(text))
    other = rng.choice([c for c in CHARSET if c != text[at]])
    return text[:at] + other + text[at + 1:]


def mixed_case(text, rng):
    """One letter in upper case among lower case: valid Bech32 with mixed case refused."""
    letters = [i for i, c in enumerate(text) if c.isalpha()]
    at = rng.choice(letters)
    return text[:at] + text[at].upper() + text[at + 1:]
