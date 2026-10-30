# Established implementations first

**Kind:** Spec
**Status:** Specified (F178), not built
**Open:** all of it: the timing fixes (§4), the switches (§5), the reasons in the manifest and their check (§2), the generated lists (§3), the documentation fixes (§7)

Faraday uses an established implementation wherever one exists for what
it needs, and writes its own only where none exists or none fits its
rules for keys, saying why for each piece. This spec states the rule
(§1), how `just lint` holds it (§2), what Faraday writes itself once the
work here is built (§3), three timing fixes (§4), the switches to
outside code available now (§5), what is checked again later (§6), two
documentation fixes (§7) and the order of work (§8). Every choice here
is the owner's of 2026-10-30 (F178).

The facts are from a review of the tree at 0.1.9 against crates.io,
docs.rs and the crates' sources, on 2026-10-30. Where a fact could not
be read from a source here, it is marked "not checked", and the batch
that builds on it checks it first.

## 1. The rule

The owner's words (2026-10-30): Faraday uses an established
implementation wherever one exists for what it needs: the standard's own
reference implementation, or the implementation the ecosystem relies on,
maintained and published by a project Faraday's dependency review trusts
(F71). Faraday writes its own only where no established implementation
exists or none fits its rules for keys, and each piece it writes says
why.

### 1.1 The reasons, a closed list

A piece Faraday writes itself gives exactly one of these:

| `why` | Reason | What it must also say |
|---|---|---|
| `own-specification` | Faraday's own specification: the format or procedure is Faraday's, so nothing outside can implement it | The decision or document that specifies it |
| `none-established` | No established implementation exists | What was looked at, where something exists that is not established (§1.2) |
| `does-not-fit` | One exists but does not fit the key rules | The crate, its version, and the rule it fails (`docs/KEYS.md`, F70, `tools/lint-secrets.sh`, `no_std`) |
| `only-part-used` | Only the part used is written: a few lines of a standard over primitives that are established | The primitives it is written over |

Size, a single caller, the number of crates a library brings or the time
a switch takes are not reasons. A cost is written in the note, beside
the reason, where a decision may want it.

### 1.2 What "established" means, mechanically

A crate is established when one of these holds:

- a `[[trusted.<crate>]]` entry in `supply-chain/audits.toml` with
  criteria `safe-to-deploy` (F71 names the publishers trusted); or
- an imported audit in `supply-chain/imports.lock`, full or delta, with
  criteria `safe-to-deploy`, covering the version `Cargo.lock` pins, as
  cargo-vet counts coverage.

Trust is written per crate name: a new crate from a trusted project
still needs its own entry. An exemption, an audit of Faraday's own
under `ai-says-safe-to-deploy`, or a vendored copy is not established.

A crate that is not established can still be used: a decision names it,
and the manifest's entry for it names that decision (§2.3). A new piece
written here likewise needs a decision. Both counts only fall unless a
decision raises them.

### 1.3 What is true to say

Faraday designs no cryptographic primitive of its own. Its curve
arithmetic, signatures, hashes, ciphers and password stretching are
outside code (libsecp256k1, RustCrypto, dalek). Where no established
library offers a published algorithm in a form Faraday can use, Faraday
implements it from its specification and checks it against published
vectors and an implementation it did not write. A few constructions are
Faraday's own (the vault file, the session's randomness, the mixing of
entropy) and carry F160's label. The list of what Faraday implements
itself is generated from the manifest (§3), and the website's
/cryptography page reads it from there.

It is not true to say that Faraday uses none of its own cryptography,
or that it implements none.

### 1.4 The text STANDARDS §0 gains

Built with §2 (batch D1), as a bullet after "Checked against an
implementation Faraday did not write":

> **Established implementations first** (F178). Faraday uses an
> established implementation wherever one exists for what it needs: the
> standard's own reference implementation, or the implementation the
> ecosystem relies on, published by a project F71 trusts. It writes its
> own only where none exists or none fits its rules for keys. Each piece
> it writes says which of four reasons holds (Faraday's own
> specification; no established implementation; one exists but does not
> fit the key rules, and which rule; only the part used is written), and
> §10 lists them. A new piece written here, or a library that is not
> established, is a decision.

## 2. Enforcement

### 2.1 The manifest's `written_here`

`implementation.written_here` in `docs/crypto/actions.toml` is a string
today. It becomes an array of tables, one per piece, so that one action
can carry pieces with different reasons (the Secure Boot keys carry
three). The manifest's `schema` goes from 4 to 5.

```toml
[[action.implementation.written_here]]
what = "AEZ v5 (`faraday-bip::aez`)"
why = "none-established"
note = "No maintained Rust implementation of AEZ exists; Faraday's follows the Go code LND uses and is tested against LND's vectors."
decision = "F178"

[[action.implementation.written_here]]
what = "scrypt (`faraday-crypto::scrypt`)"
why = "does-not-fit"
unfit = "scrypt 0.12.0 frees its passphrase-derived working buffers without wiping them"
note = "The Rust scrypt library leaves its passphrase-derived working memory unwiped; Faraday's wipes it."
decision = "F178"
```

| Key | Required | Holds |
|---|---|---|
| `what` | yes | The piece and its module, as AUDIT §3 lists it |
| `why` | yes | One of the four values of §1.1 |
| `note` | yes | One sentence, plain fact, for a reader of the website: why this piece is written here |
| `unfit` | where `why` is `does-not-fit` | The crate and version, and the rule it fails |
| `tier` | on an action of a crate in AUDIT §4 | `carries`: a format Faraday writes that computes nothing on a key (UR, BBQr, SeedQR, the QR encoder) |
| `decision` | yes | The decision that accepts the piece: F178 for those of §3, a later one for a piece added after |

### 2.2 What `docs/crypto/check.py` checks

Added to `check.py check` (and to its `--self-test` fixture):

1. Every `written_here` entry has `what`, `why`, `note` and `decision`;
   `why` is one of the four; `unfit` is there exactly when `why` is
   `does-not-fit`; `decision` names a file in `docs/decisions/` whose
   Status is Active or Amended by.
2. `tier = "carries"` appears exactly on the actions of a crate that
   AUDIT §4 lists; `check_audit_crates` then accepts a written-here
   entry on a §4 crate when it carries that tier, and keeps refusing any
   other. AUDIT §1's "written here" column is still checked against the
   §1 crates' entries.
3. The number of `written_here` entries is `written_here N` in
   `tools/lint-crypto.counts`, which only falls; raising it is a
   decision, recorded in DECISIONS.
4. Every crate an action names in `implementation.libraries` is
   established (§1.2), read from `supply-chain/audits.toml` and
   `supply-chain/imports.lock`, or has a top-level entry:

   ```toml
   [[not_established]]
   crate = "lzma-rust2"
   reason = "The Starter's xz decoder; no xz decoder comes from a publisher F71 trusts."
   decision = "F178"
   ```

   whose `decision` is checked as in item 1. The number of entries is
   `not_established N` in `tools/lint-crypto.counts`, and only falls. An
   entry whose crate has become established fails as stale.

The libraries the manifest names today that are not established by
§1.2 (read on 2026-10-30): `getrandom` (an exemption, read by an AI and
recorded under `ai-says-safe-to-deploy`), `lzma-rust2` (an exemption)
and `rqrr` (vendored and patched, `third_party/rqrr`). Each needs its
`[[not_established]]` entry with a decision before batch D1 lands. F178
does not accept them; the owner decides each (§8, batch D1).

### 2.3 The generated places

`docs/crypto/gen.py` writes two places from the entries:

- `docs/AUDIT.md` §3, between its marker lines: the pieces in four
  lists, one per reason, in §1.1's order, each line the `what`, and for
  `does-not-fit` the `unfit` after a colon. Carried formats are marked
  "(carries)". The paragraph under the list, written by hand, names the
  four reasons and points to STANDARDS §10.
- `docs/STANDARDS.md` §10, a new section, "What Faraday writes itself",
  generated between marker lines: one table per reason, with columns
  Piece, Where (the crate), Note, Decision. STANDARDS' header sentence
  that names the generated sections (§1 to §7) names §10 too. §8 and §9
  keep their numbers.

`gen.py check` holds both. The rule `crypto.generated` already covers
`docs/STANDARDS.md` and `docs/AUDIT.md`.

The website's /cryptography page (the registry's consumer
`website.cryptography`) reads `docs/crypto/actions.toml`; its reader
changes with schema 5 and lists §3's pieces with their notes. That
change is made in the website's own repository when D1 lands.

### 2.4 The rule in `tools/impact.toml`

One rule, `crypto.established`, in the crypto area: written with this
spec as `planned`, made `active` by batch D1 when `crypto-check` holds
it.

- when: `docs/crypto/actions.toml`, `supply-chain/audits.toml`,
  `supply-chain/config.toml`, `supply-chain/imports.lock`,
  `core/*/Cargo.toml`, `faraday/**/Cargo.toml`, `tools/lint-crypto.counts`
- kind: held, by `crypto-check`
- misses: whether a stated reason is true; that a crate of the graph the
  manifest does not name is established (cargo vet holds that it is
  vetted, not that it is established)

`docs/CHECKS.md` is regenerated from it in the same commit
(`python3 -I -B tools/impact.py docs`).

## 3. What Faraday writes itself

The content of STANDARDS §10 and AUDIT §3 once §4, §5 and §7 are built,
before §6. Every entry's decision is F178. The action is the manifest
entry that carries it.

### 3.1 Faraday's own specification

| Piece | Where | Action | Note |
|---|---|---|---|
| The four-equal-slot vault container and its records | `faraday/faraday-vault` | `vault-create` | Faraday's own construction, built from standard primitives (Argon2id, HKDF-SHA256, XChaCha20-Poly1305) and labelled as not yet independently reviewed (`docs/VAULT.md`, F158, F160). |
| The session's random generator: its seed and pool, and the mixing of a person's own randomness | `faraday/faraday-core/src/fresh.rs` | `fresh` | Faraday's own construction over SHA-256, labelled as not yet independently reviewed (`docs/OWN-RANDOMNESS.md` §2.3, F86). |
| Dice, coin, card, hex and camera entropy accumulation and packing, and the mix of sources | `core/faraday-entropy` | `dice` | Faraday's own procedures over SHA-256; the card, camera and mix procedures are labelled as not yet independently reviewed (F158). |
| PSBT inspection, signing orchestration and verification logic | `core/faraday-psbt` | `psbt-read` | What Faraday checks before it signs is Faraday's own rule; the PSBT format itself is rust-bitcoin's. |
| The choice among a script's satisfactions by real signature size | `core/faraday-psbt/src/finalize.rs` | `finalize` | Faraday picks the smallest witness as Bitcoin Core's `finalizepsbt` does; miniscript builds and checks each witness. |
| BIP-39 entry and display: words typed by pinyin, zhuyin, kana or hangul, the last word's candidates, the explorer's view of the checksum | `core/faraday-bip/src/bip39.rs` | `words-read` | How a person types and sees the words is Faraday's own; the words, checksum and entropy are the bip39 crate's. |

### 3.2 No established implementation

| Piece | Where | Action | Note |
|---|---|---|---|
| AEZ v5 and aezeed, with aezeed's CRC-32C | `core/faraday-bip/src/aez.rs`, `core/faraday-bip/src/aezeed.rs` | `aezeed` | No maintained Rust implementation of AEZ exists; Faraday's follows the Go code LND uses and is tested against LND's vectors. Its AES round is the aes crate's and its BLAKE2b the blake2 crate's. |
| BIP-327 MuSig2, with BIP-328 and BIP-390 | `core/faraday-bip/src/musig.rs` | `musig-key` | The libsecp256k1 bindings Faraday's Bitcoin library uses do not include MuSig2 yet (§6.1). |
| BIP-352 silent payments | `core/faraday-bip/src/silent.rs` | `silent-receive` | libsecp256k1's silent payments module is not yet in its Rust bindings (§6.2). |
| BIP-374 DLEQ proofs and BIP-375's sending fields | `core/faraday-bip/src/dleq.rs`, `core/faraday-psbt/src/silent.rs` | `silent-send` | No library implements BIP-374 proofs or BIP-375's PSBT fields. |
| The PSBT version 2 reader | `core/faraday-psbt/src/v2.rs` | `psbt-v2` | rust-bitcoin reads only version 0 PSBTs; its version 2 crate is pre-release (§6.4). |
| BIP-322 "simple" message signatures | `core/faraday-psbt/src/message.rs` | `message-sign` | The bip322 crate is pre-release (§6.4). |
| SLIP-39 Shamir backup | `core/faraday-bip/src/slip39.rs` | `slip39` | No maintained Rust implementation of SLIP-39 exists. |
| The KDBX 4 file format | `core/faraday-backup/src/kdbx.rs` | `kdbx` | No Rust KeePass library comes from a publisher Faraday's dependency review trusts. |
| CMS SignedData, Authenticode's image hash and signature, EFI signature lists and `.auth` files | `faraday/faraday-sb` | `sb-keys` | No library writes Authenticode signatures or EFI signature lists; the CMS crate for the DER version Faraday uses is a release candidate (§6.3). |
| BC-UR (carries) | `core/faraday-codec/src/ur/` | `ur` | No library for this format comes from a publisher Faraday's dependency review trusts. |
| BBQr (carries) | `faraday/faraday-qr/src/bbqr.rs` | `bbqr` | No library for this format comes from a publisher Faraday's dependency review trusts. |
| SeedQR (carries) | `core/faraday-codec/src/seedqr.rs` | `seedqr-make` | No library implements SeedQR. |
| The QR encoder, a port of Nayuki's `qrcodegen` (carries) | `core/faraday-codec/src/qr.rs` | `qr` | No QR encoder comes from a publisher Faraday's dependency review trusts; Faraday's port of Nayuki's code wipes a SeedQR's buffers. |

### 3.3 One exists but does not fit the key rules

| Piece | Where | Action | Fails | Note |
|---|---|---|---|---|
| scrypt | `core/faraday-crypto/src/scrypt.rs` | `scrypt` | scrypt 0.12.0 frees its passphrase-derived working buffers (`b`, `v`, `t`) without wiping them | The Rust scrypt library leaves its passphrase-derived working memory unwiped; Faraday's wipes it (§6.5). |
| BIP 93 codex32 | `core/faraday-bip/src/codex32/` | `codex32` | codex32 0.1.0 holds the seed in a `String` that is `Clone` and `Debug` and never wiped, which `tools/lint-secrets.sh` refuses, and needs `std`, which `faraday-bip` does not have | The reference library keeps the seed in text that is never wiped and needs a full operating system. |
| The BIP-39 seed, PBKDF2-HMAC-SHA512 over the crate's sentence through `faraday-crypto` | `core/faraday-bip/src/bip39.rs` | `seed` | bip39's `to_seed` returns the seed as a plain `[u8; 64]`, and its own PBKDF2 leaves its HMAC state unwiped (read in bip39 2.2.2's `src/pbkdf2.rs`) | The seed is derived with the RustCrypto PBKDF2 Faraday uses everywhere, into a buffer that is wiped. |

### 3.4 Only the part used is written

| Piece | Where | Action | Note |
|---|---|---|---|
| OpenPGP v4 packets for one key shape, and the paperkey format | `faraday/faraday-pgp` | `gpg-create` | Faraday writes only the packets of its one key shape, over Ed25519 from dalek and SHA-1/SHA-2 from RustCrypto, each judged by GnuPG and paperkey. |
| TOTP code derivation | `faraday/faraday-vault/src/totp.rs` | `totp` | Faraday writes only the few lines RFC 6238 needs, over standard HMAC. |
| Seed XOR | `core/faraday-entropy/src/xor.rs` | `xor` | Faraday writes only the few lines the standard needs, over standard XOR. |
| A 32-byte hash reduced modulo the curve order, without a branch | `core/faraday-bip/src/musig.rs` | `musig-key` | libsecp256k1's Rust bindings take a scalar only below the order and offer no reduction; Faraday's is a single conditional subtraction (§4.1). |

Twenty-six pieces in all, against the eighteen lines of AUDIT §3 today:
the switches remove BLAKE2b, HKDF (two copies), the AES round, the
scalar arithmetic on secrets, the BIP-39 mnemonic core, the RSA RNG
adapter, X.509 and the DER writer, the single-key and multisig
finalizer and the sealing's counter nonce; the encodings and BIP-322,
written here and never listed, are listed.

## 4. The timing fixes

Three places where a secret's value steers the time an operation takes.
On an offline device the risk is low; each is fixed before the switches
because two of them contradict what the code says of itself.

### 4.1 F-1: scalar arithmetic on secrets

**Where.** `core/faraday-bip/src/musig.rs`: `add_mod_order` ends in
`if carry == 1 || at_least_order(&sum)`, and `at_least_order` is
`*a >= CURVE_ORDER`, a comparison that stops at the first byte that
differs; `mul_mod_order`, whose doc says it selects "without a branch,
so that a secret scalar's bits do not steer the control flow", calls
`add_mod_order` twice per bit; `negate_mod_order` returns early on zero;
`reduce` branches on `at_least_order`. Secrets reach them in:

- MuSig2's signing (`musig.rs`, the partial signature): `k1`, `k2`
  negated, `d = g·gacc·d0`, `b·k2`, `k1 + b·k2`, `e·a·d` and the sum;
  the nonces `k` reduced from their hashes and checked against zero and
  the order;
- BIP-374's proof (`dleq.rs`, `generate_proof`): `k` reduced from its
  hash, `e·a` and `k + e·a`;
- BIP-352's sender sum (`silent.rs`, `sender_outputs`): each input key
  negated where its point is odd, and the running sum;
- BIP-352's `checked` (`silent.rs`), which compares a derived tweak
  `t_k` with zero and the order.

**The fix.** Every operation on a secret scalar goes through
libsecp256k1, which is in the tree (secp256k1 0.29.1):

- a secret 32-byte value becomes a scalar only through
  `SecretKey::from_slice` (libsecp256k1's `seckey_verify`), and a
  `Scalar` only through `Scalar::from(SecretKey)`, so no secret meets
  `Scalar::from_be_bytes`, whose comparison the crate documents as not
  constant time;
- `a + b` is `SecretKey::add_tweak`, `a · b` is `SecretKey::mul_tweak`,
  `-a` is `SecretKey::negate`;
- a public factor (MuSig2's `g`, `gacc`, `b`, `e`, `a`; BIP-374's `e`)
  is a `Scalar` from its bytes and multiplies the secret as the tweak;
- a hash reduced to a scalar (`reduce`) is made branch-free: `a - n` is
  always computed, and the borrow selects `a` or `a - n` by mask. It is
  the one piece of scalar arithmetic left written here (§3.4).

A `SecretKey` cannot be zero. Where an intermediate value can be zero
it is handled where it arises: in the silent-payment sum the running
value is held as "no key yet" and an `add_tweak` that gives zero sets it
back to that; MuSig2's partial signature `s`, which BIP-327 allows to be
zero, is `nonce_part.add_tweak(key_part)`, with zero returned where the
library refuses. These branches are taken only on a value of
probability about 2⁻²⁵⁶ for keys not chosen to cancel, and the code says
so where it takes them.

The remaining arithmetic in `musig.rs` (key aggregation coefficients,
tweak accumulators, verification) works on public values only; its
functions say so in their first line and are renamed with a `_public`
suffix, so that a secret passed to one shows in review. A secret held
as `Scalar` (which neither wipes nor derives `Zeroize`) lives only on
the stack for the one call, as today's `[u8; 32]` intermediates do; the
`SecretKey`s are erased with `non_secure_erase` after use, as `dleq.rs`
already does with its nonce.

**What must pass, unchanged.** Vectors `tools/vectors/bip327`,
`tools/vectors/bip328`, `tools/vectors/bip390`, `tools/vectors/bip352`,
`tools/vectors/bip374`, `tools/vectors/bip375`, `tools/vectors/psbt`;
the cross-check areas `musig`, `silent` and `silent_send`
(`just crosscheck`, which re-stamps); the tests of `faraday-bip` and
`faraday-psbt`; `just crypto-review` for `sign-musig`, `musig-key`,
`silent-send` and `silent-receive`.

**Secret handling that changes** (the manifest's
`secrets.constant_time`):

- `sign-musig`: "Every operation on a secret scalar is libsecp256k1's
  (`SecretKey` add, multiply and negate), which libsecp256k1 documents
  as constant time; a hash is reduced to a scalar without a branch; the
  code documents no other timing." (The text today says the secret
  multiplication takes the same time for every scalar, which is not so
  of `mul_mod_order`.)
- `silent-send`: "Every secret multiplication goes through secp256k1's
  ECDH, and the response `k + e·a` through libsecp256k1's `SecretKey`
  add and multiply, which it documents as constant time." (The text
  today says the response uses a multiplication right for public values
  only; the response is a secret.)
- `silent-receive`: the sender sum and the tweak check are named the
  same way.

Whether libsecp256k1 documents its tweak functions as constant time is
read from its `include/secp256k1.h` when the batch is built; if it does
not say so, the text says "not stated by the library".

### 4.2 F-2: AEZ's AES round

**Where.** `core/faraday-bip/src/aez.rs`, `round`: SubBytes is a lookup
in `SBOX` indexed by state bytes, which are secret (the state is keyed
by the scrypt output of the person's passphrase).

**The fix.** `round` becomes `aes::hazmat::cipher_round` from the `aes`
crate (RustCrypto, 0.9.3), which is SubBytes, ShiftRows, MixColumns and
AddRoundKey, the round AEZ uses; `SBOX` and `xtime` go. This is the
switch of §5.3, built as part of the timing fixes. `double` and `mult`
are already branch-free or run over public constants, and stay.

**What must pass, unchanged.** `tools/vectors/aezeed` (LND's vectors
and the AEZ reference's, read by `core/faraday-bip`'s tests); the
cross-check area `lightning`; `just crypto-review aezeed`.

**Secret handling that changes.** `aezeed`: from "not stated by the
library" to what the aes crate documents for `hazmat` and its software
and hardware paths, read from its documentation when built (the review
recalls a fixsliced software path and AES-NI or ARMv8 where present;
not checked).

### 4.3 F-3: SLIP-39's GF(256) tables

**Where.** `core/faraday-bip/src/slip39.rs`, `at` (Lagrange
interpolation): for each share byte `v`, `if v != 0 { *byte ^=
EXP[(LOG[v] + basis) % 255] }`, a branch on a secret byte and two
lookups indexed by it. The other `LOG` lookups are indexed by share
indices, which are public.

**The fix.** Branch-free: the product `v · EXP[basis]` is a
multiplication in GF(256) by shift and masked reduction, eight rounds,
with no lookup indexed by `v` and no branch. `EXP[basis]` is indexed by
a public value and stays. About twenty lines.

**What must pass, unchanged.** `tools/vectors/slip39` (Trezor's
vectors); the cross-check area `slip39` (python-shamir-mnemonic); the
tests of `faraday-bip`; `just crypto-review slip39`.

**Secret handling that changes.** `slip39`: from "Not stated by the
code." to "The interpolation multiplies share bytes without a branch or
a lookup indexed by them; the code documents no other timing." The
`codex32` entry already says its GF(32) arithmetic is table lookups and
is not changed here.

## 5. The switches

Each subsection is one batch (§8). In every switch the `faraday-crypto`
or `faraday-bip` function a caller uses keeps its signature, so the
choke point (AUDIT §1), the key boundary (F70: no key leaves
`faraday-keys` but through its reveals) and the compute token (`c:
&Compute`, `docs/COMPUTE.md`) are unchanged; what Faraday refuses before
the call is unchanged. A new direct dependency gets its `docs/deps/`
page (rule `deps.page`) and, where it is not established, its trust
entry; every switch is proved by the same vectors and cross-check areas
giving the same bytes, then `just crosscheck` and `just crypto-review`
for each action whose fingerprint changes.

### 5.1 BLAKE2b

- **Files.** `core/faraday-crypto/src/blake2b.rs` (179 lines) becomes a
  wrapper of a few lines; `core/faraday-crypto/Cargo.toml`.
- **Crate.** `blake2` 0.11.0 (RustCrypto/hashes), `Blake2bVar` for the
  48-byte output, feature `zeroize`.
- **New in the graph.** None: blake2 0.11.0 is in `Cargo.lock` through
  argon2.
- **cargo vet.** `[[trusted.blake2]]` exists.
- **deps page.** New: docs/deps/blake2.md.
- **Kept in front.** `faraday_crypto::blake2b(data, out)` and its panic
  on a length outside 1 to 64.
- **Proof.** RFC 7693's vectors, today inline in `blake2b.rs`, move to
  `core/faraday-crypto/tests/primitives.rs`; `tools/vectors/aezeed`;
  area `lightning`.
- **Size.** About two hours.

### 5.2 HKDF, both copies

- **Files.** `core/faraday-crypto/src/hash.rs` (`hkdf_sha256_expand`,
  called by `core/faraday-psbt/src/sign.rs` and
  `faraday/faraday-keys/src/session.rs`); `faraday/faraday-vault/src/lib.rs`
  (`hkdf_sha256`, extract and one expand block, hand-assembled from
  HMAC).
- **Crate.** `hkdf` 0.13.0 (RustCrypto/KDFs), whose only dependency is
  `hmac` 0.13, in the tree. `faraday-crypto` turns on `hmac/zeroize`
  already.
- **New in the graph.** `hkdf`.
- **cargo vet.** New `[[trusted.hkdf]]`, publisher `github:RustCrypto/KDFs`
  (owner's yes, 2026-10-30). The imported audits stop at 0.12.3.
- **deps page.** New: docs/deps/hkdf.md.
- **Kept in front.** `faraday-crypto` gains `hkdf_sha256(salt, ikm,
  info, out)` beside `hkdf_sha256_expand`, both over the crate; the
  vault calls it with an empty salt, so `faraday-vault` names `hmac` and
  `sha2` only where it still uses them. The 255-block refusal stays.
  Whether `Hkdf`'s state is wiped on drop through `hmac/zeroize` is read
  from hkdf 0.13.0's source when built (not checked); if not, the
  wrapper holds the pseudorandom key in a `Secret` and uses
  `Hkdf::from_prk`.
- **Proof.** RFC 5869's cases in `core/faraday-crypto/tests/primitives.rs`
  and `faraday/faraday-vault/src/tests.rs`; `tools/vectors/vault`,
  `tools/vectors/vault-primitives`; areas `vault` and `sign` (the
  signing round's aux randomness); `tools/vault/open.py` opens the same
  vectors. The vault's bytes do not change.
- **Size.** About half a day.

### 5.3 AEZ's AES round

- **Files.** `core/faraday-bip/src/aez.rs`; `core/faraday-bip/Cargo.toml`.
- **Crate.** `aes` 0.9.3 (RustCrypto/block-ciphers), its `hazmat`
  module's `cipher_round` (the feature's name is read when built).
- **New in the graph.** `aes`; its dependencies `cipher` 0.5, `cpubits`
  and `cpufeatures` 0.3 are in the tree.
- **cargo vet.** New `[[trusted.aes]]`, publisher
  `github:RustCrypto/block-ciphers` (owner's yes; the publisher's name
  is read from crates.io when the entry is written). The imported
  audits stop at 0.8.4.
- **deps page.** New: docs/deps/aes.md.
- **Kept in front.** AEZ's mode, `Extract`, `AES4`, `AES10` and the
  doubling stay Faraday's (§3.2).
- **Proof.** §4.2.
- **Size.** About half a day.

### 5.4 Scalar arithmetic

Built as §4.1. No new crate: secp256k1 0.29.1 is in the tree and
trusted. About a day.

### 5.5 The BIP-39 core

- **Files.** `core/faraday-bip/src/bip39.rs` (1,014 lines, most of them
  entry and display); `core/faraday-bip/Cargo.toml` (bip39 moves from
  dev-dependencies to dependencies); `Cargo.toml`'s workspace entry.
- **Crate.** `bip39` (rust-bitcoin/rust-bip39), features `all-languages`
  and `zeroize`, no `std`. The tree has 2.2.2, on `bitcoin_hashes` 0.14
  as bitcoin 0.32 is; 3.0.0 (2026-09-17) is out. The batch takes the
  newest version that adds no second `bitcoin_hashes` (3.0.0's
  dependencies not checked).
- **New in the graph.** None.
- **cargo vet.** The exemption for bip39 2.2.2 (`safe-to-run`) becomes
  `[[trusted.bip39]]` for its publisher (owner's yes; F71 amended). The
  publisher's crates.io user id is read when the entry is written: the
  review names tnull as the publisher of 3.0.0 and did not check 2.2.2's.
- **deps page.** New: docs/deps/bip39.md.
- **What moves to the crate.** Entropy to words, parsing a sentence,
  the checksum, entropy out (`from_entropy_in`, `parse_in_normalized`,
  `checksum`, `word_indices`, `to_entropy_array`), and the word lists.
- **Kept in front.** Faraday's `Mnemonic` stays the type callers use, a
  wrapper that keeps the indices and word count it works out once
  (CLAUDE.md, a type re-implemented from a `core/` one); NFKD and the
  first unknown word by position; typing by pinyin, zhuyin, kana and
  hangul; the last word's candidates; the refusals of `words-read`,
  `words-make` and `seed` (entropy of 16 to 32 bytes in steps of 4, the
  passphrase checked to printable ASCII, a duplicate key). The seed is
  PBKDF2-HMAC-SHA512 through `faraday_crypto::pbkdf2_hmac_sha512` (it
  takes `c: &Compute`) over the crate's sentence and Faraday's salt, in
  `Secret` buffers (§3.3); the crate's `to_seed` is not called. Seed
  words still become a `Mnemonic` only in `faraday-keys`' `words` module
  (`lint-secrets` rule 4).
- **Proof.** `tools/vectors/bip39`; area `bip39` (python-mnemonic, by
  Trezor); the self-test's `bip39_vector`; the fuzz target `bip39_words`;
  the tests of `faraday-bip` and `faraday-keys`.
  `core/faraday-bip/tests/all/crosscheck.rs`, which compares Faraday's
  BIP-39 with the bip39 crate, would compare the crate with itself and
  is removed; the manifest's `checked` for the three actions drops it.
- **Size.** One to two days.

### 5.6 The RSA RNG adapter

- **Files.** `faraday/faraday-sb/src/rng.rs` (53 lines), its callers in
  `faraday/faraday-sb/src/lib.rs`.
- **Crate.** `chacha20` 0.10.2's `ChaCha20Rng` (feature `rng`, over
  `rand_core` 0.10), with `zeroize`, which the crate wipes on drop.
- **New in the graph.** None; chacha20 and rand_core are in the tree,
  covered by imported audits.
- **cargo vet.** No change.
- **deps page.** `docs/deps/chacha20.md` adds the Secure Boot keys'
  generator to its purpose.
- **Kept in front.** One fresh 32-byte draw as the seed, one stream per
  purpose (`set_stream`: the GUID and serials, the key, the blinding),
  the draw taken as today from the system with the person's own
  randomness hashed in.
- **Output.** `ChaCha20Rng` lays out its counter and stream as djb's
  ChaCha20 (64-bit counter, 64-bit stream), not RFC 8439's 32-bit
  counter and 96-bit nonce, and buffers its blocks, so one seed gives
  different keys than today. Keys already made are unaffected: they are
  stored in a vault. `faraday/faraday-sb/tests/golden.rs` is re-pinned;
  the test in `faraday/faraday-sb/tests/judged.rs` that checks the
  stream against RFC 8439 Appendix A.1 is replaced by one that the
  stream for a seed is the crate's, and every key made is judged by
  `openssl pkey -check` and every image by `sbverify` as today.
- **Size.** About two hours, and the re-pinning.

### 5.7 X.509 and DER

- **Files.** `faraday/faraday-sb/src/der.rs` (164 lines) and
  `faraday/faraday-sb/src/x509.rs` (336 lines); the CMS, Authenticode
  and EFI code that builds DER with `der.rs`
  (`faraday/faraday-sb/src/pe.rs`, `faraday/faraday-sb/src/efi.rs`).
- **Crates.** `x509-cert` 0.3.0 with its `builder` feature, for the
  self-signed certificates and their reading; `der` 0.8, made a direct
  dependency, for the CMS SignedData and Authenticode structures, written
  as `der` types (§3.2: only those structures stay Faraday's).
- **New in the graph.** `x509-cert`; `der` 0.8, `spki` 0.8 and
  `const-oid` 0.10 are in the tree through rsa. What `builder` adds
  beyond them is read when built (not checked).
- **cargo vet.** New `[[trusted.x509-cert]]`, publisher
  `github:RustCrypto/formats`, as `der` and `spki` are (owner's yes).
- **deps pages.** New: docs/deps/x509-cert.md and docs/deps/der.md.
- **Kept in front.** The certificates' fields as today (subject, serial
  from the draw, validity, the key usage and extensions written today);
  the reader's refusals of a certificate that is not the shape Faraday
  wrote.
- **Proof.** `faraday/faraday-sb/tests/judged.rs`: every certificate and
  `.auth`'s PKCS #7 judged by `openssl`, signed images by `sbverify`;
  `faraday/faraday-sb/tests/golden.rs` gives the same bytes for the same
  draw (DER is canonical; where x509-cert orders an extension
  differently, the golden file changes and `openssl` still judges it).
  The Starter's check of a signed boot image (`starter-signed`) reads
  the same.
- **Size.** One to two days.

### 5.8 The single-key and multisig finalizer

- **Files.** `core/faraday-psbt/src/finalize.rs` (`build_final` and
  `push`).
- **Crate.** `miniscript` 12.3.7, in the tree and trusted: its
  `finalize_inp_mut`, which Faraday already calls for miniscript and
  taproot inputs, also infers pkh, wpkh, sh-wpkh, and multi under sh,
  wsh and sh-wsh from the PSBT's own scripts (read in miniscript
  12.3.7's `src/psbt/finalizer.rs`, `get_descriptor`).
- **New in the graph.** None.
- **cargo vet, deps page.** No change; `docs/deps/miniscript.md` adds
  the finalizer for every script kind.
- **Kept in front.** The interpreter check on every witness; the choice
  of the smallest satisfaction by real signature size, a `multi()`
  keeping its k smallest signatures (§3.1), extended to legacy `sh`
  multisig; the signatures under a taproot `pkh` leaf taken off as today;
  the signing fields cleared as BIP-174 requires.
- **Proof.** `tools/vectors/psbt` (BIP-174's finalizer and extractor
  vectors); area `sign` (Bitcoin Core's `finalizepsbt` and
  `testmempoolaccept`); the wallet fixtures
  (`core/faraday-psbt/tests/all/wallet_fixtures.rs`). The transactions
  are byte for byte the same.
- **Size.** One to two days.

### 5.9 The sealing's counter nonce, removed

- **Files.** `core/faraday-crypto/src/sealed.rs` (653 lines):
  `Sealed`, `SessionKey`, `SealError`, `SealedBytes` and the counter
  nonce go; `MnemonicBytes`, `SeedBytes`, `MAX_SEED_LEN` and
  `MAX_MNEMONIC_WORDS`, which `faraday-bip`, `faraday-vault` and
  `faraday-keys` use, move to `core/faraday-crypto/src/secret.rs`.
  `core/faraday-crypto/src/lib.rs`'s exports and module doc; the
  comment in `core/faraday-bip/src/keys.rs` that waits for a
  `Sealed<T>`; `tools/lint-secrets.sh`'s type names and `SECRET_FILES`.
- **New in the graph.** None; `faraday-crypto` drops its
  `chacha20poly1305` dependency (the vault keeps its own).
- **Reason.** No shipped code builds a session key or seals a value
  (the manifest's `hold`; STANDARDS §7 item 3). Should sealing in memory
  come back, it is XChaCha20-Poly1305 with a random nonce from the same
  crate, and a decision.
- **Proof.** The tests of `faraday-crypto`, `faraday-keys` and
  `faraday-core`. The `hold` action loses its written-here piece and
  STANDARDS §7 its item 3 (generated); AUDIT §1's `faraday-crypto` row
  drops `Sealed`/`SessionKey`.
- **Size.** About two hours.

## 6. Check again later

Each item here is kept written here (§3) until its condition holds; the
switch is then a batch of its own, with a decision only where §1 needs
one.

### 6.1 MuSig2, after bitcoin 0.33

libsecp256k1 has had BIP-327 since 0.6.0 (2024-11-04), and
rust-secp256k1 0.33.1 wraps it as `secp256k1::musig`. The tree's
bitcoin 0.32 uses secp256k1 0.29, which does not; bitcoin 0.33.0-beta
uses secp256k1 0.32.0-beta.2 (whether that has `musig` is not checked).
Faraday does not carry two copies of libsecp256k1 (owner, 2026-10-30).

**What would change it.** A stable bitcoin 0.33, and a miniscript that
builds on it, whose secp256k1 has `musig`. Then key aggregation,
tweaks, nonce generation, partial signing, partial verification and
aggregation move to libsecp256k1. DeterministicSign (BIP-327, used in
`core/faraday-psbt/src/sign.rs`) stays written here, as libsecp256k1
has none (`none-established`); BIP-328 and BIP-390 stay unless that
miniscript has `musig()` (not checked). About three to four days after
the bitcoin move.

### 6.2 Silent payments in rust-secp256k1

libsecp256k1 0.8.0 (2026-08-03) has a `silentpayments` module (send and
receive, full-node scanning). rust-secp256k1 0.33.1 does not wrap it.

**What would change it.** A rust-secp256k1 release, on the version the
tree uses, that wraps it. Then the scanning and the sender arithmetic
move; BIP-374, BIP-375, BIP-392's text and BIP-321 and BIP-353 stay. About
two days.

### 6.3 CMS out of its release candidate

`cms` 0.3.0-rc.0 (2026-10-13) is the one on der 0.8; the stable 0.2.3
uses der 0.7, a second major.

**What would change it.** cms 0.3.0. Then CMS SignedData moves to it,
with `[[trusted.cms]]` (`github:RustCrypto/formats`, the owner's yes of
2026-10-30). Authenticode's image hash and signature and the EFI lists
stay (`authenticode` 0.6.0 verifies only, on der 0.7).

### 6.4 PSBT version 2 and BIP-322 at 1.0

`psbt-v2` 0.3.0 (rust-bitcoin/rust-psbt) is pre-1.0 and has no
BIP-375 fields; bitcoin 0.33.0-beta has no version 2. `bip322` 0.0.12
(rust-bitcoin organisation) is pre-1.0 and on sha2 0.10.

**What would change it.** A 1.0 of either, from a publisher F71 trusts
or given a trust entry, on the RustCrypto majors the tree uses. The
version 2 reader keeps Faraday's refusals in front; BIP-375's fields
stay until the crate carries them.

### 6.5 scrypt's upstream fix

scrypt 0.12.0 frees `b` (PBKDF2 of the passphrase and salt, one round)
and the working buffers `v` and `t` without wiping them; what is left in
freed memory lets someone test a passphrase without the memory-hard
step. A change that wipes them (`Zeroizing` buffers, behind the crate's
`zeroize` feature) is proposed to RustCrypto/password-hashes (owner,
2026-10-30).

**What would change it.** A scrypt release with that change. Then
`faraday_crypto::scrypt` wraps it, with a trust entry and a deps page,
and RFC 7914's vectors and the `lightning` area prove the bytes.

### 6.6 codex32 with `no_std` and wiping

codex32 0.1.0 (2023, by a BIP-93 author) is the one release.

**What would change it.** A release that is `no_std` and holds the seed
in a type that is neither `Clone` nor `Debug` and is wiped. Faraday's
worksheets, fresh sets and error correction would stay in front.

## 7. Documentation fixes

Both are made in batch B1, ahead of the switches.

### 7.1 AUDIT §1: the session generator's SHA-256

`docs/AUDIT.md` §1 says the session's generator is written "over
`faraday-crypto`'s SHA-256". `faraday/faraday-core/src/fresh.rs` uses
`faraday_bip::bitcoin::hashes::sha256` (bitcoin_hashes, trusted under
rust-bitcoin), as the manifest's `fresh` already says. The row says so:
"over bitcoin_hashes' SHA-256", and `faraday-crypto`'s row, which calls
the crate "the hash/KDF choke point every other crate calls through",
names this one exception. The code is not changed: both SHA-256s are
established, and moving the generator would change an own
construction's fingerprint for nothing.

### 7.2 AUDIT §1: the reasons given for BIP-39 and MuSig2

`docs/AUDIT.md` §1's `faraday-bip` row gives "no vetted `no_std` crate
exists for them on this tree's pin, or (BIP-39) the explorer screens
need intermediate values a library hides". For BIP-39 it is not so: the
bip39 crate gives `word_indices`, `checksum` and `to_entropy_array`
(2.2.2, `src/lib.rs`). For MuSig2 it holds only for the tree's
secp256k1 0.29. The row is rewritten to the reasons of §3 (BIP-39 until
§5.5 lands: "the bip39 crate is used to check it; the switch is
planned"; MuSig2: "libsecp256k1's MuSig2 is not in the rust-secp256k1
bitcoin 0.32 uses"), and §5.5 rewrites it again.

## 8. Order of work

Each batch is one commit and one check: the module touched while
working, `just crosscheck` where a cross-check area's code changed,
`just crypto-review` for each action whose fingerprint changed, then
`just` at the end. The timing fixes come first.

| Batch | What | Sections | Size |
|---|---|---|---|
| A1 | Scalar arithmetic on secrets through libsecp256k1; `reduce` branch-free; the public functions renamed; the Secret handling of `sign-musig`, `silent-send`, `silent-receive` | §4.1, §5.4 | About 1 day |
| A2 | AEZ's AES round from `aes`; trust entry and deps page; the Secret handling of `aezeed` | §4.2, §5.3 | About half a day |
| A3 | SLIP-39's interpolation branch-free; the Secret handling of `slip39` | §4.3 | About 2 hours |
| B1 | AUDIT §1's two documentation fixes | §7 | Under an hour |
| C1 | BLAKE2b from `blake2`; deps page | §5.1 | About 2 hours |
| C2 | HKDF from `hkdf`, both copies; trust entry and deps page | §5.2 | About half a day |
| C3 | The sealing's counter nonce removed | §5.9 | About 2 hours |
| C4 | The RSA RNG adapter from `ChaCha20Rng`; tests re-pinned | §5.6 | About half a day |
| C5 | X.509 from `x509-cert`, DER from `der`; trust entry and deps pages | §5.7 | 1 to 2 days |
| C6 | The single-key and multisig finalizer from miniscript | §5.8 | 1 to 2 days |
| C7 | The BIP-39 core from `bip39`; trust entry and deps page; the crate's self-comparison removed | §5.5 | 1 to 2 days |
| D1 | Schema 5: `written_here` as entries with `why`, `note`, `decision`; `[[not_established]]`; `check.py` and `gen.py`; AUDIT §3 grouped; STANDARDS §10 generated and §0's bullet; the counts; `crypto.established` made active; the manifest's entries filled as §3 | §1.4, §2, §3 | About 1 day |

C1 to C7 are independent of each other and may land in any order; each
changes the manifest's `written_here` string of its action as it lands.
D1 comes after them so that the entries it writes are §3's. Before D1
lands, the owner decides `getrandom`, `lzma-rust2` and `rqrr` (§2.2), and
the website's reader of the manifest is changed in its own repository.
§6's items come as their conditions hold. In all, about six to nine
working days.
