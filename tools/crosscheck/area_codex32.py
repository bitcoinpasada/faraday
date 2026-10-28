"""Codex32 (BIP 93) against python-codex32.

The reference decodes, checksums and interpolates. Three rules BIP 93
states that the reference does not enforce are written here from the
BIP's text, and nothing else is:

- the threshold of a new secret is "0" or "2" to "9" (BIP 93, "a single
  digit between 2 and 9, or the digit 0");
- the master seed is 128, 160, 192, 224, 256 or 512 bits, so a string
  whose payload is another length is refused;
- shares are handed out in the order Faraday's split documents, the
  bech32 letters other than `s`, then the digits.

Faraday recovers from exactly `k` shares, never with the secret among
them, and derives a share only from exactly `k` strings at an unused
index; the reference also accepts more strings or a used index. The
corpus asks only what both answer, and Faraday's stricter refusals are
its own tests' business.

Every vector BIP 93 publishes (secrets, sets, alternates, and the 55
invalid strings), then random seeds, sets, splits and corrupted strings.

The booklet's worksheets (`docs/CODEX32.md` §7, §13.5) are checked the
same way. The reference has no worksheet, so its rows are written here
from BIP 93's polymod alone: a sum row is the residue over every
character up to the row's last box, a lookup row the residue of the sum
row's leading characters carried a checksum's length on from nothing,
and the ladder steps two boxes at a time (one for the first step where
the characters after the first row are odd in number). A fresh set's
initial shares are checksummed and the rest interpolated by the
reference (`interpolate_at`); translation and recovery symbols are its
`bech32_lagrange`. Their published vectors: volvelle-website's cells
(`codex32-volvelle/`), the booklet's checksum, derivation, recovery,
addition, translation and fusion tables (`codex32-booklet/`), and the
Codex32QR profile's sets (`codex32qr/`). Then random fresh sets of
every threshold up to 31 strings at 48, 74 and 127 characters, their
sheets, the sheets of corrupted strings, checksums made on a sheet, and
translations.
"""

from common import Area, use_reference, vectors

use_reference("codex32", "shims")
use_reference("codex32")
from codex32 import Codex32String  # noqa: E402
from codex32.bech32 import CHARSET, bech32_hrp_expand, convertbits, u5_parse, u5_to_chars  # noqa: E402
from codex32.bip93 import bech32_lagrange, bech32_mul, codex32_interpolate, encode  # noqa: E402
from codex32.checksums import CODEX32, CODEX32_LONG  # noqa: E402

SEED_BYTES = [16, 20, 24, 28, 32, 64]
ORDER = sorted(c for c in CHARSET if c.isalpha() and c != "s") + sorted(c for c in CHARSET if c.isdigit())
LENGTHS = [48, 54, 61, 67, 74, 127]
FRESH = {48: 26, 74: 52, 127: 103}
GLYPHS = "×ℵαβΓΔεηΘΛμΞΠρΣΦΨΩ@#%¢¥€¤⊕†‡§¶♦♥"


def u5(text):
    """Symbols of bech32 text, either case."""
    out = [CHARSET.find(c) for c in text.lower()]
    if -1 in out:
        raise ValueError("not bech32")
    return out


def digits(residue, n):
    return "".join(CHARSET[(residue >> (5 * (n - 1 - i))) & 31] for i in range(n))


def spec_for(chars):
    """BIP 93: the long checksum for a data part of 96 characters or more."""
    return CODEX32_LONG if chars - 3 >= 96 else CODEX32


def ref_worksheet(s):
    if len(s) not in LENGTHS or not s.lower().startswith("ms1"):
        raise ValueError("not a codex32 length")
    data = u5(s[3:])
    spec = spec_for(len(s))
    n, cs = len(data), spec.cs_len
    hrp = bech32_hrp_expand("ms")
    odd = (n - cs) % 2
    steps = (n - cs + 1) // 2
    start = [2 * j - 1 if odd and j else 2 * j for j in range(steps + 1)]
    widths = [1 if odd and j == 0 else 2 for j in range(steps)]
    sums = [digits(spec.polymod(hrp + data[:start[j] + cs]), cs) for j in range(steps + 1)]
    lookups = [digits(spec.polymod(u5(sums[j][:widths[j]]) + [0] * cs, residue=0), cs)
               for j in range(steps)]
    return {"hrp": digits(spec.polymod(hrp + [0] * cs), cs), "sums": sums, "lookups": lookups,
            "widths": widths, "holds": spec.polymod(hrp + data) == spec.const}


def ref_make(prefix, chars):
    if chars not in LENGTHS or not prefix.lower().startswith("ms1"):
        raise ValueError("not a codex32 length")
    spec = spec_for(chars)
    data = u5(prefix[3:])
    if len(data) + 3 + spec.cs_len != chars:
        raise ValueError("the boxes before the checksum")
    checksum = spec.create(bech32_hrp_expand("ms") + data)
    return {"string": "ms1" + u5_to_chars(data + checksum), "holds": True}


def ref_fresh(k, n, ident, chars, data):
    if k not in (0, 2, 3, 4, 5, 6, 7, 8, 9):
        raise ValueError("BIP 93: the threshold is 0 or 2 to 9")
    if (k == 0 and n != 1) or (k and not k <= n <= 31):
        raise ValueError("one string alone, or k to 31")
    if len(ident) != 4 or any(c not in CHARSET for c in ident.lower()):
        raise ValueError("four bech32 characters")
    if chars not in FRESH:
        raise ValueError("made at 48, 74 or 127 characters")
    initial = ["s"] if k == 0 else ORDER[:k]
    if len(data) != len(initial) or any(len(d) != FRESH[chars] for d in data):
        raise ValueError("one payload per initial share")
    made = [Codex32String.from_unchecksummed_string(f"ms1{k}{ident.lower()}{i}{d.lower()}")
            for i, d in zip(initial, data)]
    if k == 0:
        return {"strings": [made[0].s.lower()], "secret": made[0].s.lower()}
    derived = [Codex32String.interpolate_at(made, i) for i in ORDER[k:n]]
    return {"strings": [m.s.lower() for m in made + derived],
            "secret": Codex32String.interpolate_at(made, "s").s.lower()}


def ref_symbols(indices, target):
    xs = u5(indices)
    x = u5(target)[0]
    if not 1 <= len(xs) <= 9 or len(set(xs)) != len(xs) or x in xs:
        raise ValueError("distinct inputs and a new target")
    pairs = [[0 if i == j else bech32_lagrange([a, b], x)[0] for j, b in enumerate(xs)]
             for i, a in enumerate(xs)]
    return {"symbols": bech32_lagrange(xs, x), "pairs": pairs}


def ref_translate(strings, target):
    got = [Codex32String.from_string("ms", s) for s in strings]
    k = int(got[0].k)
    idx = [g.share_idx.lower() for g in got]
    if k == 0 or len(got) != k or target in idx or len(set(idx)) != k:
        raise ValueError("exactly k strings and a new index")
    if any((g.k, g.ident, len(g.s)) != (got[0].k, got[0].ident, len(got[0].s)) for g in got):
        raise ValueError("one set")
    rows = [u5_parse(s.lower())[1] for s in strings]
    w = bech32_lagrange([r[5] for r in rows], CHARSET.find(target))
    return {"translated": [u5_to_chars([bech32_mul(w[i], v) for v in r]) for i, r in enumerate(rows)],
            "sum": "ms1" + u5_to_chars(codex32_interpolate(rows, CHARSET.find(target)))}


def ref_table_row(pair):
    return {"row": digits(CODEX32.polymod(u5(pair) + [0] * 13, residue=0), 13)}


def describe(c):
    if len(c.data) not in SEED_BYTES:
        raise ValueError("BIP 93: the master seed is 128 to 256 bits in steps of 32, or 512")
    return {"string": c.s.lower(), "threshold": int(c.k), "identifier": c.ident.lower(),
            "index": c.share_idx.lower(), "seed": c.data.hex() if c.share_idx.lower() == "s" else None}


def ref_from_seed(k, ident, seed):
    if k not in (0, 2, 3, 4, 5, 6, 7, 8, 9):
        raise ValueError("BIP 93: the threshold is 0 or 2 to 9")
    if len(seed) not in SEED_BYTES:
        raise ValueError("BIP 93: seed size")
    return describe(Codex32String(encode("ms", f"{k}{ident}s", seed, pad_val=0)))


def ref_split(secret, k, n, fresh):
    if not 2 <= k <= 9 or not k <= n <= 31:
        raise ValueError("BIP 93: k is 2 to 9, and at most 31 shares")
    base = Codex32String.from_string("ms", secret)
    if base.share_idx.lower() != "s":
        raise ValueError("only the secret is split")
    s0 = Codex32String(encode("ms", f"{k}{base.ident.lower()}s", base.data, pad_val=0))
    plen = len(s0.payload)
    if len(fresh) < (k - 1) * plen:
        raise ValueError("not enough randomness")
    made = [s0]
    for i in range(k - 1):
        chunk = fresh[i * plen:(i + 1) * plen]
        payload = "".join(CHARSET[b & 31] for b in chunk)
        made.append(Codex32String.from_unchecksummed_string(
            f"ms1{k}{base.ident.lower()}{ORDER[i]}{payload}"))
    return {"shares": [Codex32String.interpolate_at(made, idx).s.lower() for idx in ORDER[:n]]}


class Codex32(Area):
    name = "codex32"

    def parse(self, s, expected=None):
        return self.compare("codex32.parse", {"string": s}, lambda: describe(Codex32String.from_string("ms", s)), expected)

    def from_seed(self, k, ident, seed):
        return self.compare("codex32.from_seed", {"threshold": k, "identifier": ident, "seed": seed.hex()},
                            lambda: ref_from_seed(k, ident, seed))

    def recover(self, shares, expected=None):
        def ref():
            got = [Codex32String.from_string("ms", s) for s in shares]
            if len(got) != int(got[0].k) or any(g.share_idx.lower() == "s" for g in got):
                raise ValueError("exactly k shares, the secret not among them")
            return describe(Codex32String.interpolate_at(got, "s"))
        return self.compare("codex32.recover", {"shares": shares}, ref, expected)

    def derive(self, strings, index, expected=None):
        def ref():
            got = [Codex32String.from_string("ms", s) for s in strings]
            if len(got) != int(got[0].k) or index in [g.share_idx.lower() for g in got] or index == "s":
                raise ValueError("exactly k strings and a new index")
            return describe(Codex32String.interpolate_at(got, index))
        return self.compare("codex32.derive_share", {"strings": strings, "index": index}, ref, expected)

    def split(self, secret, k, n, fresh):
        return self.compare("codex32.split", {"secret": secret, "threshold": k, "shares": n,
                                              "fresh": fresh.hex()},
                            lambda: ref_split(secret, k, n, fresh))

    def worksheet(self, s, expected=None):
        return self.compare("codex32.worksheet", {"string": s}, lambda: ref_worksheet(s), expected)

    def make(self, prefix, chars):
        return self.compare("codex32.make_checksum", {"prefix": prefix, "chars": chars},
                            lambda: ref_make(prefix, chars))

    def fresh(self, k, n, ident, chars, data, expected=None):
        return self.compare("codex32.fresh", {"threshold": k, "shares": n, "identifier": ident,
                                              "chars": chars, "data": data},
                            lambda: ref_fresh(k, n, ident, chars, data), expected)

    def symbols(self, indices, target):
        return self.compare("codex32.symbols", {"indices": indices, "target": target},
                            lambda: ref_symbols(indices, target))

    def translate(self, strings, target):
        return self.compare("codex32.translate", {"strings": strings, "target": target},
                            lambda: ref_translate(strings, target))

    def volvelle(self):
        """volvelle-website's sheets, cell for cell."""
        for sheet in vectors("codex32-volvelle", "cells.json")["sheets"]:
            rows = [r.lower() for r in sheet["rows"]]
            steps = (len(rows) - 3) // 2
            want = {"hrp": rows[1], "sums": [rows[2 + 2 * j][:13] for j in range(steps)] + [rows[-1]],
                    "lookups": [rows[3 + 2 * j] for j in range(steps)], "widths": [2] * steps,
                    "holds": True}
            self.worksheet(sheet["string"], ("ok", want))

    def booklet(self):
        """The booklet's tables: the checksum table, every derivation
        table and the recovery table through Faraday, and its addition,
        translation and fusion tables against the reference's own
        arithmetic (Faraday's GF(32) is not public)."""
        t = vectors("codex32-booklet", "tables.json")
        for pair, row in t["checksum"].items():
            self.compare("codex32.checksum_table", {"pair": pair}, lambda: ref_table_row(pair),
                         ("ok", {"row": row.lower()}))
        for d in t["derivation"]:
            for col, target in enumerate(d["columns"].lower()):
                ans = self.need(self.symbols(d["initial"].lower(), target), "derivation")
                got = "".join(GLYPHS[v] for v in ans["symbols"])
                want = "".join(d["rows"][r][col] for r in d["initial"])
                if got != want:
                    raise AssertionError(f"derivation k = {d['k']} {d['initial']} to {target}: {got}, booklet {want}")
        rec = t["recovery"]
        for other, cells in rec["rows"].items():
            for own, cell in zip(rec["columns"], cells):
                if own == other:
                    continue
                ans = self.need(self.symbols((own + other).lower(), "s"), "recovery")
                if GLYPHS[ans["pairs"][0][1]] != cell:
                    raise AssertionError(f"recovery {own} {other}: booklet {cell}")
        for row, cells in t["addition"]["rows"].items():
            for col, cell in zip(t["addition"]["columns"], cells):
                if CHARSET[u5(row)[0] ^ u5(col)[0]] != cell.lower():
                    raise AssertionError(f"addition {row} {col}")
        for row, cells in t["translation"]["rows"].items():
            for col, cell in zip(t["translation"]["columns"], cells):
                if CHARSET[bech32_mul(u5(row)[0], GLYPHS.index(col))] != cell.lower():
                    raise AssertionError(f"translation {row} {col}")
        for row, cells in t["fusion"]["rows"].items():
            for col, cell in zip(t["fusion"]["columns"], cells):
                if GLYPHS[bech32_mul(GLYPHS.index(row), GLYPHS.index(col))] != cell:
                    raise AssertionError(f"fusion {row} {col}")

    def fresh_sets(self):
        """BIP 93's vectors 1 to 3 and the Codex32QR profile's sets are
        made from initial shares: the set made from their data is theirs."""
        sets = []
        for c in vectors("bip93", "vectors.json")["sets"]:
            sets.append((c["shares"] + list(c["derived"].values()), c["secret"]))
        sets.append(([], "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw"))
        for v in vectors("codex32qr", "vectors.json")["vectors"]:
            shares = list(v["shares"].values())
            for s in shares + [v["secret"]]:
                self.parse(s)
            if shares:
                ans = self.recover(shares)
                if ans[1]["string"] != v["secret"].lower():
                    raise AssertionError(f"Codex32QR vector {v['vector']}: {ans[1]['string']}")
                sets.append((shares, v["secret"]))
        for strings, secret in sets:
            strings = sorted((s.lower() for s in strings), key=lambda s: ORDER.index(s[8]))
            k = int(secret[3])
            if not strings:
                strings = [secret.lower()]
            want = {"strings": strings, "secret": secret.lower()}
            data = [s[9:9 + FRESH[len(s)]] for s in strings[:max(k, 1)]]
            self.fresh(k, len(strings), secret[4:8].lower(), len(secret), data, ("ok", want))

    def published(self):
        self.volvelle()
        self.booklet()
        self.fresh_sets()
        v = vectors("bip93", "vectors.json")
        for c in v["secrets"]:
            ans = self.parse(c["string"])
            if ans[1]["seed"] != c["seed"]:
                raise AssertionError(f"vector {c['vector']}: seed {ans[1]['seed']}")
        for c in v["sets"]:
            for s in c["shares"] + list(c["derived"].values()) + [c["secret"]]:
                self.parse(s)
            k = int(c["shares"][0][3])
            # Each derived share from the published shares, the secret
            # making up the k where the BIP lists fewer shares.
            basis = (c["shares"] + [c["secret"]])[:k]
            for index, want in c["derived"].items():
                ans = self.derive(basis, index.lower())
                if ans[1]["string"] != want.lower():
                    raise AssertionError(f"vector {c['vector']}: derived {ans[1]['string']}")
            ans = self.recover((c["shares"] + list(c["derived"].values()))[:k])
            if ans[1]["seed"] != c["seed"]:
                raise AssertionError(f"vector {c['vector']}: recovered {ans[1]['seed']}")
        for c in v["alternates"]:
            for s in c["strings"]:
                ans = self.parse(s)
                if ans[1]["seed"] != c["seed"]:
                    raise AssertionError(f"alternate {s}: seed {ans[1]['seed']}")
        for group in v["invalid"]:
            for s in group["strings"]:
                self.parse(s, ("err", None))
                self.worksheet(s)
        for c in v["secrets"]:
            self.worksheet(c["string"])

    def corpus_case(self):
        rng = self.rng
        seed = self.randbytes(rng.choice(SEED_BYTES))
        ident = "".join(rng.choice(CHARSET) for _ in range(4))
        k = rng.choice([2, 2, 3, 4, rng.randint(2, 9)])
        self.from_seed(rng.choice([0, k]), ident, seed)
        if rng.random() < 0.05:
            self.from_seed(1, ident, seed)
        secret = self.from_seed(k, ident, seed)[1]["string"]
        n = rng.randint(k, rng.choice([k, 9, 31]))
        plen = len(Codex32String(secret).payload)
        shares = self.split(secret, k, n, self.randbytes((k - 1) * plen + rng.randint(0, 3)))[1]["shares"]
        chosen = rng.sample(shares, k)
        got = self.recover(chosen)
        if got[1]["seed"] != seed.hex():
            raise AssertionError("the shares do not recover the seed they split")
        used = {s[8] for s in chosen}
        fresh = [c for c in ORDER if c not in used]
        self.derive(chosen, rng.choice(fresh))
        self.parse(rng.choice(shares).upper())
        self.recover(chosen[:-1])
        # One character changed, one character's case changed.
        s = rng.choice(shares)
        at = rng.randrange(3, len(s))
        self.parse(s[:at] + rng.choice([c for c in CHARSET + "bio1" if c != s[at]]) + s[at + 1:])
        self.parse(s[:at] + s[at].upper() + s[at + 1:])
        # A share of another set (another identifier) in the set.
        other = self.from_seed(k, "".join(rng.choice(CHARSET) for _ in range(4)), seed)[1]["string"]
        self.recover(chosen[:-1] + [other])

    def sheets_case(self):
        """A fresh set of any threshold and size, the sheet of one of its
        strings and of that string with a box changed, its checksum made
        on a sheet, the symbols of random indices, and a translation."""
        rng = self.rng
        chars = rng.choice(list(FRESH))
        k = rng.choice([0, 2, 2, 3, 4, rng.randint(2, 9)])
        n = 1 if k == 0 else rng.randint(k, rng.choice([k, 9, 31]))
        if rng.random() < 0.05:
            k, n = rng.choice([(1, 1), (2, 32), (3, 2), (10, 10), (0, 2)])
        ident = "".join(rng.choice(CHARSET) for _ in range(4))
        data = ["".join(rng.choice(CHARSET) for _ in range(FRESH[chars])) for _ in range(max(k, 1))]
        ans = self.fresh(k, n, ident, chars, data)
        if ans[0] != "ok":
            return
        strings = ans[1]["strings"]
        s = rng.choice(strings + [ans[1]["secret"]])
        self.worksheet(s)
        at = rng.randrange(3, len(s))
        self.worksheet(s[:at] + rng.choice([c for c in CHARSET if c != s[at]]) + s[at + 1:])
        self.make(s[:len(s) - spec_for(len(s)).cs_len], len(s))
        self.symbols("".join(rng.sample(CHARSET, rng.randint(1, 9))), rng.choice(CHARSET))
        if k >= 2:
            chosen = rng.sample(strings, k)
            used = {c[8] for c in chosen}
            targets = [c for c in ORDER + ["s"] if c not in used]
            if rng.random() < 0.05:
                targets = [chosen[0][8]]
            self.translate(chosen, rng.choice(targets))

    def identifier(self):
        fp = self.randbytes(4)
        self.compare("codex32.identifier_for", {"fingerprint": fp.hex()},
                     lambda: {"identifier": u5_to_chars(convertbits(fp, 8, 5)[:4])})

    LOOPS = (("corpus", 1000),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
        self.sheets_case()
        self.identifier()
