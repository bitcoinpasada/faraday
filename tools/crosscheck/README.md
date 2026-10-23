# The crypto cross-check

F79, `docs/MULTIVENDOR.md` §10. Faraday's own Rust answers requests, and
independent pure-Python references recompute every answer, over every
published vector and a corpus of random inputs drawn from a fixed seed.
The first disagreement stops the area and is printed with its inputs
and both answers.

## Running it

    just crosscheck              the stamp run: fixed seed, about 3 to 4 minutes
    just crosscheck --fresh      before a release: a new seed, ten times the cases
    just crosscheck --area musig one area (no stamp is written)
    just crosscheck --seed S     reproduce a run

`run.sh` builds the Rust command with `cargo build --frozen` and runs
`crosscheck.py` in a network namespace of its own (`unshare --user
--net`), so nothing can be fetched. `crosscheck.py` refuses to run with
a network interface up, and refuses when a vendored reference does not
match `references.sha256`, or when an unlisted `.py` file sits in a
reference folder.

A passing full run writes `stamp`. `tools/lint-crosscheck.sh` (in `just
lint`, about a tenth of a second) hashes the crypto sources, this
folder, the references, the vectors and the crypto crates' `Cargo.lock`
entries, and fails with "crypto changed since the last cross-check: run
`just crosscheck`" when they no longer match the stamp. Commit the stamp
with the change that made it pass. `python3 stamp.py show` lists what is
hashed.

Under `--fresh`, a disagreement is kept in `found/` with its seed and
inputs, to become a vector.

## The self-test

Before every run, each area runs against the Rust command started with
`FARADAY_CROSSCHECK_SABOTAGE=<op>`, which changes every answer to that
one operation. The run fails unless every area reports that operation
as a disagreement: a cross-check that cannot fail is worse than none.

## What runs

| Area | Faraday | Reference (`tools/reference/`) | Published vectors | Random corpus |
|---|---|---|---|---|
| `musig` | `faraday_bip::musig` | BIP-327's `reference.py` (`bip327/`) | all eight files of `tools/vectors/bip327/` | sessions of 1 to 3 signers: key sort and aggregation, tweaks, nonces, partial signatures, their verification (and a tampered one), aggregation checked as BIP-340, deterministic signing |
| `frost` | `faraday_bip::frost` | BIP 445's `frost_ref` (`bip445/`) | all six files of `tools/vectors/bip445/` | the dealer against the reference's public-share interpolation, then signing sessions as for MuSig2 |
| `sp` | `faraday_bip::silent` | BIP-352's `reference.py` (`bip352/`) | every case of `tools/vectors/bip352/`, both sides | inputs of every kind BIP-352 reads, receivers with labels, a sender's outputs and what each receiver finds among them and decoys |
| `bip85` | `faraday_bip::bip85` | `ethankosakovsky/bip85` over python-mnemonic and a hardened-BIP-32 shim (`bip85/`, `bip39/`) | every application vector of `tools/vectors/bip85/` | random master keys, indices, languages, word counts and lengths, with refusals |
| `slip39` | `faraday_bip::slip39` | python-shamir-mnemonic (`slip39/`) | all 45 of `tools/vectors/slip39/` | splits fed the same random bytes, which must match word for word; recovery from qualifying and short subsets and a changed word; the passphrase cipher |
| `codex32` | `faraday_bip::codex32` | python-codex32 (`codex32/`); the worksheet's rows written in the area from its polymod | every vector of `tools/vectors/bip93/`, the 55 invalid strings included; volvelle-website's worksheet cells (`tools/vectors/codex32-volvelle/`); the booklet's checksum, derivation, recovery, addition, translation and fusion tables (`tools/vectors/codex32-booklet/`); the Codex32QR profile's sets (`tools/vectors/codex32qr/`) | secrets, splits (the same randomness), recovery, derived shares, corrupted strings; fresh sets of every threshold up to 31 strings at 48, 74 and 127 characters, the checksum worksheets of their strings and of corrupted ones, checksums made on a sheet, translation and recovery symbols, translation worksheets |
| `xor` | `faraday_entropy::SeedXor` with `faraday_bip::bip39` | python-mnemonic and a byte XOR (`bip39/`) | Coldcard's two examples (`tools/vectors/xor/coldcard.json`) and this tree's two | splits and combines in all ten languages and five lengths, with refusals |

Where a reference leaves a rule to its caller, the area writes the
BIP's own sentence and says so in its docstring: the codex32 threshold
and seed sizes, the BIP-85 length ranges and its two password
applications, BIP-352's skip of a transaction spending SegWit version 2
or above.

### Known differences

Each is a place where Faraday refuses what the reference accepts; the
corpus keeps out of it and the area says why.

- SLIP-39: Faraday refuses a group threshold of 1 over several groups;
  python-shamir-mnemonic makes the shares. `area_slip39.py` checks that
  this still holds, so a change either way is seen.
- Codex32: Faraday recovers only from exactly `k` shares without the
  secret, and derives a share only from exactly `k` strings at an unused
  index; the reference also takes more strings or a used index.
- BIP-445: the vectors' tweak lists whose flag list has another length
  are skipped; each of Faraday's tweaks carries its own flag.
- BIP-352: Faraday's sender arithmetic is test-only and has no K_max,
  so the vector that exceeds it is checked on the receiving side only.

## What the machine needs

`python3` 3.11 or later (the standard library only: `tomllib`,
`hashlib`), `unshare` from util-linux with unprivileged user namespaces
allowed, and the Rust toolchain. Nothing is installed with pip. The
vault areas of §10.5 batch 2 will need the distribution's Argon2 and
libsodium bindings (`python3-argon2`, `python3-nacl`).

## Files

| File | What |
|---|---|
| `Cargo.toml`, `src/` | `faraday-crosscheck`, the Rust command: one JSON request a line in, one answer out; never shipped |
| `run.sh` | builds the command offline and runs `crosscheck.py` with no network |
| `crosscheck.py` | the self-test, the areas in parallel, the stamp |
| `common.py` | the Rust process, the comparison, the reference check |
| `area_*.py` | one area each |
| `references.sha256` | every vendored file the areas import or read, with its SHA-256 |
| `stamp.py`, `stamp` | the stamp and its check |
