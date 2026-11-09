# Established implementations first

**Kind:** Reference
**Status:** Built (F178, F180, F181, F190, F191, F192, F193, F213): the timing fixes, the switches, the reasons in the manifest and their check, the generated lists and the cross-check's outside sources. How it was built is in `docs/history/ESTABLISHED-LOG.md`; §6 is the work kept for later. Built along the way: AEZ's AES round from the aes crate (§4.2, §5.3), BLAKE2b from the blake2 crate (§5.1), the sealing's counter nonce removed (§5.9), the Secure Boot keys' generator from the chacha20 crate's ChaCha20Rng (§5.6), X.509 from x509-cert and DER from der, judged by certtool, pesigcheck and real signed binaries (§5.7), SLIP-39's GF(256) and checksum without a branch (§4.3), the scalar arithmetic on secrets in Faraday's own branch-free form and then through libsecp256k1 (§4.1), the single-key and multisig finalizer from miniscript with btclib's finalizer beside Core in area `sign` (§5.8), the second curve lineage beside Core for the vanity search's child keys and for sweep-sign and wif-read (§9.3), the descriptor checksum from miniscript (§5.12), the PNG writer's Adler-32 from `adler2` (§5.14), the cross-check's outside judge btcd v0.27.0 for `hdkeychain` and control blocks (§9.4), btclib for PSBTs, transactions, signature hashes and control blocks (§9.4), the JSON reader from `serde_json` behind Faraday's visitor (§5.13), the inventory's name matching (§2.5), the FAT32 formatter (§3.2), base64 from base64ct (§5.10), hex from base16ct in `faraday_crypto::hex`, judged by CPython's `bytes.fromhex` in area `entropy` (§5.11), the documentation fixes of §7, the cross-check's descriptors and miniscript judges for address, descriptor and recovery (§9.4) and the quiz's seed from the session's generator (§5.16), btclib's change rule judging psbt-review (§9.4) and Core's watch-only descriptor wallet beside it, aezeed's software-round pass (§9.4), area `compile` for the policy compiler (§9.4), area `sign-check` for the signatures a PSBT carries (§9.4), area `import` reading the real wallet files the wallets' own test suites publish (§9.4), krux's numbered parts and Coinkite's BBQr test files judging area `bbqr` (§9.4), the Starter's decoder, tables, property lists and GUIDs judged by `xz`, 7-Zip, `sgdisk`, `plistlib` and `uuid` (§9.4), and zbar and zxing-cpp decoding every QR picture Faraday writes with segno as a second encoder, in area `seedqr` (§9.4), and Sequoia's `sq` over Nettle reading every GPG certificate Faraday writes and verifying every signature, with its signatures among gpg-verify's fixtures (§9.4), and area `hash` judging the hashes, Base58Check, bech32 and hex text and the file hash with Core's framework and `hashlib` (§9.4), and HMAC, PBKDF2 and scrypt judged in the same area by `hmac`, `hashlib.pbkdf2_hmac` and `hashlib.scrypt` (§9.4), and `zlib.crc32` and cbor2 judging the one CRC-32 and the one CBOR in area `ur` and the GPT's CRC fields in the Starter's tests (§9.4), the one HKDF in `faraday-crypto` (`hkdf_sha256` beside `hkdf_sha256_expand`, §5.17) used by the QR vault's message, and PyCA's `HKDFExpand` and `HKDF` judging both in area `hash` (§9.4), which closes the one copy of each kept piece (§5.17), and pyotp judging the TOTP codes, the seconds left and the periods of the vault's one-time passwords and its reading of otpauth URIs in area `totp` (§9.4), and SeedSigner's `mnemonic_generation.py` over embit's BIP-39 judging the hashed dice procedure at 50 and 99 rolls and direct selection's last word, in area `entropy` (§9.4)

Faraday uses an established implementation wherever one exists for what
it needs, and writes its own only where none exists or none fits its
rules for keys, saying why for each piece. This spec states the rule
(§1), how `just lint` holds it (§2), what Faraday writes itself once the
work here is built (§3), what is checked again later (§6), the documentation
fixes (§7) and what the cross-check compares each action with (§9). The
timing fixes (§4), the switches (§5) and the order of work (§8) are in
`docs/history/ESTABLISHED-LOG.md`. The choices are the
owner's of 2026-10-30 (F178) and 2026-10-31 (F180, F181, F191, F192).

The facts are from a review of the tree at 0.1.9 against crates.io,
docs.rs and the crates' sources on 2026-10-30, and a review of the tree
on 2026-10-31 (main after the timing fixes of §4.2, §4.3 and the
branch-free form of §4.1) that read the code Faraday wrote outside the
manifest's `written_here`, every path a secret takes, every place a
secret is held in text or a growable buffer, the facts the first review
left unchecked, and the cross-check. Where a fact could not
be read from a source, it is marked "not checked", and a fact that does not hold stops the work
that depends on it.

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
container Faraday made first, its own, is gone. The list of what Faraday implements
itself is generated from the manifest (§3), and the website's
/cryptography page reads it from there.

It is not true to say that Faraday uses none of its own cryptography,
or that it implements none.

### 1.4 The text STANDARDS §0 gains

STANDARDS §0 carries this bullet, and the two-sentence summary at its
top, between `standards-summary` markers, is read by the website's
/cryptography:

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
   decision, recorded in DECISIONS. It is the count of §3.
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
are four. The owner's decisions of 2026-10-31 (F180) and 2026-11-05 (F213):

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
  amendments.
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
- **quick-xml** (F213) gets the entry:

  ```toml
  [[not_established]]
  crate = "quick-xml"
  reason = "The tokenizer under the XML of a KDBX vault file. It does no cryptography and has no outside review; its default features are off and it is already in the dependency graph. Which references are refused, the nesting cap and everything else the reader does are Faraday's own."
  decision = "F213"
  ```

  The crate stays an exemption (F71); the entry says so on the website.

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
change is made in the website's own repository with schema 5.

### 2.4 The rule in `tools/impact.toml`

One rule, `crypto.established`, in the crypto area: written with this
spec as `planned`, `active` when `crypto-check` holds it.

- when: `docs/crypto/actions.toml`, `supply-chain/audits.toml`,
  `supply-chain/config.toml`, `supply-chain/imports.lock`,
  `core/*/Cargo.toml`, `faraday/**/Cargo.toml`, `tools/lint-crypto.counts`
- kind: held, by `crypto-check`
- misses: whether a stated reason is true; that a crate of the graph the
  manifest does not name is established (cargo vet holds that it is
  vetted, not that it is established)

`docs/CHECKS.md` is regenerated from it in the same commit
(`python3 -I -B tools/impact.py docs`).

### 2.5 The inventory's name matching

Moved to `docs/history/ESTABLISHED-LOG.md`.

## 3. What Faraday writes itself

The content of STANDARDS §10 and AUDIT §3 once §4, §5 and §7 are built,
before §6. The decision is F178 for the pieces in the tree on
2026-10-30 and F180 for the rest (the column says which). The action is
the manifest entry that carries it; where a piece's action is not given
below, the action is the one whose functions reach it, from the inventory.

### 3.1 Faraday's own specification

| Piece | Where | Action | Decision | Note |
|---|---|---|---|---|
| The session's random generator: its seed and pool, and the mixing of a person's own randomness | `faraday/faraday-core/src/fresh.rs` | `fresh` | F178 | Faraday's own procedure over SHA-256, labelled as not yet reviewed outside Faraday. |
| Dice, coin, card, hex and camera entropy accumulation and packing, and the mix of sources | `core/faraday-entropy` | `dice` | F178 | Faraday's own procedures over SHA-256; the card, camera and mix procedures are labelled as not yet reviewed outside Faraday. |
| PSBT inspection, signing orchestration and verification logic | `core/faraday-psbt` | `psbt-read` | F178 | What Faraday checks before it signs is Faraday's own rule; the PSBT format itself is rust-bitcoin's. |
| The choice among a script's satisfactions by real signature size | `core/faraday-psbt/src/finalize.rs` | `finalize` | F178 | For a miniscript Faraday picks the smallest witness as Bitcoin Core's `finalizepsbt` does; a plain multisig carries the first m signers' signatures, as Core writes them; miniscript builds and checks each witness. |
| BIP-39 entry and display: words typed by pinyin, zhuyin, kana or hangul, the last word's candidates, the explorer's view of the checksum | `core/faraday-bip/src/bip39.rs` | `words-read` | F178 | How a person types and sees the words is Faraday's own. |

### 3.2 No established implementation

| Piece | Where | Action | Decision | Note |
|---|---|---|---|---|
| AEZ v5 and aezeed, with aezeed's CRC-32C | `core/faraday-bip/src/aez.rs`, `core/faraday-bip/src/aezeed.rs` | `aezeed` | F178 | No maintained Rust implementation of AEZ exists; Faraday's follows the Go code LND uses and is tested against LND's vectors. Its AES round is the aes crate's and its BLAKE2b the blake2 crate's. |
| BIP-327 MuSig2, with BIP-328 and BIP-390 | `core/faraday-bip/src/musig.rs` | `musig-key` | F178 | libsecp256k1's MuSig2 is in rust-secp256k1 0.32 and later; bitcoin 0.32, which Faraday uses, is on 0.29, and no miniscript release builds on bitcoin 0.33 yet. |
| BIP-352 silent payments | `core/faraday-bip/src/silent.rs` | `silent-receive` | F178 | libsecp256k1's silent payments module is not yet in its Rust bindings. |
| BIP-374 DLEQ proofs and BIP-375's sending fields | `core/faraday-bip/src/dleq.rs`, `core/faraday-psbt/src/silent.rs` | `silent-send` | F178 | No Rust library implements BIP-374 proofs or BIP-375's PSBT fields. |
| The PSBT version 2 reader | `core/faraday-psbt/src/v2.rs` | `psbt-v2` | F178 | rust-bitcoin reads only version 0 PSBTs; its version 2 crate is pre-release. |
| BIP-322 "simple" message signatures | `core/faraday-psbt/src/message.rs` | `message-sign` | F178 | The bip322 crate is pre-release. |
| SLIP-39 Shamir backup | `core/faraday-bip/src/slip39.rs` | `slip39` | F178 | No maintained Rust implementation of SLIP-39 exists. |
| CMS SignedData, Authenticode's image hash and signature, EFI signature lists and `.auth` files | `faraday/faraday-sb` | `sb-keys` | F178 | No library writes Authenticode signatures or EFI signature lists; the CMS crate for the DER version Faraday uses is a release candidate. |
| BC-UR, with its CBOR (carries) | `core/faraday-codec/src/ur/`, `core/faraday-codec/src/cbor.rs` | `ur` | F178 | No library for this format comes from a publisher Faraday's dependency review trusts. |
| BBQr (carries) | `faraday/faraday-qr/src/bbqr.rs` | `bbqr` | F178 | No library for this format comes from a publisher Faraday's dependency review trusts. |
| SeedQR (carries) | `core/faraday-codec/src/seedqr.rs` | `seedqr-make` | F178 | No Rust library implements SeedQR. |
| The QR encoder, a port of Nayuki's `qrcodegen`, with its GF(256) and Reed-Solomon (carries) | `core/faraday-codec/src/qr.rs` | `qr` | F178 | No QR encoder comes from a publisher Faraday's dependency review trusts; Faraday's port of Nayuki's code wipes a SeedQR's buffers. |
| CRC-32 | `core/faraday-crypto/src/crc32.rs` | `ur` | F180 | The CRC-32 crate in the tree, crc32fast 1.5.1, is not covered by an outside review at that version; Faraday's is a branch-free loop checked against the published check values. |
| FAT16 and FAT32, read and written (carries) | `faraday/faraday-fat` | `stick-write` | F180 | No FAT library comes from a publisher Faraday's dependency review trusts. |
| The FAT32 formatter for the Starter's data partition (carries) | `faraday/faraday-fat` | `starter-write` | F180 | No FAT library comes from a publisher Faraday's dependency review trusts; the partition it makes is checked by dosfstools' `fsck.fat`, against what `mkfs.fat` makes. |
| GPT and MBR tables, read and rewritten (carries) | `faraday/shells/starter/src/gpt.rs`, `faraday/shells/starter/src/ids.rs` | `starter-write` | F180 | No partition-table library comes from a publisher Faraday's dependency review trusts. |
| The PDF writer (carries) | `faraday/faraday-core/src/pdf.rs`, `faraday/faraday-starter/src/sheets.rs` | `starter-recovery-key` | F180 | No PDF library comes from a publisher Faraday's dependency review trusts. |
| The plist reader for the disk tool's output (carries) | `faraday/shells/starter/src/macos/plist.rs` | `starter-write` | F180 | No plist library comes from a publisher Faraday's dependency review trusts. |
| WipeOnFree, the allocator that wipes what it frees | `core/faraday-crypto/src/wipe.rs` | `hold` | F180 | No wiping allocator comes from a publisher Faraday's dependency review trusts; this one is installed in no shipped program (F181). |

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
`carries` tier with the form (§2.1), and the count
below includes it.

### 3.3 One exists but does not fit the key rules

| Piece | Where | Action | Decision | Fails | Note |
|---|---|---|---|---|---|
| scrypt | `core/faraday-crypto/src/scrypt.rs` | `scrypt` | F178 | scrypt 0.12.0 frees its passphrase-derived working buffers (`b`, `v`, `t`) without wiping them | The Rust scrypt library leaves its passphrase-derived working memory unwiped; Faraday's wipes it. |
| HKDF-SHA256, one copy | `core/faraday-crypto/src/hash.rs` | `vault-qr-pgp` | F180 | hkdf 0.13.0 returns the pseudorandom key and leaves each output block T(i) in plain arrays it does not wipe | The Rust HKDF library leaves the pseudorandom key and its output blocks unwiped; Faraday's holds both in buffers it wipes. |
| BIP 93 codex32 | `core/faraday-bip/src/codex32/` | `codex32` | F178 | codex32 0.1.0 holds the seed in a `String` that is `Clone` and `Debug` and never wiped, which `tools/lint-secrets.sh` refuses, and needs `std`, which `faraday-bip` does not have | The reference library keeps the seed in text that is never wiped and needs a full operating system. |
| BIP-39 mnemonics: entropy to words and back, the checksum, a word's place in its list | `core/faraday-bip/src/bip39.rs` | `words-read` | F180 | bip39 2.2.2 finds a word by a binary search or a scan that stops at it (`find_word`) and packs entropy with a branch on each bit (`from_entropy_in`), so the time it takes follows the secret words | The Rust BIP-39 library's word lookup takes a time that follows the secret words; Faraday's looks each word up in constant time and is tested against that library and the reference. |
| The BIP-39 seed, PBKDF2-HMAC-SHA512 over the sentence through `faraday-crypto` | `core/faraday-bip/src/bip39.rs` | `seed` | F178 | bip39's `to_seed` returns the seed as a plain `[u8; 64]`, and its own PBKDF2 leaves its HMAC state unwiped (bip39 2.2.2's `src/pbkdf2.rs`) | The seed is derived with the RustCrypto PBKDF2 Faraday uses everywhere, into a buffer that is wiped. |
| The PNG writer, with zlib's stored blocks (carries) | `core/faraday-codec/src/png.rs` | `seedqr-make` | F180 | png 0.18.1 needs `std`, which `faraday-codec` does not have, and does not wipe a picture's rows | Faraday writes a SeedQR's picture in buffers it wipes; its Adler-32 is the adler2 crate's. |

### 3.4 Only the part used is written

| Piece | Where | Action | Decision | Note |
|---|---|---|---|---|
| OpenPGP v4 packets for one key shape, the armour's CRC-24, and the paperkey format | `faraday/faraday-pgp` | `gpg-create` | F178 | Faraday writes only the packets of its one key shape, over Ed25519 from dalek and SHA-1/SHA-2 from RustCrypto, each judged by GnuPG and paperkey; the armour's base64 is the base64ct crate's. |
| TOTP code derivation | `faraday/faraday-vault/src/totp.rs` | `totp` | F178 | Faraday writes only the few lines RFC 6238 needs, over standard HMAC. |
| Seed XOR | `core/faraday-entropy/src/xor.rs` | `xor` | F178 | Faraday writes only the few lines the standard needs, over standard XOR. |
| A 32-byte hash reduced modulo the curve order, without a branch | `core/faraday-bip/src/musig.rs` | `musig-key` | F178 | libsecp256k1's Rust bindings take a scalar only below the order and offer no reduction; Faraday's is a single conditional subtraction. |
| The JSON visitor that refuses a key named twice and counts depth | `core/faraday-bip/src/json.rs` | `wallet-files` | F180 | serde_json reads the text; Faraday refuses a key named twice and a nesting deeper than 32. |
| The form of a version 4 UUID (carries) | `faraday/shells/starter/src/ids.rs` | `starter-write` | F180 | Four lines setting RFC 9562's version and variant bits over bytes from getrandom. |
| The KDBX 4 container a file vault is: header, VariantDictionary, HMAC block stream, inner header and gzip framing | `faraday/faraday-vault/src/kdbx/` | `vault-kdbx` | F190 | Faraday writes only the part of KeePass's format the vault uses, over argon2, chacha20, aes and cbc, hmac, sha2 and miniz_oxide; keepass-rs, which no review Faraday trusts covers, drops the XML elements it does not read and draws a save's randomness itself. |
| The QR vault's OpenPGP message: the version 6 symmetric-key packet with the Argon2 S2K, the version 2 data packet and the literal data packet | `faraday/faraday-pgp/src/message.rs` | `vault-qr-pgp` | F190 | Faraday writes only the three packets a QR vault uses, over argon2, aes and ocb3 and Faraday's HKDF; rPGP, which no review Faraday trusts covers, brings every OpenPGP algorithm. |
| The TOML tree inside a vault: duplicates, limits and the names of `docs/VAULT.md` §2.1 and §7 | `faraday/faraday-vault/src/contents.rs`, `faraday/faraday-vault/src/contents/` | `vault-contents` | F189, F190 | toml_parser reads the tokens; Faraday builds the tree and refuses what VAULT.md does not name, as the JSON visitor does over serde_json. |

The KDBX 4 export, which stood in none of these tables because the
vault was to take it in, is folded into the vault (F190 point 10): an
export is a vault of one entry, written by the vault's own container
(`vault-kdbx`), and its separate writer is gone. The four-equal-slot FVLT
container that stood beside it is gone.

Thirty-nine pieces in all, the formatter included (five of Faraday's own
specification, nineteen with no established implementation, six that do not
fit and nine of which only the part used is written), the manifest's
`written_here` count. The switches
remove BLAKE2b, the AES round, the scalar arithmetic on secrets, the
RSA RNG adapter, X.509 and the DER writer, the single-key and multisig
finalizer, the sealing's counter nonce, four copies of base64, about
twenty of hex, the descriptor checksum, the JSON parser, Adler-32, the
hand-written wipes, and the second copy of HKDF's extract and of the
Argon2 call; the encodings, BIP-322 and the formats written here and
never listed are listed. Each kept piece is in one copy (§5.17).

## 4. The timing fixes, and what wiping reaches

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.1 F-1: scalar arithmetic on secrets

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.2 F-2: AEZ's AES round (built)

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.3 F-3: SLIP-39's GF(256) and checksum (built)

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.4 F-4: codex32's GF(32), checksum and alphabet

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.5 F-5: word lookups

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.6 F-6: comparisons of secrets

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.7 F-7: BIP-352 tweaks, and contexts that are not randomized

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.8 Copies of secrets in freed memory

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.9 A wiping allocator

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 4.10 Display-only and input-only

Moved to `docs/history/ESTABLISHED-LOG.md`.

## 5. The switches

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.1 BLAKE2b

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.2 HKDF: not a switch

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.3 AEZ's AES round (built)

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.4 Scalar arithmetic

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.5 The BIP-39 core: not a switch

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.6 The RSA RNG adapter

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.7 X.509 and DER

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.8 The single-key and multisig finalizer

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.9 The sealing's counter nonce, removed

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.10 Base64, four copies

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.11 Hex, one copy

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.12 The descriptor checksum

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.13 The JSON reader

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.14 Adler-32

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.15 The zero check, the wipes, one Argon2id call

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.16 The quiz's seed

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 5.17 One copy of each kept piece

Moved to `docs/history/ESTABLISHED-LOG.md`.

## 6. Check again later

Each item here is kept written here (§3) until its condition holds; the
switch is then made separately, with a decision only where §1 needs
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

All are made, ahead of the switches, but §7.2's MuSig2 text is right
only once the scalar switch (§4.1) is made.

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
compare it with" (true now that the lookups are constant time; before: "the bip39 crate's word
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
notes and the website's reason lines say so.

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

Moved to `docs/history/ESTABLISHED-LOG.md`.

## 9. The cross-check's outside sources

Decided (owner, 2026-10-31; F191, F192). This section says what the
cross-check (F79, `docs/CROSSCHECK.md` §10, `tools/crosscheck/README.md`)
compares each action with once the switches of §4 and §5 are made: the
rules that choose (§9.2), the places where a check would compare a
library with itself and how each is cleared (§9.3), each action's
judges before and after (§9.4), what is not taken and why (§9.5), the
tools and packages it adds (§9.6), one choice
withdrawn (§9.8). A library the app uses cannot be its own cross-check, so a
piece that moves to a library needs a judge other than that library;
the list is made now that the switches are fixed (F180 point 8).

The facts are from a review of main on 2026-10-31 that read every
action's manifest entry, every area's docstring, the vendored
references (btclib's wheels opened, `tools/reference/lnd/`'s and
`tools/reference/btcd-bip322/`'s vendored modules listed) and the
outside candidates' sources. What could not be read from a source is
marked "not checked"; it was not checked; where it does not hold, the action keeps its
present judge and says so.

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
5. **Hex through base16ct is checked against outside code.** Once `RawHex`
   decodes through base16ct, the `entropy` area's comparison with
   CPython's `bytes.fromhex` is others' code against a library, and
   `coins-hex` says so; the flips' packing and the commitments stay
   Faraday's second version.
6. **A switch lands with its cross-check change** (F191): in the same
   commit and the same passing `just crosscheck`, so no commit has a
   piece judged by the library it moved to, or an action whose judge
   was dropped and not replaced. Its cross-check change is named in §8.

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

| Where | What is compared with itself | Cleared by |
|---|---|---|
| `sign-ecdsa`, `sign-schnorr`, `message-sign`, `message-verify` | Core's signatures and verification are libsecp256k1's, as the app's are | python-ecdsa and btclib's own curve, already beside Core (`tools/crosscheck/lineage.py`); the manifest names Core as sharing the curve maths |
| `sweep-sign`, `wif-read` | Core's signatures and the WIF's public key are libsecp256k1's | python-ecdsa's first-nonce signature byte for byte and btclib's signing and finalizing for sweeps; btclib's WIF decoding and public key |
| `vanity` | Core's `deriveaddresses` derives each candidate's child keys with libsecp256k1 | the child keys by btclib on its own curve; Core judges the script and the address text only; a silent payment code is written by the BIP-352 reference's bech32m |
| `sign-check` (new area) | Core's `finalizepsbt` checks signatures with libsecp256k1 | python-ecdsa and btclib verify every signature, partial ones too; Core judges script and consensus on complete inputs |
| `sign-musig`, once its scalars are libsecp256k1's | the Core round trip's partial signatures share libsecp256k1's scalar code with those of the other scalar code | btclib's MuSig2 as a full reference beside BIP-327's; the round trip stays as a judge of BIP-373 and the descriptor, named as sharing the curve |
| `musig-key` | Core's `musig()` aggregation is libsecp256k1's musig module | BIP-327's reference and btclib's key aggregation judge the maths; Core judges BIP-390's descriptors and addresses |
| `qr`, `codex32qr`, `seedqr-make` | the app's encoder is a port of Nayuki's, compared with Nayuki's (rule 3) | segno, another encoder lineage (built); zbar and zxing-cpp decode every picture (built) |
| `tools-hash` | `base58check_round_trips_through_bitcoins_own_encoder` in `core/faraday-codec/src/encodings.rs` compares `bitcoin::base58::encode_check` with itself | Built: that assertion is replaced by a published Base58Check vector (the first valid address of Bitcoin Core's `key_io_valid.json`); the new `hash` area judges Base58Check with Core's framework |
| `aezeed` | not a library: on a builder with AES-NI both sides run the processor's round, so the fixsliced round the Pi runs is never compared | a second pass with the aes crate's software round and LND's AEZ built without assembly (built) |
| btclib's ports of Core | `script/solver.py` and `script/spendability.py` are Core's logic | not counted as a second lineage beside Core; where only they cover a rule, Faraday's second version stays (rule 1) |

No switch of §5 makes an existing judge collide: the switches to BLAKE2b, X.509 and DER, base64, hex, the JSON reader, Adler-32 and the Argon2id call move pieces onto RustCrypto crates, `serde_json`, `adler2`
and miniscript, and their judges are Go's x/crypto, OpenSSL, GnuPG,
Core's C++, CPython's `base64`, `binascii`, `json` and `zlib`, and
pykeepass with its argon2-cffi (and, once F190's vault is judged,
`keepassxc-cli` and Sequoia's `sq`). The scalar switch and the BIP-352 tweaks put more of the app on
libsecp256k1; the BIPs' references run over secp256k1lab and btclib on
its own curve, neither libsecp256k1, so they stay valid.

### 9.4 Each action

Faraday's side is the code after §4 and §5. "Kept" is the judges in use
today; "Added" names what was added; "Second version" is what of Faraday's
second version is retired or kept under rule 1; "Class" is the
`independence` class after the cross-check is extended (none: no area, judged in tests or
not at all).

| Action | Faraday's side | Kept | Added | Second version | Class |
|---|---|---|---|---|---|
| words-read, words-make, seed | Own BIP-39 core, constant time; pbkdf2, sha2, unicode-normalization | python-mnemonic | bip39_slip39 as a second reader | none | outside |
| slip39 | Own; GF(256) and RS1024 without a branch (built) | python-shamir-mnemonic | bip39_slip39 | none | outside |
| bip85 | bitcoin and Faraday's glue; base64ct, base16ct | btclib-wallet's `bip85`, ethankosakovsky/bip85 | btclib's `bip85` run as written over bip39_slip39, python-mnemonic's stand-in removed | none | outside |
| master | bitcoin and libsecp256k1; the strict decoder Faraday's | btclib-wallet's `bip32` | btcd's `hdkeychain` from btcd v0.27.0 (built) | kept: `hdkeychain` does not refuse what Faraday refuses. It reads a key of any version, a version of the other kind than the key's data, and a depth-0 key with a parent fingerprint or an index; those three rules stay Faraday's second version, and `hdkeychain` is asked of every other refusal (the checksum, the length, a scalar of 0 or past n, a public key's prefix or a point off the curve) and of every valid key | outside-and-own |
| derive, xprv-shown | bitcoin and libsecp256k1 | btclib-wallet's `bip32` | `hdkeychain` from btcd v0.27.0 (built) | none | outside |
| dice | Own specification; sha2 | Coldcard's `rolls.py`, `rolls12.py` | SeedSigner's `mnemonic_generation.py` over embit's BIP-39 (built): the hashed procedure at 50 and 99 rolls, and direct selection's last word, from SeedSigner's `calculate_checksum` at 12 and 24 words and, where it has no step (it completes 12 and 24 words only, and cuts its dice hash to 16 bytes only at exactly 50 rolls), embit's checksum through `generate_mnemonic_from_bytes` at 15, 18 and 21, compared as the entropy embit reads back from the words, the checksum itself being area `bip39`'s; the wordlist indices from faces are Faraday's, SeedSigner has no dice selection | kept: six-as-zero, the cuts to 20, 24 and 28 bytes, the counts and refused faces, which no outside code has without Node | outside-and-own |
| coins-hex | Own specification; hex through base16ct | Faraday's second version | CPython's `bytes.fromhex` named as the hex judge (rule 5) | kept for the flips and the commitments | outside-and-own |
| vanity | bitcoin and libsecp256k1 | python-mnemonic, btclib, Core | each candidate's child keys by btclib; Core for the script and address text only, and a silent payment code by the BIP-352 reference's bech32m (built) | kept: the counter, the prefix rules, the expected work, the search loop | outside-and-own |
| aezeed | Own; aes (built), blake2 | LND's `aezeed` | a pass with the software AES round on both sides: the aes crate's `aes_backend="soft"` against Yawning Angel's AEZ built with the tags `noasm` and `purego` (built) | none | outside |
| silent-receive | Own; secret scalars through libsecp256k1, the label tweak through `ecmult_gen` | BIP-352's reference over secp256k1lab | btclib-wallet's `silent_payments.py` | none | outside |
| silent-send | Own; secret scalars through libsecp256k1, the label tweak through `ecmult_gen` | BIP-374's and BIP-375's references | btclib's `dleq.py`, BIP-375 roles and BIP-352 | none | outside |
| musig-key | Own until bitcoin 0.33 (§6.1); point operations libsecp256k1's | BIP-327's reference; btclib verifies the aggregate | btclib's key aggregation and tweaks as a full reference; Core's `musig()` descriptors through `deriveaddresses` | none | outside |
| sign-musig | Own; secret scalars through libsecp256k1 | BIP-327's reference; btclib verifies; Core's round trip (`other`) | btclib's MuSig2 signing and BIP-373's PSBT roles | none | outside |
| codex32, seed-shown | Own; GF(32), checksum and alphabet without a branch | python-codex32 | rust-codex32 at a pinned commit, a program outside the workspace | the threshold rule (0 or 2 to 9) and threshold 0's share index `s` retired to rust-codex32; the seed's lengths and the share order kept | outside-and-own |
| codex32-make | Own; GF(32), checksum and alphabet without a branch | python-codex32 | volvelle-website's `checksum_worksheet.rs` run as a program over every corpus worksheet of 48 and 74 characters; rust-codex32's `interpolate_at` for a fresh set's derived shares | the rows at 48 and 74 characters retired; at 127 kept | outside-and-own |
| qr, codex32qr | Own encoder (Nayuki's, ported) and PNG writer, Adler-32 from adler2; rqrr reads | Nayuki's `qrcodegen.py` (same lineage, rule 3) | segno as a second encoder, module for module at a given mask and at its own mask choice; segno's and Nayuki's codes rendered and read by rqrr; zbar and zxing-cpp decode every picture (built: area `seedqr`; segno 1.6 module for module at Faraday's mask and at another, and at its own choice as Faraday's code or the same data under the mask it reports, its zero pad codeword where the standard adds none modelled in Nayuki's code and counted; `qr_decode.py` for the Codex32QR pictures Faraday writes, the profile's 22 pictures, and each key's SeedQR and CompactSeedQR picture, payloads compared as bytes) | none | outside |
| seedqr-make | Own | SeedSigner, Nayuki | each PNG read by Pillow and zlib; segno (built, with the QR rows above); zbar and zxing-cpp decode each picture Faraday writes of a key's SeedQR and CompactSeedQR, payloads compared as bytes (built) | kept: CompactSeedQR at 15, 18 and 21 words, which no outside code defines | outside-and-own |
| seedqr-read | Own; the comparison through subtle | SeedSigner's `DecodeQR` | zbar and zxing-cpp decode the pictures; Faraday's reader takes what they return and gives the key's words (built) | kept: Faraday's reader rule, 15, 18 and 21 words | outside-and-own |
| stick-write, outbox | Own text lines, FAT writer and PNG writer | FAT read by mtools and dosfstools in tests (required in `just crosscheck`) | each PNG read by Pillow and zlib; each picture decoded by zbar and zxing-cpp and its payload compared with the key's words (built: the stick's labelled picture of the key's SeedQR and CompactSeedQR and of a codex32 share), in area `seedqr` | none | outside |
| ur | Own; one CRC-32 and one CBOR | foundation-ur-py | `zlib.crc32` against the one CRC-32; cbor2 for the registry types' and the parts' CBOR (built: `zlib.crc32` over random messages and the parts' checksum field; cbor2 reading the parts, the single-part payloads and the registry types built with it, `account_cbor`'s output, and random and mutated CBOR beside Faraday's reader, in `tools/crosscheck/ur_cbor.py`) | none | outside |
| bbqr | Own; miniz_oxide; hex through base16ct | Coinkite's BBQr Python, SeedSigner's numbered-part reader | krux's `qr.py` and `bbqr.py` (v26.09.0) writing numbered parts and reading them; Coinkite's test files as vectors (built: `tools/reference/krux/`, `tools/vectors/bbqr/`) | the writing of numbered parts retired to krux; the refusals and the capacity shim kept | outside-and-own |
| hashes-macs | sha2, hmac, pbkdf2 | python-mnemonic, `hashlib` | Built, in area `hash`: `hmac` and `hashlib.pbkdf2_hmac` over random HMAC and PBKDF2, each over SHA-256 and SHA-512, RFC 4231's first case for each hash and RFC 7914's PBKDF2 vector (rounds 1), HMAC keys of 0 to 300 bytes across both block sizes, PBKDF2 passwords and salts to 100 bytes, rounds to 50 and lengths to 200, asked only within the callers' domain (rounds and length at least 1) | none | outside |
| hkdf | HKDF over SHA-256 Faraday's, one copy | PyCA's `cryptography` | PyCA's `HKDFExpand` over random keys, info and lengths up to 255 blocks, and PyCA's `HKDF` over random salts (none, empty and up to 100 bytes), keys and infos (built, in area `hash`: RFC 5869's first case, then lengths from 0 to 255 blocks and one past, both refused) | none | outside |
| scrypt | Own (§3.3) | LND, over Go's x/crypto | Built, in area `hash`: `hashlib.scrypt` (OpenSSL) over random N from 2 to 4096, r, p and lengths, RFC 7914's two cheap vectors, and N not a power of two, N 0 or 1, r 0, p 0 and an empty output refused by both; LND's `aezeed` in area `lightning` stays beside it | none | outside |
| tools-hash, file-hash | bitcoin (Base58, bech32, RIPEMD-160), sha2; hex through base16ct | none (published vectors in tests) | Built: area `hash`: Core's framework's `ripemd160.py`, `address.py` and `segwit_addr.py`, `hashlib`, `bytes.fromhex`, SHA-256 of random files across block boundaries (the file hash reads no buffer: it hashes the whole slice); the self-comparing Base58Check assertion replaced. Two stated differences, for bech32 text that is not a segwit address: Faraday reads all its data groups and drops the bits left over, and reads it longer than 90 characters; plain Base58 is read by the area's own lines | none | outside |
| address, descriptor | miniscript (its checksum), bitcoin | Core, BIP-380's checksum code, python-bip380, btclib's keys | btclib-wallet's descriptors and miniscript: the script at an index, the parse, the type, the round trip (built) | none | outside |
| recovery | As descriptor | As descriptor | As descriptor, and btclib's satisfaction of each path under its lock times (built) | none | outside |
| tapmulti | bitcoin, miniscript | Core's framework for the trees | btclib's `check_output_pubkey` (built); btcd v0.27.0's `ParseControlBlock` and `VerifyTaprootLeafCommitment` (built); neither hashes a leaf of version 0x50 differently from any other, so that rule stays the area's own | retired but the rule that a leaf version is not 0x50: BIP-341's script-path check written in the area is gone, and Core's framework is no longer asked about a control block | outside-and-own |
| psbt-read | bitcoin, base64ct; the inspection Faraday's | Core, HWI | btclib-wallet's `Psbt.parse`, with its validity check (built) | retired: the 65-byte signature with sighash byte 0 is refused by btclib, checked; btclib reads some PSBTs Core and Faraday refuse (a key with key data on a field that takes none), so it is not asked to refuse what Core refuses | outside |
| psbt-v2 | Own (§6.4) | HWI, Core for both version 0 forms | btclib-wallet's version 2 reader and its conversions to and from version 0 (built) | retired: the refusals of bytes after the last map, of a stream ending inside a map (checked: btclib refuses it) and of a 65-byte taproot signature with sighash byte 0; kept: the bridge's pair choice and the version 0 PSBT where HWI repairs a short value | outside-and-own |
| sighash-review | bitcoin | Core's framework | btclib's `sig_hash` (`legacy`, `segwit_v0`, `taproot`), every hash beside the framework's (built) | retired: BIP-341's refusals (SIGHASH_SINGLE without an output, a hash type it does not list) are btclib's, and the framework, which hashes those anyway, is not asked | outside |
| tx-decode | bitcoin; hex through base16ct | Core's framework | btclib's `Tx.parse`; Core's `decoderawtransaction` on the regtest node, with the known difference where bytes also read as a legacy transaction of no inputs (built) | retired but the refusal of a transaction with no inputs, which neither judge makes; the framework's reading and the exact-serialisation rule written for it are gone | outside-and-own |
| psbt-review | bitcoin, miniscript; the change rule Faraday's | Core's `decodepsbt`, `analyzepsbt`, `deriveaddresses` | btclib's `update_psbt_output`, `index_of` and descriptor wallet; Core's watch-only descriptor wallet, `getaddressinfo` and `walletprocesspsbt` (built) | the change rule retired (built); the registered-wallet rule kept | outside-and-own |
| wallet-files | bitcoin; serde_json behind Faraday's visitor; base64ct | Core, btclib | CPython's `json` reading every file the area writes and refusing a duplicate key and a depth over 32; every real wallet file the wallets' published test suites hold (built: 93 files of Sparrow, BlueWallet, Coldcard, Caravan and Liana, each read by Faraday and by the area's reading of its format, judged by Core, or refused by both) | kept for the formats no outside reader covers; the refusing hook is Faraday's second version over CPython's parser | outside-and-own |
| finalize | miniscript, and Faraday's choice of the smallest witness | Core's `finalizepsbt`, `testmempoolaccept` | btclib's `psbt.finalize` with its miniscript solver, which also chooses among satisfactions (built; the leaf of a taproot input and a plain multisig's signatures stay Core's to judge) | kept: the taproot `pkh` leaf left off, the fields BIP-174 keeps | outside-and-own |
| sign-ecdsa, sign-schnorr | libsecp256k1, bitcoin; Faraday's refusals | Core, python-ecdsa, btclib, BIP-340's reference | Core named as sharing the curve maths (built) | kept: F136's and F141's refusals, the inputs a wallet's rule leaves unsigned | outside-and-own |
| message-sign, message-verify | libsecp256k1; BIP-322 own (§3.2); base64ct | Core, btclib, btcd's `bip322`, python-ecdsa | Core named as sharing the curve maths (built) | none | outside |
| sweep-sign | libsecp256k1, bitcoin, miniscript's finalizer | Core, BIP-143's example | python-ecdsa's first-nonce signature byte for byte; btclib's signing and finalizing (built) | none | outside |
| wif-read | libsecp256k1, bitcoin | Core | btclib's WIF decoding and public key (built) | none | outside |
| sign-check | libsecp256k1 (verification), bitcoin (sighash) | none | a new area (built): python-ecdsa and btclib verify every signature on a PSBT, partial or in a finalized witness, over btclib's sighashes; btclib's script engine runs the complete inputs; Core's `finalizepsbt` and `testmempoolaccept` judge them and the transaction. Core's rules, as the area found them: a partial input names one sighash type, so its PSBT sighash-type field is written when its signatures share a non-ALL type, and an input with mixed types is not asked of Core; an input that arrives final is taken as it is, so its signatures are judged by `testmempoolaccept` on the extracted transaction, not by `finalizepsbt`'s `complete`. Known: a partial signature that is not well formed is refused by Faraday's reader where Core reads it and leaves the input unfinalized; and a bug in Faraday held until it is fixed: a finalized taproot signature of 65 bytes ending in 0x00 is called valid where BIP 341 and Core call it malformed | the report of a reused r or R, written from Faraday's own rule, failing closed (signatures that verify with one nonce, differing in key or in message) | outside-and-own |
| compile | miniscript's policy compiler | none | a new area (built): Core's `getdescriptorinfo` and `deriveaddresses` take the output; python-bip380 and btclib parse it; the policy and the parsed miniscript compared as boolean formulas over the keys and lock times. Known: a single key under `tr()` is refused by the compiler, and python-bip380's `v:multi_a` script differs from BIP 379's | the lift of a miniscript to a formula, written from the fragments' meanings, and the rules by which a policy is refused | outside-and-own |
| sb-keys | rsa, sha2, chacha20, x509-cert and der; CMS, signature lists and `.auth` Faraday's | OpenSSL (tests) | GnuTLS's `certtool` reads every certificate and verifies its self-signature; the OVMF run (`faraday/tools/sb-ovmf-check.py`) named in `other` and run by `just crosscheck` | none | none (tests) |
| sb-sign | rsa, sha2, x509-cert and der; Authenticode and CMS Faraday's | sbverify, OpenSSL (tests) | the OVMF run; pesign's `pesigcheck` (NSS) does not take the db certificate as a signer (§5.7), so it judges the real signed binaries (sb-check) and not the images made here | none | none (tests) |
| sb-check | x509-cert, der | sbverify; images signed by `sbsign` and `osslsigncode` | `pesigcheck` on two real signed binaries, each under 2 MiB, their signer and image hash named and compared with `pesign`'s; the OVMF run | none | none (tests) |
| starter-signed | rsa, sha2; the PE checksum and certificate table Faraday's | sbverify (tests) | python3-pefile's PE checksum field and certificate-table entry, compared with the image's header for every image signed, moved or real | none | none (tests) |
| gpg-create, gpg-sign, gpg-verify | Own packets over ed25519-dalek; base64ct | GnuPG 2.4 (tests) | Sequoia's `sq` 1.3.1, built over Nettle (the tests and the cross-check's tools check fail unless `sq version` says so), inspects every certificate Faraday writes (both keys, the user ID, the expiry, the renewal), takes the revocation certificate and verifies each detached signature over SHA-256 and SHA-512, refusing one over other data; a signature `sq` makes (over SHA-512, the one hash `sq sign` offers) and its key are among gpg-verify's fixtures (`tools/vectors/release-sums/sequoia`) (built) | none | none (tests) |
| kdbx | The export is a one-entry vault, written by the vault's own container (`vault-kdbx`, F190 point 10); argon2 in one call, chacha20, hmac, sha2 | pykeepass | area `vault`: the export's function (`file::create_holding`) and its vector are read by pykeepass, `keepassxc-cli` and keepass-rs, a changed byte refused | none | outside |
| totp | Own (§3.4) | none (the RFCs' vectors in tests) | Built (area `totp`): pyotp's `TOTP.at` and `generate_otp` over RFC 6238 Appendix B's vectors and random keys of 1 to 300 bytes, times from 0 to 2^64 - 1 with the edges of each period, digits 6 to 8, periods and the three hashes (the code, the seconds left, the period), and `parse_uri` over random and damaged otpauth URIs. Faraday's own rules are stated differences, each pinned one URI at a time and judged on the URI with the difference taken out: it reads what pyotp refuses (a scheme, type or algorithm in other case, a parameter name or a `counter` pyotp does not take, a label issuer the parameter does not repeat, Base32 with spaces, hyphens or misplaced `=`) and refuses what pyotp reads (a period over 3600 seconds, a parameter of the key URI written twice, in another case or with a percent-escape in its name, a parameter name with a tab or line break, a pair with no value, a type with no label slash, percent-escaped values, a fragment, a bare key under 16 characters, `encoder=steam`, which pyotp reads as Steam's five-character code); HOTP is named as HOTP by Faraday and a HOTP object by pyotp. `from_fields` and `normalise` have no pyotp reader and stay with the crate's tests | none | outside |
| vault-create, vault-unlock, vault-seal | The file vault over the KDBX container (`vault-kdbx`); argon2 in one call | argon2-cffi for Argon2id, Python's `unicodedata` for NFC | the payload layouts of VAULT §7 are the area's own; area `vault` writes the file with Faraday and reads it with pykeepass and `keepassxc-cli` (every field, group, attachment and custom data item, a wrong passphrase and a changed byte refused), and has them write files Faraday opens and seals again (built), and keepass-rs reads the files Faraday writes and writes files Faraday opens, for the format only (built) | `area_vault.py`, `tools/vault/open.py` over pykeepass | outside-and-own |
| vault-kdbx | The KDBX 4 container written here (§3.4); argon2 in one call, chacha20, aes, cbc, hmac, sha2, miniz_oxide | none: pykeepass's files read in tests | area `vault`: pykeepass and `keepassxc-cli` both ways, built, each opening the files Faraday made and Faraday opening theirs (F190 point 11, `docs/VAULT.md` §10); keepass-rs for the format only (built: reads Faraday's files, and writes files Faraday opens and seals, with Twofish, AES-KDF and Salsa20 files refused as known differences); PyCA's Argon2id as a second Argon2 lineage | none | outside |
| vault-qr-pgp | The RFC 9580 message written here (§3.4); argon2 in one call, aes, ocb3; HKDF Faraday's, one copy | none: RFC 9580's vectors in tests | area `qrvault` (built): Debian's `sq`, over Nettle, judges the primitives except Argon2, which it shares with the app's lineage; rPGP judges the format only; PyCA's Argon2id, HKDF-SHA-256 and AES-256-OCB over OpenSSL judge the primitives, under the area's own framing; `sq`, rPGP and PyCA open Faraday's seals and rPGP's and PyCA's seals are opened by Faraday (F190 point 11, VAULT §10); pysequoia once its backend is checked and OpenPGP.js when Node is taken stay out | the OpenPGP framing around PyCA's primitives and the QR TOML (`qr_toml`), both written in the area (`area_qrvault.py`) from RFC 9580 and `docs/VAULT.md` §1.2 and §2.1 | outside-and-own |
| vault-contents | The TOML tree over toml_parser (§3.4) | none: toml-test's cases in tests | CPython's `tomllib` reads every vault's TOML tree beside Faraday's reader, in area `vault`; area `qrvault` compares the QR form's writer byte for byte and its reader by item | none | outside |
| starter-write | lzma-rust2, sha2, getrandom; GPT and MBR, the FAT32 formatter, the plist reader, the UUID form and CRC-32 Faraday's | `sha256sum`, `xz` as encoder, `sfdisk` and dosfstools (tests; required in `just crosscheck`) | built (`faraday/shells/starter/tests/all/outside.rs`): `xz` decoding good and corrupted streams beside lzma-rust2, 7-Zip (`7zz` or `7z`, upstream 7-Zip, not p7zip) as a second decoder lineage, `sgdisk -v` beside `sfdisk --verify` on the GPT and MBR tables written, `plistlib` over the `diskutil` fixtures, `uuid` on the IDs drawn, `zlib.crc32` over the GPT header and partition array against their CRC fields (built) | none | none (tests) |

Unchanged:

- Faraday's own specifications, checked by Faraday's second version
  only, with no outside implementation: cards, camera, pointer-keys,
  mix, device, fresh, booklet, roller (class `own`); vault-edit's
  records, which run no primitive and take the vault's class (KeePass
  entries).
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

Now, 73 of the 94 actions of 2026-10-31 name an area: 37
`outside`, 28 `outside-and-own` (master's version rule is kept; tx-decode's no-inputs refusal), 8 `own`; 21 name none, of
which 11 are judged by outside programs in tests. The vault's actions are counted there as they were, four in
area `vault` as `outside-and-own`; once F190's vault is built and judged
(VAULT §10), vault-kdbx joins area `vault` as `outside`, vault-qr-pgp is in area
`qrvault` as `outside-and-own` (the OpenPGP framing around PyCA's
primitives is Faraday's second version), vault-contents is not cross-checked as an action (area `qrvault` compares the QR
form's writer byte for byte and its reader by item, `tomllib` reading the canonical
form the area writes), and
the class of the KDBX ones is not checked until that build says what of
Faraday's second version stays.

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
| embit beyond its BIP-39; Specter's importer over it | key derivation, PSBT, wallet files | embit loads libsecp256k1 through ctypes whenever it can, with no switch; taken in full, with that binding removed and its absence asserted, only with Specter's importer, which waited for wallet-files to hold more real files (built); not taken |
| quirc, quircs, OpenCV's QR decoder | reading pictures | rqrr is built on quirc (rule 3) |
| Sequoia over RustCrypto; keepass-rs | GPG; KDBX | RustCrypto's primitives, the app's (Debian's `sq`, over Nettle, is taken). F190 point 11, of 2026-11-02, names keepass-rs and rPGP (over RustCrypto too, not checked for keepass-rs's Argon2) as the vault's test-only judges, beside pysequoia (its backend not checked) and OpenPGP.js (Node): settled by F193, keepass-rs (built) and rPGP (not built) judge the format only, pysequoia counts once its backend is checked, OpenPGP.js waits for Node |
| anything over rust-bitcoin or rust-miniscript: BDK, Liana, the `bip322` crate, rust-psbt's `psbt-v2` | scripts, PSBT, BIP-322 | the app's libraries |
| sipa's C++ miniscript compiler | compile | it chooses differently by design, so only validity and equivalence could be compared, which the `compile` area does |
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
and required by `just crosscheck` (rule 4).
Package names are Debian's and Ubuntu's where they were read; where
they were not, they were not checked.

| Tool | Distribution packages | Vendored in `tools/reference/` (commit and SHA-256) |
|---|---|---|
| FAT, partition and OVMF checks | mtools, dosfstools, util-linux's `sfdisk`, QEMU and OVMF's Secure Boot firmware (for the OVMF run) | none |
| Secure Boot and PE checks | `gnutls-bin` (`certtool`), `pesign` (`pesigcheck`), `python3-pefile` | two real signed EFI binaries, each under 2 MiB (`tools/reference/signed-efi/`: Ubuntu's shim-signed 1.59+15.8-0ubuntu2, with its source, licence and SHA-256) |
| PNG reading | `python3-pil` (Pillow) | none |
| HKDF and OpenSSL-backed checks | `python3-cryptography` 44 or later, over OpenSSL 3.2 or later (checked on the build machine, 2026-11-03: 46.0.5 over OpenSSL 3.5.5) | none |
| codex32 | none | rust-codex32 at commit `3943cb52b2343333386fac0cd38b8ae5a4d6de02` (CC0 1.0; its `Cargo.toml` has no dependency), built offline outside the workspace; volvelle's worksheet program is in `tools/vectors/codex32-volvelle/` already (CC0 1.0) |
| BIP-39 and SLIP-39 | none | the bip39_slip39 wheel, 2026.10.24, beside btclib's |
| UR and CBOR | `python3-cbor2`; PyCA's `cryptography`, already named for the OpenSSL-backed checks, judges the HKDF | none |
| btcd | none | a released btcd's modules (`btcutil`, `txscript`, `btcec` and what they name; the root daemon module is not used), `go mod vendor`, in a folder of their own; built |
| QR | `python3-segno`, `zbar-tools`, `python3-zxing-cpp` | none |
| SeedQR | none | SeedSigner's `helpers/mnemonic_generation.py`, at the commit `tools/reference/seedsigner/` has |
| BBQr | none | krux's `src/krux/qr.py` and `bbqr.py` at v26.09.0, with three shims (`tools/reference/krux/`); Coinkite's BBQr test files (`tools/vectors/bbqr/`) (built) |
| OpenPGP | `sq` (Sequoia), built over Nettle: the build's backend is checked first (built: `tools/crosscheck/judges.py` and the GPG tests) | none |
| TOTP | `python3-pyotp` | none |
| Starter image | `xz-utils`, `7zip` (7-Zip's `7zz`, or `7z` where the system names it so; the banner must read 7-Zip, not p7zip), `gdisk` (`sgdisk`) | none |
| Wallet files | none | real wallet files from the wallets' published test suites, as fixtures (built: `tools/vectors/wallet-files/`, with each file's repository, commit, licence and SHA-256) |

The standard library covers the rest: `hashlib`, `hmac`, `json`,
`zlib`, `plistlib`, `uuid`. btclib's wheels, Core's functional-test
framework and its `crypto/ripemd160.py`, python-ecdsa, PyCA
cryptography, Go and the volvelle harness are in place already.

### 9.7 The batches

Moved to `docs/history/ESTABLISHED-LOG.md`.

### 9.8 Not decided: a vault reader written from VAULT.md alone

Withdrawn: the container it would have read, Faraday's own FVLT format,
is replaced by KDBX 4 and RFC 9580, which others' code reads (F189);
the vault is judged by the programs `docs/VAULT.md` §10 names.
