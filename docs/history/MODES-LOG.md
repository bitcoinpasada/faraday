# Modes, Practice and samples: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/MODES.md` on 2026-11-05. Each keeps its
original number. The labels in them are the ones decisions and old
commits use.

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
  offered does nothing; Practice's addresses are testnet addresses that a
  testnet wallet accepts; Practice refuses a key not public and takes a
  public one with a passphrase that makes a public key; a testnet
  wallet on an empty stick session moves it to Testnet, and is refused
  once a key is loaded; nothing in Practice follows a mainnet wallet; every
  action of DEMO §4 is dimmed in Practice and live on Testnet.
- **Docs**: DEMO §2 (the definition: Practice, a mode on testnet; the
  stick has none), §4's lead line, §5's "broken checksum" and
  "mainnet" mentions, §6's LND row (open in Practice, LND's vector only,
  from M7). `docs/learn/faraday/demo.md`: "What Practice is" (Practice is
  Faraday on testnet with sample keys; the desktop starts in it), "The
  public seeds" (on testnet; Practice takes only samples), the broken
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
- **Done**: snapshots of Home at 480x640 and 1280x800 in Practice and
  Testnet show the pill; the sheet renders on both.

### M3. The desktop starts in Practice; the online warning goes

- **Files**: `faraday/shells/desktop/src/main.rs` (`desktop_app_in`,
  `restart`, its tests); `faraday/faraday-core/src/lib.rs`
  (`Sheet::NotAirgapped`, `with_start_warning`, `without_airgap_warning`,
  `airgap_warned` gone); `faraday/faraday-core/src/screens/sheets.rs`
  (`not_airgapped_sheet` gone) and screens/mod.rs's routing of it;
  `faraday/faraday-core/src/boot_import/mod.rs` (no wait under it);
  `faraday/faraday-strings/src/en.rs` (`test_network_*` gone,
  `never_real_seed` added); `Faraday::online` and `Faraday::seed_line`
  (lib.rs), set in `faraday/shells/desktop/src/main.rs`; the line drawn at every step §4
  lists (seeds/, keygen/, scan_key.rs, the entry screens for passphrases,
  xprv, WIF, hex, SLIP-39, codex32, aezeed — find each through the
  functions that take typed secret input).
- **Tests** (desktop's own; tests/all/airgap.rs rewritten as the rule
  "no warning"): every build opens in Practice with no sheet; a release
  build leaving Practice for Testnet goes straight to Testnet in a fresh
  session; a lock in Testnet comes back in Testnet; the boot import
  offers its files at once on every build; on an online app every
  secret-typing step shows the line, in Practice and on Testnet, and on the
  stick none does.
- **Docs**: README's Faraday Demo section (starts in Practice; Testnet,
  Signet, Regtest from the pill); DEMO §1.1 and §2; `docs/STARTER.md`
  §1.1's description of the desktop file; the Learn page's "Test
  networks" (the desktop starts in Practice; never a seed that holds real
  bitcoin, stated as fact).
- **Done**: `just faraday-snap` of the desktop tour shows
  Practice on Home.

### M4. Stick and Pi keep the mode across a lock

- **Files**: `faraday/shells/stick/src/main.rs` (write the `mode` file
  before exit 75; read, check and remove it at start).
- **Tests** (the stick shell's tests, against a run directory in a
  temporary folder): a lock in Signet starts the next app in Signet; a
  file naming Practice, or anything else, starts on Mainnet; the file is
  gone after start.
- **Docs**: the image README's line on what the run directory holds.
- **Done**: `just` passes.

### M5. The samples crate, the public set, handouts by length

- **Files**: faraday/faraday-samples (new: `Cargo.toml`, `src/lib.rs`,
  the vault file, tests/); `faraday/faraday-core/src/demo.rs`
  (`SEEDS`, `words` moved out); samples/mod.rs in faraday-core (the
  types); `faraday/faraday-keys/src/session.rs` (`set_demo` by
  fingerprints, `next_handout(words)`, `is_public_node`);
  `faraday/faraday-keys/src/lightning.rs` (refuse a non-public cipher
  seed in Practice); `faraday/faraday-core/src/keygen/mod.rs` and
  `keygen/screen.rs` (handout by length); `tools/lint-keys-tests.sh`
  (§5.4); workspace `Cargo.toml`; `faraday/shells/desktop/Cargo.toml`;
  `supply-chain/` only if cargo-vet asks (it is a path crate).
- **Tests**: §5.4's; New key at 12, 15, 18, 21 and 24 words in Practice
  adds a sample of that length; with every 12-word sample loaded,
  12-word Lock in is dimmed and 24-word Lock in is not; `just lint`
  fails on a scratch change that makes faraday-stick depend on
  faraday-samples (checked by hand once, not committed).
- **Docs**: `docs/KEYS.md` (§5.4's crate beside the testkit, in one
  paragraph); DEMO §5.1; the Learn page's "New key in Practice" (a
  sample of the length chosen); the strings of §8 for New key.
- **Done**: `cargo tree -i faraday-samples` names only faraday-desktop
  (and faraday-samples' own tests); the desktop's start no longer
  derives any key.
- **As built**: `faraday_core::samples` holds `SampleKey`, `SampleWallet`,
  `SampleInputs` and `Samples` (a key's `name` is the field of its string
  in `en.rs`, which M6 adds; `parts` is what is typed or scanned;
  `plain` and `with_passphrase` are its fingerprints; `size` is what its
  form counts (M6b); a wallet's `kind` and `script` likewise; `listed` is false for
  the 3 of 5's five keys). The crate's `src/data.rs` and `data/` are
  written by `cargo run -p faraday-samples --example regenerate`. Core's
  tests do not link the crate: `testkit::demo_samples()` derives the 21
  handouts (`testkit::SAMPLE_WORDS`) and the crate's tests check the two
  agree. `Keys::node_from_aezeed` refuses a node not in the set while Practice
  is on; the action stays dimmed until M7. `Faraday::samples` is M6's.

### M6. Load samples

- **Files**: samples/screen.rs in faraday-core (the sheet, modelled on
  `faraday/faraday-core/src/vault/load_screen.rs`: Wallets, then Keys,
  each a `Ui::multi_list` (`faraday/faraday-ui/src/lib.rs`), Select all
  or Unselect all over them, Not now and Load ticked at the foot);
  samples/mod.rs (the ticks on the state, `Action::Samples`,
  `Action::SampleTick`, `Action::SamplesLoad`); the pill in
  `screens/mod.rs` and `compact/mod.rs`; en.rs (`samples_*`,
  `sample_*`).
- **One line** above the lists, muted: "In Practice, the sample passphrase is"
  (`samples_password`) then `faraday` (`demo::PASSPHRASE`) in the code
  box of §6.2. No "in use" list: with every
  passphrase `faraday`, what is in use is the keys and wallets loaded,
  which the lists already show ticked and dimmed.
- **Behaviour**: a sample already loaded is ticked and dimmed; nothing
  is ticked at first otherwise. Load ticked loads wallets first, then
  keys, through §5.1's paths, in the handler, one toast for the count;
  a refusal (a duplicate) is skipped, never shown as an error.
- **Tests** (faraday-samples' tests/, which drive a `Faraday` with
  `samples` set): in Practice the pill opens the sheet listing every sample;
  ticking Savings and Load ticked loads Savings and bacon, zebra and
  summer, and Savings' first address is the testkit's; ticking the
  SLIP-39 2 of 3 set loads its key, whose fingerprint is the vector's;
  loaded samples come
  back ticked and dimmed; New key's next handout skips a sample the
  sheet loaded; on Testnet and on the stick no pill shows and the action
  does nothing. Budget: the sheet's frames do no costly work; Load
  ticked's press is named in the budget with its derivations.
- **Docs**: DESIGN §4.8 (the Load samples pill beside the Practice pill);
  the Learn page, a section "Samples": every sample's name and, for
  keys, its words, shares or strings (Learn is not a working screen, so
  a person can type them from there), and that anything sent to them
  can be taken by anyone; DEMO §5's Keys line.
- **Done**: snapshots of the sheet at 480x640 and 1280x800.

### M6b. What each sample is, and the passphrase `faraday`

Owner, 2026-10-22, after M6.
- **Each row says what it is**, as its value (a row with a value,
  DESIGN §4.2), read from the sample's own data once when the sheet
  opens, never derived while drawing. Wallets: kind and script, e.g.
  "Savings" · "2-of-3 multisig · native SegWit", "Spending" ·
  "single-sig · native SegWit", "Taproot tree" · "single-sig · Taproot,
  script tree", "MuSig2" · "2-of-2 MuSig2 · Taproot", "3 of 5" ·
  "3-of-5 multisig · native
  SegWit". Keys: form and size, e.g. "bacon ×24" · "BIP-39 · 24 words",
  "twelve ×12" · "BIP-39 · 12 words", "SLIP-39 2 of 3" · "SLIP-39 · 2 of
  3 shares", "codex32" · "codex32 · secret", "Seed XOR" · "Seed XOR · 2
  parts". The labels are
  en.rs strings composed from the script and threshold the sample
  already records; a new sample takes its label from the same parts.
- **The passphrase is `faraday`**, all lowercase, in place of
  `password` everywhere M5 and M6 used it: `demo::PASSPHRASE`, the
  handouts' second fingerprint and the sample vault
  (remade under `faraday` by `examples/regenerate.rs`), the tests, the
  Learn page and DEMO.
- **The code box** of §6.2: the Load samples sheet's line shows
  `faraday` in it; the Learn page's Samples section shows it as inline
  code. M8 draws every Practice passphrase field the same way.
- **Tests**: every sample row offers its type; the sample vault
  opens under `faraday` and not under `password`; Practice takes a
  sample with `faraday` as its passphrase and refuses it with
  `password`.

### M7. Use a sample: keys and the scanner

- **Files**: the rows of §6 for Add a key, the scanner (every
  `ScanPurpose` in the table), Scan a key, New key's typed key, the seed
  Restore's cosigner box, the Spend tab's words; `faraday/faraday-core/src/screens/sheets.rs`
  (`scan_sheet`'s third tile); `faraday/faraday-ui/src/lib.rs`
  (`field_action`, §6.1); en.rs
  (`use_a_sample`, `scan_sample`).
- **Tests** (faraday-samples' tests/): in Practice, for each row, a person
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
- **Docs**: DESIGN §4.9's scanner band (a Sample tile, in Practice only);
  DESIGN §4.3 (Use a sample on an entry, in Practice only); DEMO §5.
- **Done**: snapshots of Add a key and the scanner in Practice, both sizes.
- **As built**: `faraday-core/src/samples/put.rs` holds the chip's state
  (`Use`, on `EntryState` and `SeedsState`; a cursor each, one for each
  word count), the choice of the next sample (`sample_choose`, a lookup
  against the keys loaded), the chip's drawing helpers (tokens named in
  `faraday-draw`) and the presses: `Action::UseSample` for Add a key, New
  key's typed key and the Spend tab's words; `Action::ScanSample` for the
  scanner, which hands the sample's bytes to `Faraday::scanned` as the
  camera would; `SeedsAction::Sample(k)` for a cosigner's box, which
  derives the account key at the path chosen of the next sample seed not
  among the wallet's (`faraday_keys::identity::key_text_of_words`; the
  press is slow). `Action::EntryField` is a press in Add a key's field;
  `SeedsAction::Open` opens a cosigner's box from Its xpub without
  counting as a press in it. Add a key's chip prefers the sample of the
  key `wanted`, or one the wallet it is opened for is short of. On the
  small panel the word entry's chip stands in the word being typed, the
  grid being below its keyboard. The Lightning cipher seed action is no
  longer dimmed in Practice. `EntryRecover` is a slow press. Not offered: a
  passphrase field (M8).

### M8. Every passphrase is `faraday`; vaults, Lightning, checks

- **Files**: every field of §6.2's table, `Faraday::demo_passphrase`;
  `faraday/faraday-core/src/demo.rs` (`PASSPHRASE` = `"faraday"`,
  `PASSPHRASE_WORDS` gone); `vault/mod.rs` (Generate with dice puts
  `faraday`; `phrase_bits` gives `None` for it); the remaining rows of
  §6's table (the Vaults screen's sample vault, Lightning's cipher seed,
  whose row opens in Practice, Check a signed message and Check an address,
  the TOTP code); en.rs (`samples_password`, `vault_demo_uses_public`).
- **Tests**: in Practice each field of §6.2 shows `faraday`, takes no
  typing, and the flow goes on: New key with a passphrase adds the
  handout's `faraday` key, which the public set takes; Create a vault
  makes one that unlocks with `faraday`; Generate with dice shows the
  rolled words and the field takes `faraday`; the sample vault appears in Vaults and
  unlocks to the Load sheet; Lightning's cipher seed row fills LND's
  vector and shows its node id, a typed cipher seed that is not the
  vector is refused, and Show stays dimmed; the signed message checks
  as valid; the address is found in Spending. Outside Practice, on Testnet
  and on the stick: a short passphrase is taken as typed, masked and
  editable; two that differ are still refused; the dice's strength
  line is drawn.
- **Docs**: DEMO §5.2 (`faraday` in place of the rolled words), §6
  (the LND row; typing a password into an Entry: now `faraday`, no
  longer typed), §4 (the Lightning rows unchanged); the Learn page: "In
  Practice, every passphrase is faraday." once, Generate with dice's
  passphrase changed, "What Practice does not make" without the cipher
  seed; DESIGN §4.10's Mask policy (a Practice exception: sample secrets
  and `faraday` are drawn unmasked); DEMO's header and every remaining
  "mainnet" meaning Practice. DEMO is then Reference again with Practice as
  built; this file goes to `docs/history/` once W1 is done.
- **Done**: `just` passes; `bash tools/lint-docs.sh` passes.
- **Amended** (owner, 2026-10-22): fields are not fixed; each empty one
  offers a `faraday` chip (§6.2), and the KeePass export is typed.

### W1. The website

§9. In the website's repository; its HANDOFF.md is the contract. The
web shell sets `Faraday::online`, so §4's line shows at every
secret-typing step there too.

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

The website no longer carries the web demo; its web shell and demo page were taken out of the website repository.

### Item 9 of §2, as it read (from docs/MODES.md)

9. The website's mode is fixed (`Keys::fix_network` on Testnet with the
   demo set): the sheet lists the other three dimmed.
