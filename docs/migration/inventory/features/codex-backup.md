# Codex32 backup item

The codex32 item of a wallet's backup checklist: for one seed kept as codex32
strings (BIP-93 shares), the person checks each paper string by typing it back
or scanning its Codex32QR, sees the strings and their Codex32QR to copy, says
how many strings exist, and acts on the set (add a share, replace a lost one,
rotate the set, make a new set). It also makes the stick files of the strings
and, inside a wallet backup flow, the round that takes each string into the
added vaults. It is drawn inside Back up's checklist (`wallet-backup`) and has
no screen or sheet of its own besides the scan.

## Screens and sheets

- The item (`item`, `faraday/faraday-core/src/codex32_backup/screen.rs:~176`), one string (`one_string`, `~270`), a set step (`set_step`, `~363`), set actions (`set_actions`, `~491`), marking (`marking`, `~528`), the scheme (`scheme`, `~556`). Drawn from `faraday/faraday-core/src/screens/backup.rs:~1134`.
- Check my copy: typing each string back from paper. Scan my copy: the camera compares the drawn Codex32QR to the string (`Sheet::Scan`, `ScanPurpose::CheckCodex32`).
- Codex32QR shown and copied block by block or row by row (`Qr`, `BlockAt`, `Rows`, `copy_grid`); version-3 Codex32QR with `QR_SIDE = 29` (`faraday/faraday-core/src/codex32_backup/mod.rs:~604`).
- Exist stepper: how many strings exist, from 31 down to k (`Press::Exist`, `faraday/faraday-core/src/codex32_backup/screen.rs:~227-241`).
- Set actions: Add a share, Replace a lost share, Rotate the set, Make the new set; a randomness wait for Make the new set (`~390`, `~420-422`).
- Vault row (`vault_row`, `faraday/faraday-core/src/codex32_backup/screen.rs:~594`), drawn from the vault list: "Share A of fab6868a" or "codex32 name . threshold 2".
- The older copy of a vault sheet (`Sheet::OlderCopy`, `vault_unlock_held`, `faraday/faraday-core/src/backups/mod.rs:~4157-4177`).

## Actions

- Presses go out as `Action::BC32(Press)`. `Press` (`faraday/faraday-core/src/codex32_backup/mod.rs:~30-59`): `Open(u8)`, `List`, `Skip` (the first press asks, the second goes on; `faraday/faraday-core/src/backups/mod.rs:~2686-2705`), `Next`, `Key(OskPress)`, `Qr`, `BlockAt`, `Rows`, `ScanCopy`, `Exist(i8)`, `Set(SetPress)`.
- `SetPress` (`faraday/faraday-core/src/codex32_backup/mod.rs:~63`): `Leave`, `Add`, `Replace(u8)`, `Rotate`, `Mark(u8,u8)` (held, lost, exposed), `Counted`, `Destroyed(u8)`, `K(i8)`, `N(i8)`, `MakeNew`. `SetStep` (`~98`): `None`, `Typing{index}`, `Marking`, `Destroying{held,out}`, `Scheme(k,n)`.
- Press handler `backup_c32` (`faraday/faraday-core/src/backups/mod.rs:~2641-2714`); type back and proof `c32_next` (`~2964-3064`); `c32_go_on` (`~3069`); `c32_set` (`~3098-3190`); `c32_make_new` (`~3197-3296`); `c32_qr` (`~2835`); `c32_show_drawn` (`~2855`); `c32_fix` (`~2756-2829`); `c32_added` (`~3379`); `c32_exist` (`~2720-2749`).
- Typed backup key `backup_name_key` (`faraday/faraday-core/src/backups/mod.rs:~2880-2900`): names a place or types a codex32 string; pins SeedQR through `Action::BPin`.
- In a wallet flow: `c32_round_copy` (`~3654`), `c32_round_wanted` (`~3662`, a toast naming the wanted string), `c32_round_type` (`~3671-3685`, opens `Screen::Entry` through `Action::EntryForm` with `Form::Codex32`), `c32_round_take` (`~3696-3760`), `c32_round_records` (`~3575-3649`: builds `Staged` share records; a missing string stops the round with `Wanted`).
- Older copy: `vault_older_act` (`faraday/faraday-core/src/backups/mod.rs:~4184-4199`): Open is `VaultAction::OlderOpen`; Keep locked closes the copy, drops its key through `copy::drop_wait` and gives up the flow. `vault_older_settle` (`~4205-4222`), `locked_unlock` (`~2577-2590`).

## Effects

- Camera: `c32_scan_open` (`faraday/faraday-core/src/backups/mod.rs:~2905-2921`) queues `CameraOn` with `ScanPurpose::CheckCodex32` on `Sheet::Scan`; `c32_copy_scanned` (`~2927-2954`) queues `CameraOff` when the copy holds (`Found::Holds`) or says the difference (`Found::Differs`). Camera text enters `Strings::scanned` (`faraday/faraday-core/src/codex32_backup/mod.rs:~333`) through `c32_scanned` (`~500`).
- Stick files: `codex32_stick_bytes` (`faraday/faraday-core/src/backups/mod.rs:~3508-3540`) and `c32_stick_file` (`~3875-3908`): the string as one line, or Codex32QR as a labelled PNG, put in the Outbox as a secret (`outbox_secret(OutReason::StickBackup)`, `~3906`; `outbox.retain` drops an earlier file of the same name, `~3904`). File names `codex32-{fp}-{index}.txt` and `codex32qr-{fp}-{index}.png` (`faraday/faraday-core/src/codex32_backup/mod.rs:~749-787`).
- Randomness: `self.fresh_bytes(b"codex32 rotation", ..)` (`faraday/faraday-core/src/backups/mod.rs:~3256`); failure shows `no_randomness`.
- Vault records: share records gathered from a vault (`vault_gather_codex32`, `~4265-4287`); `note_added_keys` (`~4233-4253`) starts the background `worker::Job::MadeOnline`, skipped while the worker is busy; vault summaries merged (`~4019-4147`, `vaults.merge_summaries`).
- No clipboard or printing.

## State and lock
- The backup area's `holds()` counts the Krux code form, the seed on show and the copy check's typed words; the checklist's codex32 `Strings` (typed back, made, held) are not counted, and a lock drops them with the backup state (faraday/faraday-core/src/wallet_backup.rs:5283-5297, 957-960).

- `Strings` (`faraday/faraday-core/src/codex32_backup/mod.rs:~124-167`, `Default`, no `Drop` of its own): `seed`, `made: Vec<(usize, Text)>` (secret), `at`, `typed: Text` (secret), `found`, `skip_ask`, `qr: Option<QrMatrix>` (secret), `shut`, `block`, `rows`, `held: Vec<Text>` (secret), `rebuilds`, `proven`, `aside` (secret), `set: SetStep`, `note`. `proved_strings` (`~180`) and `set_aside` (`~220`) copy secret strings into new vectors. Created by `Strings::new` (`~171`).
- Also secret in the handlers (`faraday/faraday-core/src/backups/mod.rs`): the codex32 parts, a new string, the fix list and the stick bytes (`~2762`, `~2996-3013`, `~3223-3278`, `~3508-3523`); `held`, `pending`, `good`, `texts`, `left` and `typed` in the set state; the round's string from `entry.state()` (`~3698`).
- Lifetime: "kept while the backup is open" (`faraday/faraday-core/src/codex32_backup/mod.rs:~129-130`); owned by `WalletBackupArea.state.c32` (`faraday/faraday-core/src/wallet_backup.rs:~735`). `Strings::list` and `open` clear typed, found, qr, block, shut (`faraday/faraday-core/src/codex32_backup/mod.rs:~273-293`); `forget_typed` clears held, proven, rebuilds (`~212`). `hide_secrets` closes an open string unless something is typed back.
- Lock: `WalletBackupArea::lock` calls `close()`, which sets `state = None` and `shown = None` (`faraday/faraday-core/src/wallet_backup.rs:5283-5298`, `957-960`), so `Strings` goes; the lock then ends the process (`faraday/faraday-core/src/vault/seal.rs:933-946`). `holds()` counts only `kef_holds_secret() || shown || copy-check words`: a `Strings` holding typed codex32 characters or made strings is not counted by `holds()`, though the lock still drops it.
- A flow's wanted string (`Live.c32_wanted`) is not kept across a lock (see `backup-flow`).

## Reach

- Drawn inside `wallet-backup`'s checklist; created by the backup flow and Back up. The vault list calls `vault_row`. The board shows codex32 chips (`backup-board`, `faraday/faraday-core/src/board/codex32.rs`).
- Opens: `Sheet::Scan`, `Screen::Entry` for a string (`Form::Codex32`), `Sheet::OlderCopy`.
- Reads: `crate::plan::codex32::{C32, index_char, CHARSET, index_of, string_of, paper_copy, paper_strings, places_of_string, PaperCopy}`, `crate::plan::{Medium, Writing, What, Shape, Answers}`; `crate::screens::{copy_blocks, copy_grid, CopyGrid, stepper, buttons_and_next, section_label, randomness::row}`; `crate::wallet_backup::WalletBackupArea` (`c32_parts`, `c32_unskip`, `c32_checked`, `c32_skipped`); `crate::wallet::fp_text`; `app.backup()`, `plan_shape`, `plan_mapped`, `place_name`, `waits_for_randomness`, `wait_label`.
- The backup flow calls `codex32_backup::{stick_names, stick_name_is}` and `c32_round_wanted`.

## Rules and refusals

Typing and checking a string (`faraday/faraday-core/src/codex32_backup/mod.rs`)

- Typed-back entry is capped at `form.length - 3`, bech32 CHARSET only, ASCII, lower-cased (`~416-425`); a string is checked only at full length (`~436-437`) and needs an open `at`; a string with no chip (not made here, not 48 long) is never typed blind (`~413-415`).
- `check` (`~646-683`): against a made string it reports the first differing box or a length mismatch; else the string must `Codex32::parse`, and header boxes 4 (threshold equals k), 5 (identifier) and 9 (index) must match, else `c32_not_of_set`.
- `scanned`: the text length must equal `form.length` and parse, else `c32_scanned_not_string` (`~333-343`); a scanned string is marked checked only when it equals a string made here (`~500-523`).
- `prove` (`~549-600`): a secret counts only if it rebuilds the key alone (`is_secret`: box 9 is `s`); a share is never proved by the secret; with `need < 2` any unproved share fails; a joint rebuild when base is below need.
- `c32_next` (`faraday/faraday-core/src/backups/mod.rs:~2964-3064`): a string is proved only when it rebuilds the seed with the strings already proved; on a rebuild failure the held strings are dropped and the item stopped.
- Status reads Checked, Skipped or Not checked as it is; proofs made before another seed was opened still count (fix dd51f1b9c).
- One press of the Codex32QR chip on a shut string opens its QR with the typed text kept as dots; the String chip then shows the text (fix 2cb7a68a4). Any key shows the typed text again (fix 0376eaab4).

Set actions

- Set typing: room `length - 3`, identifier room 4 for Scheme (`faraday/faraday-core/src/codex32_backup/mod.rs:~360-361`); non-CHARSET characters are rejected (`~367-370`); the index must parse (`Codex32::parse` then `index_of`), else `c32_checksum_fails` (`~388-394`); a string counts only if it holds, and is not counted twice (`~397-405`).
- `destroyed`: a new scheme is asked only once every held string is destroyed (`~455-467`). `step_scheme`: threshold 2 to `MAX_THRESHOLD`, strings k to `MAX_SHARES` (`~471-481`).
- `rotation` is refused when the strings out of reach (lost or exposed, plus unknown index) reach `form.need()`, with `c32_rotate_not_enough` (`faraday/faraday-core/src/codex32_backup/mod.rs:700-709`).
- Typing a share needs at least `form.need()` strings (`faraday/faraday-core/src/backups/mod.rs:~3232-3237`); a derivation error gives `c32_set_not_this_seed`; adding beyond the scheme gives the note `c32_set_full` (`~3131-3135`); `Mark(j,m)` with an index not in the marking list does nothing (`~3148-3154`).
- `c32_exist` clamps the count to `[max(known,k), MAX_SHARES]` (`faraday/faraday-core/src/backups/mod.rs:2728-2738`).
- Rotation: the identifier must be 4 bytes, not the old identifier nor a previously rotated one (`c32_set_identifier`, `c32_set_identifier_old`, `faraday/faraday-core/src/backups/mod.rs:3335-3343`); refused in Practice or Demo (`!makes_secrets()`) with no error line (`Err(None)`, `3345-3347`); a plan kept for another key order is not written over (`plan_refuses`, `~3214-3218`, silent `Err(None)`).
- `Paper::Refused(e)` from the keys shows `paper_codex32` (`faraday/faraday-core/src/backups/mod.rs:~3264-3268`).
- Set actions refused past a flow's lock (`c32_lock_refusal`, `faraday/faraday-core/src/backups/mod.rs:~3303-3319`): Add, Replace and Rotate refused at the start; `c32_exist` and `c32_make_new` refused. Locked means a wallet with a made, not Done, flow holding the seed, or the backup's locked flow.
- A destroyed index's codex32 string neither proves, disproves nor is kept (fix 652d82420, in `backup-flow`).

Round in a flow

- `c32_round_take` (`faraday/faraday-core/src/backups/mod.rs:3703-3716` for the first two, `~3703-3746` in all): another set gives `c32_round_other_set`; another share index gives `c32_round_other_share`; no key gives `c32_round_no_key`; a seed that is not the string's seed gives `c32_round_other_seed`; fewer than k strings, or keys that refuse, give `c32_round_unproved`.
- A vault holds exactly the plan's codex32 strings of the plan's set; changes re-seal it (fix df61fda33, in `backups-list`).

Stick files and pictures

- `stick_names`: only for a split set (`k >= 2`), only a seed here, the Codex32QR name only at 48 characters (`faraday/faraday-core/src/codex32_backup/mod.rs:770-776`); `stick_name` only for `Form::Codex32` writings to a file medium (`~749-759`).
- `secret_picture_reads` (`faraday/faraday-core/src/backups/mod.rs:~4295-4321`): true only when the Outbox bytes match exactly and, with one code, the key's words or string read back; more than one code is refused; no key with a code is true (`~4306-4319`).
- `vault_gather_codex32` is refused unless `may_load_keys()`.

Screen

- The Exist stepper and the set actions only when `k >= 2` (`faraday/faraday-core/src/codex32_backup/screen.rs:~227`, `~259`); the keyboard is shown only while the string is being written, hidden once it holds (`~326-336`); MakeNew only when `held.len() >= need`, and it waits for randomness (`~386-394`); Leave is always offered (`~433`).
- A set action's typing is not a string on show (fix 0376eaab4).

## Domain calls

- `faraday_bip::codex32` (`Codex32::parse`, `identifier`, `threshold`, `share_index`, `INDEX_ORDER`, `MAX_SHARES`, `MAX_THRESHOLD`) (`core/faraday-bip/`).
- `faraday_codec::qr::QrMatrix`, `faraday_codec::qr::codex32qr`.
- `faraday_keys::vault::{codex32_share_header, RecordView, Staged}`, `faraday_keys` `Paper::Refused`.
- `faraday_vault::records::field::CODEX32_KEY`.
- `faraday_forms::Form::Codex32`, `faraday_bip::keys::Fingerprint`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 2cb7a68a4 | One press of the Codex32QR chip on a shut codex32 string did not open its QR; the String chip then did not show the text. | One press of Codex32QR opens the QR with the typed text kept as dots; the String chip then shows the text, shut or not. |
| dd51f1b9c | A codex32 string typed back, skipped or made here read with the wrong status; strings proved before another seed was opened did not count toward the rest. | Status reads Checked, Skipped or Not checked as it is; earlier proofs are kept. |
| 0376eaab4 | After the idle warning or a stopped lock, a set action's typing stayed drawn when it should not, and a shut codex32 string was typed blind (no chip, not 48 characters) or failed to show its typed text again after a key. | A set action's typing is not a string on show; any key shows the typed text again (`shut` cleared in `type_in`, `key`, and the backup flow's reset at `faraday/faraday-core/src/backups/mod.rs:~2809`). |

## Doubts

- `c32_exist`'s `clamp` would panic if its lower bound exceeded `MAX_SHARES`; not verified whether that can happen.
- `next_index` (`faraday/faraday-core/src/codex32_backup/mod.rs:~738`) has no caller in the code read. Narrowing `unwrap_or(0)` conversions are silent (`faraday/faraday-core/src/codex32_backup/screen.rs:~222`, `~307-313`, `~404`, `~513`, `~537`).
- Possible panics with unverified callers: `session.keys[key]` at `faraday/faraday-core/src/backups/mod.rs:~3306`, `~3309`.
- Fix 2cb7a68a4's code change is in `faraday/faraday-core/src/backups/mod.rs`; it fits this feature or `backups-list`.
- Test modules were not read; no fixture is named for this feature.
