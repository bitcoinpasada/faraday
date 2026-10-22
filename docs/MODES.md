# Modes, Demo and samples

**Kind:** Spec
**Status:** Partly built (F81, tracker #191): M1 and M2
**Open:** M3 to M8 of §10 here, and W1 in the website's repository.

The owner's decisions are F81. This says how they are built. Where it
and `docs/DEMO.md` differ, this holds until M8 brings DEMO in line.

## 1. Words

- **Mode**: what the pill names. One of Demo, Mainnet, Testnet, Signet,
  Regtest. Demo is not a network: it is the key store's public set on
  Testnet.
- **Sample**: a public dummy: a key (in one of the forms of §5.1), a
  wallet with its keys (§5.2), or another input a flow takes (§5.3).
  "Public seed" and "the demo" leave the product's words; the Learn page
  and DEMO keep "Demo" as the mode's name.
- **Fresh session**: what a lock leaves (`Faraday::lock`, then the
  shell's restart): no keys, no wallets, no screen state.

## 2. The model (decided)

1. `faraday-core` gains a mode module (modes/mod.rs, its sheet in
   modes/screen.rs; `lib.rs`'s code map names both):

   ```rust
   pub enum Mode { Demo, Mainnet, Testnet, Signet, Regtest }
   impl Mode { pub fn network(self) -> Network /* Demo → Testnet */ }
   ```

   `Faraday::mode()` reads the state: Demo while `session.keys.demo()`,
   otherwise the session's network. Nothing stores the mode beside the
   session, so it cannot drift from it.
2. `Faraday::modes: &'static [Mode]`, set by the shell: `STICK_MODES`
   (Mainnet, Testnet, Signet, Regtest; `Faraday::new()`'s default) and
   `DESKTOP_MODES` (Demo, Testnet, Signet, Regtest). The desktop and the
   website set `DESKTOP_MODES`.
3. **Demo in faraday-keys.** `Keys::demo()` becomes "the shell set the
   demo up", with no network condition; `has_demo()` goes (one test,
   one meaning). `Keys::set_demo` takes the public set as values, not
   words: `set_demo(&mut self, handouts: &[(&'static str, u8, Fingerprint)],
   public: &[Fingerprint], nodes: &[[u8; 33]])` (a handout's name, its
   word count, its fingerprint; every sample key's fingerprint; the
   public Lightning node ids; a key that takes a passphrase is in
   `public` twice, with none and with `password`, §6.2). It is a lookup now, no probe: the
   fingerprints are literals checked by a test (§5.4). `faraday-keys`
   stays the only code that decides a key is public (`Keys::allowed`,
   `Keys::is_public`, and the new `Keys::is_public_node`). Until M5,
   core's `Session::set_demo` still probes `demo::SEEDS` and passes the
   result.
4. **Demo is on Testnet.** A shell that wants Demo makes
   `Session::on(Network::Testnet)` and calls `Session::set_demo`. Every
   key derives at coin type 1 and every address is a real testnet
   address: `Session::shown` and `broken_checksum` go, with the tests
   that encode them.
5. `Faraday::makes_secrets()` is `!session.keys.demo()`: unchanged in
   code, and now it means Demo. `learn.rs`'s two conditions become one
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
7. **What goes.** `Session::set_network` and `Keys::set_network` leave
   every path a person takes; `Keys::set_network` stays only for
   `Session::follow` (point 8) and a shell's start. Its mainnet refusal
   (`Refusal::NotPublicLoaded`) and the Network row's "Wallets loaded"
   note (`network_row_items`, `network_row_note`) go.
8. **`Session::follow`** (a wallet or a PSBT on another network) moves
   the session only when all hold: no key and no wallet loaded, not
   Demo, the network not fixed, and the mode it moves to in the
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
  "Regtest"), and for Demo its name and its network, "Demo · Testnet"
  (owner, 2026-10-22: a person sees that Demo runs on testnet), on the
  wide and the compact layout alike; the same caution pill for each. Places:
  `header_network_pill` and `network_pill_left`
  (`faraday/faraday-core/src/screens/mod.rs`), `compact::tools` and
  compact Home (`faraday/faraday-core/src/compact/mod.rs`). The
  comments "a badge … opens nothing" go.
- **Load samples pill**: in Demo only, when `Faraday::samples` is set
  (§5), immediately left of the Demo pill, a token gap between, same
  height and shape, in the accent tone (it is an action, the mode pill
  is a state). Label "Load samples" wide, "Samples" on the compact bar
  and compact Home. `network_pill_left` returns the left edge of the
  pair, so the row's next item moves along.
- **Settings**: the Network row (`network_row_items`) is replaced by one
  reference row, **Mode**, its value the mode's name, a chevron; it
  opens the same sheet. On Mainnet, where no pill shows, it is the only
  door.
- **The sheet** (`Sheet::Mode`, modes/screen.rs): title "Mode"; one row
  per mode in `self.modes`, in that order; the mode in use marked as a
  single choice is (DESIGN §4.2); the Demo row carries "Testnet" as its
  value, as a row with a value does (DESIGN §4.2); a press on another row is
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

**Desktop** (`faraday/shells/desktop/src/main.rs`):

- `desktop_app_for(release)` becomes `desktop_app_in(mode, warned)`:
  `Session::on(mode.network())`, `set_demo` when the mode is Demo,
  `app.modes = DESKTOP_MODES`, `app.samples = Some(&SAMPLES)` (from M5).
  Every build starts in Demo, the developer's, the test's and the
  release's.
- `restart` reads `app.mode_after_restart()` from the app before
  replacing it, and builds `desktop_app_in` with it: the mode carries
  across a lock.
- **No online warning** (owner, 2026-10-22). F61's `Sheet::NotAirgapped`
  goes, with `with_start_warning`, `without_airgap_warning`,
  `airgap_warned`, `test_asked`'s use for it, the `test_network_*`
  strings, the boot import's wait under it and `devices.rs`'s guard;
  `Action::Network` goes. A person can type a seed on an online computer
  in any mode, Demo included (Demo swaps it for a sample only after it
  is typed), so a sheet on one path protects nothing.
- **Instead, one line at every place a secret is typed** (owner,
  2026-10-22): on an online shell (`Faraday::online`, set by the desktop
  and the web shells; never on the stick or the Pi), every step where a
  seed or a secret can be typed or pasted draws one caution-tone line
  under its field, `never_real_seed` "Never type a seed that holds real
  bitcoin here.", in every mode, Demo included: words (BIP-39, every
  length), a seed scanner's Type and Paste, a SLIP-39 share, a codex32
  string, an LND cipher seed and its passphrase, hex entropy, an xprv or
  a WIF, a BIP-39 passphrase, a vault's or a backup's passphrase where it
  can be typed (not in Demo, where it is `password`, §6.2). A statement
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
`Keys::set_demo`'s list, so Demo takes it however it arrives.

| Sample | Form | Source | Checked against |
|---|---|---|---|
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
| Encrypted backup | an `.oskb` holding a 32-byte master seed, no words | new, made under `password` with `tools/vectors/backup/backup-v3-seed-32.oskb`'s payload (every byte `42`) | opened by `faraday-backup` and by `tools/backup/decrypt.py` to that payload |
| LND cipher seed | aezeed, for Lightning node key only; public node id | LND's published vector, `PRODUCTION_1` in `core/faraday-bip/src/aezeed.rs` (no passphrase) | the node id `aezeed.rs`'s test names |

**With a passphrase.** Every key form that takes a passphrase (BIP-39,
SLIP-39) is public twice: with none, and with `password` (§6.2). Both
fingerprints are in the public set; the vectors' own passphrases
(`TREZOR`, the Japanese case's) are used only by the test that checks
the sample against its vector, since Demo never takes another
passphrase. codex32 and a master seed take none.

Not separate samples, decided: **SeedQR and CompactSeedQR** are
spellings of words: Use a sample on a scanner that takes a seed hands
over a sample's Standard SeedQR digits, so the scanner's own decoding
runs; **xprv, WIF**: Faraday loads no key from them; **a vault**: §5.3;
**a BIP-85 child**: disabled in Demo (DEMO §4).

**Loading a set.** A set's tick loads exactly its threshold, the parts
the vector gives, and so the key; not one short. Practising the last
part is Add a key's job, with Use a sample filling the next part (§6).

**Paths** (all through what a typed or scanned key takes, never a new
one): words, through `Keys::add_words_with`; SLIP-39 shares and a
codex32 or Seed XOR set, through `inbox::Secret::add` (words,
`slip39_seed` then `add_seed`, `codex32_seed` then `add_seed`); the
`.oskb`, through `Keys::add_backup`; the cipher seed, through
`faraday_keys::lightning::node_from_aezeed`.

**Constraints already in place.** New key's handout (§7) and
`makes_secrets` (DEMO §4) are unchanged in what they refuse: SLIP-39,
Seed XOR and codex32 *splits* of a key held stay disabled (fresh
randomness), and so does Show for a node's private key. Loading a
sample set is not a split: its parts are published. DEMO §6's LND
cipher seed row opens in Demo, and `node_from_aezeed` refuses a cipher
seed whose node id is not in the public set (`Keys::is_public_node`).

### 5.2 Wallets

The testkit's wallets, testnet, coin type 1, over the test seeds bacon,
zebra and summer: Savings (2 of 3), Spending, Taproot, Nested SegWit,
Legacy, Nested multisig, Legacy multisig, Taproot multisig, Inheritance,
Taproot tree (`testkit::kits`). Also, decided:

- **MuSig2** (`testkit::musig_policy`): its keys are the three test
  seeds; MuSig2 is open in Demo.
- **FROST 2 of 3** (`testkit::threshold_record`): shares 1 and 2 are
  bacon and zebra; the record is public. Share 3, computed by the
  testkit's deal, is not a sample and is not loaded.
- **3 of 5** (`testkit::three_of_five`): its five keys are master seeds
  01…01 to 05…05, keys with no words; they load as keys and join the
  public set. It has no spend.

A wallet's tick loads the wallet and every seed of it that is a sample
(`Session::add_wallet`, then the keys). Each wallet sample carries its
unsigned spend where the testkit makes one (`testkit::unsigned`,
`musig_unsigned`, `threshold_unsigned`), for §6.

### 5.3 Other inputs

- **Passphrases and passwords**: `password`, everywhere (§6.2).
  `demo::PASSPHRASE` becomes `"password"` and stays in `faraday-core`;
  `PASSPHRASE_WORDS` goes.
- **A vault file**: a vault made like the testkit's
  (`testkit::test_vault`: test key 1 and the Savings wallet) under the
  one passphrase `password`, as a literal file.
- **A signed message**: one made by sample Spending's first receive
  address, as a literal; **an address**: Spending's receive address 0.
- **A TOTP setup code**: the testkit's `ENTRIES_FILE` first line (RFC
  6238's published example secret).

### 5.4 Where they live

- A new crate, faraday/faraday-samples, of public data only: `&str`,
  `&[u8]` (the `.oskb` and the vault, `include_bytes!` of files in the
  crate) and fingerprints as `[u8; 4]`, in the types
  `faraday_core::samples` defines (`SampleKey`, `SampleWallet`,
  `Samples`, plain structs of `&'static` fields). Its one item:
  `pub static SAMPLES: Samples`. It parses nothing and derives nothing,
  so lint-secrets rule 4 has nothing to refuse there.
- Names on screen are not in the crate: each sample carries a key into
  `faraday/faraday-strings/src/en.rs` (§8).
- `Faraday::samples: Option<&'static Samples>`, set by the desktop and
  the website. With `None` (the stick, the Pi, the Starter's Faraday)
  no pill, no sheet and no Use a sample is drawn or acted on.
- **Gate**: the crate, not a feature. `tools/lint-keys-tests.sh` gains
  a third check: `cargo tree -i faraday-samples` over faraday-stick,
  faraday-disk, faraday-grant, faraday-boot and faraday-starter-shell
  (normal edges, every target) must find nothing. The desktop depends on
  it normally; faraday-core never does.
- **The test** (faraday-samples' tests/, faraday-core's `testkit` as a
  dev-dependency): every word sample's fingerprint equals what
  `Keys::add_words_with` gives; every wallet's descriptor equals the
  testkit's; every unsigned spend equals the testkit's bytes; every
  vector sample equals its vector file read at test time (words,
  shares, strings, seed, xprv where the vector has one); the `.oskb`
  and the vault open under `password` to what the table says;
  the cipher seed's node id is the one `aezeed.rs`'s test names; and
  the handout list holds every length New key offers.
- `demo::SEEDS` and `demo::words` move into the crate; `demo.rs` keeps
  `PASSPHRASE`, `makes_secrets` and `demo_refuses`.
- The testkit stays a test-only feature (F70, `docs/KEYS.md`); the
  desktop's dev-dependency on it is unchanged.

## 6. Use a sample

In Demo, with `samples` set, each step below draws **Use a sample**
inside its empty field (§6.1), or on the scanner a **Sample** tile. A
press puts the sample into the step through the function the step's
own input already calls, as if typed or scanned, and the flow goes on
as it would. Which sample, decided: **the next one not used**, by a
cursor on the step's state that a press moves (no picker); a press
again replaces it with the next, wrapping. "Not used" means not
loaded, and for a set, not yet begun.

**Placement.** Inside the empty field, where a placeholder would be.
The scanner, which has no field: a third tile in the band, after Paste
and Type (DESIGN §4.9), with an existing icon. Never drawn on Mainnet,
Testnet, Signet or Regtest; drawn on the website.

**The same value again.** Where a flow asks again for a value a sample
filled, the button fills the matching one, read from the state that
already holds it: the next share of the set begun (`forms::Parts`), the
SeedQR of the key a copy check names, a passphrase (always `password`,
§6.2). Nothing new is kept for it. A value the person typed is theirs
to remember, as for real.

### 6.1 The field's inner action (faraday-ui)

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
  unmasked in Demo: samples are public. Typing into the field masks it
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
| The scanner: a file (a wallet, a PSBT) | `ScanPurpose::Files` from Restore, Spend, Decode | Restore: the next sample wallet not loaded, as its descriptor; Spend and Decode: the unsigned spend of the wallet in hand, else of the first loaded sample wallet |
| The scanner: a copy to check | `ScanPurpose::CheckCopy(fp)` | the SeedQR of key `fp` when it is a sample; no tile otherwise |
| The scanner: a TOTP code | `ScanPurpose::VaultEntry` | §5.3's setup code |
| Scan a key (a wallet's missing key) | `faraday/faraday-core/src/scan_key.rs` | as WalletKey above |
| Restore, seeds first: a cosigner's account key | `faraday/faraday-core/src/seeds/mod.rs` `Focus::Cosigner` | the account key, at the path chosen, of the next sample seed not among the wallet's |
| The Spend tab's words | `faraday/faraday-core/src/family/mod.rs` | as Add a key |
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

### 6.2 Every passphrase is `password`

In Demo every passphrase and password field holds `password`, drawn
unmasked, and takes no typing, no paste and no sample chip; the eye and
its 30 s countdown are not drawn there. Where a field is optional (a
key's BIP-39 passphrase, a seed read's, the potential wallet's), the
person still chooses whether to use one; when they do, it is
`password`. A second field that types a passphrase back is filled the
same, so "The two passphrases differ" never shows. The Load samples
sheet and the Learn page each say it once: "In Demo, every passphrase
is password." No field explains it.

`Faraday::demo_passphrase()` (true in Demo) is the one test; each field
below asks it where it takes a key press and where it draws, and fills
itself from `demo::PASSPHRASE` when its step opens.

| Field | Where |
|---|---|
| New key's BIP-39 passphrase and its repeat | `faraday/faraday-core/src/keygen/mod.rs`, `keygen/screen.rs` (`KPassField`) |
| A seed read's passphrase and repeat | `faraday/faraday-core/src/seeds/read.rs` (`Sheet::SeedRead`) |
| The potential wallet's passphrase | `Sheet::Potential` (`potential_key`, lib.rs) |
| Make a new wallet's passphrase slot | `faraday/faraday-core/src/create.rs` (`pass_slot`, `pass_focus`) |
| Create a vault's passphrases (every slot) and their repeats | `faraday/faraday-core/src/vault/mod.rs` (`CreateForm::phrases`), `vault/screen.rs` |
| Unlock, the boot import's passphrase, a passphrase prompt | `vault/mod.rs` `Focus::Passphrase`, `Focus::Prompt` (`Sheet::Import`, `Sheet::PassphraseSeeds`) |
| The Spend tab's vault passphrase | `faraday/faraday-core/src/family/mod.rs` |
| Check my copy's passphrase | `faraday/faraday-core/src/screens/backup.rs` (`check.passphrase`) |
| An `.oskb` opened, and one written (Out to an encrypted backup) | `inbox.rs`; the backup flow's encrypted-backup step (`faraday/faraday-core/src/backup_flow/`) |
| A KeePass export's password | the vault's KeePass export (`vault/out.rs`) |
| A vault entry's password field | Vault contents' entry form (`vault/screen.rs`) |
| An aezeed's passphrase, when added | `faraday/faraday-core/src/lightning/mod.rs` |

**Generate with dice** (DEMO §5.2) still takes the rolls and shows the
words; when the last word is in, both fields take `password` in place
of them, and the card's one line says so (`vault_demo_uses_public`).

**Refusals of a weak passphrase.** There are none to loosen: Faraday
refuses an empty passphrase and a repeat that differs
(`Vaults::phrase_ok`, `keygen_the_two_passphrases`,
`seed_read_the_two_passphrases`, `create_pass_lock_the`), and measures
strength only for words the dice made (`Vaults::phrase_bits`; a typed
passphrase's is not measured). The dice count (how many rolls make a
word, how many words) is unchanged. In Demo `phrase_bits` gives `None`
for `password` (the dice's words are dropped), so no strength line and
no "weakest passphrase" line is drawn; `PASSPHRASE_WORDS` goes. Outside
Demo nothing changes: a test types a short passphrase on Testnet and on
the stick and sees it taken as typed, masked and editable, with the
dice's strength line where the dice made it.

## 7. New key in Demo

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
`password`. A passphrase chosen on the Key card is `password` (§6.2),
and the handout is added with it. A typed key (`KeyGen::typed`) is not
replaced.

## 8. Strings

All in `faraday/faraday-strings/src/en.rs`, each with its doc line as
the file has them.

| Name | Text |
|---|---|
| `mode_sheet_title` | Mode |
| `mode_demo` | Demo |
| `mode_demo_pill` | Demo · Testnet |
| `never_real_seed` | Never type a seed that holds real bitcoin here. |
| `mode_not_now` | Not now |
| `settings_mode` | Mode |
| `samples_pill` | Load samples |
| `samples_pill_short` | Samples |
| `samples_title` | Load samples |
| `samples_wallets` | Wallets |
| `samples_keys` | Keys |
| `samples_select_all` / `samples_unselect_all` | Select all / Unselect all |
| `samples_not_now` | Not now |
| `samples_load` | Load ticked |
| `samples_password` | In Demo, every passphrase is password. |
| `samples_loaded` | Loaded {} (`{}`: how many, as the Load sheet's toast counts) |
| `use_a_sample` | Use a sample |
| `scan_sample` | Sample |
| `sample_words` | {} ×{} (a word, a count) |
| `sample_abandon` | abandon … about |
| `sample_japanese` | Japanese words |
| `sample_slip39_one` / `sample_slip39_two_of_three` / `sample_slip39_groups` | SLIP-39, one share / SLIP-39, 2 of 3 / SLIP-39, groups |
| `sample_codex32_secret` / `sample_codex32_shares` | codex32 secret / codex32, 2 shares |
| `sample_xor` | Seed XOR, 2 parts |
| `sample_backup` | Encrypted backup |
| `sample_aezeed` | LND cipher seed |
| `sample_savings` … `sample_tree` | Savings, Spending, Taproot, Nested SegWit, Legacy, Nested multisig, Legacy multisig, Taproot multisig, Inheritance, Taproot tree |
| `sample_musig` / `sample_frost` / `sample_three_of_five` | MuSig2 / FROST 2 of 3 / 3 of 5 |
| `keygen_lock_in_adds` (changed) | Lock in adds sample {} ({}) in place of these words |
| `keygen_added_public_seed` → `keygen_added_sample` | Added sample {} ({}) in place of the key made |
| `wallet_is_not_a_key` (changed) | {} is not a sample: Demo takes only samples |
| `vault_demo_uses_public` (changed) | Demo puts password in place of these words |
| `wallet_public_seeds_only`, `network_row_note_wallets` | removed |

## 9. The website (batch W1, in its repository)

The website is pinned to an older Faraday (crates still `osk-*`); W1
moves the pin to the commit after M8 by the procedure its HANDOFF.md
gives, and makes the changes below there. Faraday's repository holds
nothing of it.

- `web/Cargo.toml`: faraday-core without `testkit`; faraday-samples
  added.
- `web/src/lib.rs` (`start`): `Session::on(Network::Testnet)`,
  `set_demo` with the samples, `fix_network`, `app.modes =
  DESKTOP_MODES`, `app.samples = Some(&SAMPLES)`. The pills show "Demo ·
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
    (`backup_answers`), under the one passphrase `password` (Demo's;
    the second, easy slot goes, since it was the same word); Seed B's
    paper and the phone as above. The sticky note says `password`.
  - **Start fresh** loads no sample: the story is making a wallet, and
    its key comes from New key's handout (a 12 or 24-word sample by the
    length chosen).
  - `desk.rs`'s own `descriptor`, `account`, `spend`, `fingerprint`,
    `PATH` and `NET` go; it reads the samples. Hints and walk steps
    that name an amount say tBTC where Faraday shows tBTC; those that
    name the vault's passphrase or the easy one say `password`.
- Tests: each story walked to its end (`stories.rs`), on testnet; the
  papers' SeedQRs and the phone's PSBT equal the samples'; nothing in
  the website depends on faraday-core's `testkit` (a check in its
  `just`).
- HANDOFF.md and the page's own words follow: "mainnet" goes from the
  demo's description.

**Where the website and the desktop still differ in Demo**, by design:

| | Desktop | Website | Why |
|---|---|---|---|
| Switching mode | Demo, Testnet, Signet, Regtest | Demo only | a public page takes samples only; a person's own test seeds belong in the downloaded app |
| Files | the Inbox and Outbox are folders on disk (Downloads) | sticks in memory, drawn round the laptop | a browser has no folder Faraday may watch |
| Paste | the clipboard, a seed on a seed scanner (F75) | none | the web shell has no clipboard |
| Camera | the computer's cameras | one webcam that sees the desk's props | the props are the page's way in |

## 10. Batches

Each is one agent's work. Run the touched test module while working and
`just` once at the end (CLAUDE.md). Tests state what a person or a
wallet would notice.

### M1. Modes in the keys and the core

- **Files**: `faraday/faraday-keys/src/session.rs` (`demo`, `set_demo`
  still taking words until M5, `has_demo` gone, `set_network` kept for
  start and `follow`); `faraday/faraday-core/src/wallet.rs`
  (`set_network` gone, `follow` per §2.8, `shown` and `broken_checksum`
  gone); modes/mod.rs (new: `Mode`, `modes`, `mode`, `next_mode`,
  `mode_after_restart`, `Action::Mode`); `faraday/faraday-core/src/demo.rs`;
  `faraday/faraday-core/src/devices.rs` (`Action::Network` gone; no
  warning on a mode change, §4); `faraday/faraday-core/src/learn.rs`; lib.rs
  (code map, `Action::Mode` routed, the lock reading `next_mode`).
- **Tests** (rewrite `faraday/faraday-core/tests/all/network.rs` and
  `demo.rs`, `airgap.rs` to the new rule; new tests/all/modes.rs):
  choosing a mode with keys loaded leaves a fresh session in that mode
  (no key, no wallet; `mode_after_restart`); a lock keeps the mode;
  choosing one with LockAsk's condition shows LockAsk; a mode not
  offered does nothing; Demo's addresses are testnet addresses that a
  testnet wallet accepts; Demo refuses a key not public and takes a
  public one with a passphrase that makes a public key; a testnet
  wallet on an empty stick session moves it to Testnet, and is refused
  once a key is loaded; nothing in Demo follows a mainnet wallet; every
  action of DEMO §4 is dimmed in Demo and live on Testnet.
- **Docs**: DEMO §2 (the definition: Demo, a mode on testnet; the
  stick has none), §4's lead line, §5's "broken checksum" and
  "mainnet" mentions, §6's LND row (open in Demo, LND's vector only,
  from M7). `docs/learn/faraday/demo.md`: "What the demo is" (Demo is
  Faraday on testnet with sample keys; the desktop starts in it), "The
  public seeds" (on testnet; Demo takes only samples), the broken
  checksum section removed.
- **Done**: `just` passes; no code reads `has_demo`, `set_network` from a
  press, or `broken_checksum`.

### M2. The pill, the sheet, Settings

- **Files**: `faraday/faraday-core/src/screens/mod.rs`
  (`header_network_pill`, `network_pill_left`),
  `faraday/faraday-core/src/compact/mod.rs` (`tools`, Home), modes/screen.rs
  (new: the sheet, wide and compact),
  `faraday/faraday-core/src/screens/settings.rs` (the Mode row;
  `network_row_items`, `network_row_note` gone), lib.rs (`Sheet::Mode`,
  `Action::ModeSheet`, the sheet's Escape and Back),
  `faraday/faraday-strings/src/en.rs` (the `mode_*` and `settings_mode`
  strings). The Load samples pill is M6.
- **Tests**: off Mainnet, Home and every page offer the pill and it
  opens the sheet, on the wide and the compact layout; on Mainnet no
  pill shows and Settings' Mode row opens the sheet; the sheet offers
  each mode of `modes` and marks the one in use; a fixed session's
  sheet offers no other mode; Not now and Escape close it. The budget
  tests (`faraday/faraday-core/src/work/budget/presses.rs`) name
  nothing costly for the pill's press or the sheet's frames.
- **Docs**: DESIGN §4.8, Network row: "A caution pill naming the mode,
  DEMO / TESTNET / SIGNET / REGTEST; absent on mainnet. Pressing it
  opens the mode sheet. On Home, on every page's top row, and on every
  Sign review screen."; DESIGN §5's Settings entry (Mode row);
  `docs/FLOWS.md` where it names the Network row.
- **Done**: snapshots of Home at 480x640 and 1280x800 in Demo and
  Testnet show the pill; the sheet renders on both.

### M3. The desktop starts in Demo; the online warning goes

- **Files**: `faraday/shells/desktop/src/main.rs` (`desktop_app_in`,
  `restart`, its tests); `faraday/faraday-core/src/lib.rs`
  (`Sheet::NotAirgapped`, `with_start_warning`, `without_airgap_warning`,
  `airgap_warned` gone); `faraday/faraday-core/src/screens/sheets.rs`
  (`not_airgapped_sheet` gone) and screens/mod.rs's routing of it;
  `faraday/faraday-core/src/boot_import/mod.rs` (no wait under it);
  `faraday/faraday-strings/src/en.rs` (`test_network_*` gone,
  `never_real_seed` added); `Faraday::online` (lib.rs), set in
  `faraday/shells/desktop/src/main.rs`; the line drawn at every step §4
  lists (seeds/, keygen/, scan_key.rs, the entry screens for passphrases,
  xprv, WIF, hex, SLIP-39, codex32, aezeed — find each through the
  functions that take typed secret input).
- **Tests** (desktop's own; tests/all/airgap.rs rewritten as the rule
  "no warning"): every build opens in Demo with no sheet; a release
  build leaving Demo for Testnet goes straight to Testnet in a fresh
  session; a lock in Testnet comes back in Testnet; the boot import
  offers its files at once on every build; on an online app every
  secret-typing step shows the line, in Demo and on Testnet, and on the
  stick none does.
- **Docs**: README's Faraday Desktop section (starts in Demo; Testnet,
  Signet, Regtest from the pill); DEMO §1.1 and §2; `docs/STARTER.md`
  §1.1's description of the desktop file; the Learn page's "Test
  networks" (the desktop starts in Demo; never a seed that holds real
  bitcoin, stated as fact).
- **Done**: `just faraday-snap` of the desktop tour shows
  Demo on Home.

### M4. Stick and Pi keep the mode across a lock

- **Files**: `faraday/shells/stick/src/main.rs` (write the `mode` file
  before exit 75; read, check and remove it at start).
- **Tests** (the stick shell's tests, against a run directory in a
  temporary folder): a lock in Signet starts the next app in Signet; a
  file naming Demo, or anything else, starts on Mainnet; the file is
  gone after start.
- **Docs**: the image README's line on what the run directory holds.
- **Done**: `just` passes.

### M5. The samples crate, the public set, handouts by length

- **Files**: faraday/faraday-samples (new: `Cargo.toml`, `src/lib.rs`,
  the `.oskb` and vault files, tests/); `faraday/faraday-core/src/demo.rs`
  (`SEEDS`, `words` moved out); samples/mod.rs in faraday-core (the
  types); `faraday/faraday-keys/src/session.rs` (`set_demo` by
  fingerprints, `next_handout(words)`, `is_public_node`);
  `faraday/faraday-keys/src/lightning.rs` (refuse a non-public cipher
  seed in Demo); `faraday/faraday-core/src/keygen/mod.rs` and
  `keygen/screen.rs` (handout by length); `tools/lint-keys-tests.sh`
  (§5.4); workspace `Cargo.toml`; `faraday/shells/desktop/Cargo.toml`;
  `supply-chain/` only if cargo-vet asks (it is a path crate).
- **Tests**: §5.4's; New key at 12, 15, 18, 21 and 24 words in Demo
  adds a sample of that length; with every 12-word sample loaded,
  12-word Lock in is dimmed and 24-word Lock in is not; `just lint`
  fails on a scratch change that makes faraday-stick depend on
  faraday-samples (checked by hand once, not committed).
- **Docs**: `docs/KEYS.md` (§5.4's crate beside the testkit, in one
  paragraph); DEMO §5.1; the Learn page's "New key in the demo" (a
  sample of the length chosen); the strings of §8 for New key.
- **Done**: `cargo tree -i faraday-samples` names only faraday-desktop
  (and faraday-samples' own tests); the desktop's start no longer
  derives any key.

### M6. Load samples

- **Files**: samples/screen.rs in faraday-core (the sheet, modelled on
  `faraday/faraday-core/src/vault/load_screen.rs`: Wallets, then Keys,
  each a `Ui::multi_list` (`faraday/faraday-ui/src/lib.rs`), Select all
  or Unselect all over them, Not now and Load ticked at the foot);
  samples/mod.rs (the ticks on the state, `Action::Samples`,
  `Action::SampleTick`, `Action::SamplesLoad`); the pill in
  `screens/mod.rs` and `compact/mod.rs`; en.rs (`samples_*`,
  `sample_*`).
- **One line** above the lists, muted: "In Demo, every passphrase is
  password." (`samples_password`). No "in use" list: with every
  passphrase `password`, what is in use is the keys and wallets loaded,
  which the lists already show ticked and dimmed.
- **Behaviour**: a sample already loaded is ticked and dimmed; nothing
  is ticked at first otherwise. Load ticked loads wallets first, then
  keys, through §5.1's paths, in the handler, one toast for the count;
  a refusal (a duplicate) is skipped, never shown as an error.
- **Tests** (faraday-samples' tests/, which drive a `Faraday` with
  `samples` set): in Demo the pill opens the sheet listing every sample;
  ticking Savings and Load ticked loads Savings and bacon, zebra and
  summer, and Savings' first address is the testkit's; ticking the
  SLIP-39 2 of 3 set loads its key, whose fingerprint is the vector's;
  the `.oskb` tick loads the 32-byte seed's key; loaded samples come
  back ticked and dimmed; New key's next handout skips a sample the
  sheet loaded; on Testnet and on the stick no pill shows and the action
  does nothing. Budget: the sheet's frames do no costly work; Load
  ticked's press is named in the budget with its derivations.
- **Docs**: DESIGN §4.8 (the Load samples pill beside the Demo pill);
  the Learn page, a section "Samples": every sample's name and, for
  keys, its words, shares or strings (Learn is not a working screen, so
  a person can type them from there), and that anything sent to them
  can be taken by anyone; DEMO §5's Keys line.
- **Done**: snapshots of the sheet at 480x640 and 1280x800.

### M7. Use a sample: keys and the scanner

- **Files**: the rows of §6 for Add a key, the scanner (every
  `ScanPurpose` in the table), Scan a key, New key's typed key, the seed
  Restore's cosigner box, the Spend tab's words; `faraday/faraday-core/src/screens/sheets.rs`
  (`scan_sheet`'s third tile); `faraday/faraday-ui/src/lib.rs`
  (`field_action`, §6.1); en.rs
  (`use_a_sample`, `scan_sample`).
- **Tests** (faraday-samples' tests/): in Demo, for each row, a person
  presses Use a sample (or the Sample tile) and the flow goes on: Add a
  key with 12 words loads twelve ×12, a second press puts ocean in its
  place; SLIP-39 fills share 1 then share 2 and the key loads; codex32
  and Seed XOR alike; the seed scanner loads a sample through its
  SeedQR; a wallet's missing key loads the matching sample and that
  wallet then signs its sample spend; Restore by file loads the next
  sample wallet; the envelope takes the spend and the flow reaches the
  review; the copy check passes for a sample key; on Testnet no button
  is offered. The chip: pressing in an empty field and typing hides it
  and the typed text is kept; a sample-filled share is shown unmasked;
  the second share's chip fills the same set's next share.
- **Docs**: DESIGN §4.9's scanner band (a Sample tile, in Demo only);
  DESIGN §4.3 (Use a sample on an entry, in Demo only); DEMO §5.
- **Done**: snapshots of Add a key and the scanner in Demo, both sizes.

### M8. Every passphrase is `password`; vaults, Lightning, checks

- **Files**: every field of §6.2's table, `Faraday::demo_passphrase`;
  `faraday/faraday-core/src/demo.rs` (`PASSPHRASE` = `"password"`,
  `PASSPHRASE_WORDS` gone); `vault/mod.rs` (Generate with dice puts
  `password`; `phrase_bits` gives `None` for it); the remaining rows of
  §6's table (the Vaults screen's sample vault, Lightning's cipher seed,
  whose row opens in Demo, Check a signed message and Check an address,
  the TOTP code); en.rs (`samples_password`, `vault_demo_uses_public`).
- **Tests**: in Demo each field of §6.2 shows `password`, takes no
  typing, and the flow goes on: New key with a passphrase adds the
  handout's `password` key, which the public set takes; Create a vault
  makes one that unlocks with `password`; Generate with dice shows the
  rolled words and the field takes `password`; an `.oskb` written in
  Demo opens with `password`; the sample vault appears in Vaults and
  unlocks to the Load sheet; Lightning's cipher seed row fills LND's
  vector and shows its node id, a typed cipher seed that is not the
  vector is refused, and Show stays dimmed; the signed message checks
  as valid; the address is found in Spending. Outside Demo, on Testnet
  and on the stick: a short passphrase is taken as typed, masked and
  editable; two that differ are still refused; the dice's strength
  line is drawn.
- **Docs**: DEMO §5.2 (`password` in place of the rolled words), §6
  (the LND row; typing a password into an Entry: now `password`, no
  longer typed), §4 (the Lightning rows unchanged); the Learn page: "In
  Demo, every passphrase is password." once, Generate with dice's
  passphrase changed, "What the demo does not make" without the cipher
  seed; DESIGN §4.10's Mask policy (a Demo exception: sample secrets
  and `password` are drawn unmasked); DEMO's header and every remaining
  "mainnet" meaning the demo. DEMO is then Reference again with Demo as
  built; this file goes to `docs/history/` once W1 is done.
- **Done**: `just` passes; `bash tools/lint-docs.sh` passes.

### W1. The website

§9. In the website's repository; its HANDOFF.md is the contract. The
web shell sets `Faraday::online`, so §4's line shows at every
secret-typing step there too.

## 11. Settled with the owner (2026-10-22)

1. A lock comes back in the mode the person was in, a test network as
   Demo; leaving a mode is always a choice made on the pill.
2. Demo's addresses are real testnet addresses: the broken checksum
   goes (§2 item 4).
