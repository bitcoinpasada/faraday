"""Codex32 error correction (BIP 93, docs/CODEX32.md §8.4) against
python-codex32's decoder.

A worker of its own, apart from the codex32 area: the decoder is
python-codex32's `correction.py` at a later commit than the release
that area imports (`tools/reference/codex32/correction/`), and both are
the package `codex32`, which one process cannot hold twice.

The reference's fixed-length decoder (`_correct_fixed`) is asked with
the bound BIP 93 states, written here from its sentence and nothing
else: "up to 4 character substitutions, up to 8 unreadable characters
(called erasures), or up to 13 consecutive erasures". With `e`
unreadable boxes (`?`) and `e` at most 8, the decoder may substitute at
most `(8 - e) // 2` more; a run of 9 up to the checksum's length (13,
or 15 on the long checksum, the bound the published corpus states) is
filled with no substitution; anything else is refused. Where a string
of the same set is held, its `ms1` and threshold and identifier boxes
are the reference's immutable prefix.

Known differences, which the corpus keeps out of: python-codex32 reads a
character outside the bech32 alphabet as unreadable, and fills more
than 8 unreadable boxes that are not one run when its equations allow;
Faraday refuses both, so the damage here is bech32 characters and `?`,
and the unreadable boxes past 8 are one run or refused by both.

The published vectors: python-codex32's frozen corpus of 57 cases drawn
from the Haskell decoder of BlockstreamResearch/codex32 pull request 70
(`tools/vectors/codex32-correction/`), each case's expected string or
refusal. Then random strings of every length BIP 93 allows, with
substitutions and unreadable boxes up to and past the bound, runs one
box short of, as long as and one longer than the checksum, a held
string's header fixed, upper case, and `ms1` damaged.
"""

from common import Area, use_reference, vectors

use_reference("codex32", "correction")
from codex32.bech32 import CHARSET, bech32_encode  # noqa: E402
from codex32.checksums import _CODEX32, _CODEX32_LONG  # noqa: E402
from codex32.correction import _correct_fixed  # noqa: E402

LENGTHS = {48: 26, 54: 32, 61: 39, 67: 45, 74: 52, 127: 103}
THRESHOLDS = "023456789"


def checksum_len(chars):
    return 15 if chars == 127 else 13


def one_run(text):
    marks = [i for i, c in enumerate(text) if c == "?"]
    return not marks or marks[-1] - marks[0] + 1 == len(marks)


def ref_correct(text, held):
    if len(text) not in LENGTHS:
        raise ValueError("not a codex32 length")
    erased = text.count("?")
    prefix = None
    if held is not None:
        prefix = "ms1" + held[3:8].lower()
        prefix = prefix.upper() if text.isupper() else prefix
    if erased <= 8:
        limit = (8 - erased) // 2
    elif erased <= checksum_len(len(text)) and one_run(text):
        limit = None
    else:
        raise ValueError("BIP 93: more unreadable boxes than the bound, not one run")
    got = _correct_fixed(text, suspected_profile="ms", max_substitutions=limit,
                         immutable_prefix=prefix)
    if got is None:
        raise ValueError("no correction")
    edits = sorted([len(text) - e.reverse_index, e.observed.lower(), e.replacement.lower()]
                   for e in got.edits)
    return {"string": got.artifact.text.lower(), "edits": edits, "run": erased > 8}


class Codex32Correct(Area):
    name = "codex32-correct"

    def correct(self, text, held=None):
        inputs = {"string": text}
        if held is not None:
            inputs["held"] = held
        return self.compare("codex32.correct", inputs, lambda: ref_correct(text, held))

    def published(self):
        for case in vectors("codex32-correction", "p70_correction_vectors.json")["cases"]:
            ans = self.correct(case["damaged"])
            want = case["expected"]
            if want is None:
                if ans[0] != "err":
                    raise AssertionError(f"corpus {case['damaged']}: proposed, the corpus refuses")
            elif ans[0] != "ok" or ans[1]["string"] != want.lower():
                raise AssertionError(f"corpus {case['damaged']}: {ans}, the corpus has {want}")

    def string(self, chars, header=None):
        """A random string that holds: a header, a payload, its checksum."""
        rng = self.rng
        if header is None:
            k = rng.choice(THRESHOLDS)
            ident = "".join(rng.choice(CHARSET) for _ in range(4))
            index = "s" if k == "0" else rng.choice(CHARSET)
            header = k + ident + index
        payload = [rng.randrange(32) for _ in range(LENGTHS[chars])]
        data = [CHARSET.find(c) for c in header] + payload
        return bech32_encode("ms", data, _CODEX32_LONG if chars == 127 else _CODEX32)

    def damaged(self, s, subs, unread, start=3):
        rng = self.rng
        places = rng.sample(range(start, len(s)), subs + unread)
        b = list(s)
        for i in places[:subs]:
            b[i] = CHARSET[CHARSET.find(b[i]) ^ rng.randrange(1, 32)]
        for i in places[subs:]:
            b[i] = "?"
        return "".join(b)

    def corpus_case(self):
        rng = self.rng
        chars = rng.choice(list(LENGTHS) + [48, 48, 74, 127])
        s = self.string(chars)
        held = None
        start = 3
        if rng.random() < 0.2:
            held = self.string(chars, s[3:8] + ("s" if s[3] == "0" else rng.choice(CHARSET)))
            start = rng.choice([3, 8, 8])
        kind = rng.random()
        if kind < 0.6:
            # Up to the bound, and a little past it.
            subs = rng.choice([0, 1, 1, 2, 2, 3, 4, 4, 5, 6])
            unread = rng.randint(0, max(0, 10 - 2 * subs))
            if subs + unread == 0:
                subs = 1
            text = self.damaged(s, subs, unread, start)
        elif kind < 0.8:
            # A run near the checksum's length, sometimes with a box wrong.
            run = rng.randint(9, checksum_len(chars) + 1)
            at = rng.randrange(start, len(s) - run + 1)
            b = list(s)
            for i in range(at, at + run):
                b[i] = "?"
            if rng.random() < 0.3:
                outside = [i for i in range(start, len(s)) if not at <= i < at + run]
                i = rng.choice(outside)
                b[i] = CHARSET[CHARSET.find(b[i]) ^ rng.randrange(1, 32)]
            text = "".join(b)
        elif kind < 0.9:
            # More than 8 unreadable boxes, scattered.
            text = self.damaged(s, 0, rng.randint(9, 16), start)
        else:
            # The prefix or the header damaged, or nothing.
            text = self.damaged(s, rng.randint(0, 2), rng.randint(0, 2), 0)
        if rng.random() < 0.1:
            text = text.upper()
        self.correct(text, held)

    LOOPS = (("corpus", 10000),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
