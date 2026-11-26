# Sticks coming and going

What happens when a stick (or SD card) arrives and leaves: the stick list, the
pills that name each stick, the Files page's visit of a stick (copy files in
from the stick to the Inbox, write files out from the Outbox), a name already
taken on the stick, files the stick and Faraday both hold, a write waiting for
a stick, and the refusal of secrets while a stick is "in the way". It is for
anyone moving files between Faraday and a stick. It is reached by putting a
stick in, from the Files page (`Screen::Files`), and from flows that write
(the backup flow, take-away writes, Save to a stick).

## Screens and sheets

- `Screen::Files` with the stick's visit (drawn by `faraday/faraday-core/src/screens/visit.rs`; `faraday/faraday-core/src/screens/files.rs:435` calls `visit::draw`). The stick pane stands when on Files with a stick in and no secret held (`stick_pane`, `faraday/faraday-core/src/stick/visit_taken.rs:1852`); a Lock card shows when a stick is in but cannot be used because a secret is held (`faraday/faraday-core/src/screens/visit.rs:1454-1485`).
- `Sheet::Taken` (Replace, Keep both, Cancel; `taken_sheet`, `faraday/faraday-core/src/stick/visit_taken.rs:1465`).
- `Sheet::NameTaken` (Overwrite held, Beside, Cancel; `name_taken_sheet`, `faraday/faraday-core/src/stick/visit_taken.rs:1518`).
- `Sheet::WriteOut` (the process is not clean).
- The Outbox delete dialog (`outbox_delete_sheet`, `faraday/faraday-core/src/stick/visit_taken.rs:1563`; its hold handler is elsewhere).
- `Sheet::Pull` (the text of what needs the stick out, `pull_what`, `faraday/faraday-core/src/stick/mod.rs:~107-173`).
- The receipt row of the last write (`visit_same::receipt_row`, called from `faraday/faraday-core/src/screens/files.rs:547`; `faraday/faraday-core/src/screens/visit_same.rs`).
- Stick pills (`faraday/faraday-core/src/stick/pills.rs`): one per stick, the boot stick is 1, backup sticks 2 to 255.
- Save to a stick and its panels (`faraday/faraday-core/src/vault/screen/save.rs`): `stick_save_sheet` (`~119-167`; `V::SaveTick(k)`, `V::SaveToStickGo`, `Action::Cancel`; Save is enabled only when a file is ticked; a seed gets the "anyone who finds" line, `~139-141`); `save_panel` (`~169-311`: session keys not yet held as a seed, or with their passphrase, and wallets not yet in the vault; `V::SaveKey`, `V::SaveKeyWithPassphrase` only for a key with a passphrase, `V::SaveWallet`; "every key saved" and "every wallet saved" lines); `gpg_sign_sheet` (`~14-117`: hold to sign `V::GpgHoldSign`, enabled only with a known clock and at least one file; without a clock the time reads "not known" in the warn tone with `Action::OpenSetTime`); `years_row` (1, 2, 3, 5 years, never); `reasons_row` (four reasons 0 to 3); `images_panel` (`~365-491`, Secure Boot: Inbox files that `is_image`; Sign `V::SbPick(k)`, Check `V::SbCheck(k)`, hold `V::SbHoldSign`, `V::SbImages`).

## Actions

- `visit_act` (`faraday/faraday-core/src/stick/visit_taken.rs:1905`): `VisitStick`, `VisitSettings`, `VisitOut`, `VisitIn`, `VisitInAll`, `VisitOutAll`, `VisitInbox`, `VisitBar` (does nothing), `VisitTab`, `VisitWrite`, `TakeWrite`, `VisitReplace`, `VisitKeepBoth`, `NameTakenBeside`, `NameTakenOverwrite`, `VisitCopy`, `VisitCopyAndLoad`, `VisitCopyAndSign`, `VisitFrom(Screen::Entry | back)`. Any other action is `unreachable!()` (`2040`): the dispatcher must route only these.
- Putting a stick in or pulling it out (`StorageEvent::Sticks`, `sticks_changed`).
- A pill press queues a folder insert or pull (`BOOTSTICK`, `STICK-n`, `faraday/faraday-core/src/stick/folders.rs`) in Faraday Demo; pills are not pressable on a device (`faraday/faraday-core/src/stick/pills.rs:~262`, `~687`).
- On the Pull sheet, take the stick out; Lock when a stick is in while a secret is held.

## Effects

- Storage commands: `StorageCommand::Write` with the bytes (`faraday/faraday-core/src/stick/visit_taken.rs:~1446-1455`, `~1786-1794`); `Read` and `ReadQr` (`.png` files are read as QR codes, `~2397-2409`); `Hash` (`~2227-2230`).
- Inbox and Outbox: `save_boxes()` after `put_item` (`~1020`); `vaults.read_free()`.
- Receipt: the time from `checked_now()` (`~2270`); the receipt of the last write stays this power-on.
- Navigation: `Nav(Screen::Files)`; `VaultAction::Open` after a pull (`~1898`).
- The event handler (`faraday/faraday-core/src/stick/events.rs`) also handles Clock, Memory, Cameras and QrSeen events, Printed toasts, Hashed, StickSpace and Zeroing answers (see `delete-and-zero`), problem sticks (not FAT32 or read-only, `~63-65`), picture import, a secret written unprotected recorded by name (`~160-189`), and Faraday Demo host events (feature `demo`).
- Toasts.

## State and lock

- `VisitArea` (`faraday/faraday-core/src/stick/visit_taken.rs:149`, `~149-199`): pull, after_visit, for_key, pictures_scan, receipt (this power-on), vault_from (name, SHA-256, label), files_menu, the name_taken queue (hashes, no bytes), wrote, outbox_delete, sign_stick, sign_from, sign_file, lock_to_write, write_waiting, signed_end, same.
- `VisitState` (`faraday/faraday-core/src/stick/visit_taken.rs:2076`, `~2076-2124`): stick, tab_stick, the out, inn, inn_own and reading sets, settings, log, load_after, bars, writing, taken, unprotected (names only).
- `SameFiles` (`faraday/faraday-core/src/stick/visit_same.rs`): names and sizes only, marks per stick, dropped when the stick is no longer listed; lives at `app.visit.same`.
- `StickArea` (`faraday/faraday-core/src/stick/events.rs:~186-221`): sticks, not_now, held, problems, toggles, settings body, delete_ask, zeroing, visited, waits, boot_medium, attached. It is `impl at_lock::Public` (`faraday/faraday-core/src/stick/events.rs:224`): a lock wipes nothing of it. It holds no secret; labels and file names are kept.
- No secret is kept in these structures; file bytes travel to the disk process in the write commands.
- `VisitArea`'s lock (`faraday/faraday-core/src/stick/visit_taken.rs:2526-2541`): `holds` is "a pictures transfer read in parts"; `lock()` clears `pictures_scan`, `taken`, `name_taken` and `after_visit`. It does not clear `write_waiting`, `lock_to_write`, the receipt, `vault_from` or the sign fields.
- A lock ends the process (`faraday/faraday-core/src/vault/seal.rs:933-946`); only what the kept state encodes survives. The memory writer encodes only `lock_to_write`, as the `write-waiting` bytes (`faraday/faraday-core/src/memory/mod.rs:462-465`: a write waiting in this process is not carried over a later lock). After the restart `faraday/faraday-core/src/stick/events.rs:626` calls `visit_write_waiting`, which reads it back. So a "Lock and write" survives its own lock; an in-process `write_waiting` does not outlive a later lock, and the idle lock gives up a waiting write (fix d9492c283).
- Restart clears everything else in these areas.
- `put_outbox_pair_beside_files` (`faraday/faraday-core/src/stick/visit_taken.rs:1045`) takes plain `Vec<u8>` for a picture and a file. Its one caller, `kef_save_files` (`faraday/faraday-core/src/compute.rs:785-796`), passes a KEF code's PNG and its sealed `.kef` file, which are public.
- `lock_drops` names the Inbox files a lock drops, for the Write out sheet's own section (`faraday/faraday-core/src/stick/visit_taken.rs:1141-1147`, `faraday/faraday-core/src/screens/sheets.rs:742-755`).

## Reach

- Opens: Files, `Sheet::Taken`, `Sheet::NameTaken`, `Sheet::WriteOut`, `Sheet::Pull`, the Lock card, the vault open after a pull (`VaultAction::Open`), Entry (`VisitFrom(Screen::Entry)`), loading and signing a copied file (`VisitCopyAndLoad`, `VisitCopyAndSign`).
- On a stick's arrival the backup flow on screen takes the stick (`faraday/faraday-core/src/stick/events.rs:~555-567`; `backup-flow`) and the Import list is taken down; the boot stick's first look offers the boot import.
- Calls: `storage_api::OnTaken`, `compact`, `secrets::Exposure`, `vault::more::{Loss, written}`, `vault::screen::hold_button_in`, `file_menu`, `stick::visit_same`, `stick::settings`, `vaults` (`read_free`, `vault_files`), `wallets.spend()`, `import`, `entry`, flows (`flow_name_taken`), `message_file_waits`, `in_the_backup`, `holds_secret`, `clean()`; on arrival `kef_stick_arrived`, `seed_read_stick_arrived`, `lightning.stick_arrived`, `pass_field::stick_arrived` (`faraday/faraday-core/src/stick/events.rs:~539-548`).
- Reached from `visit_act`, `StorageEvent::NameTaken`, `Hash` answers, write answers, the lock path (`spend_write_plan`, `visit_write_waiting`), flows (`toast_out`, `put_outbox_*`), `delete-and-zero` (`Action::DeleteFiles` from the visit).
- `outbox_free_name` and `free_name` live here (`faraday/faraday-core/src/stick/visit_taken.rs:~114`, `~95`).

## Rules and refusals

Arrival and removal (`faraday/faraday-core/src/stick/events.rs`)

- A clean process takes the stick; the boot stick's first look offers import unless it holds only the settings file; a stick that arrives while not clean shows the Lock card and Import is not offered (`~575-580`).
- A stick arriving while the camera is on is held back, and Lock is offered when the scan ends (`~452-455`; fix 61abed3b8). The camera counts as on from the scan held, whatever sheet stands over it, for Add randomness as for New key (fixes 2d97efcde, f175411b3).
- A picture import takes SeedQR or secret codes only on the person's own tick (`~294-327`); a secret file read but not ticked is not kept (`~79-88`).
- A first look counts as a visit only with more than the settings file; the settings row shows only when a setting differs (fix b007ac257).
- A later card put into the Pi's own slot after boot is an attached stick, not the boot card (fix 0ec857cce).
- Backup flow slot lines (`flow_stick_slot`, `flow_record_slot`) match a stick to a slot only by bound or written record, label, serial, listing and `.kdbx` SHA-256; an ambiguous or unknown stick gives no slot (`faraday/faraday-core/src/stick/pills.rs:~317-376`).

Secrets in the way (`faraday/faraday-core/src/stick/mod.rs`)

- `refuses` (`~30-45`): a passphrase or key typed or loaded while a stick is in the way is refused (also when `!may_load_keys()` for non-entry fields; a vault entry's field only for the stick); the refusal clears focus and toasts with the medium's noun. Only Char, Backspace, Delete, Enter and Tab are refused (`~66-72`).
- Tool secret fields lose focus and are cleared (`tools_lose_secrets`, `~77-95`).
- A stick's arrival clears passphrase, phrase and dice focus (`faraday/faraday-core/src/stick/events.rs:~539-548`); the Krux code form wipes what was typed (`wallet-backup`).

Copy in and write out (`faraday/faraday-core/src/stick/visit_taken.rs`)

- `put_item` refuses any Secret-exposure item in every build (`1014-1016`); a secret enters the Outbox only through `outbox_secret`.
- `VisitOut` is refused for a file in a backup or a message waiting for its key's backup (`1931-1941`); `VisitIn` only when `unread_why` is None (`1943-1951`).
- `visit_files` (`~1111-1137`): Outbox files only when chosen, not while waiting for a backup; a take-away scope limits to the scope's names and no Inbox files. The take-away write sends only its own rows' files (fix d9492c283).
- `visit_taken` (`~1177-1200`): vault files are skipped (the disk process asks, `OnTaken::Ask`); each taken name gets a free keep-as name.
- `visit_write` (`1387`, `~1387-1460`): not clean opens `Sheet::WriteOut` and writes nothing; no stick or identity returns; the settings file only with scope None and its row on; a vault file overwrites the same vault on the stick only when not from the Inbox, or when this session unlocked or wrote it (`~1430-1431`).
- `visit_write_waiting` (`1263`, `~1263-1313`): not clean clears the wait; a hash and name count mismatch clears it; no named stick clears it; a named stick not in keeps the wait; only files whose name matches an Outbox file with the same SHA-256 and that are not Secret are written (fix 7e7cfa757).
- `WriteWait::sound` and `decode` refuse control characters and malformed lines (`~233-300`; fix 948e491e8).
- `take_write` (`1342`, `~1342-1381`): a held signed message is not written; not clean keeps the wait as `lock_to_write` and shows `Sheet::WriteOut`.
- Replace and Keep act only with their sheet (fix d9492c283).
- Name taken: Overwrite only with the SHA-256 of the file on the stick, else Beside (`~1770-1773`); resent bytes must still match the first send's SHA-256 (`~1677-1683`); a Secret file is refused in flow mode; not clean gives up all entries (`~1761-1768`); a pulled stick cancels its entries (`~1825-1846`).
- `VisitCopy*` is refused while the import tile is up (`~1909-1916`); `visit_copy` skips files with `unread_why` and files already being read (`~2387-2395`).
- `visit_default_in` (`2290`, `~2290-2310`): ticked by default only with a known kind, readable, not a seed by name, and not already in the Inbox at that size. `visit_default_out` (`~2317-2346`): with several vault files in the Outbox only one is ticked.
- `visit_sign_file` (`2359`, `~2359-2371`): exactly one `.psbt`, ticked.
- `VisitFrom(Screen::Entry)` has no `holds_secret` check; `VisitFrom(back)` needs a stick in and no secret held (`~2022-2038`).

Same files

- `visit_same_ask` (`2176`, `~2176-2232`) asks nothing while a secret is held, at the boot stick's first look before the settings state, or while the import list stands for this stick; never for the settings file; only when Faraday holds a file with the same name and size. `visit_same_hashed` (`~2237-2264`): the same name, size and SHA-256 marks it; a failed read is retried.

Pull

- `visit_pulled` (`1872`, `~1872-1902`) returns to the flow only when Files is up and the import list is down; it opens a sealed vault only if no `after_visit` waits.

Save to a stick and signing sheets (`faraday/faraday-core/src/vault/screen/save.rs`)

- Save is enabled only when a file is ticked. GPG sign needs a known clock and at least one file. Secure Boot Sign is offered only with the release-check result (F134); a refusal is named and leaves only Back; the blinding randomness row and the hold only if the check passed; Back up first replaces Sign while a backup waits.

Crafted sticks

- A FAT16 volume claiming more clusters than FAT16 holds is not offered; a truncated cluster number is refused as corrupt; an oversized vault part is removed unread (fix 466494b01).

## Domain calls

- `faraday_codec::vault_file::is_vault_file`.
- `faraday_crypto::work::Compute`, `sha256_of`, `Item::hash`.
- `faraday_intake::seedqr_words` (picture import).
- `faraday_shell_api::{Command, Orientation, Key}`, `faraday_ui::stick` (pills), `faraday_text::Text`.
- The disk work runs in the shell's disk process (faraday-fat, faraday-files), reached only through `StorageCommand`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 25033271f | A secret file pushed by the plain push could reach the Outbox: a debug panic, and in release it was silently dropped. | A secret file enters the Outbox only through `outbox_secret`, in every build. |
| 0ec857cce | A different card put into the Pi's own slot after boot was treated as the boot card, so passphrases and key loads were not refused. | A later card in the slot is an attached stick; the record keeps the kernel's disk sequence number. |
| 7e7cfa757 | A kept (waiting) write could be matched to a file by name only. | The next process writes a file only when name and SHA-256 match and it is not secret. |
| 948e491e8 | A newline in a file name or stick identity made a second line in the kept list. | No write with a control character is kept; the kept-list decoder refuses it; mismatched counts are given up. |
| b007ac257 | Faraday Demo showed a Files-on-Boot-stick tile when the stick was empty or held only settings; the settings row showed at default settings. | A first look counts as a visit only with more than the settings file; the settings row shows only when a setting differs. |
| 27112a8c5 | The boot stick button flashed only on the Pull sheet and a backup's stick, not every waiting step. | Every waiting step flashes it. |
| 8513c1c6f | Stick pill height, padding and text size did not match the mode pill on wide layouts. | Both 28 high, 12 padding, Label M. |
| d9492c283 | The take-away write sent every file, not its own rows'; a waiting write survived the idle lock; Replace and Keep could act without their sheet. | The write sends its own rows' files; the idle lock gives up a waiting write; Replace and Keep act only with their sheet. |
| 55dabefab | The Krux code's two file names read the same when cut at the end; a waiting PNG's line said "File". | Long names keep their start and end; a picture reads Picture. |
| 61abed3b8 | A stick shown by the kernel during a scan put the Lock sheet over the scan without turning the camera off. | A stick arriving while the camera is on is held back, and Lock is offered when the scan ends. |
| 2d97efcde | With the idle warning over a running scan the camera counted as off, and a stick was handed out while it ran. | `camera_on()` answers from the scan held, whatever sheet stands over it. |
| 93f293b51 | After CameraOff, Scan again's Pull never gave way to Lock with a stick in; Not now could leave a card or press up. | The camera counts as on only while the Scan sheet or idle warning stands; each way out clears card and press. |
| f175411b3 | During Add randomness's camera `clean()` said the camera was off, so a stick arriving was not held back. | The camera counts as on for Add randomness as for New key. |
| 466494b01 | A crafted stick or vault crashed or cross-linked files: a FAT16 volume claiming more clusters than FAT16 holds was offered; a cluster number was truncated when written; an oversized vault part was read; plan and share limits were not refused. | Fail closed: such a stick is not offered, a truncated cluster is refused as corrupt, an oversized part is removed unread, a plan over 1024 places reads as no plan, and no index goes past the seeds. |
| 85b325ffb | The stick's Delete files button sat at the same height as the two buttons at the foot, so their touch targets overlapped. | The touch targets keep their floor (a grid step apart). |

## Doubts

- Left out of Fixes, as about the boot finder and USB authorisation rather than this feature: c83dbbd28 (the boot-stick finder ran as root for its whole search), 88421cf04 (the finder read other devices and files; the Pi could miss a late card), a6c3e33b2 (a USB device plugged in after boot stayed authorised if the command line lost its USB word).
- Fixes 7e7cfa757 and 948e491e8 have symptoms inferred from the commit, not read in full. 466494b01 also covers plan limits that belong to the backup plan.
- `VisitFrom(Screen::Entry)` has no `holds_secret` check, unlike `VisitFrom(back)`; whether that is meant is not known.
- `faraday/faraday-core/src/screens/visit.rs` (the Files page body) was not read in full; the visit's drawing rules beyond those cited are missing.
- Two files share a name: `faraday/faraday-core/src/screens/visit_same.rs` (drawing) and `faraday/faraday-core/src/stick/visit_same.rs` (logic, `SameFiles`).
- Test modules were not read; no fixture is named for this feature (`stick::settings::FILE` is a name only).
