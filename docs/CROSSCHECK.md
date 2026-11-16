# A second implementation of each cryptographic step

**Kind:** Reference
**Status:** Option A decided (F79) and built, §10.5 items 1 to 6; the references of Faraday's own replaced by outside code where it exists (§10.7)

## 1. The idea

Every cryptographic result Faraday shows or acts on comes from Faraday's
code. The owner's idea: compute each result a second time with an
outside implementation, written by other people, in another language
where possible, and show next to the result which implementations produced
it and whether they agree. A bug in one implementation, or a tampered
dependency, then shows as a disagreement instead of a wrong address, a
wrong key or a wrong signature.

Every area of the cross-check compares Faraday's Rust with code other
people wrote; where a part has no outside implementation, a second one
Faraday's own authors wrote stands beside it and is named as such (F160,
F177). §10.7 says which parts, and what replaced the rest.

## 10. The build-time cross-check (option A)

Decided 2026-10-22 (owner, F79). Items 1 to 6 of §10.5 are built, in
`tools/crosscheck/` (its README says how to run it, what each area
checks and where Faraday reads a case otherwise than its reference;
the table and the known differences in it are written by `facts.py`
from the facts each area states about itself, and `just lint` checks
them),
with the stamp check `tools/lint-crosscheck.sh`. What item 6 leaves is
listed under it.

Not every area compares against code Faraday did not write (F160,
F177). Each action of `docs/crypto/actions.toml` with an area states, in
its `independence`, what its cross-check compares Faraday with:
`outside`, an implementation other people wrote, used as published;
`outside-and-own`, an outside implementation and, for the part named, a
second implementation Faraday's authors wrote; or `own`, only a second
implementation Faraday's authors wrote, over the standard library alone.
What is compared only with an implementation Faraday's authors wrote is
not checked against an outside source: the check catches a slip between
two implementations written in two languages from the specification,
not a misreading both share. The cross-check's README ("Others' code and
Faraday's second version"), STANDARDS' Verified by cells and the app's and the
website's Cryptography blocks are generated from it, and
`docs/crypto/verified.py` fails an action with an area and no class.
As of 2026-10-29:

- **Outside:** `bip39`, `codex32`, `codex32-correct`, `musig`,
  `sp`, `tx`, `ur`, `kdbx`, `bip85`, `miniscript`, the QR encoder in
  `seedqr`, `addr` (the address text by Core's framework, accounts'
  keys and SLIP-132 by btclib, their addresses by Core itself), and
  `bip32`'s derivation (btclib).
- **Outside and own:** `bip32`'s strict reading
  (which versions it takes), `slip39`'s new member (the refusals before
  the arithmetic), `vanity`'s search, ldk-node's key in
  `lightning`, `psbt` (the whole reader and writer), `sign` (the
  signer, the finalizer, the leaf chosen, the refusals), `msg` (BIP-137
  and BIP-322 written out), `taproot`'s control-block check, the codex32
  worksheets, `compile` (the lift of a miniscript to a formula, written
  from the fragments' meanings; Core, python-bip380 and btclib take the
  descriptor), `sign-check` (the report of a reused nonce; python-ecdsa,
  btclib and Core judge the signatures), and `vault` (the payload
  layouts of `docs/VAULT.md` §7, over argon2-cffi and `unicodedata`).
- **Own only:** none. The session's randomness, the pointer and key
  commitment, cards, the camera's commitment, a mix, the device's draw,
  the Demo roller and the booklet's dice have no area: they are held by
  the frozen vectors and the Rust tests that read them.

The vault's container, own randomness, cards, the camera's commitment,
a mix, the Demo roller, the in-memory seal and the Secure Boot generator
are of Faraday's own specification: no outside implementation of them
can exist, and each carries the label "own specification, not yet
reviewed outside Faraday" until an outside review (F160, F177, STANDARDS
§7).

### 10.1 What runs

`just crosscheck` is a recipe of its own, not part of `just`. A
test-only Rust command, never shipped in an image or app, reads one
request a line on stdin and answers one line on stdout: the operation,
its inputs and Faraday's result. A Python script sends the requests,
recomputes each result with the area's reference (code other people wrote, or a
second implementation Faraday's authors wrote, §10), and stops at the first disagreement,
naming the operation, the inputs and both results.

The inputs are every published vector under `tools/vectors/` and a
corpus of random inputs drawn from a fixed seed, so that a failure is
reproduced by running again. Secrets in the cross-check are test data
only.

How it runs (a random stream for each case, the batches and the pool of
workers, the report the same for any number of workers, PyPy, watching a
run, the funding of the cases that spend on Bitcoin Core's node) is in
`tools/crosscheck/README.md`.

### 10.2 When it runs

- **The tip is not kept cross-checked (F197).** The full cross-check runs
  before a release and on no schedule. A commit between releases may
  change the crypto without one: its stamp and the actions' fingerprints
  are the last release's, and `just` checks neither. A change to an
  area's code runs that area (`just crosscheck --area NAME`) and its
  tests and reviewers as before. What is cross-checked is a release, and
  `crosscheck-<version>.txt` beside its `SHA256SUMS` is the record.
- **The stamp.** A passing run writes a stamp, a record and not a security
  control; what it hashes and what `just lint-release` checks are in
  `tools/crosscheck/README.md`. The stamp is committed with the release.
- **What the stamp covers.** `core/faraday-crypto`, `core/faraday-bip`,
  `core/faraday-psbt`, `core/faraday-entropy`,
  `faraday/faraday-vault`, the signing and derivation code in
  `faraday/faraday-keys`, and the
  `Cargo.lock` entries of rust-bitcoin, rust-miniscript, secp256k1 and
  the RustCrypto crates. A version bump of one of those crates counts as
  a crypto change.
- **Before a release:** `just crosscheck --fresh`, with a new random
  seed and about ten times the random cases. A disagreement it finds
  keeps its seed and inputs as a new vector. Then `just lint-release` must
  pass.
- **A release's run made of two (F239).** The release's cross-check may
  be a combination of runs at one fresh seed and scale; how
  (`just crosscheck --combine RECORD`) and what it refuses are in
  `tools/crosscheck/README.md`, "A release's run made of two".

### 10.3 How long

Each run has a time budget, and each area's corpus is sized to fit it,
so adding an area does not lengthen the run much.

- The stamp run (2026-10-27): about 1,000 s of work, shared among the
  workers; signed messages 116 s of it, BIP-39 77 s,
  taproot trees 82 s, codex32 45 s, MuSig2 35 s (it keeps the
  reference's point multiplications, four times faster with the same
  answers). Pure-Python curve arithmetic takes about 5 to 30 ms an
  operation, so an area gets a few thousand random cases.
- The run before a release: ten times the cases, 465,000 corpus cases,
  the slowest area no longer setting the length of the run (2026-10-29:
  about 6,300 s of work, of
  which the funding of `sign`'s cases on Core's node is 20 s; signed
  messages 1,150 s, taproot trees 620 s, BIP-39 90 s, since
  python-mnemonic is given its wordlists with a table to find a word's
  place).

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
- **Argon2id** comes from the
  Linux distribution's package (argon2-cffi: `python3-argon2`
  or its equivalent), installed once on the machine
  that runs the cross-check and verified by the distribution's
  signatures, not by pip. Their versions are in the stamp.
- **python-ecdsa** (`python3-ecdsa`, by Brian Warner and Hubert Kario; MIT,
  pure Python) is the same kind of package, checked for presence like
  argon2-cffi (the run stops without it) and its version is in the stamp.
  With btclib's curve it is the second lineage beside libsecp256k1
  (`tools/crosscheck/lineage.py`, owner, 2026-10-29): it makes every
  ECDSA signature the `sign` and `msg` areas expect again under plain
  RFC 6979 and verifies the low-R ones; btclib verifies every BIP-340
  signature and makes it again where the auxiliary randomness is known,
  and runs MuSig2 whole (`ecc.ecc.musig2`, with btclib-wallet's BIP-373
  roles) beside BIP-327's reference. Core's `key.py` and `secp256k1lab`
  are still asked.
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
  `GOFLAGS=-mod=vendor`, `GOPROXY=off` and `GOTOOLCHAIN=local`; its vendor
  tree stays because its `go.mod` replaces four modules with folders of the
  unmerged pull request that are not kept, so only the vendor tree holds
  them. LND's and a released btcd's Go programs
  (`tools/reference/lnd/`, `tools/reference/btcd/`) have no vendor tree: the
  Go toolchain fetches their modules into its module cache, each checked
  against `go.sum`, so the cross-check needs the network once per machine
  for that fetch (`GOFLAGS=-mod=readonly`, `GOTOOLCHAIN=local`).
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
  with `-networkactive=0 -listen=0 -connect=0`, no seeds, no wallet unless
  an area asks for one (psbt-review makes a watch-only descriptor wallet
  for each case and deletes it after; the other areas run with wallet
  support off) and a scratch datadir, RPC on loopback only, with a cookie. It is stopped and
  its folders deleted on every way out of the run, and killed if the
  run dies. The stamp records Core's version and the tarball's SHA-256,
  and `just lint` fails when they are not the pinned release's.
  A second, older release (28.2, `tools/reference/core/legacy.pin`) is
  fetched and checked the same way (`fetch_core.py --legacy`); it is run
  by `tools/vectors/core-wallet/gen.py`, to make the legacy wallet
  that the pinned release's `migratewallet` turns into a sample, and by
  the `core_wallet` area, once a run, to import Faraday's before-29
  `importdescriptors` file.
- **No network.** Python runs in a network namespace whose only
  interface is loopback, which the run brings up for Core's RPC, and
  refuses to run when any other interface is there. The Rust command is
  built with `--offline --frozen`. Anything that tries to fetch fails
  instead of downloading.

### 10.5 What is checked, by area

1. Faraday's own Rust, which no other library checks: MuSig2 (BIP-327,
   328, 390), silent payments (BIP-352), BIP-85,
   SLIP-39, codex32 (BIP-93) and Seed XOR. Built.
2. The vault: Argon2id and the vault's own specification
   (`docs/VAULT.md`). Built in part (area `vault`: the payloads, the
   passphrase and Argon2id; the formats against other programs are not).
3. BIP-39 and BIP-32 derivation. Built (areas `bip39`, `bip32`).
4. What comes from rust-bitcoin and rust-miniscript: sighash, PSBT,
   addresses, descriptor checksums, transaction ids, signed messages,
   PSBT signing and finalizing end to end (`faraday_psbt::sign`,
   `finalize`), taproot outputs with script trees (BIP-341's
   `scriptPubKey` vectors), and the scripts miniscript descriptors pay
   to and their satisfactions, against Antoine Poinsot's python-bip380
   and Bitcoin Core's miniscript vectors; and the policy compiler's
   output, which Core, python-bip380 and btclib read and whose
   miniscript is compared with the policy as a boolean formula over the
   keys and waits; and the signatures a PSBT already carries, verified by
   python-ecdsa and btclib over btclib's signature hashes and run by
   btclib's script engine and Core. Built (areas `addr`, `tx`, `psbt`,
   `msg`, `sign`, `sign-check`, `taproot`, `miniscript`, `compile`).
5. Randomness a person adds of their own, mixed into the session's
   (`docs/OWN-RANDOMNESS.md` §2.3): the accumulator, the reseeding, the
   pool's and the seed's draws and the lock, over fixed system answers.
   Not cross-checked: Faraday's own specification, with no other
   implementation to check against; held by the frozen vectors
   (`tools/vectors/own-randomness/`) and the Rust tests that read them.
6. What `docs/STANDARDS.md` §0 asks of the rest: the entropy procedures
   (area `entropy`, for dice, coins and hex; cards, the camera, the
   device's draw and a mix are Faraday's own, held by the frozen vectors
   and the Rust tests that read them, not cross-checked), Faraday Demo's
   dice rollers and the booklet's dice (the same), SeedQR
   and the QR encoder (`seedqr`), BBQr and numbered parts (`bbqr`), UR
   (`ur`), the vanity search (`vanity`), the Lightning node keys and the aezeed read by
   LND's own Go code (`lightning`, in `tools/reference/lnd/`, its modules pinned by `go.sum`),
   and BIP 380's own checksum code beside Core's in `addr`. Built.

### 10.6 What it does not catch

- A library changed after the build, or a fault on the machine that
  signs; the cross-check runs at build time only.
- A misreading of a BIP that both implementations share; the BIPs' own
  reference code and the published vectors are the defence there.
- A misreading shared by Faraday's Rust and a second implementation, in Python, written by
  Faraday's own authors, which is likelier than between strangers. Where
  the reference is Faraday's own (§10), the published vectors are the
  only defence, and an action with no vectors has none.
- A flaw in a spec Faraday wrote itself (the vault, own randomness, the
  entropy procedures, the Demo roller): both sides follow the same text.
  Only an outside review finds it.
- A blind spot of the corpus generators, which Faraday's authors wrote
  too: what they never ask is never compared.

### 10.7 Replacing the references of Faraday's own

Decided (owner, 2026-10-29, F160) and built in the order of what makes or
checks keys, addresses and signatures first. Each step replaced a second
implementation Faraday's authors wrote with code other people wrote and
moved the action's `independence` to `outside`, or to `outside-and-own`
where a named part has no outside code. No area is left with Faraday's own
second implementation alone: every action with an area is `outside` or
`outside-and-own`. What each action is compared with, and the order it was
built in, are `docs/ESTABLISHED.md` §9 (F192).

1. **Bitcoin Core as a program**, required (§10.4). Built: `sign`
   (`descriptorprocesspsbt`, `finalizepsbt` and `testmempoolaccept`), `addr`,
   `miniscript` and `vanity` (`deriveaddresses`), `psbt` (`decodepsbt` for
   version 0), and BIP-137 (`signmessagewithprivkey`, `verifymessage`).
   `bip32` is judged by btclib and btcd's `hdkeychain`, which cover its
   keys; Core's `deriveaddresses` would add only addresses, which `addr`
   has. The refusals of F136 and F141 are Faraday's policy, which Core
   does not share, and stay in the implementation Faraday's authors wrote,
   labelled so.
2. **btclib** (vendored by commit and SHA-256): BIP-32 derivation and
   strict decoding, BIP-137's other headers, and BIP-340. **python-ecdsa**
   (the distribution's package): every ECDSA signature again under
   RFC 6979, byte for byte. Both outside the libsecp256k1 lineage. Built.
3. **HWI's `psbt.py`** (vendored): version 2 PSBTs, which Core refuses.
   Built.
4. **PyCA `cryptography`** (the distribution's package): the vault's
   HKDF-SHA256 is built (area `hash`). The ChaCha20 block has no
   comparison of its own: the vault's cipher is judged by the files
   pykeepass, `keepassxc-cli` and keepass-rs write and read, and no
   Faraday-written reference stands for it.
5. **embit and SeedSigner's own code** (vendored): SeedQR and numbered
   parts; **Coldcard's code**: the dice procedure and Seed XOR. Built.
6. **LND's aezeed and AEZ in Go** (a Go toolchain, offline): built, with
   LND's node keys. ldk-node's node keys stay Faraday's composition of
   btclib's steps, since the only other implementation needs Node.
7. **BIP-322:** built against btclib-wallet's `bip322` and btcd's (pull
   request 2521, Go), with the BIP's generated vectors and btcd's negative
   ones; with Core and btclib's `bms` for BIP-137, they replaced
   `area_msg.py`'s own BIP-137 and BIP-322.
8. **Faraday's own specifications** (the vault's container, own
   randomness, cards, the camera, a mix, the Demo roller; the last five
   are held by frozen vectors and Rust tests, not cross-checked) have no
   outside implementation. Each keeps its label until an implementation
   written by an outside party from the specification alone, or a named
   outside party's reproduction of the published vectors
   (`tools/vectors/vault/` and the others), is recorded on the Audits
   page (`docs/AUDIT-SUBMISSIONS.md`). The call, with the specification's
   sections, the vectors and what an outside implementation offers to take
   the place of Faraday's in an area, is `docs/AUDIT-SUBMISSIONS.md` §10.
