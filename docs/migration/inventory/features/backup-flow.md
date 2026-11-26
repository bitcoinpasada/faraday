# Backing up a key

The guided backup of a key to sticks. One flow object per key or wallet
(`BackupFlow`, `faraday/faraday-core/src/backup_flow/mod.rs:~1016`) walks a
person through a plan of places and vaults, bringing in the sticks, opening or
making the vaults, making the key (GPG or Secure Boot), "Lock and write", the
write itself and Done. It is for the person keeping a key's copies on several
sticks. Key flows (GPG, Secure Boot) start from Tools; a wallet's flow (kind
`Wallet`) starts from Back up or Make a new wallet and is described in
`wallet-backup`. The same machinery holds the GPG paperkey restore (see
`gpg-keys`), the backup quiz and the stick-write receipt.

## Screens and sheets

- `Screen::BackupFlow` (`faraday/faraday-core/src/backup_flow/steps.rs:~48`, `~69`). Kinds `Gpg`, `SecureBoot`, `Wallet` (`faraday/faraday-core/src/backup_flow/mod.rs:73-80`). Steps `Where`, `Sticks`, `Vaults`, `Make`, `Lock`, `Write`, `Done`, drawn as cards 1 to 7 of the flow container (`flow::contained`, `faraday/faraday-core/src/backup_flow/screen.rs:~44-146`, `~112`).
- Cards the plan skips are not drawn: a wallet's vault setup has only Sticks and Vaults (`faraday/faraday-core/src/backup_flow/screen.rs:~137-139`); no slots means no Sticks, Lock, Write cards (`~143`); no vault used drops Vaults (`~144`).
- The board list pane or panel stands beside the flow when the board is shown (`faraday/faraday-core/src/backup_flow/screen.rs:~67-72`, `~123-128`; see `backup-board`).
- Where card (`faraday/faraday-core/src/backup_flow/screen.rs:~297-475`): places with their vault and stick choices, "Add a place", "Continue to make the key". "Change the plan" is drawn only under the current card: not on Where or Done, not while writing, not in setup (`~273-278`).
- Bring in the sticks card (`faraday/faraday-core/src/backup_flow/screen.rs:~569-1058`): insert, read, pick which place the stick is for, the copy choice, confirm by tap or hold. The medium's word (stick or SD card) comes from `app.medium`. Sticks are numbered: the boot stick is 1, others from 2 in slot order (`faraday/faraday-core/src/backup_flow/mod.rs:~1573-1586`).
- Stick figures (`faraday/faraday-core/src/backup_flow/figures.rs`): `Sheet::{StickSheet, FigMenu, LaterAsk}`; the stick's menu and "Check it later" ask (`Live.fig_menu`, `Live.later_ask`, `faraday/faraday-core/src/backup_flow/mod.rs:~964-968`).
- Checked stick's sheet (`faraday/faraday-core/src/backup_flow/stick_sheet.rs`): a dialog or bottom sheet through `compact::dialog` (`~102`) with name, make, size, serial (`XXXX-XXXX` from a u32 as two 16-bit halves), free space, files with sizes, vault names; icon `Icon::Drive` or `Icon::SdCard` (`~96-99`); title from `figures::name_of`; rows without lines are left out (`~29-32`). Its only press is Close (`Action::Cancel`, `~101`).
- Vaults card (`faraday/faraday-core/src/backup_flow/screen.rs:~1062-1215`): per planned vault, open the existing one (passphrase, Unlock) or create the new one (Create, or the message `create_no_vault_on_stick` when only existing vaults are allowed, `~1097-1105`). "I do not have its passphrase" reveals "New vault beside it" and "Use another stick" (`~1183-1211`). The passphrase field is `pass_field::screen::vault_box` when keys may load, else a stick field with a "pull to unlock" banner (`~1064-1067`, `~1131-1146`). A "never use a real seed" warning is drawn through `never_real_seed_if` (`~1146`).
- Create-vault form: while `Live.creating` is set and `vaults.create` exists, `draw` returns `vault::screen::create` instead of the flow (`faraday/faraday-core/src/backup_flow/screen.rs:~50-52`).
- Make card (`faraday/faraday-core/src/backup_flow/screen.rs:~1237-1415`): the form (name, email and years `V::GpgYears` for GPG; owner name for Secure Boot), the randomness row above Make (`screens::randomness::row`, `~1332`), then the result card. GPG: user IDs, fingerprint, the revocation certificate when kept, Show as QR and PNG (`Action::ShowCode`, `Action::CodePng` with `Code::GpgKey`) only when `gpg::shown` gives a key (`~1366-1401`). Secure Boot: owner GUID and PK, KEK, db names, no buttons (`~1403-1410`). A wallet's keys are not drawn here (`~1412`; the wallet Make card is `wallet-backup`'s). Adding a key to another vault shows only "Copy the key" (`~1265-1286`).
- Lock card (`faraday/faraday-core/src/backup_flow/screen.rs:~1430-1495`): what is kept and what is wiped (`app.kept_wiped(false)`; a wiped line marked "nowhere" in the Error role), then "Lock and write". The key line shows only for a non-wallet with a subject.
- Write card (`faraday/faraday-core/src/backup_flow/screen.rs:~1499-1504`, `slot_rows` `~513-565`): written and pending rows per stick, the stick card. File rows only at Write (`~668-670`); a problem shown only at Write (`~624-628`); several sticks at once warns "one at a time" (`~632-642`); the boot stick ("use this read") is offered only at Write when no slot holds it (`~647-648`).
- Done card (`faraday/faraday-core/src/backup_flow/screen.rs:~1508-1561`): nothing until every slot is written; per-stick written rows; `UnlockDone(v)` for the first used vault ("Unlock for enrolment" for Secure Boot); the lead button is Primary when there is no vault button, else Tonal.
- Other sheets reached: `Sheet::Pull` when a stick arrives at Vaults or Make (`faraday/faraday-core/src/backup_flow/steps.rs:~1775-1778`); `Sheet::GpgRestore` (`~2386`); the Keep sheet for a wallet's copies (`vault::keep::gate`, `faraday/faraday-core/src/backup_flow/wallet.rs:~1496`); `Screen::SetTime` before Make; `Screen::VaultContents` (Done lands on the key's page).
- Layout card (`faraday/faraday-core/src/backup_flow/layout.rs:~17-483`): chips, zones, drag and a menu for the plan's layout.
- GPG restore: the tile dialog (`faraday/faraday-core/src/backup_flow/gpg_restore_screen.rs:~343`: Make a key `FlowAction::MakeGpg`, Restore from a paperkey `FlowAction::Restore(RestorePress::Sheet)`) and the restore sheet (`~275`); a pull-to-unlock banner when keys are not loaded (`may_load_keys`); Restore only when the lines are complete; Scan paperkey only while no line is checked.
- Backup quiz (`faraday/faraday-core/src/quiz.rs`): every word of a mnemonic asked in random order as a choice of four; key generation runs it after the words are shown. Which screen draws it is not named.
- In-progress row: `faraday/faraday-core/src/in_progress.rs:925` shows the flow (string `sidebar_flow_restore`).

## Actions

All presses are `FlowAction` (`faraday/faraday-core/src/backup_flow/action.rs:~9-158`, about 60 variants, data only), unless named otherwise. `RestorePress::{Sheet, From, Field, Next, Again, Restore, Scan, ScanPaperkey}`.

- General: `Back`, `Leave`, `Open(n)`, `ChangePlan`, `Resume`, `BackUpFirst`, `Show(i)`, `MakeGpg`, `Restore(p)`, `WalletLock`, `WalletMake`, `Plan(press)`, `Answer(list,row)`.
- Where: `Destroyed(p)`, `PlaceVault(p,v)`, `AlsoVault(p,v)`, `Together(p)` ("One stick for both", `Plan::one_stick`), `PublicAt(p)`, `RemovePlace(p)`, `AddPlace`, `VaultExisting(v)` (ignored for a found vault), `Continue` (`faraday/faraday-core/src/backup_flow/steps.rs:~295-343`).
- Sticks: `LockBring` (lock, then bring the sticks in: `lock_ask(true)`, `~345`), `SticksOn` (only when `sticks_ready()`), `UseBoot(i)`, `UseBootAt(s)`, `UseAnother`, `Hold`, `Tap`, `UseCopy(bool)`, `PickBeside(v)`, `PickFile(v,k)`, `FigMenu`, `Want`, `LaterAsk`, `LaterDo`. A figure press makes a stick wanted (only at Sticks) or opens its sheet when it is checked with no problem, and un-puts-off a put-off stick (`faraday/faraday-core/src/backup_flow/figures.rs:~93-128`).
- Vaults: `UnlockVault(v)`, `CreateVault(v)`, `NoPass(v)` (toggles), `BesideIt(v)`, `AnotherStick(v)` ("Use another stick" clears that vault's places and returns to Sticks, `flow_another_stick`); `Action::Vault(V::FocusPassphrase)`, `V::Unlock`.
- Make: `Make`, `MadeContinue` (to Done with no slots, else Lock, `faraday/faraday-core/src/backup_flow/steps.rs:~382-390`), `CopyKey`, `TypeString`, `Action::Vault(V::FocusField(k))`, `V::GpgYears`, `Action::ShowCode`, `Action::CodePng`.
- Lock and Done: `LockWrite` (`flow_lock_write`), `UnlockDone(v)` (`flow_unlock_done`, `faraday/faraday-core/src/backup_flow/steps.rs:~3267`, lands on the key's page). A written row on Done presses `Back`. Stick sheet: Close.
- Trail Back and Forward over the cards: `impl StepFlow for FlowSteps` (`faraday/faraday-core/src/backup_flow/steps.rs:~3595`, `Open(k)` `~3599`). No Back while a vault is being created or a stick written; after a change of Where only steps up to Make remain.
- Leaving: `on_screen = false`, `flows.on = None`; the flow stays in `flows.all` with its `live` (`faraday/faraday-core/src/backup_flow/steps.rs:~309-315`). Back with nothing before it lands on Tools (`Screen::Catalog`, the Tools page) for a key, Home for a wallet (`Kind::left_for`, `faraday/faraday-core/src/backup_flow/mod.rs:85-91`).
- GPG restore: type or scan paperkey lines (`push`, `hand_in`, `paperkey_code`), Restore.
- Quiz: `pick(slot)`, `confirm`, `retry` (`faraday/faraday-core/src/quiz.rs:166-215`).

## Effects

- Storage commands: `StorageCommand::StickSpace` and `Read{stick,name}` when reading starts (`faraday/faraday-core/src/backup_flow/steps.rs:~890-897`); `Write` (`~2894`, `~3110`; vault files replace, others go beside, `OnTaken::{Ask, Beside}`); `ReadQr` for read-back of pictures (`~3002`). Answers `StorageEvent::{Written, WriteFailed}` (`~2950`).
- Inbox and Outbox: `flow_tidy` removes a dropped vault's items from both (`~579-673`); `flow_take_copy` pushes a vault copy read off a stick into the Inbox and calls `save_boxes()` (`~1756-1757`), giving a different vault under the same name a free name (`~1732-1744`); `save_boxes()` after each written stick (`~3199`) and after finish (`~3264`); `flow_finish` (`~3214`) moves sealed vaults from Outbox to Inbox and public files to Inbox, and clears `public` unless Secure Boot.
- Vault store: `vaults.close_at`, `edit`, `changed`, `bump`, `read_free`, `start_work(Work::FlowMake)`, `stretch::defer(Work::Unlock)` (`~1988`), `begin_create`, `unlock_with`, `store().stage`, `seals_write`.
- Locking: "Lock and write" calls `idle.set_lock_for_flow(true)` then `self.lock()` (`faraday/faraday-core/src/backup_flow/steps.rs:~2446-2447`). A lock ends the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Receipt: `visit.set_receipt` after a written stick (`flow_written_done`, `~3126`); time from `checked_now()`, `clock_now()`.
- Key making: GPG is made at once; Secure Boot and RSA keys go through the vault worker (`vaults.start_work(Work::FlowMake)`, `~2196`). Randomness through the randomness row.
- The camera, for Scan paperkey in the GPG restore (`RestorePress::ScanPaperkey`). No clipboard or printing in the flow.
- Toasts for refusals (`faraday/faraday-core/src/backup_flow/steps.rs:~2182-2354`).
- Costly work takes `c: &Compute`: `BackupFlow::with_plan`, `with_wallet`, `new_setup`, `sync_slots`, `valid`; decoding costs work because `valid` calls `sync_slots` (`faraday/faraday-core/src/backup_flow/mod.rs:~2500`); `sha256_of` of written files and pictures; `gate_restore`, `restore_finish`, `RestoreFrom::read` (SHA-1), `RestoreLive::new`, `place_setup`.

## State and lock

- `Faraday.flows` is `BackupFlowArea` (`faraday/faraday-core/src/backup_flow/area.rs:~21-59`): `all: Vec<BackupFlow>`, `on`, `made_here`, `copied`, `sealed_for`, `owner_seq`, `board_list_offset`, `restoring: Option<Box<RestoreLive>>`, `layouts: Vec<SetupLayout>`, `restore_rows`.
- `BackupFlow` (`faraday/faraday-core/src/backup_flow/mod.rs:~1016-1052`): kind, step, plan, slots, made, made_in, subject, sealing, public, adding, on_screen, wallet, parked, restore, live, keeps.
- `Plan` (`faraday/faraday-core/src/backup_flow/mod.rs:~416-446`): `places` (vault indexes per place), `vaults`, `joined`, `public`, `files`, `place_files`, `public_stick`, `together`. `VaultPlan` (`~389-404`): `salt`, `from`, `existing`, `trying`. Default key plan `Plan::key_default` (`~463`). A wallet plan is drawn from the answers and the map (`plan_of` `~591`, `plan_of_on` `~596`).
- Sticks: `Slot` (target, files, bound, written, later), `Bound` (label, make, serial, listing, vault reads with SHA-256, picks, declined, boot, at, confirm), `Reading` (purpose `Bind` or `Check`, files, picks, problem, short, differs, hold, chosen, checked, first), `Writing`, `Written`, `Problem`, `Confirm::{Tap,Hold}`, `Pick::{New,Over}` (`faraday/faraday-core/src/backup_flow/mod.rs:~187-931`).
- `Live` (`faraday/faraday-core/src/backup_flow/mod.rs:~931-983`): screen state of a flow: reading, writing, `c32_wanted`, creating, unlocking, scroll, stick sheet, menus, board.
- Secrets held:
  - Vault files read off a stick (`ReadFiles`, `Reading.files`, `faraday/faraday-core/src/backup_flow/mod.rs:~805-841`); their bytes move into an Inbox item in `flow_take_copy`.
  - Plain files waiting to be written (`Writing.held`, `sends`, `~873`; `faraday/faraday-core/src/backup_flow/steps.rs:~2757`), including a wallet's seed files.
  - `Live.c32_wanted` (a codex32 string).
  - `Built` (the made key's record `faraday_keys::vault::MadeKey`, subject, name, outbox) and staged vault records in wallet copies.
  - GPG restore: `RestoreLive` (`faraday/faraday-core/src/backup_flow/gpg_restore.rs:~187-210`): `fingerprint [u8;20]` (public), `set: faraday_keys::gpg::Restore` (checked lines, secret), `typed: SecretText` (the line being typed), `error`, `refused`, `in_vault`, `focused`, `held`.
  - The Make form fields and vault passphrase are drawn here but held by `vault` and `pass_field`.
- Public by design: `Plan`, `Slot`, `Bound` (labels, SHA-256 of vault bytes, names), `Written`, `Sealed`, `VaultPlan.salt`.
- Quiz state holds word indices only and clears its choices on a pass (`faraday/faraday-core/src/quiz.rs:166-215`). The receipt holds names and SHA-256 hashes only; it stays across a lock and goes at power-off (`faraday/faraday-core/src/receipt.rs:~7-9`).
- Leaving the screen drops nothing of `live`, except that `flow_goto` clears `live.problem`, `no_pass`, `not_it`, `unlocking`, `reading` (`faraday/faraday-core/src/backup_flow/steps.rs:~102-117`), and `flow_sticks_changed` drops a reading whose stick is gone or whose problem is NotFat, ReadOnly or Unreadable while unbound (`~1814-1821`). A pulled reading is dropped; a pulled writing stick becomes `Problem::Pulled` (`~1765-1829`).
- Lock: a lock ends the process and a new one starts (`faraday/faraday-core/src/vault/seal.rs:933-946`); what survives is only what the kept state encodes.
  - Before the wipes, `lock_flows` (`faraday/faraday-core/src/vault/seal.rs:652-659`) calls `flow_before_lock` (`faraday/faraday-core/src/backup_flow/steps.rs:2455-2490`), which keeps the makers (`flow_keep_makers`), wipes the GPG restore's typed lines (`restore_wipe`), and resets each flow's step and `on_screen`. It does not drop `live.reading`.
  - Step reset: Lock goes to Write (or Vaults if `adding`); Make goes to Write if made and not adding, else Vaults; a made flow at Write whose used vaults lack a sealed copy goes to Vaults with `adding` set; Write with no slots goes to Done (`faraday/faraday-core/src/backup_flow/steps.rs:~2461-2488`). `on_screen` is set only for the flow on screen (also over Set the time).
  - `BackupFlowArea`'s own `AtLock` (`faraday/faraday-core/src/backup_flow/area.rs:790-799`): `holds` is "a GPG restore is under way"; `lock()` calls `restore_wipe` (`faraday/faraday-core/src/backup_flow/gpg_restore.rs:583-589`): the restore's lines are wiped and `restoring = None`; the certificates stay with their flows.
  - Kept state: `encode` (`faraday/faraday-core/src/backup_flow/mod.rs:~1710-1934`, `VERSION = "3"` `~1701`) is stored under `backup-flow`, and `gate_encode` (`~2549`) under `backup-gate`, inserted first (`faraday/faraday-core/src/memory/mod.rs:506-515`, `620`). Nothing in it is secret; it is gone at power-off. Encoded: kind, step, made, adding, subject, on_screen, plan, made_in, slots (target, files, bound, written), sealing, public, restore, the wallet part.
  - Lost at a lock: all of `Live` (reading, writing with its held plain files, `c32_wanted`, creating, unlocking, scroll, stick sheet, menus), `parked`, `keeps`, `WalletFlow.owner` (`faraday/faraday-core/src/backup_flow/mod.rs:~1045-1051`). `decode` sets `live = Live::default()` (`faraday/faraday-core/src/backup_flow/mod.rs:1973`). `Bound.make` is defaulted on decode (`~2296`).
  - `flow_restore` (`faraday/faraday-core/src/backup_flow/steps.rs:3549-3566`) reads the kept state back, gives every wallet flow a fresh owner id, toasts `backup_flow_dropped` if flows were dropped, calls `flow_makers_return`, and reopens the flow that was on screen.
  - The quiz zeroizes on a pass and on drop; it is not named in the lock list.

## Reach

- Opens: `Screen::SetTime`, `Screen::VaultContents`, `Sheet::Pull`, `Sheet::GpgRestore`, the Keep sheet, the vault create form, the randomness row, Home or Tools on Back.
- Opened from: Tools (GPG and Secure Boot key flows), Back up and Make a new wallet (wallet flows, `wallet-backup`), Vault contents (GPG Restore), the in-progress row, the kept state after a lock.
- Reads and calls: `crate::plan` (Spot, Shape, Answers, Mapped, Seed, At, What, Writing, Medium, KeyForm, `sticks_at`, `stick_vault`, `public_stick_on`, `map`, `holds_at`, `place_map`, `codex32::{Wanted, vault_marks}`); `crate::vault` (keep sheet, `plan_store`, `screen::{create, hold_button_in, stick_banner, stick_field, text_box, years_row, key_fingerprint, words_identity}`, `stretch::defer`, `CreateForm`, `Work`, `faraday/faraday-core/src/vault/seal.rs` `lock_flows`); `crate::pass_field` (`vault_box`, `Which`), `crate::demo::PassField::Unlock`; `crate::gpg` (`shown`, `DEFAULT_YEARS`, `gpg_build`, `gpg_put_public`); `crate::secure_boot` (`shown`, `OWNER_FIELD`, `sb_build`); `crate::board::screen::{list_pane, list_panel}` (`backup-board`); `crate::codex32_backup` (`codex-backup`); `crate::visit` (receipt, `vault_from_find`, `clear_pull`; `stick-visit`); `crate::inbox` (`Item`, `file_stem`); `crate::storage_api`; `crate::idle`; `crate::set_time`; `crate::flow::{contained, Page, Scroll}`; `crate::trail::{StepFlow, Steps}`; `crate::compact::dialog`; `crate::picture::labelled_png`; `crate::MAX_VAULTS = faraday_plan::MAX_VAULTS` (`faraday/faraday-core/src/lib.rs:734`).
- Reached by: `faraday/faraday-core/src/memory/mod.rs:506-515` and `620` (kept state); `faraday/faraday-core/src/vault/seal.rs:659`; `drawing::View` and `board::screen` call `draw`, `steps_of`, `card_title`, `card_name`, `summary`, `result_rows`; `faraday/faraday-core/src/backups/mod.rs` calls `seed_stick_name`; the Create and Restore screens call `create_cards`, `setup_vault_open`, `setup_pending`. Zeroing a stick or deleting a file calls `flow_copy_gone_for` (`delete-and-zero`).
- The key's backup status line `BackupRow` (`faraday/faraday-core/src/backup_flow/mod.rs:~2577`) is read by `backups-list`.

## Rules and refusals

Plan and places

- Minimum places: 2 for GPG and Secure Boot, 1 for a wallet (`Kind::min_places`, `faraday/faraday-core/src/backup_flow/mod.rs:122-129`). `flow_continue` refuses a non-wallet with fewer stick places than the minimum (`faraday/faraday-core/src/backup_flow/steps.rs:716-718`) and a wallet with its empty vault set (`705-712`).
- Remove place (`flow_remove_place`, `faraday/faraday-core/src/backup_flow/steps.rs:422-486`): wallet places only when `destroyed` (`426-428`); only a place whose slots' written state equals `destroyed` (before the write at once, after the write only as "destroyed") (`434-437`); refused when stick places would fall to or below the minimum (`438-440`); refused when the made key would be left in no vault it was made in (`441-446`). Orphaned public files move to the first remaining slot; with no slot left they are not attached (`478-482`). The Where card shows "Destroyed" and "Remove" only when sticks exceed the minimum (`faraday/faraday-core/src/backup_flow/screen.rs:327-334`, `377`).
- `Plan::toggle_place_vault` returns false rather than drop a place's last vault stick (`faraday/faraday-core/src/backup_flow/place.rs:40-43`). `Plan::remove_place` moves later places up (`~74`).
- `place_vault` is refused for a wallet, a bad place, a place with any written slot, or when the key would be lost (`faraday/faraday-core/src/backup_flow/steps.rs:488-517`); `also_vault` and `together` likewise (`521-572`); `together` needs a place with two vaults.
- `stick_target` is None for a stick index past the place's sticks (`faraday/faraday-core/src/backup_flow/place.rs:95-97`). `Target::Files` and `Target::Public` cannot be a `vault_stick` (`faraday/faraday-core/src/backup_flow/mod.rs:335-354`).
- A secret file never shares the public files' stick; public files beside an encrypted vault copy are allowed (`faraday/faraday-core/src/backup_flow/mod.rs:641-647`).
- Where card gates: "Also keep" only when the place has a vault and it is not already there (`faraday/faraday-core/src/backup_flow/screen.rs:351`); "Both" only with more than one vault (`359`); "Public here" only when a vault is kept (`366`); the existing or new switch only when the vault is not found on a stick (`428`); "at least N sticks" warning and Continue disabled below the minimum (`456-464`).
- Plan presses are refused while `live.writing` is set: ChangePlan, AddPlace, RemovePlace, Destroyed, PlaceVault, VaultExisting, Continue, Together, AlsoVault, PublicAt, Answer, Plan (`faraday/faraday-core/src/backup_flow/steps.rs:~275-293`). Back and Leave are refused while writing (`faraday/faraday-core/src/backup_flow/steps.rs:305`).
- Layout card (`faraday/faraday-core/src/backup_flow/layout.rs`): `new` and `set_vaults` clamp to `1..=MAX_VAULTS` (`40-45`, `112-118`); `put` is refused when `v >= vaults`, the stick is out of range or already there (`138-144`); `move_to` is none when the place is the same; `place_setup` (`~427-482`) does nothing if the plan already matches, forgets a stick read for a place that now holds different vaults, never changes a found vault, resets answers to defaults with `custom = true`, and goes to Sticks.

Sticks and reading

- Only `.kdbx` names are read; a file that does not fit free memory is left unread and noted as `short` (`faraday/faraday-core/src/backup_flow/steps.rs:838-845`). A boot-labelled stick is `Problem::Boot` (`faraday_boot::layout::is_boot_label`); a stick the shell reported read-only or not FAT is `ReadOnly` or `NotFat` (`878-887`). Boot sticks are never candidates (`742-746`); a boot stick holding another place is refused (`811-813`).
- `flow_examine` runs only at Sticks or Write, only when `clean()`, with exactly one candidate stick, not in `live.ignored` (`faraday/faraday-core/src/backup_flow/steps.rs:765-797`). `flow_read_answer` keeps a file only if `faraday_vault::file::inspect` accepts it as a vault (`924`). In `flow_space_answer` a size of 0 means free space unknown.
- `flow_after_read` (`faraday/faraday-core/src/backup_flow/steps.rs:1012-1043`): a stick matching another bound slot by label, serial and non-empty listing is `BoundTo`; at Write a stick this flow wrote is refused. Two copies of one vault on a stick is `TwoCopies` (`1088-1093`, `1366-1376`). An unfound existing vault with nothing offered is `Memory(why)` if a file was unread for memory, else `NoVault(v)` (`1132-1140`). At Write a differing copy is declined unless it is the one sealed from (`1101-1114`). A file another vault of the plan has is not offered (`1183-1199`, `1384-1391`).
- `flow_bind_ids` (`faraday/faraday-core/src/backup_flow/steps.rs:~1950-2028`): a file that is another vault of the plan is named and refused (`Live.not_it`); an open vault with the same id is closed only if the unlock just made it and it has no changes; a known vault's tried file is unpicked; an unknown one asks for the passphrase.
- A file is proved at unlock (by the recorded hash or its id); a file nothing proves goes beside and is never replaced (fix 95e9a0a40).
- `flow_picks_done`: `NoRoom(needed, free)` when free room is smaller than needed; an overwrite (`Pick::Over`) counts twice the vault's length (`faraday/faraday-core/src/backup_flow/steps.rs:1511-1558`).
- `flow_confirm` (`faraday/faraday-core/src/backup_flow/steps.rs:1595-1629`): needs `clean()`; refused with a problem, when already bound or for a differing copy; a Hold must match the required kind; every vault of the slot must be picked; the Check purpose needs a Hold at Write. A stick matching another bound slot needs a Hold (`1577-1591`). `flow_press_file` is refused at Write (`1422-1425`).
- `flow_bind`: the boot stick is never pulled; its reading ends at bind or goes on to the write (`faraday/faraday-core/src/backup_flow/steps.rs:1684-1685`).
- Sticks card gates: only the "Lock and bring in" button when the device is not clean (`faraday/faraday-core/src/backup_flow/screen.rs:571-584`); Continue live only when `sticks_ready()` (`599-603`); the Check purpose shows hold and "second/another" by copy count (`758-791`); "Use another" only with a Tap confirm (`1003-1015`); "Use this file" hidden at Write (`1041-1053`).
- Figures: the boot stick is offered only while waiting, when a boot stick is in and no slot holds it; "Check it later" only if `can_put_off` (step Sticks, not setup, no read in progress, slot unbound, not already later, a vault stick whose vaults are all new) (`faraday/faraday-core/src/backup_flow/figures.rs:~93-128`).
- Stick sheet: draws nothing with no flow, no stick index (`live.stick_sheet`) or an index past the slots (`faraday/faraday-core/src/backup_flow/stick_sheet.rs:~89-94`).
- A stick file is the flow's own only with its stem and extension, or a numbered copy `stem-N.ext` (`own_name`; fix 1cd960e95). `rename_written` renames only files of the same label and serial. `holds_written` (`faraday/faraday-core/src/backup_flow/area.rs:~640-672`) refuses a stick holding a file another stick's flow wrote (`HasFirst`, `HasFilesOf`) by SHA-256 or listing name.

Vaults and Make

- `flow_create_first` starts creating a vault by itself only when no vault is open, nothing is creating or unlocking, the flow is not existing-only, and no needed vault is on a stick (`faraday/faraday-core/src/backup_flow/steps.rs:~230-254`). The new vault takes the plan's slot and is unlocked at once with the passphrase typed twice (`flow_create_start`, `flow_vault_made`).
- Make refusal (`flow_make_refusal`, `faraday/faraday-core/src/backup_flow/steps.rs:2150-2168`): a vault still to unlock gives `backup_flow_unlock_first`; a GPG key is refused only when both the name and the email field are empty.
- `flow_make_ask` (`faraday/faraday-core/src/backup_flow/steps.rs:2171-2196`): no key if one is already made, for the wallet kind, or when `!makes_secrets()` (Practice); `time_asked()` stops Make until the time is set; no clock gives the toast `secureboot_this_computer`.
- `flow_make_work` (`faraday/faraday-core/src/backup_flow/steps.rs:2201-2232`): a vault not unlocked gives `backup_flow_pull_to_unlock_toast`; the wallet kind and an empty form return silently.
- `flow_save_built` (`faraday/faraday-core/src/backup_flow/steps.rs:2237-2290`): `vault_refuses` is checked for every open vault first; every vault takes the key or none does.
- `flow_copy_key` (`faraday/faraday-core/src/backup_flow/steps.rs:2317-2366`): no subject returns; no holder open or `copy_out` None gives `backup_flow_unlock_first`; each added vault must be open and pass `vault_refuses`.
- A vault closed in the id binding closes its nonce history in the same step (fix ecf9098df).

Write

- `flow_write_go` (`faraday/faraday-core/src/backup_flow/steps.rs:2746-2812`): only when `clean()` and at Write (`2746-2750`); a vault file whose exposure is Secret is refused (`NotChecked`); a plain file whose exposure is Secret is sent only as a wallet's seed file on the stick it is kept on, never the public stick (`seed_file_ok_for`, `files_stick`, `public_slot`; `2798-2812`); with nothing to send it refuses.
- `flow_after_check` (`faraday/faraday-core/src/backup_flow/steps.rs:2715-2785`): another flow's stick is refused (`holds_written`); the stick must read as before except the flow's own sealed bytes and `.part` files; label, serial and listing must match; else `NotRead(target)`.
- `flow_write_check_copies` (`faraday/faraday-core/src/backup_flow/steps.rs:~2795-2830`): a later copy, or a copy of a vault the person declined, waits for a Hold; `Problem::NoRoom(need, free)` when need exceeds free.
- `flow_write_answer` (`faraday/faraday-core/src/backup_flow/steps.rs:3015-3040`): a failed plain file is written again once; a failed vault file ends the write (`NotChecked`). `flow_write_again` (`3085-3123`): a second failure is `NoReadBack`. `flow_qr_answer` (`3052-3080`) counts a picture when the codes match or the seed or codex32 read-back matches. `flow_name_taken` (`~3125`): Cancel gives `NameTaken`.
- A plain file is held until the vault files sent with it are written; a plain file that does not read back is written again once (`faraday/faraday-core/src/backup_flow/mod.rs:856-873`).
- Secure Boot enrolment files go out only in the last write (`faraday/faraday-core/src/backup_flow/mod.rs:1603-1615`). A restored key does not wait (`1595-1596`).
- `flow_written_done` (`faraday/faraday-core/src/backup_flow/steps.rs:~3126-3176`) builds the receipt with every file `verified: true`.
- `flow_copy_gone_for` (`faraday/faraday-core/src/backup_flow/steps.rs:~3380-3420`): a deleted file or a wipe reopens that stick's card and clears `Written`; the name match is case-insensitive.
- A time for a place comes only from a checked clock (fix bec2ba6f0).
- "Lock and write" (`flow_lock_write`) acts only at `Step::Lock` (`faraday/faraday-core/src/backup_flow/steps.rs:~2442-2447`).
- An accepted plan can always be written; public files never share a stick with a secret file (fix af03eebe9).

Kept state, decode and validity

- An unknown version line drops all kept flows (`faraday/faraday-core/src/backup_flow/mod.rs:1944-1946`). A line that does not read spoils its flow; a flow failing `valid` is dropped (`1983-2001`). Unknown kind, step or target codes are refused (`73-80`, `182-184`, `304-331`; target "another" with n=0 is refused at `320`). Hex fields must decode (`2079-2108`, `2343-2415`). Index limits: a vault index must be next in order and below `MAX_VAULTS` (`2249-2254`); place below 10000; slot below 20000; a wallet shape with a vault count above `MAX_VAULTS` is refused (`2141-2146`).
- `valid` (`faraday/faraday-core/src/backup_flow/mod.rs:2459-2545`): plan indices in range; each placed stick named in the flow must be one the plan gives; picks must name plan vaults; a made flow needs a subject; an adding flow must be made; Lock or later and Write or later need a made key; a made flow past Write with an unsealed vault goes to Vaults with `adding` set; Done with an unwritten stick goes back to Write, and is refused outright if not made (`2531-2541`). A made wallet flow with no wallet part or no checksum is not kept as a flow; its gate entry holds its keys instead (`2459-2467`).
- `put_back` and restore of a flow only with an owner, and only where a Create flow of that owner is not made at the same step (`faraday/faraday-core/src/backup_flow/area.rs:~330-370`).
- `gate_restore` (`faraday/faraday-core/src/backup_flow/area.rs:~529-582`) skips lines with missing fields or an unknown kind code, adds held tokens to a matching flow; a line with two possible flows or none becomes a new made, unwritten flow (the key waits).
- `public_at` has no effect for wallet flows or out of range. Sealing marks per file (`faraday/faraday-core/src/backup_flow/area.rs:~206-271`). `drop_flow` (`~374-378`) wipes the restore's lines first.

GPG restore (see `gpg-keys` for the GPG key itself)

- The restore takes the key's public certificate (an Inbox GPG item that reads as a public key; unreadable ones are shown with why; refused rows sorted last) plus typed or scanned paperkey lines.
- `push` (`faraday/faraday-core/src/backup_flow/gpg_restore.rs:286-298`): a character must be a hex digit, a space or a colon (a second colon is refused), else `EN.gpg_restore_chars` and the line is kept; no input once the set is complete; a line ends by itself when its digits equal 2 times the expected count plus 6.
- `hand_in` ignores an empty line and a CRC mismatch on a line that ended by itself; `paperkey_code` takes a whole scanned code; `restore_typing` acts only for a flow at Make, not made, not adding, with lines incomplete.
- `restore_finish` (`faraday/faraday-core/src/backup_flow/gpg_restore.rs:~564-579`) refuses if an open vault already holds the key; the restored key record is shown under its first user ID and nothing is put for the stick (the certificate is already out).

Quiz

- `pick(slot)` only checks a slot, and is ignored unless Asking and `slot < 4`; `confirm` answers the checked slot and returns false if none is checked; a wrong answer moves to `Wrong`; `retry` asks the same word with fresh decoys; the last right answer passes (`faraday/faraday-core/src/quiz.rs:166-215`).
- Decoys share the first letter where one exists, else are random, never already on screen. The list is cut silently to `MAX_WORDS`; a zero seed is replaced by a constant; the generator is xorshift64* seeded from 8 bytes of `fresh` and is stated not to be cryptographic (`faraday/faraday-core/src/quiz.rs:~9-11`, `~81-127`).

Receipt

- A heading line (local time from the timezone minutes) and one row per verified file (name, SHA-256); only verified files count; tabs and newlines are stripped on encode; decode skips file lines whose hash is not 32 bytes or that come before a `receipt` line; a bad time gives no time; a second `receipt` line replaces the first (`faraday/faraday-core/src/receipt.rs`).

## Domain calls

- `faraday_crypto::work::Compute`, `faraday_crypto::hex::{decode_exact, encode_string}`, `faraday_crypto::hex`.
- `faraday_keys`: `vault::{RecordView, gpg_facts, Staged, VaultId, MadeKey}`, `UnlockParts`, `Part`, `Reveal` (`BackupStick`, `VaultEntry`, `PaperForm`), `KeyInfo`, `Paper::Refused`, `gpg::{Restore, Restored, LineError, LineRefused, CodeRefused}` (`faraday/faraday-keys/src/`).
- `faraday_vault`: `records::{kind, field}` (GPG, SECURE_BOOT, KEY, PLAN, WALLET, WALLET_SHARE, SLOT_LABEL, ...), `Record`, `file::inspect` (`faraday/faraday-vault/src/`).
- `faraday_pgp::{Certificate::read, fingerprint, subkey_fingerprint, user_ids, created, expiry, revoked_at, set_aside, read_public_armor}`.
- `faraday_bip::{keys::Fingerprint, policy::WalletId}`.
- `faraday_codec::{qr::QrMatrix, vault_file::is_vault_file}`.
- `faraday_boot::layout::is_boot_label` (`faraday/faraday-core/src/backup_flow/steps.rs:~878`).
- `faraday_plan::{MAX_VAULTS, hex32, read_hex32}`, `faraday_shell_api::{Key, Command}`, `faraday_text::{Text, text!}`, `faraday_forms::Form`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 7f4a3fa23 | On the wide layout a stepper's long label ("How many strings of … are there?") ran under the − and + buttons, so part of it could not be read or pressed. | Labels clear the buttons' press boxes; the label moves to the buttons' left on wide layouts; buttons keep 40 dp size and 48 dp press areas. |
| af03eebe9 | A backup plan accepted at the lock could not be written, or its public files shared a stick with a seed file. | Public files never share a stick with a secret file; an accepted plan can always be written. |
| 1cd960e95 | A file on a stick was taken as the flow's own by name alone; a flow whose vault was unsealed at lock did not go back to its vaults. | A stick file is the flow's own only with its stem and extension (or a numbered copy); the unsealed vault flow returns to its vaults. |
| aedbdbd62 | After a lock, a GPG or Secure Boot keys flow's Back read Home, not Tools; with nothing before it, Back from these flows went to Home. | Back from a Tools-started flow lands on Tools, including after the lock's restart. |
| e00205b22 | Writing a stick showed in the link colour, as if it were a tappable link. | A state reads in the text colour, with the work-under-way indicator along its foot. |
| d10ebf773 | A wallet's backup page was headed "Bitcoin wallet" while the rail's row read "Backup of <wallet name>". | The page is named as its rail row names it: Backup of and the wallet's name. |
| 3ae18d136 | A backup sheet the backup flow wrote to the stick did not show "Written to the stick" in its row; the checklist counted it done. | The sheet's row says Written (by the flow as well as by the receipt), matching the checklist. |
| 30b7a51f1 | The chart's I destroyed it on a kept old Seed XOR part could write a vault without the part, or wrongly write vaults. | It waits for an open Back up's own save, writes every open vault that keeps the plan, keeps each vault's other lines, and a kept part counts only for its own old set. |
| ef8069a8c | The chart's I destroyed it on a kept old part wrote only some vaults, changed the device's record even when no vault held the change, and did not take the part out of an open Back up; it counted a vault record's extra lines from the wrong plan. | Only vaults that keep the plan are written; the device's record changes only once a vault holds it; an open Back up drops the part; the record's spots use its own plan. |
| 8a9450198 | With Back up open at the plan, I destroyed it on a kept part could leave the part's notes behind when no vault could be written. | The open Back up's save drops the part's notes even when no vault was written at the press. |
| f8c207ab9 | The chart's kept old Seed XOR part carried a different number from the one Back up's section gives, and its sheet's hint did not fit at 480x640. | One function gives the number for both places; the hint fits. |
| bec2ba6f0 | Back up's flow recorded a time for a place from an unchecked clock, and written files carried such a time. | A time comes only from a checked clock. |
| 19de8802c | Paper only kept the words alone for a single key even where the key's seed did not give the wallet back, and then offered a wallet sheet and Write to sticks. | Paper only keeps the words alone only for a single key whose seed gives the wallet back; no wallet sheet and no Write to sticks otherwise. |
| 652d82420 | A tried file given up offered no Open again and kept its stick's pick; a read-only file with history asked a seal question; a destroyed index's codex32 string still proved, disproved or was kept. | A destroyed index's codex32 string neither proves, disproves nor is kept; a read-only file opens with no seal dialog. |
| 1def03a2f | At 480x640 the "Which vault is Vault 1?" screen cut each file's row short of the part that tells two files apart. | A row wraps and grows with its line. |
| 95e9a0a40 | The backup flow could replace a stick's file that nothing proved was the vault's, by reading it before any vault was unlocked. | A file is proved at unlock (by the recorded hash or its id); a file nothing proves goes beside and is never replaced. |
| 9bf09d71c | Two copies of one vault, learned across two locks, were shown as Vault 2 being "not that vault" with nothing pressed. | The two copies are one vault's record; Open Vault 2 is not offered. |
| d35ddc4ba | A seed's Krux code dropped out of the backup plan and the chart after a lock and an unlock that did not reload the seed. | The Krux code stays with its seed across lock and unlock. |
| 7b6a4c779 | On the small panel, the New / Existing choice of a vault on the layout step did not fit when "Already on ..." was too wide. | The choice reads "New" and "Existing" where it does not fit. |
| 655f52209 | A saved backup plan with a seed ticked in a vault row, or a passphrase row for that seed, showed no plan at all while that seed's key was not loaded. | A plan with an absent seed still shows: the seed reads as absent, its ticks and passphrase row are kept and written back at the next save; a plan of the wrong shape is still refused. |
| c78a22348 | The backup page offered paper Seed XOR for a key on a non-English word list; a press that got there gave an error. | Seed XOR is not offered for a non-English list; the page says "Not shown: this key cannot be written in this form"; Add a key restores Seed XOR parts on English only. |
| ecf9098df | A vault closed in the backup flow's id binding (not the vault asked for) kept the nonce history open, so the history compared against a closed vault for the rest of the event. | The nonce history is closed in the same step as every other close. |
| d43313d75 | Saving the backup flow for a sortedmulti wallet with its keys swapped overwrote a shares plan kept for the other key order, dropping its notes (lost, exposed, checked). | The save writes nothing and says the plan is kept in another order; the user loads the wallet in that order. |
| 0b669e128 | On windows wider than 1600 dp, opening the board's card made Back up and a wallet's backup flow jump. | Both pages stand still: the same reading column at the same x and width whether the board is open or not. |
| 5d6cd029d | A key loaded here from SLIP-39 shares or a master seed was read by the backup plan as on its own device or as never lost. | Such a key is counted as kept as shares outside the plan, counting for no check, with a line saying so. |
| 90c4949a8 | Restoring from seed alone did not say which account and path were assumed; a backup plan keeping only the seed read as rebuilding the wallet at another account or script. | Seed-alone restore states the assumed account and path; the plan records `seed_alone_fails` so a seed-only plan is not read as a rebuild at another account or script. |
| 523ad2e6e | Three places decided what to show by reading displayed English text: the backup chart's spot colour, the wallet card's source tag, and Family's route. | Decisions are made from data (a `plan::AloneKind` carried beside the line), never from displayed text. |
| 6bc0f5bcf | A place card on the backup board, and a wallet-card chart spot, said "Nothing" where the finder can neither see the balance nor spend. | The "found alone" verdict reads "Cannot see balance" for that state. |
| a03053bd2 | A backup plan saved with a vault or stick row numbered at the top of the integer range overflowed in a checked addition (panic with overflow checks). | A plan row past what a count can hold is not a plan: the text reads as absent, nothing panics. |
| 9ac5de9df | A plan with more than 64 seeds was read, though which seeds are in hand is one 64-bit mask. | Plans of more than 64 seeds are refused when read. |
| 701604d8c | On a hand-out of shares over more than 16 places, the board could say a plan rebuilds when it had not been checked (a step budget ran out); "Take it off every place" was offered for an unchecked plan. | A plan is reported as rebuilding only when checked whole; the check runs above 16 places too. |

## Doubts

- Left out of Fixes: 4108e848f (the app did not build outside the tests because a pill press used a test-only accessor); a build-only fix with no behaviour to restore.
- Many rows given to this feature concern the wallet's Back up page, the backup plan or the board rather than the key flow: 30b7a51f1, ef8069a8c, 8a9450198, f8c207ab9, 19de8802c, c78a22348, d35ddc4ba, d43313d75, 5d6cd029d, 655f52209 fit `wallet-backup`; 523ad2e6e, 6bc0f5bcf, a03053bd2, 9ac5de9df, 701604d8c fit the plan and `backup-board`; 7f4a3fa23 (a stepper in `faraday/faraday-core/src/screens/parts.rs`) may be the codex32 count stepper. They are kept here as given. Whether 19de8802c is a fix or a plan change is not settled. 9bf09d71c changed no non-test file.
- Files of this area read only in part: `faraday/faraday-core/src/backup_flow/action.rs`, `faraday/faraday-core/src/backup_flow/area.rs`, `faraday/faraday-core/src/backup_flow/figures.rs`, `faraday/faraday-core/src/backup_flow/gpg_restore.rs`, `faraday/faraday-core/src/backup_flow/gpg_restore_screen.rs` were covered only at the points cited; their rules beyond those are missing.
- The lock paths other than "Lock and write" (idle lock, a manual lock) were not traced for what `Live.writing` and `reading` do when a lock comes during a write. The Lock card reaches a lock only at `Step::Lock`, with no write under way.
- Whether the restart after "Lock and write" always goes through the kept-state path was not traced in `lock()` itself; only `lock_flows` and the comments were read.
- `flow_written_done` records every file `verified: true`; this is right only while callers always run the checks first (they do in the paths read).
- Orphaned public files when no slot is left (`faraday/faraday-core/src/backup_flow/steps.rs:478-482`) are not attached to a slot and not removed from the Inbox; whether they stay visible in Files is not known.
- The GPG restore doc says the Restore row is dimmed in Practice, but no dimming code was found in `faraday/faraday-core/src/backup_flow/gpg_restore_screen.rs` (another part of the inventory places it at `faraday/faraday-core/src/backup_flow/steps.rs:~2384`).
- The quiz's `random_word_except` would not end if the word list held fewer than four words (not checked). Which screen draws the quiz is not named. Whether a second `receipt` line is ever written is not known.
- Possible panics with unverified callers: `faraday/faraday-core/src/backup_flow/place.rs:~39`, `~48` (`self.places[p]`); `faraday/faraday-core/src/backup_flow/steps.rs:~1386`, `~1438`, `~1488`, `~1506`, `~1654` (`plan.vaults[v]`); `texts[0]`, `texts[1]` in `flow_make_work` (`~2214-2222`); `faraday/faraday-core/src/backup_flow/layout.rs:~132` (`self.on.len() - 1`) and a dead `else` at `~171`.
- Dead or odd code: `let _ = before;` (`faraday/faraday-core/src/backup_flow/mod.rs:~2499`, `~2509`); `let _ = w;` after `take()` in `flow_sticks_changed` (`faraday/faraday-core/src/backup_flow/steps.rs:~1807`); stale docs (`flow_save_built` "costs a hash of each public file"; `flow_take_copy` says Files but pushes to the Inbox). Both this module and the app have an `is_hold` (`faraday/faraday-core/src/backup_flow/action.rs:121`, `faraday/faraday-core/src/action.rs:1152`).
- Decisions F6 and F31 are cited in comments without their meaning.
- Test modules were not read by the inventory; no fixture is named for this feature.
