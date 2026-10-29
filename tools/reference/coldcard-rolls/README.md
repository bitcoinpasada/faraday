# Coldcard's dice scripts, vendored

Coldcard's own scripts for turning dice rolls into a seed, copied
unchanged, for the crypto cross-check's `entropy` area
(`tools/crosscheck/area_entropy.py`, F160): the area runs them, as
`echo ROLLS | python3 rolls.py` does, and requires their digest of
Faraday's hashed dice procedure. The other signers' procedures are
quoted in `tools/reference/dice/README.md`.

Source: the Coldcard firmware repository
(`https://github.com/Coldcard/firmware`), commit
`948dc108a0a12ea06ec8ee90575609eb3dc90c7b`, files `rolls.py` and
`rolls12.py` of its docs folder, which Coldcard publishes at
<https://coldcardwallet.com/docs/rolls.py>. Fetched through the
repository's contents interface at that commit, so the bytes are the
commit's.

Licence: each file's own header says "Public domain." and the file is
written to be copied ("This file is <https://coldcardwallet.com/docs/rolls.py>"),
so vendoring it is allowed. The firmware repository's other files are
under their own licence and are not copied.

SHA-256 of each file (also in `tools/crosscheck/references.sha256`, which
the cross-check checks before it imports anything):

| File | SHA-256 |
|---|---|
| `rolls.py` | `4348a520e57df665e0ab57baa369a95ace0f9b5fba355b3f22b0b9b2c2e6cd30` |
| `rolls12.py` | `533daff58437cdc9a482d16cd181ba9b0fe6f86a6839b792343d39b496034c85` |

## What it covers

- The hashed procedure's digest: SHA-256 of the rolls as ASCII digits,
  which `rolls.py` prints first (its 24-word output is not used).
- The 128-bit entropy: `rolls12.py` prints the first sixteen bytes.
- The 256-bit entropy: the whole digest from `rolls.py`.

## What it does not cover

- 160, 192 and 224 bits: no outside script makes them. The cross-check
  cuts `rolls.py`'s digest to that length, which is Faraday's own rule
  (shared by SeedSigner's 12-word cut).
- The count of rolls a strength asks for and the faces refused: the
  scripts accept any text and only warn.
- Six as zero and direct word selection: EntropyLab's JavaScript
  (`tools/reference/dice/README.md` §2, §3), which needs Node. The
  cross-check keeps its own transcription of them, labelled.
