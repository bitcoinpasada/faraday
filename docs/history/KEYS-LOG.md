# The key boundary: faraday-keys: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/KEYS.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header (from docs/KEYS.md)

**Status:** Built 2026-10-21 (steps 0 to 4); each step's "as built" section is the record, §1 to §3 the plan they built
**Decisions:** F70

F70 decided that one crate, `faraday-keys`, owns every secret in memory,
and that faraday-core holds handles. This spec says how, step by step,
from an inventory of the tree at `dc4ebc6a` (2026-10-21). File and line
references are from that commit.

## §1 Where secrets are today (from docs/KEYS.md)

- **The session key.**
  - `wallet::Key` (`faraday/faraday-core/src/wallet.rs`) holds the
    master key, and `words` and `passphrase` as `pub
    Option<Zeroizing<String>>`.
  - About 30 sites read `key.words` or `key.passphrase` directly. Three of
    them draw: `screens/backup.rs`, and `glance/sheet.rs`, which pushes a
    plain `String` copy of the words.
- **Operations.** 36 faraday-core source files derive, sign, encrypt or
  decrypt with a secret:
  - PSBT and message signing;
  - BSMS key records;
  - BIP-85, Lightning (aezeed), silent payments, FROST dealing, MuSig
    nonces and vanity grinding;
  - GPG, APK and Secure Boot keys, and TOTP codes;
  - fingerprints, and the vault's key identity.
- **Vault contents.** An open vault's records stay decrypted in
  `vault::OpenVault.contents`, as `Zeroizing` fields:
  - KEY and PASSPHRASE records;
  - GPG, Secure Boot and APK private keys;
  - password entries and their TOTP secrets.

  The slot key that seals them again stays inside `faraday_vault::Opened`,
  which faraday-core never opens.
- **Transient input** is held in several places:
  - words and passphrases typed in `EntryState`, `KeyGen`, `SeedRead`,
    `CreateState`, `Potential` and the Lightning and tools forms;
  - dice and other entropy in `KeyGen` and `Vaults`;
  - camera frames in `KeyGen` and `ScanState`, wiped on drop;
  - scanned SeedQRs, which go to `SeedRead` or `KeyGen`;
  - Inbox files read from a stick.
- **Where secrets leave:**
  - words and passphrases on screen;
  - SeedQR, including a vault's held SeedQR button;
  - copy-by-hand checks;
  - paper forms: Seed XOR parts, codex32 strings and SLIP-39 shares;
  - a BIP-85 child shown, and a Lightning node key shown;
  - GPG paperkey;
  - a vault entry's password or TOTP code shown;
  - an unprotected file to the Outbox;
  - a backup flow writing words or a SeedQR picture to a stick.
- **Between processes.** On the device, the Inbox and Outbox wait in a RAM
  directory between app processes, secret files included
  (`faraday-storage`'s `Boxes::Dir`). That is by design, and stays.

Found along the way, and fixed in step 0 below:
1. **Outbox gate bypasses.** Two sites skip `secrets::secret_unprotected`
   and push a secret `Item` to the Outbox directly: `vault/out.rs:303`
   and `backup_flow/wallet.rs:621`. (A third, `memory.rs:380`, only
   restores the secret flag on items already in the Outbox when the boxes
   are loaded.)
2. **Plain `String` copies of secrets:**
   - the words in `glance/sheet.rs:1267` and `vault/screen.rs`
     (`words_text`);
   - the typed passphrase behind the eye toggle in `screens/sheets.rs:86`
     and `:257`.
3. **`inbox::Item` derives `Debug`.** Its `bytes` can be a secret file.
4. **GPG key seeds** are plain `[u8; 32]` in `gpg::key_of`. Only `paperkey`
   wipes them.
5. **lint-secrets barely covers faraday-core.** Its no-output and
   no-`Debug` rules cover only `word_entry.rs`, `quiz.rs` and `grind.rs`.

## §4 The steps: the plan (from docs/KEYS.md)

Each step is its own commit, with `just` green, the literal and test-list
checks, and a timed late-flow session for steps 2 and 3 (SPEED §7).

0. **Hygiene (in faraday-core, before the crate).**
   - Fix §1's five findings:
     - every secret reaches the Outbox through one function,
       `Faraday::outbox_secret` (done);
     - the plain copies become `Zeroizing`, or borrow;
     - `Item`'s `Debug` prints names and sizes only;
     - the GPG seeds are wiped;
     - lint-secrets' rules 1 and 2 cover all of faraday-core.
   - Small, and worth having even if F70 waited.
1. **Operations.**
   - Create faraday/faraday-keys (new).
   - Move every operation in §1's list into it, as functions taking the
     values they need: the master key, the record bytes.
   - faraday-core calls them.
   - `Key` stays where it is for now.
2. **Session keys.**
   - `Key` and `Session` move in, with private fields.
   - faraday-core's ~30 direct readers become `info`, operations or
     `reveal`.
   - `Faraday.session` becomes `Faraday.keys`.
   - Transient-input hand-offs call `add_*`.
3. **The vault.**
   - Unlock calls `keys.unlock(c, file, passphrase)`, which keeps
     `Opened` and the secret records (Q1), and returns the public view
     faraday-core needs.
   - Save calls `keys.seal(…)`.
   - The vault format does not change; the vault vectors and
     `tools/vault/open.py` prove it.
4. **The lint and the record.**
   - `lint-secrets.sh` refuses, outside faraday-keys, the crates that keep
     nothing, and `tests/`:
     - `MasterKey`, `SecretKey`, `Xpriv`, `Keypair` and `to_seed`;
     - `Mnemonic` construction from words;
     - the secret-taking functions of faraday-pgp, -sb, -apk and
       -vault (`open`, `seal`, `Opened`).
   - AUDIT.md is rewritten around the boundary.
   - SECURITY-MODEL gains a paragraph.

## §7 Size and cost (from docs/KEYS.md)

- **Step 0:** small, under an hour.
- **Step 1:** about 36 files touched, most of them a call replacing a
  block.
- **Step 2:** the ~30 direct readers.
- **Step 3:** the vault's unlock, load and save, `vault/mod.rs`'s largest
  paths.
- **In real time:** step 0 under an hour, steps 1 and 2 about 2 hours
  each, step 3 about 2–3 hours (more if Q1 is (a)), and step 4 about an
  hour with the AUDIT rewrite.

## §4.1 Left after step 2, closed by step 4: the two items step 4a closed (from docs/KEYS.md)

- **Public types from step 1 that hold a secret:** `silent_receiver`
  returned a `Receiver` with the scan key in it, and `gpg::Key`, `Dealt`
  (FROST) and `Node` (Lightning) were `pub`. Closed by step 4a.
- **`wallet_seed_bases`** built a words record only to read its
  fingerprint. Closed by step 4a.

## §9 Questions for the owner (from docs/KEYS.md)

Answered 2026-10-21: (a) the whole vault; all 11 reveal reasons; step 0
first; tests stay exempt.

1. **The vault's scope in step 3.**
   - (a) faraday-keys holds the whole open vault: every decrypted record,
     and the slot key. Passwords and notes also leave only through
     `reveal`. The boundary is one sentence: "every decrypted vault byte
     is in faraday-keys". The API is larger, because the vault's edit
     flows call it for each change. Recommended.
   - (b) Only what F70 names: KEY and PASSPHRASE records, and the secret
     fields of GPG, Secure Boot, APK and TOTP entries. Passwords and
     notes stay in faraday-core.
2. **The reveal list.**
   - F70 named four reasons: show the seed, copy by hand, SeedQR, and an
     unprotected file to the Outbox.
   - The tree has more:
     - the typed passphrase behind the eye toggle;
     - paper forms (Seed XOR, codex32, SLIP-39 shares);
     - a BIP-85 child shown;
     - a Lightning node key shown;
     - GPG paperkey;
     - a vault entry's password or TOTP code;
     - the backup flow writing words or a SeedQR picture to a stick.

   Approve the full list of 11 reasons, each a `Reveal` variant?
   Recommended, since each is a place a person already chooses to see or
   write a secret.
3. **Step 0 first**, as its own commit, before the crate? Recommended.
4. **Tests that build keys directly** stay allowed (`tests/` exempt)?
   Recommended. The alternative is to route them all through `testkit`.
