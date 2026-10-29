# Own randomness: known answers

The known answers of own randomness (`docs/OWN-RANDOMNESS.md` §2.1 and
§2.3; F86, F154): the accumulator a person's additions make, the
session's seed, a fresh answer, a draw from the seed, This device's
answer, and the pointer-and-keys commitment. The construction is
Faraday's own and carries the label "own construction, not yet
independently reviewed" (F160); these answers are Faraday's, not a
published standard's. A reproduction by an outside party is asked for
in `docs/AUDIT-SUBMISSIONS.md` §10.

Faraday's tests read `vectors.json`
(`faraday/faraday-core/tests/all/own_randomness.rs`): the app is given
the system answers and the batches below, and what it draws must be
these bytes.

| Key | What |
|---|---|
| `seed_answer`, `pooled_answer` | the system's first answer (the seed) and its second (waiting in the pool), 32 bytes each |
| `what`, `draw_what`, `draw_count` | what the fresh answer is for, and what the seed's draw is for with its count, as ASCII |
| `commitments` | the batches added: the dice rolls 1 to 6, the coin flips heads, tails, tails, heads, and five pointer-and-keys events with their 17-byte records |
| `cases` | for no batch, the dice, the dice then the coins, and the pointer-and-keys batch: the accumulator after each batch (`acc`), the seed, the seed's draw and the fresh answer; for the dice, also This device's answer (`device_own`) over `seed_answer` |
| `many_fresh` | 70 fresh bytes with three answers pooled: an answer for every 32 bytes, the latest first, the last cut to what is left |

`acc` and `seed` are steps on the way: Faraday's tests check the draw,
the fresh answer, This device's answer and the commitments.

## The construction

All hashes are SHA-256, `‖` is concatenation, and labels are ASCII
with no terminator.

- A batch's commitment `c`: the dice as their ASCII digits hashed
  (`SHA-256("123456")`), the coins as the letters `H` and `T` hashed;
  pointer and keys as SHA-256("faraday pointer keys" ‖ the records),
  where an event's record is 17 bytes, big-endian: `P` ‖ count (u32) ‖
  ms (u64) ‖ x (u16) ‖ y (u16) for a pointer move, `K` ‖ count ‖ ms ‖
  code (u32) for a key, the count running from 0.
- Adding `c`: `acc` ← SHA-256("faraday own" ‖ `acc` ‖ `c`), `acc` all
  zero before the first.
- The seed: the system's first answer; after each batch, seed ←
  SHA-256("faraday reseed" ‖ seed ‖ `acc`). A batch added before the
  first answer gives the same seed once the answer comes.
- A draw from the seed: SHA-256(what ‖ seed ‖ count).
- A fresh answer for `what`, over an answer taken from the pool:
  SHA-256("faraday fresh" ‖ what ‖ answer), and with `acc` present
  SHA-256("faraday fresh" ‖ what ‖ answer ‖ `acc`).
- This device's answer with `acc` present: SHA-256("faraday device
  own" ‖ answer ‖ `acc`).

## To reproduce

From the top of the repository, with Python 3's standard library:

    import json, struct
    from hashlib import sha256
    v = json.load(open("tools/vectors/own-randomness/vectors.json"))
    H = lambda *p: sha256(b"".join(p)).digest()
    b = bytes.fromhex
    seed0, pooled = b(v["seed_answer"]), b(v["pooled_answer"])
    what, dw, dc = (v[k].encode() for k in ("what", "draw_what", "draw_count"))
    com = v["commitments"]
    recs = b""
    for i, e in enumerate(com["pointer-keys"]["events"]):
        if e["kind"] == "pointer":
            recs += b"P" + struct.pack(">IQHH", i, e["ms"], e["x"], e["y"])
        else:
            recs += b"K" + struct.pack(">IQI", i, e["ms"], e["code"])
    assert recs.hex() == com["pointer-keys"]["records"]
    made = {"dice": H(com["dice"]["rolls"].encode()),
            "coins": H(com["coins"]["flips"].encode()),
            "pointer-keys": H(b"faraday pointer keys", recs)}
    for k, c in made.items():
        assert c.hex() == com[k]["commitment"], k
    for name, case in v["cases"].items():
        acc, seed = None, seed0
        for batch, want in zip(case["batches"], case["acc"]):
            acc = H(b"faraday own", acc or bytes(32), made[batch])
            assert acc.hex() == want, name
            seed = H(b"faraday reseed", seed, acc)
        assert seed.hex() == case["seed"], name
        assert H(dw, seed, dc).hex() == case["draw"], name
        tail = (acc,) if acc else ()
        assert H(b"faraday fresh", what, pooled, *tail).hex() == case["fresh"], name
        if "device_own" in case:
            assert H(b"faraday device own", seed0, acc).hex() == case["device_own"]
    m = v["many_fresh"]
    got = b"".join(H(b"faraday fresh", what, b(a)) for a in reversed(m["pooled"]))
    assert got[:m["length"]].hex() == m["bytes"]
    print("all agree")

The answers were first computed with Python's `hashlib` from §2.3's
definitions when the tests were written; this file holds them as the
tests pinned them. The cross-check's `own` area
(`tools/crosscheck/area_own.py`) compares Faraday's Rust with a model of
the same text written by Faraday's own authors over random cases; it is
not an outside implementation.
