# Sealing a vault on lock

How an open vault's changes are sealed into a new vault file and written back
to the Outbox, at a lock (every changed open vault) or for one vault (a
passphrase change, a memory change, a vault made in a flow); what a lock
does to the vault area; and the vault area's own rules that the seal leans
on: seal counts and older copies, the summaries kept across a lock, the
memory needed to seal, a new passphrase, a new memory step, plan records and
the clock. It runs for the person who presses Lock or is locked by the idle
timer, and for flows that seal a vault before they go on.

## Screens and sheets

- Working line `EN.vault_sealing` while a seal runs on the worker.
- Lock sheet `Sheet::LockAsk` (see `lock`): the vault part is `LockBlocks` (`lock_blocks`, `faraday/faraday-core/src/vault/area.rs:~1504`; `lock_blocks_seen`, `~1493`; `lock_blocks_of`, `~1518`). It names each vault a Lock without saving would lose or leave short, with lines `vault_lock_short`, `vault_lock_unwritable`, `vault_lock_moved`; a vault with a moved item goes with its target (`~1552-1572`).
- What a lock keeps or wipes (`kept_wiped_now`, `faraday/faraday-core/src/vault/mod.rs:~145-269`): the list the Lock sheet and the power-off sheet draw (rules below).
- Change passphrase `Sheet::VaultPassphrase` (`passphrase_dialog`, `faraday/faraday-core/src/vault/rekey_screen.rs:~25-80`): two fields; Save only when `pass_field::pair::ready`.
- Memory `Sheet::VaultMemory` (`memory_dialog`, `faraday/faraday-core/src/vault/rekey_screen.rs:~85-188`): five steps from `faraday_vault::file::STEPS_KIB`, each with size, passes and bits; machine rows (`MemoryHere`, `MemoryMachine`); a warning when the next seal's memory exceeds the weakest machine's limit. Not drawn without `next_cost` or a current vault.
- Older copy dialog `Sheet::OlderCopy` ("Open this copy?"; `OpenHold`, `faraday/faraday-core/src/vault/seals.rs:~477-591`): Open (`older_take`) or Keep locked (`older_close`, restores the remembered summary). Drawn on Unlock (see `vault-unlock`).
- Vault contents head: seal count and warning (see `vault-contents`).

## Actions

- Lock (any way; see `lock`): `vault_seal_all(may_lose)` seals every changed open vault.
- `ChangePassphrase` (`vault_change_passphrase_ask`, `faraday/faraday-core/src/vault/mod.rs:~4797-4799`), then Save on the dialog (`vault_set_passphrase`).
- `MemoryAsk` (`faraday/faraday-core/src/vault/mod.rs:~815`), `MemoryHere` (`~823`), `MemoryMachine` (`~828`), a step press (`vault_memory_step`, `~4863-4876`).
- `OlderOpen` / `OlderBack` on the older copy dialog.
- `vault_lock_one(.., After)` seals and closes one vault (used by rekey and by flows; `After::{Nothing, CreateFrom(Screen), Unlock}`).

## Effects

- Each changed open vault is sealed on the worker (`Job::Seal`, `EN.vault_sealing`), one at a time, into the Outbox (`put_outbox`, `faraday/faraday-core/src/vault/seal.rs:~829`), then closed (`faraday/faraday-core/src/vault/seal.rs:~929-946`).
- The seal writes the `AMOUNTS` and `NONCES` records (`faraday/faraday-core/src/vault/seal.rs:~434-527`) and the plan edits to `PLAN_ANSWERS` (`faraday/faraday-core/src/vault/seals.rs:~320-367`).
- Randomness: `app.fresh_bytes(b"vault seal", SEAL_RANDOM_LEN)` returns `Option<Zeroizing<Vec<u8>>>` (`faraday/faraday-core/src/fresh.rs:568`), copied into a `Zeroizing<[u8; _]>` (`faraday/faraday-core/src/vault/seal.rs:568-578`); the `uuid_bytes_from` locals are `Zeroizing` (`~541-557`). `Command::RequestEntropy` with `EN.no_randomness` when the pool is empty.
- At the end of a lock: `seal_end` (`run.all`, `faraday/faraday-core/src/vault/seal.rs:675-692`): `lock_flows`, `vaults.state.close_all()`, `nonces_sync_open`, `state.form = None`, `drop_vault_dialog` (closes the `VaultPassphrase` and `VaultMemory` sheets), `state.load = None`, `state.pass_seeds = None`, then `lock_finish` (see `lock`).
- `vault-summaries` written at the lock with the boxes (`save_boxes_for_lock`).
- `idle.input` on a seal's end.
- The unlock-speed measurement (`faraday/faraday-core/src/vault/stretch.rs:~828-907`): `Job::Measure`, 64 MiB, 1 pass, 1 lane, ChaCha20, no compression, run once on the worker (`measure`, `faraday/faraday-core/src/vault/area.rs:~98-105`); a failed pass records none.

## State and lock

- `SealRun` in `app.vaults.state.sealing` (`faraday/faraday-core/src/vault/seal.rs`): a queue of vault ids, `running`, `all`, `may_lose`, `lock_after`, `after: After::{Nothing, CreateFrom(Screen), Unlock}`.
- Lock sequence for the vault area:
  1. `lock_secrets_off_screen` first (`faraday/faraday-core/src/vault/seal.rs:151-199`): `qr.close()`, `vaults.hide_secrets()` (item secrets and `sign_shown`; the TOTP code shown is taken and put back), Explore, Convert, `kef_tool`, the Core wallet tool, Lightning, Vanity, the scan card, backup, keygen, `typing.pass_eye.close`, BIP-85, seed read; then `scan_drop_at_lock`, `backup.drop_paper`, `canvas.forget_glyphs`.
  2. Each changed vault is sealed.
  3. `seal_end` as above; `close_all` (`faraday/faraday-core/src/vault/area.rs:2932-2941`): `drop_shown` (shown, seedqr, words, paperkey, code), `sign_shown` cleared, `codes = None`, `moved`, `moved_subjects`, `open`, `stuck` cleared, `store.clear()`, `search` reset.
  4. `lock_finish`: every area's `lock()`. `AtLock for VaultArea` (`faraday/faraday-core/src/vault/area.rs:4461-4487`): `holds`: `kept` = any open vault; `typed` = passphrase, prompt pass, create phrases or dice rolls. `lock` (`4477-4486`): `drop_sign_text`, `passphrase.clear()`, `prompt = None`, `create = None`, `dice.close()`. Open vaults are closed by the seal's end, not by `lock`.
- `seal_given_up` does the same without sealing (`faraday/faraday-core/src/vault/seal.rs:119-130`).
- A failed seal keeps the vault open with its changes (`faraday/faraday-core/src/vault/seal.rs:~897-899`).
- Kept across a lock (public): `summaries` (the Locked sheet reads them; gone at power-off), `fingerprints`, `accounts`, `sealed_from`, `key_files_used`, `just_made`, `unlock_error`, `form_error`, the seal file name, the `unsaved`/`step_dropped` flags until the next lock, clock and memory fields. After the restart the area starts from `Default` (`faraday/faraday-core/src/vault/area.rs:~29-45`) and `summaries` come back from the previous process (`set_summaries`, `~875`).
- Seal counts (`faraday/faraday-core/src/vault/seals.rs`): `SealSeen { count, digest (SHA-256 of the whole file), planned }` per vault in the summaries.
- Summaries (`faraday/faraday-core/src/vault/summaries.rs`): the `vault-summaries` lines `vault, copy, from, wallet, key, sum, planned, wid, pid, map, seal`.
- Clock (`faraday/faraday-core/src/vault/area.rs`): `set_clock(unix_secs, at_ms)` (`~760-763`), `clock_offset`, `clock_checked` (setters `~865-872`, with no check there), `local_secs` floors at 0 (`~3578-3580`), `CLOCK_FLOOR` 2020-01-01 (`~3572`).
- Plan records (`faraday/faraday-core/src/vault/plan_store.rs`): a wallet's backup plan (answers, holds, notes) kept in the vault through `plan_record`, `plan_forget`, `plan_take`, `plan_restore`, `plan_refused`, `plan_extras_of`.
- `older` and `save_hold` are not cleared by the seal's end or a lock hook. `vault_older_settle` (`faraday/faraday-core/src/backups/mod.rs:4205-4216`) and `vault_save_also_settle` (`faraday/faraday-core/src/vault/mod.rs:4774-4781`) drop them after a press, once their vault is gone or their sheet is down. At a lock they are cleared with the app, the not-kept rebuild included.
- A seal result that arrives when the run is gone (`sealing == None`) only clears that vault's sealing flag (`faraday/faraday-core/src/vault/seal.rs:~787-791`, `faraday/faraday-keys/src/vault/seal.rs:509-516`): after a lock without sealing (`seal_given_up`, `faraday/faraday-core/src/vault/seal.rs:121-133`) the vault is already closed and nothing happens; after a refusal or stop it is still open with its changes. A refusal before the job of a lock of one vault clears the run and shows its reason (`faraday/faraday-core/src/vault/seal.rs:~392-400`); a pending Lock and `after` are dropped there, while the seal error arm (`faraday/faraday-core/src/vault/seal.rs:~900-924`) runs the pending Lock and drops `after`.

## Reach

- Called by `lock` (`vault_seal_all`, `lock_finish`, `lock_secrets_off_screen`, `lock_after_seal`, `lock_pending`, `sealing`, `seal_overdue`, `seal_given_up`, `vault_lock_one`).
- Reaches `memory::{records_lacking, from_record, nonces}`, `idle`, `lock_ends_create`, `flow_before_lock`, `stick::visit_taken::free_name`, `more::{sealed, drop_gone}`, `wallets.set_list_changed`, `plan::text`, `gpg::date`.
- Rekey and memory changes are opened from Vault contents (`vault-contents`); the older copy dialog from Unlock (`vault-unlock`); the memory need is shared with Create (`vault-list-create`) and Unlock.
- The backup flow and the board read seal counts and `sealed_from` (`faraday/faraday-core/src/backup_flow/steps.rs`, `faraday/faraday-core/src/backups/mod.rs`).

## Rules and refusals
- A time zone offset is a sign, two digits, `:`, two digits of minutes (0, 15, 30, 45), from -12:00 to +14:00; the digit fields are read with an integer parse that also accepts a leading sign (faraday/faraday-core/src/vault/area.rs:3591-3611).
- A vault seal record's planned wallet checksums are written comma-joined in the kept summaries and split on read, empty pieces dropped (faraday/faraday-core/src/vault/summaries.rs:75, 175-178).

- One seal run at a time: `vault_lock_one` returns false while sealing (`faraday/faraday-core/src/vault/seal.rs:263-266`).
- A changed vault needs randomness, else `RequestEntropy` and `EN.no_randomness` (`~271-275`). Seal nonces are drawn only once the session's seed is in; a missing seed refuses, it does not produce zeros (fix a3c29e5c9).
- An item moved out waits for its target (`vault_moved_lock_first`, `~276-287`).
- `cannot_seal` refuses a lock of one vault; in a lock of all the vault is let go only when `may_lose`, else the run stops (`~289-292`, `~372-381`). A requested lock stops and asks when a vault cannot be sealed (fix 1d2273364).
- An unchanged, non-rekeyed or read-only vault closes unwritten (`~354-366`).
- Memory refusal: an idle lock loses the changes; otherwise a toast and the run stops (`~387-400`). A memory error in a seal on idle drops the new memory step and seals again (`~863-873`).
- A seal over `SEAL_LIMIT_MS` = 5 minutes (`faraday/faraday-core/src/vault/seal.rs:106-115`) is given up: every open vault is let go unsealed, changes lost (`set_changes_lost`), and the lock finishes (`121-133`).
- A Lock pressed during a seal is remembered; a lock of all already running is not (`~201-210`).
- Seal file name: never overwrites a different file in the Outbox or Inbox; a copy of the same vault by SHA-256 keeps its name (`~757-777`). A seal replaces only an earlier copy of the same vault; otherwise it goes beside the file as `name-2.kdbx`; out-of-memory is reported as memory, not a bad file (fix ca8d2fddb). Write-back goes only over the files the vault was sealed from; a copy from another line is confirmed by the person (fix ca3ca4251).
- A seal reads back what it wrote, byte for byte, and opens the file again with its keys before it is given (fix adc81817d).
- An unreferenced attachment is emptied at the next seal (fix 866e789ba). A changed password in an unprotected KeePass entry is written protected (fix e06019dd8). A write-back compares old and new secret values in constant time (fix 70b646b7c).
- Signed-amount memory belongs to the open vault and is not sealed into another (fix 07209d73a).
- What a lock loses (`faraday/faraday-core/src/vault/area.rs`): only a vault past the ceiling (`fv::file::CEILING` unless the store lowers it) or one that cannot be written; short memory loses nothing (`cannot_seal`, `~1412-1418`; `cannot_seal_with`, `~1435-1465`; `Cannot.short`). `usize::MAX` stands for "unwritable" (`~1349-1350`, `~1450-1453`). A vault whose changes do not fit is never closed at a lock; the lock asks first, names the vault and the overflow, and offers Lock without sealing (fix 61c38a066).
- `seal_refusal` (`faraday/faraday-core/src/vault/area.rs:3101-3133`): free memory unknown is not a refusal ("a figure it does not give is no ground to keep a vault from being saved"); known and need over free gives `vaults_needs_this_computer`. Need = max(Argon2id memory, 16 x document + 3 x attachments) + 100 MiB; the exact count is taken only where it fits in free memory and the bound alone is short.
- Stuck seal kept until the vault changes (`stuck_of`, `~1582-1588`).
- `kept_wiped_now` (`faraday/faraday-core/src/vault/mod.rs:~145-269`):
  - a held lock warns; power-off lists the flows' power lines;
  - an unsealed changed vault is wiped if there is no seed to seal with (`~171`) or it `cannot_seal` and is not short (`~185-203`);
  - read-only closes as it is (`~177-178`);
  - at power-off a vault never written or changed is lost (`~168-169`, `~206-207`);
  - Outbox files are wiped at power-off or a mode change (`~222-225`);
  - session keys with no open vault and no checked copy are listed "nowhere else"; "save" is offered only with a vault open (`~233-256`); items kept nowhere else come first (`~267`).
  - `keys_kept_nowhere` (`~287-291`) blocks the Lock sheet.
- `seal_waits` / `lock_would_lose` (`faraday/faraday-core/src/vault/mod.rs:~2677-2707`): a lock waits when the seal answers are not ready (`SEAL_RANDOM_LEN` in 32-byte pieces plus one per vault with new entries); it would lose when the session's seed has not arrived or a vault is past the ceiling by exact count.
- Cancel clears a pending mode only when no restart or seal is under way (fix 81d63888a).
- Rekey: `vault_change_passphrase_ask` is refused unless the vault is writable and nothing is being written (`faraday/faraday-core/src/vault/mod.rs:~4797-4799`); `vault_set_passphrase` is refused when empty (`vaults_every_passphrase`) or differing (`vaults_each_passphrase`). The new passphrase is written at the next seal; then the vault is locked and unlocked once with it (`vault_lock_one(.., After::Unlock)`, `~4847-4849`; F301).
- Memory dialog: `MemoryAsk` opens only when `vault_may_reseal()` and not `screen::writing` (`faraday/faraday-core/src/vault/mod.rs:~815`); `MemoryHere`/`MemoryMachine` only while the dialog is open (`~823`, `~828`); a step is offered only if `usable` (the current one, or `step_refusal` gives None) and marked as fitting only within the weakest machine's limit; `vault_memory_step` needs an index into `STEPS_KIB`, then `read_free`, then a `step_refusal` toast (`~4863-4876`); pressing the current step changes nothing and clears a pending one (fix 9783081ca, see `vault-contents`). `weakest_of` gives each ticked machine half its memory (`faraday/faraday-core/src/vault/area.rs:~3210-3220`).
- Seal counts and older copies (`faraday/faraday-core/src/vault/seals.rs`): an older-copy warning when the count is lower than the highest seen; a "same count" warning for the same count with different bytes (`~193-196`, `~246-251`); never refuses, the vault stays open (`~259-260`); plan lines (`PLAN_ANSWERS`) replaced only by a higher count (`~348-352`), written only when a line changed (`~354-365`), kept only for wallets the vault plans or holds (`~336-341`); `copy_descends`/`copy_lineage` with `sealed_from` (`~422-453`). A sealed copy records its true count (fix 2b550d917, see `lock`).
- Summaries decode (`faraday/faraday-core/src/vault/summaries.rs`): tabs and line breaks become spaces on encode; a vault whose id is not 64 lower-case hex is skipped with its following lines (`98-104`); a `seal` line needs a count of at least 1 and a valid digest (`168-183`); `wid`/`pid` kept only if 64 hex (`wid` also if empty); id lists cleared unless one per sum; a bad entry count becomes 0; a bad time becomes None. A vault's last-sealed time is read only when each field is ASCII digits, the clock fields are in range and the day exists in its month (fix 4ba41ba7d).
- Plan records (`faraday/faraday-core/src/vault/plan_store.rs`): a plan for a key set that does not map one-to-one is refused (`seed_map` None when fingerprint counts differ, `~33-39`); a record for the other key order that does not read is refused and nothing is written over it (`~119-135`, `~153`); notes beyond the wallet's seeds are dropped (`~88-89`); an identical plan is not written (`~221-228`); `plan_restore` re-inserts when the new record does not fit (`~320-327`).
- Time zone (`parse_tz_offset`, `faraday/faraday-core/src/vault/area.rs:3594-3611`): exactly six bytes, sign first, `b[3] == ':'`, minutes in {00, 15, 30, 45}, total -12:00 to +14:00. Hours and minutes parse as `i16`, so a sign inside a digit field is accepted: `"+-1:00"` gives -01:00 and `"+00:+0"` is accepted.

## Domain calls

- `faraday_keys::vault` (`faraday/faraday-keys/src/vault/mod.rs`, `faraday/faraday-keys/src/vault/seal.rs`): `OpenVaults`, `SEAL_RANDOM_LEN`, `Sealed`, `measure`.
- `faraday_keys::job` (`faraday/faraday-keys/src/job.rs`): the seal job; `faraday_keys::kef::SealJob` (`faraday/faraday-keys/src/kef.rs`).
- `faraday_vault::file::{CEILING, STEPS_KIB, Facts}` (`faraday/faraday-vault/src/file.rs`); `faraday_vault::records::field::PLAN_ANSWERS` (`faraday/faraday-vault/src/records.rs`); `faraday_vault::kdbx` (`faraday/faraday-vault/src/kdbx/`).
- `faraday_plan` (`plan::{Shape, Answers, Note, place_map, At, text}`).
- `faraday_crypto::{sha256_whole, date::civil_from_days}` (`core/faraday-crypto/src/`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 396702389 | A seal that never finished kept the device open indefinitely | A seal past five minutes ends in lock and restart |
| 61c38a066 | Entries scanned into a full vault were dropped with no message; a changed vault whose records overflow its slot could be locked and closed, losing changes | A vault whose changes do not fit is never closed at a lock; the lock asks first and names the vault and the overflow |
| a3c29e5c9 | The session seed was 32 zero bytes until the shell's first answer; a vault seal nonce and a new keyboard's code were drawn from it unchecked | Seal nonces and keyboard codes are drawn only once the seed is in; a missing seed refuses |
| ca3ca4251 | A vault sealed from an older copy replaced the newer copy on a stick without asking; a copy from another line was not asked about | Write-back goes only over the files the vault was sealed from; a copy from another line is confirmed by the person |
| adc81817d | A seal gave the file without checking it; a writer fault could produce a file that did not read back | A seal reads back what it wrote, byte for byte, and opens the file again with its keys before it is given |
| ca8d2fddb | A seal made at a lock replaced a KeePass export or another file of the same name in the Outbox; a seal from an older copy could replace a newer one; an allocation failure was reported as a bad file | A seal replaces only an earlier copy of the same vault, otherwise goes beside it as name-2.kdbx; out-of-memory is reported as memory |
| 1d2273364 | A lock the person asked for did not say when a later vault could not be sealed; a save did not say why a record was not written | A requested lock stops and asks when a vault cannot be sealed |
| 866e789ba | An entry deleted from a file vault kept its attachment in the file | An unreferenced attachment is emptied at the next seal |
| e06019dd8 | A changed password in a KeePass app's unprotected entry was written unprotected; a copy could overwrite the original | The changed password is written protected; a new copy goes beside the original under a free name unless the bytes match |
| 70b646b7c | A vault write-back compared an entry's old and new password by stopping at the first differing byte | A comparison of secret values does not stop at the first differing byte; lengths are compared openly |
| 07209d73a | An open vault's remembered signed amounts were sealed into another vault | Signed-amount memory belongs to the open vault |
| 81d63888a | Cancel cleared an agreed mode while a restart or seal was under way | Cancel clears a pending mode only when none is under way |
| 4ba41ba7d | A vault's last-sealed time could be read from a malformed stamp, giving a time that names no real moment | The sealed time is read only from well-formed fields and a real day |

## Doubts

- No rows were given to this feature; the rows above were given under Inside a vault and moved here. 2b550d917 and f65ef4ddc (the Locked sheet's notices) are in `lock`.
- 1d2273364 also changes how typed codex32 shares are tried; that part belongs to the codex32 backup feature. Its file list was partly cut.
- `seal_refusal`'s doc comment (`faraday/faraday-core/src/vault/area.rs:~3088-3100`) reads as two docs merged.
- `faraday/faraday-core/src/vault/rekey_screen.rs` and `faraday/faraday-core/src/vault/plan_store.rs` line numbers are not inventory-verified.
