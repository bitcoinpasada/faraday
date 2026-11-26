# The Backups list

The Backups page lists every wallet's backups: for each wallet, the places its
plan keeps things at, what each place holds, its state (written, waiting in the
Outbox, not made, checked, due) and its tag (secret, sealed, public). It also
keeps the record of each wallet's backup across a lock: the marks the person
made, the dated checks of each copy, and the Krux code marks. It is for the
person checking that every wallet is backed up and its copies are checked. It
is reached from Home's Backups tile and from a wallet card's backup line; it
opens a place's checklist item in Back up (`wallet-backup`) or a chart node.

## Screens and sheets

- `Screen::Backups` (`faraday/faraday-core/src/backups/screen.rs:~50`; the compact one-page panel `~250`). List and detail: each wallet's backup map (place, state, what it holds, tag) with Open, Back up or Continue, and Unlock buttons. In compact, Previous and Next page buttons (`~292-318`).
- Screen variants: the list, `BackupsOf(i)`, `BackupsPage(i)`, `BackupsPlace(w, at)` (`faraday/faraday-core/src/backups/mod.rs`).
- Per-place lines (`backups_lines`, `faraday/faraday-core/src/backups/mod.rs:~2171-2241`): tone and state text (see Rules).
- Vault line (`vault_state`, `faraday/faraday-core/src/backups/mod.rs:~2283-2394`): which vault holds the seed, "not verified", locked, "not saved yet"; currency never written, changed since, unchanged.
- A key's backup status line (`BackupRow`, `faraday/faraday-core/src/backup_flow/mod.rs:~2577`): "Not written", "Written to 1 of 2 places", "Written to every place".
- A wallet card's backup line (`backup_line_from`, `faraday/faraday-core/src/backups/mod.rs:~2498-2534`): "Not backed up", "no plan kept"; paper copies counted only when all are Ok. `backup_counts_now` feeds Home's Backups tile.
- No plan shows `backups_no_backup_plan` (`faraday/faraday-core/src/backups/screen.rs:~136-144`, `~268-271`); no wallet shows Make a new wallet and Restore (`~157-176`).

## Actions

- `backups_act` (`faraday/faraday-core/src/backups/mod.rs:~2597-2629`): `BackupsOf` opens `Screen::Backups`; `BackupsPage(k)` chooses a wallet; `BackupsPlace(i, at)` opens a chart node when one applies (`glance::sheet::node_of_now`), else the checklist item (`Screen::Backup`), else `Action::Backup(w)`. Any other action reaching it is `unreachable!()` (`~2626-2627`).
- From the screen: `OpenWallet(i)`, `Backup(i)` (Continue when unfinished), `CreateWallet`, `RestoreWallet`, `Nav` through `e.unlock` (`faraday/faraday-core/src/backups/screen.rs`).
- Checks dated from Back up: "Which copy was this?" (`copy_which(n)`, `faraday/faraday-core/src/backups/mod.rs:~1495`) when several places keep the copy; Set the time when the record's wallet is the one under way.

## Effects

- Vault writes of dated checks: `dated_into` (`faraday/faraday-core/src/backups/mod.rs:~1782`) writes a `PLAN_NOTE` line `checked <place> <thing> <date>` through `vaults.edit` (`~1859`) and then `changed` (`~1865`).
- The screen sets only scroll and region state (`scroll.content_h`, `region_open`, `region_close`, `faraday/faraday-core/src/backups/screen.rs:~101-105`).
- Nothing else leaves the app's state: no stick, camera, clipboard or job.

## State and lock

- `BackupsArea` (`faraday/faraday-core/src/backups/mod.rs:~545-560`): `at`, `records: Vec<Record>`, `older`, `undated`. It is `impl at_lock::Public` (`faraday/faraday-core/src/backups/mod.rs:559`): it holds nothing a lock wipes.
- No secret is held: fingerprints, 32-byte master ids, file SHA-256 hashes and names only (`faraday/faraday-core/src/backups/mod.rs:~1-2161`). `Dated.line` and `Held` hold a place line that begins with the place's name (RAM only).
- Marks: printed templates, envelopes, descriptor shown, passphrase checked (`backups_marked`, `backups_shown`, `backups_passphrase_checked`); the Krux code mark `KefMark` (`faraday/faraday-core/src/backups/mod.rs:~103-125`; `kef_mark` `~1913`, `backups_kef_set` `~1930`).
- `backups_sync` (`faraday/faraday-core/src/backups/mod.rs:~1155`) records the backup under way when its checklist is made; it adopts a record only when exactly one loaded wallet has the checksum (`~1162-1166`). `backup_record_of` (`~1969`).
- Lock: a lock ends the process (`faraday/faraday-core/src/vault/seal.rs:933-946`); the records come back through the kept state `backups` (`faraday/faraday-core/src/memory/mod.rs:493-494`, decode `622`). `encode` (`faraday/faraday-core/src/backups/mod.rs:~378`) and `decode` (`~463`) carry the backup line, `wid`, `spot`, `marks`, `pass`, `check`, `id`, `kef`, `plan`. Not encoded, so lost at a lock: `Record.dated`, `BackupsArea.older`, `BackupsArea.undated`.
- `older` and `undated` are RAM only and go at an app lock, as meant (`faraday/faraday-core/src/backups/mod.rs:549-555`). A held undated check is dated by Set the time, or by a later unlock in the same app of the copy the stick held or of one sealed from it, such as a single vault's lock (`undated_written`, `copy_takes`, `faraday/faraday-core/src/backups/mod.rs:1743-1770`). One no open vault takes stays held.
- `sync` (`faraday/faraday-core/src/backups/mod.rs:~610`): an id the session no longer holds is kept, not dropped (`~633-636`).

## Reach

- Opens: `wallet-backup` (`Screen::Backup` at a checklist item, or `Action::Backup`), a wallet's page, Create and Restore, the chart node sheet (`glance`), vault unlock.
- Opened from: Home's Backups tile, a wallet card's backup line.
- Read by: Back up's checklist (dated checks, marks), `backup-flow` (`seed_stick_name` is called from `faraday/faraday-core/src/backups/mod.rs`), the codex32 checklist item (its press handler lives in this file; see `codex-backup`).
- Reads: `crate::wallet_backup::Tone`, `crate::plan::Tag` (`faraday/faraday-core/src/backups/screen.rs:~11`, `~28`), `crate::panes::screen`, `crate::screens::*`, `app.backups()`, `backups_area`, `list_offset`, vault summaries.

## Rules and refusals

Screen

- Open wallet and Back up appear only for a loaded wallet (`faraday/faraday-core/src/backups/screen.rs:~180-193`). A place press opens its checklist item only for a loaded wallet with a plan (`faraday/faraday-core/src/backups/screen.rs:200-206`). Previous and Next are disabled at the ends (`~292-318`).
- A place row is a touch target tall (fix 3900930e7).

Lines and tones

- An older copy at a place is Warn; a place with no dated check is Ok when checked this power-on, else Warn "not checked"; due checks are Warn; never checked is Warn; an unprotected secret file outside the vault and in the Outbox or a verified receipt is Err (`faraday/faraday-core/src/backups/mod.rs:~2220-2223`, `~2248-2256`). Files: written on the stick (Ok), still in the Outbox (Warn), not made (`~2260-2274`).
- A date after today counts as due (fix 9b4272ff9).
- A seed counts as in a vault only if its 32-byte master id matches (F65); a locked vault's summary matches by id; a fingerprint-only summary reads "not verified"; an unknown locked vault shows locked; none shows "not saved yet" (`vault_state`, `faraday/faraday-core/src/backups/mod.rs:~2283-2394`). `summary_fits` (`~339`): every own seed held, every other vault's seed absent, the wallet id listed when required; `summary_has_seed` (`~356`) is `None` when no id is known.
- A vault is named, and a record offered for loading, only on a whole-id match with the slot's own key (fix 2ba176adc).
- Codex32 share files count as secrets let out (fix 5b36f5ce5).
- `backups_now` (`faraday/faraday-core/src/backups/mod.rs:~1993`): each wallet once by id; a locked vault's summary wallet needs a non-empty sum and id, and an id list of the same length as the sums, else that summary's wallets are dropped; a wallet a vault holds with no plan shows "no plan kept". Per wallet `backup_lines` (`~2086`), `record_lines` (`~2137`).

Dated checks

- `copy_matched(fp, form)` (`faraday/faraday-core/src/backups/mod.rs:~1196`); `copy_checked` (`~692`) is capped at the places keeping the seed; `what_dating` (`~1248`): several places ask "Which copy was this?" through `backup.ask_which`.
- `dated_add_to` (`faraday/faraday-core/src/backups/mod.rs:~1538`): no clock gives an empty date; Set the time is offered only when the record's wallet is the one under way (`~1550-1560`); a check is held as undated only when a stick copy is known.
- `dated_unlock` (`~1709`) and `dated_clock_set` (`~1728`) write only with a non-empty date; `dated_write` filters out written ones (`~1655`).
- `dated_into` (`~1782`): only current open vaults; the wallet id must parse; the vault must keep a plan; a dated place must match a record's place line exactly; else nothing is written.
- A copy's date is kept for that copy alone; a stick is dated when its vault is unlocked from it (F87; fix 28d20547a).
- `vault_stick_dated` (`faraday/faraday-core/src/backups/mod.rs:1332-1360`): skipped when diverged (not current and not an older copy), when the vault did not come from a stick, when no summary matches the vault's salt, when the copy digest is missing; each wallet once by id; a wallet needs exactly one vault it fits (`let [pv]`); a stick dates only when the plan keeps it at exactly one place (`let [p]`); an older copy is marked older instead (`older_mark`, `is_older_at` only for `What::VaultStick`).
- Mark checked never dates a Seed XOR part or a SLIP-39 share (fix d17295cd3).
- `backup_move_on` (`faraday/faraday-core/src/backups/mod.rs:~2434-2470`) does not move on for the quick path, Templates, Envelopes, Sticks, while a codex32 string is open or a which-copy question is open (`~3448-3462`).

Kept-state decode (`faraday/faraday-core/src/backups/mod.rs`)

- Lines that do not read are skipped silently: `spot` needs `at`, `tag` and an open record (`~494-496`); a `kef` line has 7, 9 or 10 fields (`455`), padded with "0" to 10, flags "0" or "1", hex exact, side a u16; `check` count must parse; `id` needs hex4 and hex32; `pass` needs hex4; `wid`, `marks` and `plan` attach only to the last `backup` line.
- `kept_field` replaces tab, LF, CR and US with spaces (`~231`).
- Tag rank `worse`: Secret over Sealed over Public (`~221`); `at_read` and `tag_read` refuse unknown strings; `at_text` maps `At::Stick(p,_)` to `place{p}` (`~244-246`).

## Domain calls

- `faraday_crypto::work::Compute` (summaries, plan map).
- `faraday_vault::records::field::PLAN_NOTE` and the plan record, through `vaults.edit`.
- `faraday_draw` for drawing. Most logic goes through `crate::plan` (faraday-plan) and `crate::glance`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| c101783ae | The one-place warning fired for a 2-of-3 with one seed loaded, saying every secret was at that place. | The warning fires only when one place holds every seed of the wallet (with passphrases), or a seed with its own passphrase. |
| df61fda33 | A codex32 string added to or removed from a vault after the lock was not seen; a string of another set (same key and index) counted as the plan's. | A vault holds exactly the plan's codex32 strings of the plan's set; changes re-seal it. |
| 5b36f5ce5 | A codex32 string's file let out for a stick was not counted as a secret let out. | Codex32 share files count as secrets in Backups. |
| bf8b2bf6d | The backup check for a wallet whose keys must all sign (MuSig2, or a script needing several keys) said any one place could be lost, because it was planned from the wrong seeds. | Such a wallet is planned from the seeds its spend ways need, so loss of one seed is reported as loss. |
| a6a29df1c | The plan's "sees the balance" and "rebuilds" counted a share kept beside the seed of the key it left off as the whole description; Restore does not combine them. | Only the whole description, or shares that together hold every key, count; the sheet names only those shares. |
| 9b4272ff9 | A backup copy whose last check was dated after today never counted as due, so a clock set forward once hid the reminder. | A date after today counts as due. |
| 5df98df9c | A vault the plan names, including one added on the plan or made from Back up, did not get the plan written at the lock. | Every vault the plan names holds the plan with the keys at the lock. |
| 3900930e7 | Backups' place rows were 28 units tall, below the touch target. | A place row is a touch target tall. |
| d17295cd3 | Mark checked could date a Seed XOR part or SLIP-39 share, and Check my copy compared a counted Seed XOR part by its own bytes. | Mark checked never dates a Seed XOR part or SLIP-39 share; a counted part is compared by its fingerprint. |
| f4653d345 | Quick paper backup had no Krux code; a words match opened the code's form behind the Check card, so the next key typed went into the code's passphrase. | The code's form does not open behind Check; Done waits for the check. |
| d4d8e11eb | A wallet name with a # was cut at the # in a sheet's setup file, so a restored multisig came back under a different name. | Names are written without the #. |
| fa702dbd4 | The backup page's account key rows (xpub file, BIP 129 record) showed, and Remove took off, a different file of the same name made by another stick or Create. | A row shows and removes only the file Faraday made for that key, under the name it went out under. |
| 28d20547a | A matched or checked copy was dated in the vault for all copies, and a vault's stick was dated only on creation; the date shown for one copy was wrong. | A copy's date is kept for that copy alone; a stick is dated when its vault is unlocked from it (F87). |
| 2075da49b | After a lock (Write to sticks), the rebuilt Backup showed printed templates and envelopes as not done. | Marks the user made are kept on the wallet's backup record and carried across the lock. |
| 5ccd43ae3 | After a lock, the descriptor shown as a code and a typed-back passphrase check were no longer counted. | Both are read from the wallet's backup record when Backup opens; the passphrase check is written to the record and kept across the lock. |
| eb8395720 | Split backup sheets and the Coldcard multisig config were offered for wallets they could not restore and written unchecked. | Offered only for wallets they restore, and checked before written. |
| 2ba176adc | A locked vault's summary lost each key's 32-byte master id; Backups' kept line and the checklist named a locked vault even when it was not the one; Load from vault offered a record whose key at the slot's path was not the slot's. | A vault is named, and a record offered for loading, only on a whole-id match with the slot's own key. |

## Doubts

- Left out of Fixes, as not about the Backups list: 98b1be3af (Words and SeedQR form pills on the board were under the touch floor; fits `backup-board`); 7c75edc79 and bce9b3c9c (a seed's Krux code picture and `.kef` file in Files get one stem, and a second wallet's code goes beside the first; fit `krux-codes`); 24995bfad (a SLIP-39 group threshold of 1 with more than one group is refused; fits the SLIP-39 set code in faraday-plan and faraday-vault).
- Rows kept here that fit `wallet-backup` as well: f4653d345, d4d8e11eb, fa702dbd4, eb8395720, 5df98df9c, d17295cd3.
- The `kef` kept line: the doc text says eight fields, a comment (`faraday/faraday-core/src/backups/mod.rs:~526-528`) says a line without the hash is dropped, and the code accepts 7, 9 or 10 fields.
- The module doc (`faraday/faraday-core/src/backups/mod.rs:~1-10`) and `Dated` (`~78`) say names never go into kept checks, but `Dated.line` and `Held` carry a place line that starts with the place's name (RAM only).
- `faraday/faraday-core/src/backups/mod.rs` lines 1-60 were seen only in a truncated preview; the drawing in `faraday/faraday-core/src/backups/screen.rs` was read once.
- Doc misplaced at `faraday/faraday-core/src/backups/mod.rs:~4149-4156`.
- Test modules were not read; no fixture is named for this feature.
