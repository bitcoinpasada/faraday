# Deleting files and zeroing a stick

Delete files on a stick: delete chosen files, zero the stick's free space, or
empty and zero the whole files partition, with one or two passes. Deleting a
vault or key file, and any zeroing, takes a hold, not a tap. Zeroing runs in
steps with a progress sheet that can be stopped. It is for the person clearing
a stick of secrets. It is reached from the stick's visit on the Files page
(Delete files), from the visit's notice of unprotected secret files, and from
the boot import's list.

## Screens and sheets

- `Sheet::Delete` (set at `faraday/faraday-core/src/stick/wipe.rs:~167`, `~489-490`; drawn by `delete_sheet`, `~512`): choose files, the free space, or the whole files partition; it shows what cannot be undone and what is not reached. The pass count is shown only for the whole partition.
- Under a hold the Go button becomes the hold button (`hold_button_in`, Error tone, `faraday/faraday-core/src/stick/wipe.rs:~723-743`) with Cancel beside it. The key-file-only wording when only key files are chosen.
- `Sheet::Zeroing` (set at `~333`, cleared at `~500-501`; `zeroing_sheet`, `~759`): percent, estimated time left, pass count (two-pass whole wipe), Stop ("Stopping", disabled, meanwhile, `~796-800`). It does not close on an outside press.
- Visit log lines: `EN.stick_wipe_deleted`, `stick_wipe_name_only`, `stick_wipe_not_deleted` (`faraday/faraday-core/src/stick/wipe.rs:~356-362`).
- A stick with 0 bytes shows "nothing" (`~562`); an empty listing shows a "no files" line (`~666-669`).

## Actions

- `Action::DeleteFiles` (`faraday/faraday-core/src/stick/wipe.rs:~831`): opens the sheet at `What::Files` with nothing chosen and asks the stick for its space (`StorageCommand::StickSpace`, `~156`).
- `Action::DeleteUnprotected` (`~832`): opens with the files the visit's secret-file notice names already chosen (`~172-184`).
- `Action::DeleteWhat(k)` (`~833`, `~235-239`): 0 chosen files, 1 free space, else the whole partition.
- `Action::DeletePick(k)` (`~834`, `~242-260`): toggles the file at listing place k and forces `What::Files`.
- `Action::DeletePasses(n)` (`835`, `263-267`): clamped to 1 or 2.
- `Action::DeleteGo` (`838-839`): ignored while `delete_holds()`; otherwise `delete_go` (`~270-336`). The hold, `VaultAction::HoldWipe`, calls `delete_go` (`faraday/faraday-core/src/vault/mod.rs:740`).
  - Files: one `StorageCommand::Wipe{stick,name}` per chosen name; the sheet closes (`~293-301`).
  - Free: `StorageCommand::ZeroFree{from:2}`, then Zeroing (`~302-334`).
  - Whole: `StorageCommand::MakeEmpty`, then Zeroing with the chosen passes.
- `Action::ZeroStop` (`~840`, `~374-387`): ends after the step in flight.
- Any other action reaching `wipe_act` is `unreachable!()` (`~842`).

## Effects

- Storage commands queued on `out.storage_out`: `StickSpace` (`faraday/faraday-core/src/stick/wipe.rs:~156-158`); `Wipe` (`~295-298`); `MakeEmpty` (`~305-307`); `ZeroFree` (`~309-312`, and `~444-447` per tick). The disk work is done by the shell's disk process (the doc at `~7-12` names `faraday_fat::Volume::wipe`, `zero_free`, `make_empty`); none of it is called here.
- `out.redraw()` and `Command::Draw` (`~448-451`, `~504-507`).
- The toast `EN.boot_import_being_imported` on a refusal (`~286`).
- `self.now_ms` read for step timing (`~392`) and the estimate (`~771`).
- Visit log lines (`visit.log_push`, `~357`, `~359`, `~503`).
- Into other areas: the boot import drops the name (`import_drop_name`, `~343`); the backup flow forgets copies on the stick (`flow_copy_gone`, `~342`; `flow_copy_gone_for`, `403`, `466`), which reopens that stick's card in the flow (`backup-flow`); the visit's chosen entry and unprotected notice are cleared (`visit.inn_retain`, `unprotected_of`, `unprotected_gone`).
- No camera, clipboard, printing, randomness or network.

## State and lock

- `DeleteAsk` (`faraday/faraday-core/src/stick/wipe.rs:~66-78`) in `self.stick.delete_ask`: `stick: Text`, `label: Text`, `what`, `chosen: Vec<String>` (file names), `passes: u8`, `space: Option<(u64,u64)>`. It lives from `delete_open` until `delete_go` takes it (`~289`) or the stick disappears (`~486-491`).
- `Zeroing` (`~81-102`) in `self.stick.zeroing`: `stick`, `label`, `whole`, `passes`, `pass`, `next: u32`, `total`, `due`, `stop`, `since`, `emptied`, `serial: Option<u32>`. It lives from `delete_go` until `zero_end` (`~498-508`) or the stick goes (`~493-495`).
- No secret is held.
- Both sit in `StickArea`, which is `at_lock::Public` (`faraday/faraday-core/src/stick/events.rs:224`): a lock clears nothing here. A lock ends the process (`faraday/faraday-core/src/vault/seal.rs:933-946`) and neither is kept state, so both are gone after a lock's restart.
- "A file chosen as a key file" is remembered only this power-on (`vaults.key_file_was_used`).

## Reach

- Opened from: the stick's visit on Files (`Action::DeleteFiles`), the visit's secret-file notice (`Action::DeleteUnprotected`), the boot import's list (`faraday/faraday-core/src/boot_import/screen.rs:201` calls `delete_files`).
- Reached from the shell's storage answers: `wiped`, `zero_step`, `zero_failed`, `zero_stop`, `zero_tick`, `zero_space`, `zero_sticks_changed` (callers outside `faraday/faraday-core/src/stick/wipe.rs`).
- Calls: `self.import_reads(stick, name)` (`~281-282`), `flow_copy_gone`, `flow_copy_gone_for`, `import_drop_name`, `vaults.key_file_was_used(f)` (`231`), `visit.{log_push, inn_retain, unprotected_of, unprotected_gone}`, `StickInfo::{is_vault, files, serial, id, label}`, `vault::screen::hold_button_in`, `faraday_ui::hold::HoldTone`, `compact::{buttons, scroll_sheet, sheet_head}`, `screens::{wrap_buttons, size_text}`, `self.medium.noun()`.
- Storage answers reach the wipe code through `faraday/faraday-core/src/stick/events.rs`: `Sticks` to `zero_sticks_changed` (`:38-41`), `Wiped` and `WipedName` to `wiped` (`:218-219`), `StickSpace` to `zero_space` (`:225`), `Zeroing` to `zero_step` (`:226-228`), `ZeroFailed` to `zero_failed` (`:229-234`), `WipeFailed` to `wiped` with the reason (`:236-239`); the shell tick runs `zero_tick` (`faraday/faraday-core/src/lib.rs:1395`) and `Action::ZeroStop` runs `zero_stop` (`faraday/faraday-core/src/stick/wipe.rs:840`).

## Rules and refusals

- A hold, not Go, is needed (`delete_holds`, `faraday/faraday-core/src/stick/wipe.rs:186-199`): for Files when any chosen file is a vault name or a key file; for Free and Whole always (F131).
- The key-file-only hold wording when only key files are chosen (`delete_holds_key_file_only`, `203-210`).
- A vault name is `StickInfo::is_vault` by contents, or a name ending `.kdbx` in any case (`223-227`). A key file is a `.keyx` name in any case, or a file chosen as a key file since power-on (`230-232`). After a restart a key file used before no longer needs the hold.
- Passes are clamped to 1 or 2 (`263-265`).
- `delete_go` refuses, with a toast and the sheet left up, when the boot import still reads a chosen file (Files) or the stick (Whole); Free is never refused here (`270-288`).
- `delete_go` does nothing when no sheet is open (`~289-291`).
- `wiped` (`~340-363`): a failure logs the reason and leaves the file; a success forgets the copies, drops the name from the import, clears the visit's chosen entry, and logs "deleted" (zeroed) or "name only".
- A whole wipe is treated as emptied on its first answer: the backup flow's copies on it are forgotten (`393-404`); it fails closed also in `zero_failed` before the first answer, or when the stick is pulled (`455-475`).
- Stop ends the step with "stopped at N%" (`~414-420`). A pass ends when done; more passes restart from cluster 2 (`~421-435`). A stick that goes fails the zeroing with `stick_wipe_the_was` (`~479-495`).
- The Go button is Primary unless Files is chosen with nothing chosen (`~559`). The estimate uses `TYPICAL_MBPS = 6` until a rate is measured (`~566`).
- The stick's Delete files button keeps a grid step from the foot buttons (fix 85b325ffb, in `stick-visit`).

## Domain calls

- `faraday_text::Text::{of, fill}`.
- None of the disk crates is called from the app: `faraday_fat::Volume::{wipe, zero_free, make_empty}` run in the shell's disk process (`faraday/faraday-fat/src/lib.rs`), reached through `StorageCommand::{Wipe, ZeroFree, MakeEmpty, StickSpace}`.

## Fixtures

None.

## Fixes

None.

## Doubts

- Fix 85b325ffb (the Delete files button's touch target overlapped the foot buttons) was given to `stick-visit`; it is about this feature's entry button.
- `chosen` holds file names as `Vec<String>` while `stick` and `label` are `Text`.
- Test modules were not read; no fixture is named for this feature.
