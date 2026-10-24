"""The vanity address search (`faraday_bip::vanity`) against a search
written here from EntropyLab's counter as `tools/reference/vanity/README.md`
quotes it, over python-mnemonic (`bip39/`), BIP-32 from `btc.py`, and
the addresses as the `addr` area writes them (Core's `segwit_addr.py`
and Base58Check, BIP-86's tweak).

The search, as the area holds it:

- The counter's digits are `a`-`z`, `A`-`Z`, `0`-`9`, `a` zero, the
  leftmost the most significant (EntropyLab's odometer). Counters 0 to 61
  are one character wide, the next 62² two, and so on up to eight.
- The passphrase dial: candidate k is the key's passphrase followed by
  counter k's text, the seed BIP-39's, the address the first receive
  address of account 0 (`m/purpose'/coin'/0'/0/0`). The account dial:
  candidate k is account k of the key's own seed, up to 2³¹ − 1.
- An address matches when it begins with the prefix, in any case for
  bech32 and exactly for base58. The prefix includes the characters
  every address of the kind starts with (`bc1q`, `bc1p`, `1`, `3`, `tb1q`,
  `2`, `bcrt1p` and so on; none for legacy on a test network, whose first
  character is `m` or `n`); the free characters must be bech32's or
  base58's. The expected work is EntropyLab's `estimateVanityWork`: 32 or
  58 a free character, the test networks' legacy first character 2.
- The search runs from a starting counter for at most a budget of
  candidates and stops at the first match.

- EntropyLab's two finds (the README's §3) on both dials.
- Counters at the edges of each width and random ones; prefixes of every
  kind on every network, valid and not, with their expected work.
- Random keys searched on both dials for prefixes of up to two free
  characters, from random starting counters, with budgets that end
  before a match and after.
"""

import hashlib
import hmac

import btc
from area_addr import P2PKH, P2SH, PURPOSE, byte_to_base58, p2tr_bip86, segwit_addr
from common import Area, use_reference

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
BECH32 = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
BASE58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
NETWORKS = ["mainnet", "testnet", "signet", "regtest"]
HRP = {"mainnet": "bc", "testnet": "tb", "signet": "tb", "regtest": "bcrt"}
MAX_WIDTH = 8
MAX_FREE = 68
U64 = (1 << 64) - 1
H = btc.HARDENED
ENGLISH = Mnemonic("english")


def counter_text(n):
    width, span = 1, 62
    while n >= span:
        n -= span
        width += 1
        if width > MAX_WIDTH:
            raise ValueError("past eight characters")
        span *= 62
    out = ""
    for _ in range(width):
        out = ALPHABET[n % 62] + out
        n //= 62
    return out


def fixed_prefix(script, network):
    main = network == "mainnet"
    if script == 0:
        return "1" if main else ""
    if script == 1:
        return "3" if main else "2"
    return HRP[network] + "1" + ("q" if script == 2 else "p")


def first_free(script, network):
    return "mn" if script == 0 and network != "mainnet" else None


def can_begin(prefix, script, network):
    fixed = fixed_prefix(script, network)
    common = min(len(fixed), len(prefix))
    if prefix[:common].lower() != fixed[:common].lower():
        return False
    free = prefix[common:]
    if not free:
        return True
    first = first_free(script, network)
    if first and free[0] not in first:
        return False
    if len(free) > MAX_FREE:
        return False
    rest = free[1:] if first else free
    if script >= 2:
        return all(c.lower() in BECH32 for c in rest)
    return all(c in BASE58 for c in rest)


def expected(prefix, script, network):
    free = max(0, len(prefix) - len(fixed_prefix(script, network)))
    if not free:
        return 1
    per = 32 if script >= 2 else 58
    first = first_free(script, network)
    total = len(first) * per ** (free - 1) if first else per ** free
    return min(total, U64)


def private_child(chain, k, i):
    """BIP-32's CKDpriv, without the parent fingerprint a serialised key
    needs (a point multiplication this search never uses)."""
    data = b"\0" + k.to_bytes(32, "big") if i >= H else btc.point(k)
    digest = hmac.new(chain, data + i.to_bytes(4, "big"), hashlib.sha512).digest()
    il = int.from_bytes(digest[:32], "big")
    child = (il + k) % btc.N
    if il >= btc.N or child == 0:
        raise btc.Invalid("an invalid child")
    return digest[32:], child


def address(seed, network, script, account):
    test = network != "mainnet"
    m = btc.master(seed, test)
    chain, k = m.chain, m.k
    for i in [PURPOSE[script] + H, (1 if test else 0) + H, account + H, 0, 0]:
        chain, k = private_child(chain, k, i)
    pub = btc.point(k)
    h = btc.hash160(pub)
    if script == 0:
        return byte_to_base58(h, P2PKH[network])
    if script == 1:
        return byte_to_base58(btc.hash160(b"\x00\x14" + h), P2SH[network])
    if script == 2:
        return segwit_addr.encode_segwit_address(HRP[network], 0, h)
    return segwit_addr.encode_segwit_address(HRP[network], 1, p2tr_bip86(pub))


def matches(addr, prefix, script):
    head = addr[:len(prefix)]
    if len(addr) < len(prefix):
        return False
    return head.lower() == prefix.lower() if script >= 2 else head == prefix


def grind(dial, key, base, script, prefix, network, start, budget):
    """`key` is the entropy (passphrase dial) or the seed (account dial)."""
    none = {"tested": 0, "find": None}
    if not can_begin(prefix, script, network):
        return none
    words = ENGLISH.to_mnemonic(key) if dial == "passphrase" else None
    tested = 0
    while tested < budget:
        k = start + tested
        if dial == "passphrase":
            try:
                suffix = counter_text(k)
            except ValueError:
                break
            passphrase = base + suffix.encode()
            if len(passphrase) > 256:
                break
            seed = hashlib.pbkdf2_hmac("sha512", words.encode(), b"mnemonic" + passphrase, 2048)
            account = 0
        else:
            if k >= H:
                break
            suffix, seed, account = "", key, k
        tested += 1
        try:
            addr = address(seed, network, script, account)
        except btc.Invalid:
            continue
        if matches(addr, prefix, script):
            return {"tested": tested, "find": {"counter": k, "suffix": suffix,
                                               "account": account, "address": addr}}
    return {"tested": tested, "find": None}


class Vanity(Area):
    name = "vanity"

    def grind(self, dial, key, base, script, prefix, network, start, budget, expected=None):
        req = {"dial": dial, "base": base.hex(), "script": script, "prefix": prefix,
               "network": network, "from": start, "budget": budget}
        req["entropy" if dial == "passphrase" else "seed"] = key.hex()
        return self.compare("vanity.grind", req,
                            lambda: grind(dial, key, base, script, prefix, network, start, budget),
                            expected)

    def counter(self, n):
        return self.compare("vanity.counter", {"counter": n}, lambda: {"text": counter_text(n)})

    def prefix(self, prefix, script, network):
        return self.compare("vanity.prefix", {"prefix": prefix, "script": script,
                                              "network": network},
                            lambda: {"can_begin": can_begin(prefix, script, network),
                                     "expected": str(expected(prefix, script, network))})

    def published(self):
        zero = bytes(16)
        self.grind("passphrase", zero, b"", 2, "bc1qq", "mainnet", 0, 100, ("ok", {
            "tested": 9, "find": {"counter": 8, "suffix": "i", "account": 0,
                                  "address": "bc1qqv2kx2d9tv4d3epztk59dc5kymkl8pk8scu6xz"}}))
        seed = ENGLISH.to_seed(ENGLISH.to_mnemonic(zero), "")
        self.grind("account", seed, b"", 2, "bc1qq", "mainnet", 0, 100, ("ok", {
            "tested": 32, "find": {"counter": 31, "suffix": "", "account": 31,
                                   "address": "bc1qqpat9khft6dnm9qp0nnrvpyvmyg2ytshn7gglv"}}))

    def edges(self):
        n = 0
        for w in range(1, MAX_WIDTH + 1):
            for d in (-1, 0, 1):
                if n + d >= 0:
                    self.counter(n + d)
            n += 62 ** w
        self.counter(n)
        self.counter(U64)

    def a_prefix(self, script, network):
        rng = self.rng
        fixed = fixed_prefix(script, network)
        r = rng.random()
        if r < 0.1:
            return fixed[:rng.randint(0, len(fixed))]
        free = rng.choice([0, 1, 1, 1, 2]) if r < 0.85 else rng.randint(3, 70)
        chars = BECH32 if script >= 2 else BASE58
        out = ""
        for i in range(free):
            if i == 0 and first_free(script, network):
                out += rng.choice("mn" if rng.random() < 0.9 else "abc")
            else:
                out += rng.choice(chars)
        if script >= 2 and rng.random() < 0.3:
            out = out.upper()
        if rng.random() < 0.05:
            out += rng.choice("0OIlbio_")
        if rng.random() < 0.05:
            fixed = fixed.upper() if rng.random() < 0.5 else fixed[::-1]
        return fixed + out

    def corpus_case(self):
        rng = self.rng
        script = rng.randrange(4)
        network = rng.choice(NETWORKS)
        self.counter(rng.choice([rng.randrange(62 ** rng.randint(1, 8)), rng.randrange(1 << 50)]))
        for _ in range(3):
            self.prefix(self.a_prefix(script, network), script, network)
        prefix = self.a_prefix(script, network)
        if len(prefix) - len(fixed_prefix(script, network)) > 2:
            return
        dial = rng.choice(["passphrase", "account"])
        budget = rng.choice([0, 1, rng.randint(1, 40), 200])
        if dial == "passphrase":
            key = self.randbytes(rng.choice([16, 20, 24, 28, 32]))
            base = bytes(rng.choice(b"abcXYZ019 !~") for _ in range(rng.choice([0, 0, 3, 20])))
            start = rng.choice([0, rng.randrange(62), rng.randrange(62 ** 3),
                                sum(62 ** w for w in range(1, 9)) - rng.randint(1, 3)])
        else:
            key = self.randbytes(rng.choice([16, 32, 64]))
            base = b""
            start = rng.choice([0, rng.randrange(1000), H - rng.randint(1, 3)])
        self.grind(dial, key, base, script, prefix, network, start, budget)

    def run(self):
        self.published()
        self.edges()
        for _ in range(self.cases(200)):
            self.corpus_case()
