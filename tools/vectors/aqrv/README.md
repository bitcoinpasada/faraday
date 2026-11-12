# AQRV vectors

The test vectors of the Argon QR Vault (AQRV) (`https://github.com/bitcoinpasada/aqrv`, `AQRV.md` §11) as a
pinned copy: `aqrv.json` (every case, with its inputs, every intermediate
value and the expected outcome), `qr-32.png`, `qr-53.png` and `qr-62.png` (the
QR code of the step 0 case at each size), and `check.py` (a second reader and
writer, written from the specification alone, run over every case). MIT (the
`LICENSE` of that repository). The specification, the vectors, `gen.py` that
writes them and `vectors/README.md`, which says what computed them, what each
case holds and how to run `check.py`, are in that repository (F249); the vectors are
regenerated there, never here.

Source: `https://github.com/bitcoinpasada/aqrv`, commit `daad0cc9d3589bd3c365600896c2e65d33d44c16`, files `vectors/aqrv.json`,
`vectors/check.py`, `vectors/qr-32.png`, `vectors/qr-53.png` and
`vectors/qr-62.png`, copied unchanged. The Rust library in `aqrv/` and
`diff.py` read this copy; it is refreshed only by copying the files of a
named commit and writing that commit and the digests below (F249).

| File | SHA-256 |
|---|---|
| `aqrv.json` | `bf7381d1493e17f4b36f186dc8155bdb29874a4080c8ed0475e9c81c50058edd` |
| `check.py` | `9c680c28f353a0b561de1052d917041917a732fe4db06cbdc6fb60c05b7b14bd` |
| `qr-32.png` | `fe6ae830529457c62033d2a91f16fa57acddc8383c106d4e402c83eddc0bab35` |
| `qr-53.png` | `389f3dea464b8bb45ecd95663db94ad5ce98a36241f72906da77fcfd2771cf86` |
| `qr-62.png` | `114d8f945eab8f86ae9e1e4ae607e03f196f7025e71d2fa4f4044878a9224d88` |

## Running

    python3 -I -B tools/vectors/aqrv/check.py          # the second reader; --no-qr skips the decoders
    cargo nextest run --locked -p aqrv                 # the Rust library over every case and the three pictures
    cargo run --locked -p aqrv-cli -- vectors          # the same cases from the command line

`check.py` needs `cryptography` (a version with Argon2id), and for the pictures `zxingcpp`, Pillow and `zbarimg`, each skipped with a note if missing. The run takes about 40 seconds and up
to 1 GiB of memory (the one Argon2id run at the 1 GiB step at a time).

## The differential run

`diff.py` is Faraday's own (not a copy) and compares the two readers and writers on many codes that are not in the vectors. From a seeded
generator (`--seed`, printed, so a run repeats) it makes valid codes with `check.py`'s `seal` at the lowest
memory step, one of every kind at every size a round, with random items, passphrases, salts and padding, and
corrupts each: bit flips in the marker, salt, ciphertext and tag, a changed step or header byte, truncation,
extension, the tag of another kind, a right tag over a plaintext the kind's layout may refuse, fixed padding,
a wrong, empty or differently normalised passphrase and, for the 32-byte size, the text form with a changed
case, character, length or first digit. Writer cases (a bad codex32 checksum, case, prefix, length or
threshold; a text too long or empty; a kind the size does not hold) and random payloads are added. Each case
goes to `check.py` and, as JSON lines in one process, to the Rust library through `aqrv-cli`'s example
`diff_batch`, which runs `open`, `seal` and the recognition as they are, Argon2id included. The outcome (not
recognised, the header, no tag, the layout rule, the item, the envelope or the refusal) must be the same;
the run prints the cases per kind and per corruption and every disagreement, and exits 1 on any.

    python3 -I -B tools/vectors/aqrv/diff.py --seed 1 --rounds 12 [--jobs 4]

It builds and runs the example with `cargo` (`--rust COMMAND` runs another binary) and needs what `check.py`
needs; a round is about 800 cases.

## Results

Run on 2026-11-11 against the library and `check.py` as they are in the commit that adds this section;
neither found a fault, and nothing was changed to make them pass.

- **Differential run:** `diff.py --seed 7 --rounds 5`, 3945 cases in 482 seconds, 0 disagreements. An
  earlier run of the same script (`--seed 20261111 --rounds 12`, 9468 cases) also had none. Every kind at
  every size it holds was sealed and read on both sides; the two writers made the same envelope byte for
  byte for every valid input and refused the same invalid ones, and the readers gave the same outcome, down
  to the layout rule, for every corrupted code. Not generated: the 1 GiB memory step, a code where two
  kinds' tags match (random inputs do not reach it), and a raw key of the wrong length (the Rust item
  cannot hold one).
- **Fuzzing:** the five `aqrv_*` targets in `fuzz/`, 600 seconds each, no crash and no failed assertion;
  the numbers are in `fuzz/README.md`, "2026-11-11: the Argon QR Vault's targets".

This shows the two readers agree and the Rust one does not crash on what it is given. It is not
independent proof that either is right: `check.py` is a second reader by Faraday's own authors, written
from the document with the generator unread, and an outside implementation is
still what the AQRV specification asks for.
