# Established implementations first

**Kind:** Spec
**Status:** Specified (F178, F180, F181, F190, F191, F192, F193); built: AEZ's AES round from the aes crate (§4.2, §5.3), BLAKE2b from the blake2 crate (§5.1), the sealing's counter nonce removed (§5.9), the Secure Boot keys' generator from the chacha20 crate's ChaCha20Rng (§5.6), X.509 from x509-cert and DER from der, judged by certtool, pesigcheck and real signed binaries (§5.7; batches C5 and X2), SLIP-39's GF(256) and checksum without a branch (§4.3), the scalar arithmetic on secrets in Faraday's own branch-free form and then through libsecp256k1 (§4.1; batch A1), the single-key and multisig finalizer from miniscript with btclib's finalizer beside Core in area `sign` (§5.8; batches C6 and X3), the second curve lineage beside Core for the vanity search's child keys and for sweep-sign and wif-read (§9.3; batch X13), the descriptor checksum from miniscript (§5.12), the PNG writer's Adler-32 from `adler2` (§5.14; batch C12), the cross-check's outside judge btcd v0.27.0 for `hdkeychain` and control blocks (§9.4, batch X14), btclib for PSBTs, transactions, signature hashes and control blocks (§9.4, batch X15), the JSON reader from `serde_json` behind Faraday's visitor (§5.13; batch C11), the inventory's name matching (§2.5), the FAT32 formatter (§3.2), base64 from base64ct (§5.10, batch C8), the documentation fixes of §7 (batch B1), the cross-check's descriptors and miniscript judges for address, descriptor and recovery (§9.4, batch X18) and the quiz's seed from the session's generator (§5.16), btclib's change rule judging psbt-review (§9.4, batch X16) and Core's watch-only descriptor wallet beside it (batch X17), aezeed's software-round pass (§9.4, batch X29), area `compile` for the policy compiler (§9.4, batch X20), area `sign-check` for the signatures a PSBT carries (§9.4, batch X19), area `import` reading the real wallet files the wallets' own test suites publish (§9.4, batch X30), the Starter's decoder, tables, property lists and GUIDs judged by `xz`, 7-Zip, `sgdisk`, `plistlib` and `uuid` (§9.4, batch X28), and zbar and zxing-cpp decoding every QR picture Faraday writes with segno as a second encoder, in area `seedqr` (§9.4, batch X21)
**Open:** the timing fixes of §4.4 to §4.7; the switches (§5) but for the finalizer (§5.8); the reasons in the manifest and their check (§2); the generated lists (§3); the wording of §7.5, which follows at D1; the cross-check's outside sources (§9, batches X1 to X30, X3, X13 to X21, X28, X29 and X30 excepted)

Faraday uses an established implementation wherever one exists for what
it needs, and writes its own only where none exists or none fits its
rules for keys, saying why for each piece. This spec states the rule
(§1), how `just lint` holds it (§2), what Faraday writes itself once the
work here is built (§3), the places where a secret's value steers time
or leaves copies, the fixes and what the security model already bounds (§4), the switches to outside code
available now (§5), what is checked again later (§6), the documentation
fixes (§7), the order of work (§8) and what the cross-check compares
each action with once the switches are made (§9). The choices are the
owner's of 2026-10-30 (F178) and 2026-10-31 (F180, F181, F191, F192).

The facts are from a review of the tree at 0.1.9 against crates.io,
docs.rs and the crates' sources on 2026-10-30, and a review of the tree
on 2026-10-31 (main after the timing fixes of §4.2, §4.3 and the
branch-free form of §4.1) that read the code Faraday wrote outside the
manifest's `written_here`, every path a secret takes, every place a
secret is held in text or a growable buffer, the facts the first review
left unchecked, and the cross-check. Where a fact could not
be read from a source, it is marked "not checked", and the batch that
builds on it checks it first and stops if it does not hold.

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
| `does-not-fit` | One exists but does not fit the key rules | The crate, its version, and the rule it fails (`docs/KEYS.md`, F70, `tools/lint-secrets.sh`, `no_std`, constant time on secrets) |
| `only-part-used` | Only the part used is written: a few lines of a standard over primitives that are established | The primitives it is written over |

Size, a single caller, the number of crates a library brings or the time
a switch takes are not reasons. A cost is written in the note, beside
the reason, where a decision may want it.

### 1.2 What "established" means, mechanically

A crate is established, at the version checked, when one of these holds:

- a `[[trusted.<crate>]]` entry in `supply-chain/audits.toml` with
  criteria `safe-to-deploy` (F71 names the publishers trusted), whose
  `user-id` or `trusted-publisher` is the publisher that
  `supply-chain/imports.lock` records for that version
  (`[[publisher.<crate>]]`), within the entry's `start` and `end`;
- an imported audit in `supply-chain/imports.lock`, full or delta, with
  criteria `safe-to-deploy`, covering that version as cargo-vet counts
  coverage (a chain of deltas counts only from a full audit; a chain
  rooted at an exemption or at an audit of Faraday's own does not); or
- an imported `wildcard-audits` entry in `supply-chain/imports.lock` with
  criteria `safe-to-deploy` whose `user-id` or `trusted-publisher` is the
  publisher recorded for that version, within its start and end
  (`unicode-normalization` is established this way, through Mozilla's
  wildcard entry for its publisher).

The version checked is the one the crate naming the library depends on,
read from `Cargo.lock`'s dependency edges (`getrandom` is pinned twice:
0.4.3, which the shells name, and 0.3.4, which `winit`'s path takes and
no action names). `check.py` reads the two files and needs no network.

Trust is written per crate name: a new crate from a trusted project
still needs its own entry. An exemption, an audit of Faraday's own
under `ai-says-safe-to-deploy`, or a vendored copy is not established.

A crate that is not established can still be used: a decision names it,
and the manifest's entry for it names that decision (§2.2). A new piece
written here likewise needs a decision. Both counts only fall unless a
decision raises them.

### 1.3 What is true to say

Faraday designs no cryptographic primitive of its own. Its curve
arithmetic, signatures, hashes, ciphers and password stretching are
outside code (libsecp256k1, RustCrypto, dalek). Where no established
library offers a published algorithm in a form Faraday can use, Faraday
implements it from its specification and checks it against published
vectors and an implementation it did not write. A few constructions are
Faraday's own (the session's randomness, the mixing of entropy) and
carry F160's label. The vault is two published formats once F190 is
built, KDBX 4 for a file and an RFC 9580 message for a QR code, each
read by programs Faraday's authors did not write; the four-slot
container of today, Faraday's own, goes then. The list of what Faraday implements
itself is generated from the manifest (§3), and the website's
/cryptography page reads it from there.

It is not true to say that Faraday uses none of its own cryptography,
or that it implements none.

### 1.4 The text STANDARDS §0 gains

Built (2026-10-30), with the two-sentence summary at §0's top between
`standards-summary` markers, which the website's /cryptography reads; batch D1
adds "and §10 lists them" once §10 is generated:

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
can carry pieces with different reasons (`words-read` carries two, the
Secure Boot keys three). The manifest's `schema` goes from 4 to 5.

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
| `tier` | on an action of a crate in AUDIT §4 or §5 | `carries`: a format Faraday reads or writes that computes nothing on a key (UR, BBQr, SeedQR, the QR encoder, the PNG writer, FAT, GPT and MBR, the PDF writer, the plist reader, the UUID form) |
| `decision` | yes | The decision that accepts the piece: F178 for those of §3 that were in the tree on 2026-10-30, F180 for those §3 adds, a later one for a piece added after |

### 2.2 What `docs/crypto/check.py` checks

Added to `check.py check` (and to its `--self-test` fixture):

1. Every `written_here` entry has `what`, `why`, `note` and `decision`;
   `why` is one of the four; `unfit` is there exactly when `why` is
   `does-not-fit`; `decision` names a file in `docs/decisions/` whose
   Status is Active or Amended by.
2. `tier = "carries"` appears exactly on the entries of actions whose
   crate AUDIT §4 or §5 lists; `check_audit_crates` then accepts a
   written-here entry on such a crate when it carries that tier, and
   keeps refusing any other. AUDIT §1's "written here" column is still
   checked against the §1 crates' entries.
3. The number of `written_here` entries is `written_here N` in
   `tools/lint-crypto.counts`, which only falls; raising it is a
   decision, recorded in DECISIONS. D1 writes it as the count of §3 at
   D1's landing.
4. Every crate an action names in `implementation.libraries` or
   `implementation.through` is established (§1.2) at the version checked,
   read from `supply-chain/audits.toml` and `supply-chain/imports.lock`,
   or has a top-level entry:

   ```toml
   [[not_established]]
   crate = "lzma-rust2"
   reason = "The Starter's xz decoder. No established pure-Rust streaming xz decoder exists, and STARTER §1.3 rules out the C library liblzma; what it writes is checked against the signed manifest's SHA-256 and size."
   decision = "F180"
   ```

   whose `decision` is checked as in item 1. The number of entries is
   `not_established N` in `tools/lint-crypto.counts`, and only falls. An
   entry whose crate has become established fails as stale.

The libraries the manifest names that are not established by §1.2 (read
on 2026-10-31; established are aes, argon2, bitcoin, bitcoin_hashes,
chacha20, chacha20poly1305, ed25519-dalek, hmac, miniscript, pbkdf2,
rand_core, rsa, secp256k1, sha1, sha2, subtle and unicode-normalization)
are three. The owner's decisions of 2026-10-31 (F180):

- **getrandom** gets a trust entry: F71 gains the Rand project's
  publisher. In `supply-chain/audits.toml`:

  ```toml
  [[trusted.getrandom]]
  criteria = "safe-to-deploy"
  trusted-publisher = "github:rust-random/getrandom"
  start = "2025-12-27"
  end = "2027-10-01"
  notes = "F71: trusted publisher (docs/decisions/F71.md), added by F180"
  ```

  `start` is the date of 0.4.0-rc.0, the first version published by
  trusted publishing from `rust-random/getrandom`; it covers 0.4.3, which
  the shells name. 0.3.4, published by a person and reached only through
  `winit`'s `ahash`, stays an exemption; the manifest names no action
  that reaches it. `just vet-refresh` (online) records the publisher in
  `supply-chain/imports.lock` in the same commit, and `config.toml`'s
  exemption for 0.4.3 goes. `docs/deps/getrandom.md`'s Review paragraph
  says it is trusted under F71 by F180. F71 gains the line under its
  amendments. Made in batch D1.
- **lzma-rust2** gets the `[[not_established]]` entry shown above: no
  established pure-Rust streaming xz decoder exists (liblzma, xz2 and
  lzma-sys are the C library STARTER §1.3 rules out; lzma-rs, xz4rust
  and the others the deps page names are not established), and what it
  decodes is checked against the signed manifest's SHA-256 and size
  before the write is called good, so a fault in it cannot pass a wrong
  image as right.
- **rqrr** gets the entry:

  ```toml
  [[not_established]]
  crate = "rqrr"
  reason = "The camera's QR decoder, vendored and patched against panics on hostile frames (third_party/rqrr/PATCHES.md): the only maintained pure-Rust QR decoder. What it reads is untrusted input that Faraday's parsers check. The panic fixes are offered upstream, where the same panic is reported as rqrr's issue 54."
  decision = "F180"
  ```

  Offering the patches upstream is a public action, made by the owner.
  Even merged, rqrr stays not established (one publisher, not one F71
  names); a merge lets Faraday take the crates.io release in place of
  the copy.

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
it. D1 also corrects its `misses` text, which names schema 4: the check
comes with schema 5.

- when: `docs/crypto/actions.toml`, `supply-chain/audits.toml`,
  `supply-chain/config.toml`, `supply-chain/imports.lock`,
  `core/*/Cargo.toml`, `faraday/**/Cargo.toml`, `tools/lint-crypto.counts`
- kind: held, by `crypto-check`
- misses: whether a stated reason is true; that a crate of the graph the
  manifest does not name is established (cargo vet holds that it is
  vetted, not that it is established)

`docs/CHECKS.md` is regenerated from it in the same commit
(`python3 -I -B tools/impact.py docs`).

### 2.5 The inventory's name matching (batch A0)

The crypto inventory (`tools/costs-lint/src/inventory.rs`) decides which
functions an action's fingerprint covers. Built: a path starting with
`crate`, `super` or `self` is made absolute against the caller's module
and names the functions of that module only, or what a `pub use` there
re-exports, or nothing (`Self::f` is not moved by a `use` of the type in
another module of the file); a method whose name is in `STD_METHODS` is
an edge from a caller whose signature or body names its type, as well as
from the type's own impls. The fixture crate of `selftest.rs` holds a
check for each. `slip39` gained `Points::at`, `gf_mul` and `gf_inv`;
`codex32-make` and `codex32-correct` no longer carry SLIP-39's `gf_mul`
and `polymod`.

## 3. What Faraday writes itself

The content of STANDARDS §10 and AUDIT §3 once §4, §5 and §7 are built,
before §6. The decision is F178 for the pieces in the tree on
2026-10-30 and F180 for the rest (the column says which). The action is
the manifest entry that carries it; where a piece's action is not given
below, D1 takes the action whose functions reach it, from the inventory.

### 3.1 Faraday's own specification

| Piece | Where | Action | Decision | Note |
|---|---|---|---|---|
| The session's random generator: its seed and pool, and the mixing of a person's own randomness | `faraday/faraday-core/src/fresh.rs` | `fresh` | F178 | Faraday's own construction over SHA-256, labelled as not yet independently reviewed (`docs/OWN-RANDOMNESS.md` §2.3, F86). |
| Dice, coin, card, hex and camera entropy accumulation and packing, and the mix of sources | `core/faraday-entropy` | `dice` | F178 | Faraday's own procedures over SHA-256; the card, camera and mix procedures are labelled as not yet independently reviewed (F158). |
| PSBT inspection, signing orchestration and verification logic | `core/faraday-psbt` | `psbt-read` | F178 | What Faraday checks before it signs is Faraday's own rule; the PSBT format itself is rust-bitcoin's. |
| The choice among a script's satisfactions by real signature size | `core/faraday-psbt/src/finalize.rs` | `finalize` | F178 | For a miniscript Faraday picks the smallest witness as Bitcoin Core's `finalizepsbt` does; a plain multisig carries the first m signers' signatures, as Core writes them; miniscript builds and checks each witness. |
| BIP-39 entry and display: words typed by pinyin, zhuyin, kana or hangul, the last word's candidates, the explorer's view of the checksum | `core/faraday-bip/src/bip39.rs` | `words-read` | F178 | How a person types and sees the words is Faraday's own. |

### 3.2 No established implementation

| Piece | Where | Action | Decision | Note |
|---|---|---|---|---|
| AEZ v5 and aezeed, with aezeed's CRC-32C | `core/faraday-bip/src/aez.rs`, `core/faraday-bip/src/aezeed.rs` | `aezeed` | F178 | No maintained Rust implementation of AEZ exists; Faraday's follows the Go code LND uses and is tested against LND's vectors. Its AES round is the aes crate's and its BLAKE2b the blake2 crate's. |
| BIP-327 MuSig2, with BIP-328 and BIP-390 | `core/faraday-bip/src/musig.rs` | `musig-key` | F178 | libsecp256k1's MuSig2 is in rust-secp256k1 0.32 and later; bitcoin 0.32, which Faraday uses, is on 0.29, and no miniscript release builds on bitcoin 0.33 yet (§6.1). |
| BIP-352 silent payments | `core/faraday-bip/src/silent.rs` | `silent-receive` | F178 | libsecp256k1's silent payments module is not yet in its Rust bindings (§6.2). |
| BIP-374 DLEQ proofs and BIP-375's sending fields | `core/faraday-bip/src/dleq.rs`, `core/faraday-psbt/src/silent.rs` | `silent-send` | F178 | No Rust library implements BIP-374 proofs or BIP-375's PSBT fields. |
| The PSBT version 2 reader | `core/faraday-psbt/src/v2.rs` | `psbt-v2` | F178 | rust-bitcoin reads only version 0 PSBTs; its version 2 crate is pre-release (§6.4). |
| BIP-322 "simple" message signatures | `core/faraday-psbt/src/message.rs` | `message-sign` | F178 | The bip322 crate is pre-release (§6.4). |
| SLIP-39 Shamir backup | `core/faraday-bip/src/slip39.rs` | `slip39` | F178 | No maintained Rust implementation of SLIP-39 exists. |
| CMS SignedData, Authenticode's image hash and signature, EFI signature lists and `.auth` files | `faraday/faraday-sb` | `sb-keys` | F178 | No library writes Authenticode signatures or EFI signature lists; the CMS crate for the DER version Faraday uses is a release candidate (§6.3). |
| BC-UR, with its CBOR (carries) | `core/faraday-codec/src/ur/`, `faraday/faraday-qr/src/cbor.rs` | `ur` | F178 | No library for this format comes from a publisher Faraday's dependency review trusts. |
| BBQr (carries) | `faraday/faraday-qr/src/bbqr.rs` | `bbqr` | F178 | No library for this format comes from a publisher Faraday's dependency review trusts. |
| SeedQR (carries) | `core/faraday-codec/src/seedqr.rs` | `seedqr-make` | F178 | No Rust library implements SeedQR. |
| The QR encoder, a port of Nayuki's `qrcodegen`, with its GF(256) and Reed-Solomon (carries) | `core/faraday-codec/src/qr.rs` | `qr` | F178 | No QR encoder comes from a publisher Faraday's dependency review trusts; Faraday's port of Nayuki's code wipes a SeedQR's buffers. |
| CRC-32 | `core/faraday-crypto/src/crc32.rs` | `ur` | F180 | The CRC-32 crate in the tree, crc32fast 1.5.1, is not covered by an outside review at that version; Faraday's is a branch-free loop checked against the published check values (§6.8). |
| FAT16 and FAT32, read and written (carries) | `faraday/faraday-fat` | `stick-write` | F180 | No FAT library comes from a publisher Faraday's dependency review trusts. |
| The FAT32 formatter for the Starter's data partition (carries) | `faraday/faraday-fat` | `starter-write` | F180 | No FAT library comes from a publisher Faraday's dependency review trusts; the partition it makes is checked by dosfstools' `fsck.fat`, against what `mkfs.fat` makes. Built (`faraday/faraday-fat/src/format.rs`). |
| GPT and MBR tables, read and rewritten (carries) | `faraday/shells/starter/src/gpt.rs`, `faraday/shells/starter/src/ids.rs` | `starter-write` | F180 | No partition-table library comes from a publisher Faraday's dependency review trusts. |
| The PDF writer (carries) | `faraday/faraday-core/src/pdf.rs`, `faraday/faraday-starter/src/sheets.rs` | `starter-recovery-key` | F180 | No PDF library comes from a publisher Faraday's dependency review trusts. |
| The plist reader for `diskutil`'s output (carries) | `faraday/shells/starter/src/macos/plist.rs` | `starter-write` | F180 | No plist library comes from a publisher Faraday's dependency review trusts. |
| WipeOnFree, the allocator that wipes what it frees | `core/faraday-crypto/src/wipe.rs` | `hold` | F180 | No wiping allocator comes from a publisher Faraday's dependency review trusts. |

**The FAT32 formatter** (owner, 2026-10-31; F180). The Starter makes the
stick's data partition itself, as `mkfs.fat -F 32` makes one
(`docs/STICK-IDENTITY.md`): a new piece written here, `none-established`
(fatfs 0.3.6, which has a formatter, has no trust entry and no imported
audit). It is written in `faraday/faraday-fat`, beside the reader, not in
the Starter's shell, and it is tested against dosfstools: each
partition it makes passes `fsck.fat -n`, and its boot sector, FSInfo
and FATs have the fields `mkfs.fat -F 32` gives for the same size and
label, apart from the serial (skipped with a note where dosfstools is
missing). faraday-fat's module doc says "The Starter formats the data
partition; nothing else formats one". It is built (`format32`, with its
tests in `faraday/faraday-fat/tests/formatted.rs`, which also hold the
volume it makes against `mkfs.fat` at the size either side of every
cluster-size step); its manifest entry is written with the others of the
`carries` tier in D1, which brings the entry's form (§2.1), and the count
below includes it.

### 3.3 One exists but does not fit the key rules

| Piece | Where | Action | Decision | Fails | Note |
|---|---|---|---|---|---|
| scrypt | `core/faraday-crypto/src/scrypt.rs` | `scrypt` | F178 | scrypt 0.12.0 frees its passphrase-derived working buffers (`b`, `v`, `t`) without wiping them | The Rust scrypt library leaves its passphrase-derived working memory unwiped; Faraday's wipes it (§6.5). |
| HKDF-SHA256, one copy | `core/faraday-crypto/src/hash.rs` | `vault-qr-pgp` | F180 | hkdf 0.13.0 returns the pseudorandom key and leaves each output block T(i) in plain arrays it does not wipe | The Rust HKDF library leaves the pseudorandom key and its output blocks unwiped; Faraday's holds both in buffers it wipes (§6.7). |
| BIP 93 codex32 | `core/faraday-bip/src/codex32/` | `codex32` | F178 | codex32 0.1.0 holds the seed in a `String` that is `Clone` and `Debug` and never wiped, which `tools/lint-secrets.sh` refuses, and needs `std`, which `faraday-bip` does not have | The reference library keeps the seed in text that is never wiped and needs a full operating system. |
| BIP-39 mnemonics: entropy to words and back, the checksum, a word's place in its list | `core/faraday-bip/src/bip39.rs` | `words-read` | F180 | bip39 3.0.0 finds a word by a binary search or a scan that stops at it (`find_word`) and packs entropy with a branch on each bit (`from_entropy_in`), so the time it takes follows the secret words | The Rust BIP-39 library's word lookup takes a time that follows the secret words; Faraday's looks each word up in constant time and is tested against that library and the reference. |
| The BIP-39 seed, PBKDF2-HMAC-SHA512 over the sentence through `faraday-crypto` | `core/faraday-bip/src/bip39.rs` | `seed` | F178 | bip39's `to_seed` returns the seed as a plain `[u8; 64]`, and its own PBKDF2 leaves its HMAC state unwiped (bip39 2.2.2's `src/pbkdf2.rs`) | The seed is derived with the RustCrypto PBKDF2 Faraday uses everywhere, into a buffer that is wiped. |
| The PNG writer, with zlib's stored blocks (carries) | `core/faraday-codec/src/png.rs` | `seedqr-make` | F180 | png 0.18.1 needs `std`, which `faraday-codec` does not have, and does not wipe a picture's rows | Faraday writes a SeedQR's picture in buffers it wipes; its Adler-32 is the adler2 crate's. |

### 3.4 Only the part used is written

| Piece | Where | Action | Decision | Note |
|---|---|---|---|---|
| OpenPGP v4 packets for one key shape, the armour's CRC-24, and the paperkey format | `faraday/faraday-pgp` | `gpg-create` | F178 | Faraday writes only the packets of its one key shape, over Ed25519 from dalek and SHA-1/SHA-2 from RustCrypto, each judged by GnuPG and paperkey; the armour's base64 is the base64ct crate's. |
| TOTP code derivation | `faraday/faraday-vault/src/totp.rs` | `totp` | F178 | Faraday writes only the few lines RFC 6238 needs, over standard HMAC. |
| Seed XOR | `core/faraday-entropy/src/xor.rs` | `xor` | F178 | Faraday writes only the few lines the standard needs, over standard XOR. |
| A 32-byte hash reduced modulo the curve order, without a branch | `core/faraday-bip/src/musig.rs` | `musig-key` | F178 | libsecp256k1's Rust bindings take a scalar only below the order and offer no reduction; Faraday's is a single conditional subtraction (§4.1). |
| The JSON visitor that refuses a key named twice and counts depth | `core/faraday-bip/src/json.rs` | `wallet-files` | F180 | serde_json reads the text; Faraday refuses a key named twice and a nesting deeper than 32. |
| The form of a version 4 UUID (carries) | `faraday/shells/starter/src/ids.rs` | `starter-write` | F180 | Four lines setting RFC 9562's version and variant bits over bytes from getrandom. |
| The KDBX 4 container a file vault is: header, VariantDictionary, HMAC block stream, inner header and gzip framing | `faraday/faraday-vault/src/kdbx/` | `vault-kdbx` | F190 | Faraday writes only the part of KeePass's format the vault uses (F190 point 11), over argon2, chacha20, aes and cbc, hmac, sha2 and miniz_oxide; keepass-rs, which no review Faraday trusts covers (§1.2), drops the XML elements it does not model and draws a save's randomness itself. |
| The QR vault's OpenPGP message: the version 6 symmetric-key packet with the Argon2 S2K, the version 2 data packet and the literal data packet | `faraday/faraday-pgp/src/message.rs` | `vault-qr-pgp` | F190 | Faraday writes only the three packets a QR vault uses (F190 point 11), over argon2, aes and ocb3 and Faraday's HKDF (§3.3); rPGP, which no review Faraday trusts covers (§1.2), brings every OpenPGP algorithm. |
| The TOML tree inside a vault: duplicates, limits and the names of `docs/VAULT.md` §2.1 and §7 | `faraday/faraday-vault/src/contents.rs`, `faraday/faraday-vault/src/contents/` | `vault-contents` | F189, F190 | toml_parser reads the tokens; Faraday builds the tree and refuses what VAULT.md does not name, as the JSON visitor does over serde_json (`docs/deps/toml_parser.md`). |

Two pieces in the tree today are in none of these tables, because
F190's vault replaces them: the four-equal-slot FVLT container and its
records (`faraday/faraday-vault`, `vault-create`, F178,
`own-specification`), read by nothing once the vault is built (F189
point 10, F190 point 9), and the KDBX 4 export
(`core/faraday-backup/src/kdbx.rs`, `kdbx`, F178, `none-established`:
no Rust KeePass library comes from a publisher Faraday's dependency
review trusts), which is folded into the vault (F190 point 10). Until then
each carries the reason given here.

Thirty-nine pieces in all once the vault is built, the formatter
included, against the eighteen lines of AUDIT §3 today. The switches
remove BLAKE2b, the AES round, the scalar arithmetic on secrets, the
RSA RNG adapter, X.509 and the DER writer, the single-key and multisig
finalizer, the sealing's counter nonce, four copies of base64, about
twenty of hex, the descriptor checksum, the JSON parser, Adler-32, the
hand-written wipes, and the second copy of HKDF's extract and of the
Argon2 call; the encodings, BIP-322 and the formats written here and
never listed are listed. Each kept piece is in one copy (§5.17).

## 4. The timing fixes, and what wiping reaches

Places where a secret's value steers the time an operation takes or the
memory it reads, and what the security model already bounds of the
copies a secret leaves in freed memory (§4.8, §4.9). On an offline
device the risk is low: an attacker needs code on the device or a probe
on its power or emissions. Each timing fix comes before the switches. Where an outside crate is no better (§4.5's word
lookup, §4.4's GF(32)), the fix is Faraday's own form, written without a
branch or a lookup on a secret, with the reason in §3.

Every batch here is proved by the vectors and cross-check areas it
names giving the same bytes, the tests of the crates touched, `just
crosscheck` where an area's code changed, and `just crypto-review` for
each action `python3 -I -B docs/crypto/check.py check` names as changed.
Each changes the manifest's `secrets.constant_time` (or `secrets.wiped`)
text of the actions named to the text given.

### 4.1 F-1: scalar arithmetic on secrets

**Where.** `core/faraday-bip/src/musig.rs`, `core/faraday-bip/src/dleq.rs`
and `core/faraday-bip/src/silent.rs`. Main has them branch-free in
Faraday's own form: `add_mod_order`, `negate_mod_order`, `reduce` and
`at_least_order` carry each borrow and carry as a number over u128 limbs
and pick a result by mask, and `mul_mod_order`'s double-and-add runs 256
fixed rounds over them. Secrets reach them in:

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

**Decided** (owner, 2026-10-31, F180, as F178 says): every operation on a
secret scalar goes through libsecp256k1, which is in the tree (secp256k1
0.29.1). The branch-free functions on main stay only where this section
says.

**The fix.**

- A secret 32-byte value becomes a scalar only through
  `SecretKey::from_slice` (libsecp256k1's `seckey_verify`), and a
  `Scalar` only through `Scalar::from(SecretKey)` (which copies the bytes
  and compares nothing), so no secret meets `Scalar::from_be_bytes`,
  whose comparison the crate documents as not constant time. A secret
  held as a `Scalar` is only handed to the library as a tweak; it is
  never compared, ordered or read by Faraday's code.
- `a + b` is `SecretKey::add_tweak`, `a · b` is `SecretKey::mul_tweak`,
  `-a` is `SecretKey::negate`, held in a type of Faraday's,
  `SecretScalar(Option<SecretKey>)`, where `None` is zero.
- A public factor (MuSig2's `g`, `gacc`, `b`, `e`, `a`; BIP-374's `e`)
  is a `Scalar` from its bytes and multiplies the secret as the tweak.
  Every such factor is below the order by construction (a hash reduced
  by `reduce`, or a key coefficient); `Scalar::from_be_bytes(..)` on it
  is an `expect` whose message says so. A public factor of zero gives
  zero.
- A hash reduced to a scalar (`reduce`) keeps main's branch-free form:
  `a - n` is always computed, and the borrow selects `a` or `a - n` by
  mask. It is the one piece of scalar arithmetic left written here
  (§3.4). `is_zero`, `zero_bit` and the secret form of `at_least_order`
  are gone: a nonce reduced from its hash is checked by
  `SecretKey::from_slice` (`seckey_verify`), which refuses zero and a
  value at or above the order.

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
functions say so in their first line and carry a `_public` suffix
(`add_mod_order_public`, `negate_mod_order_public`,
`mul_mod_order_public`, `at_least_order_public`), so that a secret passed
to one shows in review; being public they may branch.

**The limit, stated.** In rust-secp256k1 0.29.1 `SecretKey` is `Copy`,
and `add_tweak` and `mul_tweak` take `self` by value: every operation
copies the key. `SecretKey`s are therefore kept in short scopes, one
call's stack frame, and erased with `non_secure_erase` when that scope
ends (`Drop` for `SecretScalar`, and after each use in `silent.rs` and
`dleq.rs`), which reaches the last copy only; the earlier copies are
left on the stack, as the `[u8; 32]` intermediates are today. The
manifest's `secrets.wiped` of `sign-musig`, `silent-send` and
`silent-receive` says so.

**What must pass, unchanged.** Vectors `tools/vectors/bip327`,
`tools/vectors/bip328`, `tools/vectors/bip390`, `tools/vectors/bip352`,
`tools/vectors/bip374`, `tools/vectors/bip375`, `tools/vectors/psbt`;
the cross-check areas `musig`, `silent` and `silent_send`
(`just crosscheck`, which re-stamps); the tests of `faraday-bip` and
`faraday-psbt`, with a new test file, core/faraday-bip/tests/all/silent.rs, for the
sum's zero cases; `just crypto-review` for `sign-musig`, `musig-key`,
`silent-send` and `silent-receive`, and for every other action
`check.py check` names, since the renamed functions sit in a lock block
those actions share.

**Built.** Every point above is built: the secret arithmetic of
`musig.rs`, `dleq.rs` and `silent.rs` goes through libsecp256k1, the
public arithmetic carries the `_public` suffix, `reduce` is unchanged,
the `secrets.wiped` and `secrets.constant_time` texts say what is and is
not reached, the actions' lock and fingerprints are written again, and
the cross-check stamp is current.

**Secret handling that changes** (`secrets.constant_time`):

- `sign-musig`: "Every operation on a secret scalar is libsecp256k1's
  (`SecretKey` add, multiply and negate). libsecp256k1's header says its
  code is written and tested to be constant time whatever the secret
  values; it states nothing of these functions by name. A hash is
  reduced to a scalar without a branch; the code documents no other
  timing."
- `silent-send`: "Every secret multiplication goes through secp256k1's
  ECDH, and the response `k + e·a` through libsecp256k1's `SecretKey`
  add and multiply, of which its header says what `sign-musig` says."
- `silent-receive`: the sender sum and the tweak check, named the same
  way.

### 4.2 F-2: AEZ's AES round (built)

**Where.** `core/faraday-bip/src/aez.rs`, `round`, whose SubBytes was a
lookup in `SBOX` indexed by the state's secret bytes.

**Built.** `round` is `aes::hazmat::cipher_round` (aes 0.9.3,
`default-features = false`, feature `hazmat`); `SBOX` and `xtime` are
gone; `double` is masked and `mult` runs over public constants. The aes
crate documents every backend as designed to run in constant time: a
fixsliced software round with no table and no data-dependent branch,
AES-NI or VAES detected at run time on x86, ARMv8's instructions on
aarch64 where present (`src/lib.rs`, `src/hazmat.rs`: the hazmat round
takes the same paths as the cipher). Which backend runs is fixed by the
target (aes 0.9.3 `src/hazmat.rs`, `src/lib.rs`; nothing in the
repository sets `aes_backend`):

- The stick (`x86_64-unknown-linux-musl`) and the desktop app
  (`x86_64-unknown-linux-gnu`) take AES-NI where CPUID reports it, and
  the fixsliced round otherwise.
- The Pi image is `armv7-unknown-linux-gnueabihf`
  (`faraday/image/boards/pi3/board.conf`'s `RUST_TARGET`). 32-bit ARM is
  not in the crate's intrinsics `cfg`, so the Pi always runs the
  fixsliced software round, whatever its processor has: AND, OR, XOR,
  NOT and shifts by constant amounts on a machine word (cpubits 0.1.1
  makes it a `u64` on ARMv7), with no table and no data-dependent
  branch.

Both are documented constant time. The `aezeed` action says so.

That the compiled round on the Pi's Cortex-A53 holds no branch or
variable-time instruction is not measured. The owner may check it on a
Pi with the dev image; it is not a batch: a timing test of
`cipher_round` built for `armv7-unknown-linux-gnueabihf` with dudect's
method (Reparaz, Balasch and Verbauwhede, 2017: a fixed input against
random inputs, interleaved, each batch timed with
`CLOCK_MONOTONIC_RAW`, Welch's t staying under 4.5 as the count grows),
and `objdump -d` of the built round showing no conditional branch and no
load indexed by the state. Faraday has no timing harness of this kind.

### 4.3 F-3: SLIP-39's GF(256) and checksum (built)

**Built.** `core/faraday-bip/src/slip39.rs`'s `gf_mul` is eight masked
shift-and-XOR rounds modulo 0x11B, `gf_inv` is `a^254`, `Points::at`
branches only on public share indices, each Lagrange basis is a product
of quotients, and RS1024's `polymod` XORs its generator under a mask;
the `EXP` and `LOG` tables are gone. The `slip39` action says so. What is
left of SLIP-39 is its word lookup (§4.5) and its digest compare (§4.6).

### 4.4 F-4: codex32's GF(32), checksum and alphabet (batch A4)

**Where.** `core/faraday-bip/src/codex32/`:

- `gf_mul` (`mod.rs`) branches on the bits of `b` and on `a >= 32`;
  `mul32` (`correct.rs`) is the same code again as a `const fn`.
  Secret symbols reach them in interpolation (`interpolate`, the
  recovery of a secret from shares, `derive_share`, `split` and
  `fresh_set`), the worksheet's translation (`worksheet.rs`) and the
  corrector's elimination (`correct.rs`).
- `residue_step` (`mod.rs`) branches on the residue's top five bits; the
  residue is a running function of every secret character, folded by
  `polymod` and `long_polymod` (`verify_checksum` on every share or
  secret read, `append_checksum` in `from_seed`, `split` and
  `fresh_set`) and by the worksheet's rows.
- `charval` and `CHARSET` look a character up by its secret value
  (`mod.rs`, `correct.rs`); `StringCheck`
  (`faraday/faraday-keys/src/codex32.rs`) finds a character with
  `position()`, a scan that stops at it.
- `fresh_set` (`fresh.rs`) reserves room for the initial shares only and
  then pushes the derived ones past it: the reallocation frees the old
  block, with the initial shares in it, unwiped.

`INV` is not indexed by a secret: `lagrange` and the worksheet index it
by products of share indices, which are printed on every share, and the
corrector by pivots of the erased positions, which the person marked.
It stays.

**The fix.** No established crate does better: codex32 0.1.0's multiply
is a table with a zero branch, and the bech32 crate's `Fe32` multiplies
by `LOG` and `LOG_INV` tables and finds a character in `CHARS_INV`.
Faraday's own form, as SLIP-39's (§4.3):

- One `gf_mul`, a `const fn` in `mod.rs` that `correct.rs` calls in place
  of `mul32`: five rounds of
  `res ^= a & ((b >> i) & 1).wrapping_neg(); let hi = (a >> 4) & 1; a = ((a << 1) & 31) ^ (9 & hi.wrapping_neg());`
  (41 is x⁵ + x³ + 1; with bit five dropped by the mask, the reduction is
  XOR with 9).
- `residue_step`'s loop becomes
  `residue ^= generator[i] & 0u128.wrapping_sub(((b >> i) & 1) as u128)`;
  it stays a `const fn` (no `black_box`, as SLIP-39's `polymod`). The
  final compare with the constant is one compare whose result is public.
- `charval` becomes a scan of all 32 alphabet entries with a masked
  select (`v |= i & eq_mask(CHARSET[i], c)`), the character's validity
  taken from whether any entry matched; encoding a symbol is a masked
  scan likewise. `StringCheck`'s lookup calls the same function. A
  character outside the alphabet is refused as today; that the text
  typed holds such a character is not secret.
- `fresh_set` reserves the set's full count of strings
  (`scheme.strings_len()`) before the first push.

**Tests.** A test keeps the old `gf_mul` in the test module and checks
every one of the 32 × 32 products unchanged, and the old `residue_step`
against the new over every symbol and a sample of residues.

**What must pass, unchanged.** `tools/vectors/bip93`,
`tools/vectors/codex32-booklet`, `tools/vectors/codex32-volvelle`,
`tools/vectors/codex32-correction`; the cross-check areas `codex32` and
`codex32_correct`; the tests of `faraday-bip` and `faraday-keys`;
`just crypto-review` for `codex32`, `codex32-make` and `codex32-correct`.

**Secret handling that changes.** `codex32`, `codex32-make`,
`codex32-correct`: "GF(32) multiplication, the checksum's residue and the
alphabet's lookup are written without a branch or a table index on a
symbol's value, as the code documents; the code states no other
timing." (The text today says "the arithmetic is table lookups over
GF(32)", which is not so: it is shift-and-add with branches.)

About half a day.

### 4.5 F-5: word lookups (batch A5)

**Where.** The time to find a word follows its place in the list, so
each word's 10 or 11 bits are in the timing.

- `Language::index_of` and `index_of_nfkd`
  (`core/faraday-bip/src/bip39.rs`): a scan over 2048 entries that stops
  at the word, each compare stopping at the first byte that differs.
  Reached by `Mnemonic::parse`, by words pasted, scanned or read from a
  file (`faraday/faraday-core/src/scan_key.rs`), by the Inbox and scan
  classifier on every word of a secret text
  (`faraday/faraday-core/src/scan_read.rs`, over every language until one
  fails, so the time also says which list), and by aezeed's 24 words in
  English (`faraday/faraday-core/src/lightning/mod.rs`). Words typed on
  the on-screen keyboard come in as indices and do not take it.
- `slip39::index_of` (`core/faraday-bip/src/slip39.rs`): the same scan
  over 1024 words with `starts_with`, reached by `Share::parse` on every
  share typed, pasted or read from a file.
- `set11` (`bip39.rs`) writes each bit of a word's index with a branch,
  reached by `Mnemonic::entropy` and `validate_checksum`. `get11` is
  already branch-free.
- `aezeed::pack` (`core/faraday-bip/src/aezeed.rs`) branches on each bit
  of the 24 words, reached by the classifier and every decode; under the
  default passphrase "aezeed" the words are as good as the entropy.
- The sentence for PBKDF2 (`bip39.rs`, the seed's password) is gathered
  word by word with `word_nfkd(w)` by secret index: a load from the
  2048-entry table and a copy whose length follows the word.

**Decided** (owner, 2026-10-31, F180): the BIP-39 core stays Faraday's
(§3.3, §5.5): the bip39 crate's `find_word` is a binary search on four
lists and a scan that stops at the word on the others, its
`from_entropy_in` branches on each bit, and its `to_entropy_array` calls
`find_word`, so it takes the time Faraday's takes today.

**The fix.** Faraday's own form:

- `index_of` and `index_of_nfkd` read all 2048 entries for every word:
  each entry, padded to `MAX_NFKD_BYTES`, is compared with the word,
  padded the same way, by `subtle::ConstantTimeEq`, and the index kept
  by `ConditionallySelectable`; the result is the index or none. 2048 ×
  24 fixed-length compares per sentence take well under a millisecond.
  The classifier keeps trying each language, each with the full scan;
  which list a sentence is in is then given by its words, not its time.
- `slip39::index_of`: the first four letters, which identify a SLIP-39
  word, are compared with all 1024 entries' first four in constant time,
  and the rest of the typed word against the entry found, masked.
- `set11`: `let m = 1 << (7 - bit % 8); let set = 0u8.wrapping_sub(((v >> (10 - b)) & 1) as u8); buf[bit / 8] = (buf[bit / 8] & !m) | (m & set);`.
- `aezeed::pack`: `out[at / 8] |= (((word >> (10 - bit)) & 1) as u8) << (7 - at % 8);`.
- The sentence: each word is gathered by a scan of all 2048 entries,
  copying each into a fixed slot of `MAX_NFKD_BYTES` under a mask, and
  then the slot's bytes up to the word's length into the `Secret`
  buffer. The length of each word, and so of the sentence, still follows
  the words; BIP-39 allows no padding, so this is stated, not hidden
  (it decides how many SHA-512 blocks HMAC's key takes, once per seed).

**What must pass, unchanged.** `tools/vectors/bip39` in every language,
`tools/vectors/slip39`, `tools/vectors/aezeed`; the cross-check areas
`bip39`, `slip39` and `lightning`; the comparison of Faraday's BIP-39
with the bip39 crate (`core/faraday-bip/tests/all/crosscheck.rs`), which
stays; the fuzz target `bip39_words`; the tests of `faraday-bip`,
`faraday-keys` and `faraday-core`; `just crypto-review` for
`words-read`, `words-make`, `seed`, `slip39` and `aezeed`.

**Secret handling that changes.** `words-read`, `seed`: "A word is found
by comparing it with every entry of its list in constant time (subtle),
and the entropy's bits are written without a branch; the sentence is
gathered by a masked scan, and its length follows the words, as BIP-39
requires; the code states no other timing." `slip39`: the word lookup
named the same way, beside the text of §4.3. `aezeed`: "The words are
packed without a branch on their bits", beside §4.2's text.

About a day.

### 4.6 F-6: comparisons of secrets (batch A6)

**Where.** In `faraday/faraday-keys/src/session.rs`:
`Keys::passphrase_is` compares a key's BIP-39 passphrase with a vault
record's by `==`, which stops at the first byte that differs, beside the
constant-time `passphrase_matches`; `with_same_words` compares two keys'
word texts by `==`; `seedqr_reads_as` compares a key's SeedQR digits with
the digits scanned by `==`. In `core/faraday-bip/src/slip39.rs`, the
digest check compares two 4-byte slices by `!=`. In
`faraday/faraday-pgp/src/lib.rs`, `crc24` branches on the CRC's top bit
over paperkey data, which is secret key material.

**The fix.**

- `passphrase_is` calls `passphrase_matches` (one function stays).
  Unequal lengths still return early, as subtle's slice `ct_eq`
  documents; the length is stated.
- `with_same_words` compares the kept `words_id` (SHA-256 of the
  entropy), as `same_words` does.
- `seedqr_reads_as` and SLIP-39's digest compare with
  `subtle::ConstantTimeEq`.
- `crc24`'s step is masked as `crc32.rs`'s is:
  `crc ^= 0x186_4CFB & 0u32.wrapping_sub((crc >> 24) & 1)`.

**What must pass, unchanged.** The tests of `faraday-keys`,
`faraday-bip` and `faraday-pgp`; `tools/vectors/slip39`,
`tools/vectors/paperkey`; GnuPG's and paperkey's judgements in
`faraday-pgp`'s tests; `just crypto-review` for the actions named.

**Secret handling that changes.** `gpg-paperkey`: from "Not applicable:
no secret-dependent comparison is made" to "The armour's and paperkey's
CRC-24 runs without a branch on the data; the code states no other
timing." `slip39`: the digest is compared in constant time.

About two hours.

### 4.7 F-7: BIP-352 tweaks, and contexts that are not randomized (batch A7)

**Where.** BIP-352's label tweak `hash(b_scan ‖ m)` goes through
`PublicKey::add_exp_tweak` in `labelled_spend_key`
(`core/faraday-bip/src/silent.rs`), which is
`secp256k1_ec_pubkey_tweak_add`: it computes `tweak·G` with
libsecp256k1's variable-time `ecmult`, and the tweak comes through
`Scalar::from_be_bytes`, which rust-secp256k1 0.29.1 documents as not
constant time. A label tweak with its address links the label to the
receiver: privacy, not money. `label_point` in the same file already
computes the same point through a secret key.

The output tweak `t_k` (`scan`, the receiver's, and `sender_outputs`)
also goes through `add_exp_tweak`, and stays so. libsecp256k1's own
silent payments module (v0.8.0, commit 6e2c8bc4,
`src/modules/silentpayments/main_impl.h`) computes `t_k` the same way:
`create_output_tweak` declassifies the hashed secret ("The only thing
that the attacker can do with the hashed secret is derive the final
pubkeys"), and the output is `secp256k1_eckey_pubkey_tweak_add`, the
variable-time `ecmult` (`src/eckey_impl.h`). Its label tweak it treats
as secret: `recipient_label_create` computes it with `ecmult_gen` and
does not declassify it. Faraday follows the reference in both.
(Faraday's secp256k1-sys 0.10.1 bundles an older libsecp256k1 without
this module; §6.2.) (`group_scripts` in `core/faraday-psbt/src/silent.rs`
works on shares public to the PSBT's participants and is not changed.)

`docs/KEYS.md` §2 says each master key's curve operations run in a
context randomized from the key and the session. rust-secp256k1 0.29.1's
`Secp256k1::new()` randomizes only with its `rand-std` feature, which
Faraday does not turn on, so these contexts are not randomized:
MuSig2 signing (`core/faraday-psbt/src/sign.rs`), the vanity grinds
(`core/faraday-bip/src/vanity.rs`), and silent payments
(`faraday/faraday-keys/src/silent.rs`'s label points and
`find_payments`, `core/faraday-bip/src/silent.rs`'s `decode_scan_key`).
`ecmult_gen` is constant time either way; randomization is protection
against power and emission leakage on top.

**The fix.** `labelled_spend_key` is the spend key `combine`d with
`label_point` (`SecretKey::from_slice` of the tweak, then
`PublicKey::from_secret_key`: `ecmult_gen`, constant time and blinded),
the sum of two public points, as libsecp256k1's
`create_labeled_spend_pubkey` adds a stored label point; a tweak that is
zero or at or above the order is refused as `checked` refuses it today.
`t_k` is not changed. Each context named above is the key's own
(`MasterKey::secp`, `core/faraday-bip/src/keys.rs`) or one built as
`blinded_context` builds it.

**What must pass, unchanged.** `tools/vectors/bip352`,
`tools/vectors/bip375`, `tools/vectors/bip327`; the cross-check areas
`silent`, `silent_send` and `musig`; the vanity tests; `just
crypto-review` for `silent-receive`, `silent-send`, `sign-musig` and
`vanity`. `docs/KEYS.md` §2's sentence is then true, and is not changed.

**Secret handling that changes.** `silent-receive`: "A label tweak
times G is libsecp256k1's `ecmult_gen` through a secret key, in a
randomized context; the output tweak is treated as public and
multiplied by `ecmult`, as libsecp256k1's own silent payments module
does." `silent-send`: "The output tweak is treated as public and
multiplied by `ecmult`, as libsecp256k1's own silent payments module
does."

About half a day.

### 4.8 Copies of secrets in freed memory

The secrets that matter are wiped where Faraday's own types hold them
(faraday-keys' types, `SecretText`, `Zeroizing`). Other copies are left
in freed heap: buffers that grow, plain copies, other crates' buffers.
On the stick and the Pi each is bounded by the security model: the
app's process restarts at every lock and the kernel zeroes its pages
(`init_on_free=1`), there is no swap, and the computer is powered off
after use (§4.9, F181). They are not chased one by one. Faraday Demo
and the Starter run on an online computer, and no memory or timing work
is done for them alone. Where a manifest line or a module doc claims
more than the code does, it is corrected (§7.8).

### 4.9 A wiping allocator

`core/faraday-crypto/src/wipe.rs`'s `WipeOnFree` is a global allocator
that wipes every block it frees, and the old block on every grow or
shrink (a reallocation is always a move). It is installed in no shipped
program.

**Not installed** (F181). It writes every
freed byte and moves every growing buffer, on every allocation in the
program, so its cost falls on drawing, the camera and the Starter's
writes, most on the Pi; what it adds is a wipe of copies in freed heap
before the process ends. Faraday runs on an offline computer that is
powered off after use, and `docs/SECURITY-MODEL.md` already leaves
cold-boot attacks on RAM out of what it defends against. On the stick
and the Pi, a copy in freed heap is bounded already: the kernel boots
with `init_on_free=1` (`faraday/image/boards/pi3/cmdline.txt`,
`faraday/image/boards/x86_64-uefi/cmdline.txt`), which zeroes pages as
they are freed, the app's process restarts at every lock, and there is
no swap, suspend, hibernation or core dump. The cost is not worth that
gain. The secrets that matter are wiped where they are held (faraday-keys'
types, `SecretText`, `Zeroizing`); the other copies are bounded as §4.8
says.

**What is reopened, and how.** Installing it in any program is a new
decision, made with numbers: the program's late-flow timings with and
without it, its peak memory, the full test suite run with it installed,
the allocator run under Miri, and a fuzz target that grows and shrinks
buffers through it. It is never installed in the website's build.
`wipe.rs` stays in `faraday-crypto`, tested, and the AUDIT row says it is
installed by no binary (§7.3).

**What is not reached**, the stated limits:

1. A secret held in a plain type while it is live: it is in memory, in
   the clear, until it is dropped.
2. A copy in freed heap, until its block is reused or the process ends:
   on the device, the next lock at the latest (the app restarts), or
   power-off for faraday-disk, which outlives the app's locks
   (`faraday/faraday-disk/src/main.rs`); in Faraday Demo and the Starter,
   until the program exits.
3. The stack and registers, including the copies of `Copy` types
   (rust-secp256k1 0.29.1's `SecretKey`, §4.1) and the slots a value was
   moved from; hmac's and pbkdf2's key blocks (§6.7).
4. Memory other code allocates itself: C libraries' own allocators, the
   graphics stack, the window system's text input and clipboard (on the
   desktop another process owns the clipboard), the kernel's pipe
   buffers to faraday-disk, and files written.
5. Outside the process: swap, sleep images and crash dumps. The stick
   and the Pi have none (`docs/SECURITY-MODEL.md`); for Faraday Demo and
   the Starter they are the desktop operating system's to decide.

§7.9 has `docs/KEYS.md` and the manifest say these limits.

### 4.10 Display-only and input-only

These let a secret steer time but are not fixed, because what the timing
gives is what the screen shows at the moment it is computed: someone who
reads the timing learns at most what anyone seeing the screen learns
then. The risk is lower, not nil (a timing channel can be read by
someone who cannot see the screen), and the shared functions of §4.4 to
§4.6 fix most of them anyway.

- Display-only: BIP-39 word display and the predictive keyboard
  (`Language::word`, `candidates`; the words are drawn); the last word's
  candidates (shown); dice, coin and card statistics and warnings
  (`core/faraday-entropy`; the rolls are on the pad as typed); codex32's
  worksheets and corrector (`worksheet.rs`, `correct.rs`'s
  Berlekamp-Massey and Forney steps; every value is a cell shown or a
  correction the person accepts box by box); `compare_words` and
  `another_seed` (the screen says which word differs); xprv, WIF, hex
  and BIP-85 text when shown or written out by the person's choice
  (base58ck's `Display`, BIP-85's base85 and base64 passwords); SeedQR
  and QR encoding (the mask choice and penalty scoring); TOTP's dynamic
  truncation.
- Input-only: the QR decoder over camera frames (rqrr branches on
  pixels; the secret is held up to the camera). This is so for timing
  only: the payload it decodes is a secret in a buffer, bounded as §4.8
  says.
- By design of the algorithm: scrypt's ROMix and Argon2id's second half
  read memory by passphrase-derived values (RFC 7914, RFC 9106 §3.4). The
  manifest's text for `scrypt`, `aezeed`, `vault-kdbx`, `vault-qr-pgp`
  and `kdbx` says so.
- A stated limit: RSA key generation for the Secure Boot keys
  (`faraday/faraday-sb`, `x509::generate`, rsa 0.10.0-rc.18) is not
  constant time. Its prime search, crypto-primes 0.7.2, branches on the
  candidate that becomes p or q: the strong Lucas test's ladder branches
  on every bit of `d` (`src/hazmat/lucas.rs`, `bit_vartime`), and the
  Jacobi symbol, the gcd and Miller-Rabin's loop count are `_vartime`.
  No constant-time prime search exists in Rust, and rsa has no option
  for one. It runs once per key, offline, on the device that will hold
  it; the keys guard boot integrity, not money. Signing is blinded
  (rsa's `rsa_decrypt_and_check` blinds the base with a value drawn from
  a ChaCha20 stream seeded from the system; the CRT exponentiation is
  crypto-bigint 0.7.10's constant-time `pow`).
  RUSTSEC-2023-0071 (Marvin) is a decryption-padding oracle and is not
  reachable: Faraday never decrypts with RSA, signs only digests of what
  it built, and has no network (`deny.toml` ignores it with that
  reason). Checked again when rsa 0.10.0 is released or crypto-primes
  gains a constant-time Lucas ladder.

## 5. The switches

Each subsection is one batch (§8). In every switch the function a caller
uses keeps its signature, so the choke point (AUDIT §1), the key
boundary (F70: no key leaves `faraday-keys` but through its reveals) and
the compute token (`c: &Compute`, `docs/COMPUTE.md`) are unchanged; what
Faraday refuses before the call is unchanged. Every switch also does
these, which the subsections do not repeat:

1. **Manifest (F179).** `[library.<crate>] author = "<the project's
   developers>"` is added to `docs/crypto/actions.toml` for a crate first
   named; the crate is named in `implementation.libraries` of each action
   whose functions call it directly and in `implementation.through` (with
   the Faraday crate it is reached through) of each action that reaches
   it through another crate; a library no longer reached is dropped;
   `faraday/faraday-learn/src/how.rs` is regenerated
   (`docs/crypto/gen_how.py`).
2. **Inventory.** A crate that computes cryptography is added to
   `PRIMITIVE_CRATES` (`tools/costs-lint/src/inventory.rs`) in the batch
   that makes it a direct dependency: `blake2` (C1), `x509-cert` (C5).
   `der`, `miniscript`, `base64ct`, `base16ct`, `serde_json`, `adler2`,
   `subtle` and `zeroize` compute nothing on a key and are not.
3. **Locks.** `Cargo.lock`, `fuzz/Cargo.lock` and `faraday/fuzz/Cargo.lock`
   change with any crate a crate they build gains; each subsection's "New
   in the graph" is for the main lock, and the batch updates the fuzz
   locks with `cargo update -p <crate> --offline` in each fuzz workspace.
4. **Trust.** A new trust entry is written with `start` the date of the
   first version the publisher published, `end = "2027-10-01"` and
   `notes = "F71: trusted publisher (docs/decisions/F71.md)"`, and `just
   vet-refresh` (online) records the publisher in
   `supply-chain/imports.lock` in the same commit.
5. **deps page.** A new direct dependency gets its `docs/deps/` page
   (rule `deps.page`); a crate already a direct dependency elsewhere has
   its page's purpose extended.
6. **Fingerprints.** After the change `python3 -I -B docs/crypto/check.py
   check` names every action whose fingerprint changed; each is a
   `just crypto-review <id>`. `just crosscheck` runs where an area's
   code changed, and re-stamps.
7. **AUDIT.** The `written_here` string of the action changes as given
   (before D1 makes it an entry), and AUDIT §3's line, and §1's row where
   named, with it (`check_audit_crates`).

### 5.1 BLAKE2b (batch C1, built)

**Built.** `core/faraday-crypto/src/blake2b.rs` is a wrapper of about 25
lines over `blake2` 0.11.0's `Blake2bVarCore`; RFC 7693's vectors and every
length 1 to 64 are in `core/faraday-crypto/tests/primitives.rs`. What
follows is the plan it was built from.

- **Files.** `core/faraday-crypto/src/blake2b.rs` (179 lines) becomes a
  wrapper of about 25 lines; `core/faraday-crypto/Cargo.toml`.
- **Crate.** `blake2` 0.11.0 (RustCrypto/hashes), `default-features =
  false, features = ["zeroize"]` (its `Drop` wipes the state under it).
  blake2 0.11.0 has no `Blake2bVar` (removed in 0.11.0); it has
  `Blake2b<OutSize>` with the size a type, and `Blake2bVarCore` with the
  size given at run time through digest's `VariableOutputCore`.
- **The wrapper.** `faraday_crypto::blake2b(data, out)` keeps its
  run-time length, 1 to 64 bytes, and its panic outside that, over
  `Blake2bVarCore`: `VariableOutputCore::new(out.len())`, a
  `BlockBuffer` for the blocks, `finalize_variable_core`, then the first
  `out.len()` bytes; the core's output buffer is wiped after the copy.
- **New in the graph.** None: blake2 0.11.0 is in `Cargo.lock` through
  argon2. The edge `faraday-crypto -> blake2` is new.
- **cargo vet.** `[[trusted.blake2]]` exists.
- **deps page.** New: docs/deps/blake2.md.
- **Manifest.** `aezeed`'s `through` gains `{ library = "blake2", crate =
  "core/faraday-crypto" }`; `[library.blake2] author = "the RustCrypto
  developers"`; `scrypt`'s `written_here` becomes "scrypt
  (`faraday-crypto::scrypt`)". AUDIT §3 loses "BLAKE2b".
- **Proof.** RFC 7693's vectors, today inline in `blake2b.rs`, move to
  `core/faraday-crypto/tests/primitives.rs`, with the existing tests at
  every length 1 to 64; `tools/vectors/aezeed`; area `lightning`.
- **Size.** About three hours.

### 5.2 HKDF: not a switch

Decided (owner, 2026-10-31, F180): HKDF stays Faraday's, `does-not-fit`
(§3.3). hkdf 0.13.0 wipes its HMAC state on drop (through `hmac/zeroize`,
`sha2/zeroize` and `block-buffer/zeroize`, which `faraday-crypto` turns
on) but returns the pseudorandom key as a plain `Output<H>`
(`GenericHkdfExtract::finalize`), discards it unwiped in
`GenericHkdf::new`, and keeps each output block T(i) in a plain
`Option<Output<H>>` it does not wipe; for a 32-byte output T(1) is the
key. Faraday's code holds the key in `Secret` and T in `Zeroizing`. A
change that wipes them is proposed to RustCrypto/KDFs, as for scrypt;
§6.7 says when the switch is made. The two copies become one in batch
C15 (§5.17). F178's trust entry for `hkdf` is not written.

### 5.3 AEZ's AES round (built)

Built with §4.2: `aes` 0.9.3 (RustCrypto/block-ciphers), feature
`hazmat`, `default-features = false`; `[[trusted.aes]]` with
`trusted-publisher = "github:RustCrypto/block-ciphers"`, start
2026-04-10, recorded in `supply-chain/imports.lock`; docs/deps/aes.md;
`aes` in `PRIMITIVE_CRATES`. AEZ's mode, `Extract`, `AES4`, `AES10` and
the doubling stay Faraday's (§3.2).

### 5.4 Scalar arithmetic

Built as §4.1 (batch A1). No new crate: secp256k1 0.29.1 is in the tree
and trusted.

### 5.5 The BIP-39 core: not a switch

Decided (owner, 2026-10-31, F180): the BIP-39 core stays Faraday's,
`does-not-fit` (§3.3), made constant time in batch A5 (§4.5), because
the bip39 crate's word lookup and bit packing take a time that follows
the words. The bip39 crate stays what the tests compare against: a
dev-dependency of `faraday-bip` at 2.2.2 under its exemption,
`core/faraday-bip/tests/all/crosscheck.rs` and area `bip39`
(python-mnemonic) as today. F178's trust entry for the bip39 crate's
publisher is not written; the crate's three publishers (stevenroose,
elsirion, who published 2.2.2, and tnull, who published 3.0.0) are
recorded in its deps page should it be taken later (§6.9). The
`words-read` written-here string becomes two entries at D1 (§3.1, §3.3);
before D1 it stays "BIP-39 mnemonics (`faraday-bip::bip39`)".

### 5.6 The RSA RNG adapter (batch C4, built)

- **Files.** `faraday/faraday-sb/src/rng.rs` (now a constructor of about 10 lines), its callers in
  `faraday/faraday-sb/src/lib.rs`.
- **Crate.** `chacha20` 0.10.2's `ChaCha20Rng`: `faraday-sb`'s chacha20
  gains feature `rng` (over `rand_core` 0.10), with `zeroize`, under
  which the generator is `ZeroizeOnDrop` and wipes its block buffer.
- **New in the graph.** None: chacha20's dependency list gains
  `rand_core` 0.10.1, in the tree, covered by imported audits.
- **cargo vet.** No change.
- **deps page.** `docs/deps/chacha20.md` adds the Secure Boot keys'
  generator to its purpose.
- **Kept in front.** One fresh 32-byte draw as the seed, taken as today
  from the system with the person's own randomness hashed in;
  `Stream::new(seed, what)` becomes `ChaCha20Rng::from_seed(seed)` then
  `set_stream(u64::from(what))` before any output, with the same `what`
  values as today (the GUID and serials, the key, the blinding).
- **Output.** `ChaCha20Rng` lays out its counter and stream as djb's
  ChaCha20 (64-bit counter, 64-bit stream), not RFC 8439's 32-bit
  counter and 96-bit nonce, and buffers its blocks, so one seed gives
  different keys than today. Keys already made are unaffected: they are
  stored in a vault. `faraday/faraday-sb/tests/golden.rs` is re-pinned;
  the test in `faraday/faraday-sb/tests/judged.rs` that checks the
  stream against RFC 8439 Appendix A.1 is replaced by one that the
  stream for a seed is the crate's, and every key made is judged by
  `openssl pkey -check` and every image by `sbverify` as today.
- **Manifest.** `sb-keys` keeps `chacha20` and `rand_core`; its `adds`
  drops the adapter; `written_here` becomes "X.509/CMS structure and
  Authenticode signing (`faraday-sb`)".
- **Size.** About half a day with the re-pinning.

### 5.7 X.509 and DER (batch C5, built)

**Built.** The hand-written DER writer is gone. `x509.rs` builds the
certificates with `x509-cert`'s `CertificateBuilder` (a profile of
Faraday's three extensions in today's order) and signs through `rsa`'s
`pkcs1v15::SigningKey<Sha256>`, which meets the builder's bounds
(`build_with_rng` with C4's blinding stream) with `faraday-sb`'s rsa
features; reads a certificate with `Certificate::from_der`. `cms.rs`
writes and reads the SignedData, SignerInfo, ContentInfo and
Authenticode structures as `der` types. The golden file is unchanged:
every certificate, `.auth` and signed image is the same bytes for the
same draw, and `openssl` and `sbverify` judge them as before. The
cross-check's change (batch X2, §9.7) is built: GnuTLS's `certtool` reads
every certificate made, `pesign` and `pesigcheck` read the real signed
binaries of `tools/reference/signed-efi/` and name their signer and image
hash as Faraday does, and python3-pefile reads the checksum and
certificate-table entry of every image. `pesigcheck` does not verify an
image Faraday signs: NSS takes only a signer certificate that is not a CA
and carries the code-signing extended key usage, and the db certificate
(§5.7's profile, kept so that every certificate is the same bytes) is a CA
with key usage alone; `pesigcheck` also aborts on an image with a byte
changed. Whether the db certificate's profile changes is the owner's call.

- **Files.** The DER writer (164 lines) and
  `faraday/faraday-sb/src/x509.rs` (336 lines); the CMS, Authenticode
  and EFI code that builds DER with that writer
  (`faraday/faraday-sb/src/pe.rs`, `faraday/faraday-sb/src/efi.rs`),
  now through `faraday/faraday-sb/src/cms.rs`.
- **Crates.** `x509-cert` 0.3.0, `default-features = false, features =
  ["builder"]` (`pem` and `std` off: Faraday writes DER), for the
  self-signed certificates and their reading; `der` 0.8, a direct
  dependency, `default-features = false, features = ["alloc", "oid"]`,
  for the CMS SignedData and Authenticode structures, written as `der`
  types (§3.2: only those structures stay Faraday's). The builder signs
  through `signature` 3's traits: the signer is rsa's
  `pkcs1v15::SigningKey<Sha256>` with its `RandomizedSigner` taking C4's
  blinding stream. Whether that type meets the builder's bound with
  `faraday-sb`'s rsa features is not checked; the batch checks it first
  and stops if it does not.
- **New in the graph.** `x509-cert` 0.3.0; its `der` features `derive`
  and `flagset` add `der_derive` 0.8.0 (a proc-macro over proc-macro2,
  quote and syn 2, in the tree) and `flagset` 0.4.7. sha1 0.11.0,
  signature 3.0.0, spki 0.8.0, const-oid 0.10.2 and der 0.8.2 are in the
  tree.
- **cargo vet.**
  - `[[trusted.x509-cert]]`, `trusted-publisher =
    "github:RustCrypto/formats"`, `start = "2026-07-09"` (0.3.0, its
    first release by trusted publishing).
  - `[[trusted.der_derive]]`, `user-id = 267 # Tony Arcieri (tarcieri)`,
    the RustCrypto person F71 already trusts for base16ct, base64ct,
    const-oid and others, `start = "2020-01-01"`.
  - `[[exemptions.flagset]]` 0.4.7, `safe-to-deploy`, with the note that
    Mozilla's audit covers 0.4.3 (F71: an exemption; no deps page, as it
    is not a direct dependency).
- **deps pages.** New: docs/deps/x509-cert.md and docs/deps/der.md.
- **Kept in front.** The certificates' fields as today (subject, serial
  from the draw, validity, the key usage and extensions written today);
  the reader's refusals of a certificate that is not the shape Faraday
  wrote, which the batch lists in docs/deps/x509-cert.md from today's
  `x509.rs` before replacing it, and each keeps a test.
- **Manifest.** `sb-keys`' `libraries` gain `x509-cert` and `der`;
  `[library.x509-cert]` and `[library.der]`, author "the RustCrypto
  developers"; `x509-cert` in `PRIMITIVE_CRATES`; `written_here`
  becomes "CMS SignedData, Authenticode's image hash and signature, EFI
  signature lists and `.auth` files (`faraday-sb`)".
- **Proof.** `faraday/faraday-sb/tests/judged.rs`: every certificate and
  `.auth`'s PKCS #7 judged by `openssl`, signed images by `sbverify`;
  `faraday/faraday-sb/tests/golden.rs` gives the same bytes for the same
  draw (DER is canonical; where x509-cert orders an extension
  differently, the golden file changes and `openssl` still judges it).
  The Starter's check of a signed boot image (`starter-signed`) reads
  the same.
- **Size.** One and a half to two days.

### 5.8 The single-key and multisig finalizer (batch C6, built)


**Built.** `build_final` calls miniscript's `finalize_inp_mut` for every script kind, with the interpreter check; a plain multisig carries the first m signers' signatures as Bitcoin Core writes them, and only a miniscript is given the smallest satisfaction by signature size. Area `sign` also asks btclib-wallet's finalizer for the scriptSig and witness of every input Faraday finalizes (batch X3, §9.7). What follows is the plan it was built from.

- **Files.** `core/faraday-psbt/src/finalize.rs` (`build_final` and
  `push`).
- **Crate.** `miniscript`, in the tree and trusted: its
  `finalize_inp_mut`, which Faraday already calls for miniscript and
  taproot inputs, also infers pk, pkh, wpkh, sh-wpkh, and multi under sh,
  wsh and sh-wsh from the PSBT's own scripts (`get_descriptor` in
  `src/psbt/finalizer.rs`). The batch stays on 12.3.7; 12.3.8
  (2026-10-27) is taken by the ordinary dependency update, not here.
- **New in the graph.** None.
- **cargo vet, deps page.** No change; `docs/deps/miniscript.md` adds
  the finalizer for every script kind.
- **Kept in front.** The interpreter check on every witness; the choice
  of the smallest satisfaction by real signature size for a miniscript
  (a `multi()` inside it keeping its k smallest signatures); a plain
  multisig, under `sh`, `wsh` or `sh(wsh)`, carrying the first m
  signers' signatures as Bitcoin Core's `finalizepsbt` writes them (§3.1);
  the signatures under a taproot `pkh` leaf taken off as today;
  the finalizer run on a copy of the input, as `satisfy` does today,
  because miniscript's `finalize_input` drops the unknown fields:
  Faraday keeps the UTXO and unknown fields and clears the signing
  fields and the hash preimages as BIP-174 requires, and the
  proprietary fields too (BIP-174 does not require keeping them).
- **Proof.** `tools/vectors/psbt` (BIP-174's finalizer and extractor
  vectors); area `sign` (Bitcoin Core's `finalizepsbt` and
  `testmempoolaccept`); the wallet fixtures
  (`core/faraday-psbt/tests/all/wallet_fixtures.rs`). The transactions
  are byte for byte the same.
- **Size.** One to two days.

### 5.9 The sealing's counter nonce, removed (batch C3, built)

**Built.** `core/faraday-crypto` has no `Sealed`, `SessionKey`, `SealError` or
`SealedBytes`, and no `chacha20poly1305` dependency; `MnemonicBytes`,
`SeedBytes`, `MAX_SEED_LEN` and `MAX_MNEMONIC_WORDS` are in
`core/faraday-crypto/src/secret.rs`. `KEY_LEN`, `NONCE_LEN` and `TAG_LEN` had
no user and went. What follows is the plan it was built from.

- **Files.** The sealing's file (653 lines, `sealed.rs`):
  `Sealed`, `SessionKey`, `SealError`, `SealedBytes`, the counter nonce,
  and `KEY_LEN`, `NONCE_LEN` and `TAG_LEN` (unless a user is found, then
  they move with it) go; `MnemonicBytes`, `SeedBytes`, `MAX_SEED_LEN` and
  `MAX_MNEMONIC_WORDS`, which `faraday-bip`, `faraday-keys` and
  `tools/crosscheck`'s Rust use, move to `core/faraday-crypto/src/secret.rs`.
  `core/faraday-crypto/src/lib.rs`'s exports and module doc; the
  comment in `core/faraday-bip/src/keys.rs` that waits for a
  `Sealed<T>`; `tools/lint-secrets.sh`: rule 2's type list loses `Sealed`
  and `SessionKey` (the allowed name `backup_flow`'s `Sealed` in
  `faraday-core` stays), and `SECRET_FILES` loses `sealed.rs` and gains
  `core/faraday-crypto/src/secret.rs` if it is not there.
- **New in the graph.** None; `faraday-crypto` drops its
  `chacha20poly1305` dependency (`faraday-vault` keeps its own for the
  FVLT container, and drops it when F190's vault replaces that
  container).
- **Reason.** No shipped code builds a session key or seals a value
  (the manifest's `hold`; STANDARDS §7 item 3). Should sealing in memory
  come back, it is XChaCha20-Poly1305 with a random nonce from the same
  crate, and a decision.
- **Manifest.** `hold`'s `written_here` goes; its `libraries` drop
  `chacha20poly1305` (and `subtle` if nothing else in its functions uses
  it); its `adds` drops the sentence on the in-memory sealing and its
  `limits` "Nothing held in memory is sealed"; how.rs is regenerated.
  STANDARDS §7 loses item 3 (generated); AUDIT §1's `faraday-crypto` row
  drops `Sealed`/`SessionKey`.
- **Proof.** The tests of `faraday-crypto`, `faraday-keys` and
  `faraday-core`.
- **Size.** About two hours.

### 5.10 Base64, four copies (batch C8, built)

**Built.** `faraday_bip::base64`, the armour in `faraday-pgp`, the KDBX
writer in `faraday-backup` and the Starter's plist reader are
`base64ct` 1.8.3's. A PSBT with padding bits that are not zero, or a
missing `=`, is refused (`core/faraday-psbt/tests/all/vectors.rs`); no
BIP-174 vector or wallet fixture carries one. What follows is the plan it
was built from.

- **Files.** `core/faraday-bip/src/base64.rs` (170 lines; PSBT v0 and v2,
  BIP-322, BSMS, BIP-85's password through `encode_into`);
  `faraday/faraday-pgp/src/lib.rs` (`base64`, the armour's encoder, and
  `Base64Reader` with `base64_value`); `core/faraday-backup/src/kdbx.rs`
  (`base64`, protected values and the UUID);
  `faraday/shells/starter/src/macos/plist.rs` (`base64`, `diskutil`'s
  data).
- **Crate.** `base64ct` 1.8.3 (RustCrypto/formats), `default-features =
  false, features = ["alloc"]`: `Base64::encode_string`,
  `Base64::decode_vec`, and the slice forms `encode` and `decode`. It is
  `no_std` and constant time with respect to the data (not the length).
- **The forms.** `faraday_bip::base64::{encode, encode_into, decode}`
  keep their signatures over base64ct, `DecodeError` mapped from
  `base64ct::Error`; the module's reason sentence (bitcoin's base64
  feature pulls `std`) goes. faraday-pgp's armour encoder is
  `encode_string`; `Base64Reader` is replaced by gathering the body's
  characters with CR, LF, space and tab removed into one buffer and
  `decode_vec` (armour line widths vary between writers, so
  `Decoder::new_wrapped` is not used). kdbx's `base64` encodes into its
  `out` with `encode`. plist's strips ASCII white space, then
  `decode_vec`.
- **Edge cases.** base64ct refuses padding bits that are not zero and a
  missing `=`; today's decoders accept the first. Faraday refuses them
  too (RFC 4648 §3.5 allows it, and an encoding with a second spelling
  is not taken). A test feeds a PSBT with non-zero padding bits and sees
  it refused. If a BIP-174 vector or a wallet fixture carries such a
  PSBT, the batch stops and the owner is told.
- **New in the graph.** None: base64ct 1.8.3 is in the lock (through
  argon2, spki and pem-rfc7468). `faraday-bip`, `faraday-pgp`,
  `faraday-backup` and the Starter's shell gain the edge.
- **cargo vet.** `[[trusted.base64ct]]` exists (tarcieri, 267).
- **deps page.** New: docs/deps/base64ct.md, listing the four users.
- **Manifest.** `bip85`'s `secrets.constant_time`: from "not stated by
  the library" to "The password's base64 is the base64ct crate's, which
  documents its encoding as constant time with respect to the data; its
  base85 is shown (§4.10)." `[library.base64ct] author = "the RustCrypto
  developers"`; `libraries` and `through` of `psbt-read`, `psbt-v2`,
  `message-sign`, `message-verify`, `bsms`, `bip85`, `gpg-create`,
  `kdbx` and the actions §5 item 1 finds.
- **Proof.** `tools/vectors/psbt`, `tools/vectors/psbt-bips`,
  `tools/vectors/message`, `tools/vectors/bip129`, `tools/vectors/bip85`;
  areas `psbt`, `msg`, `bip85`, `kdbx`, `import`; GnuPG reading the
  armour in `faraday-pgp`'s tests; the Starter's macOS plist fixtures;
  RFC 4648 §10's vectors in `faraday-bip`'s tests.
- **Size.** About half a day.

### 5.11 Hex, one copy (batch C9)

- **Files.** The copies carrying secrets:
  `faraday/faraday-intake/src/secret_buf.rs` (`hex`, private keys, which
  `faraday/faraday-keys/src/lightning.rs`'s `private_hex` reaches),
  `faraday/faraday-keys/src/bip85.rs` (`hex`, a duplicate of the last,
  which goes), `core/faraday-entropy/src/hex.rs` (`RawHex`, typed
  entropy), `faraday/faraday-pgp/src/paperkey.rs` (paperkey's base16 of
  secret key material). The public copies:
  `core/faraday-codec/src/encodings.rs`, `core/faraday-psbt/src/verify.rs`,
  `core/faraday-psbt/src/sign.rs`, `faraday/faraday-core/src/backup_flow/mod.rs`,
  `faraday/faraday-core/src/backup_flow/gpg_restore.rs`,
  `faraday/faraday-core/src/backups/mod.rs`, `faraday/faraday-core/src/decode.rs`,
  `faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-qr/src/bbqr.rs`,
  `faraday/faraday-qr/src/envelope.rs`, `faraday/faraday-payload/src/lib.rs`,
  `faraday/shells/starter/src/windows/disks.rs`,
  `faraday/shells/starter/src/windows/elevated.rs`,
  `faraday/faraday-intake/src/single_key.rs`, `core/faraday-selftest/src/lib.rs`,
  and `faraday/faraday-vault/src/contents.rs` (`hex` and `unhex`, the
  fingerprints in a vault's TOML, added since).
  (Each was read on 2026-10-31; the batch greps for 16-entry hex tables
  and `to_digit(16)` and `from_str_radix(.., 16)` over bytes and takes
  every hit outside tests.)
- **Crate.** `base16ct` 1.0.0 (RustCrypto/formats), `default-features =
  false, features = ["alloc"]`: `lower::encode`, `lower::encode_str`,
  `mixed::decode`, `decoded_len`; `no_std`, "without data-dependent
  branches or lookup tables". hex-conservative, also trusted and in the
  lock, looks digits up in a table and is not used for this.
- **One place.** A module `faraday_crypto::hex` with
  `encode_lower(bytes, out: &mut [u8]) -> &str`,
  `encode_secret(bytes) -> Zeroizing<String>` and
  `decode_into(text, out: &mut [u8]) -> Option<usize>` (either case, the
  length checked first), over base16ct. Every crate above that depends
  on `faraday-crypto` calls it; `faraday-pgp` and `faraday-payload`,
  which do not, call base16ct directly. Upper-case output, where a
  format asks for it (paperkey), is `upper::encode`.
- **New in the graph.** None: base16ct 1.0.0 is in the lock (through
  serdect). The edges from `faraday-crypto`, `faraday-pgp` and
  `faraday-payload` are new.
- **cargo vet.** `[[trusted.base16ct]]` exists (tarcieri, 267).
- **deps page.** New: docs/deps/base16ct.md.
- **Manifest.** `[library.base16ct]`; the actions §5 item 1 finds
  (at least `wif-read`, `gpg-paperkey`, `bip85`, `coins-hex`,
  `lightning-node`).
- **Proof.** RFC 4648 §10's base16 vectors in `faraday-crypto`'s tests;
  `tools/vectors/paperkey` (paperkey 1.6 reads what is written);
  `tools/vectors/bip85`; `tools/vectors/entropy`; the tests of every
  crate touched.
- **Size.** About half a day.

### 5.12 The descriptor checksum (batch C10)

- **Files.** `core/faraday-bip/src/descriptor.rs`: `Engine`,
  `INPUT_CHARSET`, `CHECKSUM_CHARSET`, `GENERATOR` go;
  `descriptor_checksum`, `verify_checksum` and `checksum_facts` keep
  their signatures.
- **Crate.** `miniscript`, in the tree and trusted:
  `miniscript::descriptor::checksum::desc_checksum(&str) -> Result<String,
  Error>`, which works on the text, so it serves BIP-392's `sp(..)`
  descriptors too. `descriptor_checksum` is it, its error giving `None`;
  `verify_checksum` splits at the ninth character from the end as today
  and compares `desc_checksum(body)` with the given eight (miniscript's
  own `verify_checksum` is not public).
- **New in the graph.** None.
- **cargo vet, deps page.** No change; `docs/deps/miniscript.md` adds the
  checksum.
- **Manifest.** `descriptor`'s functions keep their names; `libraries`
  already names miniscript.
- **Proof.** Area `addr` (Bitcoin Core's `getdescriptorinfo` and BIP-380's
  code); the wallet fixtures; `tools/vectors/bip392` for `sp()`.
- **Size.** About two hours. Built (batch C10).

### 5.13 The JSON reader (batch C11)

- **Files.** `core/faraday-bip/src/json.rs` (402 lines): the parser goes;
  `Value`, `Error`, `parse`, `quote` and `Value`'s accessors stay;
  `faraday/faraday-intake/src/wallet.rs` (`holds_secret`).
- **Crate.** `serde_json` 1.0.151 (dtolnay), `default-features = false,
  features = ["alloc"]`, moved in `faraday-bip` from dev-dependencies to
  dependencies, with `serde` (`default-features = false, features =
  ["alloc"]`). Whether `faraday-bip` builds with `alloc` alone is not
  checked: the batch checks it first and stops if not.
- **Kept in front.** The 1 MiB limit, checked on the text before it is
  read. A `DeserializeSeed` for Faraday's `Value` (the visitor of §3.4)
  carries the depth and refuses past 32 (serde_json's own limit of 128
  stays behind it), and refuses an object with a key named twice
  (`Error::DuplicateKey`). Objects keep their members in the order
  written. A number becomes `Value::Number` holding the decimal text Rust
  writes for the visitor's `u64`, `i64` or `f64` (so `1e3` reads as
  `1000`); `arbitrary_precision` is not turned on, since it changes
  serde_json for every user in the workspace. Callers ask numbers only
  through `as_u64`; the test in `core/faraday-bip/tests/all/json.rs` that
  expects a number "as written" changes to this rule.
- **`holds_secret`.** An Electrum file's keystore is checked for `xprv`,
  `seed` and `keypairs` by deserializing it into a struct whose fields
  are `Option<serde::de::IgnoredAny>`: no `String` is built for any
  value, so no copy of a secret is made beyond serde_json's scratch
  buffer for a string with escapes. The manifest's `wallet-files` names
  `faraday_intake::wallet::holds_secret` in its functions, and its
  secret text, today "Nothing secret is held", says: "An Electrum file's
  keystore is checked for a private key or seed without its values being
  kept; the file's own bytes are the caller's."
- **New in the graph.** None: serde_json and its dependencies (itoa,
  memchr, serde_core, zmij) are in the lock.
- **cargo vet.** `[[trusted.serde_json]]` exists (dtolnay, 3618).
- **deps page.** New: docs/deps/serde_json.md.
- **Manifest.** `wallet-files`' `adds` changes "JSON is read by
  Faraday's own reader" to "JSON is read by serde_json, behind
  Faraday's refusal of a key named twice and of a nesting deeper than
  32"; `[library.serde_json] author = "David Tolnay"`.
- **Proof.** RFC 8259's examples and the duplicate-key and depth tests in
  `core/faraday-bip/tests/all/json.rs`; area `import`; the wallet
  fixtures; the fuzz target `json`.
- **Size.** About a day.

### 5.14 Adler-32 (batch C12)

- **Files.** `core/faraday-codec/src/png.rs`: `adler32` goes.
- **Crate.** `adler2` 2.0.1, `default-features = false`:
  `adler2::Adler32::new()`, `write_slice`, `checksum`.
- **New in the graph.** None: in the lock through miniz_oxide; the edge
  from `faraday-codec` is new.
- **cargo vet.** Established by imported audits (Google's full audit of
  2.0.0, Mozilla's and Zcash's deltas to 2.0.1).
- **deps page.** New: docs/deps/adler2.md.
- **Proof.** RFC 1950's check values in `faraday-codec`'s tests; every
  PNG the tests write read by the png crate's decoder as today.
- **Size.** Under an hour.

### 5.15 The zero check, the wipes, one Argon2id call (batch C13)

- **The zero check: withdrawn.** `committed` in
  `faraday/faraday-vault/src/lib.rs` is the FVLT container's commitment
  block, which goes with that container when F190's vault replaces it;
  KDBX and the QR vault's message have no such block.
- **Files.** The Argon2 memory's `fill` and `black_box` in
  `core/faraday-backup/src/lib.rs`, `faraday/faraday-vault/src/kdbx/keys.rs`
  and `faraday/faraday-pgp/src/message.rs`, and those of
  `faraday/faraday-fat/src/lib.rs` (a cluster block of file data), become
  `zeroize::Zeroize::zeroize` (argon2's `Block` implements `Zeroize`).
- **One Argon2 call.** `faraday-backup`'s `argon2id`, the KDBX
  container's `argon_run` and the OpenPGP message's `s2k` are the same
  function: reserve the memory, `hash_password_into_with_memory`, wipe.
  It moves to `faraday_crypto::argon2(algorithm, password, salt, params,
  out: &mut Secret<..>)`, which takes `c: &Compute` and says "Costs an
  Argon2 derivation" in its first line, the algorithm Argon2id or, for a
  KeePass file read, Argon2d; all three call it, and their parameters
  and errors stay as they are. The FVLT container's own call
  (`derive`) goes with that container and is not moved; if C13 lands
  first, it is left as it is.
- **New in the graph.** None; `faraday-fat` gains `zeroize` (trusted, in
  the tree).
- **deps page.** `docs/deps/zeroize.md` adds the users.
- **Manifest.** `kdbx`, `vault-kdbx` and `vault-qr-pgp` gain `through`
  argon2 by `core/faraday-crypto`.
- **Proof.** `tools/vectors/kdbx`, `tools/vectors/kdf`,
  `tools/vectors/openpgp` (with its ignored Argon2 S2K test run); area
  `kdbx`, and area `vault` once it judges the KDBX vault and the QR
  vault (`docs/VAULT.md` §10); the tests of `faraday-fat`.
- **Size.** About two hours.

### 5.16 The quiz's seed (batch C14)

- **Files.** `faraday/faraday-core/src/quiz.rs`: `Quiz::seed` is
  `now_ms ^ SHA-256(word indices)[..8]`, feeding a xorshift64* whose
  outputs order the questions and pick the decoy words on screen: 64 bits
  of a hash of the words, in a generator whose output is shown.
- **The fix.** The seed is 8 bytes from the session's generator
  (`faraday-core::fresh`); the words leave it. The xorshift generator
  stays (it orders a quiz; it guards nothing).
- **Manifest.** `words-identity`'s `adds` drops the seed from the words.
- **Proof.** The quiz's tests, changed to the rule: a quiz's order does
  not follow from the words alone.
- **Size.** About an hour.

### 5.17 One copy of each kept piece (batch C15)

The pieces §3 keeps are each in one place:

- **GF(32) and the bech32 alphabet.** `faraday-bip` exports its
  `gf_mul` (§4.4) and the alphabet; the Starter's blank templates
  (`faraday/faraday-starter/src/sheets.rs`: `gf_mul`, `ALPHA`, `val`,
  `up`) use them. The alphabet constants in
  `faraday/faraday-vault/src/codex32.rs`, `faraday/faraday-keys/src/codex32.rs`,
  `faraday/faraday-core/src/codex32/mod.rs`,
  `faraday/faraday-core/src/codex32_correct/mod.rs`,
  `faraday/faraday-core/src/plan/codex32.rs`,
  `core/faraday-entropy/src/booklet.rs`, `core/faraday-bip/src/address.rs`
  and `core/faraday-bip/src/vanity.rs` become `faraday_bip`'s, or
  `bech32::Fe32::from_char` and `to_char` where the text is public.
- **GPT, MBR and the boot sector.** `faraday/shells/starter/src/ids.rs`
  reads the GPT header's fields again after `gpt::read` and a FAT boot
  sector's fields itself (`root_region`, `serial`): `gpt::Table` carries
  the header fields `ids.rs` reads, and faraday-fat gains a public
  `BootSector` (bytes per sector, sectors per cluster, reserved, FATs,
  root entries, FAT size, root cluster, the serial's offset and the
  backup boot sector) that `ids.rs` and `boot_sector_serial` both use.
  `gpt.rs`'s `BOOT_AT`, `BOOT_LEN`, `DATA_AT` and `DATA_LEN` become
  `faraday/faraday-boot/src/layout.rs`'s `BOOT_START`, `BOOT_SECTORS`,
  `DATA_START` and `DATA_SECTORS`.
- **CBOR.** `core/faraday-codec/src/ur/`'s `cbor_bytes`,
  `cbor_byte_string` and the fountain part's CBOR, and
  `faraday/faraday-qr/src/cbor.rs`, become one module in
  `faraday-codec`, which `faraday-qr` uses.
- **PDF text.** `faraday/faraday-core/src/pdf.rs`'s `pdf_text` and the
  Starter's narrower copy in `sheets.rs` become one, `faraday-core`'s.
- **HKDF.** `faraday/faraday-pgp/src/message.rs`'s `hkdf`, an extract
  through `faraday_crypto::hmac_sha256` before `hkdf_sha256_expand`,
  becomes `faraday_crypto::hkdf_sha256(salt, ikm, info, out: &mut
  Secret<..>)` beside `hkdf_sha256_expand`, the pseudorandom key in a
  `Secret`; the message calls it with the salts it uses today. The FVLT
  container's own `hkdf_sha256` (`faraday/faraday-vault/src/lib.rs`)
  goes with that container when F190's vault replaces it and is not
  moved. RFC 5869's cases move to
  `core/faraday-crypto/tests/primitives.rs`.

**Proof.** The tests of every crate touched; the Starter's stick and
card tests (`tests/all/sticks.rs` in its shell); `tools/vectors/ur`,
`tools/vectors/openpgp`; areas `ur` and `sign`, and `vault` once it
judges the QR vault (`docs/VAULT.md` §10); the blank templates'
snapshots unchanged.

**Size.** About a day. It is code more than one app uses: the device
app's and the Starter's builds both follow it.

## 6. Check again later

Each item here is kept written here (§3) until its condition holds; the
switch is then a batch of its own, with a decision only where §1 needs
one.

### 6.1 MuSig2, after bitcoin 0.33

libsecp256k1 has had BIP-327 since 0.6.0 (2024-11-04). rust-secp256k1
wraps it as `secp256k1::musig` from 0.32.0-beta.2 (2025-10-30), which
bitcoin 0.33.0-beta (2026-02-23) depends on. The tree's bitcoin 0.32
uses secp256k1 0.29, which does not. No miniscript release builds on
bitcoin 0.33: 13.2.0 (2026-10-17) and 12.3.8 (2026-10-27) are both on
bitcoin 0.32, and none has `musig()`. Faraday does not carry two copies
of libsecp256k1 (owner, 2026-10-30).

**What would change it.** A stable bitcoin 0.33, and a miniscript that
builds on it. Then key aggregation, tweaks, nonce generation, partial
signing, partial verification and aggregation move to libsecp256k1.
DeterministicSign (BIP-327, used in `core/faraday-psbt/src/sign.rs`)
stays written here, as libsecp256k1 has none (`none-established`);
BIP-328 and BIP-390 stay until a miniscript has `musig()`. About three
to four days after the bitcoin move.

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

**What would change it.** A release that is `no_std`, holds the seed
in a type that is neither `Clone` nor `Debug` and is wiped, and computes
without a branch on a symbol (§4.4). Faraday's worksheets, fresh sets
and error correction would stay in front.

### 6.7 HKDF's upstream fix

hkdf 0.13.0 leaves the pseudorandom key and each output block T(i)
unwiped (§5.2). A change that holds both in `Zeroizing` (the PRK out of
`GenericHkdfExtract::finalize` and `GenericHkdf::new`, `prev` in
`expand_multi_info`) is proposed to RustCrypto/KDFs (owner, 2026-10-31).

The same class of upstream issue, on the stack, where §4.9's
`WipeOnFree` does not reach: hmac 0.13.0's key setup (`get_der_key`'s
block and hash, `new_from_slice`'s key XOR ipad and opad) and pbkdf2
0.13.0's U_i are stack arrays never wiped, and hmac's `zeroize` feature
reaches only the hash cores. They hold the seed sentence's key block at
each BIP-39 seed. They are checked again with HKDF's; nothing changes
in Faraday until a release wipes them.

**What would change it.** An hkdf release with that change. Then
`faraday_crypto::hkdf_sha256` and `hkdf_sha256_expand` wrap it, with
`hkdf = { version = "<that release>", default-features = false }` (it
adds `hkdf` alone; its one dependency, hmac, is in the tree),
`[[trusted.hkdf]]` with `trusted-publisher = "github:RustCrypto/KDFs"`,
`start = "2026-03-30"` (0.13.0, the first release by trusted
publishing), `hkdf` in `PRIMITIVE_CRATES`, docs/deps/hkdf.md, and
`through` hkdf by `core/faraday-crypto` on `vault-qr-pgp` and the actions
that reach `hkdf_sha256_expand` (signing's aux randomness, the session's
blinds). RFC 5869's cases, `tools/vectors/openpgp` and areas `sign` and
`vault` prove the bytes. About half a day.

### 6.8 crc32fast's review

crc32fast 1.5.1 is in the lock (through png and flate2) and `no_std`;
Google's full audit covers 1.4.2 and Mozilla's delta 1.3.2 to 1.4.2,
and the step to 1.5.1 is only Faraday's own `ai-says-safe-to-deploy`
record, so it is not established at that version.

**What would change it.** An imported audit covering the version the
lock pins. Then `faraday_crypto::crc32` wraps it, adding no crate.

### 6.9 A constant-time bip39 crate

The bip39 crate (3.0.0, 2026-09-17) finds words by `find_word` and packs
entropy with a branch per bit (§4.5).

**What would change it.** A release whose word lookup and packing do not
follow the words. Then the BIP-39 core moves to it, with a trust entry
for the publisher of that release (3.0.0 is tnull's, user-id 204543;
2.2.2 elsirion's, 199480; 2.2.0 and earlier stevenroose's, 28012), F71
amended, and Faraday's entry and display, the seed's sentence in a
`Secret` buffer and its PBKDF2 through `faraday-crypto` stay in front.
The comparison test with the crate then compares the crate with itself
and is removed.

## 7. Documentation fixes

All are made in batch B1, ahead of the switches, but §7.2's MuSig2 text,
which is right only after A1.

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
bip39 crate gives `word_indices`, `checksum` and `to_entropy_array`. The
row is rewritten to §3's reasons. BIP-39: "the bip39 crate's word lookup
takes a time that follows the words, so Faraday's own core, which looks
each word up in constant time, is kept, and the crate is what the tests
compare it with" (true once A5 lands; before it: "the bip39 crate's word
lookup takes a time that follows the words; Faraday's is being made
constant time, and the crate is what the tests compare it with").
MuSig2: "libsecp256k1's MuSig2 is in rust-secp256k1 0.32 and later;
bitcoin 0.32, which Faraday uses, is on 0.29, and no miniscript release
builds on bitcoin 0.33 yet".

### 7.3 AUDIT §1: what `faraday-crypto` holds

`faraday-crypto`'s row names neither `crc32` nor `WipeOnFree`, both in
the crate. It names them: "CRC-32 (UR, PNG, the Starter's GPT)" and
"`WipeOnFree`, a global allocator that wipes what it frees, installed by
no binary (F181)".

### 7.4 The QR encoder's module doc

`core/faraday-codec/src/qr.rs`'s module doc says "no published QR
encoder is `no_std`". qrcodegen-no-heap 1.8.1 (Nayuki) is `no_std`; it is
not established (no trust entry, no imported audit), and it does not
wipe. The sentence becomes: "No QR encoder comes from a publisher
Faraday's dependency review trusts; this port of Nayuki's `qrcodegen`
wipes a SeedQR's buffers."

### 7.5 "No library implements"

The manifest's notes for BIP-374 and BIP-375 and for SeedQR said "No
library implements"; btclib implements BIP-374 and BIP-375 in Python,
and SeedSigner (Python) and Sparrow's drongo (Java) implement SeedQR.
§3.2 says "No Rust library implements"; the manifest's `written_here`
notes and the website's reason lines follow at D1.

### 7.6 The storage command's comment

`faraday/faraday-core/src/storage_api.rs`'s comment on
`StorageCommand::SaveBoxes` says the Inbox and Outbox "hold nothing
secret". The Outbox keeps the secret files let out past the secret
sheet across locks, and the Inbox keeps a words file until a lock. The
comment says so: "They may hold a secret file let out past the secret
sheet, or a words file until a lock." The
behaviour is by design and is not changed here.

### 7.7 docs/deps/rsa.md: Marvin

`docs/deps/rsa.md` says the 0.9 line "carries RUSTSEC-2023-0071, the
Marvin timing side channel, which 0.10's `crypto-bigint` arithmetic
closes". The advisory lists 0.10.0-rc.18 as still affected, with no
patched version, and rsa 0.10.0-rc.18's own README says the crate is
vulnerable to the Marvin Attack. The sentence says what `deny.toml`
says: 0.9 and 0.10 both carry the advisory, which is not reachable in
Faraday, because Faraday never decrypts with RSA and signs with
blinding; and the page names §4.10's limit, that key generation's prime
search is not constant time.

### 7.8 Secret handling that claims more than the code does

A manifest line or a module doc that says a secret is wiped, or that no
secret is held, where the code leaves a copy, says what the code does,
and the code is not changed for it (§4.8). The review of 2026-10-31
found: `wif-read`'s "The decoded buffer is zeroized before the reader
returns" (base58ck's own scratch and result vectors are freed unwiped);
`wallet-files`' "no secret is held" (`faraday_intake::wallet::holds_secret`
parses an Electrum file holding an `xprv` or a seed into plain strings);
and `totp`, `bip85`, `gpg-paperkey` and `codex32`, whose lines name
wiping or constant time that §4.4 to §4.6 or this section correct.
Each line gains what is left, plainly: "<what> is freed unwiped, until
the next lock (§4.8)."

### 7.9 What wiping does not reach, said where secrets are described

`docs/KEYS.md` §2 says what Faraday's types wipe, and the manifest's
Secret handling lines say what each action wipes; neither says what
wiping cannot reach. KEYS §2 gains §4.9's five stated limits (a secret
in a plain type while live; a copy in freed heap until the process ends; the stack and registers, `Copy` types and
moved-from slots; memory other code allocates itself; swap, sleep
images and crash dumps, none on the stick and the Pi, the desktop
operating system's for Faraday Demo and the Starter), and the `hold`
action's `secrets.wiped` names them in one sentence: "Not reached: a
secret while it is live in a plain type, the stack and registers,
memory other code allocates, a copy in freed heap until the process
ends (the next lock on the device), and anything outside the process
(none on the device; the computer's own on the desktop)."

## 8. Order of work

Each batch is one commit and one check: the module touched while
working, `just crosscheck` where a cross-check area's code changed,
`just crypto-review` for each action whose fingerprint changed, then
`just` at the end. Each batch edits this document's `**Status:**` and
`**Open:**` lines to name what is left, and `docs/README.md`'s line with
them. The batch that builds the last open item moves §4, §5, §8 and §9.7 to
docs/history/ESTABLISHED-LOG.md and makes the document a Reference
(CLAUDE.md); the work kept for later (§6) is then its own Spec or the
Reference's list of conditions.

| Batch | What | Sections | Size |
|---|---|---|---|
| A0 | The inventory's name matching; the actions it moves re-stamped | §2.5 | Built |
| A1 | Scalar arithmetic on secrets through libsecp256k1, from the written batch; the re-stamps it lacks; with X8 | §4.1, §5.4, §9.7 | Built, with X8 |
| A2 | AEZ's AES round from `aes` | §4.2, §5.3 | Built |
| A3 | SLIP-39's GF(256) and checksum without a branch | §4.3 | Built |
| A4 | codex32's GF(32), checksum and alphabet without a branch; one `gf_mul`; `fresh_set` sized; with X10 | §4.4, §9.7 | About half a day, and X10's |
| A5 | Word lookups, `set11`, aezeed's `pack` and the sentence's gathering in constant time; with X11 | §4.5, §9.7 | About 1 day, and X11's |
| A6 | Secret comparisons through subtle; paperkey's CRC-24 masked | §4.6 | About 2 hours |
| A7 | BIP-352's label tweak times G through a secret key, `t_k` as libsecp256k1's own module computes it; the key's own randomized contexts; with X9 | §4.7, §9.7 | About half a day, and X9's |
| B1 | AUDIT §1's rows, the QR encoder's module doc, the storage command's comment, `docs/deps/rsa.md`'s Marvin sentence, the Secret handling lines that claim more than the code does (§7.8), KEYS §2 and the `hold` action saying what wiping does not reach | §7 | Built |
| C1 | BLAKE2b from `blake2`'s `Blake2bVarCore` | §5.1 | Built |
| C2 | (HKDF: not a switch; kept, §5.2, §6.7) | §5.2 | None |
| C3 | The sealing's counter nonce removed | §5.9 | Built |
| C4 | The RSA RNG adapter from `ChaCha20Rng`; tests re-pinned | §5.6 | Built |
| C5 | X.509 from `x509-cert`, DER from `der`; trust entries for x509-cert and der_derive, flagset's exemption; with X2 | §5.7, §9.7 | Built |
| C6 | The single-key and multisig finalizer from miniscript; with X3 | §5.8, §9.7 | Built, with X3 |
| C7 | (The BIP-39 core: not a switch; kept, §5.5, §6.9) | §5.5 | None |
| C8 | Base64 from `base64ct`, four copies | §5.10 | Built |
| C9 | Hex from `base16ct`, one module; with X4 | §5.11, §9.7 | About half a day, and X4's |
| C10 | The descriptor checksum from miniscript | §5.12 | Built |
| C11 | JSON from `serde_json` behind Faraday's visitor; `holds_secret` keeps no value; with X5 | §5.13, §9.7 | Built, with X5 |
| C12 | Adler-32 from `adler2`; with X6 | §5.14, §9.7 | Built, with X6 |
| C13 | The wipes through zeroize, one Argon2 call; with X7 (the zero check withdrawn: it was the FVLT container's, which F190's vault replaces) | §5.15, §9.7 | About 2 hours, and X7's |
| C14 | The quiz's seed from the session's generator | §5.16 | Built |
| C15 | One copy of each kept piece; with X12 | §5.17, §9.7 | About 1 day, and X12's |
| D1 | Schema 5: `written_here` as entries with `why`, `note`, `decision`; `[[not_established]]` for lzma-rust2 and rqrr; getrandom's trust entry; `check.py` (§1.2's three ways, the version checked) and `gen.py`; AUDIT §3 grouped; STANDARDS §10 generated and §0's bullet; the counts; `crypto.established` made active; the manifest's entries filled as §3 | §1.2, §1.4, §2, §3 | 1 to 1.5 days |
| E1 | The FAT32 formatter in faraday-fat, its `written_here` entry and the count raised under F180 | §3.2 | Built, with the Windows stick work |
| X1 | The cross-check's judges required in `just crosscheck` (§9.2, rule 4) | §9.7 | 1 to 1.5 days |
| X2 to X12 | The cross-check changes that land with C5, C6, C9, C11, C12, C13, A1, A7, A4, A5 and C15 | §9.7 | About 10.5 to 12.5 days in all, inside those commits |
| X13 to X30 | The cross-check changes standing alone (X25 withdrawn) | §9.7 | About 21.5 to 27.5 days in all |

The order (owner, 2026-10-31 and 2026-11-02; F191): A0, A1, B1, X1,
C1 to C14, then A4 to A7, then C15, then D1. A1 lands second because it
was written before this order was set. A0 comes first, because it changes
fingerprints every later batch re-stamps. C1 to C14 are independent of
each other and may land in any order; A4 to A7 come after them,
and C15 after A4 (it moves the masked `gf_mul`). Each changes the
manifest's `written_here` string of its action as it lands, and each
that names an X batch lands with it, in one commit and one passing
`just crosscheck` (§9.2, rule 6). X1 comes after B1 and before C1, so
that the judges the tied batches add are required from the first. The
X batches standing alone may land at any point after X1, in §9.7's
order. D1 comes after the switches so that the entries it writes are
§3's; before D1 lands, the website's reader of the manifest is changed
in its own repository. §6's items come
as their conditions hold. In all, A0 to D1 take about twelve to sixteen
working days, and the cross-check's
batches about 33.5 to 41.5 more, of which about 11.5 to 14 come with X1 and
the switches.

## 9. The cross-check's outside sources

Decided (owner, 2026-10-31; F191, F192). This section says what the
cross-check (F79, `docs/CROSSCHECK.md` §10, `tools/crosscheck/README.md`)
compares each action with once the switches of §4 and §5 are made: the
rules that choose (§9.2), the places where a check would compare a
library with itself and how each is cleared (§9.3), each action's
judges before and after (§9.4), what is not taken and why (§9.5), the
tools and packages it adds (§9.6), the batches (§9.7) and one choice
withdrawn (§9.8). A library the app uses cannot be its own cross-check, so a
piece that moves to a library needs a judge other than that library;
the list is made now that the switches are fixed (F180 point 8).

The facts are from a review of main on 2026-10-31 that read every
action's manifest entry, every area's docstring, the vendored
references (btclib's wheels opened, `tools/reference/lnd/`'s and
`tools/reference/btcd-bip322/`'s vendored modules listed) and the
outside candidates' sources. What could not be read from a source is
marked "not checked"; the batch that builds on it checks it first and,
if it does not hold, keeps the action's present judge and says so.

Before this work, 66 of the manifest's 94 actions name a cross-check
area: 30 `outside`, 27 `outside-and-own`, 9 `own`; 28 name none, of
which the six GnuPG actions, the three Secure Boot actions and
`starter-signed` are judged by outside programs in required tests.
Since then three vault actions have been added with no area
(`vault-contents`, `vault-kdbx`, `vault-qr-pgp`, for F189's and F190's
vault): 66 of 97 name an area, 31 none.

### 9.1 The words

- **Others' code**: an implementation Faraday's authors did not write
  (F160 point 1). btclib's and EntropyLab's authors are not Faraday's
  authors (owner, 2026-10-31), so their code is others' code.
- **Faraday's second version**: a part an `area_*.py` writes itself,
  from the specification. It catches a slip between two transcriptions,
  not a misreading both share (F160, F179).
- **Lineage**: code and the code it was taken from. A port is the same
  lineage as its original (§9.2, rule 3). Two implementations by
  authors who share a group but no code (libsecp256k1 and secp256k1lab,
  HWI and Core) are separate lineages, as F160 point 7 has them.
- **Collision**: a check that compares a library with itself: the same
  crate, a binding to or another build of the same C library, or
  another version of either, for the work they share (rule 2).

### 9.2 The rules

1. **Faraday's second version is retired where others' code of another
   lineage covers the same inputs, and kept where none does** (amends
   F160 point 8). "Covers" means the outside judge is asked every case
   the second version is asked, and makes the same refusals; where it
   covers some, the rest stays, named in the action's `ours`. A part
   whose only outside judge shares its lineage with the app (Core's
   libsecp256k1, a btclib module that says it is a port of Core) keeps
   its second version.
2. **Another version of the same library is a collision for the curve
   maths, not for consensus and script logic.** Bitcoin Core's
   libsecp256k1 (Core's own copy; the app's is 0.4.1, in secp256k1-sys
   0.10.1) is the app's curve library: where Core signs,
   verifies or derives a key, a pure-curve judge of another lineage
   sits beside it (btclib on its own curve, python-ecdsa, the BIPs'
   references over secp256k1lab). Core still judges consensus, script
   and descriptor logic, PSBT handling and the encodings, which are its
   own C++. This narrows F160 point 7's acceptance of libsecp256k1 as
   shared.
3. **A port is the same lineage as its original.** Faraday's QR
   encoder, a port of Nayuki's, compared with Nayuki's catches a slip
   in the port, not a misreading of ISO/IEC 18004; a second encoder of
   another lineage sits beside it. btclib's modules that say they are
   ports of Core v31.1 (`script/solver.py`, `script/spendability.py`)
   are Core's lineage for those rules. A reader built on another
   (rqrr on quirc) is that reader's lineage.
4. **Every judge `just crosscheck` names is required there.** A missing
   tool fails the run and names it; its version goes into the stamp. In
   `just`, the tests that run an outside program and skip without it
   (mtools and dosfstools in `faraday/faraday-fat/tests/judged.rs`,
   `sfdisk` and dosfstools in the Starter's `tests/all/ids.rs`, and the
   ones this section adds) still skip with a note; `just crosscheck`
   runs those test files with `FARADAY_JUDGES_REQUIRED=1`, under which
   the skip is a failure. The tests that need GnuPG 2.4, paperkey 1.6,
   OpenSSL and sbverify stay required in `just`, as today. Each tool is
   a prerequisite in `docs/DEVELOPING.md` §1, added by the commit that
   first needs it.
5. **Hex after C9 is checked against outside code.** Once `RawHex`
   decodes through base16ct, the `entropy` area's comparison with
   CPython's `bytes.fromhex` is others' code against a library, and
   `coins-hex` says so; the flips' packing and the commitments stay
   Faraday's second version.
6. **A switch lands with its cross-check change** (F191): in the same
   commit and the same passing `just crosscheck`, so no commit has a
   piece judged by the library it moved to, or an action whose judge
   was dropped and not replaced. The switch's row in §8 names its X
   batch.

The other choices of 2026-10-31 decide single rows and are in §9.4 and
§9.5: Go reaches a released btcd, pinned, for new uses, and pull
request 2521's copies stay for BIP-322 alone; Rust programs outside the
workspace are allowed as references, pinned by commit and SHA-256, never
shipped, outside `cargo vet`, as volvelle's harness is; bip39_slip39 is
vendored; embit is taken in full only with Specter's importer, not now;
btclib's change rule comes first and Core's wallet second; aezeed gets
a run with the software AES round; Node, a JVM, Swift and .NET are not
taken now; libsecp256k1's silent payments module is not a reference.
The vault waited for an outside reviewer (F192 point 7); since F189 and
F190 it is two published formats that others' code reads, judged as
F190 point 11 and `docs/VAULT.md` §10 say (§9.4), and §9.8 is
withdrawn.

### 9.3 Collisions, and how each is cleared

| Where | What is compared with itself | Cleared by | Batch |
|---|---|---|---|
| `sign-ecdsa`, `sign-schnorr`, `message-sign`, `message-verify` | Core's signatures and verification are libsecp256k1's, as the app's are | python-ecdsa and btclib's own curve, already beside Core (`tools/crosscheck/lineage.py`); the manifest names Core as sharing the curve maths | X13 (built) |
| `sweep-sign`, `wif-read` | Core's signatures and the WIF's public key are libsecp256k1's | python-ecdsa's first-nonce signature byte for byte and btclib's signing and finalizing for sweeps; btclib's WIF decoding and public key | X13 (built) |
| `vanity` | Core's `deriveaddresses` derives each candidate's child keys with libsecp256k1 | the child keys by btclib on its own curve; Core judges the script and the address text only | X13 (built) |
| `sign-check` (new area) | Core's `finalizepsbt` checks signatures with libsecp256k1 | python-ecdsa and btclib verify every signature, partial ones too; Core judges script and consensus on complete inputs | X19 (built) |
| `sign-musig`, after A1 | the Core round trip's partial signatures share libsecp256k1's scalar code with A1's | btclib's MuSig2 as a full reference beside BIP-327's; the round trip stays as a judge of BIP-373 and the descriptor, named as sharing the curve | X8 |
| `musig-key` | Core's `musig()` aggregation is libsecp256k1's musig module | BIP-327's reference and btclib's key aggregation judge the maths; Core judges BIP-390's descriptors and addresses | X8 |
| `qr`, `codex32qr`, `seedqr-make` | the app's encoder is a port of Nayuki's, compared with Nayuki's (rule 3) | segno, another encoder lineage (X21, built); zbar and zxing-cpp decode every picture (X21, built) | X21 |
| `tools-hash` | `base58check_round_trips_through_bitcoins_own_encoder` in `core/faraday-codec/src/encodings.rs` compares `bitcoin::base58::encode_check` with itself | that assertion replaced by a published Base58Check vector; the new `hash` area judges Base58Check with Core's framework | X27 |
| `aezeed` | not a library: on a builder with AES-NI both sides run the processor's round, so the fixsliced round the Pi runs is never compared | a second pass with the aes crate's software round and LND's AEZ built without assembly (built: X29) | X29 |
| btclib's ports of Core | `script/solver.py` and `script/spendability.py` are Core's logic | not counted as a second lineage beside Core; where only they cover a rule, Faraday's second version stays (rule 1) | X15, X18 |

No switch of §5 makes an existing judge collide: C1, C5, C8, C9, C11,
C12 and C13 move pieces onto RustCrypto crates, `serde_json`, `adler2`
and miniscript, and their judges are Go's x/crypto, OpenSSL, GnuPG,
Core's C++, CPython's `base64`, `binascii`, `json` and `zlib`, and
pykeepass with its argon2-cffi (and, once F190's vault is judged,
`keepassxc-cli` and Sequoia's `sq`). A1 and A7 put more of the app on
libsecp256k1; the BIPs' references run over secp256k1lab and btclib on
its own curve, neither libsecp256k1, so they stay valid.

### 9.4 Each action

Faraday's side is the code after §4 and §5. "Kept" is the judges in use
today; "Added" names the batch; "Second version" is what of Faraday's
second version is retired or kept under rule 1; "Class" is the
`independence` class after the batch (none: no area, judged in tests or
not at all).

| Action | Faraday's side | Kept | Added | Second version | Class |
|---|---|---|---|---|---|
| words-read, words-make, seed | Own BIP-39 core, constant time (A5); pbkdf2, sha2, unicode-normalization | python-mnemonic | bip39_slip39 as a second reader (X11) | none | outside |
| slip39 | Own; GF(256) and RS1024 without a branch (built) | python-shamir-mnemonic | bip39_slip39 (X11) | none | outside |
| bip85 | bitcoin and Faraday's glue; base64ct (C8), base16ct (C9) | btclib-wallet's `bip85`, ethankosakovsky/bip85 | btclib's `bip85` run as written over bip39_slip39, python-mnemonic's stand-in removed (X11) | none | outside |
| master | bitcoin and libsecp256k1; the strict decoder Faraday's | btclib-wallet's `bip32` | btcd's `hdkeychain` from btcd v0.27.0 (X14, built) | kept: `hdkeychain` does not refuse what Faraday refuses. It reads a key of any version, a version of the other kind than the key's data, and a depth-0 key with a parent fingerprint or an index; those three rules stay Faraday's second version, and `hdkeychain` is asked of every other refusal (the checksum, the length, a scalar of 0 or past n, a public key's prefix or a point off the curve) and of every valid key | outside-and-own |
| derive, xprv-shown | bitcoin and libsecp256k1 | btclib-wallet's `bip32` | `hdkeychain` from btcd v0.27.0 (X14, built) | none | outside |
| dice | Own specification; sha2 | Coldcard's `rolls.py`, `rolls12.py` | SeedSigner's `mnemonic_generation.py` over embit's BIP-39: direct selection's last word at every length, the hashed procedure at 50 and 99 rolls (X22) | kept: six-as-zero, the cuts to 20, 24 and 28 bytes, the counts and refused faces, which no outside code has without Node | outside-and-own |
| coins-hex | Own specification; hex through base16ct (C9) | Faraday's second version | CPython's `bytes.fromhex` named as the hex judge (X4, rule 5) | kept for the flips and the commitments | outside-and-own |
| vanity | bitcoin and libsecp256k1 | python-mnemonic, btclib, Core | each candidate's child keys by btclib; Core for the script and address text only (X13, built) | kept: the counter, the prefix rules, the expected work, the search loop | outside-and-own |
| aezeed | Own; aes (built), blake2 (C1) | LND's `aezeed` | a pass with the software AES round on both sides: the aes crate's `aes_backend="soft"` against Yawning Angel's AEZ built with the tags `noasm` and `purego` (X29, built) | none | outside |
| silent-receive | Own; secret scalars through libsecp256k1 (A1), the label tweak through `ecmult_gen` (A7) | BIP-352's reference over secp256k1lab | btclib-wallet's `silent_payments.py` (X9) | none | outside |
| silent-send | Own; A1, A7 | BIP-374's and BIP-375's references | btclib's `dleq.py`, BIP-375 roles and BIP-352 (X9) | none | outside |
| musig-key | Own until bitcoin 0.33 (§6.1); point operations libsecp256k1's | BIP-327's reference; btclib verifies the aggregate | btclib's key aggregation and tweaks as a full reference; Core's `musig()` descriptors through `deriveaddresses` (X8) | none | outside |
| sign-musig | Own; secret scalars through libsecp256k1 (A1) | BIP-327's reference; btclib verifies; Core's round trip (`other`) | btclib's MuSig2 signing and BIP-373's PSBT roles (X8) | none | outside |
| codex32, seed-shown | Own; GF(32), checksum and alphabet without a branch (A4) | python-codex32 | rust-codex32 at a pinned commit, a program outside the workspace (X10) | the threshold rule (0 or 2 to 9) and threshold 0's share index `s` retired to rust-codex32; the seed's lengths and the share order kept | outside-and-own |
| codex32-make | Own; A4 | python-codex32 | volvelle-website's `checksum_worksheet.rs` run as a program over every corpus worksheet of 48 and 74 characters; rust-codex32's `interpolate_at` for a fresh set's derived shares (X10) | the rows at 48 and 74 characters retired; at 127 kept | outside-and-own |
| qr, codex32qr | Own encoder (Nayuki's, ported) and PNG writer, Adler-32 from adler2 (C12); rqrr reads | Nayuki's `qrcodegen.py` (same lineage, rule 3) | segno as a second encoder, module for module at a given mask and at its own mask choice; segno's and Nayuki's codes rendered and read by rqrr; zbar and zxing-cpp decode every picture (X21, built: area `seedqr`; segno 1.6 module for module at Faraday's mask and at another, and at its own choice as Faraday's code or the same data under the mask it reports, its zero pad codeword where the standard adds none modelled in Nayuki's code and counted; `qr_decode.py` for the Codex32QR pictures Faraday writes, the profile's 22 pictures, and each key's SeedQR and CompactSeedQR picture, payloads compared as bytes) | none | outside |
| seedqr-make | Own | SeedSigner, Nayuki | each PNG read by Pillow and zlib (X6); segno (X21, built, with the QR rows above); zbar and zxing-cpp decode each picture Faraday writes of a key's SeedQR and CompactSeedQR, payloads compared as bytes (X21, built) | kept: CompactSeedQR at 15, 18 and 21 words, which no outside code defines | outside-and-own |
| seedqr-read | Own; the comparison through subtle (A6) | SeedSigner's `DecodeQR` | zbar and zxing-cpp decode the pictures; Faraday's reader takes what they return and gives the key's words (X21, built) | kept: Faraday's reader rule, 15, 18 and 21 words | outside-and-own |
| stick-write, outbox | Own text lines, FAT writer and PNG writer | FAT read by mtools and dosfstools in tests (required in `just crosscheck`, X1) | each PNG read by Pillow and zlib (X6); each picture decoded by zbar and zxing-cpp and its payload compared with the key's words (X21, built: the stick's labelled picture of the key's SeedQR and CompactSeedQR and of a codex32 share), in area `seedqr` | none | outside |
| ur | Own; one CRC-32 and one CBOR (C15) | foundation-ur-py | `zlib.crc32` against the one CRC-32; cbor2 for the registry types' and the parts' CBOR (X12) | none | outside |
| bbqr | Own; miniz_oxide; hex through base16ct (C9) | Coinkite's BBQr Python, SeedSigner's numbered-part reader | krux's `qr.py` writing numbered parts; Coinkite's test files as vectors (X23) | the writing of numbered parts retired to krux; the refusals and the capacity shim kept | outside-and-own |
| hashes-macs | sha2, hmac, pbkdf2; HKDF-Expand Faraday's, one copy (C15) | python-mnemonic, `hashlib` | `hashlib` and `hmac` over random HMAC-SHA256 and PBKDF2-HMAC-SHA256 (X27); PyCA's `HKDFExpand` over random keys, info and lengths up to 255 blocks (X12) | none | outside |
| scrypt | Own (§3.3) | LND, over Go's x/crypto | `hashlib.scrypt` over random N, r, p and lengths (X27) | none | outside |
| tools-hash, file-hash | bitcoin (Base58, bech32, RIPEMD-160), sha2; hex through base16ct (C9) | none (published vectors in tests) | a new area `hash`: Core's framework's `ripemd160.py`, `address.py` and `segwit_addr.py`, `hashlib`, `bytes.fromhex`, SHA-256 of random files across block boundaries; the self-comparing Base58Check assertion replaced (X27) | none | outside |
| address, descriptor | miniscript (its checksum, C10), bitcoin | Core, BIP-380's checksum code, python-bip380, btclib's keys | btclib-wallet's descriptors and miniscript: the script at an index, the parse, the type, the round trip (X18, built) | none | outside |
| recovery | As descriptor | As descriptor | As descriptor, and btclib's satisfaction of each path under its lock times (X18, built) | none | outside |
| tapmulti | bitcoin, miniscript | Core's framework for the trees | btclib's `check_output_pubkey` (X15, built); btcd v0.27.0's `ParseControlBlock` and `VerifyTaprootLeafCommitment` (X14, built); neither hashes a leaf of version 0x50 differently from any other, so that rule stays the area's own | retired but the rule that a leaf version is not 0x50: BIP-341's script-path check written in the area is gone, and Core's framework is no longer asked about a control block | outside-and-own |
| psbt-read | bitcoin, base64ct (C8); the inspection Faraday's | Core, HWI | btclib-wallet's `Psbt.parse`, with its validity check (X15, built) | retired: the 65-byte signature with sighash byte 0 is refused by btclib, checked; btclib reads some PSBTs Core and Faraday refuse (a key with key data on a field that takes none), so it is not asked to refuse what Core refuses | outside |
| psbt-v2 | Own (§6.4) | HWI, Core for both version 0 forms | btclib-wallet's version 2 reader and its conversions to and from version 0 (X15, built) | retired: the refusals of bytes after the last map, of a stream ending inside a map (checked: btclib refuses it) and of a 65-byte taproot signature with sighash byte 0; kept: the bridge's pair choice and the version 0 PSBT where HWI repairs a short value | outside-and-own |
| sighash-review | bitcoin | Core's framework | btclib's `sig_hash` (`legacy`, `segwit_v0`, `taproot`), every hash beside the framework's (X15, built) | retired: BIP-341's refusals (SIGHASH_SINGLE without an output, a hash type it does not list) are btclib's, and the framework, which hashes those anyway, is not asked | outside |
| tx-decode | bitcoin; hex through base16ct (C9) | Core's framework | btclib's `Tx.parse`; Core's `decoderawtransaction` on the regtest node, with the known difference where bytes also read as a legacy transaction of no inputs (X15, built) | retired but the refusal of a transaction with no inputs, which neither judge makes; the framework's reading and the exact-serialisation rule written for it are gone | outside-and-own |
| psbt-review | bitcoin, miniscript; the change rule Faraday's | Core's `decodepsbt`, `analyzepsbt`, `deriveaddresses` | btclib's `update_psbt_output`, `index_of` and descriptor wallet (X16); Core's watch-only descriptor wallet, `getaddressinfo` and `walletprocesspsbt` (built, X17) | the change rule retired (built, X16); the registered-wallet rule kept | outside-and-own |
| wallet-files | bitcoin; serde_json behind Faraday's visitor (C11); base64ct (C8) | Core, btclib | CPython's `json` reading every file the area writes and refusing a duplicate key and a depth over 32 (X5); every real wallet file the wallets' published test suites hold (X30, built: 93 files of Sparrow, BlueWallet, Coldcard, Caravan and Liana, each read by Faraday and by the area's reading of its format, judged by Core, or refused by both) | kept for the formats no outside reader covers; the refusing hook is Faraday's second version over CPython's parser | outside-and-own |
| finalize | miniscript (C6), and Faraday's choice of the smallest witness | Core's `finalizepsbt`, `testmempoolaccept` | btclib's `psbt.finalize` with its miniscript solver, which also chooses among satisfactions (X3, built; the leaf of a taproot input and a plain multisig's signatures stay Core's to judge) | kept: the taproot `pkh` leaf left off, the fields BIP-174 keeps | outside-and-own |
| sign-ecdsa, sign-schnorr | libsecp256k1, bitcoin; Faraday's refusals | Core, python-ecdsa, btclib, BIP-340's reference | Core named as sharing the curve maths (X13, built) | kept: F136's and F141's refusals, the inputs a wallet's rule leaves unsigned | outside-and-own |
| message-sign, message-verify | libsecp256k1; BIP-322 own (§3.2); base64ct (C8) | Core, btclib, btcd's `bip322`, python-ecdsa | Core named as sharing the curve maths (X13, built) | none | outside |
| sweep-sign | libsecp256k1, bitcoin, miniscript's finalizer (C6) | Core, BIP-143's example | python-ecdsa's first-nonce signature byte for byte; btclib's signing and finalizing (X13, built) | none | outside |
| wif-read | libsecp256k1, bitcoin; base16ct (C9) | Core | btclib's WIF decoding and public key (X13, built) | none | outside |
| sign-check | libsecp256k1 (verification), bitcoin (sighash) | none | a new area (X19, built): python-ecdsa and btclib verify every signature on a PSBT, partial or in a finalized witness, over btclib's sighashes; btclib's script engine runs the complete inputs; Core's `finalizepsbt` and `testmempoolaccept` judge them and the transaction. Core's rules, as the area found them: a partial input names one sighash type, so its PSBT sighash-type field is written when its signatures share a non-ALL type, and an input with mixed types is not asked of Core; an input that arrives final is taken as it is, so its signatures are judged by `testmempoolaccept` on the extracted transaction, not by `finalizepsbt`'s `complete`. Known: a partial signature that is not well formed is refused by Faraday's reader where Core reads it and leaves the input unfinalized; and a bug in Faraday held until it is fixed: a finalized taproot signature of 65 bytes ending in 0x00 is called valid where BIP 341 and Core call it malformed | the report of a reused r or R, written from the definition (two signatures that verify, under one key, over different messages, with one nonce) | outside-and-own |
| compile | miniscript's policy compiler | none | a new area (X20, built): Core's `getdescriptorinfo` and `deriveaddresses` take the output; python-bip380 and btclib parse it; the policy and the parsed miniscript compared as boolean formulas over the keys and lock times. Known: a single key under `tr()` is refused by the compiler, and python-bip380's `v:multi_a` script differs from BIP 379's | the lift of a miniscript to a formula, written from the fragments' meanings, and the rules by which a policy is refused | outside-and-own |
| sb-keys | rsa, sha2, chacha20 (C4), x509-cert and der (C5); CMS, signature lists and `.auth` Faraday's | OpenSSL (tests) | GnuTLS's `certtool` reads every certificate and verifies its self-signature (X2); the OVMF run (`faraday/tools/sb-ovmf-check.py`) named in `other` and run by `just crosscheck` (X1) | none | none (tests) |
| sb-sign | rsa, sha2, x509-cert and der (C5); Authenticode and CMS Faraday's | sbverify, OpenSSL (tests) | the OVMF run (X1); pesign's `pesigcheck` (NSS) does not take the db certificate as a signer (§5.7), so it judges the real signed binaries (sb-check) and not the images made here | none | none (tests) |
| sb-check | x509-cert (C5), der | sbverify; images signed by `sbsign` and `osslsigncode` | `pesigcheck` on two real signed binaries, each under 2 MiB, their signer and image hash named and compared with `pesign`'s (X2); the OVMF run (X1) | none | none (tests) |
| starter-signed | rsa, sha2; the PE checksum and certificate table Faraday's | sbverify (tests) | python3-pefile's PE checksum field and certificate-table entry, compared with the image's header for every image signed, moved or real (X2) | none | none (tests) |
| gpg-create, gpg-sign, gpg-verify | Own packets over ed25519-dalek; base64ct (C8) | GnuPG 2.4 (tests) | Sequoia's `sq`, built over Nettle, reads every certificate and verifies every signature; signatures `sq` makes join gpg-verify's fixtures (X24) | none | none (tests) |
| kdbx | Own format until the export is folded into the vault (F190 point 10); argon2 in one call (C13), chacha20, hmac, sha2 | pykeepass | none: X25 is withdrawn, `keepassxc-cli` reading the vault the export becomes (`vault-kdbx`) | none | outside |
| totp | Own (§3.4) | none (the RFCs' vectors in tests) | a new area `totp`: pyotp over random keys, times, digits, periods and hashes, and its `parse_uri` over otpauth URIs (X26) | none | outside |
| vault-create, vault-unlock, vault-seal | The FVLT container, Faraday's own, until F190's vault replaces it (F189 point 10, F190 point 9); its Argon2 call and HKDF go with it, not moved (C13, C15) | argon2-cffi, PyNaCl, PyCA, until then | none: the actions become the KDBX vault's, judged as vault-kdbx is | the container's second version (`area_vault.py`, `tools/vault/open.py`'s FVLT reader) goes with the container; what stays of it for VAULT §7's names is not checked | outside-and-own until then; after, not checked |
| vault-kdbx | The KDBX 4 container written here (§3.4); argon2 in one call (C13), chacha20, aes, cbc, hmac, sha2, miniz_oxide | none: pykeepass's files read in tests | area `vault`: pykeepass and `keepassxc-cli` for the primitives, keepass-rs for the format only (F193), each opening the files the other made (F190 point 11, `docs/VAULT.md` §10, built with the vault and not an X batch); PyCA's Argon2id as a second Argon2 lineage (X7) | none | outside |
| vault-qr-pgp | The RFC 9580 message written here (§3.4); argon2 in one call (C13), aes, ocb3; HKDF Faraday's, one copy (C15) | none: RFC 9580's vectors in tests | area `vault`: Debian's `sq`, over Nettle, for the primitives, rPGP for the format only (F193), pysequoia once its backend is checked, OpenPGP.js when Node is taken, each opening the files the other made (F190 point 11, VAULT §10, built with the vault); PyCA's Argon2id (X7) | none | outside |
| vault-contents | The TOML tree over toml_parser (§3.4) | none: toml-test's cases in tests | CPython's `tomllib` reads every vault's TOML tree beside Faraday's reader, in area `vault` (F193) | none | outside |
| starter-write | lzma-rust2, sha2, getrandom; GPT and MBR, the FAT32 formatter (E1), the plist reader, the UUID form and CRC-32 Faraday's | `sha256sum`, `xz` as encoder, `sfdisk` and dosfstools (tests; required in `just crosscheck`, X1) | built (X28, `faraday/shells/starter/tests/all/outside.rs`): `xz` decoding good and corrupted streams beside lzma-rust2, 7-Zip (`7zz` or `7z`, upstream 7-Zip, not p7zip) as a second decoder lineage, `sgdisk -v` beside `sfdisk --verify` on the GPT and MBR tables written, `plistlib` over the `diskutil` fixtures, `uuid` on the IDs drawn; open: `zlib.crc32` (X12) | none | none (tests) |

Unchanged:

- Faraday's own specifications, checked by Faraday's second version
  only, with no outside implementation: cards, camera, pointer-keys,
  mix, device, fresh, booklet, roller (class `own`); vault-edit's
  records, which run no primitive and take the vault's class (FVLT's
  typed records today, KeePass entries once F190's vault is built).
- Judged by others' code already, with nothing chosen to add:
  multisig-account, xpub-spellings, wif-shown, address-read,
  vault-load (`outside`); lightning-node (its ldk-node composition stays
  Faraday's second version, since lightning.js needs Node), bsms,
  codex32-correct, xor (`outside-and-own`); gpg-revoke, gpg-paperkey,
  gpg-restore (GnuPG and paperkey in tests).
- Nothing to compare with a second program: hold, words-shown,
  system-random, diceware (the committed EFF files are the judge),
  words-identity, selftest, envelope, inbox-secret, vault-reveal,
  starter-recovery-key.

After X1 to X30, 73 of the 94 actions of 2026-10-31 name an area: 37
`outside`, 28 `outside-and-own` (master's version rule is kept, X14; tx-decode's no-inputs refusal, X15), 8 `own`; 21 name none, of
which 11 are judged by outside programs in tests. The vault's actions are counted there as they were, four in
area `vault` as `outside-and-own`; once F190's vault is built and judged
(VAULT §10), vault-kdbx and vault-qr-pgp join area `vault` as `outside`,
vault-contents has no area, and the class of the four is not checked
until that build says what of Faraday's second version stays.

### 9.5 Not taken, and why

| Reference | For | Why not |
|---|---|---|
| EntropyLab's `app.js`, `vanity.js`, `lightning.js` | six-as-zero and direct selection; the vanity search's expected work and prefix rules; ldk-node's and LND's node keys | Node, not now (owner); SeedSigner's step covers part of dice without it |
| Ian Coleman's BIP-39 tool | dice mode, the hashed cuts at 15, 18 and 21 words, raw coins and hex | Node, not now |
| npm `aezeed` over npm `aez` | aezeed | Node, not now; its AEZ is Yawning Angel's Go lineage, which LND already is |
| Caravan's packages, Coinkite's BBQr JavaScript, kdbxweb | wallet files, BBQr, KDBX | Node, not now |
| Sparrow and drongo; Hummingbird | wallet files, BSMS, CompactSeedQR reading, PSBT change; UR | a JVM, not taken (owner) |
| URKit (Swift); KeePass 2 (.NET) | UR's original; KDBX's author's program | Swift and .NET, not taken (owner) |
| libsecp256k1's silent payments module | silent-send's sender | the project of the app's curve library; btclib instead (owner) |
| EntropyLab's `vanity-wasm` | the vanity search | collides: `secp256k1-sys` (libsecp256k1) and RustCrypto's sha2, the app's |
| the `lightning` crate, ldk-node | ldk-node's key | collides: it derives through rust-bitcoin's `bip32` and rust-secp256k1, the app's; the five ldk vectors in `tools/vectors/aezeed/` came from the same code |
| rust-silentpayments; coincurve, python-secp256k1, python-bip32, bip_utils, bitcoinjs's `bip32`, Electrum's `electrum_ecc` | silent payments; key derivation | collide: libsecp256k1 or a binding to it |
| embit beyond its BIP-39; Specter's importer over it | key derivation, PSBT, wallet files | embit loads libsecp256k1 through ctypes whenever it can, with no switch; taken in full, with that binding removed and its absence asserted, only with Specter's importer, which waited for wallet-files to hold more real files (X30, built); not taken |
| quirc, quircs, OpenCV's QR decoder | reading pictures | rqrr is built on quirc (rule 3) |
| Sequoia over RustCrypto; keepass-rs | GPG; KDBX | RustCrypto's primitives, the app's (Debian's `sq`, over Nettle, is taken). F190 point 11, of 2026-11-02, names keepass-rs and rPGP (over RustCrypto too, not checked for keepass-rs's Argon2) as the vault's test-only judges, beside pysequoia (its backend not checked) and OpenPGP.js (Node): settled by F193, keepass-rs and rPGP judge the format only, pysequoia counts once its backend is checked, OpenPGP.js waits for Node |
| anything over rust-bitcoin or rust-miniscript: BDK, Liana, the `bip322` crate, rust-psbt's `psbt-v2` | scripts, PSBT, BIP-322 | the app's libraries |
| sipa's C++ miniscript compiler | compile | it chooses differently by design, so only validity and equivalence could be compared, which X20 does |
| the Go SLIP-39 ports | slip39 | each says it is a port of python-shamir-mnemonic (rule 3) |
| Python's `lzma` beside `xz` | starter-write | the same decoder, liblzma: counted once |
| libsodium's Argon2 (PyNaCl's `crypto_pwhash`) | the vault, kdbx | from the reference code, argon2-cffi's lineage (from the projects' descriptions, not checked) |

Not taken now, with nothing against them: ur-rs as a program outside
the workspace (rule-wise allowed); btcd as a fourth reader of PSBTs,
sighashes, transactions, signatures and finalizing; Bitcoin Core on
mainnet for the extended key versions `master` takes; Core's
`validateaddress` for `address-read`; `hdkeychain` for
`multisig-account`, `wif-shown` and ldk-node's steps; `oathtool`;
RNP; LIEF, signify and Windows' own `signtool` in the test virtual
machine; Project Wycheproof's vector files; BIP 93's own listings.
Each is a later choice.

### 9.6 Tools and packages

No new toolchain: Python, Go, Rust and a C compiler are prerequisites
already. Each item below is a prerequisite in `docs/DEVELOPING.md` §1,
added by the batch named, and required by `just crosscheck` (rule 4).
Package names are Debian's and Ubuntu's where they were read; where
they were not, the batch checks them first.

| Batch | Distribution packages | Vendored in `tools/reference/` (commit and SHA-256) |
|---|---|---|
| X1 | mtools, dosfstools, util-linux's `sfdisk`, QEMU and OVMF's Secure Boot firmware (for the OVMF run) | none |
| X2 | `gnutls-bin` (`certtool`), `pesign` (`pesigcheck`), `python3-pefile` | two real signed EFI binaries, each under 2 MiB (`tools/reference/signed-efi/`: Ubuntu's shim-signed 1.59+15.8-0ubuntu2, with its source, licence and SHA-256) |
| X6 | `python3-pil` (Pillow) | none |
| X7 | `python3-cryptography` 44 or later, over OpenSSL 3.2 or later (not checked on the build machine) | none |
| X10 | none | rust-codex32 at commit `3943cb52b2343333386fac0cd38b8ae5a4d6de02` (CC0 1.0; its `Cargo.toml` has no dependency), built offline outside the workspace; volvelle's worksheet program is in `tools/vectors/codex32-volvelle/` already (CC0 1.0) |
| X11 | none | the bip39_slip39 wheel, 2026.10.24, beside btclib's |
| X12 | `python3-cbor2` | none |
| X14 | none | a released btcd's modules (`btcutil`, `txscript`, `btcec` and what they name; the root daemon module is not used), `go mod vendor`, in a folder of their own; built |
| X21 | `python3-segno`, `zbar-tools`, `python3-zxing-cpp` | none |
| X22 | none | SeedSigner's `helpers/mnemonic_generation.py`, at the commit `tools/reference/seedsigner/` has |
| X23 | none | krux's `src/krux/qr.py` and what it imports, with shims; Coinkite's BBQr test files |
| X24 | `sq` (Sequoia), built over Nettle: the batch checks the build's backend | none |
| X25 | (withdrawn: `keepassxc` comes with the vault's cross-check, `docs/VAULT.md` §10) | none |
| X26 | `python3-pyotp` | none |
| X28 | `xz-utils`, `7zip` (7-Zip's `7zz`, or `7z` where the system names it so; the banner must read 7-Zip, not p7zip), `gdisk` (`sgdisk`) | none |
| X30 | none | real wallet files from the wallets' published test suites, as fixtures (built: `tools/vectors/wallet-files/`, with each file's repository, commit, licence and SHA-256) |

The standard library covers the rest: `hashlib`, `hmac`, `json`,
`zlib`, `plistlib`, `uuid`. btclib's wheels, Core's functional-test
framework and its `crypto/ripemd160.py`, python-ecdsa, PyCA
cryptography, Go and the volvelle harness are in place already.

### 9.7 The batches

Each is one commit and one passing `just crosscheck`, and changes the
actions' `independence` in `docs/crypto/actions.toml`, the areas'
facts, `tools/crosscheck/README.md`'s tables (`facts.py`, `gen.py`),
`references.sha256` and the stamp, and, where it adds a tool,
`docs/DEVELOPING.md` §1. Where it retires a part of Faraday's second
version, the area's code for that part goes in the same commit and the
area's sabotage cases are kept against the outside judge. A batch with
a switch lands in that switch's commit (rule 6).

| Batch | With | What | Size |
|---|---|---|---|
| X1 | alone, after B1, before C1 | rule 4: `just crosscheck` checks every tool it needs first and names a missing one; the versions in the stamp; the judged test files run with `FARADAY_JUDGES_REQUIRED=1`; the OVMF run named in sb-keys', sb-sign's and sb-check's `other` and run | 1 to 1.5 days |
| X2 | C5 | `certtool` reads every certificate; `pesigcheck` verifies the real signed binaries (not the images Faraday signs, §5.7); real signed binaries read and their signer named; pefile's PE checksum and certificate-table entry | 1.5 to 2 days |
| X3 | C6 | btclib's finalizer beside Core in area `sign` | about 1 day |
| X4 | C9 | coins-hex's hex judged as outside (rule 5) | about an hour |
| X5 | C11 | CPython's `json` reads every wallet file the area writes and refuses a duplicate key and a depth over 32, beside the visitor | about 2 hours |
| X6 | C12 | every PNG the app writes read by Pillow and zlib | about 2 hours |
| X7 | C13 | PyCA's Argon2id beside argon2-cffi for the one Argon2 call: in area `kdbx` over the export's, and in area `vault` over the KDBX vault's and the QR vault's once VAULT §10 builds it | about 2 hours |
| X8 | A1 | btclib's MuSig2 in full and BIP-373's roles; Core's `musig()` descriptors; the Core round trip named as sharing the curve | 1.5 to 2 days |
| X9 | A7 | btclib's BIP-352, BIP-374 and BIP-375 in areas `sp` and `sps` | 1.5 to 2 days |
| X10 | A4 | rust-codex32 and volvelle's worksheet program; the threshold rule and the 48- and 74-character rows retired | about 2 days |
| X11 | A5 | bip39_slip39 vendored, in areas `bip39`, `slip39` and `bip85` | 1 to 1.5 days |
| X12 | C15 | `zlib.crc32`, cbor2 and PyCA's `HKDFExpand` against the one CRC-32, CBOR and HKDF-Expand | about 1 day |
| X13 | alone | rule 2 (built): vanity's child keys by btclib; sweep-sign and wif-read judged on btclib's curve and by python-ecdsa; Core named as sharing the curve | 1 to 1.5 days |
| X14 | alone | a released btcd vendored; `hdkeychain` for master, derive and xprv-shown; btcd's control-block check for tapmulti. Built: btcd v0.27.0 in `tools/reference/btcd/`; master's strict reader is kept (§9.4) | 1.5 to 2 days |
| X15 | alone | btclib in place of Faraday's second version for psbt-read, psbt-v2, sighash-review, tx-decode and tapmulti; Core's `decoderawtransaction`. Built: psbt-read and psbt-v2 (area `psbt`, with `psbt.v0` and `psbt.v2` in the self-test), sighash-review, tx-decode (area `tx`, with `tx.read` and `tx.sighash`) and tapmulti (area `taproot`, with `taproot.verify`) | Built |
| X16 | alone | btclib's change rule for psbt-review (built) | about 1 day |
| X17 | after X16 | a watch-only descriptor wallet on Core's regtest node (`core_node.py`, CROSSCHECK §10.4's "no wallet" changed), `ismine` and `ischange` (built: `watch_wallet`, made by the area that asks and deleted after; psbt-review asks it of every output btclib calls change or spoof, `ischange` being Core's "the wallet's, not in its address book", true of both chains, so the chain is the parent descriptor's and the position the key origin `walletprocesspsbt` writes) | 1.5 to 2 days |
| X18 | alone | btclib's descriptors and miniscript for address, descriptor and recovery | Built |
| X19 | alone | area `sign-check`. Built: partial and complete signatures of every script kind and sighash type, verified by python-ecdsa and btclib over btclib's sighashes, run by btclib's engine and judged by Core, and the nonces reused on purpose (§9.4) | Built |
| X20 | alone | area `compile`. Built: policies compiled by Faraday, taken by Core, python-bip380 and btclib, and compared with the miniscript as boolean formulas (§9.4) | Built |
| X21 | alone | segno, zbar and zxing-cpp; outside codes read by rqrr. Built: segno 1.6 as a second encoder in area `seedqr`, and zbar (`zbarimg --raw -Sbinary`) and zxing-cpp decoding every picture of it, the payloads compared as bytes (`tools/crosscheck/qr_decode.py`) | 2.5 to 3 days |
| X22 | alone | SeedSigner's dice steps | half a day to 1 day |
| X23 | alone | krux's numbered parts; Coinkite's test files | 1 to 1.5 days |
| X24 | alone | `sq` in the GPG tests | about 1 day |
| X25 | alone | (withdrawn: `keepassxc-cli` judges the vault the export is folded into, in area `vault`, F190 points 10 and 11, `docs/VAULT.md` §10) | None |
| X26 | alone | area `totp` with pyotp | about half a day |
| X27 | alone | area `hash`; `hashlib` and `hmac` for HMAC-SHA256 and PBKDF2-HMAC-SHA256; `hashlib.scrypt`; the Base58Check assertion | about 1.5 days |
| X28 | alone | starter-write's decoders, tables, plist and IDs. Built: tests in the Starter's `outside` module, run with `FARADAY_JUDGES_REQUIRED=1` by `just crosscheck`: `xz` and 7-Zip over good and damaged streams, `sgdisk -v` beside `sfdisk --verify`, `plistlib` over the `diskutil` fixtures, `uuid` on the GUIDs on a stick | Built |
| X29 | alone | aezeed's software-round pass | Built |
| X30 | alone | more real wallet files | Built |

The batches standing alone change no app code but the one test X27
changes, and may land at any point after X1, in the order listed, which
puts what checks keys, addresses and signatures first. X1 takes 1 to
1.5 working days; X2 to X12 about 10.5 to 12.5, inside their switches'
commits; X13 to X30 about 21.5 to 27.5; in all about 33.5 to 41.5.

### 9.8 Not decided: a vault reader written from VAULT.md alone

Withdrawn: the container it would have read, Faraday's own FVLT format,
is replaced by KDBX 4 and RFC 9580, which others' code reads (F189, F190);
the vault is judged by the programs `docs/VAULT.md` §10 names.
