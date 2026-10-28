"""BIP-85 against its reference implementation (ethankosakovsky/bip85),
which runs unchanged over python-mnemonic, base58 and a pycoin shim for
hardened BIP-32 (`tools/reference/bip85/shims/`).

The reference has no password applications; those two are the BIP's
own sentences over the reference's derived entropy: base64 or base85
(RFC 1924, which is Python's `b85encode`) of all 64 bytes, cut to the
length. The BIP's stated ranges for the hex and password lengths are
checked here too.

Every application vector the BIP publishes, then random master keys,
indices, languages, word counts and lengths, with refusals.
"""

import base64

from common import Area, use_reference, vectors

use_reference("bip39")
use_reference("bip85")
use_reference("bip85", "shims")
import base58  # noqa: E402
from bip85 import BIP85, app  # noqa: E402

LANGUAGES = ["english", "japanese", "korean", "spanish", "chinese_simplified",
             "chinese_traditional", "french", "italian", "czech", "portuguese"]
N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141


def entropy(xprv, path):
    return BIP85().bip32_xprv_to_entropy(path, xprv)


def ref_mnemonic(xprv, language, words, index):
    if language < 9:  # app.py's table stops at Czech
        sentence = app.bip39(xprv, LANGUAGES[language], words, index)
    else:
        e = entropy(xprv, f"83696968p/39p/{language}p/{words}p/{index}p")
        sentence = BIP85().entropy_to_bip39(e, words, LANGUAGES[language])
    from mnemonic import Mnemonic
    m = Mnemonic(LANGUAGES[language])
    return {"indices": [m.wordlist.index(w) for w in sentence.split(m.delimiter)]}


def ref_hex(xprv, nbytes, index):
    if not 16 <= nbytes <= 64:
        raise ValueError("BIP-85: 16 <= num_bytes <= 64")
    return {"hex": app.hex(xprv, index, nbytes)}


def ref_password(xprv, kind, length, index):
    lo, hi, app_no = {"base64": (20, 86, 707764), "base85": (10, 80, 707785)}[kind]
    if not lo <= length <= hi:
        raise ValueError(f"BIP-85: {lo} <= pwd_len <= {hi}")
    e = entropy(xprv, f"83696968p/{app_no}p/{length}p/{index}p")
    enc = base64.b64encode(e) if kind == "base64" else base64.b85encode(e)
    return {"password": enc.decode()[:length]}


class Bip85(Area):
    name = "bip85"

    def check(self, op, xprv, index, expected=None, **extra):
        refs = {
            "mnemonic": lambda: ref_mnemonic(xprv, extra["language"], extra["words"], index),
            "wif": lambda: {"wif": app.wif(xprv, index)},
            "xprv": lambda: {"xprv": app.xprv(xprv, index)},
            "hex": lambda: ref_hex(xprv, extra["bytes"], index),
            "base64": lambda: ref_password(xprv, "base64", extra["length"], index),
            "base85": lambda: ref_password(xprv, "base85", extra["length"], index),
        }
        return self.compare(f"bip85.{op}", dict(extra, xprv=xprv, index=index), refs[op], expected)

    def published(self):
        v = vectors("bip85", "vectors.json")
        master = v["master"]
        from mnemonic import Mnemonic
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

