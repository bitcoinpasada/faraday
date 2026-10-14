# The Nostr key

**Status:** spec v0.1 · 2026-10-14 · decided by the owner
(`docs/DECISIONS.md` F27; §12 records the answers). Not built, and not
scheduled: it is built only when the owner says so.

The problem it solves: a Nostr identity is one secp256k1 key, and a
person who loses it loses the identity. BIP-85 2.1.0 (2026-08-02) adds
application 128002', which derives Nostr keys from a Bitcoin seed, so
the seed's backup and two numbers are the Nostr key's backup too.
Faraday already derives six BIP-85 applications from a loaded key
(`faraday/faraday-core/src/bip85.rs`); this adds the seventh.

Signing Nostr events is not part of this spec; it is a later item.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every dimension a
  token; labels, not prose, on working screens (the explanations are
  §9's Learn page); tests for user-facing behaviour only; `just` once at
  the end of the batch; no `rm`; no commits; no agents.
- Nothing in `core/` or `opensigner/` changes. OpenSigner lists the
  Nostr application as open (`docs/PLANNING.md` §16.114); an edit there
  would be drift.
- The nsec is handled as BIP-85's WIF and extended private key are
  today (§4.4). A change that would need it handled otherwise stops and
  asks the owner.
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
  tile, **Nostr key**, opens that screen with Nostr chosen.
- The screen shows the npub (public, with Show as QR and PNG), the
  nsec (secret, as WIF and xprv are) and the path. The plain nsec is
  never a QR or a picture anywhere: not on this screen, not in the
  vault. **Find a pair** lists the
  npubs of the first five identities and accounts, so a forgotten pair
  can be matched by its npub.
- **Save into {vault}** writes an Entry: the npub as its username, the
  nsec as its password, the path and the key's fingerprint in its notes
  (§5).
- **Export encrypted** writes the nsec as NIP-49 `ncryptsec1…` under a
  password, as text, a QR, a PNG and a file For the stick (§6).
- The vault's saved Nostr entry offers Show as QR and PNG for the npub,
  and Export encrypted, so a saved key goes to a phone without being
  derived again (§5.1).
- On Faraday Desktop's mainnet demo, the npub and nsec are shown with a
  broken checksum, as F20 shows addresses, and Export encrypted is
  refused (§7).
- The code is `faraday/faraday-core/src/nostr.rs`, over `osk-bip`'s
  public key derivation and `osk-crypto`'s HMAC and scrypt; Bech32 is
  `bitcoin`'s. No new crate enters the graph (§2.3, §6.4).

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
  `nsec`.
- **npub**: NIP-19 Bech32 of the 32-byte x-only public key (BIP-340),
  HRP `npub`.
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
secret) is zeroized before the function returns, as `osk-bip`'s own
BIP-85 code does. When OpenSigner adds the application, Faraday moves
to its function and `nostr.rs` keeps only the encodings.

## 3. Where it starts

- **The BIP-85 screen** (`Screen::Bip85`): Nostr is the seventh row of
  the Application card, after OpenSigner's six.
  `opensigner_core::Bip85App` is OpenSigner's and is not extended:
  `bip85.rs` gets its own `App` (`Os(Bip85App)` or `Nostr`), and
  `APPS` becomes its seven, OpenSigner's six first. The name is
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
  between them; Enter is Continue, as today.
- A field that is 0 or empty shows "Identity and account start at 1"
  in the error colour, and Continue does nothing.
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
  Identity field, leaving out a row already listed above.

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
  `Code::NostrNpub`). The PNG, For the stick, is
  `nostr-{fp}-{identity}-{account}-npub.png`, labelled "npub · identity
  {i} · account {a}" under the code.
- **nsec**: masked, "••••••••••••", until **Show**; derived for the
  frame it is drawn in and wiped with it, as WIF and xprv are today.
  No QR, no PNG, no copy: the WIF and the xprv have none, and no `Code`
  variant ever carries an nsec.
- The actions: **Show**/**Hide**, **Save into {vault}** (or the way to
  make or unlock one, `docs/FLOWS.md` decision 10), **Export
  encrypted…** (§6), **Out unprotected…**.

Leaving the screen and locking drop the flow's state as they do today
(`Faraday::lock_now`, the screen change's `bip85 = None`); the state
holds no secret.

**Out unprotected…** goes through the secret sheet as WIF and xprv do:
the file `bip85-{fp}-nostr-{identity}-{account}.txt` holds the path,
the npub and the nsec, a line each; the sheet names it "A Nostr secret
key" and says "Whoever has it can sign as this Nostr identity and read
its direct messages".

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

Why an Entry and not a Note, as WIF and xprv are:

- The vault's view shows an Entry's username in clear and holds its
  password behind the reveal (`vault_screens.rs`); a Note is one held
  text, so the npub would be hidden with the nsec.
- The fields are separate, so the npub, the nsec and the path are read
  without parsing a note.
- KeePass export already carries an Entry, with the nsec as the
  password, for a Nostr app or signer that reads KeePass.
- BIP-85's passwords are already Entries.

The record carries everything that makes the key again without the
vault: the fingerprint names the seed, the path holds the application,
identity and account. The nsec is recoverable from the seed's words and
the two numbers, so a lost vault does not lose it; the two numbers are
written down with the seed's backup (§9).

### 5.1 The saved entry's page

Today an Entry's page in Vault contents (`vault_screens.rs`, the
`kind::ENTRY` arm) shows the username, URL and notes as plain rows, the
password and TOTP secret as held rows revealed by a hold, the live TOTP
code, and two actions, **Edit** and **Export for KeePass**. No row has a
QR or a PNG: the only codes on a vault page are GPG's certificate,
revocation and signatures (`Code::GpgKey`, `GpgRevocation`,
`GpgSignature`). So there is no generic per-field QR for the nsec to
leak through, and none is added.

A **Nostr entry** is an Entry whose password reads as an `nsec1…` with
a valid checksum and whose username is the npub of that nsec. Its page
adds, after Edit:

- **Show as QR** and **PNG** for the npub, over `Code::NostrNpub`, the
  PNG named after the entry's title;
- **Export encrypted…**, §6's flow over the entry's nsec, ending on the
  same ncryptsec text, **Show as QR**, **PNG** and **For the stick**.

The recognition is by content, not by the notes' path: notes are
edited freely, and an nsec typed into a new entry by hand is as much a
Nostr key as a derived one. The npub check keeps a mistyped or edited
username from being shown as the key's npub. Nothing more general is
added: a QR of any Entry's username or URL would help little, and a
QR of a password field would put a secret in a code, which §12's
item 8 rules out for the nsec.
The password row stays a held row, revealed by a hold, with no code.

## 6. Encrypted export (NIP-49)

### 6.1 The flow

**Export encrypted…** on the Value card opens the password prompt the
KeePass export uses (`vaults.prompt`): the password, then again. The
password is printable ASCII (`0x20`–`0x7e`), at least one character; any
other character is refused with "Use letters, digits and symbols on the
keyboard". NIP-49 asks for NFKC normalisation; printable ASCII is
unchanged by it, so Faraday needs no Unicode tables.

The result replaces the nsec row's actions with:

- **ncryptsec**: the string, in full, in clear (it is sealed);
- **Show as QR** and **PNG**, over `Code::Ncryptsec`: one code (162
  characters); the PNG is
  `nostr-{fp}-{identity}-{account}-ncryptsec.png`, labelled "ncryptsec
  · identity {i} · account {a}", and is sealed as the text is (§6.5);
- **For the stick**: `nostr-{fp}-{identity}-{account}.ncryptsec.txt`,
  the string and a newline, into the Outbox as a sealed file.

Before any of it is shown, Faraday decrypts what it wrote with the
password and compares the key; a mismatch shows nothing and says
"Export failed".

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
  `osk_crypto::scrypt`, zeroized after use, as are the password and the
  secret.
- The 40 random bytes come from one fresh draw (`Faraday::fresh`,
  labelled `ncryptsec`), spread over the bytes by SHA-256 as the KeePass
  export spreads its draw.
- Bech32 with HRP `ncryptsec` over the 91 bytes. The string is longer
  than BIP-173's 90 characters; bech32 0.11's `encode::<Bech32>` takes
  up to its `CODE_LENGTH` of 1023. The implementer confirms this and
  stops if it does not.

### 6.3 The key-security byte

NIP-49's values: `0x00` the key is known to have been handled
insecurely, `0x01` it is not known to have been, `0x02` the client does
not track this. Faraday writes `0x02`. The same nsec comes from the seed
and two numbers in any BIP-85 implementation, in any session, and
Faraday cannot know what was done with it there, so `0x01` is a claim it
cannot make; `0x00` would be untrue of most keys.

### 6.4 Dependencies

| Need | Where it comes from | Cost |
|---|---|---|
| scrypt | `osk_crypto::scrypt`, written in the tree for LND's aezeed (`docs/PLANNING.md` §16.116) | None. No `scrypt` crate is in `Cargo.lock`. |
| XChaCha20-Poly1305 | `chacha20poly1305` 0.11, already used by `osk-crypto`, `osk-backup`, `faraday-vault` and others (`docs/deps/chacha20poly1305.md`) | None new: `faraday-core` adds it as a workspace dependency. |
| Bech32 | `bitcoin`'s re-export of `bech32` 0.11 | None. |
| NFKC | not needed (printable ASCII only) | `unicode-normalization` is in the graph only as `osk-bip`'s dev-dependency; not added. |

Memory: scrypt's 64 MiB is freed and zeroized before the result shows,
well under a vault unlock's; time is about 0.1 s on a desktop and about
a second or two on a Pi 3.

### 6.5 Reading the file back

A file whose text is exactly one `ncryptsec1…` string with a valid
checksum is sealed (`secrets::Exposure::Sealed`), as a vault or an
encrypted backup is: it goes out as it is and is kept across a lock.
Faraday does not import an ncryptsec.

## 7. Faraday Desktop

The same screen and the same vault page. On mainnet Faraday Desktop is
the demo (F20), whose public seeds make Nostr keys anyone can derive.
There, as F20 does for addresses:

- the npub and the nsec are shown, coded and saved with their checksum
  broken in one character (`Session::shown`'s rule), so no Nostr app
  takes them;
- **Export encrypted** is refused, on the Value card and on a vault
  entry, with "Not in the demo".

Test networks show real keys, behind F20's warning.

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
| `bip85_key_fp` | Key {} |
| `bip85_a_nostr_key` | A Nostr secret key |
| `bip85_whoever_has_nostr` | Whoever has it can sign as this Nostr identity and read its direct messages |
| `bip85_export_encrypted` | Export encrypted… |
| `bip85_ncryptsec` | ncryptsec |
| `bip85_for_the_stick` | For the stick |
| `bip85_not_in_the_demo` | Not in the demo |
| `bip85_npub_png` | npub · identity {} · account {} |
| `bip85_ncryptsec_png` | ncryptsec · identity {} · account {} |
| `bip85_printable_only` | Use letters, digits and symbols on the keyboard |
| `bip85_export_failed` | Export failed |
| `catalog_nostr_key` | Nostr key |

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
  addresses or to another identity's npub; only the seed does.
- **Identities and accounts**: an identity per separate public life,
  accounts within one; 0 of each is reserved by the BIP.
- **Encrypted export**: what an ncryptsec is, that the password is
  needed to use it, and that Nostr apps that import NIP-49 read it.
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

In `faraday/faraday-core/tests/nostr.rs`:

- every BIP vector through `faraday_core::nostr::derive` from the BIP's
  master key gives the BIP's nsec and `npub.py`'s npub;
- NIP-19's examples encode as published;
- identity 0 or account 0 is refused, by the function and on the card
  (Continue does nothing, the error line shows);
- the Tools tile opens the BIP-85 screen with Nostr chosen, the
  Identity and account card open at 1 and 1; with no key it reads "Load
  a key";
- Find a pair lists identities 1 to 5 at account 1 and accounts at the
  typed identity, and a tap fills the pair and opens the Value card;
- the Value card shows the npub with the nsec masked until Show;
- Save into the vault, lock, unlock: the vault's entry shows the same
  npub and the path, and its password is the nsec;
- NIP-49's vector decrypts to its published key; an export decrypts with
  its password to the same nsec and fails with another; its LOG_N is 16
  and its key-security byte 0x02; a password with a character outside
  printable ASCII is refused;
- Out unprotected goes through the secret sheet, nothing to the Outbox
  before it;
- the npub's and the ncryptsec's Show as QR decode (with `zbarimg` or
  `rqrr`) to the npub and the ncryptsec, and their PNGs are written For
  the stick under §4.4's and §6.1's names;
- no screen of the flow and no vault page offers a code or a PNG whose
  content is the nsec;
- a saved Nostr entry's page offers Show as QR for its npub and Export
  encrypted, whose ncryptsec decrypts to the entry's nsec; an Entry
  whose username is not the nsec's npub offers neither;
- in the demo, the npub fails its Bech32 checksum and Export encrypted
  is refused.

## 11. Batch

Not scheduled. The owner says when it is built; nothing here puts it
before any other work.

- **N1, the Nostr key.** §2's `nostr.rs`; §3's application and tile;
  §4's cards, Find a pair and the Value card; §5's vault entry and
  §5.1's page actions; §7's demo rules; §6's
  encrypted export and the file's exposure; §8's strings; §9's Learn
  page; §10's vectors and tests. Renders: the tile, the Identity and
  account card, Find a pair, the Value card and a saved entry's page at
  `1280x800` and `480x640`.

## 12. Decided by the owner (2026-10-14)

1. BIP-85 128002' only; no NIP-06.
2. A BIP-85 application and a Tools tile.
3. Identity and account start at 1/1, filled in.
4. NIP-49 export is part of the batch.
5. Find a pair; its rows stay text.
6. Specified now, built when the owner says.
7. The demo shows the npub and nsec with a broken checksum, as F20
   shows addresses, and refuses Export encrypted (§7).
8. No QR or picture of the plain nsec anywhere (§4.4, §5.1); the
   ncryptsec's QR moves the key to a phone under a password.
9. QR codes wherever they help: Show as QR and PNG for the npub and the
   ncryptsec, in the tool and on a saved entry's page (§4.4, §5.1, §6.1).
10. Faraday Companion refuses `nsec1` and `ncryptsec1`, as it refuses
    other private keys and sealed files, in a later Companion batch,
    not N1.
