# Cards: known answers

The known answers of entropy from playing cards (F158 item 1;
`tools/reference/entropy/README.md`; `docs/STANDARDS.md` §3). The
procedure is Faraday's own, no other wallet's card mode is claimed to
match it, and it carries the label "own construction, not yet
independently reviewed" (F160). A reproduction by an outside party is
asked for in `docs/AUDIT-SUBMISSIONS.md` §10.

Faraday's tests read `vectors.json` (`core/faraday-entropy/src/cards.rs`):
the cards are dealt into `CardDraws` and its commitment, entropy and
refusal must be these.

| Key | What |
|---|---|
| `ranks`, `suits` | the order the indices number them in: `A` to `K` with `T` for the ten, then ♠ ♥ ♦ ♣ |
| `needed` | the cards asked for at 128, 160 and 256 bits: the least number of draws whose bits reach the strength |
| `vectors` | 25 cards (the spades and the first twelve hearts) at 128 bits, refused at 256 for too few; 31 cards (the spades, the hearts and five diamonds) at 160 bits: the cards, their indices, the bits they carry to two places, the commitment and the entropy |

## The procedure

- A card's index is suit × 13 + rank, with rank 0 to 12 and suit 0 to 3
  in the orders above, so the ace of spades is 0 and the king of clubs
  51.
- The commitment is SHA-256 of the draws' indices, one byte each, in
  the order drawn.
- The entropy at a strength is the commitment's first strength / 8
  bytes, once enough cards are drawn.
- A draw from a deck with k cards left carries log2(k) bits: the i-th
  draw (from 0) is one of 52 − (i mod 52), since the 53rd card starts a
  second deck. The cards needed are the least n whose bits reach the
  strength; fewer are refused.

## To reproduce

From the top of the repository, with Python 3's standard library:

    import json
    from hashlib import sha256
    from math import log2
    v = json.load(open("tools/vectors/cards/vectors.json"))
    bits = lambda n: sum(log2(52 - i % 52) for i in range(n))
    for strength, n in v["needed"].items():
        assert bits(n - 1) < int(strength) <= bits(n)
    for case in v["vectors"]:
        cards = case["cards"]
        idx = [v["suits"].index(cards[i + 1]) * 13 + v["ranks"].index(cards[i])
               for i in range(0, len(cards), 2)]
        assert idx == case["indices"]
        c = sha256(bytes(idx)).digest()
        assert c.hex() == case["commitment"]
        assert c[:case["strength"] // 8].hex() == case["entropy"]
        assert round(bits(len(idx)), 2) == case["bits"]
        if "too_few" in case:
            assert v["needed"][str(case["too_few"]["strength"])] == case["too_few"]["need"]
    print("all agree")

The answers were computed with Python's `hashlib` from the procedure
above; Faraday's test also works the bits out by its own sum. The
cross-check's `entropy` area (`tools/crosscheck/area_entropy.py`)
compares Faraday's Rust with a model written by Faraday's own authors;
it is not an outside implementation.
