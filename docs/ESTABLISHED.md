# Established implementations first

**Kind:** Spec
**Status:** Specified (F178, F180, F181); built: AEZ's AES round from the aes crate (§4.2, §5.3), SLIP-39's GF(256) and checksum without a branch (§4.3), and the scalar arithmetic on secrets in Faraday's own branch-free form (§4.1's starting point)
**Open:** the inventory's name matching (§2.5); the scalar arithmetic on secrets through libsecp256k1 (§4.1); the timing fixes of §4.4 to §4.7; the switches (§5); the reasons in the manifest and their check (§2); the generated lists (§3); the documentation fixes (§7); the FAT32 formatter (§3.2, with the stick work of `docs/STICK-IDENTITY.md`)

Faraday uses an established implementation wherever one exists for what
it needs, and writes its own only where none exists or none fits its
rules for keys, saying why for each piece. This spec states the rule
(§1), how `just lint` holds it (§2), what Faraday writes itself once the
work here is built (§3), the places where a secret's value steers time
or leaves copies, the fixes and what the security model already bounds (§4), the switches to outside code
available now (§5), what is checked again later (§6), the documentation
fixes (§7), the order of work (§8) and how the cross-check's outside
sources are chosen (§9). The choices are the owner's of 2026-10-30
(F178) and 2026-10-31 (F180, F181).

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
Faraday's own (the vault file, the session's randomness, the mixing of
entropy) and carry F160's label. The list of what Faraday implements
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
functions an action's fingerprint covers. It gets two cases wrong, and
both change fingerprints the later batches re-stamp, so A0 comes first.

**(a) A method named like a standard method, on a Faraday type with no
rule of its own, called from outside its impl, is missed.** SLIP-39's
`recover_secret` and `split_secret` call `Points::at`
(`core/faraday-bip/src/slip39.rs`); `at` is in `STD_METHODS`, `Points::at`
calls no primitive itself, and `recover_secret` is a free function, so
no branch of `link`'s method rule takes it: `Points::at`, `gf_mul` and
`gf_inv` are absent from the `slip39` block of `docs/crypto/actions.lock`.

**(b) A same-named free function elsewhere in the crate is counted.**
`codex32/correct.rs` and `codex32/worksheet.rs` import `gf_mul`,
`polymod` and `verify_checksum` with `use super::{..}`; `narrowed` keeps
every free function of that name in the crate for a path starting with
`crate` or `super`, so `codex32-make` and `codex32-correct` carry
`slip39.rs`'s `gf_mul` and `polymod`.

The fix:

1. `narrowed`: a path whose first segment is `crate`, `super` or `self`
   is made absolute against the caller's module (`Func.module`, which is
   `file_mods(file)` plus inline `mod`s): `crate::a::b::f` names module
   `[a, b]`; `super::f` the caller's module without its last segment
   (repeated for each `super`); `self::f` the caller's module. A free
   function fits only when its `module` equals that module; a method
   `T::f` when its type is `T` and its module is that module. When none
   fits, the call is followed through `PubUses` (which
   `read_reexports` collects) and otherwise links to nothing. The
   fallback that keeps every function of the name stays only for a bare
   name with no `use` line and a glob import in the file.
2. `link`, methods: a callee `T::m` whose name is in `STD_METHODS` is
   also taken when the caller's signature or body names `T`
   (`Func.idents`) and `T` is a type of the same crate. The file-level
   test (`file_names`) stays for names not in `STD_METHODS`.
3. `tools/costs-lint/src/selftest.rs` gains, in its fixture crate: two
   modules each with a free `gf_mul`, one importing the other's through
   `use super::gf_mul`, and a check that the caller links to its
   parent's only; and a private `struct Pts` with `fn at(&self)` that
   calls a hash, called from a free function taking `&Pts`, with a check
   that the free function links to `Pts::at` and that a free function
   not naming `Pts` but calling `.at()` does not.
4. After the change, `python3 -I -B docs/crypto/check.py check` names
   every action whose lock lines change; each is re-stamped with
   `just crypto-review` in the same commit (at least `slip39`, gaining
   `Points::at`, `gf_mul` and `gf_inv`, and `codex32-make` and
   `codex32-correct`, losing SLIP-39's `gf_mul` and `polymod`).

About half a day with the re-stamps.

## 3. What Faraday writes itself

The content of STANDARDS §10 and AUDIT §3 once §4, §5 and §7 are built,
before §6. The decision is F178 for the pieces in the tree on
2026-10-30 and F180 for the rest (the column says which). The action is
the manifest entry that carries it; where a piece's action is not given
below, D1 takes the action whose functions reach it, from the inventory.

### 3.1 Faraday's own specification

| Piece | Where | Action | Decision | Note |
|---|---|---|---|---|
| The four-equal-slot vault container and its records | `faraday/faraday-vault` | `vault-create` | F178 | Faraday's own construction, built from standard primitives (Argon2id, HKDF-SHA256, XChaCha20-Poly1305) and labelled as not yet independently reviewed (`docs/VAULT.md`, F158, F160). |
| The session's random generator: its seed and pool, and the mixing of a person's own randomness | `faraday/faraday-core/src/fresh.rs` | `fresh` | F178 | Faraday's own construction over SHA-256, labelled as not yet independently reviewed (`docs/OWN-RANDOMNESS.md` §2.3, F86). |
| Dice, coin, card, hex and camera entropy accumulation and packing, and the mix of sources | `core/faraday-entropy` | `dice` | F178 | Faraday's own procedures over SHA-256; the card, camera and mix procedures are labelled as not yet independently reviewed (F158). |
| PSBT inspection, signing orchestration and verification logic | `core/faraday-psbt` | `psbt-read` | F178 | What Faraday checks before it signs is Faraday's own rule; the PSBT format itself is rust-bitcoin's. |
| The choice among a script's satisfactions by real signature size | `core/faraday-psbt/src/finalize.rs` | `finalize` | F178 | Faraday picks the smallest witness as Bitcoin Core's `finalizepsbt` does; miniscript builds and checks each witness. |
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
| The KDBX 4 file format | `core/faraday-backup/src/kdbx.rs` | `kdbx` | F178 | No Rust KeePass library comes from a publisher Faraday's dependency review trusts. |
| CMS SignedData, Authenticode's image hash and signature, EFI signature lists and `.auth` files | `faraday/faraday-sb` | `sb-keys` | F178 | No library writes Authenticode signatures or EFI signature lists; the CMS crate for the DER version Faraday uses is a release candidate (§6.3). |
| BC-UR, with its CBOR (carries) | `core/faraday-codec/src/ur/`, `faraday/faraday-qr/src/cbor.rs` | `ur` | F178 | No library for this format comes from a publisher Faraday's dependency review trusts. |
| BBQr (carries) | `faraday/faraday-qr/src/bbqr.rs` | `bbqr` | F178 | No library for this format comes from a publisher Faraday's dependency review trusts. |
| SeedQR (carries) | `core/faraday-codec/src/seedqr.rs` | `seedqr-make` | F178 | No Rust library implements SeedQR. |
| The QR encoder, a port of Nayuki's `qrcodegen`, with its GF(256) and Reed-Solomon (carries) | `core/faraday-codec/src/qr.rs` | `qr` | F178 | No QR encoder comes from a publisher Faraday's dependency review trusts; Faraday's port of Nayuki's code wipes a SeedQR's buffers. |
| CRC-32 | `core/faraday-crypto/src/crc32.rs` | `ur` | F180 | The CRC-32 crate in the tree, crc32fast 1.5.1, is not covered by an outside review at that version; Faraday's is a branch-free loop checked against the published check values (§6.8). |
| FAT16 and FAT32, read and written (carries) | `faraday/faraday-fat` | `stick-write` | F180 | No FAT library comes from a publisher Faraday's dependency review trusts. |
| The FAT32 formatter for the Starter's data partition (carries) | `faraday/faraday-fat` | `starter-write` | F180 | No FAT library comes from a publisher Faraday's dependency review trusts; the partition it makes is checked by dosfstools' `fsck.fat`, against what `mkfs.fat` makes. Built in batch E1. |
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
missing). faraday-fat's module doc changes "Nothing formats a
partition" to "The Starter formats the data partition; nothing else
formats one". Its batch, E1, goes with the Windows stick work of
STICK-IDENTITY and starts on the owner's word; until it lands, the entry
is not written and the count does not include it.

### 3.3 One exists but does not fit the key rules

| Piece | Where | Action | Decision | Fails | Note |
|---|---|---|---|---|---|
| scrypt | `core/faraday-crypto/src/scrypt.rs` | `scrypt` | F178 | scrypt 0.12.0 frees its passphrase-derived working buffers (`b`, `v`, `t`) without wiping them | The Rust scrypt library leaves its passphrase-derived working memory unwiped; Faraday's wipes it (§6.5). |
| HKDF-SHA256, one copy | `core/faraday-crypto/src/hash.rs` | `vault-create` | F180 | hkdf 0.13.0 returns the pseudorandom key and leaves each output block T(i) in plain arrays it does not wipe | The Rust HKDF library leaves the pseudorandom key and its output blocks unwiped; Faraday's holds both in buffers it wipes (§6.7). |
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

Thirty-eight pieces in all once E1 lands (thirty-seven before), against
the eighteen lines of AUDIT §3 today. The switches remove BLAKE2b, the
AES round, the scalar arithmetic on secrets, the RSA RNG adapter, X.509
and the DER writer, the single-key and multisig finalizer, the sealing's
counter nonce, four copies of base64, about twenty of hex, the
descriptor checksum, the JSON parser, Adler-32, the hand-written wipes
and constant-time zero check, and one of the two copies of HKDF and of
the Argon2id call; the encodings, BIP-322 and the formats written here
and never listed are listed. Each kept piece is in one copy (§5.17).

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
  (§3.4). `is_zero` and the masked `at_least_order` stay for the nonce's
  check after `reduce`.

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

**Built so far.** The batch is written and not yet merged: it meets
every point above but the re-stamping. `docs/crypto/actions.lock` still
names `musig.rs`'s `add_mod_order`, `at_least_order`, `negate_mod_order`
and `zero_bit`, which it renames or removes, so `check_fingerprints`
fails until the reviews are made. Landing it is: rebase on main, run
`check.py check`, `just crypto-review` for each action it names,
`just crosscheck` again if the stamp's inputs changed.

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
  manifest's text for `scrypt`, `aezeed`, `vault-unlock` and `kdbx` says
  so.
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

### 5.1 BLAKE2b (batch C1)

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

### 5.6 The RSA RNG adapter (batch C4)

- **Files.** `faraday/faraday-sb/src/rng.rs` (53 lines), its callers in
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

### 5.7 X.509 and DER (batch C5)

- **Files.** `faraday/faraday-sb/src/der.rs` (164 lines) and
  `faraday/faraday-sb/src/x509.rs` (336 lines); the CMS, Authenticode
  and EFI code that builds DER with `der.rs`
  (`faraday/faraday-sb/src/pe.rs`, `faraday/faraday-sb/src/efi.rs`).
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

### 5.8 The single-key and multisig finalizer (batch C6)

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
  of the smallest satisfaction by real signature size, a `multi()`
  keeping its k smallest signatures (§3.1), extended to legacy `sh`
  multisig; the signatures under a taproot `pkh` leaf taken off as today;
  the finalizer run on a copy of the input, as `satisfy` does today,
  because miniscript's `finalize_input` drops unknown and proprietary
  fields: Faraday keeps those and clears the signing fields as BIP-174
  requires.
- **Proof.** `tools/vectors/psbt` (BIP-174's finalizer and extractor
  vectors); area `sign` (Bitcoin Core's `finalizepsbt` and
  `testmempoolaccept`); the wallet fixtures
  (`core/faraday-psbt/tests/all/wallet_fixtures.rs`). The transactions
  are byte for byte the same.
- **Size.** One to two days.

### 5.9 The sealing's counter nonce, removed (batch C3)

- **Files.** `core/faraday-crypto/src/sealed.rs` (653 lines):
  `Sealed`, `SessionKey`, `SealError`, `SealedBytes`, the counter nonce,
  and `KEY_LEN`, `NONCE_LEN` and `TAG_LEN` (unless a user is found, then
  they move with it) go; `MnemonicBytes`, `SeedBytes`, `MAX_SEED_LEN` and
  `MAX_MNEMONIC_WORDS`, which `faraday-bip`, `faraday-vault` and
  `faraday-keys` use, move to `core/faraday-crypto/src/secret.rs`.
  `core/faraday-crypto/src/lib.rs`'s exports and module doc; the
  comment in `core/faraday-bip/src/keys.rs` that waits for a
  `Sealed<T>`; `tools/lint-secrets.sh`: rule 2's type list loses `Sealed`
  and `SessionKey` (the allowed name `backup_flow`'s `Sealed` in
  `faraday-core` stays), and `SECRET_FILES` loses `sealed.rs` and gains
  `core/faraday-crypto/src/secret.rs` if it is not there.
- **New in the graph.** None; `faraday-crypto` drops its
  `chacha20poly1305` dependency (the vault keeps its own).
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

### 5.10 Base64, four copies (batch C8)

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
  `faraday/faraday-intake/src/single_key.rs`, `core/faraday-selftest/src/lib.rs`.
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
- **Size.** About two hours.

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

- **Files.** `faraday/faraday-vault/src/lib.rs`: `committed`, an OR-fold
  and `black_box`, becomes `block.ct_eq(&[0u8; N]).into()` through
  `subtle::ConstantTimeEq`; the Argon2 memory's `fill(0)` and
  `black_box` and those of `core/faraday-backup/src/lib.rs` and
  `faraday/faraday-fat/src/lib.rs` (a cluster block of file data) become
  `zeroize::Zeroize::zeroize` (argon2's `Block` implements `Zeroize`).
- **One Argon2id call.** `faraday-vault`'s Argon2id and
  `core/faraday-backup/src/lib.rs`'s `argon2id` are the same function:
  reserve the memory, `hash_password_into_with_memory`, wipe. It moves to
  `faraday_crypto::argon2id(password, salt, params, out: &mut Secret<..>)`,
  which takes `c: &Compute` and says "Costs an Argon2id derivation" in
  its first line; both call it, and their parameters stay as they are.
- **New in the graph.** None; `faraday-fat` gains `zeroize` (trusted, in
  the tree).
- **deps page.** `docs/deps/zeroize.md` and `docs/deps/subtle.md` add the
  users.
- **Manifest.** `vault-create`, `vault-unlock` and `kdbx` gain `through`
  argon2 by `core/faraday-crypto`.
- **Proof.** `tools/vectors/vault`, `tools/vectors/vault-primitives`,
  `tools/vectors/kdf`; areas `vault` and `kdbx`; `tools/vault/open.py`
  opens the same vectors; the tests of `faraday-fat`.
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
- **HKDF.** `faraday/faraday-vault/src/lib.rs`'s `hkdf_sha256` moves to
  `faraday_crypto::hkdf_sha256(salt, ikm, info, out: &mut Secret<..>)`
  beside `hkdf_sha256_expand`, the pseudorandom key in a `Secret`; the
  vault calls it with an empty salt and drops its own `hmac` and `sha2`
  where nothing else uses them. RFC 5869's cases move to
  `core/faraday-crypto/tests/primitives.rs`.

**Proof.** The tests of every crate touched; the Starter's stick and
card tests (`tests/all/sticks.rs` in its shell); `tools/vectors/ur`,
`tools/vectors/vault`, `tools/vectors/vault-primitives`; areas `ur`,
`vault`, `sign`; the blank templates' snapshots unchanged.

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
`through` hkdf by `core/faraday-crypto` on `vault-create` and the actions
that reach `hkdf_sha256_expand` (signing's aux randomness, the session's
blinds). RFC 5869's cases, `tools/vectors/vault`,
`tools/vectors/vault-primitives` and areas `vault` and `sign` prove the
bytes. About half a day.

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
them. The batch that builds the last open item moves §4, §5 and §8 to
docs/history/ESTABLISHED-LOG.md and makes the document a Reference
(CLAUDE.md); the work kept for later (§6) is then its own Spec or the
Reference's list of conditions.

| Batch | What | Sections | Size |
|---|---|---|---|
| A0 | The inventory's name matching; the actions it moves re-stamped | §2.5 | About half a day |
| A1 | Scalar arithmetic on secrets through libsecp256k1, from the written batch; the re-stamps it lacks | §4.1, §5.4 | About half a day |
| A2 | AEZ's AES round from `aes` | §4.2, §5.3 | Built |
| A3 | SLIP-39's GF(256) and checksum without a branch | §4.3 | Built |
| A4 | codex32's GF(32), checksum and alphabet without a branch; one `gf_mul`; `fresh_set` sized | §4.4 | About half a day |
| A5 | Word lookups, `set11`, aezeed's `pack` and the sentence's gathering in constant time | §4.5 | About 1 day |
| A6 | Secret comparisons through subtle; paperkey's CRC-24 masked | §4.6 | About 2 hours |
| A7 | BIP-352's label tweak times G through a secret key, `t_k` as libsecp256k1's own module computes it; the key's own randomized contexts | §4.7 | About half a day |
| B1 | AUDIT §1's rows, the QR encoder's module doc, the storage command's comment, `docs/deps/rsa.md`'s Marvin sentence, the Secret handling lines that claim more than the code does (§7.8), KEYS §2 and the `hold` action saying what wiping does not reach | §7 | About 3.5 hours |
| C1 | BLAKE2b from `blake2`'s `Blake2bVarCore` | §5.1 | About 3 hours |
| C2 | (HKDF: not a switch; kept, §5.2, §6.7) | §5.2 | None |
| C3 | The sealing's counter nonce removed | §5.9 | About 2 hours |
| C4 | The RSA RNG adapter from `ChaCha20Rng`; tests re-pinned | §5.6 | About half a day |
| C5 | X.509 from `x509-cert`, DER from `der`; trust entries for x509-cert and der_derive, flagset's exemption | §5.7 | 1.5 to 2 days |
| C6 | The single-key and multisig finalizer from miniscript | §5.8 | 1 to 2 days |
| C7 | (The BIP-39 core: not a switch; kept, §5.5, §6.9) | §5.5 | None |
| C8 | Base64 from `base64ct`, four copies | §5.10 | About half a day |
| C9 | Hex from `base16ct`, one module | §5.11 | About half a day |
| C10 | The descriptor checksum from miniscript | §5.12 | About 2 hours |
| C11 | JSON from `serde_json` behind Faraday's visitor; `holds_secret` keeps no value | §5.13 | About 1 day |
| C12 | Adler-32 from `adler2` | §5.14 | Under an hour |
| C13 | The zero check through subtle, the wipes through zeroize, one Argon2id call | §5.15 | About 2 hours |
| C14 | The quiz's seed from the session's generator | §5.16 | About an hour |
| C15 | One copy of each kept piece | §5.17 | About 1 day |
| D1 | Schema 5: `written_here` as entries with `why`, `note`, `decision`; `[[not_established]]` for lzma-rust2 and rqrr; getrandom's trust entry; `check.py` (§1.2's three ways, the version checked) and `gen.py`; AUDIT §3 grouped; STANDARDS §10 generated and §0's bullet; the counts; `crypto.established` made active; the manifest's entries filled as §3 | §1.2, §1.4, §2, §3 | 1 to 1.5 days |
| E1 | The FAT32 formatter in faraday-fat, its `written_here` entry and the count raised under F180 | §3.2 | 1.5 to 2 days, on the owner's word, with the Windows stick work |

A0 comes first, because it changes
fingerprints every later batch re-stamps. A1 and A4 to A7 come before
the switches. C1 to C15 are independent of each other and may land in any
order, but C15 after A4 (it moves the masked `gf_mul`), and each changes
the manifest's `written_here` string of its action as it lands.
D1 comes after them so that the entries it writes are §3's; before D1
lands, the website's reader of the manifest is changed in its own
repository. E1 starts on the owner's word. §6's items come as their
conditions hold. In all, A0 to D1 take about twelve to sixteen working
days, and E1 one and a half to two more.

## 9. The cross-check's outside sources

Not decided (owner, 2026-10-31). The rule: the outside implementations
the cross-check (F79, `tools/crosscheck/README.md`) compares Faraday
with are chosen after the app's own switches (§4, §5) are fixed, because
a library the app uses cannot be its own cross-check: once a piece
moves to a library, the cross-check of that piece needs a different
implementation, and the list is made then.

The review of 2026-10-31 found these, the inputs to that choice; none
is a batch yet:

- 66 of the manifest's 94 actions name a cross-check area: 30 outside,
  27 outside-and-own, 9 own; 28 name none.
- Vendored already: btclib and btclib-wallet carry outside code for
  parts the areas still write themselves: BIP-341's control-block check
  (`tapmulti`), BIP-341's sighash refusals and exact transaction reading
  (`sighash-review`, `tx-decode`), the PSBT refusals of trailing bytes
  and an explicit SIGHASH_DEFAULT on a 65-byte taproot signature
  (`psbt-read`, `psbt-v2`), and a third PSBT version 2 reader. btclib's
  MuSig2, BIP-352, BIP-374 and BIP-375 are a second lineage beside the
  BIPs' references, which all run over secp256k1lab.
- Outside programs in Rust that would run with no new toolchain:
  volvelle-website's `checksum_worksheet.rs` (already vendored as
  vectors) for codex32's worksheet rows at 48 and 74 characters;
  EntropyLab's vanity grinder for the vanity search on mainnet; the
  `lightning` crate for ldk-node's key.
- Outside programs for actions with no area: Python's `hashlib` and Core's
  framework for `tools-hash`; pyotp or oathtool for `totp`; xz-utils for
  `starter-write`'s decoder; Core's `finalizepsbt`, python-ecdsa and
  btclib's script engine for `sign-check`; an outside QR decoder (zbar or
  zxing-cpp) for every picture Faraday draws, a different lineage from
  Nayuki's encoder, which Faraday's is a port of; `keepassxc-cli` beside
  pykeepass for `kdbx`.
- Bitcoin Core as judge of more: a mainnet node for the extended key
  versions `master` takes; Core's wallet (`ismine`, `ischange`) for
  `psbt-review`'s change rule.
- Behind a toolchain decision: Node for EntropyLab's dice procedures and
  Ian Coleman's raw coin and hex mode; a JVM for Sparrow (wallet files,
  BSMS, CompactSeedQR reading). Specter's importer (Python, over embit,
  vendored) reads Electrum and Specter files without either.
- Corrections to earlier lists: SeedSigner's coin flips are hashed, so
  they are not Faraday's packing; the booklet's PostScript computes no
  dice and no worksheet rows; no outside encoder of CompactSeedQR at 15,
  18 or 21 words was found.
- btclib's and EntropyLab's authors are not Faraday's authors (owner,
  2026-10-31), so their code counts as others' code under F160, whose
  independence rests on authorship.
