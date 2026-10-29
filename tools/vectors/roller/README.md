# Faraday Demo's five dice: known answers

The known answers of Faraday Demo's five-dice roller (`docs/CODEX32.md`
§5.5 and §5.6; `tools/reference/booklet-dice/README.md`; F96): dice
rolled from the session's randomness, filling the codex32 booklet's
worksheet as the same faces typed would. The roller is Faraday's own,
offered only in Faraday Demo, testnet only, and carries the label "own
construction, not yet independently reviewed" (F160). A reproduction by
an outside party is asked for in `docs/AUDIT-SUBMISSIONS.md` §10.

Faraday's tests read `vectors.json`
(`faraday/faraday-core/src/fresh/roller/five_tests.rs`): the roller is
given answers whose bytes, in the order it uses them, are `bytes`, and
the faces it lands and the character the worksheet makes of them must
be these.

| Key | What |
|---|---|
| `tree` | the bech32 characters in the order the five bits name them |
| `five_dice` | `clean`: six-faced dice, every second value higher than its first; `tied`: six-faced, die 1's two values equal, so it is rolled again; `twenty`: twenty-faced, with bytes at 240 and over that are skipped. Each with its bytes in the order used, the faces they make, and the character's value and letter |

## The procedure

- Each value is one byte of a 32-byte answer, taken from its last byte
  to its first; a new answer is asked for when one is used up.
- On dice of N faces (2 to 20), a byte below 256 − (256 mod N), the
  largest multiple of N not over 256, gives byte mod N + 1; any other
  byte is skipped, so no face is likelier. For six faces that is a byte
  below 252.
- The faces fill the worksheet in order: die 1 to die 5 of the first
  roll, then die 1 to die 5 of the second. A die whose second value
  equals its first is cleared, both places, and rolled again, first
  value then second, before the dice after it.
- Once all ten places hold pairs that differ, each die gives a bit, 1
  if its second value is higher and 0 if lower, die 1 the most
  significant, and the five bits are the character's value in `tree`.

## To reproduce

From the top of the repository, with Python 3's standard library:

    import json
    v = json.load(open("tools/vectors/roller/vectors.json"))
    for case in v["five_dice"]:
        n = case["faces"]
        rolls = [x % n + 1 for x in case["bytes"] if x < 256 - 256 % n]
        assert rolls == case["rolls"], case["name"]
        first, second = [None] * 5, [None] * 5
        for f in rolls:
            if None in first:
                first[first.index(None)] = f
            else:
                d = second.index(None)
                second[d] = f
                if second[d] == first[d]:
                    first[d] = second[d] = None
        value = 0
        for a, b in zip(first, second):
            value = 2 * value + (b > a)
        assert value == case["value"], case["name"]
        assert v["tree"][value] == case["character"], case["name"]
        answer = bytes(reversed(case["bytes"])).rjust(32, b"\0")
        assert list(reversed(answer))[:len(case["bytes"])] == case["bytes"]
    print("all agree")

The last two lines show where the bytes sit in one answer: the first
byte used is the answer's last. The answers are those Faraday's tests
pinned when they were written. The cross-check's `roller` area
(`tools/crosscheck/area_roller.py`) compares Faraday's Rust with a model
written by Faraday's own authors over random answers and clicks,
including the one die beside the rolls (§5.6), whose values have no
known answers here; it is not an outside implementation.
