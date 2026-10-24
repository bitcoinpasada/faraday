"""Randomness a person adds of their own (`docs/OWN-RANDOMNESS.md` §2.3,
F86) against a model written here from §2.3 over `hashlib` alone, with
the system's answers fixed.

§2.3, as the model holds it:

- Adding a batch whose commitment is c: acc ← SHA-256("faraday own" ‖
  acc ‖ c), acc all zero before the first. A batch of nothing is not
  taken.
- The seed: the system's first answer, or, when acc exists then,
  SHA-256("faraday reseed" ‖ answer ‖ acc); and at each batch taken
  while the seed exists, seed ← SHA-256("faraday reseed" ‖ seed ‖ acc).
  Acc alone makes no seed.
- A later answer waits in the pool (at most six); `fresh(what)` takes
  the last one in, SHA-256("faraday fresh" ‖ what ‖ answer ‖ acc), with
  acc left out when none was added: exactly the bytes from before §2.3.
- A seed draw for what and count: SHA-256(what ‖ seed ‖ count).
- A lock wipes acc and the counts; the seed and the pool stay with the
  process, which the lock ends.

The commitments are New key's (`faraday-entropy`), written here from
§2.1: dice, SHA-256 of the rolls as ASCII digits; coins, SHA-256 of the
flips as `H` and `T`; a camera frame, taken as New key's camera takes
one (refused under 32 distinct luma values), SHA-256 over the frames'
SHA-256 digests in order, here one frame: SHA-256(SHA-256(frame)).

- Fixed sessions: nothing added (the bytes from before §2.3), dice added
  after and before the system's answers, dice then coins chained, a
  camera frame, and a lock, with the known answers of
  `faraday/faraday-core/tests/all/own_randomness.rs` where it has them.
- Random sessions of answers, dice, coins, frames (some refused, some
  batches empty), locks, seed draws, fresh bytes and counts, in any
  order: before and after the seed, one batch and chained ones.
"""

import hashlib

from common import Area

SEED = bytes([0x11]) * 32
POOLED = bytes([0x22]) * 32
WHAT = b"faraday test"
DRAW = b"faraday test draw"
COUNT = b"count"
# The known answers own_randomness.rs states (computed there with
# hashlib from §2.3, independently of the code).
DRAW_PLAIN = "5e1e6e312247cd1efb95edae7db2c66f3a6588cde135c90ee91e75c29679364f"
FRESH_PLAIN = "bf1ad502ef69b59b91987efa113f86dc832293d47fa42f3a604743799fdf4c07"
DRAW_DICE = "7b795d54616b572836b470cc594e232e72c1a24c0bbae282c5186b76b602d031"
FRESH_DICE = "7e3f2c6b2ebffb02a1d505b8004ffefbacfb7806df8032b1b93589e3693a6c65"
DRAW_DICE_COINS = "c1b0177f5056c4ec9f56f550f1743f96366034354f5ba066b03ffa105f9d09b6"
FRESH_DICE_COINS = "881bfdaab65025a43c5b28bcf6b98c228a2b05c2d67c911098962b13a24dfc53"

POOL_MAX = 6
MAX_ROLLS = 200
MAX_FLIPS = 256
MIN_DISTINCT = 32


def sha(*parts):
    return hashlib.sha256(b"".join(parts)).digest()


class Session:
    """§2.3's state: the seed, the pool, acc and the counts."""

    def __init__(self):
        self.seed = None
        self.pool = []
        self.acc = None
        self.counts = dict.fromkeys(
            ["camera_frames", "dice_rolls", "coin_flips", "cards", "pointer_keys"], 0)

    def answer(self, b):
        if self.seed is None:
            self.seed = sha(b"faraday reseed", b, self.acc) if self.acc else b
        elif len(self.pool) < POOL_MAX:
            self.pool.append(b)

    def add(self, count, commitment, n):
        if n == 0:
            return False
        self.acc = sha(b"faraday own", self.acc or bytes(32), commitment)
        self.counts[count] += n
        if self.seed is not None:
            self.seed = sha(b"faraday reseed", self.seed, self.acc)
        return True

    def step(self, s):
        do = s["do"]
        if do == "answer":
            self.answer(bytes.fromhex(s["bytes"]))
            return None
        if do == "dice":
            rolls = s["rolls"]
            if any(c not in "123456" for c in rolls) or len(rolls) > MAX_ROLLS:
                return False
            return self.add("dice_rolls", sha(rolls.encode()), len(rolls))
        if do == "coins":
            flips = s["flips"]
            if len(flips) > MAX_FLIPS:
                return False
            return self.add("coin_flips", sha(flips.encode()), len(flips))
        if do == "frame":
            luma = bytes.fromhex(s["luma"])
            if len(set(luma)) < MIN_DISTINCT:
                return False
            return self.add("camera_frames", sha(sha(luma)), 1)
        if do == "lock":
            self.acc = None
            self.counts = dict.fromkeys(self.counts, 0)
            return None
        if do == "draw":
            if self.seed is None:
                return None
            return sha(bytes.fromhex(s["what"]), self.seed, bytes.fromhex(s["count"])).hex()
        if do == "fresh":
            if not self.pool:
                return None
            b = self.pool.pop()
            return sha(b"faraday fresh", bytes.fromhex(s["what"]), b, self.acc or b"").hex()
        if do == "counts":
            return dict(self.counts)
        raise AssertionError(do)


def run_session(steps):
    s = Session()
    return {"out": [s.step(x) for x in steps]}


def answer(b):
    return {"do": "answer", "bytes": b.hex()}


def draw(what=DRAW, count=COUNT):
    return {"do": "draw", "what": what.hex(), "count": count.hex()}


def fresh(what=WHAT):
    return {"do": "fresh", "what": what.hex()}


DICE = {"do": "dice", "rolls": "123456"}
COINS = {"do": "coins", "flips": "HTTH"}
COUNTS = {"do": "counts"}
LOCK = {"do": "lock"}


def counts(**n):
    out = dict.fromkeys(["camera_frames", "dice_rolls", "coin_flips", "cards", "pointer_keys"], 0)
    out.update(n)
    return out


class Own(Area):
    name = "own-randomness"

    def session(self, steps, expected=None):
        return self.compare("own.session", {"steps": steps}, lambda: run_session(steps),
                            None if expected is None else ("ok", {"out": expected}))

    def fixed(self):
        a = [answer(SEED), answer(POOLED)]
        # Nothing added: the bytes from before §2.3.
        self.session(a + [draw(), fresh(), COUNTS],
                     [None, None, DRAW_PLAIN, FRESH_PLAIN, counts()])
        # Dice after the answers, and before them: the same bytes.
        self.session(a + [DICE, draw(), fresh()], [None, None, True, DRAW_DICE, FRESH_DICE])
        self.session([DICE] + a + [draw(), fresh()], [True, None, None, DRAW_DICE, FRESH_DICE])
        # Dice then coins, chained.
        self.session(a + [DICE, COINS, draw(), fresh(), COUNTS],
                     [None, None, True, True, DRAW_DICE_COINS, FRESH_DICE_COINS,
                      counts(dice_rolls=6, coin_flips=4)])
        # A batch of nothing is not taken.
        self.session(a + [{"do": "dice", "rolls": ""}, draw(), fresh()],
                     [None, None, False, DRAW_PLAIN, FRESH_PLAIN])
        # Before the system answers, nothing is drawn.
        self.session([DICE, draw(), fresh(), answer(SEED), draw(), fresh()],
                     [True, None, None, None, DRAW_DICE, None])
        # A lock: what was added goes, and the pooled answer is plain again.
        self.session(a + [DICE, LOCK, COUNTS, fresh()],
                     [None, None, True, None, counts(), FRESH_PLAIN])
        # One camera frame with every luma value in it, as own_randomness.rs
        # adds it; and one of a single value, refused.
        frame = bytes(i % 256 for i in range(4096))
        self.session(a + [{"do": "frame", "luma": frame.hex()}, draw(), fresh(), COUNTS])
        self.session(a + [{"do": "frame", "luma": bytes(4096).hex()}, draw(), fresh()],
                     [None, None, False, DRAW_PLAIN, FRESH_PLAIN])

    def random_step(self):
        rng = self.rng
        kind = rng.choices(["answer", "dice", "coins", "frame", "lock", "draw", "fresh", "counts"],
                           [4, 3, 2, 2, 0.3, 3, 3, 1])[0]
        if kind == "answer":
            return answer(self.randbytes(32))
        if kind == "dice":
            n = rng.choice([0, 1, 6, 50, 99, rng.randint(1, 210)])
            rolls = "".join(rng.choice("123456") for _ in range(n))
            if n and rng.random() < 0.1:
                at = rng.randrange(n)
                rolls = rolls[:at] + rng.choice("0789") + rolls[at + 1:]
            return {"do": "dice", "rolls": rolls}
        if kind == "coins":
            n = rng.choice([0, 1, 4, 128, rng.randint(1, 260)])
            return {"do": "coins", "flips": "".join(rng.choice("HT") for _ in range(n))}
        if kind == "frame":
            k = rng.choice([1, 8, 31, 32, 33, 200, 256])
            values = rng.sample(range(256), k)
            luma = bytes(values) + bytes(rng.choice(values) for _ in range(rng.randint(0, 2000)))
            return {"do": "frame", "luma": luma.hex()}
        if kind == "lock":
            return LOCK
        if kind == "draw":
            return draw(self.randbytes(rng.randint(0, 40)), self.randbytes(rng.choice([0, 8, 16])))
        if kind == "fresh":
            return fresh(self.randbytes(rng.randint(0, 40)))
        return COUNTS

    def run(self):
        self.fixed()
        for _ in range(self.cases(400)):
            self.session([self.random_step() for _ in range(self.rng.randint(1, 24))])
