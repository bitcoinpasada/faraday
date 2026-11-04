"""TOTP codes (`faraday_vault::totp`) against pyotp 2.9 (python3-pyotp).

- RFC 6238 Appendix B's vectors (SHA-1, SHA-256 and SHA-512, 8 digits, the
  six times it lists), as Faraday and pyotp each work them out from the
  RFC's seeds.
- Random keys of 1 to 300 bytes, in `otpauth://totp/` URIs and, from
  10 bytes (16 Base32 characters), as bare keys; digits 6 to 8, periods
  from 1 to 3600, SHA-1, SHA-256 and SHA-512; times from 0 to 2^64 - 1,
  the edges of each period (the last second of one, the first of the next)
  among them: the code, the seconds left and the period.
  `pyotp.TOTP.at` is asked until the end of year 9999 (the last Unix time
  Python's `datetime` holds) with an aware UTC time, and
  `generate_otp(time // interval)` after it.
- `pyotp.parse_uri` over those URIs and over the same URIs damaged
  (a character changed, added or removed, cut, a pair dropped or doubled,
  the type or the parameter names changed): the same key, digits, period
  and hash, or both refuse. pyotp's key is read when a code is asked for,
  so the area asks for one.
- The differences below pinned one URI at a time, each with the answer of
  each side, so that none can widen.

`from_fields` (KeePass 2's `TimeOtp-*` fields, KeePassXC's `TOTP Seed` and
`TOTP Settings`) and `normalise` (what the entry form stores into field 6)
have no pyotp reader: the crate's tests hold them.
"""

import base64
import datetime
import hashlib
import re

import pyotp

from common import Area, Mismatch, reference

KINDS = {"SHA1": hashlib.sha1, "SHA256": hashlib.sha256, "SHA512": hashlib.sha512}
# The last Unix time `datetime` holds: 9999-12-31T23:59:59Z.
LAST_DATETIME = 253402300799
# The parameter names pyotp's `parse_uri` takes; any other name it refuses.
PYOTP_NAMES = {"secret", "issuer", "algorithm", "encoder", "digits", "period", "counter", "image"}

# RFC 6238 Appendix B: time, then the SHA-1, SHA-256 and SHA-512 codes (8 digits).
RFC_6238 = [
    (59, "94287082", "46119246", "90693936"),
    (1_111_111_109, "07081804", "68084774", "25091201"),
    (1_111_111_111, "14050471", "67062674", "99943326"),
    (1_234_567_890, "89005924", "91819424", "93441116"),
    (2_000_000_000, "69279037", "90698825", "38618901"),
    (20_000_000_000, "65353130", "77737706", "47863826"),
]
RFC_SEEDS = {
    "SHA1": b"12345678901234567890",
    "SHA256": b"12345678901234567890123456789012",
    "SHA512": b"1234567890" * 6 + b"1234",
}

KEY = "JBSWY3DPEHPK3PXP"

# One URI each: what Faraday answers ("ok", "hotp" or "unreadable") and what
# pyotp does ("totp", "hotp", "steam", or "err": it refuses, or cannot work a
# code out of the key). A row that is not the same on both sides is a stated
# difference of the area.
PINNED = [
    (f"otpauth://totp/x?secret={KEY}", "ok", "totp"),
    (f"otpauth://totp/x?secret={KEY}&issuer=X&image=https://x/y.png", "ok", "totp"),
    (f"otpauth://totp/x?secret={KEY}&digits=8&period=3600&algorithm=SHA512", "ok", "totp"),
    (f"otpauth://totp/x?secret={KEY}&digits=5", "unreadable", "err"),
    (f"otpauth://totp/x?secret={KEY}&digits=9", "unreadable", "err"),
    (f"otpauth://totp/x?secret={KEY}&period=0", "unreadable", "err"),
    (f"otpauth://totp/x?secret={KEY}&algorithm=MD5", "unreadable", "err"),
    (f"otpauth://totp/x?secret={KEY}&digits=abc", "unreadable", "err"),
    ("otpauth://totp/x", "unreadable", "err"),
    ("otpauth://totp/x?secret=", "unreadable", "err"),
    ("http://totp/x?secret=" + KEY, "unreadable", "err"),
    (f"otpauth://totp/x?secret={KEY}&secret=GEZDGNBVGY3TQOJQ", "ok", "totp"),
    # HOTP: pyotp reads it as a HOTP; Faraday names it and reads no code.
    (f"otpauth://hotp/x?secret={KEY}&counter=1", "hotp", "hotp"),
    (f"otpauth://hotp/x?secret={KEY}", "hotp", "hotp"),
    # Steam: pyotp's `encoder=steam` is five characters from Steam's alphabet;
    # Faraday reads no code, as it makes none Steam would take.
    # `otpauth://steam/` both refuse.
    (f"otpauth://totp/x?secret={KEY}&encoder=steam", "unreadable", "steam"),
    (f"otpauth://steam/x?secret={KEY}", "unreadable", "err"),
    # Faraday reads the scheme, the type and the algorithm in any case.
    (f"OTPAUTH://TOTP/x?secret={KEY}", "ok", "err"),
    (f"otpauth://totp/x?secret={KEY}&algorithm=sha256", "ok", "err"),
    # Faraday's period is 1 to 3600 seconds.
    (f"otpauth://totp/x?secret={KEY}&period=3601", "unreadable", "totp"),
    # Base32 as services write it: Faraday ignores spaces and `=` anywhere;
    # pyotp pads by itself and takes `=` only where the padding is right.
    (f"otpauth://totp/x?secret={KEY[:8]} {KEY[8:]}", "ok", "err"),
    (f"otpauth://totp/x?secret={KEY}====", "ok", "err"),
    (f"otpauth://totp/x?secret={KEY.lower()}", "ok", "totp"),
    (f"otpauth://totp/x?secret={KEY[:15]}", "ok", "totp"),
    ("otpauth://totp/x?secret=GEZDGNBVA", "ok", "err"),
    ("otpauth://totp/x?secret=G", "unreadable", "err"),
    (f"otpauth://totp/x?secret={KEY[:8]}-{KEY[8:]}", "unreadable", "err"),
    # Faraday reads a value as it is written; pyotp unquotes the whole URI.
    (f"otpauth://totp/x?secret={KEY}&digits=%38", "unreadable", "totp"),
    (f"otpauth://totp/x?secret={KEY[:8]}%20{KEY[8:]}", "unreadable", "err"),
    # Query forms: Faraday refuses a pair without a value or with an empty one,
    # a fragment (it is part of the last value) and a type with no label
    # slash; pyotp skips the pairs, drops the fragment and reads the type.
    (f"otpauth://totp/x?secret={KEY}&flag", "unreadable", "totp"),
    (f"otpauth://totp/x?secret={KEY}&period=", "unreadable", "totp"),
    (f"otpauth://totp/x?secret={KEY}#frag", "unreadable", "totp"),
    (f"otpauth://totp?secret={KEY}", "unreadable", "totp"),
    # pyotp refuses a name it does not know, a label issuer that the
    # `issuer` parameter does not repeat, and `counter` on a TOTP; Faraday
    # ignores what it does not know.
    (f"otpauth://totp/x?secret={KEY}&foo=1", "ok", "err"),
    (f"otpauth://totp/x?secret={KEY}&SECRET={KEY}", "ok", "err"),
    (f"otpauth://totp/A:b?secret={KEY}&issuer=C", "ok", "err"),
    (f"otpauth://totp/x?secret={KEY}&counter=1", "ok", "err"),
]

# Bare keys: Faraday's rule (at least 16 characters once spaces, hyphens and
# `=` are taken out); pyotp has no reader for one, so its `TOTP(key)` is the
# nearest: "ok" is a code at a time, "err" a refusal.
BARE = [
    (KEY, "ok", "ok"),
    (KEY.lower(), "ok", "ok"),
    (KEY[:15], "unreadable", "ok"),
    ("GEZDGNBVGY3TQOJQ", "ok", "ok"),
    ("JBSW Y3DP EHPK 3PXP", "ok", "err"),
    ("JBSW-Y3DP-EHPK-3PXP", "ok", "err"),
    (KEY + "====", "ok", "err"),
    ("password", "unreadable", "ok"),
    ("", "unreadable", "ok"),
]

# What changes a URI so that pyotp reads it as Faraday does: each is a
# difference the area states. Where Faraday reads and pyotp refuses
# (LENIENT) the answers on the changed URI must agree and Faraday must read
# the original as it reads the changed one; where Faraday refuses and pyotp
# reads (STRICT) the answers on the changed URI must agree. Nothing else
# explains a disagreement.
STRICT = {"period over 3600", "no label slash", "pair without a value"}
LENIENT = {"scheme, type or algorithm case", "name pyotp does not know",
           "counter", "label issuer", "base32 spelling"}


def b32(data, pad=True):
    s = base64.b32encode(data).decode()
    return s if pad else s.rstrip("=")


def base32_spelling(value):
    """The key's characters as pyotp wants them: no space or `=`, a length
    that Base32 can have (the last character dropped when it cannot)."""
    clean = re.sub(r"[ =]", "", value)
    if len(clean) % 8 in (1, 3, 6):
        clean = clean[:-1]
    return clean


def changed(uri):
    """`uri` with every difference of the lists above taken out, and which
    differences there were."""
    why = []
    if not re.match(r"(?i)otpauth://", uri):
        # A bare key: spaces, hyphens, `=`.
        clean = base32_spelling(re.sub(r"-", "", uri))
        if clean != uri:
            why.append("base32 spelling")
        return clean, why
    m = re.match(r"(?i)(otpauth://)([^/?]*)(.*)", uri, re.S)
    scheme, kind, rest = "otpauth://", m.group(2).lower(), m.group(3)
    if m.group(1) + m.group(2) != scheme + kind:
        why.append("scheme, type or algorithm case")
    if rest.startswith("?"):
        rest = "/" + rest
        why.append("no label slash")
    path, _, query = rest.partition("?")
    label = path[1:]
    pairs, out = query.split("&") if query else [], []
    issuer = None
    for pair in pairs:
        k, eq, v = pair.partition("=")
        if pair == "":
            out.append(pair)
        elif not eq or not v:
            why.append("pair without a value")
        elif k == "algorithm" and v != v.upper():
            out.append(f"{k}={v.upper()}")
            why.append("scheme, type or algorithm case")
        elif k == "counter":
            why.append("counter")
        elif k not in PYOTP_NAMES:
            why.append("name pyotp does not know")
        elif k == "period" and v.isascii() and v.isdigit() and int(v) > 3600:
            out.append("period=30")
            why.append("period over 3600")
        elif k == "secret":
            s = base32_spelling(v)
            out.append(f"secret={s}")
            if s != v:
                why.append("base32 spelling")
        elif k == "issuer":
            issuer = v
            out.append(pair)
        else:
            out.append(pair)
    if ":" in label and issuer is not None and label.split(":", 1)[0] != issuer:
        out = [p for p in out if not p.startswith("issuer=")]
        why.append("label issuer")
    tail = ("?" + "&".join(out)) if query or out else ""
    return f"{scheme}{kind}{path}{tail}", why


def otp_of(field):
    """pyotp's reading of a field 6 value: a URI by `parse_uri` (a TOTP, with
    its key worked out), a bare key by `TOTP(key)`."""
    if re.match(r"(?i)otpauth://", field):
        otp = pyotp.parse_uri(field)
        if type(otp) is not pyotp.TOTP:
            raise ValueError(f"{type(otp).__name__}, not a TOTP")
    else:
        otp = pyotp.TOTP(field)
    otp.byte_secret()
    if otp.interval <= 0:
        raise ValueError("a period that is not positive")
    return otp


def code_at(otp, t):
    """`at` as far as `datetime` goes, `generate_otp` of the same counter after."""
    if t <= LAST_DATETIME:
        return otp.at(datetime.datetime.fromtimestamp(t, datetime.timezone.utc))
    return otp.generate_otp(t // otp.interval)


def ref_read(field, times):
    otp = otp_of(field)
    return {"codes": [code_at(otp, t) for t in times], "period": otp.interval}


def ref_code(field, t):
    otp = otp_of(field)
    # pyotp's documentation: the seconds left are `interval - time % interval`.
    return {"code": code_at(otp, t), "left": otp.interval - t % otp.interval,
            "period": otp.interval}


def pyotp_kind(field):
    try:
        otp = pyotp.parse_uri(field)
    except Exception:  # noqa: BLE001
        return "err"
    kind = {pyotp.TOTP: "totp", pyotp.HOTP: "hotp"}.get(type(otp), "steam")
    if kind != "totp":
        return kind
    try:
        otp_of(field)
    except Exception:  # noqa: BLE001
        return "err"
    return kind


def agree(got, ref):
    return got[0] == ref[0] and (got[0] == "err" or got[1] == ref[1])


class Totp(Area):
    name = "totp"
    faraday = (
        "`faraday_vault::totp` (`parse`, `Totp::code`, `left` and `period`)"
    )
    reference = (
        "pyotp 2.9 (python3-pyotp): `TOTP.at` and `generate_otp` for the code, "
        "`parse_uri` for a URI's key, digits, period and hash"
    )
    reference_checked = (
        "RFC 6238 Appendix B's codes are required of pyotp as well as of "
        "Faraday, and each pinned URI's answer from each side is stated in the area"
    )
    vectors = (
        "RFC 6238 Appendix B: the codes of the SHA-1, SHA-256 and SHA-512 seeds at "
        "its six times, 8 digits"
    )
    corpus = (
        "random keys of 1 to 300 bytes in `otpauth://totp/` URIs and, from 10 bytes, as bare "
        "keys, digits 6 to 8, periods 1 to 3600 (and past it), the three hashes, times from 0 to "
        "2^64 - 1 with the edges of each period, the code, the seconds left and the period; the "
        "URIs written in other spellings and damaged (a character changed, added, removed, a "
        "cut, a pair dropped or doubled), read by both; the pinned URIs"
    )
    differences = [
        (
            "Where Faraday reads and pyotp refuses: a scheme, type or `algorithm` in other "
            "than lower (upper for the algorithm) case; a parameter name pyotp does not know, a "
            "`counter` on a TOTP, a label issuer that the `issuer` parameter does not repeat; "
            "Base32 with spaces or hyphens (a bare key) or `=` in the wrong place, or a length "
            "that Base32 cannot have. The area asks both about the URI with that difference "
            "taken out and requires the same answer, and Faraday's answer to the original to be "
            "that answer."
        ),
        (
            "Where Faraday refuses and pyotp reads: a period over 3600 seconds (Faraday's rule), "
            "a pair with no `=` or no value, a type with no `/` before its `?`, a value "
            "written percent-encoded (Faraday reads it as it is), a fragment after the last "
            "value, a bare key of fewer than 16 characters, and `encoder=steam`, which pyotp "
            "reads as a Steam code of five characters and Faraday refuses, making no such code. "
            "The area asks both about the URI with a period, pair or type difference taken "
            "out and requires the same answer; the percent escapes, the fragment, the short "
            "key and Steam's code are pinned one URI at a time."
        ),
        (
            "HOTP: pyotp reads `otpauth://hotp/` as a HOTP; Faraday refuses it, naming it HOTP "
            "(`NotTotp::Hotp`) and reading no code."
        ),
        (
            "Times: pyotp works from a `datetime`, which ends in year 9999; past it the area "
            "asks `generate_otp` for the counter, `time // period`."
        ),
    ]
    not_compared = (
        "`from_fields` (KeePass 2's `TimeOtp-*` fields and KeePassXC's `TOTP Seed` and `TOTP "
        "Settings`) and `normalise` (what the entry form writes into field 6), which no pyotp "
        "reader covers: the crate's tests hold them; `grouped`, the display grouping of a code"
    )

    def ask(self, op, field, times):
        """Both sides on one value: `read` (the codes at `times`) or `code`
        (the code, the seconds left and the period at `times[0]`)."""
        req = {"field": field}
        if op == "read":
            req["times"] = times
        else:
            req["time"] = times[0]

        def ref_of(f):
            return reference(lambda: ref_read(f, times) if op == "read"
                             else ref_code(f, times[0]))

        got = self.rust.call(f"totp.{op}", **req)
        ref = ref_of(field)
        self.compared += 1
        if agree(got, ref):
            return got
        if (got == ("err", "unreadable") and ref[0] == "ok"
                and not re.match(r"(?i)otpauth://", field)
                and len(re.sub(r"[\s=-]", "", field)) < 16):
            # A bare key under 16 characters (a cut-off URI is one): Faraday's rule.
            self.known += 1
            return got
        norm, why = changed(field)
        names = set(why)
        if norm != field:
            got_n = self.rust.call(f"totp.{op}", **{**req, "field": norm})
            ok = agree(got_n, ref_of(norm))
            if got[0] == "ok" and ref[0] == "err":
                ok = ok and names & LENIENT and got == got_n
            elif got[0] == "err" and ref[0] == "ok":
                ok = ok and names & STRICT
            else:
                ok = False
            if ok:
                self.known += 1
                return got
        raise Mismatch(self.name, f"totp.{op}", req, got, ref)

    def pinned(self):
        for uri, faraday, other in PINNED:
            got = self.rust.call("totp.read", field=uri, times=[59])
            mine = "ok" if got[0] == "ok" else got[1]
            if mine != faraday or pyotp_kind(uri) != other:
                raise Mismatch(self.name, "totp.read", {"field": uri}, got,
                               ("pinned", faraday, other, pyotp_kind(uri)))
            self.compared += 1
            if (mine == "ok") != (other == "totp"):
                self.known += 1
        for key, faraday, other in BARE:
            got = self.rust.call("totp.read", field=key, times=[59])
            mine = "ok" if got[0] == "ok" else got[1]
            try:
                otp_of(key)
                theirs = "ok"
            except Exception:  # noqa: BLE001
                theirs = "err"
            if mine != faraday or theirs != other:
                raise Mismatch(self.name, "totp.read", {"field": key}, got,
                               ("pinned", faraday, other, theirs))
            self.compared += 1
            if mine != theirs:
                self.known += 1

    def published(self):
        for algorithm, seed in RFC_SEEDS.items():
            col = 1 + list(RFC_SEEDS).index(algorithm)
            times = [row[0] for row in RFC_6238]
            codes = [row[col] for row in RFC_6238]
            uri = f"otpauth://totp/x?secret={b32(seed)}&algorithm={algorithm}&digits=8"
            want = ("ok", {"codes": codes, "period": 30})
            got = self.ask("read", uri, times)
            if got != want or ("ok", ref_read(uri, times)) != want:
                raise Mismatch(self.name, "totp.read", {"field": uri}, got, want,
                               "RFC 6238 Appendix B")
            for t, code in zip(times, codes):
                one = self.ask("code", uri, [t])
                if one != ("ok", {"code": code, "left": 30 - t % 30, "period": 30}):
                    raise Mismatch(self.name, "totp.code", {"field": uri, "time": t}, one,
                                   code, "RFC 6238 Appendix B")

    def times(self, period):
        rng = self.rng
        p = period
        ts = [0, 59, p - 1, p, p + 1, rng.randint(0, 2 ** 32)]
        for _ in range(3):
            k = rng.randint(1, 2 ** rng.randint(1, 34))
            ts += [k * p - 1, k * p, k * p + rng.randint(0, p)]
        if rng.random() < 0.3:
            ts.append(rng.randint(LAST_DATETIME - 2 * p, LAST_DATETIME))
        if rng.random() < 0.3:
            ts.append(rng.choice([LAST_DATETIME + 1, 2 ** 63, 2 ** 64 - 1,
                                  rng.randint(LAST_DATETIME + 1, 2 ** 64 - 1)]))
        ts = [t for t in ts if 0 <= t < 2 ** 64]
        rng.shuffle(ts)
        return ts

    def uri(self):
        """A random well-formed `otpauth://totp/` URI, its key's bytes and its
        period, with now and then one of the spellings the area states."""
        rng = self.rng
        key = self.randbytes(rng.choice([1, 2, 5, 10, 20, 32, 64, 65, 100, 128, 129,
                                         rng.randint(1, 300)]))
        secret = b32(key, pad=rng.random() < 0.5)
        if rng.random() < 0.2:
            secret = secret.lower()
        r = rng.random()
        if r < 0.08:
            secret = " ".join(secret[i:i + 4] for i in range(0, len(secret), 4))
        elif r < 0.12:
            secret += "=" * rng.randint(1, 3)
        params = [("secret", secret)]
        digits = rng.choice([None, 6, 7, 8])
        if rng.random() < 0.05:
            digits = rng.choice([0, 1, 5, 9, 10, 100])
        period = rng.choice([None, 1, 2, 15, 30, 60, 3600, rng.randint(1, 3600)])
        if rng.random() < 0.05:
            period = rng.choice([0, 3601, 86400])
        algorithm = rng.choice([None, "SHA1", "SHA256", "SHA512"])
        if algorithm and rng.random() < 0.05:
            algorithm = rng.choice([algorithm.lower(), "MD5", "SHA-256", "SHA384"])
        for name, value in (("digits", digits), ("period", period), ("algorithm", algorithm)):
            if value is not None:
                params.append((name, str(value)))
        if rng.random() < 0.3:
            params.append(("issuer", "Example"))
        if rng.random() < 0.08:
            params.append(rng.choice([("image", "https://x/y.png"), ("encoder", "steam"),
                                      ("encoder", "other"), ("counter", "5"), ("foo", "bar")]))
        rng.shuffle(params)
        kind = rng.choice(["totp"] * 30 + ["hotp", "Totp", "TOTP"])
        label = rng.choice(["x", "Example:alice", "Example:a%40b", "a b", ""])
        if label == "Example:alice" and ("issuer", "Example") not in params:
            label = "x"
        text = f"otpauth://{kind}/{label}?" + "&".join(f"{k}={v}" for k, v in params)
        return text, period if period and period > 0 else 30

    def damage(self, text):
        rng = self.rng
        alphabet = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-.~:/?&=@!$'()*,;"
        for _ in range(rng.choice([1, 1, 1, 2, 3])):
            i = rng.randrange(len(text) + 1)
            r = rng.random()
            if r < 0.2 and text:
                text = text[:i] + text[i + 1:]
            elif r < 0.45:
                text = text[:i] + rng.choice(alphabet) + text[i:]
            elif r < 0.7 and text:
                i = min(i, len(text) - 1)
                text = text[:i] + rng.choice(alphabet) + text[i + 1:]
            elif r < 0.78:
                text = text[:i]
            elif r < 0.9 and "?" in text:
                head, _, query = text.partition("?")
                pairs = query.split("&")
                j = rng.randrange(len(pairs))
                pairs = pairs[:j] + pairs[j + 1:] if rng.random() < 0.5 else pairs + [pairs[j]]
                text = head + "?" + "&".join(pairs)
            elif "?" in text:
                head, _, query = text.partition("?")
                text = head + "?" + query.replace("=", rng.choice(["", "==", "&"]), 1)
        return text

    def bare(self):
        rng = self.rng
        key = self.randbytes(rng.choice([10, 11, 15, 20, 32, 64, rng.randint(10, 200)]))
        text = b32(key, pad=rng.random() < 0.5)
        if rng.random() < 0.3:
            text = text.lower()
        r = rng.random()
        if r < 0.15:
            text = rng.choice([" ", "-"]).join(text[i:i + 4] for i in range(0, len(text), 4))
        elif r < 0.2:
            text += "=" * rng.randint(1, 4)
        return text

    LOOPS = (("uri", 1500), ("damaged", 1500), ("bare", 300))

    def prelude(self):
        self.published()
        self.pinned()

    def check(self, field, period):
        times = self.times(period)
        self.ask("read", field, times)
        self.ask("code", field, [self.rng.choice(times)])

    def case_uri(self):
        text, period = self.uri()
        self.check(text, period)

    def case_damaged(self):
        text, period = self.uri()
        self.check(self.damage(text), period)

    def case_bare(self):
        self.check(self.bare(), 30)
