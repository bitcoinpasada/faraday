# Importing from the boot stick

On the first look at a boot stick, the app lists the stick's files (name,
size, a tick each, all ticked) and reads nothing. Later reads nothing and goes
Home with the list kept; Import reads only the ticked files and brings their
wallets, keys and files in through the Inbox loader, and their vaults into
Files, locked, each offered its Unlock with Cancel. It is for a person who
carried files on the boot stick itself. It is reached automatically when the
boot stick is seen at start; its list stands on the Files page in place of the
stick card. Picture files on the stick are read for QR codes as the camera
would read them, and a picture whose text could be a CompactSeedQR asks before
it loads. Decisions: the boot stick import (F60) and F130.

## Screens and sheets

- The list step: a tile on `Screen::Files` (`faraday/faraday-core/src/boot_import/screen.rs:~59`, `BootImportArea::tile`, `faraday/faraday-core/src/boot_import/mod.rs:~319`). It replaces the stick card while `import_tile_up()` (`faraday/faraday-core/src/screens/visit.rs:~179-184, 222`). It reuses the Files page's `stick_heading`, `visit_list_bar` and `delete_files` (`faraday/faraday-core/src/boot_import/screen.rs:114, 190, 201`).
  - Each file: name, size, a tick; a file Faraday does not read is listed NotRead without a tick.
  - The one button reads "Import and load any keys" when a ticked file may hold a key.
  - Pulling the stick before Import: the sheet asks for it back (`faraday/faraday-core/src/boot_import/screen.rs:~141-142`).
- `Sheet::Import`: the vault passphrase step, over Home only (`faraday/faraday-core/src/boot_import/screen.rs:~33`, `faraday/faraday-core/src/boot_import/mod.rs:~1557`). Per imported vault: an Unlock sheet (`faraday/faraday-core/src/boot_import/screen.rs:~260-323`), then the vault's Load sheet (the vault load offer, F62) via `load_sheet_offer` (`faraday/faraday-core/src/boot_import/mod.rs:~1610`). The unlock cost is shown by `vaults.time_text(memory, passes)` (`faraday/faraday-core/src/boot_import/screen.rs:~386-398`).
- `Sheet::TextSeed` (`faraday/faraday-core/src/boot_import/text_ask.rs`): a picture code of 16 or 32 bytes that also reads as a CompactSeedQR asks once per code, Load (as a key) or Leave (as a file) (`faraday/faraday-core/src/boot_import/text_ask.rs:~18-23, 61-85`).
- `Sheet::NewInput`, `Sheet::Devices` and `Sheet::IdleWarn` can cover the import (`faraday/faraday-core/src/boot_import/mod.rs:~1584-1590`). The device check remembers the Import sheet it covered and gives it back (`DeviceCheck::over_import`).

## Actions

`ImportAction` (`faraday/faraday-core/src/boot_import/mod.rs:~33-52`):
- `Open` (`~1167`): another stick came in; the import is dropped and Files opens (`~1172-1186`).
- `Later` (`~1199`): leaves Files to Home; the list is kept.
- `File(id)`: toggles a tick, enabled files only (`~1210`).
- `All` (`~1223`): ticks all; unticks all if any is ticked.
- `Go` (`~1274`): Import.
- `Submit` (`~1241`): unlocks the offered vault (`VaultAction::Unlock`).
- `Cancel` (`~1251`): the vault stays locked in Files.
- `TextSeedAnswer::Load | Leave` on the text-seed sheet.

## Effects

- Stick reads, queued in `out.storage_out` only on Import: `StorageCommand::ReadQr` for `.png`, else `Read` (`faraday/faraday-core/src/boot_import/mod.rs:~1297-1301`). Answers `StorageEvent::Read`, `ReadFailed`, `QrRead` are routed by `import_event` (`~818-848`).
- Inbox writes: items pushed to `self.inbox` (`~1381, 1419-1423`), then `save_boxes()` (`~468, 523, 1424`); SeedQR words copied in (`~519-523, 565-579`).
- Camera swap and scan to read picture codes (`~433, 457, 892, 922`): QR codes in a picture are read as the camera does, for a stick visit (`qr_read`, `~424-477`) and for an Import-asked picture (`import_codes`, `~879-963`).
- Visit log and toasts (`~427, 471-474, 525, 580, 600, 1507`); `visit.unprotected_add` for each secret item left on the stick (`~1416-1418`).
- Key loads: `load_found_in` (`~1411`; defined `faraday/faraday-core/src/inbox.rs:1302`), `import_load_after_pull` (`~1513-1529`).
- Vaults: `vault_read_from` (`~1379`), `vaults.read_free()` (`~1279`), `vault_act(Unlock)` (`~1249`).
- `refresh_spend`, `wallets.set_wallet(0)` (`~1425-1426`); the screen moves to Files or Home.
- No randomness, clipboard, printing or network.
- In Faraday Demo, the boot stick, once plugged in at start, has its settings file read and its other files copied to memory (`faraday/shells/demo/src/lib.rs:~144-146, 360-366`, through `stick_info_after(..., name == BOOT)`).

## State and lock

- `BootImportArea` (`faraday/faraday-core/src/boot_import/mod.rs:~308-325`, field `import`):
  - `state: Option<ImportState>`;
  - `load_after: BTreeSet<Text>`: files imported with the stick in, whose seeds load on the pull;
  - `under_input`, `tile`, `resume`;
  - `text: TextAsks` (`queue: VecDeque<Ask>`, `back`).
- `ImportState` (`~95-122`): label, stick id, `files: Vec<StagedFile>` (no bytes), `seen`, `asked`, `items: Vec<Item>` (the read bytes, secret, wiped as each `Item` drops), `skip`, `unlocking`, `wallets_before`.
- `Ask.code: Zeroizing<Vec<u8>>`: a picture code waiting for its answer (secret, wiped on drop).
- Lock (`faraday/faraday-core/src/boot_import/mod.rs:1632-1638`): `AtLock::lock` is `clear_all()`, which drops the state with its items, `load_after` and the text queue (`text_forget`, `faraday/faraday-core/src/boot_import/text_ask.rs:~54-57`). `holds()` is `Holds::session(state.is_some())`, a session hold, not a typed one.
- `clear()` (`faraday/faraday-core/src/boot_import/mod.rs:360-364`) drops the state, `tile` and `resume` but keeps `load_after`. `import_next` clears on the last vault (`~1540`). `import_stick_gone` marks reads Failed and keeps the state (`~977-981`).

## Reach

- Uses: `stick::visit_taken::free_name` (`faraday/faraday-core/src/stick/visit_taken.rs:95`), `stick::settings::is_file` (`faraday/faraday-core/src/stick/settings.rs:41`), `vault::{Focus, VaultAction}`, `vault::screen::{stick_banner, stick_field}`, `vault::file_text`, `wallet::FileKind`, `scan_read::{is_compact_text, read_code}`, `secrets::Exposure`, `pass_field::screen::vault_box`, `demo::PassField::Unlock`, `screens::{kind_name, kind_line, size_text, never_real_seed_if, never_under, working_bar, visit::{stick_heading, visit_list_bar, delete_files}}`, `file_menu::dots`, `compact::{dialog, scroll_sheet, sheet_head, buttons}`, `idle.under()`, `devices.over_import`, `visit`, `camera`, `inbox`, `session.keys`, `session.wallets`, `out.storage_out`, `medium`.
- Reached by: `Faraday::{import_event, import_go, import_act, import_view, import_count, import_reading, import_may_go, import_may_load_keys, import_tile_up, import_on_show, import_stick_gone, import_load_after_pull, import_unlocked, import_after_unlock, import_resume}`; `text_seed_ask` and `text_seed_answer` from the scanning code; `qr_read`; `picture_holds`.
- Opens: Files (`files-inbox`), the vault Unlock and Load sheets, Home.
- The file menu (`files-inbox`) refuses rename and delete of a file the import reads.

## Rules and refusals

- `Go` (Import) is refused unless the stick is in, `clean_now`, not asked yet, the listing is held and still the stick's, and some enabled file is ticked (`import_may_go`, `faraday/faraday-core/src/boot_import/mod.rs:1073-1087`).
- A file is enabled only while Import is not pressed and it is Listed; ticks apply only to enabled files (`~1151, 1213`). A tick names its file, not its place.
- A file of a kind Faraday does not read is listed NotRead, not enabled (`~636-640`). The settings file is never listed (`~628, 241`).
- Only the listing the import knows is followed: a listed file gone or at another size leaves the list; new files never join (`~258-269, 662-676`). FAT names match regardless of case (`~275-282, 694`).
- The stick out with a differing listing ends the import (`~708-719, 760-763`). A non-boot stick never keeps Home (`~746-748`). The tile is not up while a secret is held (`~786`).
- While the list is up, In Faraday's controls, Delete files, zeroing and renames are drawn as with no stick and refused; any other stick does not take the list down (history fix 0bab55cbf).
- A vault file too large for free memory is not read: `Staged::Short` with the reason from `vaults.read_refusal` (`faraday/faraday-core/src/boot_import/mod.rs:1288-1292`). The Import end says what memory is needed (`~1336-1341`).
- `Submit` is refused unless a vault is offered and `may_load_keys` (`faraday/faraday-core/src/boot_import/mod.rs:1241-1250`). The passphrase field is offered only when keys may load; `stick_field` is shown instead while the stick is in (`~996, 329, 342`; `faraday/faraday-core/src/boot_import/screen.rs:~341-342`).
- Text of 16 or 32 bytes that reads as a CompactSeedQR never loads at once; each code asks Load or Leave (`~510-514, 895-898`). Leave loads nothing (`~556-559, 598-600`).
- A SeedQR becomes a `<stem>-words.txt` item; the digits are wiped after the read (`~519-523, 900-908`). Copied seed words wait for the pull while the stick is in; otherwise they go to `load_after` (`~571-578`).
- Keys load only while `may_load_keys`: `load_found_in` gates key loading on `may_load_keys()` (`faraday/faraday-core/src/inbox.rs:1317`); wallets still load; `faraday/faraday-core/src/boot_import/mod.rs:~1411-1415` then holds the names for the pull.
- A same name with the same bytes already in Files replaces a vault entry; different bytes are renamed to a free name (`~1370-1377`). A plain file with the same name and bytes is read but not kept twice (`~1382-1384, 1422`). Plain items are renamed to a free name and a toast names them (`~1386-1393, 1497-1506`).
- Items of secret exposure left on the stick are noted as unprotected (`~1350-1355, 1416-1418`).
- After an unlock, the Load sheet comes first only when the vault has wallets or keys (`~1605-1613`).
- A picture code that is not a PSBT is refused with a "not a PSBT" note (`~915-916`).
- The boot import takes codes only from the stick it imports; a picture's codes name their stick (history fix 4c460e32b).
- A picture whose codes hold a secret (a key share, a TOTP setup, secret text) comes in only on the person's own tick (history fix 658c5f846).

## Domain calls

- `Item::new(&c, name, bytes)` (kind detection; `faraday/faraday-core/src/item.rs`), `seedqr_words`, `FileKind::{exposure, may_be_secret}`, `name_may_hold_key`, `zeroize`.
- No other domain crate is named; the crate behind `Item::new` and `seedqr_words` was not settled by the inventory.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 22e77b5c3 | After the first secret in a fresh process opened the reminders sheet, it replaced the boot import's Import sheet over its passphrase; closing both lost the import. | The device hold remembers the Import sheet it covered and restores it on release (`DeviceCheck::over_import`). |
| 658c5f846 | A picture at a stick visit whose codes held a secret came in without the person's tick; a file scanned while a question waited closed that question's sheet. | A secret-holding picture comes in only on the person's own tick, else it is dropped and stays listed; a scanned file closes the scan sheet only when it is open. |
| c3dd3d15d | A picture on a stick whose text was 16 or 32 bytes that also read as a CompactSeedQR was loaded as a key without asking. | Asks before it loads: Load as a key or Leave as a file, at a stick visit and at the boot import. |
| 4c460e32b | A picture copied in on a stick visit left the camera's scan open, so parts of an animated code could carry into a later Home Scan; the boot import took a same-named picture from another stick. | A stick visit reads pictures in its own scan; a picture's codes name their stick; the boot import takes codes only from the stick it imports. |
| a7c9aed6c | The Import button's words did not count a vault the listing missed, and changed while a ticked file was read. | The words count as the listing does and hold per answer. |
| 578f5021a | The one button did not say a ticked file may hold a key. | It reads "Import and load any keys". |

## Doubts

- a7c9aed6c and 578f5021a were given to the Files feature; they are about this one and are carried here. 0bab55cbf, c8bb85314, 2b14e801e and 5d9019486 touch both and are in `files-inbox`.
- Most line numbers in `faraday/faraday-core/src/boot_import/` come from one report and were not re-read; only those written plainly were checked.
- The doc of `import_go` (`faraday/faraday-core/src/boot_import/mod.rs:~1270-1271`) starts with a stale-looking cost line.
- `import_stick_gone` goes to Home only when `vaults.focus.is_none()` (`~983`).
- The "QR read" count includes text-seed-asked codes that are not named (`~941-953`).
- Which domain crate holds `Item::new` and `seedqr_words` is not settled.
