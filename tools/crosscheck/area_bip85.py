"""BIP-85 against two implementations Faraday did not write: btclib-wallet's
`bip85` (`tools/reference/btclib/`, on btclib's own curve with
libsecp256k1 turned off) and the BIP's reference implementation
(ethankosakovsky/bip85), which runs unchanged over python-mnemonic,
base58 and a pycoin shim whose every call is btclib's
(`tools/reference/bip85/shims/`).

btclib answers every application Faraday has: the BIP-39 mnemonic, the
WIF, the xprv, the hex bytes and the two passwords, with the BIP's
ranges (16 to 64 bytes, 20 to 86 base64 characters, 10 to 80 base85, the
word counts and languages of its tables). Its mnemonic writes the
sentence with `bip39_slip39`, which is not vendored: the area hands that
one call to python-mnemonic (`bip39/`), the same BIP-39 code the BIP's
reference uses. The reference has no password applications and no
ranges of its own, so it is asked the mnemonic, the WIF, the xprv and
the hex bytes, the hex bytes only where btclib finds the length in the
BIP's range.

Every application vector the BIP publishes, then random master keys,
indices, languages, word counts and lengths, with refusals.
"""

import sys
import types

import bip32_ref  # noqa: F401 - unpacks btclib, on its own curve, before the shim
from common import Area, use_reference, vectors

use_reference("bip39")
use_reference("bip85")
use_reference("bip85", "shims")
import base58  # noqa: E402
from bip85 import BIP85, app  # noqa: E402
from mnemonic import Mnemonic  # noqa: E402

LANGUAGES = ["english", "japanese", "korean", "spanish", "chinese_simplified",
             "chinese_traditional", "french", "italian", "czech", "portuguese"]
# BIP-85's language table as btclib names it, in the BIP's order.
CODES = ["en", "ja", "ko", "es", "zh", "zh_tw", "fr", "it", "cs", "pt"]
N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141


def _sentence(entropy, lang):
    """`bip39_slip39.bip39.mnemonic_from_entropy`, the one call btclib's
    mnemonic makes of a package not vendored, answered by python-mnemonic."""
    return Mnemonic(LANGUAGES[CODES.index(lang)]).to_mnemonic(entropy)


_stub = types.ModuleType("bip39_slip39")
_stub.bip39 = types.ModuleType("bip39_slip39.bip39")
_stub.bip39.mnemonic_from_entropy = _sentence
sys.modules.setdefault("bip39_slip39", _stub)
sys.modules.setdefault("bip39_slip39.bip39", _stub.bip39)
from btclib_wallet import bip85 as btclib85  # noqa: E402


def indices(sentence, language):
    m = Mnemonic(LANGUAGES[language])
    return {"indices": [m.wordlist.index(w) for w in sentence.split(m.delimiter)]}


def entropy(xprv, path):
    return BIP85().bip32_xprv_to_entropy(path, xprv)


def ref_mnemonic(xprv, language, words, index):
    """The BIP's reference."""
    if language < 9:  # app.py's table stops at Czech
        sentence = app.bip39(xprv, LANGUAGES[language], words, index)
    else:
        e = entropy(xprv, f"83696968p/39p/{language}p/{words}p/{index}p")
        sentence = BIP85().entropy_to_bip39(e, words, LANGUAGES[language])
    return indices(sentence, language)


def btclib_mnemonic(xprv, language, words, index):
    return indices(btclib85.mnemonic_from_root_key(xprv, words, CODES[language], index),
                   language)


class Bip85(Area):
    name = "bip85"
    faraday = (
        '`faraday_bip::bip85`'
    )
    reference = (
        "btclib-wallet's `bip85` (`btclib/`, on btclib's own curve), its one call to "
        "`bip39_slip39` answered by python-mnemonic; and `ethankosakovsky/bip85` over "
        'python-mnemonic and a pycoin shim whose every call is btclib\'s (`bip85/`, '
        '`bip39/`)'
    )
    reference_checked = (
        'their vendored files and wheels are hashed against `references.sha256` before '
        "any run, and a published vector's answer is required of each reference as well "
        'as of Faraday'
    )
    vectors = (
        'every application vector of `tools/vectors/bip85/`'
    )
    corpus = (
        'random master keys, indices, languages, word counts and lengths, with refusals'
    )
    differences = [
        (
            'none'
        ),
    ]
    not_compared = (
        "the BIP's reference is not asked the passwords, which it does not have, nor hex "
        'bytes of a length outside the BIP\'s range, which it does not refuse: btclib '
        'alone answers those'
    )

    def btclib_ref(self, op, xprv, index, extra):
        return {
            "mnemonic": lambda: btclib_mnemonic(xprv, extra["language"], extra["words"], index),
            "wif": lambda: {"wif": btclib85.wif_from_root_key(xprv, index)},
            "xprv": lambda: {"xprv": btclib85.xprv_from_root_key(xprv, index)},
            "hex": lambda: {"hex": btclib85.bytes_entropy_from_root_key(
                xprv, extra["bytes"], index).hex()},
            "base64": lambda: {"password": btclib85.base64_password_from_root_key(
                xprv, extra["length"], index)},
            "base85": lambda: {"password": btclib85.base85_password_from_root_key(
                xprv, extra["length"], index)},
        }[op]

    def bip_ref(self, op, xprv, index, extra):
        return {
            "mnemonic": lambda: ref_mnemonic(xprv, extra["language"], extra["words"], index),
            "wif": lambda: {"wif": app.wif(xprv, index)},
            "xprv": lambda: {"xprv": app.xprv(xprv, index)},
            "hex": lambda: {"hex": app.hex(xprv, index, extra["bytes"])},
        }.get(op)

    def check(self, op, xprv, index, expected=None, **extra):
        inputs = dict(extra, xprv=xprv, index=index)
        got = self.compare(f"bip85.{op}", inputs, self.btclib_ref(op, xprv, index, extra),
                           expected)
        bip = self.bip_ref(op, xprv, index, extra)
        if bip is not None and (op != "hex" or got[0] == "ok"):
            self.compare(f"bip85.{op}", inputs, bip, expected)
        return got

    def published(self):
        v = vectors("bip85", "vectors.json")
        master = v["master"]
        en = Mnemonic("english")
        for c in v["applications"]["bip39"]:
            want = {"indices": [en.wordlist.index(w) for w in c["mnemonic"].split()]}
            self.check("mnemonic", master, c["index"], ("ok", want),
                       language=c["language"], words=c["words"])
        for c in v["applications"]["wif"]:
            self.check("wif", master, c["index"], ("ok", {"wif": c["wif"]}))
        for c in v["applications"]["xprv"]:
            self.check("xprv", master, c["index"], ("ok", {"xprv": c["xprv"]}))
        for c in v["applications"]["hex"]:
            self.check("hex", master, c["index"], ("ok", {"hex": c["entropy"][:2 * c["bytes"]]}),
                       bytes=c["bytes"])
        for kind in ["base64", "base85"]:
            for c in v["applications"][kind]:
                self.check(kind, master, c["index"], ("ok", {"password": c["password"]}),
                           length=c["length"])

    def random_master(self):
        while True:
            k = int.from_bytes(self.randbytes(32), "big")
            if 0 < k < N:
                break
        raw = (bytes.fromhex("0488ade4") + bytes(9) + self.randbytes(32) + b"\0"
               + k.to_bytes(32, "big"))
        return base58.b58encode_check(raw).decode()

    def corpus_case(self):
        rng = self.rng
        xprv = self.random_master()
        index = rng.choice([0, 1, rng.randrange(1000), rng.randrange(2**31), 2**31 - 1])
        self.check("mnemonic", xprv, index, language=rng.randrange(10),
                   words=rng.choice([12, 15, 18, 21, 24, 24, rng.randint(0, 30)]))
        self.check("wif", xprv, index)
        self.check("xprv", xprv, index)
        self.check("hex", xprv, index, bytes=rng.choice([16, 32, 64, rng.randint(0, 80)]))
        self.check("base64", xprv, index, length=rng.choice([20, 86, rng.randint(0, 100)]))
        self.check("base85", xprv, index, length=rng.choice([10, 80, rng.randint(0, 100)]))
        if rng.random() < 0.1:
            self.check("wif", xprv, 2**31 + rng.randrange(2**31))

    LOOPS = (("corpus", 3000),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
