"""The entropy procedures (`faraday_entropy`) against Coldcard's own scripts
and a reference written here, never from the Rust.

- Outside code: SeedSigner's `mnemonic_generation.py` over embit's BIP-39
  (`tools/reference/seedsigner/`, `tools/reference/embit/`) judges the
  hashed procedure at 50 and 99 rolls and direct selection's last word at
  every length (SeedSigner's `calculate_checksum` at 12 and 24 words, its
  `generate_mnemonic_from_bytes` at 15, 18 and 21), compared as the entropy
  embit reads back from the words; the hashed procedure
  at every count, and every source's rolls
  commitment, are Coldcard's `rolls.py` and `rolls12.py` run
  (`tools/reference/coldcard-rolls/`); hex digits become bytes through
  CPython's `bytes.fromhex`. Ours, labelled: EntropyLab's
  six-as-zero and direct word selection (its JavaScript needs Node,
  which the build machines lack, transcribed in `tools/reference/dice/README.md`),
  the cut of Coldcard's digest to 20, 24 and 28 bytes, the counts and
  refusals, coins, which characters and how many digits hex takes, cards, camera noise, the device and the mix
  (`tools/reference/entropy/README.md`; no outside implementation).
- The dice answers other tools print (`tools/vectors/entropy/dice.json`):
  Coldcard's `rolls.py`, EntropyLab's six-as-zero and direct word
  selection.
- The counts each strength asks for, from the stated rules.
- Random runs of every source at every strength, short of the count, at
  it and past it, with inputs the pads refuse: a die face outside 1 to
  6, a 5 or 6 where direct selection rerolls, a card already out of its
  deck or past 51, a frame of too few distinct values, a hex character
  that is not a digit, every source past its limit.
- Random mixes of two to five sources (and one, and six), each source
  given by its inputs and committed as that source commits.
"""

import contextlib
import hashlib
import io
import json
import math

from common import ROOT, Area, Mismatch, use_reference

use_reference("coldcard-rolls")
import rolls as coldcard_rolls  # noqa: E402
import rolls12 as coldcard_rolls12  # noqa: E402

use_reference("seedsigner", "shims")
use_reference("seedsigner")
use_reference("embit")
import outside_imports  # noqa: E402

outside_imports.install()
from embit import bip39 as embit_bip39  # noqa: E402
from seedsigner.helpers import mnemonic_generation as seedsigner_dice_module  # noqa: E402
from seedsigner.models.seed import Seed as SeedSignerSeed  # noqa: E402

STRENGTHS = [16, 20, 24, 28, 32]
PROCEDURES = ["hashed", "six_as_zero", "words"]
MAX_ROLLS = 200
MAX_FLIPS = 256
MAX_HEX = 64
DECK = 52
MAX_CARDS = 2 * DECK
MAX_FRAMES = 8
MIN_DISTINCT = 32
MIN_MIX, MAX_MIX = 2, 5


def sha(b):
    return hashlib.sha256(b).digest()


def words(n):
    bits = 8 * n
    return (bits + bits // 32) // 11


def rolls_needed(n):
    if n == 32:
        return 99
    return math.ceil(8 * n / math.log2(6))


def cards_needed(n):
    worth, k = 0.0, 0
    while worth < 8 * n:
        worth += math.log2(DECK - k % DECK)
        k += 1
    return k


def frames_needed(n):
    return -(-3 * 8 * n // 128)


def cut(digest, n, have, need):
    return digest[:n].hex() if have >= need else None


# ----- dice (tools/reference/dice/README.md) -----

def dice_kept(rolls, procedure):
    kept, out = [], []
    for r in rolls:
        ok = 1 <= r <= 6 and len(kept) < MAX_ROLLS
        if ok and procedure == "words" and len(kept) % 6 < 5 and r > 4:
            ok = False
        if ok:
            kept.append(r)
        out.append(ok)
    return kept, out


def coldcard(module, text):
    """The first line Coldcard's own script prints for the rolls `text`:
    its `main()` run with `input` answering `text`, as `echo text |
    python3 rolls.py` does (rolls12.py prints the first sixteen bytes)."""
    module.input = lambda: text
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        module.main()
    return bytes.fromhex(out.getvalue().splitlines()[0])


def dice_commitment(kept):
    """SHA-256 of the rolls as ASCII digits: Coldcard's rolls.py's digest."""
    return coldcard(coldcard_rolls, "".join(str(r) for r in kept))


def hashed_entropy(kept, n):
    """Coldcard's own answer: rolls12.py's sixteen bytes at 128 bits, rolls.py's
    thirty-two at 256. The other lengths are a cut of rolls.py's digest, which
    is ours (no outside script makes 20, 24 or 28 bytes)."""
    text = "".join(str(r) for r in kept)
    if n == 16:
        return coldcard(coldcard_rolls12, text)
    return coldcard(coldcard_rolls, text)[:n]


def dice_entropy(kept, procedure, n):
    if procedure == "hashed":
        if len(kept) < rolls_needed(n):
            return None
        return hashed_entropy(kept, n).hex()
    if procedure == "six_as_zero":
        return cut(sha("".join("0" if r == 6 else str(r) for r in kept).encode()), n,
                   len(kept), rolls_needed(n))
    w = words(n)
    if len(kept) < 6 * w:
        return None
    bits = "".join(format(i, "011b") for i in word_indices(kept, w))
    bits = bits[:8 * n]
    return int(bits, 2).to_bytes(n, "big").hex()


def word_indices(kept, w):
    """Direct selection's wordlist indices: two bits from each of five rolls
    (faces 1 to 4) and the sixth roll read as a coin."""
    out = []
    for i in range(w):
        group = kept[6 * i:6 * i + 6]
        index = 0
        for face in group[:5]:
            index = index * 4 + (face - 1)
        out.append(index * 2 + (0 if group[5] <= 3 else 1))
    return out


# ----- SeedSigner's dice steps over embit's BIP-39 -----

def seedsigner_entropy(kept, procedure, n, fault=False):
    """The entropy SeedSigner's `mnemonic_generation.py` gives, as embit's
    BIP-39 reads it back from the words, or None where SeedSigner has no
    such step. Its dice function hashes the rolls as ASCII digits and cuts
    to 16 bytes only at exactly 50 rolls, and not at all otherwise, so it is
    asked at 50 rolls (16 bytes) and 99 (32 bytes) only. Its
    `calculate_checksum` completes 12 and 24 words only, so direct
    selection's last word is asked of it at those lengths and of
    `generate_mnemonic_from_bytes` (embit) at 15, 18 and 21; SeedSigner has
    no six-as-zero step, no cut to 20, 24 or 28 bytes and no direct
    selection from dice (the word indices are Faraday's rule, ours)."""
    entropy = None
    if procedure == "hashed" and (len(kept), n) in ((50, 16), (99, 32)):
        text = "".join(str(r) for r in kept)
        mnemonic = seedsigner_dice_module.generate_mnemonic_from_dice(text)
        entropy = embit_bip39.mnemonic_to_bytes(" ".join(mnemonic))
    elif procedure == "words" and len(kept) >= 6 * words(n):
        w = words(n)
        wordlist = SeedSignerSeed.get_wordlist("en")
        chosen = [wordlist[i] for i in word_indices(kept, w)]
        if w in (12, 24):
            full = seedsigner_dice_module.calculate_checksum(chosen)
        else:
            raw = embit_bip39.mnemonic_to_bytes(" ".join(chosen), ignore_checksum=True)
            full = seedsigner_dice_module.generate_mnemonic_from_bytes(raw)
        entropy = embit_bip39.mnemonic_to_bytes(" ".join(full))
    if entropy is None:
        return None
    if fault:
        entropy = bytes([entropy[0] ^ 1]) + entropy[1:]
    return entropy.hex()


# ----- the other sources (tools/reference/entropy/README.md) -----

def coins(flips, n):
    kept = flips[:MAX_FLIPS]
    out = [i < MAX_FLIPS for i in range(len(flips))]
    entropy = None
    if len(kept) >= 8 * n:
        entropy = int("".join("1" if f == "H" else "0" for f in kept[:8 * n]), 2) \
            .to_bytes(n, "big").hex()
    return out, sha(kept.encode()), entropy


HEXDIGITS = set("0123456789abcdefABCDEF")


def hexes(text):
    kept, out = "", []
    for c in text:
        ok = c in HEXDIGITS and len(kept) < MAX_HEX
        if ok:
            kept += c.lower()
        out.append(ok)
    entropy = None
    if len(kept) % 2 == 0 and len(kept) // 2 in STRENGTHS:
        entropy = bytes.fromhex(kept).hex()
    parsed = None
    if all(c in HEXDIGITS for c in text) and len(text) % 2 == 0 and len(text) // 2 in STRENGTHS:
        parsed = bytes.fromhex(text).hex()
    return out, sha(kept.encode()), entropy, parsed


def cards(indices, n):
    kept, out = [], []
    for i in indices:
        deck = kept[len(kept) - len(kept) % DECK:]
        ok = 0 <= i < DECK and len(kept) < MAX_CARDS and i not in deck
        if ok:
            kept.append(i)
        out.append(ok)
    c = sha(bytes(kept))
    return out, c, cut(c, n, len(kept), cards_needed(n))


def camera(frames, n):
    digests, out = [], []
    for f in frames:
        distinct = len(set(f))
        taken = distinct >= MIN_DISTINCT and len(digests) < MAX_FRAMES
        if taken:
            digests.append(sha(f))
        out.append({"taken": taken, "distinct": distinct, "pixels": len(f)})
    c = sha(b"".join(digests))
    return out, c, cut(c, n, len(digests), frames_needed(n))


def source_commitment(src):
    kind = src["kind"]
    if kind == "dice":
        return dice_commitment(dice_kept(src["rolls"], PROCEDURES[src["procedure"]])[0])
    if kind == "coins":
        return coins(src["flips"], 16)[1]
    if kind == "cards":
        return cards(src["cards"], 16)[1]
    if kind == "camera":
        return camera([bytes.fromhex(f) for f in src["frames"]], 16)[1]
    if kind == "device":
        return sha(bytes.fromhex(src["random"]))
    if kind == "hex":
        return hexes(src["digits"])[1]
    raise ValueError(kind)


class Entropy(Area):
    name = "entropy"
    faraday = (
        '`faraday_entropy`: dice under the three procedures, coins, hex, cards, camera '
        "frames, the device's answer, a mix"
    )
    reference = (
        'the procedures written out in `area_entropy.py` over `hashlib`, from '
        '`tools/reference/entropy/README.md` and the dice sources quoted in '
        '`tools/reference/dice/README.md`, except that the hashed dice procedure and every '
        "rolls commitment are Coldcard's own `rolls.py` and `rolls12.py` "
        "(`tools/reference/coldcard-rolls/`, run, public domain) and hex digits become bytes "
        "through CPython's `bytes.fromhex`; SeedSigner's `mnemonic_generation.py` "
        "(`tools/reference/seedsigner/`) over embit's BIP-39 (`tools/reference/embit/`) "
        "reads the hashed procedure at 50 and 99 rolls and direct selection's last word back"
    )
    reference_checked = (
        "the hashed dice procedure is Coldcard's own scripts run, their files hashed against "
        '`references.sha256` before any run; SeedSigner\'s and embit\'s files are hashed '
        'the same way and their functions run; the rest is written in the area over `hashlib` '
        'from the statements of `tools/reference/entropy/README.md` and '
        '`tools/reference/dice/README.md`, and the published dice answers are required of '
        'them as well as of Faraday'
    )
    vectors = (
        "Coldcard's `rolls.py` digest, EntropyLab's six-as-zero and direct word "
        'selection (`tools/vectors/entropy/`)'
    )
    corpus = (
        'runs of every source at every strength, short of the count, at it and past it, '
        'with faces, cards, frames and characters the pads refuse and every source past '
        'its limit; mixes of one to six sources given by their inputs'
    )
    differences = [
        (
            "SeedSigner's `generate_mnemonic_from_dice` cuts to 16 bytes only at exactly 50 "
            'rolls and takes every other count as 32 bytes, and its `calculate_checksum` '
            "completes 12 and 24 words only: SeedSigner judges the hashed procedure at 50 and "
            "99 rolls and direct selection's last word at 12 and 24 words (at 15, 18 and 21 "
            "embit's checksum through `generate_mnemonic_from_bytes`), compared as the entropy "
            "embit reads back from those words, the checksum itself being area `bip39`'s; it "
            "has no six-as-zero, "
            "no cut to 20, 24 or 28 bytes, and its dice take no direct selection, which stay "
            "the area's own"
        ),
    ]
    not_compared = (
        'none'
    )

    def __init__(self, exe, seed, scale, sabotage=None):
        # The self-test makes SeedSigner's judge wrong on its own:
        # `entropy.seedsigner` flips a bit of its answer.
        self.seedsigner_fault = sabotage == "entropy.seedsigner"
        super().__init__(exe, seed, scale, None if self.seedsigner_fault else sabotage)

    # ----- the requests -----

    def dice(self, rolls, procedure, n, expected=None):
        kept, _ = dice_kept(rolls, PROCEDURES[procedure])
        outside = seedsigner_entropy(kept, PROCEDURES[procedure], n, self.seedsigner_fault)
        if outside is not None:
            inside = dice_entropy(kept, PROCEDURES[procedure], n)
            if inside != outside:
                raise Mismatch(self.name, "entropy.seedsigner",
                               {"rolls": rolls, "procedure": procedure, "bytes": n},
                               ("ok", outside), ("ok", inside),
                               "SeedSigner's dice steps over embit's BIP-39 differ from the area's")

        def ref():
            kept, out = dice_kept(rolls, PROCEDURES[procedure])
            return {"kept": out, "commitment": dice_commitment(kept).hex(),
                    "entropy": dice_entropy(kept, PROCEDURES[procedure], n)}
        return self.compare("entropy.dice", {"rolls": rolls, "procedure": procedure, "bytes": n},
                            ref, expected)

    def coins(self, flips, n):
        def ref():
            out, c, e = coins(flips, n)
            return {"kept": out, "commitment": c.hex(), "entropy": e}
        return self.compare("entropy.coins", {"flips": flips, "bytes": n}, ref)

    def hexes(self, digits):
        def ref():
            out, c, e, p = hexes(digits)
            return {"kept": out, "commitment": c.hex(), "entropy": e, "parsed": p}
        return self.compare("entropy.hex", {"digits": digits}, ref)

    def cards(self, indices, n):
        def ref():
            out, c, e = cards(indices, n)
            return {"kept": out, "commitment": c.hex(), "entropy": e}
        return self.compare("entropy.cards", {"cards": indices, "bytes": n}, ref)

    def camera(self, frames, n):
        def ref():
            out, c, e = camera(frames, n)
            return {"frames": out, "commitment": c.hex(), "entropy": e}
        return self.compare("entropy.camera", {"frames": [f.hex() for f in frames], "bytes": n},
                            ref)

    def device(self, random, n):
        def ref():
            c = sha(random)
            return {"commitment": c.hex(), "entropy": c[:n].hex()}
        return self.compare("entropy.device", {"random": random.hex(), "bytes": n}, ref)

    def mix(self, sources, n):
        def ref():
            commitments = [source_commitment(s) for s in sources]
            taken = commitments[:MAX_MIX]
            c = sha(b"".join(taken))
            return {"commitments": [x.hex() for x in commitments],
                    "pushed": [i < MAX_MIX for i in range(len(commitments))],
                    "commitment": c.hex(),
                    "entropy": c[:n].hex() if len(taken) >= MIN_MIX else None}
        return self.compare("entropy.mix", {"sources": sources, "bytes": n}, ref)

    # ----- vectors and counts -----

    def published(self):
        v = json.loads((ROOT / "tools/vectors/entropy/dice.json").read_text())
        for c in v["commitments"]:
            got = self.dice([int(r) for r in c["rolls"]], 0, 16)
            if got[0] != "ok" or got[1]["commitment"] != c["commitment"]:
                raise AssertionError(f"Coldcard's rolls.py digest: {got}")
        for case in v["vectors"]:
            rolls = [int(r) for r in case["rolls"]]
            p = PROCEDURES.index(case["procedure"])
            want = dice_kept(rolls, case["procedure"])
            expected = ("ok", {"kept": want[1], "commitment": dice_commitment(want[0]).hex(),
                               "entropy": case["entropy"]})
            self.dice(rolls, p, case["bytes"], expected)

    def counts(self):
        for n in STRENGTHS:
            self.compare("entropy.needed", {"bytes": n}, lambda n=n: {
                "rolls": rolls_needed(n), "word_rolls": 6 * words(n), "flips": 8 * n,
                "digits": 2 * n, "cards": cards_needed(n), "frames": frames_needed(n),
                "words": words(n)})

    # ----- the corpus -----

    def around(self, need, limit):
        """A count short of `need`, at it, past it, or past `limit`."""
        rng = self.rng
        return rng.choice([max(0, need - rng.randint(1, 5)), need, need + rng.randint(1, 20),
                           limit + rng.randint(0, 5), rng.randint(0, limit)])

    def rolls(self, procedure, n):
        rng = self.rng
        count = self.around(rolls_needed(n) if procedure < 2 else 6 * words(n), MAX_ROLLS)
        faces = [1, 2, 3, 4, 5, 6]
        out = []
        for _ in range(count):
            r = rng.choice(faces)
            if procedure == 2 and len(out) % 6 < 5 and r > 4 and rng.random() < 0.9:
                r = rng.randint(1, 4)
            if rng.random() < 0.01:
                r = rng.choice([0, 7, 9])
            out.append(r)
        return out

    def flips(self, n):
        return "".join(self.rng.choice("HT") for _ in range(self.around(8 * n, MAX_FLIPS)))

    def digits(self):
        rng = self.rng
        count = rng.choice([2 * rng.choice(STRENGTHS), rng.randint(0, 70)])
        alphabet = "0123456789abcdefABCDEF"
        text = "".join(rng.choice(alphabet) for _ in range(count))
        if rng.random() < 0.1 and text:
            i = rng.randrange(len(text))
            text = text[:i] + rng.choice("gz x-é") + text[i + 1:]
        return text

    def card_run(self, n):
        rng = self.rng
        count = self.around(cards_needed(n), MAX_CARDS)
        out, deck = [], list(range(DECK))
        rng.shuffle(deck)
        for _ in range(count):
            if not deck:
                deck = list(range(DECK))
                rng.shuffle(deck)
            r = rng.random()
            if r < 0.03 and out:
                out.append(rng.choice(out[len(out) - len(out) % DECK:] or out))
            elif r < 0.04:
                out.append(rng.randint(DECK, 255))
            else:
                out.append(deck.pop())
        return out

    def frames(self, n):
        rng = self.rng
        count = self.around(frames_needed(n), MAX_FRAMES)
        out = []
        for _ in range(count):
            size = rng.randint(1, 600)
            levels = rng.choice([1, 8, MIN_DISTINCT - 1, MIN_DISTINCT, 256])
            base = rng.randrange(256)
            out.append(bytes((base + rng.randrange(levels)) % 256 for _ in range(size)))
        return out

    def a_source(self):
        rng = self.rng
        n = rng.choice(STRENGTHS)
        kind = rng.choice(["dice", "coins", "cards", "camera", "device", "hex"])
        if kind == "dice":
            p = rng.randrange(3)
            return {"kind": kind, "procedure": p, "rolls": self.rolls(p, n)}
        if kind == "coins":
            return {"kind": kind, "flips": self.flips(n)}
        if kind == "cards":
            return {"kind": kind, "cards": self.card_run(n)}
        if kind == "camera":
            return {"kind": kind, "frames": [f.hex() for f in self.frames(n)]}
        if kind == "device":
            return {"kind": kind, "random": self.randbytes(32).hex()}
        return {"kind": kind, "digits": self.digits()}

    def corpus_case(self):
        rng = self.rng
        n = rng.choice(STRENGTHS)
        p = rng.randrange(3)
        self.dice(self.rolls(p, n), p, n)
        self.coins(self.flips(n), n)
        self.hexes(self.digits())
        self.cards(self.card_run(n), n)
        self.camera(self.frames(n), n)
        self.device(self.randbytes(32), n)
        self.mix([self.a_source() for _ in range(rng.choice([1, 2, 2, 3, 4, 5, 6]))], n)

    LOOPS = (("corpus", 3000),)

    def seedsigner_cases(self):
        """SeedSigner's steps at every count it has: the hashed procedure at
        50 and 99 rolls, direct selection at every strength."""
        rng = self.rng
        for n, count in ((16, 50), (32, 99)):
            for _ in range(3):
                self.dice([rng.randint(1, 6) for _ in range(count)], 0, n)
        for n in STRENGTHS:
            for _ in range(3):
                faces = [rng.randint(1, 6 if i % 6 == 5 else 4) for i in range(6 * words(n))]
                self.dice(faces, 2, n)

    def prelude(self):
        self.published()
        self.counts()
        self.seedsigner_cases()

    def case_corpus(self):
        self.corpus_case()
