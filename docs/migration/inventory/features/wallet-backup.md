# Backing up a wallet

Back up: the backup of one loaded wallet. It has two stages: the plan (what is
kept where, drawn on the backup board) and the checklist (numbered items, each
with its card: templates, sticks, copies, passphrase, Seed XOR parts, codex32
strings, Krux code, vault, seed files, sheets, descriptor, public files,
envelopes). A quick paper path backs up a single key as words. When the plan
keeps anything on sticks or in vaults, a wallet backup flow (`BackupFlow` of
kind `Wallet`, see `backup-flow`) sets up the vaults, locks and writes. It is
for the person keeping a wallet's keys and description. It is reached from a
wallet's page, the Backups list, the wallet done card of Create and Restore,
Make a new wallet, and a vault's contents.

## Screens and sheets

- `Screen::Backup`, drawn by `faraday/faraday-core/src/screens/backup.rs`. No backup open shows only a bar with Back to Home; no wallet draws nothing (`faraday/faraday-core/src/screens/backup.rs:~214-233`). The Back arrow goes to `Nav(Screen::VaultContents)` when opened from a vault, else `OpenWallet(b.wallet)` (`backup_back`, `~325-331`).
- Plan stage (`plan_draw`, `faraday/faraday-core/src/screens/backup.rs:~246-321`): plan cards, the map of what is kept where, "Make the checklist" (`Action::BChecklist`); the board list beside it (`board::screen::list_pane`, `~313-315`; see `backup-board`). Step flows `BackupSteps` (`faraday/faraday-core/src/wallet_backup.rs:~3671`, `BACKUP_STEPS` `~3674`) and `BackupPlanSteps` (`~3678`).
- Checklist stage: one card of numbered items with done marks (`checklist_draw` `faraday/faraday-core/src/screens/backup.rs:~336-401`, `checklist_card` `~406-464`); each opens its card (`Action::BStep(it)`). Card titles come from `faraday/faraday-core/src/wallet_backup/heading.rs` (`item_heading`, empty when no backup or the plan is not mapped; `templates_heading`).
- Items:
  - Templates: 12 or 24 words (`BWords`); sheets are printed from Faraday Starter on the online computer. With no word sheets only codex32 or Krux code lines (`faraday/faraday-core/src/screens/backup.rs:~72-81`).
  - Sticks (`sticks_item`, `~1986-2063`, foot `Flow(WalletLock)`): stick rows, a wiped thing with nowhere to keep it as Error (`~2003-2009`).
  - Copy: words or SeedQR, type back (`BCheck`), scan (`BScan`), keep words only (`BKeepWords`), "Which copy was this?" (`BWhich(n)`).
  - Passphrase: reveal (`BReveal`) and a typed check.
  - Seed XOR parts (`parts_item`, `paper_section`, `BXor(2|3)`, `BPaperHide`), old parts and old SLIP-39 shares kept lines (`old_shares`, `old_parts`, `xor_not_taken`; actions `BSlipDestroy/Keep`, `BOldDestroy/Keep`).
  - Codex32 strings: drawn by `codex32_backup::screen::item` (`faraday/faraday-core/src/screens/backup.rs:~1134`; see `codex-backup`).
  - Krux KEF code (`kef_code_block`, `seedqr_block`; see Krux code below).
  - Vault item (`BVaultSave`, `BNewVault`, `BVaultLoad`), with a load-at-unlock checkbox (`~2158-2174`, F62).
  - Seed files (`BFile`).
  - Sheets: the backup PDF (`BOut(3)`), share sheets (`BOut(4)`), the shares slider and audit.
  - Show descriptor (`QrWallet`).
  - Public files (`public_files`, `faraday/faraday-core/src/screens/backup.rs:~2656-2818`): descriptor, wallet JSON, multisig config, backup PDF, BSMS, Bitcoin Core import, per-key cosigner files and BSMS records; More options: Core before 29, Electrum, Caravan, Specter DIY, sp() descriptor, Wasabi. Make (`PublicOut`, `WalletKeyOut`, `WalletKeyBsms`, `SDescriptorOut`), remove (`PublicRemove`), view (`ShowCode`, `CodePng`). See `public-files`.
  - Envelopes (`BNext`).
- Wallet chip and picker when several wallets are loaded (`wallet_pick`, `faraday/faraday-core/src/screens/backup.rs:~610-650`, `Backup(k)`, `BWallets`; list at least 340 wide, `~616`). Set the time button in the foot (`OpenSetTime`).
- Quick paper backup (`faraday/faraday-core/src/screens/backup/quick.rs`; logic in `faraday/faraday-core/src/wallet_backup/quick.rs`): cards "Write the words down" (`Step::Write`, `QUICK_WRITE` 200), "Check my copy" (`Step::Check`, `QUICK_CHECK` 201), Krux code (`Step::Code`, `QUICK_CODE` 202): Show the seed, SeedQR, Blank template (`template_sheet` dialog, `Sheet::QuickTemplate`), Krux code, check, "I wrote them down", Next, Done, More ways.
- Wallet done card (`wallet_done`, `faraday/faraday-core/src/screens/backup.rs:~714-870`, `From::{Create, Backup, Restore}`), shared by Create, Restore and Backup: Back up now, Show wallet QR, Open the wallet, Lock, Power off (Quit online), Change the plan, Make a wallet from the same key (`NewWalletFrom`), Back up (`BQuick(Quick::Open)`).
- Sheets: `Sheet::Scan` (`ScanPurpose::CheckCopy`, `CheckKef`), `Sheet::KefOptions`, `Sheet::QuickTemplate`, the Keep sheet in wallet copies.
- Shown seed on the copy page (`faraday/faraday-core/src/wallet_backup/shown.rs`): `SeedShown { key, numbers: Vec<Text>, qr: Option<(bool, QrMatrix)> }`, drawn while the eye is open.
- Wallet backup flow cards (`faraday/faraday-core/src/backup_flow/wallet_screen.rs`): plan card `where_body` (`~30`): plan question cards (`PlanPress::Q(q)`), Continue only when enabled, replaced by a held line (a disabled pin when `ui.pinning`) or a stickless warning (`~57-73`). Make card `make_body` (`~120`): "Add from copy" (`Action::Entry(None)`) when a seed is kept only on paper, "Type this string" (`FlowAction::TypeString`) for a codex32 share the copy stopped on, "Copy key" (`CopyKey`), "Continue" (`MadeContinue`), "Make the keys" (`WalletMake`) before the keys exist. Plan cards via `crate::plan_cards_of` (`~31-51`).
- Make a new wallet cards (`create_cards`, order Kind, Quorum, Where, How many, Sticks, Vaults, Backup, Keys, `faraday/faraday-core/src/backup_flow/wallet.rs:~2463-2500`).

## Actions

- `backup_act` (`faraday/faraday-core/src/wallet_backup.rs:~2038-2347`): `Backup(i)` (refused for `i >= wallets.len()`; reopens a half-way quick backup; else loads the last plan or starts the board; the template count is 12 words if every seed has at most 12, else 24), `BackupChecklist(i)`, `BackupFirst(w)`, `BQuick(..)`, `BKey(k)`, `BQ(n)`, `BWallets`, `BReveal`, `BPaperHide`, `BCompact`, `BPin`, `BRows`, `BWords` (to `BackupState::edit`, `~1131-1181`), `BQNext`, `BAnswer(list,row)`, `BName`, `BChecklist`, `BPlan`, `BStep`, `BNext`, `BXor(n)`, `BOldDestroy/BOldKeep`, `BSlipDestroy/BSlipKeep`, `BPartCheck(i)`, `BCheck`, `BScan`, `BCheckClear`, `BKeepWords`, `BWhich`, `BC32(p)`, `BVaultSave`, `BVaultLoad`, `BNewVault`, `BFilesMake`, `BFile`, `BOmit(n)`, `BOut(what)`.
- Typing: `name_key` (`faraday/faraday-core/src/wallet_backup.rs:~1225-1268`): a place name; Enter and Escape close; arrow keys move the grid pin; codex32 keys go to `c32_key`. `check_key`, `copy_check_key` for typed-back words and passphrase.
- Quick: `Quick::{Open, Card, Show, Wrote, Done, Template, SeedQr, Code, Next, Copy, More}`.
- Krux code (`faraday/faraday-core/src/wallet_backup/kef.rs`): `KefAct::NotOnPaper`, `ScanCopy`, dice presses, typing (only on `Screen::Backup`), Escape closes Options only.
- Flow presses for a wallet: `flow_plan_press` (`P::Q`, `P::Next`, `P::Answer`, `P::Omit(n)`, `faraday/faraday-core/src/backup_flow/wallet.rs:~2557-2585`); `flow_wallet_edit` (`~2858-2944`) and `flow_wallet_add_place` (`~2947-2952`); "Add a passphrase" on the plan's check lines (`plan_add_passphrase`, `~2959-2990`) sets `Screen::Create` on the Keys card; `WalletLock` (`wallet_lock_write`, `~996-1213`); `WalletMake`.
- Quick SeedQR (`faraday/faraday-core/src/wallet_backup.rs:5207-5217`): with a plan map, puts the seed's SeedQR on it first (refusal stops the action); then `quick_seedqr` sets the SeedQR answer, the Write step, clears scan and check, reveals the seed (`faraday/faraday-core/src/wallet_backup/quick.rs:91-107`) and the plan is saved.

## Effects

- Camera: `copy_scan_open` (`faraday/faraday-core/src/wallet_backup.rs:~2460-2466`: `camera.open`, `Sheet::Scan`, `Command::CameraOn`); Krux code scan (`ScanPurpose::CheckKef`, `Command::CameraOn/Off`).
- Randomness: `paper_xor` draws `fresh_bytes(b"seed xor", seed_xor_random_len(parts))`.
- Vault writes: the plan is saved as vault record type 11 (`faraday_vault::records::kind::PLAN`) in open vaults (`plan_save_with`, `faraday/faraday-core/src/vault/plan_store.rs`); `vault_push_in`, `plan_take`, `plan_restore`, `seals_write`; `vault_lock_one(..After::CreateFrom(Screen::Backup))` for `BNewVault`; the dated checks as `PLAN_NOTE` lines (see `backups-list`).
- Session keys: `session.keys.replace_xor`, `convert::set::record`.
- Outbox: `put_outbox` (`faraday/faraday-core/src/wallet_backup.rs:~3605`); a wallet flow's seed files as secret Outbox items (`outbox_secret(OutReason::StickBackup)`, `faraday/faraday-core/src/backup_flow/wallet.rs:~779`, `put_outbox` `~886`); public files (`public_files::checklist_start`).
- Key reveals for seed files: `session.keys.reveal(k, Reveal::BackupStick(Part::Words|Passphrase|SeedQr))` (`faraday/faraday-core/src/backup_flow/wallet.rs:~920-951`); for the copy page `Reveal::CopyByHand`, `Reveal::SeedQr`.
- Lock and write for a wallet locks the device (see `backup-flow`).
- Toasts. No clipboard or printing (sheets are printed from the Starter).

## State and lock
- Picking another seed in the checklist clears what was shown, checked, scanned or split for the one before; the parts matched (`paper_ok`) are read only while parts are on screen, and the SeedQR scan misses are kept per seed fingerprint (faraday/faraday-core/src/wallet_backup.rs:1115-1126, 1859, 1878).

- `WalletBackupArea` (`faraday/faraday-core/src/wallet_backup.rs:~389-400`) holds `state: Option<BackupState>` and `shown`.
- `BackupState` (`faraday/faraday-core/src/wallet_backup.rs:~196-371`): wallet, stage (`BStage::{Plan, Checklist}`), question, plan answers, place names (vault only), open item, `reveal`, typed check (`check: Option<WordCheck>`), `scanned`, copy records (`checked`, `misses`, `kef_misses`, codex32 `c32`, `c32_checked`, `c32_skipped`, `c32_base`, `c32_made`), Seed XOR parts `paper: Option<Held<Vec<Text>>>` with `paper_ok`, `paper_skip`, `parts_checked`, old parts kept, vault records (`before`, `prior`, `extras`, `no_load`), quick flags, `keeps`, `board`, `edits`, `then_item`, `sent`, `shown: Option<Box<SeedShown>>`, `xpub_names` (public), `more_ways` (public).
- Secrets held: `paper` (Seed XOR parts), `c32`, `c32_base`, `c32_made` (codex32 strings), `check` (typed words and an optional passphrase `PassField`), `scanned`, `kef` (Krux code forms), `shown` (the shown seed and its SeedQR).
- `WalletFlow` (`faraday/faraday-core/src/backup_flow/mod.rs:~1057-1110`): `shape`, `answers`, `checksum`, `id`, `name`, `held`, `sealed`, `before`, `others`, `base`, `again`, `readback`, `owner`, `setup`, `pieces`, `custom`. Public: answers, before, others, base, held (fingerprints and tokens `k{fp}`, `p{fp}`, `a{fp}`, `s{id}:{n}`, `w{sum}`). Constants `WALLET_MARK=1_000_000`, `PASS_MARK=2_000_000`, `SHARE_MARK=3_000_000`, `OMIT_MARK=4_000_000`, `PASS_ALONE_MARK=5_000_000`, `OTHER_WALLET="*"` (`faraday/faraday-core/src/backup_flow/wallet.rs:3024-3037`).
- Lock (`faraday/faraday-core/src/wallet_backup.rs:5283-5297`): `holds()` is "a Krux code is held, or `shown` is set, or words are typed back in the open check"; `lock()` calls `close()` (`957-961`), which sets `shown = None` and `state = None`: the whole `BackupState` goes (paper, codex32 strings, check, answers, marks). `drop_paper` (`~965-972`): only a lock drops the Seed XOR parts. A lock then ends the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- `hide_secrets` (`faraday/faraday-core/src/wallet_backup.rs:975-986`): `reveal = false`, closes an open codex32 string unless something is typed back, drops the shown seed; keeps the typed check, `paper` and the checked lists.
- `shown_tidy` (`faraday/faraday-core/src/wallet_backup.rs:~2012-2035`): the shown seed is held only on `Screen::Backup` with reveal on and no check open; any other state drops it (and Explore's state, `explore_left`). `SeedShown` is dropped on hide, page left or lock (`faraday/faraday-core/src/wallet_backup/shown.rs`).
- `key_removed` (`faraday/faraday-core/src/wallet_backup.rs:~997-1028`): ends the open item, drops codex32 strings of removed seeds, clears check, scan, reveal; if the backup's own key is gone also clears `paper`, `paper_ok`, `paper_skip`, `quick`, `quick_shown`. `pick_key` clears check, reveal, scan, paper but not `paper_ok` or misses. `wallet_left` (`~1030-1051`) renumbers xpub names and ends the backup if it was of that wallet.
- Typed words are cleared after compare on every path read (`faraday/faraday-core/src/wallet_backup.rs:~2593`, `~2631`, `~2650`, `~2777`); the typed passphrase is cleared by `passphrase_differs`, `clear_typed` and `check_close` (`~1931`, `~1987-1991`).
- Krux code forms: `kef_holds_secret`; `dropped` closes each form; `stick_attached(wipe)` wipes what was typed when a stick is attached; `hide` closes the passphrase eye (`faraday/faraday-core/src/wallet_backup/kef.rs:~317-335`); `kef_drop_gone` drops forms whose key is no longer loaded; `tidy` closes forms when the page is left, except for Randomness (`~615-617`).
- Across a lock: the backup's record and marks (templates, envelopes, descriptor shown, passphrase checked) are kept on the Backups record (`backups-list`; fixes 2075da49b, 5ccd43ae3 in that feature). The wallet flow survives through the backup flow's kept state; `flow_keep_makers` (`faraday/faraday-core/src/backup_flow/wallet.rs:~3199-3237`) lets the keys typed or made so far in Make a new wallet go with the lock and keeps only the `Again` record (`restore`, `kind`, `m`, `n`, `backup`) on the flow (`~3240-3298`); `flow_makers_return` hands the flow back to Create or Restore after the lock.
- After a lock, the vault load and then the checklist: `wallet_flow_land` (`faraday/faraday-core/src/backup_flow/wallet.rs:~1400`), `Action::BackupChecklist` (`~1425`).

## Reach

- Opens: `backup-board` (the plan), `codex-backup` (the codex32 item), `krux-codes` (the Krux code form), `public-files`, the backup flow (`backup-flow`), Set the time, Create's Keys card, Entry (`Action::Entry`, `Screen::Entry` for a codex32 string), the vault create and load, the camera scan.
- Opened from: a wallet's page, `backups-list`, Create and Restore (the wallet done card), Make a new wallet, Vault contents.
- Reads: `crate::{plan, codex32_backup, kef, kef_form, vault, vault::keep, restore, copy_check, quick, shown, board, glance (plan_of_now), create::NewKind, public_files, public_kind, spend_ways}`; `crate::create` (`NewKind`, `Source`, `BackupForm`, `create.set`, `create_open_keys`) and `crate::restore` (`RestoreState`, `open_after_setup`, `sheets_rebuild`) from the wallet flow; `restore::repeats_a_seed` (`faraday/faraday-core/src/wallet_backup.rs:183`).
- Writes the Backups record (`backups_sync`, `backups-list`).

## Rules and refusals
- A wallet's share record is made from the loaded wallet, else from a wallet record, plan record or description file in an open vault or Files that reads as the same id; where none is found or the throwaway read fails, no record is made and no message is shown (faraday/faraday-core/src/backup_flow/wallet_files.rs:365-430). A held share token of unknown wallet id matches by a 64-character id of the same number (wallet_files.rs:340-357).
- Making Seed XOR parts again for a key that already has a set: refused with "no more old parts" when the plan has no old set for the seed or cannot keep another old set (faraday/faraday-core/src/wallet_backup.rs:2872-2875).

Checklist and plan

- An item waiting on an undone one takes no press, and its line names what it waits for (`faraday/faraday-core/src/screens/backup.rs:452-454`). The Set the time button shows only when a match is undated and the clock is not checked (`~472`).
- `BChecklist` (`faraday/faraday-core/src/wallet_backup.rs:2190-2214`): past the lock gives the toast `backup_flow_past_lock`; a copy kept nowhere, a vault with no place, or `plan_refuses` returns silently (a comment calls the empty-vault case "a warning only" but the code returns); on success the plan is saved, then `public_files::checklist_start`.
- `BNext(n)` needs the item done, except Envelopes and Templates; `BStep` only if reachable; `BNewVault` toasts `backup_flow_set_up_holds_other` when the set-up vault already holds the other seeds; `BName` only if a vault is open; `BOmit` while a locked flow is open goes to `flow_plan_press(Omit)`; `keep_words_only` is refused by `plan_refuses`, or unless the item is a Copy including SeedQR.
- `BAnswer` takes only `qrow::SOFTWARE` and `qrow::FORM`; a locked flow routes it to `flow_wallet_answer`. A place name refuses control characters and 32 or more characters (`name_key`, `faraday/faraday-core/src/wallet_backup.rs:~1225-1268`).
- Templates offers only 12 and 24 words (`faraday/faraday-core/src/screens/backup.rs:1030`). `BWords(n)` has no range check.

Copy and passphrase

- Copy item: no key returns (`faraday/faraday-core/src/screens/backup.rs:~1074-1076`); words and SeedQR are not drawn while a check is open (`~1118`, `~1671`); "Keep the words only" only when the seed has a SeedQR copy and the item is not done, offered once misses reach `copy_check::MISSES`, with a scans-without-match line (`1403-1406`, `~1923-1941`).
- SeedQR is refused for non-English lists (`backup_seedqr_english_only`, `faraday/faraday-core/src/screens/backup.rs:1870-1888`); Seed XOR parts are refused for non-English lists (`~2273-2285`).
- Passphrase: shown only while `reveal` is on and no check is open; with no passphrase kept the line reads "no passphrase kept" (`faraday/faraday-core/src/screens/backup.rs:1670-1681`). A typed passphrase check draws as a code field only if `pass_is_public`, else a masked box (`~1345-1355`). Typed words are masked cells (`~1580-1586`, `~1608-1614`). The receive address is withheld when `receive_held(wl)` (`~1484-1487`).
- `copy_check_after` (`faraday/faraday-core/src/wallet_backup.rs:~2646`) compares only once all words are typed; a match closes and records; a wrong word is named (`WrongWord` or `AnotherSeed`) and typing resumes. The passphrase check is exact (`passphrase_matches`); a mismatch wipes it and shows `copy_check_passphrase_differs`.
- `part_check_counted` compares a Seed XOR part by fingerprint with an empty passphrase (`faraday/faraday-core/src/wallet_backup.rs:~2531`); `part_check_open` is refused if a check is open or the part number is past the count; `key_copy_scan` only for a key with words.

Seed XOR

- `paper_xor` (`faraday/faraday-core/src/wallet_backup.rs:2788-2810`): in Practice or Demo (`!makes_secrets()`) returns silently; no words toasts `paper_no_words_to_split`; a non-English list gives `convert_not_given`; no randomness gives `no_randomness`; then `keys.seed_xor(&c, .., Reveal::PaperForm)`.
- `paper_xor_count` (`faraday/faraday-core/src/wallet_backup.rs:~2833-2890`): the toast `paper_old_too_many` and no change when the key has an old set but the plan cannot keep it, or when the plan has no old set for the key (`2868-2876`: `old.is_none() || make_new_parts(..).is_none()`).
- Old SLIP-39 shares kept line only for this seed (`faraday/faraday-core/src/screens/backup.rs:~2453`); the whole-set rebuild line only if all parts are kept (`~2567-2579`).

Vault item and plan save

- Vault item: a warning when the open vault holds another vault's seed (`faraday/faraday-core/src/screens/backup.rs:~2097-2104`); the unlock button only when there is no vault yet (`~2177-2184`); the action is "Lock new vault" when the target is taken, else "Save into" (`~1794-1801`).
- `backup_vault_fits` and `backup_open_fits` (`faraday/faraday-core/src/wallet_backup.rs:~3412-3495`): a vault locked since power-on counts only when its summary fits; vaults holding shares or passphrases count as done only while open; no seed of another vault; codex32 strays are refused.
- Plan save (`plan_save_with`, `faraday/faraday-core/src/vault/plan_store.rs`; `faraday/faraday-core/src/wallet_backup.rs:~4570-4976`): open vaults only; each target vault must take its record and seal lines (`room_for_seals`) or the save is refused with the toast `vault_fault_wallet_why`; `plan_save_refused` gives `plan_store_other_key_order`; with no target vault and answers that keep a plan it returns true and writes nothing. In the `no_room` closure (`~4677-4681`) a vault with seals but no matching record skips the room check.
- A save or a lock is refused for a wallet whose keys are swapped (a shares plan kept for the other key order) before any vault gets a record or a codex32 share is asked for; each vault keeps one current plan for the wallet (fixes 5e729c7e7, 4911dc5c9).

Codex32 and parts on the checklist

- The codex32 item's action is the first string not checked, skipping those held by another (`faraday/faraday-core/src/screens/backup.rs:~1753-1770`); Parts: the first unchecked part (`~1779-1787`). Details in `codex-backup`.

Sheets and public files

- Share sheet row only when every share sheet is made (`faraday/faraday-core/src/screens/backup.rs:~1249-1252`); the shares slider's "most keys left off" is `min(m-1, keys_n-1)` (`~918-919`); omit is clamped; the share audit warns when a quorum does not rebuild (`~964-994`). The flow's omit is clamped the same way (`faraday/faraday-core/src/backup_flow/wallet.rs:2580-2583`).
- Public files: a held silent-payments wallet gives no QR code for its descriptor; the multisig config only when `multisig_config` is `Some` (`faraday/faraday-core/src/screens/backup.rs:~2692`); BSMS only for wsh and sh(wsh) (BIP 129, `~2712-2718`); no Bitcoin Core import for silent payments (`~2731`); per-key BSMS only for Multi and MultiNested (`~2748-2749`); the sp() descriptor drawn as a secret sheet (`~2798-2801`); remove only while waiting for the stick (`~2853-2856`); codes only where the plan's form has a QR (`~2835-2851`).
- `rebuilt_as_policy` (`faraday/faraday-core/src/wallet_backup.rs:~181-200`) refuses a wallet with two keys of one seed (`restore::repeats_a_seed`, `183`); only `sortedmulti` in `sh`, `wsh`, `sh(wsh)`, or Taproot with sorted tapscript and an unspendable key path rebuilds.
- `config` (`faraday/faraday-core/src/wallet_backup.rs:~242-282`) uses `00000000` for a missing fingerprint and `m` for a missing path; `config_name` strips `#`; `sanitize` keeps ASCII graphic characters and space, 20 characters; `wallet_json` blockheight 0; a silent-payments wallet has one key line and no per-address lines; `public_more_files` lists only files the wallet writes; BIP 129 record only for `Multi` and `MultiNested`. Addresses 0/0, 0/1, 0/2, 1/0 are costed by `Compute` (`~397-399`).
- File names: `{stem}-descriptor.txt`, `-multisig-config.txt`, `-wallet.json`, `-bsms.txt`, `-bitcoin-core.json`, pictures, `xpub-<fp>.txt`, `xpub-<fp>-bsms.txt`, `{stem}-{fp}-words.txt`, `-seedqr.png`, `-compactseedqr.png` (`faraday/faraday-core/src/wallet_backup.rs:~4372-4388`, `~4455-4458`). An account key file is reported only if an Outbox entry carries the name (`~3623-3626`, `~3638-3642`). A row shows and removes only the file Faraday made for that key (fix fa702dbd4, in `backups-list`).
- Items past 255 are dropped silently (`faraday/faraday-core/src/wallet_backup.rs:~3751-3753`).

Wallet done card

- Lock and Power off are drawn only when `from != From::Create`; Power off reads Quit online (`faraday/faraday-core/src/screens/backup.rs:850-866`). A restored wallet shows "not recorded", not "not made" (`~779-783`); Back up now is pinned on a small panel (`~816-820`); "Make a wallet from the same key" only on restore with a `NewKind` (`~847-853`); Back up (restore) only when not backed up (`~840-846`).
- The wallet chip shows only when several wallets are loaded (`~265`, `~349`); Backup from Create has no tap (`~281-287`).

Quick paper backup

- `quick_takes` (`faraday/faraday-core/src/wallet_backup.rs:~5097-5143`, `quick_try` `~5068`): only at Plan with the board, one key with words, no passphrase, not shares, not `seed_alone_fails`, no vaults, not split, not "more ways", no codex32, Seed XOR or SLIP-39 set, exactly one backup key, fingerprint not yet checked and `Copy(0)` present. `Show` needs quick; `Wrote` needs quick and shown; `Done` only when `quick_matched_now`; `SeedQr` needs `quick_put`.
- `seed_alone_gives` (`faraday/faraday-core/src/wallet_backup.rs:~5017-5038`) refuses silent payments, not single-key, not exactly one key, no path, or a path that is not native SegWit account 0 of the session network.
- Screen: SeedQR and the seed grid only while revealed and no check is open (`faraday/faraday-core/src/screens/backup/quick.rs:~80`, `~116`); Krux code button only when `key.kef_offered()` (`~96-103`); Done, Next and Wrote only at their step (`faraday/faraday-core/src/screens/backup/quick.rs:178-182`): Wrote needs `quick_shown`, Next needs a match and `quick_code`, Done needs a match (or the Code step with `code_done`). An undated match shows a Set the time button when `b.undated && !clock_checked()` (`faraday/faraday-core/src/screens/backup/quick.rs:166-177`); Done is not gated on it.
- `quick_seedqr` sets `answers.seeds[SEEDQR] = true` on the quick path in every case (`faraday/faraday-core/src/wallet_backup/quick.rs:~95-97`).
- The Krux code's form does not open behind Check; Done waits for the check (fix f4653d345, in `backups-list`).

Krux code on the checklist (`faraday/faraday-core/src/wallet_backup/kef.rs`)

- One `KefForm` per seed whose `KefCode(i)` item is open, only at the Checklist stage. `new_form` refuses when the seed is not in the list, its key is not loaded or its word count is unknown.
- `KefAct::NotOnPaper` is refused while `plan_refuses`, then saves the answers and syncs. `ScanCopy` needs a camera and a form that offers it. Typing only on `Screen::Backup`.
- `kef_bytes` returns bytes only when the named file's SHA-256 equals the record's (Outbox first, then Files; `153-163`). `stick_put` never replaces a name already in the Outbox and writes only from Files (`224-242`). A scan counts only on a byte-exact match, else `EN.kef_not_this_code` (`~483-495`).

Wallet flow: setup and plan (`faraday/faraday-core/src/backup_flow/wallet.rs`)

- Vault setup before the keys: `wallet_vault_count`, `wallet_vaults_changed`, `wallet_flow_begin`, `wallet_flow_leave`, `wallet_flow_back` (`~303-533`). Answering "no" to vaults removes the pending setup flow (`~326-329`); the count is at least 1 (`~312`); nothing about places, paper or key placement is asked in setup (`~336-337`). `new_setup` (`faraday/faraday-core/src/backup_flow/mod.rs:~1217`) starts at `Step::Sticks`. A wallet flow left before its keys exist goes back to Make a new wallet or Restore (`wallet_flow_back`, `~502`).
- Plan answers in a flow are taken only for the Software and Form lists. Add place is capped at `qstep::MAX_PLACES` (`2947-2952`).
- Owner numbers are never 0 (`fresh_owner`, `133-136`).

Wallet Lock and write (`wallet_lock_write`, `faraday/faraday-core/src/backup_flow/wallet.rs:996-1213`; each refusal returns with nothing written)

- Stage not Checklist; the Sticks item absent; any other checklist item not done: silent returns.
- A backup past its lock and not done: toast `backup_flow_past_lock`.
- A secret file on the public-files stick: toast `backup_flow_public_secret`.
- A codex32 string not held by the session: toast `c32_vault_missing`.
- A secret file at a place not yet held for "Save without encryption?" (F129): `plain_ask`; nothing is made until it is held for each.
- A planned vault not open: `backup_flow_unlock_first`.
- A plan save that fails reverts the answers (`1070-1073`). A vault left out of keeping the plan still gets its keys but no plan.
- `wallet_vault_open` (`1218-1241`): the vault the flow made or chose stays its vault while it fits; a flow past its lock or knowing the vault by salt never gets another; two planned vaults never share an open vault.
- `wallet_seed_released` (`1300-1334`): needs all sticks written; a key that no plan seed is stays held; the seed must have sticks of its own among the written sticks. `wallet_vault_has_its_seeds` (`~1340-1375`): a seed taken out or moved after the lock makes the seal no copy of it.
- Seed files are secret Outbox items, never on the public stick, only for a seed whose key is loaded; SeedQR only for English words (`943-945`); a held silent wallet's descriptor picture is never made (`690-694`). Read-back: SeedQR by the key's fingerprint, the Krux code by SHA-256 of its base43 text.

Wallet copies into added vaults (`flow_wallet_copy`, `wallet_copy_*`, `faraday/faraday-core/src/backup_flow/wallet.rs:1458-1617`, `1759-2300`)

- No checksum: nothing. Every added vault must be open (`backup_flow_unlock_first`); each takes its share or none does; a plan copy refused gives `backup_flow_copy_plan_kept` or `vault_fault_wallet_why`; a record the vault refuses stops the copy and is named; the Keep sheet decides what leaves the source; a passphrase is not copied where the plan keeps it apart; a seed kept only on paper gives `backup_flow_key_on_paper`; a missing description gives `backup_flow_no_description`; two wallets give `backup_flow_two_wallets`; records the vault held before are never taken out; keys are kept when another wallet's records are in the vault; a passphrase with no key in an open vault to take it from gives `backup_flow_unlock_first`.
- The swapped-key refusal runs over every added vault, and stops on a locked added vault, before any vault asks for a share or typed words (fixes cf5002d40, e5a674c48, 66713db42).

Edits to a locked plan (`flow_wallet_edit`, `faraday/faraday-core/src/backup_flow/wallet.rs:2858-2944`)

- Nothing under a write (`2863-2865`, silent). File changes after the lock: `backup_flow_files_after_lock`. A written stick must stay: `backup_flow_written_stays`. A held key must stay on a stick with its passphrase: `backup_flow_held_stays`. Never take out what the vault held before or another wallet uses (`flow_takes_out_foreign`, `2705-2787`; `backup_flow_other_wallet_key`, `backup_flow_held_before`).
- `made_plan_changed` (`faraday/faraday-core/src/backup_flow/wallet_files.rs:46-99`): a vault whose sealed marks differ is stale: it loses `made_in`, `sealing` and its slots' `written`; the flow goes to Sticks if an existing missing vault is bound, else Vaults; with `advance`, a flow before Write and not at Lock moves to Lock.

Seed files on sticks

- Two name schemes are both accepted for a wallet's seed file: `seed_stick_name` (`faraday/faraday-core/src/backup_flow/seed_names.rs:6-25`): `seed-{fp}.txt` (Words written as a file), `seedqr-{fp}.png` (Words as SeedQR file), `passphrase-{fp}.txt` (`What::PassphraseFile`), fingerprint lower case, any other form None; `seed_stick_file_of` (`~29-40`) tests a name against the three. `seed_file_of(stem, fps, name)` (`faraday/faraday-core/src/backup_flow/wallet_files.rs:29-37`): `{stem}-{fp lower}-words.txt`, `-seedqr.png`, `-compactseedqr.png`. Both are used in `seed_file_ok` and `seed_file_ok_for` (`~136`, `~163`, `~172`, `~197`, `~208`).
- `seed_stick_bytes` (`faraday/faraday-core/src/backup_flow/wallet.rs:~896-956`) builds the words line, the passphrase line or the SeedQR PNG; `seedqr_picture` (`faraday/faraday-core/src/backup_flow/mod.rs:~2619-2630`) renders a labelled PNG titled only with the fingerprint and word count.

Wallet share records

- `wallet_ids_of` (`faraday/faraday-core/src/backup_flow/wallet_files.rs:~274-316`) takes ids from one source at a time (loaded wallets, open vault records, then untrusted files); `share_held_before` (`~340-357`) by tokens `s{cs}:{num}`, `s{id}:{num}`, and a 64-character fallback (a length check only); `wallet_share_record` (`~365-440`) returns the refusal `backup_sheets_rebuild_another` when the sheets do not rebuild the wallet, and clamps share number and count to `u8::MAX`.
- The split sheets show when the answer `split` is on and `wallet_backup::splits(wallet)` holds; the title (`faraday/faraday-core/src/screens/backup.rs:133`) reads that predicate through `shape.splits` (`faraday/faraday-core/src/wallet_backup.rs:4244`), the body and action read it directly (`faraday/faraday-core/src/screens/backup.rs:1211`, `1713-1717`): one predicate for one wallet. The title's key count is `keys_n - omit.min(m - 1)` (`faraday/faraday-core/src/screens/backup.rs:142`), never below 1 while m is at most n.
- The wallet done card shows Lock and Power off (Quit when online) on every card but the one reached from Create (`from != From::Create`, `faraday/faraday-core/src/screens/backup.rs:~853-865`); a Restore of a key made here shows them too. Lock opens the Lock sheet.

## Domain calls

- `faraday_keys::{identity::fingerprint_of_mnemonic, words::from_indices, seed_xor_random_len, seed_xor, Held, KeyId, KeyInfo, compare_words, CopyCheck}` (`faraday/faraday-keys/src/`); `faraday_keys` `Reveal::{BackupStick, CopyByHand, SeedQr, PaperForm}`, `seedqr_matrix`, `seedqr_reads_as`, `codex32_*`, `keep_codex32`, `set_codex32`, `position_of`.
- `faraday_bip::{core_import::writes_pair, policy::{WalletId, WalletPolicy}, keys::Fingerprint, bip39::{Language, Mnemonic}, codex32::{Codex32, MAX_SHARES}}`.
- `faraday_intake::{electrum, caravan, specter_diy, single_key::wasabi_writes}`.
- `faraday_plan::split_plan`, `faraday_plan::MAX_VAULTS`.
- `faraday_paper::pdf::{structural_dark, structural_fixed}`.
- `faraday_vault::{records::kind::PLAN, records::field::PLAN_ANSWERS, xor::XorFacts, slip39::Slip39Facts, codex32::{SetFacts, State}}`.
- `faraday_codec::{qr::{QrMatrix, codex32qr}, seedqr::{encode_seedqr, encode_compact}, kef::base43_encode}`.
- `faraday_forms::Form::{Words, Codex32, Krux}`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 16506fb24 | A silent payments wallet's backup sheet stopped the app (it has no policy keys). | The sheet names its key, account and address; no receive addresses are listed. |
| d71a5c5e4 | After the lock no wallet was loaded, and copying keys into a vault was refused with words that did not fit; the copy did not take the holder's plan record whole. | The copy replaces the other-order plan with the holder's plan (extras and notes kept); it is refused only when the other-order record holds shares the holder's description does not read. |
| cf5002d40 | Copying keys into a vault with the wallet's shares plan for the other key order asked for a codex32 share before refusing; the toast did not say what is so. | The refusal happens before any codex32 share is asked for; the toast states the situation and what to do. |
| e5a674c48 | The swapped-key refusal's pre-pass ran after some vaults had already asked for a share or typed seed; its toast told the person to load the wallet first, which was wrong. | The refusal runs over every added vault before any vault asks for input; the toast is accurate. |
| 66713db42 | The refusal's pre-pass did not stop on a locked added vault, so other vaults still asked for shares or typed words. | The pre-pass stops on a locked added vault before any vault asks for a share or typed words. |
| 33d9175a0 | A passphrase seed kept apart, loaded here with its passphrase and held by no vault, had its words saved to the added vault wrongly described as kept only on paper. | Its words alone are saved to the added vault; the fingerprint match falls back to the seed's name when no passphrase is wanted. |
| 5e729c7e7 | A save refused for a wallet whose keys are swapped did not stop the whole backup step before writing; vault records and codex32 steps could still be written. | A refusal is checked before any vault gets a record or a codex32 share is asked for. |
| 4911dc5c9 | A save into a vault, and the lock, did not check the vault being written for the swapped-key refusal; the copy into a later-added vault left it with a second plan record for the other key order. | The refusal covers the vault written into; each vault keeps one current plan for the wallet. |

## Doubts

- The line numbers of the public-files rule for a held silent-payments wallet were not confirmed; the rule is probably in `public_files`.
- Unchecked arithmetic: `audit` uses `1u32 << n` (overflow for n of 32 or more) and `structural` uses `n - 17` and `n - 8` (underflow for tiny n); callers are presumed to bound n.
- `expect`s on the dispatcher having checked first: `faraday/faraday-core/src/screens/backup.rs:~1007-1008`, `~2202`, `~1800`; `flows.all[i]` at `faraday/faraday-core/src/backup_flow/wallet.rs:~1426-1427`.
- Stale or garbled comments: `faraday/faraday-core/src/wallet_backup.rs:~3592-3594`, `~4043-4044`, `~4336-4339`; `faraday/faraday-core/src/screens/backup.rs:~1361-1369`; `faraday/faraday-core/src/backup_flow/wallet_screen.rs:~1-6` mentions cards Restore gains that are not in the file.
- Decisions named in comments: F129 ("Save without encryption?" hold), F240 (a wallet made from another has no backup-form card), F254 (a throwaway session builds a share record with no sheet), F285 (Seed XOR), F293 (old SLIP-39 shares), F62 (load at unlock).
- Test modules were not read; no fixture is named for this feature.
