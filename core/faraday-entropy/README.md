# faraday-entropy

Turns what a person supplies into the 128, 160, 192, 224 or 256 bits a
BIP-39 key of 12 to 24 words takes, and says what looks wrong with it. It
draws no randomness itself; `no_std` + `alloc`.

## What it computes

- `DiceRolls`: three published dice procedures (SHA-256 of the rolls as ASCII
  digits, the same with 6 written as 0, and the BitBox word-by-word one).
- `BookletChar`: the codex32 booklet's dice de-biasing worksheet, ten rolls of five dice to one character.
- `CoinFlips`: one bit a flip, packed MSB first.
- `RawHex`: hex digits taken as they are.
- `CardDraws`: SHA-256 of the cards' indices, with a deck tracker.
- Camera noise: `FrameStats` judges a frame before it is used.
- `SeedXor`: combines or splits BIP-39 seeds by XOR.
- `Warnings`, `DiceStats`, `CoinStats`, `CardStats`: the mistakes real hands
  make (runs, bias, repeats) as warnings.

## Written in this crate

All of it: the accumulation and packing above and the warnings. SHA-256 is
`faraday_crypto::sha256`. No published vector set exists for "pack these
dice as entropy"; the dice procedures reproduce what Coldcard, SeedSigner,
Keystone and BitBox publish.

## Checked against

- `tools/reference/booklet-dice/README.md` quotes the booklet's worksheet and tree; the unit tests in `src/booklet.rs` walk all 32 paths of its tree.
- `tools/reference/dice/README.md` quotes each procedure's source; the
  inline unit tests in `src/dice.rs` check against it.
- `tools/vectors/xor/` (`12-words-2-parts.txt`), read by
  `faraday/faraday-core/tests/all/forms.rs`.
- End to end through `faraday/faraday-core/tests/all/keygen.rs`.

## `unsafe`

None. The workspace sets `unsafe_code = "forbid"`.

## Secrets

`RawEntropy`, `DiceRolls`, `CoinFlips`, `RawHex`, `CardDraws` and `SeedXor`
are fixed buffers that implement `Zeroize` and `ZeroizeOnDrop` and are
erased on drop. Every file of `src/` is in `tools/lint-secrets.sh`'s `SECRET_FILES`.
