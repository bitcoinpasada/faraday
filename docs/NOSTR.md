# The Nostr key

**Status:** spec v0.2 · 2026-10-14 · decided by the owner
(`docs/DECISIONS.md` F27; §12 records the answers). Not built, and not
scheduled: it is built only when the owner says so.

The problem it solves: a Nostr identity is one secp256k1 key, and a
person who loses it loses the identity. BIP-85 2.1.0 (2026-08-02) adds
application 128002', which derives Nostr keys from a Bitcoin seed, so
the seed's backup and two numbers are the Nostr key's backup too.
Faraday already derives six BIP-85 applications from a loaded key
(`faraday/faraday-core/src/bip85.rs`); this adds the seventh, on the
device only (the stick and the Pi).

Signing Nostr events is not part of this spec; it is a later item.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every dimension a
  token; labels, not prose, on working screens (the explanations are
  §9's Learn page); tests for user-facing behaviour only; `just` once at
  the end of the batch; no `rm`; no commits; no agents.
- Nothing in `core/` or `opensigner/` changes. OpenSigner lists the
  Nostr application as open (`docs/PLANNING.md` §16.114); an edit there
  would be drift.
- On screen the nsec is handled as BIP-85's WIF and extended private
  key are today (§4.4). Unlike them it has no **Out unprotected**: the
  ncryptsec (§6) is the only form in which it leaves the device, apart
  from the sealed vault file itself. A change that would need it
  handled otherwise stops and asks the owner.
- Nothing that can leave the device carries the key's fingerprint, or
  an identity or account number: no file name, no label under a code,
  no file's content. Files are named by the npub's first characters
  (§2.5). The fingerprint stays inside the vault (§5).
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## 1. What is decided

- Nostr keys come from BIP-85 application 128002' only, at
  `m/83696968'/128002'/{identity}'/{account}'`. No NIP-06
  (`m/44'/1237'/…`): NIP-06 is applied by a Nostr app to the words it is
  given, and its safe use is with words that hold nothing but Nostr,
  which Faraday already makes as a BIP-85 child seed. Faraday applying
  NIP-06 to the main seed gives nothing over 128002'.
- Identity and account are each 1 or more; 0 is reserved by the BIP.
  Both start at 1, filled in.
- It is a seventh application on Faraday's BIP-85 screen, and a Tools
  tile, **Nostr key**, opens that screen with Nostr chosen. Neither
  exists on Faraday Desktop or in the demo (§7).
- The screen shows the npub (public, with Show as QR and PNG), the
  nsec (secret, shown as WIF and xprv are) and the path. The plain nsec
  is never a QR or a picture anywhere: not on this screen, not in the
  vault, not in Files. **Find a pair** lists the npubs of the first
  five identities and accounts, so a forgotten pair can be matched by
  its npub.
- **Save into {vault}** writes an Entry: the npub as its username, the
  nsec as its password, the path and the key's fingerprint in its notes
  (§5). An Entry holding an nsec is not exported for KeePass (§5.3).
- **Export encrypted** writes the nsec as NIP-49 `ncryptsec1…` under a
  password typed twice, as text, a QR, a PNG and a file For the stick
  (§6). It is the only way the nsec leaves.
- The vault's saved Nostr entry offers Show as QR and PNG for the npub,
  and Export encrypted, so a saved key goes to a phone without being
  derived again (§5.1).
- A file holding an nsec is a secret kind with no code (§6.6); Faraday
  Companion refuses `nsec1` and `ncryptsec1` (§6.7). Both are in N1.
- The reading rules for `nsec`, `npub` and `ncryptsec` text are
  `faraday/faraday-intake/src/nostr.rs`, which the device and the
  Companion share (§2.4). The derivation and NIP-49 are
  `faraday/faraday-core/src/nostr.rs`, over `osk-bip`'s public key
  derivation and `osk-crypto`'s HMAC and scrypt; Bech32 is `bitcoin`'s.
  No new crate enters the graph (§2.3, §6.4).

## 2. The key

### 2.1 Derivation

Per the BIP's "Nostr" section (`tools/vectors/bip85/bip-0085.mediawiki`,
line 426):

1. Derive the private key at `m/83696968'/128002'/{identity}'/{account}'`
   from the loaded key's master.
2. `HMAC-SHA512("bip-entropy-from-k", k)` over its 32-byte private key:
   64 bytes of entropy.
3. The first 32 bytes are the secp256k1 secret key; the last 32 are
   discarded.
4. When those 32 bytes are zero or not below the curve order, the
   derivation fails, as the BIP's footnote says for the WIF
   application: the screen says "Not a valid key; use the next account"
   and shows no value. The chance is below 1 in 2¹²⁷.

Identity and account are each 1 to 2³¹ − 1 (`osk_bip::bip85::MAX_INDEX`).

### 2.2 Encoding

- **nsec**: NIP-19 Bech32 (not Bech32m) of the 32-byte secret key, HRP
  `nsec`, lower case.
- **npub**: NIP-19 Bech32 of the 32-byte x-only public key (BIP-340),
  HRP `npub`, lower case. The x-only key does not depend on the
  public key's y parity.
- Bech32 from `osk_bip::bitcoin::bech32` (bech32 0.11, already in the
  graph through `bitcoin`). The x-only key from `bitcoin`'s secp256k1,
  through the derived key's own context.

### 2.3 Where the code lives

`osk-bip`'s BIP-85 derivation (`derive`, its 64-byte `Bytes`) is
private, and its public functions are the six applications. Faraday
does not make it public, which would be an edit to `core/`. The public
pieces are enough, about twenty lines:

- `osk_bip::keys::MasterKey::derive(&DerivationPath)` →
  `DerivedKey::secret_key()` (and `DerivedKey::secp()`);
- `osk_crypto::hmac_sha512(b"bip-entropy-from-k", k)`;
- `osk_bip::bitcoin::secp256k1::SecretKey::from_slice` for step 4.

`faraday/faraday-core/src/nostr.rs` holds:

- `pub fn derive(master: &MasterKey, identity: u32, account: u32) ->
  Result<NostrKey, Error>`, with `Error::Reserved` for 0,
  `Error::IndexOutOfRange` and `Error::InvalidKey`;
- `NostrKey`: the 32-byte secret in a buffer that zeroizes on drop, with
  `npub()` and `nsec()` (the latter `Zeroizing<String>`); no `Debug`,
  no `Clone`;
- `pub fn path(identity, account) -> String`;
- the NIP-49 functions of §6.

Every intermediate (the derived private key, the 64 bytes, the 32-byte
secret, a `SecretKey` through `non_secure_erase`) is zeroized before
the function returns, as `osk-bip`'s own BIP-85 code does. When
OpenSigner adds the application, Faraday moves to its function and
`nostr.rs` keeps only the encodings.

### 2.4 Reading the encodings

`faraday/faraday-intake/src/nostr.rs` decides what reads as each kind,
for the vault page (§5.1), Files (§6.5, §6.6), KeePass export (§5.3)
and the Companion (§6.7). Faraday never takes a looser reading.

A string reads as an **nsec** when, with surrounding white space
trimmed:

- it is all lower case or all upper case (BIP-173);
- it decodes as Bech32, not Bech32m, with
  `CheckedHrpstring::new::<Bech32>` (not `bech32::decode`, which also
  takes Bech32m);
- its HRP is exactly `nsec`;
- its data is exactly 32 bytes;
- those bytes are a valid secp256k1 secret key: not zero, below the
  curve order.

An **npub** reads the same way with HRP `npub` and 32 bytes that are a
valid x-only public key. An **ncryptsec** reads the same way with HRP
`ncryptsec` and exactly 91 bytes whose first byte is `0x02`; its LOG_N
is its second byte. Bech32 0.11 checks the length against
`Bech32::CODE_LENGTH` (1023), not BIP-173's 90; the implementer
confirms this for the 162-character ncryptsec and stops if it does not
hold.

### 2.5 The short name

`{n}` below is the npub's first 13 characters: `npub1` and the next
eight, as `npub1qqqsyqcy`. It names every Nostr file Faraday writes and
labels every Nostr code. It is public, as the npub is, and says nothing
about the seed, the fingerprint, the identity or the account.

## 3. Where it starts

Nothing in this section exists while `Faraday::online` is set (Faraday
Desktop, on every network) or the session is the demo
(`Session::demo`, which the demo website runs): §7.

- **The BIP-85 screen** (`Screen::Bip85`): Nostr is the seventh row of
  the Application card, after OpenSigner's six.
  `opensigner_core::Bip85App` is OpenSigner's and is not extended:
  `bip85.rs` gets its own `App` (`Os(Bip85App)` or `Nostr`), and the
  list is OpenSigner's six, then Nostr where §7 allows it. The name is
  Faraday's string, the others stay OpenSigner's.
- **The Tools tile** (`catalog.rs`): **Nostr key**, in Keys and seeds
  after Child seeds and passwords, its line "Derived from a key here"
  (the existing `catalog_derived_from`), its tags `BIP-85`, `NIP-19`,
  `NIP-49`. `Go::Nostr` needs a loaded key exactly as `Go::Bip85` does
  ("Load a key"). It opens the BIP-85 screen with Nostr chosen and the
  Application card done: with one key loaded, the Identity and account
  card is open; with several, the Key card is. The column's back label
  names where it returns to: "Tools" from the tile, "Wallets" from a
  wallet, as `back` already decides where it goes.

## 4. The screen

The BIP-85 step-card column, unchanged for the other six applications.
With Nostr chosen:

### 4.1 Key

As today: the loaded keys, one picked.

### 4.2 Application

Nostr has no length: no −/+ row. Its summary is "Nostr".

### 4.3 Identity and account

Takes the place of the Index card; its title is "Identity and
account".

- Two rows, **Identity** and **Account**, each with −, the field and +,
  as the Index card's row. Both start at 1, filled in. − stops at 1, +
  at 2³¹ − 1.
- Digits typed go to the field last tapped, Identity first; Tab moves
  between them; Enter is Continue, as today. A field takes at most ten
  digits.
- A field that is 0 or empty shows "Identity and account start at 1"
  in the error colour; a field above 2³¹ − 1 shows "At most
  2147483647". In either case Continue does nothing.
- The closed card's summary is "Identity 1 · account 1".
- **Find a pair** turns the card's body into the list below; **Type a
  pair** turns it back.

**Find a pair** is DESIGN §5's Menu, a Navigate list, with §4.5's
reference-row value (the npub elided to its first and last 8, in groups
of four), as Silent payments' Labels list is. In Faraday's column it is
drawn with `button_rows`, the row the Key card already uses, one line
per row: the pair, then the elided npub, "1 · 1 npub1qqq … 2wxy". It
lists:

- under "Account 1", identities 1 to 5 at account 1;
- under "Identity {n}", accounts 1 to 5 at the identity in the
  Identity field, leaving out a row already listed above. An Identity
  field that is empty, 0 or above 2³¹ − 1 counts as 1 here, so the
  list is then nine rows.

Nine or ten rows, worked out when the list opens and kept, as npub text
only, until it closes, the screen is left or the session locks. Each
secret behind them is zeroized as soon as its npub is made. A tap fills
both fields with that pair, marks the card done and opens the Value
card. No nsec is derived for a row until its pair is picked and the
Value card shows it.

### 4.4 Value

In order:

- **Path** `m/83696968'/128002'/1'/1'`, as today.
- **npub**: the full npub in clear (public), then **Show as QR** and
  **PNG** on its row, as GPG's certificate has them
  (`Action::ShowCode` and `Action::CodePng` over a new
  `Code::NostrNpub`). The npub is worked out when the Value card opens
  and kept in the flow's state as text only, as Find a pair's rows are;
  the key behind it is zeroized at once. The PNG, For the stick, is
  `{n}.png`, labelled "npub · {n}…" under the code.
- **nsec**: masked, "••••••••••••", until **Show**; derived for the
  frame it is drawn in and wiped with it, as WIF and xprv are today.
  No QR, no PNG, no copy, no file: the WIF and the xprv have no code
  either, and no `Code` variant ever carries an nsec.
- The actions: **Show**/**Hide**, **Save into {vault}** (or the way to
  make or unlock one, `docs/FLOWS.md` decision 10) and **Export
  encrypted…** (§6). There is no **Out unprotected…** for Nostr: the
  secret sheet is never offered an nsec.

Leaving the screen and locking drop the flow's state as they do today
(`Faraday::lock_now`, the screen change's `bip85 = None`). The state
holds no secret except the export prompt's two fields while it is open
(§6.1), which are wiped with it. A change of key, identity or account
drops the npub text, an open export prompt and a shown ncryptsec.

None of the Nostr cards carries a guide line (`flow::Card::guide` is
`None`): the Learn page (§9) is where the explanation is.

## 5. Saving to the vault

An **Entry** (record type 6, `docs/VAULT.md` §7). No new record type.

| Field | Holds |
|---|---|
| 1 title | "BIP-85 Nostr · {fp} · identity {i} · account {a}" |
| 2 username | the npub |
| 3 password | the nsec |
| 5 notes | the path, then "Key {fp}", a line each |

The fingerprint, identity and account appear here because the vault is
sealed and never leaves except as the whole vault file; nothing taken
from the entry (a code, a PNG, an ncryptsec file) carries the title or
the notes (§5.1, §6.1).

Why an Entry and not a Note, as WIF and xprv are:

- The vault's view shows an Entry's username in clear and holds its
  password behind the reveal (`vault_screens.rs`); a Note is one held
  text, so the npub would be hidden with the nsec.
- The fields are separate, so the npub, the nsec and the path are read
  without parsing a note.
- BIP-85's passwords are already Entries.

The record carries everything that makes the key again without the
vault: the fingerprint names the seed, the path holds the application,
identity and account. The nsec is recoverable from the seed's words and
the two numbers, so a lost vault does not lose it; the two numbers are
written down with the seed's backup (§9).

**The backup flow** (`docs/BACKUP-FLOW.md`, F31). **Save into a
vault** is the shared backup flow: where the entry will be kept (any
number of places, none included, with no warning), each place's stick
brought in with nothing secret open, the vaults unlocked or made, then
the entry saved into every vault the plan keeps it in, one lock, and
each stick written by place. The key's seed must be loaded at the save:
the unlock loads the keys its vaults hold, any other is loaded as a key
is, and the npub is compared with the one chosen. With no place, the
entry goes into the vaults already open and no stick is written, as
Save into does today. The saved entry's page has the row **Backup**
with **Continue the backup** until every place is written. Nothing waits
for it: the key comes back from the seed's words and the two numbers,
so **Export encrypted**, Show as QR and PNG work at once. Whichever of
batch N1 and the flow's batch B1 is built second adds this.

### 5.1 The saved entry's page

Today an Entry's page in Vault contents (`vault_screens.rs`, the
`kind::ENTRY` arm) shows the username, URL and notes as plain rows, the
password and TOTP secret as held rows revealed by a hold, the live TOTP
code, and two actions, **Edit** and **Export for KeePass**. Since F28
(`docs/QR.md` §7) the TOTP secret also has **Hold to show as QR**: its
`otpauth://` URI on the QR sheet marked Secret, opened by a hold, with
no PNG and no file. Its usual destination, an authenticator app, takes
it only by scanning. That is the one secret field of an Entry with a
code. The password field has none, and none is added: there is no
generic per-field QR for the nsec to leak through. The other codes on a
vault page are GPG's certificate and signatures
(`Code::GpgKey`, `GpgSignature`; the revocation has no code, F32) and, since F28, a
recovery sheet's descriptor.

A **Nostr entry** is an Entry whose password reads as an nsec (§2.4)
and whose username reads as an npub equal to that nsec's npub. Its page
adds, after Edit:

- **Show as QR** and **PNG** for the npub, over `Code::NostrNpub`, the
  PNG named `{n}.png` and labelled "npub · {n}…", never after the
  entry's title;
- **Export encrypted…**, §6's flow over the entry's nsec, ending on the
  same ncryptsec text, **Show as QR**, **PNG** and **For the stick**,
  named as §6.1 names them;
- **Path**, a row under the notes, only when a loaded key's
  fingerprint is the one on the notes' "Key {fp}" line and the notes
  hold a 128002' path: Faraday derives that path from that key and
  shows "Path checked", or "Path does not give this key" in the error
  colour when the npub differs. With no such key loaded the row is not
  shown.

The recognition is by content, not by the notes' path: notes are
edited freely, and an nsec typed into a new entry by hand is as much a
Nostr key as a derived one. The npub check keeps a mistyped or edited
username from being shown as the key's npub. Edit works as on any
Entry: an entry whose nsec or npub is edited so that they no longer
match is an ordinary Entry, without these actions, until they match
again. Nothing more general is added: a QR of any Entry's username or
URL would help little, and a QR of the password field would put a
secret in a code, which §12's item 8 rules out for the nsec. The
password row stays a held row, revealed by a hold, with no code.

### 5.2 An nsec file into the vault

A file Files reads as holding an nsec (§6.6) offers **Save into
{vault}**, which writes an Entry: title "Nostr · {n}", the npub as its
username, the nsec as its password, no notes. It is then a Nostr entry
like a derived one.

### 5.3 No KeePass export of an nsec

**Export for KeePass** is not offered on an Entry whose password reads
as an nsec (§2.4), whether or not it is a Nostr entry, and
`vault_kdbx` refuses such an entry if it is reached another way. The
whole entry stays out: its title and notes hold the fingerprint and the
path, and a KeePass file goes to an online password manager. The
encrypted export (§6) is the only way the nsec leaves.

## 6. Encrypted export (NIP-49)

### 6.1 The flow

**Export encrypted…** on the Value card, or on a Nostr entry's page,
opens the flow's own prompt in place of the actions: two masked fields,
**Password** and **Again**, then **Export** and **Cancel**.

- Typing goes to the focused field, Password first; Tab moves between
  them; Enter in Password moves to Again; Enter in Again is Export.
- Each field is a `TextBox` (wiped when dropped). The prompt is
  `NostrPrompt` in the flow's state, not `vaults.prompt`, which is one
  line in Vault contents and acts on the selected entry. Cancel,
  leaving the screen, a change of key or pair, and a lock drop it and
  wipe both fields.
- Export checks, in order, and shows the first that fails under the
  fields in the error colour, keeping what is typed:
  1. Password is printable ASCII (`0x20`–`0x7e`): otherwise "Use
     letters, digits and symbols on the keyboard".
  2. Password is at least 12 characters, as Amethyst requires when it
     makes one: otherwise "At least 12 characters".
  3. Again is the same: otherwise "The two differ".
- NIP-49 sets no minimum, so the 12 is a rule for making an ncryptsec
  only. NIP-49 asks for NFKC normalisation; printable ASCII is
  unchanged by it, so Faraday needs no Unicode tables.
- With no randomness from the system yet (`Faraday::fresh` returns
  `None`), the prompt says "No randomness from the system yet. Try
  again" (Faraday's existing `vaults_no_randomness`) and nothing is
  made.
- The work runs as KeePass export's does (`vaults.working` with its
  `drawn` flag): one frame showing "Encrypting" is drawn first, then
  scrypt runs.

The result replaces the prompt with:

- **ncryptsec**: the string, in full, in clear (it is sealed);
- **Show as QR** and **PNG**, over `Code::Ncryptsec`: one code (162
  characters); the PNG is `{n}-ncryptsec.png`, labelled "ncryptsec ·
  {n}…", and is sealed as the text is (§6.5);
- **For the stick**: `{n}.ncryptsec.txt`, the string and a newline,
  into the Outbox as a sealed file.

No file name or label carries the fingerprint, the identity or the
account (§0).

Before any of it is shown, Faraday decrypts what it wrote, from the
string and the password through the full NIP-49 path (Bech32, the
fields, scrypt, XChaCha20-Poly1305), and compares the key; a mismatch
shows nothing and says "Export failed". The decrypted copy is zeroized
after the comparison.

### 6.2 The format

As NIP-49 writes it:

| Part | Value |
|---|---|
| Version | `0x02` |
| LOG_N | 16 (N = 65536; r = 8, p = 1): 64 MiB, NIP-49's own first row |
| Salt | 16 random bytes |
| Nonce | 24 random bytes |
| Key-security byte | `0x02`, also the associated data |
| Ciphertext | XChaCha20-Poly1305 of the 32-byte secret, 48 bytes |

- The symmetric key is `scrypt(password, salt, N, 8, 1)`, 32 bytes, from
  `osk_crypto::scrypt`. The caller zeroizes it after use
  (`osk_crypto::scrypt` zeroizes its own working memory, not its
  output), and zeroizes the password's copy, the plaintext secret and
  the decrypted copy.
- The 40 random bytes come from one fresh draw (`Faraday::fresh`,
  labelled `ncryptsec`), spread over the bytes by SHA-256 as the KeePass
  export spreads its draw. Every export draws again, so no salt or
  nonce is reused.
- Bech32 with HRP `ncryptsec` over the 91 bytes (§2.4 on the length).

### 6.3 The key-security byte

NIP-49's values: `0x00` the key is known to have been handled
insecurely, `0x01` it is not known to have been, `0x02` the client does
not track this. Faraday writes `0x02`. The same nsec comes from the seed
and two numbers in any BIP-85 implementation, in any session, and
Faraday cannot know what was done with it there, so `0x01` is a claim it
cannot make; `0x00` would be untrue of most keys.

### 6.4 Dependencies, time and memory

| Need | Where it comes from | Cost |
|---|---|---|
| scrypt | `osk_crypto::scrypt`, written in the tree for LND's aezeed (`docs/PLANNING.md` §16.116) | None. No `scrypt` crate is in `Cargo.lock`. |
| XChaCha20-Poly1305 | `chacha20poly1305` 0.11, already used by `osk-crypto`, `osk-backup`, `faraday-vault` and others (`docs/deps/chacha20poly1305.md`) | None new: `faraday-core` adds it as a workspace dependency. |
| Bech32 | `bitcoin`'s re-export of `bech32` 0.11 | None. |
| NFKC | not needed (printable ASCII only) | `unicode-normalization` is in the graph only as `osk-bip`'s dev-dependency; not added. |

An export runs scrypt twice at LOG_N 16: once to encrypt, once for the
check. Each holds 64 MiB, the same as a Light vault's unlock, freed and
zeroized before the next. `osk_crypto::scrypt` allocates with `vec!`,
which ends the app if the allocation fails; `core/` is not edited for
this, and a Pi 3's 1 GiB holds it, since a vault's own key-derivation
memory is freed once the vault is open. Time is an estimate, not a
measurement: under a second on a desktop, several seconds on a Pi 3.
The completion report gives the time measured on the build machine; the
Pi 3's is taken at the owner's next Pi trial.

### 6.5 Reading the file back

A text file whose trimmed text is one string that reads as an
ncryptsec (§2.4) is `FileKind::Ncryptsec`. Its exposure is
`secrets::Exposure::Sealed` when its LOG_N is 16 or more, as a vault or
an encrypted backup is: it goes out as it is and is kept across a lock.
Below 16 it is `Exposure::Secret`: the password costs too little to
guess to count as a seal. The kind is read from the header bytes alone;
no scrypt runs to classify a file.

Faraday does not decrypt an ncryptsec it did not just make. There is no
import: `nostr::decrypt` refuses any LOG_N but 16 (`Error::Cost`) and
is called only by §6.1's check and by the tests, so a foreign file's
LOG_N never sets how much memory Faraday asks for. (Recommended here;
§13 asks the owner to confirm.)

### 6.6 A file holding an nsec

A text file, not read as any kind before it in `wallet::classify`
(after Entries and seed parts, before Text), with a white-space
separated word that reads as an nsec (§2.4), is
`FileKind::NostrSecret`: exposure Secret, `may_be_secret`, dropped
from the Inbox at a lock as the other secret kinds are. Files offers
no **Show as QR** and no **PNG** of it, and does not offer it to the
secret sheet, so it never goes to the Outbox; it goes into the vault
as §5.2 says. Faraday itself never writes such a file.

### 6.7 Faraday Companion

The Companion refuses an nsec and an ncryptsec, as it refuses other
private keys and sealed files (`docs/COMPANION.md` §5):
`faraday_intake::Secret` gains `NostrSecret` and `NostrEncrypted`,
found by §2.4's rules in any text that arrives by camera, paste or
share, and in a Faraday file envelope's file. A file holding either is
refused whole. The refusal lines are "A Nostr secret key stays on the
Faraday device." and "An encrypted Nostr key stays on the Faraday
device." N1 adds both kinds to `docs/COMPANION.md` §5's refused row.

An ncryptsec's QR is for the person's Nostr app, which scans it
itself; the Companion is not the way it reaches a phone.

## 7. Faraday Desktop and the demo

No Nostr key on Faraday Desktop, on any network, and none in the demo.
While `Faraday::online` is set (Faraday Desktop) or `Session::demo` is
on (Faraday Desktop's mainnet and the demo website):

- the **Nostr key** tile is not in Tools, and Find a tool does not
  find it;
- the BIP-85 Application card lists OpenSigner's six only;
- a Nostr entry in a vault opened there is an ordinary Entry: its page
  adds no Show as QR, PNG, Export encrypted or Path row. §5.3 still
  holds: it is not exported for KeePass.

Faraday Desktop runs on an online computer, and BIP-85 128002' does not
depend on the network: a test network would show the person's real
Nostr key there. The demo's public seeds would give Nostr keys anyone
can derive.

§6.5 and §6.6 apply on Faraday Desktop too, since they only classify
files.

## 8. Strings

In `faraday/faraday-strings/src/en.rs`, Faraday's file; none in
OpenSigner's. The implementer adds them; the values are the labels in
this spec.

| Key | Value |
|---|---|
| `bip85_app_nostr` | Nostr |
| `bip85_identity_and_account` | Identity and account |
| `bip85_identity` | Identity |
| `bip85_account` | Account |
| `bip85_identity_account` | Identity {} · account {} |
| `bip85_start_at_1` | Identity and account start at 1 |
| `bip85_at_most` | At most 2147483647 |
| `bip85_not_a_valid_key` | Not a valid key; use the next account |
| `bip85_find_a_pair` | Find a pair |
| `bip85_type_a_pair` | Type a pair |
| `bip85_account_1` | Account 1 |
| `bip85_identity_n` | Identity {} |
| `bip85_pair` | {} · {} |
| `bip85_npub` | npub |
| `bip85_nsec` | nsec |
| `bip85_show_as_qr` | Show as QR |
| `bip85_tools` | Tools |
| `bip85_nostr_title` | BIP-85 Nostr · {} · identity {} · account {} |
| `bip85_nostr_file_title` | Nostr · {} |
| `bip85_key_fp` | Key {} |
| `bip85_export_encrypted` | Export encrypted… |
| `bip85_password` | Password |
| `bip85_again` | Again |
| `bip85_export` | Export |
| `bip85_two_differ` | The two differ |
| `bip85_encrypting` | Encrypting |
| `bip85_ncryptsec` | ncryptsec |
| `bip85_for_the_stick` | For the stick |
| `bip85_npub_label` | npub · {}… |
| `bip85_ncryptsec_label` | ncryptsec · {}… |
| `bip85_printable_only` | Use letters, digits and symbols on the keyboard |
| `bip85_password_too_short` | At least 12 characters |
| `bip85_export_failed` | Export failed |
| `bip85_path` | Path |
| `bip85_path_checked` | Path checked |
| `bip85_path_differs` | Path does not give this key |
| `catalog_nostr_key` | Nostr key |
| `companion_refuse_nsec` | A Nostr secret key stays on the Faraday device. |
| `companion_refuse_ncryptsec` | An encrypted Nostr key stays on the Faraday device. |

## 9. Learn page

One page, `docs/learn/faraday/nostr.md`, id `nostr` in
`faraday-learn`'s `OWN`, opened from the Nostr tile's info and the
BIP-85 screen with Nostr chosen. Outline, a heading each:

- **What the Nostr key is**: one secret key, the nsec, and its public
  key, the npub, which names the identity; whoever has the nsec signs
  as it.
- **It comes from the seed and two numbers**: BIP-85 application
  128002', identity and account, each from 1; the same seed and numbers
  give the same key in any program that follows BIP-85.
- **Write the two numbers down** with the seed's backup. The seed's
  words and the two numbers make the nsec again; a lost vault or phone
  does not lose it. Find a pair matches a forgotten pair by its npub.
- **Unlinkable**: nothing in an npub ties it to the seed's Bitcoin
  addresses or to another identity's npub; only the seed does. Faraday
  names the files it writes by the npub, never by the key's
  fingerprint or the two numbers.
- **Identities and accounts**: an identity per separate public life,
  accounts within one; 0 of each is reserved by the BIP.
- **Encrypted export**: what an ncryptsec is, that the password is
  needed to use it, that Nostr apps that import NIP-49 read it, and
  that it is the only way Faraday lets the nsec out.
- **Only on the device**: Faraday Desktop and the demo have no Nostr
  key, and why (§7).
- **Why not NIP-06**: §1's reason, in a paragraph.

Working screens carry none of this (`CLAUDE.md`).

## 10. Tests

User-facing behaviour and published vectors (`CLAUDE.md`).

### 10.1 Vectors

`tools/vectors/bip85/` is OpenSigner's (`core/osk-bip/tests/bip85.rs`
reads it) and is not edited: its extractor skips Nostr, and adding it
there would be drift. Faraday's are a new folder, `tools/vectors/nostr/`,
as `tools/vectors/vault/` is Faraday's:

- `extract.py` reads `../bip85/bip-0085.mediawiki`'s Nostr section (its
  three cases sit under bold lines, not headings) into `vectors.json`'s
  `bip85` list: path, identity, account, entropy, nsec.
- `npub.py`, standard library only (curve arithmetic and Bech32 written
  out), adds each case's npub: an implementation independent of
  libsecp256k1 and rust-bech32. The BIP publishes no npub.
- NIP-19's published hex and Bech32 examples, and NIP-49's decryption
  vector (password `nostr`, LOG_N 16), copied with their source URL,
  commit and date.
- `README.md` with the SHA-256 table, as `tools/vectors/bip85/README.md`
  has.

### 10.2 Faraday's tests

In `faraday/faraday-core/tests/nostr.rs`, unless named otherwise:

- every BIP vector (all three) through `faraday_core::nostr::derive`
  from the BIP's master key gives the BIP's nsec and `npub.py`'s npub;
- NIP-19's examples encode as published;
- §2.4's reading: an upper-case nsec reads; a mixed-case one, a Bech32m
  one, one with another HRP, one of 31 or 33 bytes and one whose bytes
  are zero do not;
- identity 0 or account 0 is refused, by the function and on the card
  (Continue does nothing, the error line shows), and so is 2³¹;
- the Tools tile opens the BIP-85 screen with Nostr chosen, the
  Identity and account card open at 1 and 1; with no key it reads "Load
  a key";
- Find a pair lists identities 1 to 5 at account 1 and accounts at the
  typed identity, nine rows with the field empty, and a tap fills the
  pair and opens the Value card;
- the Value card shows the npub with the nsec masked until Show, and
  offers no Out unprotected;
- Save into the vault, lock, unlock: the vault's entry shows the same
  npub and the path, and its password is the nsec;
- a vault entry holding an nsec offers no Export for KeePass, with a
  matching npub or without;
- NIP-49's vector decrypts to its published key (its password is 5
  characters: the 12 is for making only); an export decrypts with its
  password to the same nsec and fails with another; its LOG_N is 16 and
  its key-security byte 0x02; two exports of one key under one password
  differ; `decrypt` refuses LOG_N 15 and 17;
- the export prompt refuses a password with a character outside
  printable ASCII, one of 11 characters, and an Again that differs;
  a lock with the prompt open drops it;
- the npub's and the ncryptsec's Show as QR decode (with `zbarimg` or
  `rqrr`) to the npub and the ncryptsec, and their PNGs are written For
  the stick as `{n}.png` and `{n}-ncryptsec.png`, the text as
  `{n}.ncryptsec.txt`: no name holds the fingerprint, and no label
  under a code holds it or the two numbers;
- an ncryptsec file reads as sealed at LOG_N 16 and as a secret at 15;
- a file holding an nsec is a secret kind in Files with no Show as QR,
  no PNG and no way to the Outbox, and Save into the vault makes a
  Nostr entry;
- no screen of the flow, no vault page and no Files page offers a code
  or a PNG whose content is the nsec;
- a saved Nostr entry's page offers Show as QR for its npub and Export
  encrypted, whose ncryptsec decrypts to the entry's nsec; an Entry
  whose username is not the nsec's npub offers neither; with the key
  loaded, an entry whose notes' path was edited shows "Path does not
  give this key";
- with `online` set, on mainnet and on testnet, and in the demo: no
  Nostr key tile, six applications, and a Nostr entry's page with no
  Nostr actions;
- in `faraday/faraday-companion/tests/refused.rs`'s samples: an nsec
  and an ncryptsec, by camera, paste and share, refused with their
  lines and nothing kept.

## 11. Batch

Not scheduled. The owner says when it is built; nothing here puts it
before any other work.

- **N1, the Nostr key.** §2's `nostr.rs` files, in `faraday-core` and
  `faraday-intake`; §3's application and tile; §4's cards, Find a pair
  and the Value card; §5's vault entry, §5.1's page actions and Path
  row, §5.2 and §5.3; §6's encrypted export, its prompt and the file's
  exposure; §6.6's file kind; §6.7's Companion refusals, with
  `docs/COMPANION.md` §5's row; §7's rules for Faraday Desktop and the
  demo; §8's strings; §9's Learn page; §10's vectors and tests; the
  Nostr rows of `docs/QR.md` (§7's table) brought to these file names.
  Renders: the tile, the Identity and account card, Find a pair, the
  Value card, the export prompt and a saved entry's page at `1280x800`
  and `480x640`. It touches the device app and the Companion, so both
  are built after it is committed.

## 12. Decided by the owner (2026-10-14)

1. BIP-85 128002' only; no NIP-06.
2. A BIP-85 application and a Tools tile.
3. Identity and account start at 1/1, filled in.
4. NIP-49 export is part of the batch.
5. Find a pair; its rows stay text.
6. Specified now, built when the owner says.
7. Superseded by 13.
8. No QR or picture of the plain nsec anywhere (§4.4, §5.1, §6.6); the
   ncryptsec's QR moves the key to a phone under a password.
9. QR codes wherever they help: Show as QR and PNG for the npub and the
   ncryptsec, in the tool and on a saved entry's page (§4.4, §5.1, §6.1).
10. Faraday Companion refuses `nsec1` and `ncryptsec1`, as it refuses
    other private keys and sealed files, in N1 (§6.7).
11. A new ncryptsec's password is at least 12 characters.

After the review of v0.1, the same day:

12. The nsec is left out of KeePass export: an Entry holding one is not
    exportable for KeePass (§5.3). The encrypted export is the only way
    the nsec leaves; so there is no Out unprotected for it either (§4.4).
13. No Nostr key on Faraday Desktop, on any network, nor in the demo
    (§7).
14. No fingerprint, identity or account in any file name or label that
    can leave the device; files are named by the npub's first
    characters; the fingerprint stays inside the vault (§0, §2.5).
15. The export prompt asks for the password twice (§6.1).

## 13. For the owner

1. Faraday reads no ncryptsec it did not make (§6.5): recommended,
   since reading one needs a cap on its LOG_N and risks the memory for
   little use. Confirm, or say what reading one would be for.
2. "Encrypted export is the only way the nsec leaves" was applied to
   Out unprotected as well as to KeePass export (§12 item 12). Say if
   Out unprotected should stay.
