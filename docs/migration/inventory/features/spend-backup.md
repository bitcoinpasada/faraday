# Spending from a backup

The Spend tab: a guided walk-through for spending from a backed-up wallet
when the wallet app is gone, from what the person has in hand (seed words,
a SeedQR, SLIP-39 or codex32 shares, multisig sheets, a vault, a wallet
file). It finds or rebuilds the wallet, guesses the wallet's shape from the
seeds, checks the money is there, brings in the PSBT and then runs the same
signing as Sign a transaction (see `spend-signing`). It is for a person
recovering or an heir. It is reached from the sidebar's Spend tab, the Tools
tile SpendBackup and Home. Code: `faraday/faraday-core/src/spend_backup/mod.rs`
(state, route, actions), `faraday/faraday-core/src/spend_backup/screen.rs`
(drawing), `faraday/faraday-core/src/spend_backup/text.rs` (wording).

## Screens and sheets

- `Screen::SpendBackup`; `Screen::Spend`, `Entry`, `Home` and
  `WalletsStart` appear as origins or returns. Sheets: `Sheet::Scan` (the
  intake scanner), `Sheet::CancelFlow(Going::Spend)`.
- Pages (`faraday/faraday-core/src/spend_backup/mod.rs:~68-89`): MAP, SAFE
  (no longer shown), HOLDING ("What do you have?"), OPEN (legacy, becomes
  HOLDING), CHECK, WRITE, BRING, AWAY, COUNT. Steps: TRANSACTION, TXID,
  PATH, NONCES, SIGNERS, SIGN, COLLECT, FINISH.
- With a wallet or seed loaded, an overview (`loaded` / `loaded_compact`);
  else the walk-through step cards through `flow::contained`.
- Cards:
  - MAP: six stops.
  - HOLDING: scan and type buttons, a vault section, copy from stick, the
    description question, the seed-shape question, restore or rebuild.
  - CHECK: receive and change address rows with QR, guess progress, "No
    money shows".
  - WRITE.
  - BRING: scan the PSBT, copy from stick, use a PSBT in Files, a sample
    PSBT in a "Demo only" card. The current spend's PSBT is marked
    "Signing this one"; a spend with no wallet open warns.
  - SIGNERS: who signs, Scan a key, unlock vaults, Collect.
  - AWAY: Lock, Power off, Start over.
- Vault section inside the tab: locked vault files, a passphrase field
  (`pass_field::screen::vault_box`), Unlock with a working bar and time
  estimate.
- `faraday/faraday-core/src/spend_backup/text.rs`: plain statements; More about this folded; the lead line
  depends on `app.online`; `signers_missing` decides SIGNERS' More; on the
  Paper route the one-seed line. The Lock line says the vault is sealed
  again with the keys inside it and none is left in memory.
The overview is drawn by `Screen::SpendBackup` (not `Screen::Spend`): `loaded_compact` on a compact panel, else `loaded`, when `spend_backup_overview()` holds (a key or a wallet is loaded, no walk-through chosen, no route taken) (faraday/faraday-core/src/spend_backup/screen.rs:27-45, faraday/faraday-core/src/spend_backup/mod.rs:310-312, 421-424).
- Helpers: `screens::spend::step_body` draws a spend step by number (0 wallet, 1 addresses, 2 review, 4 keys, 5 sign, 6 collect, 7 finish, 8 paths, 9 nonces) then the spend's error line (`faraday/faraday-core/src/screens/spend.rs:763-805`), called from `faraday/faraday-core/src/spend_backup/screen.rs:630`; `screens::vault_panel` draws each open vault's seeds and wallets with Select all and Load (`faraday/faraday-core/src/screens/files.rs:566`, called from `faraday/faraday-core/src/spend_backup/screen.rs:1107`).

## Actions

`SpendBackupAction` (`faraday/faraday-core/src/spend_backup/mod.rs:~113-183`,
29 variants, all handled in `spend_backup_act`, `~682-915`):
- `Card`, `Next` (on CHECK marks the wallet checked this power-on),
  `Unsure`, `Scan`, `Everything`, `NoDescription`, `StrayPass`, `NoMoney`,
  `GuessWay`, `Passphrase`, `Again`, `More`, `Pick`, `Unlock`, `TypeWords`
  (only if keys may load), `UseWallet` / `TakeFile`, `FromFile`,
  `FromSeedFile`, `Rebuild`, `Choose`, `UsePsbt`, `MoreAddresses` (+5, at
  most 47, `faraday/faraday-core/src/spend_backup/mod.rs:822-824`), `QrTx`,
  `SpendFrom`, `Walkthrough`, `SeedWallet`, `Done`, `StartOver` (asks first
  if steps are done).
- Entries outside the enum: `spend_backup_intake_seed`, `intake_key_part`,
  `spend_backup_intake_file`, `spend_backup_arrived`,
  `spend_backup_psbt_for_open_spend`, `spend_backup_guess_pass`.
- Actions drawn: the above plus `Action::{ScanFor(Ask::..),
  VisitFrom(Screen::SpendBackup), RestoreWallet, OpenWallet,
  Vault(VaultAction::*), ScanKey(wi), BackupFirst(wi), ShowCode(Code::Receive),
  QrWallet, LockAsk, PowerAsk, SamplePsbt}` (SamplePsbt in Demo).

## Effects

- Camera take, open, close and say; `Command::CameraOn/Off`.
- `open_qr(QrView::text)` for the finished transaction.
- `session.wallet_replay(Action::SpendBackup(..), inbox_print())`: the
  action is re-run after a confirmation sheet.
- `add_wallet`, key adds, `vaults.load_all`, `vault_act(Pick/Unlock)`.
- `signing::read::start_spend` (`faraday/faraday-core/src/spend_backup/mod.rs:~2126`)
  and `signing::collect::start_collect_on_spend_backup` (`~2232`): worker
  jobs of `spend-signing`.
- `flow_end(Going::Spend)`.
- No storage writes, clipboard, clock or randomness in this module; kept
  bytes are returned to a caller.
- Drawing writes the scroll offset
  (`faraday/faraday-core/src/spend_backup/screen.rs:~43`, `~116`, `~362`).

## State and lock

- `SpendBackupState` (`faraday/faraday-core/src/spend_backup/mod.rs:~245-293`):
  route, open, closed[8], help, more, addresses, wallet id, scroll, error,
  walkthrough, start_over_asked, from_card, `seeds: SeedsState` (typed seed
  words, cosigner text), from, came, plain_of, everything, no_description,
  guess.
- Secrets: the seed words themselves are in `session.keys`, wiped by
  `Session::wipe` (`faraday/faraday-core/src/wallet.rs:1539-1549`).
  `SeedsState` keeps fingerprints and cosigner text. The typed vault
  passphrase is `app.vaults.passphrase` (vault area).
- Lock (`AtLock for SpendBackupArea`,
  `faraday/faraday-core/src/spend_backup/mod.rs:2549-2555`): `holds` is
  `seeds.cosigner_typed()`; `lock()` replaces `state.seeds` with
  `SeedsState::default()`. Everything else in the state stays for the
  kept-state writer. The open spend is cleared by `WalletsArea::lock`
  (`faraday/faraday-core/src/wallet/state.rs:1264-1274`). A lock then
  restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Kept across a restart (`spend_backup_kept`,
  `faraday/faraday-core/src/spend_backup/mod.rs:~2266-2315`): route, open
  page, closed pages, wallet id, `from_card`, file names. Not kept: seeds,
  help, more, addresses, error, guess, plain_of, everything, no_description,
  the seed `came` entries. On restore (`~2318-2357`) a kept OPEN becomes
  HOLDING, pages past COUNT are dropped, and an 8-character-checksum form
  names no wallet.
- The chosen wallet is identified by its descriptor checksum; the
  typed-words wallet by its origin, not its name text.
- The drawing has no state of its own; it reads `app.spend_backup.state`,
  `app.wallets.spend()`, `app.vaults` (open, pick, passphrase, working,
  memory_free_mib, unlock_error), `app.stick`, `app.online`, `app.medium`,
  Inbox and Outbox.
- `SpendBackupState` (`faraday/faraday-core/src/spend_backup/mod.rs:245`) derives `Debug` but holds no seed. Seeds are named by fingerprint in `SeedsState.keys` and `ticked` (`faraday/faraday-core/src/seeds/mod.rs:100-104`). Typed boxes are `Text`, whose `Debug` prints only a length (`core/faraday-text/src/text.rs:383-387`).

## Reach

- Out: `wallet`, `create`, `scan_key`, `vault`, `seeds`,
  `held::wallet_held_on`, `glance`, `signing`, `restore::{merge,
  name_or_sheets}`, `seed_check::take_part`, `stick::keys_load`, `forms`,
  `in_progress`, `trail`, `flow`, `at_lock`.
- Exports: `spend_button`, `psbt_for`, `spend_backup_*`, `guess_*`,
  `kind_name`, `SINGLE`, `MULTI_ORDER`.
- Opens: `spend-signing` (the spend), the scanner, Add a key (Type words),
  vault unlock, Files (`VisitFrom`), Restore, a wallet, the receive code,
  the Lock and Power off sheets.
- Opened from: `tools` (SpendBackup tile), the sidebar, Home (Home's scan
  takes a PSBT only for an open spend).

## Rules and refusals

Intake scanner:
- Not a seed: `spend_backup_not_a_seedqr`; the camera stays.
- A seed that is a missing key of the described wallet is added with an
  empty passphrase; already a key: "key already"; not a key of the wallet:
  `seed_read_for_wallet` with the reason.
- SLIP-39 and codex32 parts only if they parse; mixed-case codex32 is
  refused; parts are not loadable when keys may not load (`keys_load`
  text).
- A wallet file is taken only when no description is loaded. Split sheets
  are merged; rebuilding needs them `whole`. The sheets' name is kept on
  every path.

Question and auto-open:
- `spend_backup_question` (`faraday/faraday-core/src/spend_backup/mod.rs:~1176-1191`):
  None if a wallet is ready or described, or several wallets came from a
  vault; Anything for one seed; Description for several; Shape after
  "can't find".
- Auto-open (`~923-967`): only with no wallet loaded, never on the paper
  route, the vault route only when every open vault's keys are seeds, and
  only one wallet file.

Guess (`~1421-1540`; way table at
`faraday/faraday-core/src/spend_backup/mod.rs:1462-1474`):
- Multi kinds in `MULTI_ORDER`; every multisig kind is tried once.
- Single: way 0 NativeSegwit, way 1 NativeSegwit with passphrase, 2
  Taproot, 3 Nested, 4 Legacy (the passphrase also on later ways when
  `g.pass`). No money after way 0 asks for a passphrase; an empty
  passphrase goes on to way 2
  (`faraday/faraday-core/src/spend_backup/mod.rs:1611-1620`); the last way
  sets `none`.
- Each wallet with no money is listed once.
- A guess holding a cosigner's key that no loaded key gives keeps its guess
  until the hold and continues from it; a guess of only typed seeds asks
  nothing.

Pages:
- Pages past HOLDING only when a wallet is ready (except AWAY). Ready is
  `held_toward_quorum >= needed`; not ready opens the wallet instead of
  spending.
- CHECK is done only when the wallet is checked this power-on. A wallet
  change re-opens the money check.
- WRITE and BRING are done only with a spend; a spend open with no spend
  moves to Bring or HOLDING.
- Choosing another wallet resets CHECK and WRITE closed flags and the
  address count.

PSBT arrival:
- The same txid is collected into the spend.
- Another PSBT while a spend is under way (not finished, with an open step
  card, signatures or collected parts): a toast, refused.
- A spend for another wallet (including a two-account Coldcard export)
  asks the cancel sheet first.
- A seed-words file is refused unless keys may load and its kind is Words.
- An unconfirmed wallet (a cosigner key no loaded key gives) is held for
  its sheet, and the way replays.

Drawn rules:
- Type words is disabled without `may_load_keys()`.
- Copy from stick only when not ready; `stick_row` is disabled with no
  stick, a vault open or a secret held.
- Lock beside the stick button only with a stick present and something
  open.
- Vault section: a stick banner when `!may_load_keys()`; Unlock is Primary
  only when `may_load_keys()`.
- "Show more addresses" only while `n < 50`
  (`faraday/faraday-core/src/spend_backup/screen.rs:1635`).
- Held address rows are Warning and not openable; change rows never open a
  code.
- Rebuild only if not ready and the sheets merge whole.
- Signers: "Finish backup" for a held key waiting for backup, a
  passphrase-key button, load from vault, "Signs here" / "Not here"; short
  when `here < needed`, or paths differ and not complete (`Kind::Miniscript
  | Tree`).
- Check draws nothing for a wallet awaiting the published-words caution.

## Domain calls

- `faraday_bip::{policy::WalletId, multisig_config::parse_named,
  slip39::Share::parse, codex32::Codex32::parse, keys::Fingerprint}`
  (`core/faraday-bip/src/`).
- `faraday_keys::{KeyId, KeyInfo}` (`faraday/faraday-keys/src/`).
- `faraday_crypto::work::Compute`.

## Fixtures

- The Demo spends `faraday/faraday-samples/data/{savings, spending,
  taproot, nested, legacy, vault-nested, vault-legacy, taproot-multisig,
  inheritance, tree, musig}.psbt`. The "Demo only" `SamplePsbt` press runs
  `sample_psbt_use` (`faraday/faraday-core/src/samples/put.rs:417-442`),
  which scans the loaded sample wallet's spend (`sample_spend`), as an
  intake scan when the tab is on HOLDING, else as a Files scan.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| fde37a810 | The Lock line in Spend said the keys are forgotten | The vault is sealed again with the keys inside it; none left in memory |
| 5ccb6626b | A guess holding a cosigner's key no loaded key gives did not keep its guess before the hold or continue from it | The guess is kept until the hold and continues; a guess of only typed seeds asks nothing |
| 6d057e83b | Choosing another wallet, or its transaction, kept the money-check answer of the first | A wallet change re-opens the money check |
| d672a4def | Of two wallets with one name, the wrong one could show | The chosen wallet is identified by its descriptor checksum |
| c829da944 | A wallet file named "Typed words" was dropped with the typed-words wallet | The typed-words wallet is found by its origin |
| b5c8fe935 | A Taproot multisig rebuilt from sheets showed "Restored wallet" | The sheets' name is kept on every path |
| fa6f2f592 | A way answering No money twice, after the next was cancelled, was listed twice | Each wallet with no money is listed once |
| 1da1e56aa | A guess from seeds shaped as one multisig kind did not go on to the others | Every multisig kind is tried once |
| 4e38374d4 | Spend from backup on a wallet file dropped the spend under way before the file was accepted, and the flow started again for another wallet kept every file that had come in. | The spend stays until the file is accepted (a refusal or a cancelled sheet leaves it); the flow started again for another wallet keeps only the file just taken as what came in (`faraday/faraday-core/src/spend_backup/mod.rs:1970-1978,2009-2035`). |

## Doubts

- `spend_backup_set_wallet(i)` indexes `session.wallets[i]` unchecked
  (`faraday/faraday-core/src/spend_backup/mod.rs:~1008`).
- `STEP_AT = 16` and `LATER_AT = 32` card numbering assumes fewer than 16
  steps.
- The drawing does work on every frame (`restore::merge`,
  `create::read_key`, `from_utf8_lossy`, `receive_line` for up to 3 plus
  the address rows per side, `spend_backup_guess()` several times,
  `wallet_rows` quorum counts); whether these are cached elsewhere is
  unclear.
- `faraday/faraday-core/src/samples/put.rs:417-442` was read from the code
  at `d89dde5de`.
