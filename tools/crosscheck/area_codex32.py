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
"""

from common import Area, use_reference, vectors

use_reference("codex32", "shims")
use_reference("codex32")
from codex32 import Codex32String  # noqa: E402
from codex32.bech32 import CHARSET, convertbits, u5_to_chars  # noqa: E402
from codex32.bip93 import encode  # noqa: E402

SEED_BYTES = [16, 20, 24, 28, 32, 64]
ORDER = sorted(c for c in CHARSET if c.isalpha() and c != "s") + sorted(c for c in CHARSET if c.isdigit())


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

    def published(self):
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

    def identifier(self):
        fp = self.randbytes(4)
        self.compare("codex32.identifier_for", {"fingerprint": fp.hex()},
                     lambda: {"identifier": u5_to_chars(convertbits(fp, 8, 5)[:4])})

    def run(self):
        self.published()
        for _ in range(self.cases(1000)):
            self.corpus_case()
            self.identifier()
