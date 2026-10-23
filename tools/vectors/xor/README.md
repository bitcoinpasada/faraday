# Seed XOR vectors

Coldcard's Seed XOR splits a seed's entropy into parts of the same
length whose bitwise XOR is the seed again. Each part is a valid BIP-39
mnemonic of the same word count, and every part is needed.

| File | Words | Parts |
|---|---|---|
| `12-words-2-parts.txt` | 12 | 2 |
| `24-words-3-parts.txt` | 24 | 3 |

Each file lists every part and then the seed, as entropy hex and as the
English mnemonic that entropy encodes.

## Where these numbers come from

Coldcard publishes worked examples in the docs/seed-xor.md of its
firmware repository. This build box has no network access and no copy of
that file, so the examples here are not Coldcard's: they are constructed
from the definition of Seed XOR, with values chosen so that anyone can
reproduce them.

The seed and all but the last part are the leading 16 or 32 bytes of the
SHA-256 of a label:

    seed   = sha256("OpenSignerKit Seed XOR vector 12 seed")[..16]
    part 1 = sha256("OpenSignerKit Seed XOR vector 12 part 1")[..16]
    part 2 = seed XOR part 1

    seed   = sha256("OpenSignerKit Seed XOR vector 24 seed")[..32]
    part 1 = sha256("OpenSignerKit Seed XOR vector 24 part 1")[..32]
    part 2 = sha256("OpenSignerKit Seed XOR vector 24 part 2")[..32]
    part 3 = seed XOR part 1 XOR part 2

The mnemonics are the standard BIP-39 encoding of those bytes.

Because the files were not taken from a published source, the test that
reads them, `core/faraday-entropy/tests/xor.rs`, trusts neither the files
nor `faraday_entropy::SeedXor` alone: each part's words and the seed's
are decoded by rust-bitcoin's `bip39`, which checks their checksum, and
must be the file's hex; the parts XORed by a plain byte fold must be the
seed; and `SeedXor` must give the same seed.
`seed_xor_parts_combine_into_the_seeds_words`
(`faraday/faraday-core/tests/all/forms.rs`) also uses the 12-word
example's words.

Swap these for Coldcard's own examples when a machine with network
access can fetch them; the file format and the test do not change.
