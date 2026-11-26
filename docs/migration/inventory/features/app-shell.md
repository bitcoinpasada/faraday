# The app and its event loop

The `Faraday` app value and its `App` implementation: it takes every event the shell sends (press, key, scroll, wheel, tick, camera frame, scanned code, clipboard text, display change), routes every press through one dispatcher, and draws every screen. It serves every product (the device app, Faraday Demo, the Starter's Demo) and is reached by every other feature. It does no input or output of its own: files go through the storage queue, everything else through shell commands.

## Screens and sheets

- `Screen` (`faraday/faraday-core/src/lib.rs:~335-425`): Home, Check, WalletsStart, Wallets, Spend, Files, Entry (Add a key), Backup, Message, CheckMessage, Create, Restore, Settings, SetTime, Randomness, Vaults, Backups, CreateVault, KeyGen, Bip85, Convert, KefTool, OtsCheck, SeedCheck, MultisigCheck, CoreWallet, Silent, Explore, Lightning, Tools, Unlock, VaultContents, Codes, SpendBackup, Vanity, Decode, Invoice, CheckAddress, NonceCheck, SignerCheck, Catalog, BackupFlow; `Transfer` only with feature `demo` (`faraday/faraday-core/src/lib.rs:~420-421`).
- `Sheet` (`faraday/faraday-core/src/lib.rs:~504-663`), by purpose:
  - Lock and idle: Lock, LockAsk, Locked, Power, IdleWarn, WriteOut, SecretOut.
  - Stick and files: Import, Pull, Taken, NameTaken, Delete, Zeroing, OutboxDelete, VaultSave, FileMenu, Rename, Details, Choices, NavMenu, PowerMenu, StartMenu, Devices, NewInput, Samples, Mode.
  - QR and camera: Qr, Scan, SeedRead, KefRead, TextSeed.
  - Guides and lists: Guide, WordList.
  - Keys and seeds: Potential, Sweep, KeyFile, ChildRecord, ChildNotice, KefOptions, OtherForms, KeyLengths, Published, PassphraseSeeds, Account, Register, WalletConfirm, MadeWallet, Chart, GpgTile, GpgSign, GpgRestore, Trim, SaveKeeps, SaveAlso, CopyTo, OlderCopy, UnlockLoad, VaultPassphrase, VaultMemory, SignerSeed.
  - Backup board: BoardList, BoardSees, BoardPartials, BoardMenu, FigMenu, LaterAsk, StickSheet, QuickTemplate, CancelFlow, CancelKey, CheckBackups, CheckBackupsOther, AddressFiles.
- Failed start-up self-test: the whole screen names the failed check, with Exit its only control (`faraday/faraday-core/src/lib.rs:~1050-1054`; drawn by the screen frame).

## Actions

- Every press is an `Action` (`faraday/faraday-core/src/action.rs:~23-1129`), a plain `Copy` value. Helpers: `KeyTool` (`:~13`), `ChoicesPress` (`:~1133`), `MarkTip` (`:~1144`), `is_hold` (`faraday/faraday-core/src/action.rs:1152-1176`, the hold rule the pointer uses), `offers_restart` (`:~1193`, offered when `!shell_sticks`), `screen_name` (`:~1199`).
- Entry points: `Faraday::act`, `act_noted`, `act_dealt`, `act_inner`, `cancel_act` (`faraday/faraday-core/src/action.rs:~1206, ~1216, ~1275, ~1402, ~1330`).
- Dispatch order in `act_inner` (verified):
  1. Self-test failed: every press dropped except PowerOff (`faraday/faraday-core/src/action.rs:1405-1410`); PowerOff runs `exit_forgetting`.
  2. `demo_refuses`: one toast `practice_refused` (`faraday/faraday-core/src/action.rs:1411-1416`).
  3. Stick-bar press, Lock-sheet Scan; the Chart sheet closes on any press except Chart, Osk, TypeHere(Chart), KeyTick, PassEye (`faraday/faraday-core/src/action.rs:1430-1442`).
  4. `ScanFor` over the Lock sheet ends the read card (`faraday/faraday-core/src/action.rs:1445-1449`).
  5. `device_holds`: `Sheet::Pull` if the stick must be pulled and nothing is held, else `device_hold` (`faraday/faraday-core/src/action.rs:1450-1466`).
  6. A key-loading press with a stick in the way opens `Sheet::Pull` (`faraday/faraday-core/src/action.rs:1467-1476`).
  7. The big match (`faraday/faraday-core/src/action.rs:1477-2033`), no catch-all arm.
- Routes, by area: wallet_act, inbox_act, visit_act, power_act (Lock, LockUnsealed, LockAsk, NotNow, PowerAsk, PowerOff, Restart, HoldPowerOff, HoldRestart, IdleLock/IdleOff minutes, 0 = never), devices_act, restore_act, register_act, memory_press (SealAmounts, NonceHistory), time_act, codes_act, chart_act/glance, backup_act/backups_act, board_press/choices_act, create_act, message_act, keygen_act/keygen_codex32, own_press (Add randomness), bip85_act, silent_act, explore_act, lightning_act, tools_act, tool_act (Convert, KefTool, SeedCheck), ots_check, multisig_check, core_wallet::tool, catalog_go (`faraday/faraday-core/src/action.rs:1944`), check::press, decode_act, invoice::act, address_check::act, seeds_act, seed_read_act, kef_act/kef_file, sweep_act, wipe_act (DeleteFiles, Unprotected, What, Pick, Passes, Go, ZeroStop), secret_act, scan_act, qr_act (PDF presses online only), appearance_act (Scale, Rotation, TextSize, ReduceMotion, Motion, Speed, Theme), guide_press/guide_word/guide_shares, wordlist, entry_act, osk_press, nav/panes_nav, row_open, file_menu, vault_act, vanity_act, spend_backup_act; Transfer presses only with `demo`; Practice PassChip.
- Hold-only presses (act on hold): Vault HoldDelete, HoldWipe, SbHoldSign, SbEnrol, GpgHoldSign, GpgHoldRevoke; FlowAction::Hold; HoldPowerOff; HoldRestart; RemoveKeysHold; SecretHold; WalletHold; PublishedHold; OutboxDeleteHold; Sweep Hold; RegisterHold; ReviewHold; DevicesHold; NameTakenOverwrite.
- Event handling (`faraday/faraday-core/src/lib.rs:~1127-1456`):
  - While `Sheet::IdleWarn` is up, any key, Back, Forward, scroll or wheel only puts the warning away; touch Down wakes and is not dispatched; other touch phases are dropped (`faraday/faraday-core/src/lib.rs:~1191-1268`).
  - A slow key press is drawn first with a "working" line chosen by `seed_check::here`, and done on the next tick (`Later`) (`faraday/faraday-core/src/lib.rs:1211`, `~1205-1219`, `~1113-1122`).
  - Wheel: shells send 48 px a notch (`motion::SHELL_NOTCH_PX`), converted to the layout's notch of 56 units (`faraday/faraday-core/src/lib.rs:1271-1274`).
  - Hover redraws only when the hover target changes (`:~1282-1305`); a scroll with no scroll-end is ended after `SCROLL_QUIET_MS` (`:~1337-1341`).
  - Camera frames go to `camera.frame_in`, or to `noise_frame` when `noise_camera_on()` (both in `faraday/faraday-core/src/fresh.rs:875-880`). A scanned code runs `scanned(bytes)` then `release_held_stick` (`faraday/faraday-core/src/lib.rs:~1423-1426`). Camera failures map to NoPicture reasons (`:~1428-1432`). Clipboard text, or empty when unavailable (`:~1433-1434`).
  - Typed characters in seed or passphrase entry reset the typed-at clock (`:~1155-1157`); `pass_field::settle/before_key/after_key/left` (`:~1179, ~1449, ~1453`).
  - Tick: live clock redrawn when the second changes; toast cleared after its time; calls `kef_tool::tick`, `keygen.dice_tick`, `address_check::tick` (`faraday/faraday-core/src/lib.rs:1343-1344, 1378`).
  - `poll_command` calls `note_secrets()` before popping the next command (`:~1462-1465`). `frame` renders only moved rows when allowed and runs a fade step when due (`:~1467-1485`).

## Effects

- Files: through `Faraday::storage` and `Faraday::poll_storage` (`faraday/faraday-core/src/storage_api.rs:1065-1084`).
- `StorageCommand::ForgetBoxes`; `OpenInstitute` (online only); `Command::Exit`; `CameraOff` on a scan Cancel.
- `faraday_selftest::run` for `SelfTestRun` (toast only; result kept in `self.selftest`).
- `exit_forgetting` clears the import, Inbox secrets, the session, own randomness, sends ForgetBoxes, then Exit (`faraday/faraday-core/src/action.rs:1317-1326`); also reached from the PowerAsk fast path.
- Exit codes: 75 = fresh process (lock or restart), 76 = restart the computer (`faraday/faraday-core/src/lib.rs:~328, ~332`).

## State and lock

- All state lives on `Faraday` (`faraday/faraday-core/src/lib.rs:~897-1102`), one process long. The action layer keeps none of its own.
- Areas that hold secrets, by their doc comments: `entry`, `keygen`, `secrets`, `seed_read`, `fresh`, `vaults`, `flows`, `decode`, `import`, `backup`, `create`, `restore`, `message`, `spend_backup`, `vanity`, `bip85`, `silent`, `lightning`, `explore`, `tools`, `glance`, `camera`. Each one's lock is in its own feature file.
- `tainted: bool` (`faraday/faraday-core/src/lib.rs:~924`): set once a secret has been held or typed in this process; only a fresh process is clean.
- Kept across a lock: `memory` (signed amounts, vault-sealed flags, wallets checked), `backups_area` (gone at power-off), `settings` (`faraday/faraday-core/src/lib.rs:~917, ~939, ~943`).
- Lock: `lock_areas` runs `AtLock::lock` on each listed area (`faraday/faraday-core/src/at_lock.rs:145-153`, areas listed at `faraday/faraday-core/src/at_lock.rs:175-191`). Then `lock_finish` saves the boxes, sets `restart = true` and sends `Command::Exit` (`faraday/faraday-core/src/vault/seal.rs:933-946`). Every lock ends in a fresh process, so state marked `at_lock::Public` does not outlive it.
- Types marked "holds nothing a lock wipes" (`at_lock::Public`): Medium, Screen, Sheet, Disclosed, Later, Pointer, Scroll, Out, SetTimeArea.
- Idle timers: lock after 30 min (`IDLE_LOCK_MIN`), power off after 60 min (`IDLE_OFF_MIN`), warning 5 min before, or half the lock time when that is shorter (`IDLE_WARN_MS`) (`faraday/faraday-core/src/lib.rs:319-325`). These are first values; Settings changes both (`faraday/faraday-core/src/state.rs:2558-2563`).
- `Faraday::new()` (`faraday/faraday-core/src/state.rs:~2678-2764`): Home, 1280x800, scale 1.0, `online` false, `tainted` false, `Medium::Stick`, Secure Boot Unknown.
- Dead field: `disclosed: Option<Disclosed>` is documented "always none now" and is only ever set to `None` (`faraday/faraday-core/src/lib.rs:1046`, `faraday/faraday-core/src/state.rs:2744`).

## Reach

- Opens every feature through its routes (above).
- Reads lock (`lock`), the self-test, the stick and storage, the camera and scanner, randomness (`randomness`), the typing caret (`typing`), motion (`motion`), the screen frame (`screen-frame`), the small panel (`small-panel`).
- `pane::scanning` from the address check is read at `faraday/faraday-core/src/action.rs:1236`; `pane::left` at `faraday/faraday-core/src/action.rs:1246`.
- `demo_refuses` and `device_holds` are called from `act_inner` (`faraday/faraday-core/src/action.rs:1411, 1453`).

## Rules and refusals

- Failed self-test drops every press but PowerOff (`faraday/faraday-core/src/action.rs:1405-1410`).
- `OpenInstitute` acts only when `self.online` (`faraday/faraday-core/src/action.rs:1696-1701`).
- `KeyTool(tool, fp)` does nothing unless `session.keys.position_of(fp)` is some; with a key it opens BIP-85, Silent payments or Lightning for that key. No note is shown otherwise (`faraday/faraday-core/src/action.rs:1590-1600`).
- The Chart sheet closes on any press but its exception list (`faraday/faraday-core/src/action.rs:1430-1442`).
- `act_noted`: "Lock and write" waits only on Lock, LockAsk, WriteOut; a mode change waits only on LockAsk (`faraday/faraday-core/src/action.rs:~1251-1272`). On the Files tab with details open, a press only closes them (`:~1237-1243`).
- Cancel clears: lock_to_write, read-again, vault rekey, pull, stick save, devices, KEF tool, KEF job, seed_read, delete-ask, taken files, `next_mode` unless restarting or sealing, secret_cancel, word list, potential, camera (keeps parts), chart, glance work, backup pick, input sheet, import resume (`faraday/faraday-core/src/action.rs:~1363-1396`). A Cancel reaching `act_inner` that is not `Action::Cancel` is `unreachable!()` (`:~1397-1398`).
- Cancel per sheet: BoardMenu -> MenuClose; FileMenu, Rename, Details -> file Close; UnlockLoad -> LoadNotNow; WalletConfirm -> WalletCancel (drops the waiting wallet only); GpgSign and OtherForms close; KefRead -> kef_cancel; NameTaken -> name_taken_cancel; TextSeed -> Leave; PassphraseSeeds -> pass_seeds_close (`faraday/faraday-core/src/action.rs:~1333-1362`).
- Back closes Choices, OtherForms and StartMenu first (`faraday/faraday-core/src/action.rs:~1667-1674`).
- Nav to Vaults with no vault in Files and no sample makes "make a vault" the only choice (`:~1734-1739`). Nav to SpendBackup records where it was opened from (`:~1710-1713`).
- `Host` and `Mark` are no-ops; `Tab` is converted in `act`; `StickPill` is handled in `stick_bar_press`.
- Shell flags (`faraday/faraday-core/src/lib.rs:~1055-1071`): `online` (sheets become PDFs), `held` (only Transfer and the guide sheet), `shell_sticks` (never on the device), `clipboard` (Paste dimmed when unset; never on the device).
- `spend_opens_round` (`faraday/faraday-core/src/lib.rs:~1491-1493`): any input with `musig` set counts as opening a MuSig2 round; the decision is made only when the pass signs.
- Feature gates: `transfer` screen and field only with `demo` (`faraday/faraday-core/src/lib.rs:~252-253, ~310-311, ~420-421, ~1029-1030`); `leaks` and `testkit` only with `testkit` (`:~257-268`).
- Stick file kinds:
  - `seed_by_name`: name ends `words.txt` or `seedqr.png`, or starts `bip85-`, case-folded (`faraday/faraday-core/src/lib.rs:448-451`).
  - `stick_kind`: the settings file gives none; the extension picks the label (psbt, txt/json/descriptor = text, txn, faraday-sheet, kdbx = vault, efi, png = QR codes, else file).
  - `name_may_hold_key` (F130): refuses psbt, txn, faraday-sheet, kdbx, efi, descriptor, asc, sig (any case) and the settings file; no bytes read (`faraday/faraday-core/src/lib.rs:483-500`).
- `qr_fits` for Outbox files: a vault never; PSBT, Wallet, Key, Message, Share always; others only when non-empty and at most `faraday_qr::envelope::MAX_FILE` (256 KiB) (`faraday/faraday-core/src/lib.rs:672-680`).
- One-line key forms (`faraday/faraday-core/src/one_line.rs:77-94`):
  - Trimmed text that is empty or has more than one word is not a key form.
  - A line `secret_text` reads as an extended private key gives `Form::Xprv` (any version; the refusals come when the form is opened).
  - A line `parse_hex_seed` accepts gives `Form::HexSeed`.
  - `xprv_refusal` names: Base58, length (with count), unknown version, zero depth, padding, key range, SLIP-132 version bytes, wrong network, not a master key, origin unreadable, path, or differs from its own fingerprint (`:~12-61`).
  - `hex_refusal`: not hex, odd digits, wrong length (`:~64-70`).
- Wallet share records: vault record kind `WALLET_SHARE`, field `SHARE_WALLET` names the wallet, share number from `SHARE_NUMBER` (`faraday/faraday-core/src/lib.rs:~838-863`). `WalletKey` holds public text only (`:~682-698`).
- Card and plan indexes: `cstep`, `rstep` (`:~746-761`), `qstep` (`:~776-786`, `MAX_PLACES` 240), `BStage`, `PlanPress`, `plan_cards` (`:~791-831`), `VaultAsk` (`:~712-730`).
- Small shared pieces:
  - ⋮ menu (`faraday/faraday-core/src/menu.rs`): a popup under its ⋮ on wide layouts, a sheet on the small panel. Down and Up move (wrapping, nothing on an empty list); Enter and Space choose; Escape closes; other keys are taken and ignored (`:~73-81`). Tab and Shift+Tab move to the next or previous ⋮ (`:~85-98`). `measure` registers no presses (`:~143-153`).
  - Medium names (`faraday/faraday-core/src/medium.rs`): stick versus "card or stick" wording; `SdCard` never says "stick" where the Pi starts (`:~215-216`).
  - Starter install names (`faraday/faraday-core/src/names.rs`): Linux bin, menu ids, Windows names, plus `LEGACY` and `LEGACY_DESKTOP` for find-and-remove only. Read by `faraday/shells/starter/build.rs` and `faraday/shells/demo/build.rs`.
  - Picture writer (`faraday/faraday-core/src/picture.rs`): `labelled_png` draws the QR black on white with title and label lines, 640 px wide, height clamped to the code's side and 1600 px (`MAX_HEIGHT`, `:~52-54, ~76-89`). Lines past the bottom are dropped with no note (`:~126-129`). `picture_codes`: one code if the text fits version 25 at medium error correction, else BBQr parts at `QR_PARTS[1]`; a BBQr failure gives an empty list (`:~229-235`). Counted as `Work::Picture`.

## Domain calls

- `core/faraday-selftest`: `faraday_selftest::run`.
- `core/faraday-crypto` (`Compute`), `faraday/faraday-intake`, `core/faraday-text`, `faraday/faraday-plan`, `faraday/faraday-qr` (`envelope::MAX_FILE`), `faraday/faraday-keys` (`RecordView`), `core/faraday-bip` (`WalletId`, `WalletPolicy`), `faraday/faraday-vault` records, `core/faraday-psbt` (`Inspection`), `core/faraday-shell-api`, `core/faraday-draw`, `faraday/faraday-strings`.
- `faraday_ui::menu` for the ⋮ menu.

## Fixtures

None.

## Fixes

None.

## Doubts

- Where the shell reads exit codes 75 and 76 is not in the code the inventory read for this feature.
- `Faraday::new` sets 1280x800 at scale 1.0; whether the first Display event always overrides it is not known.
- The screens and sheets differ per product: `Screen::Transfer` and `ScanPurpose::Transfer` exist only with `demo`; testkit items only with `testkit`. No list of which screens exist on the device, in Demo and in the Starter was made.
- `picture::draw` returns its canvas; no caller was checked.
- The board tokens `BOARD_MENU_DOT` and `BOARD_MARK` are used for every ⋮ menu (`faraday/faraday-core/src/menu.rs:~100-107`), a possible naming leftover.
- Cosmetic doc defects named but not re-read: `faraday/faraday-core/src/action.rs:~47-48, ~794-796`.
- Per-area lock behaviour of the areas listed under State was not checked here; see each area's file and `lock`.
