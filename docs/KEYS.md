# The key boundary: faraday-keys

**Kind:** Reference
**Status:** Built (steps 0 to 4); the "as built" sections of §4 are the boundary as it stands
**Decisions:** F70

F70 decided that one crate, `faraday-keys`, owns every secret in memory,
and that faraday-core holds handles. This document says what the
boundary is.

## 1. Where secrets are today

Moved to `docs/history/KEYS-LOG.md`.

## 2. What the boundary is

- **Key material** lives in `faraday-keys` and nowhere else:
  - a key's words, passphrase, seed and master key;
  - anything derived from them that is secret;
  - FROST and MuSig secret nonces;
  - the master seed bytes of a key loaded from a seed (`Keys::add_seed`:
    codex32, SLIP-39), kept with the key so that
    its paper forms can be written again;
  - a codex32 work set, the set being made by hand (`WorkSet`,
    `faraday/faraday-keys/src/codex32.rs`): the characters entered, the
    checksum worksheets and strings made from them, and the seed;
  - the codex32 strings of a key's set this session has seen, kept with
    the key (`Keys::keep_codex32`), leaving only as a stick's file
    (`BackupStick(Part::Codex32)`) or a vault's share record
    (`VaultEntry(Part::Codex32)`);
  - GPG and Secure Boot private keys;
  - TOTP secrets;
  - the whole open vault: every decrypted record, passwords and notes
    included (owner, 2026-10-21, Q1 of §9: every decrypted vault byte is
    in faraday-keys);
  - the slot key that seals them again.
- **Operations** on key material are functions of `faraday-keys`. They
  return public results: signatures, xpubs, addresses, fingerprints,
  signed files and codes.
- **Leaving.** Key material leaves only through `reveal(id, Reveal::…)`.
  It returns a guard that borrows the store and wipes its copy on drop.
  The list of reasons is closed, eleven `Reveal` variants
  (`faraday/faraday-keys/src/session.rs`; owner, 2026-10-21, Q2 of §9):
  the words shown; the typed passphrase shown; SeedQR; a copy-by-hand
  check; paper forms (Seed XOR, codex32, SLIP-39 shares, and the work
  set's worksheets and strings); a BIP-85 child
  shown; a Lightning node key shown; GPG paperkey; a vault entry's secret
  (a password or TOTP code); an unprotected file to the Outbox; the
  backup flow writing words or a SeedQR picture to a stick. A new reason
  is a new variant, seen in review.
- **Transient input** stays where it arrives, under lint-secrets' rules,
  until it is handed in:
  - typed words at Done;
  - a scanned SeedQR when it is read;
  - a vault passphrase at Open;
  - an Inbox secret file when it is loaded.
- **The crates that compute and keep nothing** stay as they are:
  faraday-bip, -psbt, -crypto, -backup, -vault, -pgp and -sb. Only
  faraday-keys calls their secret-taking functions.

The crate is faraday/faraday-keys (new). It sits beside faraday-vault,
faraday-pgp and faraday-sb, which it uses, and holds no flow
and no wording.

## 3. The API, in outline

```rust
pub struct Keys { /* session keys, nonces, the open vault's secrets */ }

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyId([u8; 32]); // today's master_id: a hash of public values

pub struct KeyInfo {        // kept values: public, and read by drawing
    pub id: KeyId,
    pub fingerprint: Fingerprint,
    pub label: String,
    pub language: Language,
    pub word_count: Option<u8>,
    pub has_passphrase: bool,
    pub share: Option<PublicKey>,
}

impl Keys {
    // In: costs, so each takes the token.
    pub fn add_words(&mut self, c: &Compute, words: &Mnemonic,
                     passphrase: Option<&SecretText>, label: &str) -> KeyId;
    pub fn add_seed(&mut self, c: &Compute, seed: &[u8], label: &str) -> KeyId;
    pub fn forget(&mut self, id: KeyId);

    // Public reads.
    pub fn info(&self, id: KeyId) -> Option<&KeyInfo>;
    pub fn all(&self) -> impl Iterator<Item = &KeyInfo>;
    pub fn same_words(&self, a: KeyId, b: KeyId) -> bool;
    pub fn xpub_text(&self, c: &Compute, id: KeyId, at: &Path) -> String;

    // Operations (one per group in §1), for example:
    pub fn sign_psbt(&mut self, c: &Compute, psbt: &mut Psbt,
                     seed: &SignSeed) -> SignReport;
    pub fn sign_message(&self, c: &Compute, id: KeyId, at: &Path,
                        msg: &str) -> Result<MessageSig, Error>;
    pub fn bip85(&self, c: &Compute, id: KeyId, app: Bip85App) -> Child;

    // Out: the only way words, passphrases or private keys leave.
    pub fn reveal(&self, id: KeyId, why: Reveal) -> Revealed<'_>;
}
```

- A codex32 work set is a new secret in the same way: `WorkSet::new`
  takes a public `Scheme`; `push` hands it the person's characters one
  at a time as each is complete (the rolls of the character in progress
  stay on the pad, as keygen's do); it computes the worksheet rows, the
  checksums, the derived shares, the secret and the seed there and keeps
  them in step with the characters at every push and pop. It leaves only
  as a paper form: `worksheet`, `translation` and `strings` lend their
  results for `Reveal::PaperForm` and for no other reason (no twelfth
  variant), and as a key through `Keys::add_codex32`, which hands the
  seed to `add_seed`. It wipes on drop.
- A key loaded by `add_seed` keeps its seed bytes (`holds_seed_bytes`
  tells whether), wiped when the key is dropped.
  `Keys::codex32_shares(c, i, k, n, draw, why)` makes any `k` (2 to 9)
  of `n` (`k` to 31) shares, or with `k` 0 and `n` 1 the secret alone,
  from the 64-byte BIP-39 seed of a key with words or from the seed
  bytes of one without, at that seed's own length.
- A BIP-85 child, a vanity find or a FROST share is a new secret. It comes
  back as a `Child` held inside the crate, which leaves only by `add_*` or
  `reveal`.
- **Drawing.** `View` gets `&Keys` read-only and reaches secret text only
  through `View::reveal`. That is one named accessor, and it joins
  COMPUTE's `Allowed` list beside `SecretShown`.
- **Event code** holds `&mut Keys` through `Faraday` as today, with the
  `keys` field in place of `session`.

## 4. The steps

The plan of steps 0 to 4: moved to `docs/history/KEYS-LOG.md`. What each step built:

### Step 2 as built

- `faraday-keys` has `session`: `Keys` (the store, with the network and the
  demo's public seeds), `KeyInfo` and `KeyId`, `Reveal` and `Revealed`.
  `Key`'s fields are private to that module.
- faraday-core's `Session` holds `wallets` and `keys: Keys`, since the
  wallets and the keys share one network. Callers address a key by its
  place in the store, as before.
- `Reveal` has eleven variants; `OutboxFile`, `BackupStick` and
  `VaultEntry` carry `Part` (the words or the passphrase). A SeedQR
  picture, a paper form, a Lightning key, a BIP-85 child, a vanity find and
  a vault key record are operations of `Keys`, since they cost and take a
  token.
- Until the wallet kinds' derivation tables move in (step 4),
  `Keys::with_master` and `Keys::public_text` hand a key's master key to a
  closure that returns public text. Tests reach it through
  `Keys::master_for_tests` (feature `master-for-tests`, which
  `tools/lint-keys-tests.sh` keeps out of every shipped binary).

### Step 3a as built

Step 3 is two commits. 3a moves where the open vault is kept and how it
is opened and sealed; nothing else changes. 3b narrows what faraday-core
can read.

- **The store.** `faraday-keys::vault::OpenVaults<R>` holds, for each open
  vault, its `Opened` (the slot key), the file the other slots are copied
  from, and its records. faraday-core holds it as `Vaults::store`, beside
  `Vaults::open`, which keeps what the screens say of each vault (name,
  changes, skip, header) and its `VaultId`. They are made and closed
  together (`Vaults::close`, `Vaults::close_all`).
  - It is a sibling of `Keys`, not a field of it, because the records'
    type is faraday-core's `VaultRecords`: the records with the wallet
    descriptions read from them (`Described`), which need faraday-core's
    wallet code. `OpenVaults` is generic over any type that reads as
    `Contents` and is made from it. Held in `Vaults`, a record is read
    through `self.vaults.store` while another field of `self` is in use.
- **Functions of faraday-keys.**
  - `OpenVaults::unlock` (Costs an Argon2id run; keeps the slot and
    returns the `VaultId` and the public header), `sealed`, `close`,
    `clear`.
  - `load_from_vault` and `load_key_record`, which add key records to
    `Keys`, and `passphrase_kept_for`, now theirs.
  - `vault::create`, `vault::measure`, `vault::write_kdbx` (encrypted
    bytes), `seed_of` and `sheet_parts`.
- **Temporary.** `OpenVaults::vault_records` and `vault_records_mut` lent
  the records to faraday-core's reads and edits. Step 3b removed them.
- The slot key is private to `vault`, as before.

### Step 3b as built

faraday-core sees an open vault's public fields and nothing else.

- **The table.** `vault::SECRET_FIELDS` lists the secret fields by record
  type and number, and `vault::is_secret` reads it; everything it does
  not list is public. Secret: a KEY's payload and passphrase, a
  PASSPHRASE's passphrase, a NOTE's text, an ENTRY's password, TOTP
  secret and notes, a ROUND's state, a GPG
  record's two seeds and its revocation certificate, a SECURE_BOOT
  record's three private keys. The revocation certificate is secret
  because whoever has it can revoke the key (F32; owner, 2026-10-21):
  `gpg_keep_revocation` makes and keeps it inside the store,
  `gpg_revocation_file` and `gpg_new_revocation_file` give the `.rev`
  file only as a held value for `Reveal::OutboxFile(Part::Revocation)`,
  and `gpg_revocation_kept_at` gives the date it was made.
- **Reading.** `OpenVaults::records(id)` gives `Records`, whose records are
  `RecordView`s: `kind()`, `public`, `public_text`, `public_all`, `has`,
  `len`, `loads_at_unlock`, `size`. `public` is `None` for a secret field.
  `OpenVaults::version(id)` counts the edits of a vault.
- **Revealing.** `OpenVaults::reveal(c, id, record, field, why)` lends a
  secret field as a `Revealed`, for a `Reveal` whose `Part` names it
  (`Password`, `Totp`, `Note`, `EntryNotes`, `Passphrase`, `Words`); a key
  record's words are written out from its payload (it costs a parse).
  `words_of` gives the words as a held `Mnemonic` for a SeedQR or a file,
  `gpg_paperkey` the paperkey text, `totp` the code and account of an
  entry, `kdbx_entry` a KeePass database of one.
- **Operations by record.** `load_from_vault`, `load_key_record`,
  `load_key_with_passphrase`, `same_words`, `stores_passphrase_of`,
  `passphrase_record_is`, `holds_session_words`, `word_count` and
  `words_id` (worked out once per change, inside the store),
  `round_section`, `sb_enrolment`, `sb_sign_image`, the GPG
  certificate, kept revocation, revocation files and signature, and the
  free functions `key_record_id`, `key_identity`, `key_account`,
  `gpg_facts` and `gpg_revocation_kept_at`. faraday-core keeps the caches of those it already kept
  (`Vaults::fingerprints`, `Vaults::accounts`) and asks for them by
  `key_record_id`.
- **Editing.** `insert`, `insert_at`, `replace`, `remove`, `remove_where`,
  `set_public`, `set_public_all`, `set_public_all_in`, `set_secret`,
  `set_fields` (a form), `set_loads_at_unlock`, `copy_record` and `stage`
  (a copy of a record that waits to go into a vault as a `Staged`,
  readable by `view()` and nothing else), `fits`. A secret field is
  written only from input the caller holds, or copied inside the crate.
- **The cache.** `OpenVaults` is no longer generic. faraday-core keeps the
  wallet descriptions read from the public view in `OpenVault::read`, one
  per open vault, with the version they were read at. `Vaults::store` is
  private: `Vaults::store()` reads, and `Vaults::edit(id, change)` is the
  one way the records change, and drops the cache. Every lookup checks the
  version in a debug build;
  `a_wallet_record_changed_in_an_open_vault_shows_as_it_now_is` changes
  the records to test it. Word counts and words' ids are facts the store
  works out and drops on every edit.
- **Left in faraday-core.** A `Record` it builds from input it holds (a
  typed password, a wallet description); and a key record's words,
  handed as a held `Mnemonic` to the SeedQR, the stick file and the
  screen. (A key made from fresh entropy and a ROUND record's
  `CarrySection` were left here too; step 4a moved them in.)
- **Tests.** Tests read records through the same views; those that need a
  raw record read it through `contents_for_tests`, behind
  `master-for-tests`.

### Step 4a as built

Step 4a closes §4.1's public types. Each type that holds a secret is
now opaque: its secret is in a private field, it is neither `Clone` nor
`Debug`, and the secret leaves only through a method that names a
`Reveal` reason.

- **Silent payments.** `silent::Receiver` wraps BIP-352's receiver.
  Public: `network`, `scan_public_key`, `spend_public_key`, `address`,
  `payments_in`. The scan key leaves through `scan_key(why)`, for
  `VaultEntry` or `OutboxFile` of `Part::ScanKey`.
- **Lightning.** `lightning::Node` gives `public` and `birthday`; the
  private key leaves as hex through `private_hex(why)`, for
  `LightningKey` (on screen), or `VaultEntry` or `OutboxFile` of
  `Part::NodeKey`. `Keys::lightning_node` returns the node itself.
- **FROST.** `Keys::deal` returns a `Dealt` whose `record` is public and
  whose new shares' words are private; `Keys::add_dealt` loads them as
  keys, labelled by the caller. `make_group` and `Dealer` are the
  crate's own. All of it exists only with the `unreleased` feature:
  FROST is compiled out of the app (F109), and without it no key is
  a share.
- **Keys made from fresh entropy.** `gpg::create` and
  `secure_boot::create` make the key and the record every vault of the
  plan takes, as a `vault::MadeKey`: its public fields through `view()`,
  a copy into each vault made inside the crate (`IntoRecord for
  &MadeKey`). They return the public parts beside it: the GPG facts and
  certificate, the owner GUID and the enrolment
  files. `gpg::Key`, `secure_boot::Keys` and `Pair` are the crate's own,
  with every function that takes them.
- **The carry section.** `carry::Section` wraps a FROST spend's
  `CarrySection` and tells only `is_empty`. `carry::parse` reads a carry
  file into one, `round_section` returns one, and `carry::inspect` and
  `Keys::sign` (which now takes it, and returns the one the pass leaves
  beside the result) read its nonces inside the crate. faraday-psbt's
  `CarrySection` keeps its public fields: its own tests forge sections
  to check that a forged one is refused.
- **`wallet_seed_bases`.** `Keys::words_record_id` (a hash) and
  `Keys::words_identity` (a PBKDF2) give the id and identity a vault
  record of a key's words alone would have, and faraday-core keeps the
  identity in the vault's cache by that id, as it does a record's. No
  record of the words leaves; `words_record` is gone.
- **`MusigSession`** (faraday-psbt) was already opaque: its secret
  nonces are private fields.

### Step 4b as built

- **`faraday_keys::words`** is where seed words become a `Mnemonic`:
  `from_indices`, `parse`, `from_entropy`, `from_seedqr_digits`,
  `from_compact_seedqr`, and `slip39_split` for fresh entropy made into
  SLIP-39 shares. faraday-core holds what comes back as input in hand
  (§2): to check it, show it for writing down, or hand it to `Keys`. It
  cannot derive a key from one: `to_seed` and the master key are refused
  to it.
- **The master-key test helpers** (`NewKind::key_text`,
  `key_text_account`, `key_text_at`) left `create.rs`. Tests use
  `testkit::KindText` over `Keys::master_for_tests`.
- **A WIF private key** pasted is told by `identity::is_wif`.
- **`IntoRecord`** gives a record's `size`, so faraday-core never takes a
  held record out of its guard to measure it.
- **The testkit was in the desktop app.** The desktop shell turned on
  faraday-core's `testkit` feature for one constant, the network it
  starts on. It names the network itself now, takes the testkit only in
  its tests, and `tools/lint-keys-tests.sh` fails when a shipped binary
  turns the feature on.
- **The lint.** `tools/lint-secrets.sh` rule 4 refuses, under faraday/
  outside faraday-keys:
  - (a) a key or seed in hand: `MasterKey`, `SecretKey`, `Xpriv`,
    `Keypair`, `PrivateKey`, `DerivedKey`, `SecShare`, `SecNonce`,
    `SlotKey`, `to_seed`;
  - (b) seed words made into a `Mnemonic`, a SeedQR decoded, a SLIP-39
    split, a vault record's words read;
  - (c) the primitive crates' secret-taking functions and types: a GPG
    key, Secure Boot keys made or used, a vault opened, sealed,
    created or its slot key derived, a carry section's nonces, a PSBT
    signed, BIP-352's receiver, a FROST deal or secret share, an aezeed
    decoded, by path or in a `use` list;
  - (d) `Held::release` and `into_record` outside the one named site, a
    GPG paperkey drawn while it is held.

  Exempt: the primitive crates themselves; `tests/`, `examples/` and
  `fuzz/`; the snapshot shell and `faraday/tools/` (test renders and test
  data, never shipped); faraday-core's `testkit.rs`, which
  `lint-keys-tests.sh` keeps out of shipped binaries. faraday-intake is
  exempt from (b) only: it reads text as it arrives and keeps none of it,
  and faraday-keys depends on it.

### 4.1 Left after step 2, closed by step 4

Step 2 (2026-10-21) removed every API that handed out a master key, a
closure over one, or seed bytes. The operations that return secret
material take their `Reveal` reason and return `Held<T>`, which is not
`Clone` or `Debug`, wipes on drop, and has `release()` for the point
where the secret goes into a vault record.

What was open after step 3:
- Public types holding a secret, and `wallet_seed_bases`: closed by step 4a;
  moved to `docs/history/KEYS-LOG.md`.
- **FROST shares** leave as `PaperForm`, the reason that fits them: words
  the person writes down. They are keys once dealt (step 4a), and leave
  as any key's words do.
- **`Keys::master_for_tests`**, behind the `master-for-tests` feature,
  stays for tests. `tools/lint-keys-tests.sh` keeps that feature out of
  every binary Faraday ships.
- **Names.** `Faraday.session` remains `Session { wallets, keys }`,
  because the wallets and the keys share one network. Handles are still
  indices and `[u8; 32]` ids. Neither weakens the boundary.

## 5. What does not change

- No format: vaults, KDBX, PSBTs and files written to sticks.
- No screen and no string: every test passes unchanged, except where a
  test reached a secret field directly. That test moves to an operation
  or `reveal`.
- The compute token's rules. Every costly function still counts itself,
  and drawing still cannot mint a token.

## 6. Tests

- Tests may build keys directly: `tests/` is exempt from the lint.
  - 5 test files call `MasterKey::from_seed` or `to_seed`.
  - 22 build a `Mnemonic`.
  - `testkit` does the same behind a feature.
- The samples (F81, `docs/MODES.md` §5): `faraday/faraday-samples` is the
  public set Demo takes (keys in every form Faraday reads, the testkit's
  wallets, a vault, a signed message), as data only: strings, bytes and
  fingerprints in the shapes `faraday_core::samples` defines. It derives
  nothing, so rule 4 has nothing to refuse there. The testkit and the
  samples crate are the two bodies of dummy data, and neither is key
  code: the testkit stays out of every shipped binary by
  `lint-keys-tests.sh`'s first two checks, and the samples crate by its
  third, which lets only the desktop app depend on it. Its tests check
  every value against the testkit, the published vectors and another
  implementation of BIP-39 and BIP-32. `Keys::set_demo` takes the
  fingerprints as literals; nothing derives at the desktop's start.
- `faraday-keys` gets its own `tests/`, one per operation group. Each says
  what a person or wallet would notice (a signature that verifies, a key
  that loads back from a vault) and checks against the vectors the
  primitive crates already use.

## 7. Size and cost

Moved to `docs/history/KEYS-LOG.md`.

## 8. What it buys

An auditor reads faraday/faraday-keys and the primitive crates, plus
the reveal list, and can state:
- every byte of key material lives in one crate;
- it leaves through these N named reasons;
- `just` fails if any other code touches a secret type.

Before F70 that took reading 36 files of faraday-core. As built, the
three statements are `docs/AUDIT.md` §2.

## 9. Questions for the owner

Answered 2026-10-21 (F70); the answers in force are in §2 (Q1, the vault's scope; Q2, the eleven reveals) and §6 (Q4, tests exempt), and the questions: moved to `docs/history/KEYS-LOG.md`.
