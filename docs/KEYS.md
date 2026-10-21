# The key boundary: faraday-keys

**Kind:** Spec
**Status:** Not started; the owner answered §9 on 2026-10-21 (the recommendation on all four)
**Decisions:** F70

F70 decided that one crate, `faraday-keys`, owns every secret in memory,
and that faraday-core holds handles. This spec says how, step by step,
from an inventory of the tree at `dc4ebc6a` (2026-10-21). File and line
references are from that commit.

## 1. Where secrets are today

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
1. **Outbox gate bypasses.** Three sites skip `secrets::secret_unprotected`
   and push a secret `Item` to the Outbox directly: `vault/out.rs:303`,
   `backup_flow/wallet.rs:621` and `memory.rs:380`.
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

## 2. What the boundary is

- **Key material** lives in `faraday-keys` and nowhere else:
  - a key's words, passphrase, seed and master key;
  - anything derived from them that is secret;
  - FROST and MuSig secret nonces;
  - GPG, APK and Secure Boot private keys;
  - TOTP secrets;
  - the open vault's decrypted records (the scope is Q1);
  - the slot key that seals them again.
- **Operations** on key material are functions of `faraday-keys`. They
  return public results: signatures, xpubs, addresses, fingerprints,
  signed files and codes.
- **Leaving.** Key material leaves only through `reveal(id, Reveal::…)`.
  It returns a guard that borrows the store and wipes its copy on drop.
  The list of reasons is closed (Q2).
- **Transient input** stays where it arrives, under lint-secrets' rules,
  until it is handed in:
  - typed words at Done;
  - a scanned SeedQR when it is read;
  - a vault passphrase at Open;
  - an Inbox secret file when it is loaded.
- **The crates that compute and keep nothing** stay as they are:
  faraday-bip, -psbt, -crypto, -backup, -vault, -pgp, -sb and -apk. Only
  faraday-keys calls their secret-taking functions.

The crate is faraday/faraday-keys (new). It sits beside faraday-vault,
faraday-pgp, faraday-sb and faraday-apk, which it uses, and holds no flow
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

- A BIP-85 child, a vanity find or a FROST share is a new secret. It comes
  back as a `Child` held inside the crate, which leaves only by `add_*` or
  `reveal`.
- **Drawing.** `View` gets `&Keys` read-only and reaches secret text only
  through `View::reveal`. That is one named accessor, and it joins
  COMPUTE's `Allowed` list beside `SecretShown`.
- **Event code** holds `&mut Keys` through `Faraday` as today, with the
  `keys` field in place of `session`.

## 4. The steps

Each step is its own commit, with `just` green, the literal and test-list
checks, and a timed late-flow session for steps 2 and 3 (SPEED §7).

0. **Hygiene (in faraday-core, before the crate).**
   - Fix §1's five findings:
     - the three Outbox bypasses go through one function;
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

## 5. What does not change

- No format: vaults, `.oskb`, KDBX, PSBTs and files written to sticks.
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
- `faraday-keys` gets its own `tests/`, one per operation group. Each says
  what a person or wallet would notice (a signature that verifies, a key
  that loads back from a vault) and checks against the vectors the
  primitive crates already use.

## 7. Size and cost

- **Step 0:** small, under an hour.
- **Step 1:** about 36 files touched, most of them a call replacing a
  block.
- **Step 2:** the ~30 direct readers.
- **Step 3:** the vault's unlock, load and save, `vault/mod.rs`'s largest
  paths.
- **In real time:** step 0 under an hour, steps 1 and 2 about 2 hours
  each, step 3 about 2–3 hours (more if Q1 is (a)), and step 4 about an
  hour with the AUDIT rewrite.

## 8. What it buys

An auditor reads faraday/faraday-keys (new) and the primitive crates, plus
the reveal list, and can state:
- every byte of key material lives in one crate;
- it leaves through these N named reasons;
- `just` fails if any other code touches a secret type.

Today that takes reading 36 files of faraday-core.

## 9. Questions for the owner

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
