"""Faraday Desktop's dice rollers (`faraday_core`'s `fresh/roller/`)
against a model written here from `docs/CODEX32.md` §5.5 and §5.6 and
the booklet's worksheet (`tools/reference/booklet-dice/README.md`),
with the system's answers fixed.

§5.6's "The values", as the model holds it: each value is one byte of a
32-byte answer, the bytes taken from the last to the first; a byte
below 252 gives `byte mod 6 + 1` and a byte of 252 or more is skipped;
a new answer is asked for when one is used up; a reset drops what is
left of an answer.

The one die (§5.6): a click rolls 1, 5, 10, the number typed (1 to
9,999; anything else 1) or every value left, never more than the
entries' room.

The five dice (§5.5) fill the booklet's worksheet: each face goes to
the first empty place, die 1 to die 5 of the first roll and then of the
second; a die whose second value equals its first is cleared, both
places, and rolled again; once all ten places hold a pair that differ,
the five bits (lower 0, higher 1, die 1 the most significant) name the
bech32 character `qpzry9x8gf2tvdw0s3jn54khce6mua7l`[value]. A click of
One pass fills the row the first empty place is in, ending early when an
equal pair clears a die or the character is made; One character makes
the character in progress; One share the characters left in the share;
To the end every character left. Nothing rolls once every character is
in, and nothing lands once the answers run out.

- Known answers: an answer of one byte value repeated, for every byte
  value, so that each face's 42 bytes and the four skipped are seen;
  the distribution that gives (42 bytes each) checked as such.
- Random answers, some heavy in skipped bytes, under random clicks of
  every size into entries of random room, with resets between; random
  clicks of every reach into codex32 entries of random share counts and
  lengths, with the answers running out part way in some.
"""

from common import Area

TREE = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
COUNT_MAX = 9999
PERS = ["one", "five", "ten", "end", "typed"]


class Stream:
    """§5.6's values from a list of answers."""

    def __init__(self, answers):
        self.answers = list(answers)
        self.used = 0
        self.left = b""

    def next(self):
        while True:
            if not self.left:
                if self.used == len(self.answers):
                    return None
                self.left = self.answers[self.used]
                self.used += 1
            b, self.left = self.left[-1], self.left[:-1]
            if b < 252:
                return b % 6 + 1

    def reset(self):
        self.left = b""


def click_count(per, typed):
    if per == "one":
        return 1
    if per == "five":
        return 5
    if per == "ten":
        return 10
    if per == "end":
        return None
    try:
        return min(max(int(typed), 1), COUNT_MAX) if typed.isdigit() else 1
    except ValueError:
        return 1


def model_one(answers, room, steps):
    s = Stream(answers)
    values = []
    for step in steps:
        if step["do"] == "reset":
            s.reset()
            continue
        n = click_count(step["per"], step.get("typed", ""))
        left = room - len(values) if n is None else min(n, room - len(values))
        for _ in range(left):
            v = s.next()
            if v is None:
                break
            values.append(v)
    return {"values": values, "answers_used": s.used}


class Sheet:
    """The booklet's worksheet for one character."""

    def __init__(self):
        self.places = [0] * 10

    def empty(self):
        return next((i for i, p in enumerate(self.places) if p == 0), None)

    def enter(self, face):
        i = self.empty()
        self.places[i] = face
        if i >= 5 and self.places[i] == self.places[i - 5]:
            self.places[i] = self.places[i - 5] = 0

    def char(self):
        if 0 in self.places:
            return None
        value = 0
        for d in range(5):
            value = value * 2 + (1 if self.places[5 + d] > self.places[d] else 0)
        return TREE[value]


def model_five(answers, shares, share_len, steps):
    s = Stream(answers)
    sheet, seq, chars, faces = Sheet(), [], "", []
    total = shares * share_len
    dry = False

    def share_left():
        done = len(chars)
        return min(share_len - done % share_len, total - done)

    def face():
        """One face entered; whether it made a character."""
        nonlocal sheet, seq, chars, dry
        f = s.next()
        if f is None:
            dry = True
            return False
        faces.append(f)
        seq.append(f)
        sheet.enter(f)
        c = sheet.char()
        if c:
            chars += c
            sheet, seq = Sheet(), []
            return True
        return False

    for step in steps:
        reach = step["reach"]
        if dry or len(chars) == total:
            continue
        if reach == 0:
            row = sheet.empty() // 5
            while not dry:
                if face() or sheet.empty() // 5 != row:
                    break
            continue
        n = {1: 1, 2: share_left(), 3: total - len(chars)}[reach]
        target = len(chars) + n
        while not dry and len(chars) < target:
            face()
    return {"faces": faces, "chars": chars, "pending": seq, "answers_used": s.used}


class Roller(Area):
    name = "roller"

    def one(self, answers, room, steps):
        return self.compare(
            "roller.one", {"answers": [a.hex() for a in answers], "room": room, "steps": steps},
            lambda: model_one(answers, room, steps))

    def five(self, answers, shares, share_len, steps):
        return self.compare(
            "roller.five", {"answers": [a.hex() for a in answers], "shares": shares,
                            "share_len": share_len, "steps": steps},
            lambda: model_five(answers, shares, share_len, steps))

    def known_answers(self):
        counts = [0] * 7
        end = {"do": "click", "per": "end"}
        for b in range(256):
            got = self.need(self.one([bytes([b]) * 32, bytes([0]) * 32], 1, [end]), "one")
            if b < 252:
                counts[got["values"][0]] += 1
                if got["answers_used"] != 1:
                    raise AssertionError(f"byte {b} took a second answer")
            elif got != {"values": [1], "answers_used": 2}:
                raise AssertionError(f"byte {b} of 252 or more was not skipped: {got}")
        if counts[1:] != [42] * 6:
            raise AssertionError(f"each face should have 42 bytes: {counts[1:]}")
        # One answer's order: 0, 1, ..., 31 gives faces from byte 31 down.
        order = bytes(range(32))
        self.one([order], 32, [end])
        self.five([order] * 4, 1, 2, [{"reach": 3}])

    def answers(self):
        rng = self.rng
        heavy = rng.random() < 0.2
        out = []
        for _ in range(rng.randint(0, 12)):
            a = bytearray(self.randbytes(32))
            if heavy:
                for i in range(32):
                    if rng.random() < 0.6:
                        a[i] = rng.randint(252, 255)
            out.append(bytes(a))
        return out

    def corpus_case(self):
        rng = self.rng
        steps = []
        for _ in range(rng.randint(1, 6)):
            if rng.random() < 0.15:
                steps.append({"do": "reset"})
                continue
            per = rng.choice(PERS)
            step = {"do": "click", "per": per}
            if per == "typed":
                step["typed"] = rng.choice(["", "0", "1", "7", str(rng.randint(1, 300)),
                                            "9999", "10000"[:rng.randint(1, 4)]])
            steps.append(step)
        self.one(self.answers(), rng.choice([0, 1, 5, rng.randint(1, 200)]), steps)
        reaches = [{"reach": rng.randrange(4)} for _ in range(rng.randint(1, 8))]
        self.five(self.answers(), rng.randint(1, 3), rng.choice([1, 2, rng.randint(1, 48)]),
                  reaches)

    def run(self):
        self.known_answers()
        for _ in range(self.cases(3000)):
            self.corpus_case()
