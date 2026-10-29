# A mix of sources: known answers

The known answers of a mix (F158; `tools/reference/entropy/README.md`):
New key's entropy from two to five sources, as SHA-256 over their
32-byte commitments in the order taken. The construction is Faraday's
own and carries the label "own construction, not yet independently
reviewed" (F160). A reproduction by an outside party is asked for in
`docs/AUDIT-SUBMISSIONS.md` §10.

Faraday's tests read `vectors.json` (`core/faraday-entropy/src/mix.rs`):
the sources are entered, their commitments pushed into `Mixed`, and its
commitment, entropy and refusals must be these.

| Key | What |
|---|---|
| `vectors` | fifty dice rolls (the transcript of `tools/vectors/entropy/dice.json`) then 128 coin flips, flip i heads when i mod 3 is 0: each source's commitment, the mix's commitment and its entropy at 128 bits |
| `refused` | a mix of no source and of one source: too few, two needed |

## The construction

- The dice's commitment is SHA-256 of the rolls as ASCII digits; the
  coins' is SHA-256 of the flips as the letters `H` and `T`.
- The mix's commitment is SHA-256 over the sources' 32-byte commitments,
  concatenated in the order taken; the order is part of it.
- The entropy at a strength is the commitment's first strength / 8
  bytes, with two to five sources; fewer are refused. (New key also
  asks for dice or This device among them, each entered to its full
  count, `docs/OWN-RANDOMNESS.md` §2.3.)

## To reproduce

From the top of the repository, with Python 3's standard library:

    import json
    from hashlib import sha256
    v = json.load(open("tools/vectors/mix/vectors.json"))
    for case in v["vectors"]:
        cs = []
        for s in case["sources"]:
            text = s["rolls"] if s["source"] == "dice" else s["flips"]
            c = sha256(text.encode()).digest()
            assert c.hex() == s["commitment"], s["source"]
            cs.append(c)
        m = sha256(b"".join(cs)).digest()
        assert m.hex() == case["commitment"]
        assert m[:case["strength"] // 8].hex() == case["entropy"]
        assert sha256(b"".join(reversed(cs))).digest() != m
    for r in v["refused"]:
        assert r["have"] == r["sources"] < r["need"] == 2
    print("all agree")

The answers were computed with Python's `hashlib` from the construction
above. The cross-check's `entropy` area
(`tools/crosscheck/area_entropy.py`) compares Faraday's Rust with a
model written by Faraday's own authors; it is not an outside
implementation.
