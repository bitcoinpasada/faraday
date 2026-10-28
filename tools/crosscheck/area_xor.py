"""Seed XOR against python-mnemonic (Trezor's BIP-39 reference) and a
plain byte XOR.

Coldcard's two published examples and this tree's two constructed
vectors, then random splits and combines in every BIP-39 language and
length, and refusals: a part with a broken checksum, parts of different
lengths, one part alone, five parts.
"""

from common import Area, ROOT, use_reference

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

LANGUAGES = ["english", "japanese", "korean", "spanish", "chinese_simplified",
             "chinese_traditional", "french", "italian", "czech", "portuguese"]
LENGTHS = [16, 20, 24, 28, 32]
_MNEMONICS = {}


def mnemonic(lang):
    if lang not in _MNEMONICS:
        _MNEMONICS[lang] = Mnemonic(LANGUAGES[lang])
    return _MNEMONICS[lang]


def indices(lang, entropy):
    m = mnemonic(lang)
    return [m.wordlist.index(w) for w in m.to_mnemonic(entropy).split(m.delimiter)]


def xor_all(parts):
    """Coldcard's Seed XOR: 2 to 4 parts of one BIP-39 length."""
    if not 2 <= len(parts) <= 4:
        raise ValueError(f"{len(parts)} parts")
    if len({len(p) for p in parts}) != 1 or len(parts[0]) not in LENGTHS:
        raise ValueError("parts of different or wrong lengths")
    out = bytes(len(parts[0]))
    for p in parts:
        out = bytes(a ^ b for a, b in zip(out, p))
    return out


def answer(lang, entropy):
    return {"entropy": entropy.hex(), "indices": indices(lang, entropy)}


class Xor(Area):
    name = "xor"
    faraday = (
        '`faraday_entropy::SeedXor` with `faraday_bip::bip39`'
    )
    reference = (
        'python-mnemonic and a byte XOR (`bip39/`)'
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        'Faraday; the byte XOR is written in the area, 2 to 4 parts of one BIP-39 '
        'length'
    )
    vectors = (
        "Coldcard's two examples (`tools/vectors/xor/coldcard.json`) and this tree's "
        'two'
    )
    corpus = (
        'splits and combines in all ten languages and five lengths, with refusals'
    )
    differences = [
        (
            'none'
        ),
    ]
    not_compared = (
        'none'
    )

    def combine(self, lang, sentences, expected=None):
        def ref():
            m = mnemonic(lang)
            return answer(lang, xor_all([bytes(m.to_entropy(s.split(m.delimiter)))
                                         for s in sentences]))
        return self.compare("xor.combine", {"language": lang, "parts": sentences}, ref, expected)

    def split(self, lang, randoms, seed):
        return self.compare(
            "xor.split",
            {"language": lang, "random_parts": [r.hex() for r in randoms], "seed": seed.hex()},
            lambda: answer(lang, xor_all(randoms + [seed])))

    def published(self):
        import json
        m = mnemonic(0)
        coldcard = json.loads((ROOT / "tools/vectors/xor/coldcard.json").read_text())
        for ex in coldcard["examples"]:
            want = answer(0, bytes(m.to_entropy(ex["seed"])))
            self.combine(0, ex["parts"], ("ok", want))
        for name in ["12-words-2-parts.txt", "24-words-3-parts.txt"]:
            text = (ROOT / "tools/vectors/xor" / name).read_text()
            value = lambda key: [l.split(":", 1)[1].strip() for l in text.splitlines()  # noqa: E731
                                 if l.startswith(key)]
            parts = [w for i in range(1, 5) for w in value(f"part {i} words:")]
            seed = bytes.fromhex(value("seed entropy:")[0])
            self.combine(0, parts, ("ok", answer(0, seed)))

    def corpus_case(self):
        rng = self.rng
        lang = rng.randrange(len(LANGUAGES))
        n = rng.choice(LENGTHS)
        count = rng.randint(2, 4)
        m = mnemonic(lang)
        parts = [self.randbytes(n) for _ in range(count)]
        sentences = [m.to_mnemonic(p) for p in parts]
        self.combine(lang, sentences)
        seed = self.randbytes(n)
        last = self.need(self.split(lang, parts[:-1], seed), "split")["entropy"]
        # The parts of a split combine to its seed.
        whole = self.need(self.combine(lang, sentences[:-1] + [m.to_mnemonic(bytes.fromhex(last))]),
                          "combine")
        if whole["entropy"] != seed.hex():
            raise AssertionError("a split's parts do not combine to its seed")
        # Refusals: a word changed (the checksum then fails, or the
        # sentence still passes by chance and both must agree on it),
        # mixed lengths, too few and too many parts.
        bad = sentences[0].split(m.delimiter)
        bad[rng.randrange(len(bad))] = m.wordlist[rng.randrange(2048)]
        self.combine(lang, [m.delimiter.join(bad)] + sentences[1:])
        other = rng.choice([x for x in LENGTHS if x != n])
        self.combine(lang, sentences[:1] + [m.to_mnemonic(self.randbytes(other))])
        self.combine(lang, sentences[:1])
        self.split(lang, [self.randbytes(n) for _ in range(4)], self.randbytes(n))

    LOOPS = (("corpus", 4000),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
