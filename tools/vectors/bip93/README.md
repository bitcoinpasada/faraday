# BIP 93 codex32 vectors

BIP 93 ships no vector files: its vectors are prose and bullet lists in
the BIP text. The text is here verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/master/bip-0093.mediawiki`,
and `extract.py` pulls the vectors out of it into `vectors.json`:

    cd tools/vectors/bip93 && python3 extract.py > vectors.json

To refresh, download the text again, re-run the script and check each
digest changed for a reason.

| File | Downloaded | SHA-256 | Read by |
|---|---|---|---|
| `bip-0093.mediawiki` | 2026-09-18 | `c847c6e513b64ee6ab10975690757d64a875f77c0e75866a86b733f3985784ec` | `extract.py`, people |
| `extract.py` | written here | `b9015466d4fcbe1a7ff66a7cbbd30d0ce802d1da333967be9f46c6621f456890` | people |
| `vectors.json` | 2026-09-18 | `9f6d9d1969072c0a635bf217a26934f7ab5d40e136c9f3d4a4d35acfb4d1447e` | `core/osk-bip/tests/codex32.rs` |

`vectors.json` has four keys, each entry naming the BIP vector it came
from:

- `secrets` — the unshared codex32 secrets (vectors 1, 4, 5, 6, 7 and
  8), each with its master seed and, for vectors 1 to 5, the master node
  xprv. Vector 5 also carries the identifier, payload and checksum the
  BIP names separately, and is the only long string.
- `sets` — the two share sets (vectors 2 and 3): the shares the BIP
  publishes, the shares it derives from them by index, the secret they
  recover, the master seed and the xprv.
- `alternates` — the other codex32 secrets the BIP lists as equally
  valid encodings of one master seed (vectors 3 and 4). They differ only
  in the padding bits, which decoding discards.
- `invalid` — all 55 strings the BIP refuses, grouped under the sentence
  that says why: bad checksums, the wrong checksum variant for the
  length, improper lengths, a `0` threshold with a non-`s` index, a
  threshold that is not a digit, a missing `ms`/`1` prefix, and mixed
  case.

`core/osk-bip/tests/codex32.rs` reads the file whole. Every secret is
decoded to its seed and re-encoded; every set derives each share the BIP
derives and recovers the secret from every combination of the threshold
many shares; every alternate decodes to the same seed; every invalid
string is refused. Where the BIP gives an xprv, the seed goes through
`bitcoin::bip32` and the master key is compared against it.
