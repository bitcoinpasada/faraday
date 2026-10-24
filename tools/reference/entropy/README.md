# The entropy procedures, as stated

Coins, hex, playing cards, camera noise, the device's answer and a mix
of sources have no published standard: they are Faraday's own
procedures (`docs/STANDARDS.md` §3). The cross-check's `entropy` area
(`tools/crosscheck/area_entropy.py`) recomputes them over Python's
`hashlib`, written from the statements on this page and never from the
Rust. The dice procedures are other signers' and are quoted, with their
sources, in `tools/reference/dice/README.md`; the area writes them from
that page.

The statements are those `docs/STANDARDS.md` §3 and the user guide make
to a person who wants to check a key by hand, gathered here with the
limits a pad holds.

## Strengths

A key's entropy is 16, 20, 24, 28 or 32 bytes (12, 15, 18, 21 or 24
BIP-39 words: the bits plus one checksum bit per 32 bits, over 11 bits
a word). Every source's entropy at a strength is its value cut to that
many bytes, and a source with fewer inputs than the strength asks for
gives none.

## Each source

- **Dice, hashed.** The rolls written as the ASCII digits `1` to `6`,
  every roll taken, hashed with SHA-256. That digest is the rolls'
  commitment; the entropy is its first bytes. A strength asks for its
  bits over log2 6, rounded up (50, 62, 75, 87), except 256 bits, which
  asks for 99, the count Coldcard and SeedSigner take.
- **Dice, six as zero.** The same, with every 6 written `0` before the
  hash. The commitment is still the hashed procedure's digest of the
  rolls as rolled.
- **Dice, words.** Six rolls a word: five of 1 to 4 read as base-4
  digits, most significant first, then a sixth read as a coin, 1 to 3
  heads (0) and 4 to 6 tails (1), so the word's index is
  `digits × 2 + coin`. A 5 or a 6 in a word's first five places is a
  reroll and is not kept. A strength asks for six rolls for each of its
  words; the entropy is the words' 11-bit indices written one after
  another, most significant bit first, cut to the strength's bits. The
  commitment, as under every dice procedure, is the hashed procedure's
  digest of the rolls kept.
- **Coins.** One flip a bit, heads 1 and tails 0, packed most
  significant bit first (the first flip is the top bit of the first
  byte), no hashing; a strength asks for one flip per bit. The
  commitment is SHA-256 of every flip written as the letters `H` and
  `T`.
- **Hex.** The digits typed are the bytes, two a byte, as they are:
  exactly 32, 40, 48, 56 or 64 digits, either case. The commitment is
  SHA-256 of the digits in lower case.
- **Playing cards.** A card's index is `suit × 13 + rank`, the suits in
  the order ♠ ♥ ♦ ♣ and the ranks A 2 3 4 5 6 7 8 9 T J Q K, so 0 to 51.
  Cards come from a shuffled deck: a card already out of the deck being
  drawn from is refused, and after the fifty-second card a fresh deck
  follows. The commitment is SHA-256 of the indices, one byte each, in
  the order drawn; the entropy is its first bytes. A draw is worth log2
  of the cards left in its deck (the fifty-second card of a deck nothing),
  and a strength asks for the fewest draws whose worth reaches its bits
  (25, 31, 39, 50, 58).
- **Camera noise.** Each frame is its luma bytes. A frame with fewer
  than 32 distinct byte values is refused (a covered or saturated lens).
  The commitment is SHA-256 over the SHA-256 digests of the frames
  taken, in order; the entropy is its first bytes. A strength asks for
  three frames per 128 bits, rounded up (3, 4, 5, 6, 6).
- **The device.** The commitment is SHA-256 of the 32 bytes the system
  answered with; the entropy is its first bytes.
- **A mix.** SHA-256 over the sources' 32-byte commitments,
  concatenated in the order they were taken; the entropy is its first
  bytes. A mix takes two sources at least and five at most.

## Limits

What a pad holds, and refuses past: 200 rolls, 256 flips, 64 hex
digits, 104 cards (two decks), 8 camera frames, 5 sources in a mix.
