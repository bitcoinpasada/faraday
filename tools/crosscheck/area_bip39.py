"""BIP-39 against python-mnemonic (Trezor's reference, `bip39/`).

- The ten wordlists, word for word, as published and in NFKD.
- Every vector of `tools/vectors/bip39/vectors.json` in Faraday's ten
  languages (entropy to words, words to entropy, the seed under
  "TREZOR") and the Japanese vectors of `test_JP_BIP39.json`, whose
  passphrases are not ASCII and whose sentences are written composed;
  both sides read a sentence in NFKD, as BIP-39 requires, so a sentence
  typed composed (NFC), as most keyboards type it, is the same words.
- Random entropy of every length in every language, both ways; the seed
  (PBKDF2-HMAC-SHA512, 2048 rounds) under random printable-ASCII
  passphrases up to 256 bytes; sentences with a word changed (the
  checksum fails, or holds by chance and both must agree), a word not in
  the list, or a word too many or too few; and every valid last word for
  11 to 23 words.

python-mnemonic's own `to_hd_master_key` is checked against the seed's
master key here too, as a second BIP-32 master.

Faraday's `Mnemonic::to_seed`, the path a typed passphrase takes, takes
printable ASCII only, at most 256 bytes; python-mnemonic takes any text.
Non-ASCII passphrases go through `to_seed_unchecked` (NFKD bytes), which
is what Faraday's vector tests and start-up self-test call; that the
product path still refuses them, and passphrases past 256 bytes, is
checked as a known difference.
"""

import json
import unicodedata

from common import ROOT, Area, Mismatch, use_reference

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

LANGUAGES = ["english", "japanese", "korean", "spanish", "chinese_simplified",
             "chinese_traditional", "french", "italian", "czech", "portuguese"]
LENGTHS = [16, 20, 24, 28, 32]
_M = {}


def mnemonic(lang):
    if lang not in _M:
        _M[lang] = Mnemonic(LANGUAGES[lang])
    return _M[lang]


def nfkd(s):
    return unicodedata.normalize("NFKD", s)


class Bip39(Area):
    name = "bip39"

    def wordlists(self):
        for lang in range(len(LANGUAGES)):
            words = mnemonic(lang).wordlist
            self.compare("bip39.wordlist", {"language": lang},
                         lambda: {"words": words, "nfkd": [nfkd(w) for w in words]})

    def from_entropy(self, lang, entropy, expected=None):
        m = mnemonic(lang)

        def ref():
            if len(entropy) not in LENGTHS:
                raise ValueError("entropy length")
            return {"indices": [m.wordlist.index(w) for w in
                                m.to_mnemonic(entropy).split(m.delimiter)]}
        return self.compare("bip39.from_entropy", {"language": lang, "entropy": entropy.hex()},
                            ref, expected)

    def parse(self, lang, sentence, expected=None):
        """Words to entropy. Both read the sentence in NFKD, as BIP-39
        takes it, so a word typed composed is the listed word."""
        m = mnemonic(lang)

        def ref():
            words = nfkd(sentence).split(" ")
            if len(words) not in (12, 15, 18, 21, 24) or not m.check(sentence):
                raise ValueError("not a valid mnemonic")
            return {"entropy": bytes(m.to_entropy(words)).hex(),
                    "indices": [m.wordlist.index(w) for w in words]}
        return self.compare("bip39.parse", {"language": lang, "sentence": sentence}, ref,
                            expected)

    def seed(self, lang, sentence, passphrase, expected=None):
        return self.compare("bip39.seed", {"language": lang, "sentence": sentence,
                                           "passphrase": passphrase},
                            lambda: {"seed": Mnemonic.to_seed(sentence, passphrase).hex()},
                            expected)

    def seed_nfkd(self, lang, sentence, passphrase, expected=None):
        return self.compare("bip39.seed_nfkd", {"language": lang, "sentence": sentence,
                                                "passphrase": nfkd(passphrase).encode().hex()},
                            lambda: {"seed": Mnemonic.to_seed(sentence, passphrase).hex()},
                            expected)

    def last_words(self, first):
        m = mnemonic(0)

        def ref():
            if len(first) not in (11, 14, 17, 20, 23) or any(i >= 2048 for i in first):
                raise ValueError("count")
            head = " ".join(m.wordlist[i] for i in first)
            return {"indices": [i for i in range(2048) if m.check(f"{head} {m.wordlist[i]}")]}
        return self.compare("bip39.last_words", {"indices": first}, ref)

    def published(self):
        self.wordlists()
        vectors = json.loads((ROOT / "tools/vectors/bip39/vectors.json").read_text())
        for lang, name in enumerate(LANGUAGES):
            for entropy, sentence, seed, xprv in vectors[name]:
                m = mnemonic(lang)
                indices = [m.wordlist.index(w) for w in sentence.split(m.delimiter)]
                self.from_entropy(lang, bytes.fromhex(entropy), ("ok", {"indices": indices}))
                self.parse(lang, sentence, ("ok", {"entropy": entropy, "indices": indices}))
                self.seed(lang, sentence, "TREZOR", ("ok", {"seed": seed}))
                if Mnemonic.to_hd_master_key(bytes.fromhex(seed)) != xprv:
                    raise AssertionError("python-mnemonic's master key differs from its vector")
        for v in json.loads((ROOT / "tools/vectors/bip39/test_JP_BIP39.json").read_text()):
            # The vectors write their sentences composed (NFC), with the
            # ideographic space; read as published and in NFKD.
            want = ("ok", {"entropy": v["entropy"], "indices": [
                mnemonic(1).wordlist.index(w) for w in nfkd(v["mnemonic"]).split(" ")]})
            self.parse(1, v["mnemonic"], want)
            self.parse(1, nfkd(v["mnemonic"]).replace(" ", "\u3000"), want)
            self.seed_nfkd(1, nfkd(v["mnemonic"]), v["passphrase"], ("ok", {"seed": v["seed"]}))

    def known_differences(self):
        """The product path refuses a passphrase that is not printable
        ASCII, or past 256 bytes; python-mnemonic derives a seed."""
        m = mnemonic(0)
        sentence = m.to_mnemonic(bytes(16))
        for passphrase in ["㍍ガバヴァぱばぐゞちぢ十人十色", "tab\there", "x" * 257]:
            rust = self.rust.call("bip39.seed", language=0, sentence=sentence,
                                  passphrase=passphrase)
            if rust[0] != "err":
                raise Mismatch(self.name, "bip39.seed", {"passphrase": passphrase}, rust,
                               ("ok", Mnemonic.to_seed(sentence, passphrase).hex()),
                               "a known difference (the passphrase Faraday takes) no longer holds")

    def ascii(self, n):
        return "".join(chr(self.rng.randint(0x20, 0x7E)) for _ in range(n))

    def corpus_case(self):
        rng = self.rng
        lang = rng.randrange(len(LANGUAGES))
        m = mnemonic(lang)
        entropy = self.randbytes(rng.choice(LENGTHS))
        self.from_entropy(lang, entropy)
        sentence = m.to_mnemonic(entropy)
        self.parse(lang, sentence)
        if rng.random() < 0.2:
            self.parse(lang, unicodedata.normalize("NFC", sentence))
        if rng.random() < 0.25:
            self.seed(lang, sentence, self.ascii(rng.choice([0, 6, 30, rng.randint(0, 256)])))
        if rng.random() < 0.05:
            text = "".join(rng.choice(["é", "ß", "ﬁ", "Å", "가", "ガ", "①", "x"])
                           for _ in range(rng.randint(1, 20)))
            self.seed_nfkd(lang, sentence, text)
        words = sentence.split(m.delimiter)
        bad = list(words)
        bad[rng.randrange(len(bad))] = m.wordlist[rng.randrange(2048)]
        self.parse(lang, m.delimiter.join(bad))
        how = rng.randrange(3)
        if how == 0:
            bad = words + [m.wordlist[rng.randrange(2048)]]
        elif how == 1:
            bad = words[:-1]
        else:
            bad = list(words)
            bad[rng.randrange(len(bad))] = rng.choice(["zzzz", "abandonn", "ab", "Abandon"])
        self.parse(lang, m.delimiter.join(bad))
        if rng.random() < 0.1:
            self.entropy_length_case(lang)
        if rng.random() < 0.05:
            n = rng.choice([11, 14, 17, 20, 23, rng.randint(0, 25)])
            self.last_words([rng.randrange(2048) for _ in range(n)])

    def entropy_length_case(self, lang):
        self.from_entropy(lang, self.randbytes(self.rng.choice([0, 12, 15, 17, 33, 64])))

    def run(self):
        self.known = 0
        self.published()
        self.known_differences()
        for _ in range(self.cases(3000)):
            self.corpus_case()
