# Modes, Practice and samples

**Kind:** Spec
**Status:** Built here (F81), except the website
**Open:** §9, the website, in its own repository: moving its pin to the current Faraday and the changes below are not built.

The owner's decisions are F81. This says how they are built. Where it
and `docs/DEMO.md` differ, this holds until DEMO is brought in line.

## 1. Words

- **Mode**: what the pill names. One of Practice, Mainnet, Testnet, Signet,
  Regtest. Practice is not a network: it is the key store's public set on
  Testnet.
- **Sample**: a public dummy: a key (in one of the forms of §5.1), a
  wallet with its keys (§5.2), or another input a flow takes (§5.3).
  "Public seed" and "the demo" leave the product's words; the mode's name
  in the product, Learn and these documents is Practice (it was Demo until
  2026-10-26, F119); the code keeps `Demo`.
- **Fresh session**: what a lock leaves (`Faraday::lock`, then the
  shell's restart): no keys, no wallets, no screen state.

## 2. The model (decided)

1. `faraday-core` gains a mode module (modes/mod.rs, its sheet in
   modes/screen.rs; `lib.rs`'s code map names both):

   ```rust
   pub enum Mode { Demo, Mainnet, Testnet, Signet, Regtest }
   impl Mode { pub fn network(self) -> Network /* Demo → Testnet */ }
   ```

   The variant `Mode::Demo` is the mode a person reads as Practice (F119);
   the code identifiers keep Demo.

   `Faraday::mode()` reads the state: Practice while `session.keys.demo()`,
   otherwise the session's network. Nothing stores the mode beside the
   session, so it cannot drift from it.
2. `Faraday::modes: &'static [Mode]`, set by the shell: `STICK_MODES`
   (Mainnet, Testnet, Signet, Regtest; `Faraday::new()`'s default) and
   `DESKTOP_MODES` (Practice, Testnet, Signet, Regtest). The desktop and the
   website set `DESKTOP_MODES`.
3. **Practice in faraday-keys.** `Keys::demo()` becomes "the shell set the
   demo up", with no network condition; `has_demo()` goes (one test,
   one meaning). `Keys::set_demo` takes the public set as values, not
   words: `set_demo(&mut self, handouts: &[(&'static str, u8, Fingerprint)],
   public: &[Fingerprint], nodes: &[[u8; 33]])` (a handout's name, its
   word count, its fingerprint; every sample key's fingerprint; the
   public Lightning node ids; a key that takes a passphrase is in
   `public` twice, with none and with `faraday`, §6.2). It is a lookup now, no probe: the
   fingerprints are literals checked by a test (§5.4). `faraday-keys`
   stays the only code that decides a key is public (`Keys::allowed`,
   `Keys::is_public`, and the new `Keys::is_public_node`). Built:
   a handout is a `faraday_keys::Handout` (name, word count and two
   fingerprints, with none and with `faraday`, so a handout loaded with
   either counts as loaded, §7), and core's `Session::set_demo` takes a
   `faraday_core::samples::Samples` and passes its handouts, public set
   and node ids.
4. **Practice is on Testnet.** A shell that wants Practice makes
   `Session::on(Network::Testnet)` and calls `Session::set_demo`. Every
   key derives at coin type 1 and every address is a real testnet
   address: `Session::shown` and `broken_checksum` go, with the tests
   that encode them.
5. `Faraday::makes_secrets()` is `!session.keys.demo()`: unchanged in
   code, and now it means Practice. `learn.rs`'s two conditions become one
   (`keys.demo()`).
6. **A mode change is a lock.** `Action::Mode(Mode)`:
   - the mode in use, or one not in `self.modes`, or any while the
     session's network is fixed: closes the sheet, nothing else;
   - otherwise `self.next_mode = Some(m)` and `self.lock()`: the same
     LockAsk, the same refusal when the state cannot be kept
     (`not_kept`, which also clears `next_mode`), the same sheet the Lock
     action shows when a stick is attached.
   `Faraday::mode_after_restart()` is `next_mode.unwrap_or(self.mode())`:
   a lock keeps the mode the person was in; a mode change takes the one
   chosen. The shell reads it before it drops the app.
   A mode change empties more than a lock does (owner, 2026-10-22): the
   Inbox (From the stick) and the Outbox (For the stick) are emptied and
   the shell is told to keep none of them (`SaveBoxes` with nothing), on
   the desktop and on the stick and the Pi alike; nothing the lock keeps
   in memory is carried but the mode. A plain lock keeps both boxes as
   before. Files on a stick are not touched. The Lock sheet of a mode
   change says it keeps nothing but the mode (`lock_mode_keeps_nothing`)
   and does not list the files For the stick as kept.
7. **What goes.** `Session::set_network` and `Keys::set_network` leave
   every path a person takes; `Keys::set_network` stays only for
   `Session::follow` (point 8) and a shell's start. Its mainnet refusal
   (`Refusal::NotPublicLoaded`) and the Network row's "Wallets loaded"
   note (`network_row_items`, `network_row_note`) go.
8. **`Session::follow`** (a wallet or a PSBT on another network) moves
   the session only when all hold: no key and no wallet loaded, not
   Practice, the network not fixed, and the mode it moves to in the
   caller's `modes`; it is then the same as choosing that mode on an
   empty session, so no lock is needed. Otherwise a wallet or PSBT of
   another network kind is refused with today's `Refusal::Network` text.
   `follow` takes the allowed modes as an argument (`Session` does not
   know `Faraday`).
9. The website's mode is fixed (`Keys::fix_network` on Testnet with the
   demo set): the sheet lists the other three dimmed.

## 3. The pill and the mode sheet

- **The pill** (DESIGN §4.8, Network): drawn where it is drawn today and
  only off Mainnet, now with a hit target: `Action::ModeSheet` opens
  `Sheet::Mode`. Label: the mode's name ("Testnet", "Signet",
  "Regtest"), and for Practice its name and its network, "Practice · Testnet"
  (owner, 2026-10-22: a person sees that Practice runs on testnet), on the
  wide layout; on the small panel's bar (a compact window, the Pi's 268
  dp) "Practice" alone, as every other mode's pill reads its name, so
  the bar keeps Cancel, the eye and the ? (owner, 2026-10-26;
  DESIGN §4.1); the same caution pill for each. Places:
  `header_network_pill` and `network_pill_left`
  (`faraday/faraday-core/src/screens/mod.rs`), `compact::tools` and
  compact Home (`faraday/faraday-core/src/compact/mod.rs`). The
  comments "a badge … opens nothing" go.
- **Samples**: in Practice only, when `Faraday::samples` is set (§5),
  in Faraday Demo's host strip after the sticks, outside the app
  (F118, `docs/DESKTOP-HOST.md` §3); it opens the Load samples sheet.
  The app's bar holds the mode pill alone.
- **Settings**: the Network row (`network_row_items`) is replaced by one
  row, **Mode**, the mode's name muted at its right, a chevron; it
  opens the same sheet. On Mainnet, where no pill shows, it is the only
  door.
- **The sheet** (`Sheet::Mode`, modes/screen.rs): title "Mode"; one row
  per mode in `self.modes`, in that order, divided by rules; the mode in
  use marked by a tick in the accent at the row's right; the Practice row
  carries "Testnet" muted before the tick's place; a press on another row is
  `Action::Mode(m)`; **Not now** at the foot. A row that cannot be
  chosen (the website's three) is dimmed, its label alone: no reason
  line (DESIGN §4.11, nothing on the device changes it). No line says a
  change locks: the Lock sheet or LockAsk, where the lock needs one,
  says what it keeps.
- Wide: a sheet centred, at most the Load sheet's width (F62,
  `faraday/faraday-core/src/vault/load_screen.rs`). Compact:
  `compact::scroll_sheet` with `sheet_head`, as the Load sheet.
- Escape and Not now close it; Back lands where it was opened (F77).

## 4. Starting, locking, no warning

**Faraday Demo** (`faraday/shells/desktop/src/main.rs`):

- `desktop_app_for(release)` becomes `desktop_app_in(mode, warned)`:
  `Session::on(mode.network())`, `set_demo` when the mode is Practice,
  `app.modes = DESKTOP_MODES`, `app.samples = Some(&SAMPLES)` (with the pill; `set_demo` also takes
  `SAMPLES`).
  Every build starts in Practice, the developer's, the test's and the
  release's.
- `restart` reads `app.mode_after_restart()` from the app before
  replacing it, and builds `desktop_app_in` with it: the mode carries
  across a lock.
- **No online warning** (owner, 2026-10-22). F61's `Sheet::NotAirgapped`
  goes, with `with_start_warning`, `without_airgap_warning`,
  `airgap_warned`, `test_asked`'s use for it, the `test_network_*`
  strings, the boot import's wait under it and `devices.rs`'s guard;
  `Action::Network` goes. A person can type a seed on an online computer
  in any mode, Practice included (Practice swaps it for a sample only after it
  is typed), so a sheet on one path protects nothing.
- **Instead, one line at every place a secret is typed** (owner,
  2026-10-22): where the shell sets `Faraday::seed_line` (a seed typed
  here is on an online computer: the desktop, with `Faraday::online`, and
  the website alone, which does not become Faraday Demo; never the
  stick or the Pi), every step where a
  seed or a secret can be typed or pasted draws one caution-tone line
  under its field, `never_real_seed` "Never type a seed that holds real
  bitcoin here.", in every mode, Practice included: words (BIP-39, every
  length), a seed scanner's Type and Paste, a SLIP-39 share, a codex32
  string, an LND cipher seed and its passphrase, hex entropy, an xprv or
  a WIF, a BIP-39 passphrase, a vault's passphrase where it
  can be typed (not in Practice, where it is `faraday`, §6.2). A statement
  of fact in the caution tone, as a key card's line is (DESIGN §4.11);
  one string, one placement rule (under the field, above the keyboard,
  on the wide and the compact layout). Not in README or on the website's
  download page: the app says it where it matters.

**Stick and Pi** (`faraday/shells/stick/src/main.rs`): a lock is a new
process (exit code 75, the inittab's loop). Before exiting for a lock,
the shell writes `mode_after_restart()`'s name into a file `mode` in the
run directory it keeps the Inbox and Outbox in; at start it reads it,
takes it only if it names one of `STICK_MODES` exactly, removes it, and
starts the session on that network. No file, or anything else in it:
Mainnet. Settings' file (`faraday-settings.txt`) still holds no network.

**Website**: §9.

## 5. The samples

### 5.1 Keys, every form Faraday reads

Each row is one tick in Load samples and one candidate for Use a
sample. "Public set" means its fingerprint (or node id) is in
`Keys::set_demo`'s list, so Practice takes it however it arrives.

| Sample | Form | Source | Checked against |
|---|---|---|---|
| Guide key · 3dca6805, listed first | BIP-39, 12 words | the user guide's key (`docs/guide/walk/new-key.md`): EntropyLab's dice transcript, `tools/vectors/entropy/dice.json`'s first words vector (F143); not a handout | the vector's entropy, and another implementation's fingerprints |
| bacon, flag, gas, great, slice, solution, summer, they, trade, trap, zebra ×24 | BIP-39 English, 24 words | `demo::SEEDS`; the only words valid 24 times (computed: 11 of 2048) | `Keys::add_words_with` |
| twelve, ocean, piano, silver ×12 | BIP-39, 12 words | new; 130 of the 2048 words are valid 12 times | the same |
| lucky, olive ×15; glad, school ×18; brave, sunset ×21 | BIP-39, 15, 18, 21 words | new; 57, 26 and 17 words are valid at those lengths | the same |
| abandon … about | BIP-39, 12 words, the all-zero entropy | `tools/vectors/bip39/vectors.json`, first English case | the vector's entropy and seed (under its `TREZOR`) |
| Japanese words | BIP-39 Japanese, 12 words | `tools/vectors/bip39/test_JP_BIP39.json`, first case | the vector's entropy and seed (under its own passphrase) |
| SLIP-39, one share | SLIP-39, 1 of 1, 128 bits | `tools/vectors/slip39/vectors.json` case 1 | the vector's master secret (under `TREZOR`) |
| SLIP-39, 2 of 3 | SLIP-39 set; the tick loads its two shares | case 4 | the same |
| SLIP-39, groups | SLIP-39, two levels, 256 bits | case 37 | the same |
| codex32 secret | codex32, k = 0 | `tools/vectors/bip93/vectors.json`, secrets, vector 1 | the vector's seed and xprv |
| codex32, 2 shares | codex32 set; the tick loads both shares | sets, vector 2 | the same |
| Seed XOR, 2 parts | Seed XOR set; the tick loads the combined key | `tools/vectors/xor/12-words-2-parts.txt` (constructed, not Coldcard's; its README says so) | the file's seed words |
| LND cipher seed | aezeed, for Lightning node key only; public node id | LND's published vector, `PRODUCTION_1` in `core/faraday-bip/src/aezeed.rs` (no passphrase) | the node id `aezeed.rs`'s test names |
| Core Lightning hsm_secret, plain | a 32-byte `hsm_secret` put in Files; Lightning node key only; public node id | made for Faraday (F228) | the node id `faraday-samples` gives, which `faraday-keys` derives |
| Core Lightning hsm_secret, encrypted | the same secret sealed by libsodium under `faraday` | the plain sample's node id | the same node id |
| Core Lightning hsm_secret, words form | a zero passphrase hash then the guide key's English words; the same words typed, with none and with `faraday` | the Core Lightning node of the guide key and of every other English words sample (the sample words and the abandon vector), with none and with `faraday` | those node ids |

**With a passphrase.** Every key form that takes a passphrase (BIP-39,
SLIP-39) is public twice: with none, and with `faraday` (§6.2). Both
fingerprints are in the public set; the vectors' own passphrases
(`TREZOR`, the Japanese case's) are used only by the test that checks
the sample against its vector, since Practice never takes another
passphrase. codex32 and a master seed take none.

Not separate samples, decided: **SeedQR and CompactSeedQR** are
spellings of words: Use a sample on a scanner that takes a seed hands
over a sample's Standard SeedQR digits, so the scanner's own decoding
runs; **xprv, WIF**: Faraday loads no key from them; **a vault**: §5.3;
**a BIP-85 child**: disabled in Practice (DEMO §4).

**Loading a set.** A set's tick loads exactly its threshold, the parts
the vector gives, and so the key; not one short. Practising the last
part is Add a key's job, with Use a sample filling the next part (§6).

**Paths** (all through what a typed or scanned key takes, never a new
one): words, through `Keys::add_words_with`; SLIP-39 shares and a
codex32 or Seed XOR set, through `inbox::Secret::add` (words,
`slip39_seed` then `add_seed`, `codex32_seed` then `add_seed`); the
cipher seed, through
`faraday_keys::lightning::node_from_aezeed`.

**Constraints already in place.** New key's handout (§7) and
`makes_secrets` (DEMO §4) are unchanged in what they refuse: SLIP-39,
Seed XOR and codex32 *splits* of a key held stay disabled (fresh
randomness), and so does Show for a node's private key. Loading a
sample set is not a split: its parts are published. DEMO §6's LND
cipher seed row opens in Practice, and `node_from_aezeed` refuses a cipher
seed whose node id is not in the public set (`Keys::is_public_node`).

### 5.2 Wallets

The testkit's wallets, testnet, coin type 1, over the test seeds bacon,
zebra and summer: Savings (2 of 3), Spending, Taproot, Nested SegWit,
Legacy, Nested multisig, Legacy multisig, Taproot multisig, Inheritance,
Taproot tree (`testkit::kits`). Also, decided:

- **MuSig2** (`testkit::musig_policy`): its keys are the three test
  seeds; MuSig2 is open in Practice.
- **3 of 5** (`testkit::three_of_five`): its five keys are master seeds
  01…01 to 05…05, keys with no words; they load as keys and join the
  public set. It has no spend.

A wallet's tick loads the wallet and every seed of it that is a sample
(`Session::add_wallet`, then the keys). Each wallet sample carries its
unsigned spend where the testkit makes one (`testkit::unsigned`,
`musig_unsigned`), for §6.

### 5.3 Other inputs

- **Passphrases and passwords**: `faraday`, everywhere (§6.2).
  `demo::PASSPHRASE` becomes `"faraday"` and stays in `faraday-core`;
  `PASSPHRASE_WORDS` goes.
- **A vault file**: a vault made like the testkit's
  (`testkit::test_vault`: test key 1 and the Savings wallet) under the
  one passphrase `faraday`, as a literal file.
- **A signed message**: one made by sample Spending's first receive
  address, as a literal; **an address**: Spending's receive address 0.
- **A TOTP setup code**: the testkit's `ENTRIES_FILE` first line (RFC
  6238's published example secret).
- **Check a signer's pair**: three PSBTs made from sample Spending and
  sample key 1: its unsigned spend as Sent, that spend signed by the key
  as a clean Returned, and the clean one with one proprietary pair
  (prefix `device`, subtype 1) as a second Returned. Both of Check a
  signer's scanners offer them, as Load sample PSBT: the Sent scanner
  hands over the Sent; the Returned scanner hands over the clean one, and
  the proprietary one while the Returned card holds the clean one, so two
  presses show both results.
- **Check a timestamp**: the OpenTimestamps proof of the US Commerce
  Department's gdp2q25-2nd.pdf (`gdp2q25-2nd.pdf.ots` of the reference
  client's examples, `tools/vectors/opentimestamps/`), which reaches its
  Bitcoin block through SHA-256 only; the SHA-256 of that file, in hex,
  which the tile's hash field takes in place of the file; and the real
  80-byte header of block 912095, the block the proof names, in hex. The
  tile opens with the three taken and the proof counted. The block header's
  scanner offers the header as Load sample block header. A test checks that
  the header's double SHA-256 is the block's hash and that the proof's path
  from the file's hash ends at the header's merkle root.

### 5.4 Where they live

- A new crate, faraday/faraday-samples, of public data only: `&str`,
  `&[u8]` (the vault, `include_bytes!` of a file in the
  crate) and fingerprints as `[u8; 4]`, in the types
  `faraday_core::samples` defines (`SampleKey`, `SampleWallet`,
  `Samples`, plain structs of `&'static` fields). Its one item:
  `pub static SAMPLES: Samples`. It parses nothing and derives nothing,
  so lint-secrets rule 4 has nothing to refuse there.
- Names on screen are not in the crate: each sample carries a key into
  `faraday/faraday-strings/src/en.rs` (§8).
- `Faraday::samples: Option<&'static Samples>`, set by Faraday Demo (in
  its own window and inside the Starter, F126) and the website. With
  `None` (the stick, the Pi, the Faraday the Starter holds for Transfer
  and Learn)
  no pill, no sheet and no Use a sample is drawn or acted on.
- **Gate**: the crate, not a feature. `tools/lint-keys-tests.sh` gains
  a third check: `cargo tree -i faraday-samples` over faraday-stick,
  faraday-disk, faraday-grant, faraday-boot and the Starter's own crate
  `faraday-starter` (normal edges, every target) must find nothing. The
  desktop depends on it normally, and the Starter's binary
  (`faraday-starter-shell`) through `faraday-desktop` alone, since
  Faraday Demo runs inside it (F126); faraday-core never does.
- **The test** (faraday-samples' tests/, faraday-core's `testkit` as a
  dev-dependency): every word sample's fingerprint equals what
  `Keys::add_words_with` gives; every wallet's descriptor equals the
  testkit's; every unsigned spend equals the testkit's bytes; every
  vector sample equals its vector file read at test time (words,
  shares, strings, seed, xprv where the vector has one); the vault
  opens under `faraday` to what the table says;
  the cipher seed's node id is the one `aezeed.rs`'s test names; and
  the handout list holds every length New key offers.
- `demo::SEEDS` and `demo::words` move into the crate; `demo.rs` keeps
  `PASSPHRASE`, `makes_secrets` and `demo_refuses`.
- The testkit stays a test-only feature (F70, `docs/KEYS.md`); the
  desktop's dev-dependency on it is unchanged.

## 6. Use a sample

In Practice, with `samples` set, each step below hands **Use a sample** to
the page's "Demo only" card while its field is empty, and the scanner
its sample (owner, 2026-10-24; `docs/DEMO.md` §14). A
press puts the sample into the step through the function the step's
own input already calls, as if typed or scanned, and the flow goes on
as it would. Which sample, decided: **the next one not used**, by a
cursor on the step's state that a press moves (no picker); a press
again replaces it with the next, wrapping. "Not used" means not
loaded, and for a set, not yet begun.

**Placement.** In Faraday Demo's host strip (DESIGN §4.19,
`docs/DEMO.md` §14), never inside the field or the scanner's band: the field keeps its
caret and placeholder as in every mode. Never drawn on Mainnet,
Testnet, Signet or Regtest; drawn on the website. The scanner's button
says what it hands over (Load sample PSBT, wallet, seed, share, key,
SeedQR, code, address). Every scanner that takes a file offers it; a
step that asks for the transaction (the Spend tab's What do you have?,
its first step's shortcut, Write the payment) also hands the card Load
sample PSBT, which hands over the same spend and takes the same path as
a scan of it. The same
transaction read again is the file already in Files, not a second copy.

**The same value again.** Where a flow asks again for a value a sample
filled, the button fills the matching one, read from the state that
already holds it: the next share of the set begun (`forms::Parts`), the
SeedQR of the key a copy check names, a passphrase (always `faraday`,
§6.2). Nothing new is kept for it. A value the person typed is theirs
to remember, as for real.

### 6.1 The field's inner action (faraday-ui)

Since 2026-10-24 (owner) the button stands outside the field, and
since 2026-10-25 in Faraday Demo's host strip (`docs/DEMO.md` §14); it shows while the
field is empty and the person has not typed in it, and a press in the
field no longer hides it. The rest of this section, the table of steps
and what a press puts in, stands.

- `Ui::field_action(x, y, w, h, label, action)`, drawn by every entry
  field of the table below (DESIGN §4.3), inside the field's box: a
  secondary-styled chip, left-aligned at the field's text inset,
  vertically centred, labelled "Use a sample"; its hit target is the
  chip, at least the touch floor's height on compact and the field's
  height on wide. The field's caret and placeholder are not drawn while
  the chip shows.
- **Shown** while the field is empty and the person has not pressed in
  it or typed this visit. A press elsewhere in the field focuses it and
  hides the chip for that visit, so a person can type and watch the
  field behave; emptying the field again does not bring it back until
  the step is left and opened again. A chip press fills the field and
  hides the chip.
- **A secret filled by a sample** (a seed's words, a share) is drawn
  unmasked in Practice: samples are public. Typing into the field masks it
  again as the field does today.
- **Wide**: the chip in the field. **Compact** and the small panel: the
  same chip; where the field is narrower than the chip, its label is
  "Sample" (`scan_sample`).
- The state ("chip shown", "filled by a sample") lives on the step's
  state beside its field, not in the drawing memo; drawing reads it.

| Step | Where (file, function) | What a press puts in |
|---|---|---|
| Add a key, BIP-39 words | `faraday/faraday-core/src/screens/entry.rs` `entry_form`, `entry_words`; `forms.rs` `Form::Words` | the next word sample of the word count chosen (`word_count_row`), as typed words |
| Add a key, SLIP-39 / codex32 / Seed XOR | the same; `forms.rs` `Parts` | the next part of the set begun; with none begun, part 1 of the first set not loaded |
| The scanner: a seed, a part, a wallet's missing key, the Spend tab's envelope | `faraday/faraday-core/src/screens/sheets.rs` `scan_sheet`; `qr_view.rs` `ScanPurpose::Seed`, `KeyPart`, `WalletKey`, `Intake`; handed to `Faraday::scanned` | Seed: a word sample's SeedQR digits; KeyPart: as Add a key; WalletKey(n): the sample whose fingerprint is a missing key of wallet n (no tile when none is); Intake: the unsigned spend of the first loaded sample wallet, else Savings' |
| The scanner: a file (a wallet, a PSBT) | `ScanPurpose::Files` from any screen | Restore: the next sample wallet not loaded, as its descriptor; Spend and Decode: the unsigned spend of the wallet in hand, else of the first loaded sample wallet |
| The scanner: a copy to check | `ScanPurpose::CheckCopy(fp)` | the SeedQR of key `fp` when it is a sample; no tile otherwise |
| The scanner: Check a signer's Sent and Returned | `ScanPurpose::CheckSent`, `CheckReturned` | §5.3's pair: the Sent; the clean Returned, then the one with a proprietary pair while the card holds the clean one |
| The scanner: a TOTP code | `ScanPurpose::VaultEntry` | §5.3's setup code |
| The scanner: Check a timestamp's block header | `ScanPurpose::OtsHeader` | §5.3's header, as hex |
| Tools: Check a timestamp, in Practice with no proof in Files | `faraday/faraday-core/src/ots_check/mod.rs` `open_sample` | §5.3's proof, file hash and header, taken |
| Scan a key (a wallet's missing key) | `faraday/faraday-core/src/scan_key.rs` | as WalletKey above |
| Restore, seeds first: a cosigner's account key | `faraday/faraday-core/src/seeds/mod.rs` `Focus::Cosigner` | the account key, at the path chosen, of the next sample seed not among the wallet's |
| The Spend tab's words | `faraday/faraday-core/src/spend_backup/mod.rs` | as Add a key |
| New key, a typed key | `faraday/faraday-core/src/keygen/mod.rs` `kstep::TYPED` | as Add a key |
| The Vaults screen with no vault | `Screen::Vaults` | the sample vault, into the Inbox as an item of the session, not written to disk |
| Lightning node key, An LND cipher seed | `faraday/faraday-core/src/lightning/mod.rs`, `lightning/screen.rs` | the sample cipher seed's 24 words |
| Check a signed message; Check an address | `faraday/faraday-core/src/screens/check_message.rs` | §5.3's message; §5.3's address |

Not offered, decided: New key's sources (dice, coins, cards, hex,
camera): the person makes the entropy and §7 replaces the result;
Sign a message's text, a wallet's name, a vault entry's title and
notes, Set the time, the calculators: free input, not a sample; Silent
payments' Check a payment: no sample transaction pays a silent
address; every passphrase and password: §6.2.

**Cost.** Whether the chip shows and which sample is next are lookups:
fingerprint literals against keys loaded (`Keys::position`) and
against what the state keeps (a wallet's missing keys are kept on it).
Nothing is derived while drawing. A press derives through the step's
own path, with the handler's `Compute`.

### 6.2 Every passphrase field offers `faraday`

In Practice every passphrase and password field works as it does outside
Practice: empty, typed by the person, masked, with the eye and with the
caution line "Never type a seed that holds real bitcoin here." under it
where the shell is online. In addition an empty field hands the "Demo
only" card **Use faraday** (owner, 2026-10-24; `docs/DEMO.md` §14); a
passphrase and its repeat hand one, which fills both where they are
empty. Nothing is drawn inside the field, which keeps its caret and,
where it is optional, its **No passphrase** pill. It is gone once the
field holds something. A press on it puts `faraday` (all lowercase; owner, 2026-10-22)
in the field, drawn unmasked in the mono face inside a code box (a
token-padded, tinted, rounded box, as inline code is drawn), so its
exact characters are plain; the eye is not drawn on it. A press in that
field lets the person clear or replace it; what the person types is
masked and is theirs. A second field that types a passphrase back, and
an unlock, offer the same chip, so `faraday` is always one press away,
and "The two passphrases differ" shows only when they do. Optional
fields stay optional (**No passphrase** as before). The Load samples
sheet and the Learn page each say once what the chip is: "In Practice, the
sample passphrase is" followed by `faraday` in the same code box. No
field explains it.

The KeePass export's passphrase has no chip: `faraday` is under its
minimum of 8 characters, which is unchanged, so it is typed by the
person, masked, with the eye and the caution line.

`Faraday::demo_passphrase()` (true in Practice) is the one test;
`Faraday::pass_chip` and `pass_chip_draw` draw the chip and
`Action::PassChip(PassField)` fills the field. Nothing is prefilled and
no typing is swallowed.

| Field (`demo::PassField`) | Where |
|---|---|
| `KeyGen(0 or 1)`: New key's BIP-39 passphrase and its repeat | `faraday/faraday-core/src/keygen/mod.rs`, `keygen/screen.rs` (`KPassField`) |
| `Entry`: Add a key's passphrase | `faraday/faraday-core/src/screens/entry.rs`, `compact/screen.rs` |
| `SeedRead(0 or 1)`: a seed read's passphrase and repeat | `faraday/faraday-core/src/seeds/read.rs` (`Sheet::SeedRead`) |
| `Potential`: the potential wallet's | `Sheet::Potential` (`screens/sheets.rs`) |
| `Slot(0 or 1)`: Make a new wallet's passphrase slot | `faraday/faraday-core/src/create.rs` (`pass_slot`, `pass_focus`) |
| `Phrase(i, again)`: Create a vault's passphrases (every slot) and their repeats | `faraday/faraday-core/src/vault/mod.rs` (`CreateForm::phrases`), `vault/screen.rs` |
| `Unlock`: unlock, the boot import's passphrase | `vault/mod.rs` `Focus::Passphrase` (`Sheet::Import`) |
| `Prompt`: a passphrase prompt | `vault/mod.rs` `Focus::Prompt` (`Sheet::PassphraseSeeds`); not the KeePass export's |
| `Unlock`: the Spend tab's vault passphrase | `faraday/faraday-core/src/spend_backup/mod.rs` |
| `Check`: Check my copy's passphrase | `faraday/faraday-core/src/screens/backup.rs` (`check.passphrase`) |
| `Password(k)`: a vault entry's password field | Vault contents' entry form (`vault/screen.rs`) |
| `Aezeed`: an aezeed's passphrase | `faraday/faraday-core/src/lightning/mod.rs` |
| A KeePass export's password: no chip | the vault's KeePass export (`vault/out.rs`) |

**Generate with dice** (DEMO §5.2) still takes the rolls and shows the
words; when the last word is in, both fields take `faraday` in place
of them, and the card's one line says so (`vault_demo_uses_public`).
"or type your own below" stays beside it.

**Refusals of a weak passphrase.** There are none to loosen: Faraday
refuses an empty passphrase and a repeat that differs
(`Vaults::phrase_ok`, `pass_pair_differ`), and measures
strength only for words the dice made (`Vaults::phrase_bits`; a typed
passphrase's is not measured). The dice count (how many rolls make a
word, how many words) is unchanged. In Practice `phrase_bits` gives `None`
for `faraday` (the dice's words are dropped), so no strength line is
drawn for it and the "weakest passphrase" line is left out while every
passphrase is `faraday`; `PASSPHRASE_WORDS` goes. Outside
Practice nothing changes: a test types a short passphrase on Testnet and on
the stick and sees it taken as typed, masked and editable, with the
dice's strength line where the dice made it.

## 7. New key in Practice

Every source still runs and shows its words. At Lock in, the key made
is replaced by the next handout **of the word count chosen**:
`Keys::next_handout(words: u8)`, the first handout of that length not
loaded (§5.1: eleven of 24, four of 12, two each of 15, 18, 21). The
Key card's line, a statement of fact in the warning tone as now
(`keygen/screen.rs`): "Lock in adds sample ocean ×12 (1a2b3c4d) in place
of these words"; after Lock in, the toast "Added sample ocean ×12
(1a2b3c4d) in place of the key made". With every handout of that length
loaded, Lock in is dimmed (`demo_refuses`, `Action::KLock`). Ticking in
Load samples skips nothing: the handout takes the first not loaded,
whatever loaded it; a handout counts as loaded with or without
`faraday`. A passphrase chosen on the Key card is `faraday` (§6.2),
and the handout is added with it. A typed key (`KeyGen::typed`) is not
replaced.

Words made that are a sample's own, word for word (the guide's dice
rolls make Guide key · 3dca6805), are kept as made (F143): the Key
card's line reads "These words are sample Guide key · 3dca6805", Lock in
is not dimmed for them, and it adds them as made, with the passphrase
chosen; after Lock in the same line stands under the fingerprint.
`Faraday::made_sample_now` compares the words made with the samples'
words (a lookup); the store still takes only the public set's
fingerprints, so nothing else is let through.

## 8. Strings

All in `faraday/faraday-strings/src/en.rs`, each with its doc line as
the file has them.

| Name | Text |
|---|---|
| `mode_sheet_title` | Mode |
| `mode_demo` | Practice |
| `mode_demo_pill` | Practice · Testnet |
| `never_real_seed` | Never type a seed that holds real bitcoin here. |
| `mode_not_now` | Not now |
| `settings_mode` | Mode |
| `samples_pill_short` | Samples |
| `samples_title` | Load samples |
| `samples_wallets` | Wallets |
| `samples_keys` | Keys |
| `samples_select_all` / `samples_unselect_all` | Select all / Unselect all |
| `samples_not_now` | Not now |
| `samples_load` | Load ticked |
| `samples_password` | In Practice, the sample passphrase is (the passphrase follows in its code box) |
| `sample_type` | {} · {} (kind and script, or form and size) |
| `sample_kind_single` / `sample_kind_multisig` / `sample_kind_musig` / `sample_kind_timelock` / `sample_script_tree` | single-sig / {}-of-{} multisig / {}-key MuSig2 / timelock recovery / Taproot, script tree (the other scripts are `wallet_legacy`, `wallet_nested_segwit`, `wallet_native_segwit`, `wallet_taproot`) |
| `sample_form_bip39` / `_slip39` / `_codex32` / `_xor` / `_seed` / `_aezeed` | BIP-39 / SLIP-39 / codex32 / Seed XOR / Master seed / LND cipher seed |
| `sample_size_words` / `_words_japanese` / `_share` / `_shares` / `_threshold` / `_groups` / `_secret` / `_parts` / `_bytes` | {} words / {} words, Japanese / 1 share / {} shares / {} of {} shares / groups, {} shares / secret / {} parts / {} bytes |
| `samples_loaded` | Loaded {} (`{}`: how many, as the Load sheet's toast counts) |
| `use_a_sample` | Use a sample |
| `scan_sample` | Sample |
| `sample_words` | {} ×{} (a word, a count) |
| `sample_guide` | Guide key · 3dca6805 |
| `sample_abandon` | abandon … about |
| `sample_japanese` | Japanese words |
| `sample_slip39_one` / `sample_slip39_two_of_three` / `sample_slip39_groups` | SLIP-39, one share / SLIP-39, 2 of 3 / SLIP-39, groups |
| `sample_codex32_secret` / `sample_codex32_shares` | codex32 secret / codex32, 2 shares |
| `sample_xor` | Seed XOR, 2 parts |
| `sample_aezeed` | LND cipher seed |
| `sample_hsm_plain` / `sample_hsm_encrypted` / `sample_hsm_words` | hsm_secret / hsm_secret-encrypted / hsm_secret-words (files put in Files) |
| `sample_savings` … `sample_tree` | Savings, Spending, Taproot, Nested SegWit, Legacy, Nested multisig, Legacy multisig, Taproot multisig, Inheritance, Taproot tree |
| `sample_musig` / `sample_three_of_five` | MuSig2 / 3 of 5 |
| `keygen_lock_in_adds` (changed) | Lock in adds sample {} ({}) in place of these words |
| `keygen_added_public_seed` → `keygen_added_sample` | Added sample {} ({}) in place of the key made |
| `keygen_words_are_sample` | These words are sample {} (F143) |
| `wallet_is_not_a_key` (changed) | {} is not a sample: Practice takes only samples |
| `vault_demo_uses_public` (changed) | Practice puts faraday in place of these words |
| `wallet_public_seeds_only`, `network_row_note_wallets` | removed |

## 9. The website, in its repository

The website is pinned to an older Faraday; it
moves the pin to the current commit by the procedure its HANDOFF.md
gives, and makes the changes below there. Faraday's repository holds
nothing of it.

- `web/Cargo.toml`: faraday-core without `testkit`; faraday-samples
  added.
- `web/src/lib.rs` (`start`): `Session::on(Network::Testnet)`,
  `set_demo` with the samples, `fix_network`, `app.modes =
  DESKTOP_MODES`, `app.samples = Some(&SAMPLES)`. The pills show "Practice ·
  Testnet" and Load samples; the pill and Settings' Mode row open the
  mode sheet, where Testnet, Signet and Regtest are dimmed.
- **The stories use the samples** (`web/src/desk.rs`, `story.rs`,
  `stories.rs`, `walk.rs`):
  - **Bring in a wallet**: the wallet is sample Savings. Seed papers A,
    B and C are SeedQRs of Savings' three seeds (bacon, zebra, summer,
    the words they are today); the Restore they make gives Savings'
    descriptor, testnet, BIP-48 account 0 at coin type 1 (today's
    is coin type 0). The phone shows Savings' sample unsigned spend:
    0.1 tBTC in, 0.015 tBTC to the testkit's recipient (tb1), the rest
    less 2,140 sat to change.
  - **Spend using a Faraday backup**: the boot stick's vault holds Seed
    A and Savings' sample descriptor, with the plan and places as today
    (`backup_answers`), under the one passphrase `faraday` (Practice's;
    the second, easy slot goes, since it was the same word); Seed B's
    paper and the phone as above. The sticky note says `faraday`.
  - **Start fresh** loads no sample: the story is making a wallet, and
    its key comes from New key's handout (a 12 or 24-word sample by the
    length chosen).
  - `desk.rs`'s own `descriptor`, `account`, `spend`, `fingerprint`,
    `PATH` and `NET` go; it reads the samples. Hints and walk steps
    that name an amount say tBTC where Faraday shows tBTC; those that
    name the vault's passphrase or the easy one say `faraday`.
- Tests: each story walked to its end (`stories.rs`), on testnet; the
  papers' SeedQRs and the phone's PSBT equal the samples'; nothing in
  the website depends on faraday-core's `testkit` (a check in its
  `just`).
- HANDOFF.md and the page's own words follow: "mainnet" goes from the
  demo's description.

**Where the website and the desktop still differ in Practice**, by design:

| | Faraday Demo | Website | Why |
|---|---|---|---|
| Switching mode | Practice, Testnet, Signet, Regtest | Practice only | a public page takes samples only; a person's own test seeds belong in the downloaded app |
| Files | the Inbox and Outbox are folders on disk (Downloads) | sticks in memory, drawn round the laptop | a browser has no folder Faraday may watch |
| Paste | the clipboard, a seed on a seed scanner and a key's words on the words form, by Ctrl+V or the strip's Paste words, in every mode Faraday Demo has (F75) | none | the web shell has no clipboard |
| Camera | the computer's cameras | one webcam that sees the desk's props | the props are the page's way in |

## 10. Batches

Moved to `docs/history/MODES-LOG.md`.

## 11. Settled with the owner (2026-10-22)

1. A lock comes back in the mode the person was in, a test network as
   Practice; leaving a mode is always a choice made on the pill.
2. Practice's addresses are real testnet addresses: the broken checksum
   goes (§2 item 4).
