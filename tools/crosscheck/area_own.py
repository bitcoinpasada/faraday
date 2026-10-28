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
- New key's This device keeps the shell's answer b as it is, or, when
  acc exists, SHA-256("faraday device own" ‖ b ‖ acc).
- A lock wipes acc and the counts; the seed and the pool stay with the
  process, which the lock ends.

The commitments are New key's (`faraday-entropy`), written here from
§2.1: dice, SHA-256 of the rolls as ASCII digits; coins, SHA-256 of the
flips as `H` and `T`; a camera frame, taken as New key's camera takes
one (refused under 32 distinct luma values), SHA-256 over the frames'
SHA-256 digests in order, here one frame: SHA-256(SHA-256(frame)).
Pointer and keys, from §2.1: each event a 17-byte big-endian record,
`P` ‖ running count (u32) ‖ ms (u64) ‖ x (u16) ‖ y (u16) for a move and
`K` ‖ running count ‖ ms ‖ code (u32) for a key, the count running from
0 across batches; each batch of at most 64 committed as
SHA-256("faraday pointer keys" ‖ its records), the last however few.

- Fixed sessions: nothing added (the bytes from before §2.3), dice added
  after and before the system's answers, dice then coins chained, a
  camera frame, a batch of pointer moves and keys, and a lock, with the
  known answers of
  `faraday/faraday-core/tests/all/own_randomness.rs` where it has them.
- Random sessions of answers, dice, coins, frames (some refused, some
  batches empty), pointer moves and keys (none to past two batches), locks, seed draws, fresh bytes and counts, in any
  order: before and after the seed, one batch and chained ones.
"""

import hashlib
import struct

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
DEVICE_DICE = "71018a2755836a806283b23356523a88c063153b225d6eeffe5176180be792b1"
# Three moves, `a` and Backspace (code 0x110000), as own_randomness.rs
# adds them after the system's answers.
POINTER_EVENTS = [["P", 1000, 100, 200], ["P", 1016, 101, 202], ["P", 1032, 102, 204],
                  ["K", 1100, 0x61], ["K", 1150, 0x110000]]
DRAW_POINTER_KEYS = "af3906fa1a2bfe06b925227ae70c6f4591e40964f162a6d49ceb065cc23420ab"
FRESH_POINTER_KEYS = "db3dc34ce02d9bda8d5fd4c4b28fcf9678880b590d0a75f69f26aaa10fa80c1d"
POINTER_KEYS_BATCH = 64

POOL_MAX = 6
MAX_ROLLS = 200
MAX_FLIPS = 256
MIN_DISTINCT = 32


def sha(*parts):
    return hashlib.sha256(b"".join(parts)).digest()


def pointer_keys_batches(events):
    """§2.1's commitments of `events`, a batch per 64 and the rest."""
    records = []
    for i, e in enumerate(events):
        if e[0] == "P":
            records.append(b"P" + struct.pack(">IQHH", i, e[1], e[2], e[3]))
        else:
            records.append(b"K" + struct.pack(">IQI", i, e[1], e[2]))
    return [(sha(b"faraday pointer keys", *records[at:at + POINTER_KEYS_BATCH]),
             len(records[at:at + POINTER_KEYS_BATCH]))
            for at in range(0, len(records), POINTER_KEYS_BATCH)]


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
        if do == "pointer_keys":
            taken = [self.add("pointer_keys", c, n) for c, n in pointer_keys_batches(s["events"])]
            return bool(taken) and all(taken)
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
        if do == "device":
            b = bytes.fromhex(s["bytes"])
            return (sha(b"faraday device own", b, self.acc) if self.acc else b).hex()
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
        # New key's This device: the answer as it is, then with the dice.
        dev = {"do": "device", "bytes": SEED.hex()}
        self.session([dev, DICE, dev], [SEED.hex(), True, DEVICE_DICE])
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
        # Pointer and keys: one batch of five, as own_randomness.rs adds it.
        pk = {"do": "pointer_keys", "events": POINTER_EVENTS}
        self.session(a + [pk, draw(), fresh(), COUNTS],
                     [None, None, True, DRAW_POINTER_KEYS, FRESH_POINTER_KEYS,
                      counts(pointer_keys=5)])
        # None taken; and 130 events, two full batches and two more.
        self.session(a + [{"do": "pointer_keys", "events": []}, draw()],
                     [None, None, False, DRAW_PLAIN])
        many = [["P", 7 * i, i, 2 * i] if i % 3 else ["K", 7 * i, 0x30 + i % 10]
                for i in range(130)]
        self.session(a + [{"do": "pointer_keys", "events": many}, draw(), fresh(), COUNTS])

    def random_step(self):
        rng = self.rng
        kind = rng.choices(["answer", "dice", "coins", "frame", "lock", "draw", "fresh", "counts",
                            "device", "pointer_keys"],
                           [4, 3, 2, 2, 0.3, 3, 3, 1, 2, 2])[0]
        if kind == "pointer_keys":
            n = rng.choice([0, 1, 5, 63, 64, 65, rng.randint(1, 200)])
            events = []
            for _ in range(n):
                ms = rng.randrange(1 << 40)
                if rng.random() < 0.7:
                    events.append(["P", ms, rng.randrange(1 << 16), rng.randrange(1 << 16)])
                else:
                    code = rng.choice([rng.randrange(0x20, 0x7f), rng.randrange(0x110000),
                                       0x110000, 0x110001])
                    events.append(["K", ms, code])
            return {"do": "pointer_keys", "events": events}
        if kind == "device":
            return {"do": "device", "bytes": self.randbytes(32).hex()}
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

    LOOPS = (("session", 400),)

    def prelude(self):
        self.fixed()

    def case_session(self):
        self.session([self.random_step() for _ in range(self.rng.randint(1, 24))])
