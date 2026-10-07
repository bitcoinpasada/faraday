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

Coldcard publishes worked examples in the `docs/seed-xor.md` of its
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
reads them (`opensigner/opensigner-core/tests/xor.rs`) does not trust
either the files or `osk_entropy::SeedXor` alone. It recovers each
mnemonic's entropy by packing the 11-bit word indices itself, checks the
BIP-39 checksum itself, XORs the parts with a plain byte fold, and
compares all three answers: the file's hex, its own computation, and
`SeedXor`. A wrong file or a wrong implementation fails the test; only
all three being wrong the same way could pass it.

Swap these for Coldcard's own examples when a machine with network
access can fetch them; the file format and the test do not change.
