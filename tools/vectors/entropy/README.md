# Entropy procedure vectors

The dice answers other tools print, gathered from
`tools/reference/dice/README.md` (which quotes the sources, their
commits and the commands run) for the cross-check's `entropy` area
(`tools/crosscheck/area_entropy.py`).

| File | What |
|---|---|
| `dice.json` | Coldcard's `rolls.py` digest of the fifty-roll transcript, which is the rolls' commitment and, cut to 16 bytes, their 128-bit entropy; EntropyLab's "Dice [1-6] / Hashed rolls" at 128 and 256 bits; EntropyLab's direct word selection with embit's checksum completion, twice |

Fifty rolls are short of the 99 that 256 bits takes, so `rolls.py`'s
whole digest is checked as the commitment, the same hash before it is
cut to the strength.
