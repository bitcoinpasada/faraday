# Two implementations of every cryptographic step

**Kind:** Spec
**Status:** Option A decided (F79) and built, §10.5 items 1 to 4; options B and C deferred
**Open:** what §10.5 item 4 leaves (PSBT signing and finalizing end to end, taproot outputs with script trees, miniscript); B and C wait for the owner (§9).

## 1. The idea

Every cryptographic result Faraday shows or acts on comes from Faraday's
code. The owner's idea: compute each result a second time with an
independent implementation, written by other people, in another language
where possible, and show next to the result which implementations produced
it and whether they agree. A bug in one implementation, or a tampered
dependency, then shows as a disagreement instead of a wrong address, a
wrong key or a wrong signature.

## 2. What Faraday is made of

Faraday is partly its own Rust and partly libraries. Which part a
result comes from decides what a second implementation has to differ from.

| Part | Where it comes from |
|---|---|
| BIP-39 words to seed, BIP-85, SLIP-39, codex32 (BIP-93), Seed XOR, aezeed, MuSig2 (BIP-327, 328, 390), FROST (BIP-445), silent payments (BIP-352), the descriptor and wallet-policy layer | Faraday's own Rust, in `faraday-bip` |
| Transactions, PSBT (BIP-174, 370), BIP-32 derivation, sighash (BIP-143, 341), addresses, script | `rust-bitcoin` 0.32 |
| Descriptors and miniscript parsing and satisfaction | `rust-miniscript` 12 |
| ECDSA and Schnorr signing and verifying, all curve arithmetic | `rust-secp256k1`, which wraps Bitcoin Core's C library `libsecp256k1` |
| SHA-2, HMAC, PBKDF2, Argon2id, ChaCha20-Poly1305 | the RustCrypto crates |

Most "other" Bitcoin libraries also call `libsecp256k1`: libwally,
python-bitcoinlib, python-bitcointx, and embit when its C binding is
present. A second implementation that uses it agrees with Faraday on the
curve by construction and checks only the layers above. Real independence
at the curve needs one of the implementations that do their own arithmetic.

## 3. Candidate second implementations

| Candidate | Language | Curve arithmetic | Covers | Notes |
|---|---|---|---|---|
| Bitcoin Core's functional-test framework (`test_framework/key.py`, `script.py`, `messages.py`, `descriptors.py`) | Python | its own, pure Python | ECDSA with RFC 6979, BIP-340 Schnorr, every sighash, transaction serialisation, descriptor checksums | Written by Core's developers; no dependencies; slow |
| The BIPs' own reference code (BIP-340, 327 MuSig2, 445 FROST, 352 silent payments, 93 codex32) | Python | pure Python | exactly the BIP each comes with | Written by the BIPs' authors: the strongest second opinion for those BIPs |
| `python-mnemonic` (Trezor) | Python | none needed | BIP-39 | The BIP's reference implementation |
| `python-shamir-mnemonic` (Trezor) | Python | none needed | SLIP-39 | The SLIP's reference implementation |
| embit | Python | pure Python fallback, or `libsecp256k1` | BIP-32, 39, PSBT, descriptors, miniscript, addresses | Used by SeedSigner and Specter; runs on MicroPython (not yet checked whether its pure fallback does) |
| noble-curves, scure-bip32/39, btc-signer | JavaScript | its own, audited | the same set as embit | Needs a JavaScript engine (QuickJS is about 1 MB) |
| btcd / btcec | Go | its own | the same set | A static binary; a large one |
| `argon2-cffi`, PyNaCl | Python over C | n/a | Argon2id (the PHC reference C), XChaCha20-Poly1305 (libsodium) | Independent of RustCrypto; `tools/vault/open.py` already opens a vault this way |

## 4. What can be computed twice

| Operation | Standards | Needs a secret | Second implementation |
|---|---|---|---|
| Seed from words | BIP-39 | yes | python-mnemonic |
| Master fingerprint, account xpub, child keys | BIP-32, 44, 48, 49, 84, 86 | from a seed: yes; from an xpub: no | Core's `key.py` plus a BIP-32 routine, or embit |
| Addresses of a wallet | BIP-380 to 386, 388, 389 | no | Core's `descriptors.py` and `script.py`, or embit |
| Descriptor checksum | BIP-380 | no | Core's `descriptors.py` |
| Sighash of each input | BIP-143, 341, 342 | no | Core's `script.py` |
| Signature verification | ECDSA, BIP-340 | no | Core's `key.py` |
| Signature recomputation (the nonce check, done twice) | RFC 6979, BIP-340 | yes | Core's `key.py`: Faraday signs deterministically, so the signatures must match byte for byte |
| Transaction id, finished transaction | BIP-141, 144 | no | Core's `messages.py` |
| MuSig2 aggregate key, partial signature verification | BIP-327, 328, 390 | no | the BIP's reference code |
| FROST group key, partial signature verification | BIP-445 | no | the BIP's reference code |
| Silent payment address, output detection | BIP-352 | scan key: yes | the BIP's reference code |
| Child seeds and passwords | BIP-85 | yes | its reference implementation |
| SLIP-39, codex32, Seed XOR combine | SLIP-39, BIP-93 | yes | the references above |
| Signed message | BIP-322, 137 | signing: yes; checking: no | Core's framework for BIP-322 |
| Vault cost and seal | Argon2id, HKDF, XChaCha20-Poly1305 | yes | argon2-cffi, PyNaCl |

Not worth computing twice: drawing randomness (only its conversion from
dice, coins or cards is deterministic, and that can be checked), reading QR
codes, and the screens.

## 5. Where the second implementation runs

**A. At build time, in `just`.** A differential test runs Faraday and
the Python references side by side over many random inputs and every
published vector: words, paths, descriptors, transactions, signatures.
It adds nothing to the stick, costs nothing at run time, and catches most
implementation bugs before a build. It does not catch a dependency changed
after the build, or a fault on the machine at hand.

**B. At run time, public data only.** A verifier process on the stick,
started by the app, with its own user, no files and no network: it
receives public inputs over a pipe and returns its results. Every check in
§4 that needs no secret runs here, which covers addresses, checksums,
sighashes, signature verification, transaction ids and the MuSig2 and
FROST public steps.

**C. At run time, secrets too.** The same process also receives seeds and
private keys, for seed to fingerprint, signature recomputation, BIP-85 and
the share schemes.

## 6. On screen

Each result that was computed twice carries a small mark beside it:
"✓ Rust · ✓ Python", in green when they agree. Pressing it lists the
implementations, their versions and what each produced. A result computed
once carries "Rust only". A disagreement is red, names both results, and
stops what depends on it: no export, no signing, no loading of the wallet.

## 7. Costs and risks

- **More code trusted.** CPython is about 20 MB in a Buildroot image;
  MicroPython well under 1 MB. Either is more code in the image and in the
  reproducible build, pinned and hashed like everything else.
- **Secrets in a second language.** Python cannot reliably wipe a seed:
  strings are immutable and copied by the runtime. The lock cycle still
  ends the process and the kernel zeroes its memory (`docs/history/PLAN.md` §5.3), but
  between locks the seed would sit in a second process's heap. This is the
  cost of option C, and the reason to keep it separate from B.
- **Speed.** Pure-Python curve arithmetic takes tens of milliseconds an
  operation on a laptop and more on a Pi. Enough for the handful of
  results on one screen; too slow for a search over hundreds of addresses
  or a vanity search, which stay single.
- **Agreement is not proof.** Two implementations can share a
  misreading of a BIP; the BIPs' own reference code and published vectors
  are the defence there.

## 8. Recommendation, and what was decided

1. A first: build-time differential tests against the Python references.
   Cheapest, catches the most. **Decided** (F79): §10.
2. B next: the run-time verifier for public data, with the mark on screen.
   **Deferred** (F79).
3. C last, behind a setting, once the owner accepts a second process
   holding seeds. **Deferred** (F79).

The stick's boot partition is 64 MiB (F80), so B or C can still add an
interpreter to the boot file later without a new layout.

## 9. Open questions for the owner

Only for B and C, when they come back:

1. Stick image first, or the desktop app first?
2. MicroPython (small, fewer libraries) or CPython (larger, every library
   in §3)?
3. Should the second implementation ever receive seeds (option C)?

## 10. The build-time cross-check (option A)

Decided 2026-10-22 (owner, F79). Items 1 to 4 of §10.5 are built, in
`tools/crosscheck/` (its README says how to run it, what each area
checks and where Faraday reads a case otherwise than its reference),
with the stamp check `tools/lint-crosscheck.sh`. What item 4 leaves is
listed under it.

### 10.1 What runs

`just crosscheck` is a recipe of its own, not part of `just`. A
test-only Rust command, never shipped in an image or app, reads one
request a line on stdin and answers one line on stdout: the operation,
its inputs and Faraday's result. A Python script sends the requests,
recomputes each result with the references in §3, and stops at the first
disagreement, naming the operation, the inputs and both results.

The inputs are every published vector under `tools/vectors/` and a
corpus of random inputs drawn from a fixed seed, so that a failure is
reproduced by running again. Each area runs in its own process, in
parallel. Secrets in the cross-check are test data only.

### 10.2 When it runs

- **The stamp.** A passing run writes a stamp file beside the
  cross-check: one hash over the crypto sources and the crypto crates'
  entries in `Cargo.lock`, and the versions of the references and the
  Python interpreter it ran with. `just lint` hashes the same inputs and
  fails when they do not match the stamp, with "crypto changed since the
  last cross-check: run `just crosscheck`". The check costs about a
  second; the cross-check itself runs only when it fails. The stamp is
  committed with the change that made it pass. It is a record, not a
  security control.
- **What the stamp covers.** `core/faraday-crypto`, `core/faraday-bip`,
  `core/faraday-psbt`, `core/faraday-entropy`, `core/faraday-backup`,
  `faraday/faraday-vault`, the signing and derivation code in
  `faraday/faraday-keys`, and the
  `Cargo.lock` entries of rust-bitcoin, rust-miniscript, secp256k1 and
  the RustCrypto crates. A version bump of one of those crates counts as
  a crypto change.
- **Before a release:** `just crosscheck --fresh`, with a new random
  seed and about ten times the random cases. A disagreement it finds
  keeps its seed and inputs as a new vector.

### 10.3 How long

Each run has a time budget, and each area's corpus is sized to fit it,
so adding an area does not lengthen the run much.

- The stamp run, all fifteen areas in parallel on the build machine
  (2026-10-23, shared with other work): 184 s, the slowest area MuSig2
  at 181 s; the batch 2 to 4 areas 27 to 160 s each (vault 61 s, of
  which 26 whole vaults at 8 MiB of Argon2id; BIP-39 144 s; signed
  messages 160 s). Pure-Python curve arithmetic takes about 5 to 30 ms
  an operation, so an area gets a few thousand random cases.
- The run before a release: about ten times the cases, estimated at 30
  to 60 minutes.

### 10.4 Offline, and pinned

The cross-check fetches nothing, ever.

- **The pure-Python references** (the BIPs' reference code, Bitcoin
  Core's functional-test framework, `python-mnemonic`,
  `python-shamir-mnemonic`) are copied into `tools/reference/`, each with
  its source (the git commit in the BIPs repository, Core's release tag,
  Trezor's release) and its SHA-256. The cross-check refuses to run when
  a file does not match its hash. Adding or updating one is a reviewed
  commit. A first copy is checked against something other than the
  web's certificates: the BIPs repository's commit hash, Core's signed
  tag, Trezor's signed release.
- **Argon2id and XChaCha20-Poly1305** come from the Linux
  distribution's packages (argon2-cffi and PyNaCl: `python3-argon2`,
  `python3-nacl` or their equivalents), installed once on the machine
  that runs the cross-check and verified by the distribution's
  signatures, not by pip. Their versions are in the stamp.
- **Bitcoin Core's functional-test framework** is copied from the
  release tag `v31.1` (`tools/reference/core/`); BIP-32, BIP-137,
  BIP-322 and the PSBT BIPs' readers are written in the areas from the
  BIPs' text, each checked against the BIPs' own vectors first.
- **No network.** Python runs in a network namespace with no
  interfaces, and the Rust command is built with `--offline --frozen`.
  Anything that tries to fetch fails instead of downloading.

### 10.5 Order of work

1. Faraday's own Rust, which no other library checks: MuSig2 (BIP-327,
   328, 390), FROST (BIP-445), silent payments (BIP-352), BIP-85,
   SLIP-39, codex32 (BIP-93) and Seed XOR. Built.
2. The vault: Argon2id, HKDF, XChaCha20-Poly1305 and the vault's own
   construction (`docs/VAULT.md`). Built (area `vault`).
3. BIP-39 and BIP-32 derivation. Built (areas `bip39`, `bip32`).
4. What comes from rust-bitcoin and rust-miniscript: sighash, PSBT,
   addresses, descriptor checksums, transaction ids, signed messages.
   Built (areas `addr`, `tx`, `psbt`, `msg`), except: PSBT signing and
   finalizing end to end (`faraday_psbt::sign`, `finalize`), whose
   nonces and sighashes are checked separately but not the signed PSBT;
   taproot outputs with script trees (BIP-341's `scriptPubKey` vectors);
   and miniscript's scripts and satisfactions. Not scheduled.

### 10.6 What it does not catch

A library changed after the build, or a fault on the machine that
signs; only option B catches those. A misreading of a BIP that both
implementations share; the BIPs' own reference code and the published
vectors are the defence there.
