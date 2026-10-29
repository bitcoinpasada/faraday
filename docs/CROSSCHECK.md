# A second implementation of each cryptographic step

**Kind:** Spec
**Status:** Option A decided (F79) and built, §10.5 items 1 to 6; options B and C deferred
**Open:** the references of Faraday's own replaced by outside code (§10.7), planned; B and C wait for the owner (§9).

## 1. The idea

Every cryptographic result Faraday shows or acts on comes from Faraday's
code. The owner's idea: compute each result a second time with an
independent implementation, written by other people, in another language
where possible, and show next to the result which implementations produced
it and whether they agree. A bug in one implementation, or a tampered
dependency, then shows as a disagreement instead of a wrong address, a
wrong key or a wrong signature.

Today the cross-check meets this only in part: about half its areas
compare Faraday's Rust with code other people wrote, and the rest with
Python models Faraday's own authors wrote, which are not independent
(F160). §10.7 says which and what replaces them.

## 2. What Faraday is made of

Faraday is partly its own Rust and partly libraries. Which part a
result comes from decides what a second implementation has to differ from.

| Part | Where it comes from |
|---|---|
| BIP-39 words to seed, BIP-85, SLIP-39, codex32 (BIP-93), Seed XOR, aezeed, MuSig2 (BIP-327, 328, 390), silent payments (BIP-352), the descriptor and wallet-policy layer | Faraday's own Rust, in `faraday-bip` |
| Transactions, PSBT (BIP-174, 370), BIP-32 derivation, sighash (BIP-143, 341), addresses, script | `rust-bitcoin` 0.32 |
| Descriptors and miniscript parsing and satisfaction | `rust-miniscript` 12 |
| ECDSA and Schnorr signing and verifying, all curve arithmetic | `rust-secp256k1`, which wraps Bitcoin Core's C library `libsecp256k1` |
| SHA-2, HMAC, PBKDF2, Argon2id, ChaCha20-Poly1305 | the RustCrypto crates |

Most "other" Bitcoin libraries also call `libsecp256k1`: libwally,
python-bitcoinlib, python-bitcointx, and embit when its C binding is
present. A second implementation that uses it agrees with Faraday on the
curve by construction and checks only the layers above. Real independence
at the curve needs one of the implementations that do their own arithmetic.

The lineage is narrower still. libsecp256k1's developers (Pieter Wuille
among them) also wrote
Core's test framework's `key.py`, `secp256k1lab` and the BIP-340, 327 and
352 reference code, so every curve check today sets code from one small
group of authors against more of its own. Every miniscript
implementation, rust-miniscript, Core's and python-bip380, comes from one
group too. Both are accepted (owner, 2026-10-29, F160); btclib and
python-ecdsa, which share nothing with that group, are to be the second
lineage for the curve (§10.7).

## 3. Candidate second implementations

| Candidate | Language | Curve arithmetic | Covers | Notes |
|---|---|---|---|---|
| Bitcoin Core's functional-test framework (`test_framework/key.py`, `script.py`, `messages.py`, `descriptors.py`) | Python | its own, pure Python | ECDSA with RFC 6979, BIP-340 Schnorr, every sighash, transaction serialisation, descriptor checksums | Written by Core's developers; no dependencies; slow |
| The BIPs' own reference code (BIP-340, 327 MuSig2, 352 silent payments, 93 codex32) | Python | pure Python | exactly the BIP each comes with | Written by the BIPs' authors: the strongest second opinion for those BIPs |
| `python-mnemonic` (Trezor) | Python | none needed | BIP-39 | The BIP's reference implementation |
| `python-shamir-mnemonic` (Trezor) | Python | none needed | SLIP-39 | The SLIP's reference implementation |
| embit | Python | pure Python fallback, or `libsecp256k1` | BIP-32, 39, PSBT, descriptors, miniscript, addresses | Used by SeedSigner and Specter; runs on MicroPython (not yet checked whether its pure fallback does) |
| noble-curves, scure-bip32/39, btc-signer | JavaScript | its own, audited | the same set as embit | Needs a JavaScript engine (QuickJS is about 1 MB) |
| btcd / btcec, LND's `aezeed` | Go | its own | the same set; LND's aezeed with AEZ | Vendored in `tools/reference/lnd/` and built by `run.sh`; the `lightning` area asks it |
| `argon2-cffi`, PyNaCl, PyCA cryptography | Python over C | n/a | Argon2id (the PHC reference C), XChaCha20-Poly1305 (libsodium), HKDF-SHA256 and ChaCha20 (OpenSSL) | Independent of RustCrypto; `tools/vault/open.py` already opens a vault this way |

## 4. What can be computed twice

| Operation | Standards | Needs a secret | Second implementation |
|---|---|---|---|
| Seed from words | BIP-39 | yes | python-mnemonic |
| Master fingerprint, account xpub, child keys | BIP-32, 44, 48, 49, 84, 86 | from a seed: yes; from an xpub: no | Core's `key.py` plus a BIP-32 routine (today `btc.py`, Faraday's own), or embit, btclib or Core itself |
| Addresses of a wallet | BIP-380 to 386, 388, 389 | no | Core's `descriptors.py` and `script.py`, or embit |
| Descriptor checksum | BIP-380 | no | Core's `descriptors.py` |
| Sighash of each input | BIP-143, 341, 342 | no | Core's `script.py` |
| Signature verification | ECDSA, BIP-340 | no | Core's `key.py` |
| Signature recomputation (the nonce check, done twice) | RFC 6979, BIP-340 | yes | Core's `key.py`: Faraday signs deterministically, so the signatures must match byte for byte |
| Transaction id, finished transaction | BIP-141, 144 | no | Core's `messages.py` |
| MuSig2 aggregate key, partial signature verification | BIP-327, 328, 390 | no | the BIP's reference code |
| Silent payment address, output detection | BIP-352 | scan key: yes | the BIP's reference code |
| Child seeds and passwords | BIP-85 | yes | its reference implementation |
| SLIP-39, codex32, Seed XOR combine | SLIP-39, BIP-93 | yes | the references above |
| Codex32 sets made (secret and derived shares, checksum worksheets) and error correction | BIP-93 | yes | BIP 93's reference code and python-codex32 (`tools/crosscheck/area_codex32.py`, `tools/crosscheck/area_codex32_correct.py`) |
| Signed message | BIP-322, 137 | signing: yes; checking: no | Core's `signmessagewithprivkey` and btclib for BIP-137; Core has no BIP-322, which needs an outside implementation of its own (§10.7) |
| Vault cost and seal | Argon2id, HKDF, XChaCha20-Poly1305 | yes | argon2-cffi, PyNaCl, PyCA `cryptography`, for the primitives; the container is Faraday's own construction and has no outside implementation |

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
sighashes, signature verification, transaction ids and the MuSig2
public steps.

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
  are the defence there. Where the reference is a model Faraday's own
  authors wrote, a shared misreading is likelier, and the published
  vectors are the only defence (§10.7).

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

Decided 2026-10-22 (owner, F79). Items 1 to 6 of §10.5 are built, in
`tools/crosscheck/` (its README says how to run it, what each area
checks and where Faraday reads a case otherwise than its reference;
the table and the known differences in it are written by `facts.py`
from the facts each area states about itself, and `just lint` checks
them),
with the stamp check `tools/lint-crosscheck.sh`. What item 6 leaves is
listed under it.

Not every area compares against code Faraday did not write (F160). Each
action of `docs/crypto/actions.toml` with an area states its reference's
`independence`: `outside`, code other people wrote, used as published;
`own-over-outside`, a model Faraday's authors wrote over outside code,
with the part that is Faraday's own named; or `own-only`, a model
Faraday's authors wrote over the standard library alone. Only the first
is an independent check. The cross-check's README ("Who wrote each
reference"), STANDARDS' Verified by cells and the app's and the
website's Cryptography blocks are generated from it, and
`docs/crypto/verified.py` fails an action with an area and no class.
As of 2026-10-29:

- **Outside:** `bip39`, `codex32`, `codex32-correct`, `slip39`, `musig`,
  `sp`, `tx`, `ur`, `kdbx`, the QR encoder in `seedqr`, and the address
  text in `addr`.
- **Faraday's own model over outside code:** `bip32` (`btc.py`), the keys
  under `addr`, `miniscript`, `vanity` and `lightning`, BIP-86's tweak,
  SLIP-132's versions, `psbt` (the whole reader and writer), `sign` (the
  signer, the finalizer, the leaf chosen, the refusals), `msg` (BIP-137
  and BIP-322 written out), `taproot`'s control-block check, `bip85`'s
  hardened shim, the codex32 worksheets, and `vault` (the container,
  HKDF and the ChaCha20 block, over argon2-cffi and libsodium).
- **Faraday's own model only:** `own-randomness`, `entropy` and `roller`.

The vault's container, own randomness, cards, the camera's commitment,
a mix, the Demo roller, the in-memory seal and the Secure Boot generator
are Faraday's own constructions: no outside implementation of them can
exist, and each carries the label "own construction, not yet
independently reviewed" until an outside review (F160, STANDARDS §7).

### 10.1 What runs

`just crosscheck` is a recipe of its own, not part of `just`. A
test-only Rust command, never shipped in an image or app, reads one
request a line on stdin and answers one line on stdout: the operation,
its inputs and Faraday's result. A Python script sends the requests,
recomputes each result with the area's reference (code from §3, or a
model of Faraday's own, §10), and stops at the first disagreement,
naming the operation, the inputs and both results.

The inputs are every published vector under `tools/vectors/` and a
corpus of random inputs drawn from a fixed seed, so that a failure is
reproduced by running again. Secrets in the cross-check are test data
only.

Every corpus case draws from a random stream of its own, seeded with the
run's seed, its area, its loop and its number (owner, 2026-10-27: until
then each area drew all its cases from one stream, and seeds from before
reproduce on their own release tags). Each area is cut into batches, its
published vectors and its corpus loops by case number, and a pool of
workers, one a core unless told otherwise, takes them from one queue,
the heaviest areas' first, so every core is busy until the end. The
answers are put back in case order: an area's result is its first
batch in case order that did not pass, as one worker running every case
in turn would give it, so the report, the counts and the verdict are the
same for any number of workers and under CPython or PyPy, and a run
anyone makes with a release's seed and scale gives that release's
report. The run writes its progress as it goes (`out/crosscheck/`,
`just crosscheck-status`); the README says how to watch it, how much
memory a worker takes and how to run it under PyPy.

An area whose cases spend on Bitcoin Core's node (§10.4) has its coins
before the self-test. The node's coinbases pay P2WSH(OP_TRUE) and are
fanned out into coins of one value, the same in every run, as a regtest
coinbase's txid depends only on its height and what it pays. The
workers build each case's funding transactions from the case's own
random stream and the run sends and buries them all (for `sign`, 1,100
blocks, past every timelock its corpus spends with). A case then builds
the same funding again, so the transaction Faraday finalizes spends real
outputs, and Core's `testmempoolaccept` judges it with its own script
interpreter and mempool policy.

### 10.2 When it runs

- **The stamp.** A passing run writes a stamp file beside the
  cross-check: one hash over the crypto sources and the crypto crates'
  entries in `Cargo.lock`, and the versions of the references and the
  Python interpreter it ran with. `just lint` hashes the same inputs and
  fails when they do not match the stamp, with "crypto changed since the
  last cross-check: run `just crosscheck`". The check costs about a
  second; the cross-check itself runs only when it fails. The stamp is
  committed with the change that made it pass. It is a record, not a
  security control. It also holds each area's corpus case count for
  that run, one `cases <area> <count>` line each.
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

- The stamp run, on twelve workers on the build machine (2026-10-27):
  89 s, about 1,000 s of work; signed messages 116 s of it, BIP-39 77 s,
  taproot trees 82 s, codex32 45 s, MuSig2 35 s (it keeps the
  reference's point multiplications, four times faster with the same
  answers). Pure-Python curve arithmetic takes about 5 to 30 ms an
  operation, so an area gets a few thousand random cases.
- The run before a release: ten times the cases, 465,000 corpus cases,
  under 10 minutes on twelve workers, the slowest area no longer setting
  the length of the run (2026-10-29: about 9 minutes, 6,300 s of work, of
  which the funding of `sign`'s cases on Core's node is 20 s; signed
  messages 1,150 s, taproot trees 620 s, BIP-39 90 s, since
  python-mnemonic is given its wordlists with a table to find a word's
  place, and the BIP-32 keys of `btc.py` work out a parent's fingerprint
  only when it is read).

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
- **Argon2id, XChaCha20-Poly1305, HKDF-SHA256 and ChaCha20** come from the
  Linux distribution's packages (argon2-cffi, PyNaCl and PyCA
  cryptography: `python3-argon2`, `python3-nacl`, `python3-cryptography`
  or their equivalents), installed once on the machine
  that runs the cross-check and verified by the distribution's
  signatures, not by pip. Their versions are in the stamp.
- **python-bip380**, Antoine Poinsot's miniscript, is copied from its
  signed commit (`tools/reference/miniscript/README.md`), the signature
  checked against the key Bitcoin Core's `guix.sigs` publishes; two
  shims stand in for the libsecp256k1 packages it imports.
- **Bitcoin Core's functional-test framework** is copied from the
  release tag `v31.1` (`tools/reference/core/`); BIP-32 is written in
  the areas from the BIP's text, checked against the BIP's own vectors
  first.
- **HWI's PSBT reader** (`hwilib/psbt.py` and the modules it imports) is
  copied from a signed commit on HWI's `master`, the signature checked
  against the key Bitcoin Core's `guix.sigs` publishes
  (`tools/reference/hwi/README.md`, which says why not its last
  release); a shim stands in for `typing_extensions`.
- **btclib, ecc and btclib-wallet** are their published wheels, byte for
  byte, pinned by the SHA-256 btclib-wallet's `uv.lock` names and
  compared with the tagged commits (`tools/reference/btclib/`); the run
  unpacks them after the hash check and uses btclib's own curve, never
  libsecp256k1.
- **btcd's BIP-322** (pull request 2521, unmerged, at its head commit)
  is a Go module vendored with `go mod vendor`
  (`tools/reference/btcd-bip322/`), built by `run.sh` with
  `GOFLAGS=-mod=vendor`, `GOPROXY=off` and `GOTOOLCHAIN=local`.
- **Bitcoin Core v31.1 itself** (`bitcoind` and `bitcoin-cli`) is a
  binary reference, required (owner, 2026-10-29). Its release tarball
  is over the tree's 2 MiB limit and is not in the tree, nor in Ubuntu's
  packages. So the official tarball for the machine (x86_64 or aarch64
  Linux) is pinned by its SHA-256 in `tools/reference/core/release.pin`,
  listed in `references.sha256` like a vendored file. A person fetches
  it once with `tools/crosscheck/fetch_core.py`, which requires the
  release's `SHA256SUMS` to carry good signatures from at least five of
  the builders whose keys `guix.sigs` publishes (at a pinned commit), and
  no bad one, and the tarball to match both `SHA256SUMS` and the pin. It
  is kept outside the repository, in the folder `FARADAY_BITCOIN_CORE`
  names (default `~/.cache/faraday/bitcoin-core`). Every run hashes the
  tarball against the pin and unpacks the two programs from it into a
  scratch folder, so what runs is what was checked. A missing or
  mismatched tarball fails the run, naming it. The node runs on regtest
  with `-networkactive=0 -listen=0 -connect=0`, no seeds, no wallet and
  a scratch datadir, RPC on loopback only, with a cookie. It is stopped and
  its folders deleted on every way out of the run, and killed if the
  run dies. The stamp records Core's version and the tarball's SHA-256,
  and `just lint` fails when they are not the pinned release's.
- **No network.** Python runs in a network namespace whose only
  interface is loopback, which the run brings up for Core's RPC, and
  refuses to run when any other interface is there. The Rust command is
  built with `--offline --frozen`. Anything that tries to fetch fails
  instead of downloading.

### 10.5 Order of work

1. Faraday's own Rust, which no other library checks: MuSig2 (BIP-327,
   328, 390), silent payments (BIP-352), BIP-85,
   SLIP-39, codex32 (BIP-93) and Seed XOR. Built.
2. The vault: Argon2id, HKDF, XChaCha20-Poly1305 and the vault's own
   construction (`docs/VAULT.md`). Built (area `vault`).
3. BIP-39 and BIP-32 derivation. Built (areas `bip39`, `bip32`).
4. What comes from rust-bitcoin and rust-miniscript: sighash, PSBT,
   addresses, descriptor checksums, transaction ids, signed messages,
   PSBT signing and finalizing end to end (`faraday_psbt::sign`,
   `finalize`), taproot outputs with script trees (BIP-341's
   `scriptPubKey` vectors), and the scripts miniscript descriptors pay
   to and their satisfactions, against Antoine Poinsot's python-bip380
   and Bitcoin Core's miniscript vectors. Built (areas `addr`, `tx`,
   `psbt`, `msg`, `sign`, `taproot`, `miniscript`).
5. Randomness a person adds of their own, mixed into the session's
   (`docs/OWN-RANDOMNESS.md` §2.3): the accumulator, the reseeding, the
   pool's and the seed's draws and the lock, over fixed system answers.
   Built (area `own-randomness`).
6. What `docs/STANDARDS.md` §0 asks of the rest: the entropy procedures
   (area `entropy`), Faraday Demo's dice rollers (`roller`), SeedQR
   and the QR encoder (`seedqr`), BBQr and numbered parts (`bbqr`), UR
   (`ur`), the vanity search (`vanity`), the KDBX export read by
   pykeepass (`kdbx`), the Lightning node keys and the aezeed read by
   LND's own Go code (`lightning`, vendored in `tools/reference/lnd/`),
   and BIP 380's own checksum code beside Core's in `addr`. Built.

### 10.6 What it does not catch

- A library changed after the build, or a fault on the machine that
  signs; only option B catches those.
- A misreading of a BIP that both implementations share; the BIPs' own
  reference code and the published vectors are the defence there.
- A misreading shared by Faraday's Rust and a Python model written by
  Faraday's own authors, which is likelier than between strangers. Where
  the reference is Faraday's own (§10), the published vectors are the
  only defence, and an action with no vectors has none.
- A flaw in a spec Faraday wrote itself (the vault, own randomness, the
  entropy procedures, the Demo roller): both sides follow the same text.
  Only an outside review finds it.
- A blind spot of the corpus generators, which Faraday's authors wrote
  too: what they never ask is never compared.

### 10.7 Replacing the references of Faraday's own

Planned (owner, 2026-10-29, F160), in order of what makes or checks keys,
addresses and signatures first. Each step replaces a model of Faraday's
own with code other people wrote, moves the action's `independence` to
`outside`, and retires the model once nothing needs it.

1. **Bitcoin Core as a program**, required (§10.4). Built for `sign`
   (2026-10-29): `descriptorprocesspsbt` signs every PSBT of its vectors
   and corpus from the held masters' extended private keys, so Core
   derives the keys and chooses where each signs, and its ECDSA
   signatures are compared byte for byte; `finalizepsbt` finalizes and
   extracts, compared byte for byte; `testmempoolaccept` judges every
   transaction Faraday finalizes over funding on the node (§10.1). Built
   for `addr` (2026-10-29): `deriveaddresses` from the seed's master.
   Planned: `deriveaddresses` for `bip32`, `miniscript` and `vanity`;
   `decodepsbt` for version 0 PSBTs; `signmessagewithprivkey` and
   `verifymessage` for BIP-137 with P2PKH. The refusals of F136 and F141
   are Faraday's policy, which Core does not share, and stay in Faraday's
   model, labelled so.
2. **btclib** (vendored by commit and SHA-256): BIP-32 derivation and
   strict decoding, BIP-137's other headers, and BIP-340 with its
   randomness given. **python-ecdsa** (the distribution's package): every
   ECDSA signature again under RFC 6979, byte for byte. Both outside the
   libsecp256k1 lineage (§2).
3. **HWI's `psbt.py`** (vendored): version 2 PSBTs, which Core refuses.
4. **PyCA `cryptography`** (the distribution's package): the vault's
   HKDF-SHA256 and the ChaCha20 block.
5. **embit and SeedSigner's own code** (vendored): SeedQR and numbered
   parts; **Coldcard's code**, where its licence allows: the dice
   procedure and Seed XOR.
6. **LND's aezeed and AEZ in Go** (a Go toolchain, offline): the second
   AEZ §10.5 item 6 lacks, and LND's node keys.
7. **BIP-322:** against an outside implementation if a reasonable one
   exists; if none does, BIP-322 is held out of the app until it can be
   checked, never labelled self-checked. Built: btclib-wallet's `bip322`
   and btcd's (pull request 2521, Go) answer every case of the `msg`
   area, with the BIP's generated vectors and btcd's negative ones; with
   Core's `signmessagewithprivkey` and `verifymessage` and btclib's `bms`
   for BIP-137 (the message parts of items 1 and 2), they replaced
   `area_msg.py`'s own BIP-137 and BIP-322.
8. **Faraday's own constructions** (the vault's container, own
   randomness, cards, the camera, a mix, the Demo roller): a model
   written by an outside party from the spec alone, or a named outside
   party's reproduction of the published vectors (`tools/vectors/vault/`
   and the others), recorded on the Audits page
   (`docs/AUDIT-SUBMISSIONS.md`). Until then each keeps its label. The
   call, with the spec sections, the vectors and what a model offers to
   take a model's place in an area, is `docs/AUDIT-SUBMISSIONS.md` §10.
