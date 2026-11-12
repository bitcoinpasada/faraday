# The key boundary: faraday-keys

**Kind:** Reference
**Status:** Built (steps 0 to 4); the "as built" sections of §4 are the boundary as it stands
**Decisions:** F70, F125

F70 decided that one crate, `faraday-keys`, owns every secret in memory,
and that faraday-core holds handles. This document says what the
boundary is.

## 1. Where secrets are today

Moved to `docs/history/KEYS-LOG.md`.

## 2. What the boundary is

- **Key material** lives in `faraday-keys` and nowhere else:
  - a key's words, passphrase, seed and master key;
  - anything derived from them that is secret;
  - MuSig secret nonces;
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
  - GPG and Secure Boot private keys, and a GPG key's paperkey being
    typed back (the restore set, `gpg::Restore`,
    `faraday/faraday-keys/src/gpg.rs`): the bytes of each line checked;
  - TOTP secrets;
  - a WIF private key being swept (`Keys::sweep_read`,
    `faraday/faraday-keys/src/sweep.rs`, F171): one at most, never kept,
    erased at the sweep's Done, a lock or leaving its sheet; it leaves
    only as signatures (`Keys::sweep_sign`), and has no reveal;
  - the whole open vault: every decrypted record, passwords and notes
    included (owner, 2026-10-21, Q1 of §9: every decrypted vault byte is
    in faraday-keys);
  - the slot key that seals them again.
- **Operations** on key material are functions of `faraday-keys`. They
  return public results: signatures, xpubs, addresses, fingerprints,
  signed files and codes. Each master key's curve operations run in a
  context of its own, randomized once from the key when it is made and
  again from the session's randomness: as the key joins the store, or
  when the session's seed arrives if it joined before, and for every
  key whenever randomness the person adds changes the seed
  (`Keys::reblind`). Blinding changes no result.
- **No key joins while a stick is attached.** Every way of adding a key
  refuses, before anything is derived, while faraday-core says a stick
  is attached (`Keys::set_stick_attached`, set where the sticks the
  shell reports change; `docs/SECURITY-MODEL.md`).
- **Leaving.** Key material leaves only through `reveal(id, Reveal::…)`.
  It returns a guard that borrows the store and wipes its copy on drop.
  The list of reasons is closed, sixteen `Reveal` variants
  (`faraday/faraday-keys/src/session.rs`; owner, 2026-10-21, Q2 of §9;
  the three before the last two, owner, 2026-10-26, F125; the last two, F219 and F226):
  the words shown; the typed passphrase shown; SeedQR; a copy-by-hand
  check; paper forms (Seed XOR, codex32, SLIP-39 shares, a master seed as hex,
  an xprv, and the work
  set's worksheets and strings); a BIP-85 child
  shown; a Lightning node key shown; GPG paperkey; a vault entry's secret
  (a password or TOTP code); an unprotected file to the Outbox; the
  backup flow writing words or a SeedQR picture to a stick; and on
  Explore, each behind its eye, a key's master seed (`SeedShown`), an
  extended private key (`ExtendedPrivateKey`) and an address's private
  key (`AddressKey`); and a seed's entropy sealed as a Krux encrypted code
  (`KefCode`); and the item a QR vault was opened to, lent to the form or
  the eye that takes it (`QrVault`). A new reason is a new variant, seen in review.
  A reason gives only its own text: `Reveal::part` names the stored
  text a reason lends, and Explore's three lend none, so `reveal` and
  a vault record's reveal give them nothing.
- **Transient input** stays where it arrives, under lint-secrets' rules,
  until it is handed in:
  - typed words at Done;
  - a scanned SeedQR when it is read;
  - a vault passphrase at Open;
  - an Inbox secret file when it is loaded.
- **A stretch job** (`faraday/faraday-keys/src/job.rs`, WORKERS §6) is
  where a vault passphrase goes on to be stretched on the worker. An
  Open's passphrase is moved into a `StretchJob` (a `SecretText`, taken
  out of the typed box, so no copy is made); Create's passphrases are
  copied in once, because the form keeps them for a Back or an error,
  and the first is kept beside the job for a flow that unlocks the vault
  it made. The answer, `Stretched`, holds the slot key and the records of
  a vault opened, or the bytes of a vault made. It leaves job.rs only
  into `OpenVaults::adopt`, on the main thread. Everything wipes as it
  drops: at the job's end on the worker, or where a stale or locked
  answer is dropped. The Argon2id call is `faraday_vault::open` and
  `create`, unchanged.
  An aezeed read (`lightning::AezeedJob`) and a KeePass export
  (`job::KdbxJob`) are jobs of the same kind: the word indices and the
  passphrase, or the entry's text and password and the passphrase, are
  copied in once (the screen keeps its fields for a Back or an error)
  and wipe with the job. The node key comes back inside faraday-keys'
  `Node`, through `Keys::aezeed_node`, which applies the demo's refusal
  on the main thread; the KeePass file is encrypted, so public. The
  scrypt and Argon2id calls are unchanged. A Core Lightning hsm_secret
  (`hsm_secret::HsmJob`, F228; `hsm_node::Keys::hsm_node`) is a job of the same kind: the file's
  bytes and the passphrase are copied in once and wipe with the job, the
  node key comes back through `Keys::hsm_node` with the same demo
  refusal. The mnemonic form (a passphrase hash, then English words) is read by the same job, which says whether it asks for a passphrase (`hsm_secret::needs_passphrase`). English BIP-39 words give the node key without a job through
  `Keys::hsm_words_node`, with the same demo refusal, and a loaded key's
  words through `Keys::hsm_key_node`; words in another language, typed
  (told as Add a key tells them) or of a loaded key, are refused
  (`WordsRefusal::NotEnglish`), as Core Lightning refuses them.
  A new passphrase for an open vault's next seal
  (`OpenVaults::set_passphrase`) is moved into the vault's slot as a
  `SecretText`, never read back, and replaced or wiped with the slot; a
  `SealJob` gets one copy that wipes with the job, so a seal that fails
  keeps the pending passphrase for a retry.
  A vault's key file (32 bytes) and YubiKey secret (20 bytes) are held
  in the same way (`faraday_vault::kdbx::Components`, neither `Clone` nor
  `Debug`, read from the key file's bytes and the typed secret by
  `UnlockParts::read` and moved into `StretchJob::keyed`, then into
  `OpenVaults::unlock_with` and the opened file, wiped as it drops, never
  shown; no second copy is kept, and each seal makes a short-lived copy
  of them (`Components::changed`) that wipes as it drops; a key file added
  to an open vault is read in faraday-keys by `OpenVaults::set_key_file_from`,
  so faraday-core never holds its key; the `impl Into<UnlockParts>` argument
  of `StretchJob::keyed` and `From<Components> for UnlockParts` stay public
  only because faraday-keys/tests/vault_keyed.rs and
  `testkit::kdbx_vault_keyed` build a `Components` directly): every seal writes them back
  unless `set_key_file` (add, replace or remove) or `remove_yubikey` is
  pending, and a `SealJob` gets one copy of a pending key file that wipes
  with the job. `OpenVaults::needs` says which the next seal writes. A
  key file leaves only if Faraday made it (`OpenVaults::make_key_file`, or
  `StretchJob::making_key_file` at a make; "made" means the caller drew
  fresh randomness for it, which the store cannot check), as its KeePass document through
  `OpenVaults::key_file_document` or `Stretched::key_file_document`, held
  for `Reveal::OutboxFile(Part::KeyFile)` like `gpg_revocation_file`. A
  drawn key is written only after its document is taken: until then a seal,
  a lock's included, writes the file under the key file it had, and
  `needs` says so. A document comes only from the draw that made the
  vault's key, and a failed make gives none; `OpenVaults::key_file_document`
  hands it out once (a second call gives none until another key is made)
  and not while the vault seals, and `OpenVaults::unmake_key_file` takes a
  made key back where its document was not kept. The parts of a vault
  just made, which a copy or backup flow unlocks at once, wait in
  `OpenVaults` (`hold_made_parts`) as an opaque `UnlockParts`, taken once
  by the unlock (`take_made_parts`) and wiped by `clear_made_parts` or
  `OpenVaults::clear` at a lock; the screens keep none. A
  key file chosen (`set_key_file`) or opened with has no way out. A make
  never takes a YubiKey's secret (F208): `StretchJob::keyed` refuses it.
- **A signing job** (`job::SignJob`, WORKERS §6) shares the keys, it does
  not copy them. The store holds each master key behind an `Arc` that is
  never opened for change: a change of network makes a new key
  (`MasterKey::for_network`), and `Keys::reblind` reblinds a key only when
  the store's is the only `Arc`, and marks any other as owing. `Keys::sign_job` gives the job a clone of
  the `Arc` of each selected key, with the PSBT, the context's key
  references and policies (public), the dangers held past, the seed
  (`Zeroizing`) and the MuSig2 round, all owned. A key forgotten or a lock
  while it signs drops the store's `Arc`; the job's keeps the key until the
  job ends, and `Pinned` wipes it as the last one drops. A key a job holds
  keeps the context the job started with and is reblinded
  (`Keys::reblind_owed`) as soon as the job's answer is applied or dropped
  (blinding changes no result). No master key
  crosses the thread other than by `Arc`; `Pinned` is `Send` and `Sync`
  where its value is, as a `Box` is. The signing call is
  `faraday_psbt::sign`, unchanged.
  The MuSig2 round moves into the job and comes back with the answer
  (`Signing::into_parts`), to be put back on the spend on the main thread.
  A stale answer (Back, or a lock) drops the round with its secret nonces:
  none is used again, none was shared (the PSBT that carried the public
  nonce is dropped too), and a round that had begun is lost, so the next
  press opens a new one from a new seed. A round is never put back from a
  stale answer.
  After it signs, the job also works the spend out on a view of every
  loaded key, as a Collect's does, and the view is dropped before the
  answer is sent; the round's public view (`Signing::round_view`, no
  nonce) is all of it the work reads.
- **A Collect job** (`signing/collect.rs`, WORKERS §6) is given a view of
  every loaded key, not a selection: `Keys::job_view` makes a store whose
  entries hold an `Arc` clone of each master, as `SignJob` does, and the
  key's public facts; its words, passphrase, seed, codex32 set and every
  other record are empty. It reveals nothing and is not a reveal. The
  nonce check's recompute and the signature count's extended public keys
  at hardened paths use the masters; the MuSig2 round does not go, only
  its public view. The view is dropped on the worker before the answer is
  sent, the answer holds public values and verdicts, and a reblind owed
  meanwhile is paid as the answer is applied or dropped, as for a signing
  job.
- **What wiping does not reach.** Faraday's types wipe what they hold;
  five things are outside them (`docs/ESTABLISHED.md` §4.9): a secret
  held in a plain type while it is live; a copy in freed heap until its
  block is reused or the process ends (on the device the next lock, or
  power-off for faraday-disk; in Faraday Demo and the Starter, until the
  program exits); the stack and registers, including copies of `Copy`
  types and the slots a value was moved from; memory other code
  allocates itself (C libraries, the graphics stack, the window system's
  text input and clipboard, the kernel's pipe buffers, files written);
  and swap, sleep images and crash dumps, none on the stick and the Pi,
  the desktop operating system's for Faraday Demo and the Starter.
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
    pub child_of: Option<ChildOf>, // the BIP-85 parent, when recorded (F166)
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
  results for `Reveal::PaperForm` and for no other reason (no variant
  of its own), and as a key through `Keys::add_codex32`, which hands the
  seed to `add_seed`. It wipes on drop.
- A SLIP-39 set typed into Convert a seed (`slip39_set.rs`, F204) is held
  as an aezeed is: `Keys::slip39_set_hold` takes one share at a time,
  refusing what SLIP-39 refuses; the set is never a key, and is wiped by
  `slip39_set_forget`, by an aezeed held and on lock. What it shows a
  screen (`slip39_set_facts`) holds no secret. It leaves only as the
  shares `slip39_set_share` (a share at a member index) and
  `slip39_set_rotated` (a new set of the same EMS) make, for
  `Reveal::PaperForm` and no other reason; no passphrase is taken and
  nothing is decrypted.
- A key loaded by `add_seed` keeps its seed bytes (`holds_seed_bytes`
  tells whether), wiped when the key is dropped.
- A GPG key restored from its paperkey (`docs/GPG-PAPERKEY.md` §6) is
  held the same way: `gpg::Restore::new` takes the public certificate;
  `line` takes each line as it is ended, checks its CRC and what the
  data's headers say, and keeps its bytes or refuses it and keeps
  nothing; `paperkey_code` takes a paperkey read from a code (paperkey's lines or
  its raw form), every line checked as typed, whole or not at all; what
  it tells is public (the lines checked, the count, the
  fingerprints read). The lines are never shown again, so it has no
  reveal. `finish` makes the key and gives it as `create` does, a
  `MadeKey` with its facts and certificate; `wipe` forgets the lines and
  keeps the certificate, for a lock. It wipes on drop.
  `Keys::codex32_shares(i, k, n, draw, why)` makes any `k` (2 to 9)
  of `n` (`k` to 31) shares, or with `k` 0 and `n` 1 the secret alone,
  from the seed bytes of a key without words, at that seed's own length.
  A key with words is refused (`Paper::Words`): its 64-byte BIP-39 seed
  has the passphrase merged in and would give back neither the words nor
  the passphrase.
- A BIP-85 child or a vanity find is a new secret. It comes
  back as a `Child` held inside the crate, which leaves only by `add_*` or
  `reveal`.
- **Explore's eyes** (F125; `faraday/faraday-core/src/explore/shown.rs`).
  Explore shows a loaded key's secrets, on every build and network, each
  behind its own eye: one press shows, the next hides, and every eye is
  closed when Explore opens. The eyes and what each gives, to Explore
  alone:
  - the key's own secret: for a key of words, its words and passphrase,
    lent each frame by `reveal(i, Reveal::Words)` and
    `reveal(i, Reveal::Passphrase)` (a borrow, nothing kept or made);
    for a key with no words, `Keys::seed_shown(i, Reveal::SeedShown)`:
    the master seed in hex, and where its codex32 set is known, the
    set's secret recovered from the strings this session holds (BIP 93,
    as written, padding included) and those strings. A codex32 recovery
    and a checksum per string;
  - the extended private keys: `Keys::xprv_at(c, i, path,
    Reveal::ExtendedPrivateKey)`, the root's (the empty path) and the
    one at the explored path's account (the path itself where it is in
    none). Costs a key derivation each, so it takes the token;
  - an address's private key: `Keys::wif_at(c, i, path,
    Reveal::AddressKey)`, compressed WIF on the store's network, for the
    address at the path and for each address on the Addresses page, each
    its own eye. Costs a key derivation;
  - a seed's entropy for a Krux encrypted code: `Keys::kef_entropy(c, i,
    Reveal::KefCode)`, 16 bytes for a seed of 12 English words and 32 for 24,
    held and wiped on drop, and nothing for a seed of 15, 18 or 21 words, another
    list's or a key with no words (`docs/KEF.md` §6). Costs a parse of the words.
  - whether what a Krux encrypted code opened to is a seed's entropy:
    `Keys::kef_matches(c, i, Reveal::KefCode, plaintext)`, a yes or a no; the
    entropy is held for the comparison and wiped, and never leaves
    `faraday-keys` (`docs/KEF.md` §15.1). Costs a parse of the words.
  - the QR vault tool (`faraday-keys/src/qr_vault.rs`, `docs/VAULT.md`
    §13): a key's words, a file vault's item or a typed note sealed as a
    QR vault (`Keys::qr_seal_key_job`, `OpenVaults::qr_seal_item_job`,
    `qr_seal_note_job`; the tool's seal runs as a `QrSealJob` on the
    worker), the words never leaving the crate, each read back
    under the same passphrase before the code's bytes are given; and a
    code opened into one held item (`QrOpenJob::run` on the worker, its
    opaque `QrOpened` kept by `Keys::qr_hold`), in place of any held
    before, wiped by `Keys::qr_forget` and by `Keys::clear` (a lock).
    The held item leaves only for `Reveal::QrVault`: `Keys::qr_words`,
    `qr_share`, `qr_passphrase` and `qr_note` lend it,
    `qr_add_passphrase` loads a passphrase as a key, `qr_passphrase_offer`
    says from the keys loaded, in fingerprints only, whether a held
    passphrase adds to one of them, is loaded already or has no key to
    add to (worked out when the code opens and after an add, never while
    drawing), and `OpenVaults::qr_save_note` puts a note into an open
    vault. The Open card's seed goes to the seed sheet through
    `qr_words`, a scanned SeedQR's flow, and its share to the codex32
    form through `qr_share`; each lent copy is wiped as the flow takes it. Every
    seal and open of a QR vault in the app goes through this module
    (`tools/lint-secrets.sh`).

  - the private keys of a Bitcoin Core descriptor wallet (`faraday-keys/src/core_wallet.rs`,
    F229): `core_wallet::check_private_key(c, pubkey, value)` for a
    `walletdescriptorkey` record and `check_encrypted_keys(c, mkey,
    passphrase, keys)` for an encrypted wallet's `walletdescriptorckey`
    records, opened with the master key the passphrase gives. Each key is
    wiped as it is checked and only a pass or a fail (`KeyVerdict`)
    leaves, never the key; a wrong passphrase is `OpenError`. The `_accounts`
    forms (`check_private_key_accounts`, `check_encrypted_keys_accounts`)
    also derive each cached account key (`AccountCheck`) along its hardened
    path from the key and say whether it matches, wiping each step; one
    hardened derivation per step. Costs the
    master key's rounds (at most 10,000,000), so both take the token.
    `faraday-core-wallet`'s `read` calls them.

  Each comes back as `Held` (an `XprivAscii`, a `WifAscii`, or
  `Zeroizing` strings, every one wiped on drop) and is made once, at the
  eye's press, and kept on Explore's state (`Shown`) with the key and
  network it is of. Drawing reads it and derives nothing, never per
  frame. What is no longer on screen is dropped after every press and
  key on Explore (another key, another path or account, another page);
  closing its eye, leaving Explore, the idle warning and a lock drop all
  of it.
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
- `Reveal` had eleven variants (fourteen since F125, §2); `OutboxFile`, `BackupStick` and
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
  entry, `kdbx_job` a vault of one entry for a KeePass app, written by `KdbxJob::run`.
- **Operations by record.** `load_from_vault`, `load_key_record`,
  `load_key_with_passphrase`, `same_words`, `stores_passphrase_of`,
  `passphrase_record_is`, `holds_session_words`, `word_count` and
  `words_id` (worked out once per change, inside the store),
  `sb_enrolment`, `sb_sign_image`, the GPG
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
  `Part::NodeKey`. `Keys::lightning_node` returns the node itself. A Core Lightning hsm_secret gives the same `Node` (`hsm_secret::node_from_file`, `hsm_node::node_from_words`, `hsm_secret::node_from_secret`).
- **Keys made from fresh entropy.** `gpg::create` and
  `secure_boot::create` make the key and the record every vault of the
  plan takes, as a `vault::MadeKey`: its public fields through `view()`,
  a copy into each vault made inside the crate (`IntoRecord for
  &MadeKey`). They return the public parts beside it: the GPG facts and
  certificate, the owner GUID and the enrolment
  files. `gpg::Key`, `secure_boot::Keys` and `Pair` are the crate's own,
  with every function that takes them.
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
    `Keypair`, `PrivateKey`, `DerivedKey`, `SecNonce`,
    `SlotKey`, `to_seed`;
  - (b) seed words made into a `Mnemonic`, a SeedQR decoded, a SLIP-39
    split, a vault record's words read;
  - (c) the primitive crates' secret-taking functions and types: a GPG
    key, Secure Boot keys made or used, a vault opened, sealed,
    created or its slot key derived, a PSBT signed, BIP-352's receiver,
    an aezeed
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
  public set Practice takes (keys in every form Faraday reads, the testkit's
  wallets, a vault, a signed message), as data only: strings, bytes and
  fingerprints in the shapes `faraday_core::samples` defines. It derives
  nothing, so rule 4 has nothing to refuse there. The testkit and the
  samples crate are the two bodies of dummy data, and neither is key
  code: the testkit stays out of every shipped binary by
  `lint-keys-tests.sh`'s first two checks, and the samples crate by its
  third, which lets only Faraday Demo's two binaries depend on it: the
  desktop app, and the Starter, in whose window it runs (F126), through
  `faraday-desktop` alone. Its tests check
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
- it leaves through these sixteen named reasons;
- `just` fails if any other code touches a secret type.

Before F70 that took reading 36 files of faraday-core. As built, the
three statements are `docs/AUDIT.md` §2.

## 9. Questions for the owner

Answered 2026-10-21 (F70); the answers in force are in §2 (Q1, the vault's scope; Q2, the eleven reveals, fourteen since F125) and §6 (Q4, tests exempt), and the questions: moved to `docs/history/KEYS-LOG.md`.
