"""The entropy procedures (`faraday_entropy`) against a reference written
here over `hashlib`, from the statements of `tools/reference/entropy/README.md`
(coins, hex, cards, camera noise, the device, a mix) and the procedures
quoted in `tools/reference/dice/README.md` (the three dice procedures),
never from the Rust.

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

import hashlib
import json
import math

from common import ROOT, Area

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


def dice_commitment(kept):
    return sha("".join(str(r) for r in kept).encode())


def dice_entropy(kept, procedure, n):
    if procedure == "hashed":
        return cut(dice_commitment(kept), n, len(kept), rolls_needed(n))
    if procedure == "six_as_zero":
        return cut(sha("".join("0" if r == 6 else str(r) for r in kept).encode()), n,
                   len(kept), rolls_needed(n))
    w = words(n)
    if len(kept) < 6 * w:
        return None
    bits = ""
    for i in range(w):
        group = kept[6 * i:6 * i + 6]
        index = 0
        for face in group[:5]:
            index = index * 4 + (face - 1)
        index = index * 2 + (0 if group[5] <= 3 else 1)
        bits += format(index, "011b")
    bits = bits[:8 * n]
    return int(bits, 2).to_bytes(n, "big").hex()


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
        entropy = kept
    parsed = None
    if all(c in HEXDIGITS for c in text) and len(text) % 2 == 0 and len(text) // 2 in STRENGTHS:
        parsed = text.lower()
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
        '`tools/reference/dice/README.md`'
    )
    reference_checked = (
        'the procedures are written in the area over `hashlib` from the statements of '
        '`tools/reference/entropy/README.md` and `tools/reference/dice/README.md`, and '
        'the published dice answers are required of them as well as of Faraday'
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
            'none'
        ),
    ]
    not_compared = (
        'none'
    )

    # ----- the requests -----

    def dice(self, rolls, procedure, n, expected=None):
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

    def prelude(self):
        self.published()
        self.counts()

    def case_corpus(self):
        self.corpus_case()
