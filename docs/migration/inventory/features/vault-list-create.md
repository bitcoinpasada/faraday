# The Vaults list and making a vault

The Vaults list shows every vault file in Files (on a stick or in the
Outbox) with its state, and the Create a vault flow makes a new KeePass-format
vault file (`.kdbx`) sealed under one or more passphrases and, if chosen, a
key file. It is for the person on the offline device or in Faraday Demo. The
list is reached from Home, Files and Wallets start; Create a vault from the
list, from Unlock when no vault is locked, from the backup and copy flows,
and on its own when Files holds no vault file. This file also lists the vault
area's whole state.

## Screens and sheets

- Vaults list `Screen::Vaults` (`list`, `faraday/faraday-core/src/vault/screen.rs:~351`; `list_compact`, `~645`): one card per vault file. Each card has a title from `panes_screen::look`, the file name, where it is (stick or Outbox), a summary line, a currency line and buttons. An open vault shows View contents and Lock; a locked one shows Unlock. With no vault file: "no vault in files" and, in Faraday Demo, a sample offer (`samples::put::offer`, `Action::UseSample`, `~379`, `~669`; only when `app.put_on()`).
- List card panes (`faraday/faraday-core/src/vault/panes_screen.rs`): lock icon, title, file, state line. An unopened vault reads "needs a medium" (`vault_remove_to_unlock`) when keys may not be loaded (`~75-87`).
- Summary line (`VaultSummary::line`, `faraday/faraday-core/src/vault/area.rs:~3494-3534`): one or two wallets by name and shape, more by count; keys by fingerprint; entries by count; the seen time when the clock is known.
- Currency line (`currency`, `faraday/faraday-core/src/vault/mod.rs:~2932-2980`; `Currency`, `faraday/faraday-core/src/vault/area.rs:~3389-3416`): Changed (open with changes), Current (a receipt names the file and hash, unchanged), NeverWritten (in the Outbox), Unchanged (from a stick, not changed). `vault_changed` names the first changed vault for Home (`~2984`).
- `vault_files` (`faraday/faraday-core/src/vault/mod.rs:~295-319`): a file in both boxes is listed once with the Outbox's copy first; a copy under another name is listed as a copy.
- Create a vault `Screen::CreateVault` (`create`, `faraday/faraday-core/src/vault/screen.rs:~923`; `create_body`, `~1098`; `create_summary(ies)`, `~973`; `VSTEPS`, `~916`). Steps `vstep::{PRESET, PHRASES, WHERE, COST}`:
  - PRESET: size rows (`CSizeRow(r)`) and Customise (`CCustomise`).
  - WHERE: machines the vault must open on: this computer (`CHere`), other machines (`CMachine(k)`), Next (`CNext`).
  - COST: preset rows (`CPreset(k)`), Custom (`CMemory(i)`); Next shows only for Custom, since a preset moves on by itself.
  - The default step: name (`CName`), passphrase pairs typed or made by dice (`Dice(i)`, `CFocus(i, second)`, `Action::PassEye`), key file (`create_key::draw`, `~1447`), Create button.
  - Without Customise the steps are PRESET, PHRASES; with it, WHERE, COST, PHRASES (`faraday/faraday-core/src/vault/area.rs:~2199-2205`). The default size set is chosen and `done[PRESET]` is true.
- Create foot (`faraday/faraday-core/src/vault/screen/create.rs:7-139`): summary of file, size, strength, memory and unlock time; warnings: this computer lacks the memory (`vault_need_mib(mem, new_file_len()) > free`, `~25-32`), memory over the weakest machine's limit (`~33-41`), slowest ticked machine 10 s or more (`~44-51`); Randomness row (`own_row`, `~124`); Create vault button (`pin_button`, `V::CGo`), drawn primary only when `may_load_keys()` and the passphrases do not differ (`125-132`).
- Drawing gates: a preset over the weakest machine's limit shows "too large" (`faraday/faraday-core/src/vault/screen.rs:~1138-1145`); "choose memory" for Custom (`~1139-1140`); the size row warns "this computer has X" (`~1255-1268`); the dice button is disabled without `may_load_keys` (`~1343`).
- Strength line (`faraday/faraday-core/src/vault/strength.rs:~14-37`): empty when every passphrase is Practice's `faraday` (`demo::PASSPHRASE`); "no passphrase" when all are empty; else the weakest bits plus the unlock cost; else "not measured". Dice bits show in the good colour at `STRONG_BITS` or more (`faraday/faraday-core/src/vault/screen.rs:~1420-1423`); a typed passphrase that is not public gets a "typed its own" line (`~1426-1429`).
- File line: the name the file will get, numbered when taken (fix f672dc9b3); the key file chip shows the name the key file will get (fix be8683f90).
- Dice entry (`crate::dice_entry`, `faraday/faraday-core/src/vault/dice.rs`): rolls typed or pressed on the pad, the word list, Use these words; in Faraday Demo a roller beside them.
- Key file dialog on Create (`faraday/faraday-core/src/vault/create_key.rs`): Make a key file, Choose a file, Cancel, Drop.
- Memory dialog: see `vault-seal`.

## Actions

- List: `VaultAction::Create` (`faraday/faraday-core/src/vault/screen.rs:~370`, `~659`), `VaultAction::Open(i)` (`~407`, `~411`, `~679`, `~683`), `Action::LockAsk` (`~408`, `~680`), `Action::UseSample`.
- Create: `Create`, `CreateFrom(Screen)` (a blank form, focus on the first passphrase, `faraday/faraday-core/src/vault/mod.rs:~503-524`), `CStep(k)` (leaving Passphrases marks it done, `faraday/faraday-core/src/vault/act_create.rs:~17-18`), `CNext(k)` (k < COUNT, clears the error, `~32`, `~49`), `CMachine(k)` (k < `MACHINES.len`, `~64`), `CHere`, `CPreset(k)` (clamped to CUSTOM, `~73`; CUSTOM = 5 opens the memory row), `CSizeRow(k)` (index in SIZES, `~80`), `CCustomise` (`~88-102`), `CMemory(k)` (clamped, sets custom, `~106`), `CFocus(i, second)` (only if `may_load`, else focus cleared, `~111`), `CName` (`~113`), `CGo`.
- Key file: `CKeyAsk`, `CKeyMake`, `CKeyChoose`, `CKeyCancel`, `CKeyDrop`.
- Dice: `Dice`, `DiceList`, `DiceUse`, `DiceClose`, `DiceKey`, `Roller`.
- Enter on the second field of the last passphrase starts Create (`CGo`, `faraday/faraday-core/src/vault/mod.rs:~2844-2850`).
- `CreateVaultSteps` plugs into Back and Forward (`faraday/faraday-core/src/vault/area.rs:~2215-2228`, `~2242-2249`): a vault being made takes no Back; `CSizeRow` moves to PRESET; `CPreset(k < CUSTOM)` moves to COST.
- `CHere`, `CMachine` and `CPreset` re-check later answers and clear those that no longer fit (`keep_what_fits`, `faraday/faraday-core/src/vault/act_create.rs:~128-145`; WHERE for CHere and CMachine, COST for CPreset). `create_later_fits` keeps COST only within the weakest chosen machine's limit; passphrases and name always fit (`faraday/faraday-core/src/vault/area.rs:~3016-3023`).

## Effects

- The Create job (`faraday/faraday-core/src/vault/stretch.rs:~265-369`) runs on the worker (`stretch::defer(self, Work::Create)`, `faraday/faraday-core/src/vault/mod.rs:~2612`; `defer` shows the working screen for one frame, then runs, `faraday/faraday-core/src/vault/stretch.rs:~103-112`).
- Randomness: vault id 16 bytes, seal `SEAL_RANDOM_LEN`, key file 32, entries 32 (`faraday/faraday-core/src/vault/stretch.rs:~276-296`); `Command::RequestEntropy` when the pool is empty.
- The made vault goes to the Outbox (`put_outbox`, `faraday/faraday-core/src/vault/stretch.rs:~582`); a key file drawn for it goes to the Outbox as `<name>.keyx` through `outbox_secret(OutReason::KeyFile)` (`~574-576`). A used key file is removed from Files (`~580`).
- The summaries are merged after a make (`faraday/faraday-core/src/vault/stretch.rs:~596-602`).
- Dice: `app.fresh(b"faraday dice roller")` per roll tick (`faraday/faraday-core/src/vault/dice.rs:~55`).
- `UseSample` puts the sample vault file into Files (`faraday/faraday-core/src/samples/put.rs:~698`, `EN.sample_vault_file`).

## State and lock

- `VaultArea` (`faraday/faraday-core/src/vault/area.rs:~24-27`): `state: Vaults`, `item_offset`; derefs to `Vaults` (`~50-56`). `Default` at `~29-45`.
- `Vaults` (`faraday/faraday-core/src/vault/area.rs:~2360-2632`) fields: `open: Vec<OpenVault>`, `store: OpenVaults` (keys and records, secret), `edits`, `sealed_from`, `create: Option<CreateForm>`, `memory_ticks`, `pick`, `passphrase: PassField`, `unlock_error`, `form_error`, `unlock_follow`, `unlock_key` (name only), `key_files_used`, `more`, `key_chosen`, `key_outbox`, `unlock_tried`, `unlock_made`, `current`, `category`, `category_chosen`, `category_scroll`, `item: [usize; 7]`, `item_open`, `form: Option<Form>` (may hold typed secrets), `saving`, `focus`, memory fields (`memory_free_mib`, `memory_reader`, `memory_read_at`, `ms_per_unit`, `measure`), `working`, `sealing`, `lock_named`, `stuck`, `drawn`, `pressed_at`, `just_made`, `made_by_seal`, `refused`, `acting`, `replayed`, `loads_started`, `key_added`, `key_refused`, `back_to`, `from_home`, `skipped_from`, `dice: DiceEntry`, `prompt: Option<Prompt>`, clock fields, `summaries`, `older`, `unsaved`, `step_dropped`, `save_hold`, GPG fields, `typed_shown`, Secure Boot fields, `fingerprints`, `accounts`, `seedqr`, `words`, `paperkey`, `code`, `codes`, `shown`, `stick_save`, `load`, `pass_seeds`, `copy_to`, `copy_wait`, `keep_save`, `trim`, `moved`, `moved_subjects`, `search`.
- `OpenVault` (`faraday/faraday-core/src/vault/area.rs:~2004-2039`): `name`, `id`, `facts`, `vault_id`, `changes`, `skip: BTreeSet<usize>`, `each_shown`, `read: Kept`, `seal_digest`, `seal_warn`, `read_only`, `size`, `settings`.
- `CreateForm` (`faraday/faraday-core/src/vault/area.rs:~2104-2142`): `open`, `done`, `customise`, `scroll`, `on: [bool; 5]`, `not_here`, `preset`, `memory: u32` (default 512, used only for Custom), `name: TextBox`, `phrases: Vec<PassField>`, `from_dice`, `error`, `key: KeyChoice {None, Make, File(Text)}`, `slowest`, `on_each`. It exists from `begin_create` (`~471`) to `drop_create` (`~446`) or `leave_create` (`~464`).
- Secret-holding state (checked):
  - Typed passphrases: `passphrase: PassField`, `Prompt.pass: PassField`, `CreateForm.phrases: Vec<PassField>`. `TextBox.text` is `SecretText`, which wraps a wiping `Text` (`faraday/faraday-intake/src/secret_buf.rs:25`).
  - `words: Option<(usize, Held<Mnemonic>)>` (`faraday/faraday-core/src/vault/area.rs:2585`), `paperkey: Option<(usize, Held<Text>)>` (`2588`), `seedqr: Option<(usize, QrMatrix)>` (`2581`). `Mnemonic`, `QrMatrix` and `Text` zeroize on drop (`core/faraday-bip/src/bip39.rs:759`, `core/faraday-codec/src/qr.rs:188-190`, `core/faraday-text/src/text.rs:363`).
  - `code: Option<CodeHeld>`, `codes: Option<CodesHeld>` (TOTP); `sign_wraps`/`sign_seen` (text of files chosen to sign; wiped by `drop_sign_text`); `dice` (rolls and passphrase words); `from_dice: Text`; `store: OpenVaults` (wiped by faraday-keys on close); `form` (typed entries, notes, words, TOTP secret). `load.passphrases` and `pass_seeds` hold labels; the typed passphrase is `prompt.pass`.
  - Public data: `summaries` (names, fingerprints, master ids, counts), `fingerprints` (FIFO of 1024), `accounts` (public key and chain code, cap 256), `sealed_from` digests, `key_files_used` names.
- `drop_create` (`faraday/faraday-core/src/vault/area.rs:~446-451`) clears `create`, `skipped_from`, closes dice and clears focus; `leave_create` (`~464-467`) also clears `back_to`.
- At a lock `VaultArea::lock` sets `create = None` and closes `dice` (`faraday/faraday-core/src/vault/area.rs:4477-4486`); `holds().typed` counts create phrases and dice rolls (`4461-4487`). Dice rolls are a held secret while the form is open and go when it closes.
- Every way into Create a vault opens with the eye closed (fix d31f89ac0); the eye on a passphrase closes when a step is left (F104, `faraday/faraday-core/src/vault/area.rs:~2286-2288`).

## Reach

- Opens Unlock (`vault-unlock`) for a locked vault and Vault contents (`vault-contents`) for an open one; opens the Lock sheet (`lock`).
- Create is opened from the copy and move flow (`vault-contents`), the backup flow's New vault, Unlock's "no locked vault" state, and the boot import.
- Uses `super::screen::create_summaries`, `app.vaults.create_later_fits`, `app.typing.pass_eye.close()`, `crate::fresh::roller::{Act, Sim, PerClick}`, `crate::dice_entry`, `crate::demo::PASSPHRASE`, `unlock_keys::{file_parts, cancel, key_slot, Bad}`, `trail::{StepFlow, Steps}`, `flow::Contained`, `pass_field::{screen::vault_box, pair::pass_pair}`, `dice_entry::{screen, STRONG_BITS}`, `samples::put::offer`.
- The memory dialog and memory steps: `vault-seal`.

## Rules and refusals
- A new vault's file name is the typed name reduced to ASCII letters, digits, `-` and `_` (whitespace becomes `-`, other characters are dropped, at most `NAME_MAX` characters, Windows device names refused), then the first of `name.kdbx`, `name-2.kdbx` to `name-99.kdbx` free in the Inbox and Outbox by file name and by key file name, else `name-100.kdbx` (faraday/faraday-core/src/vault/area.rs:3331-3360; vault/mod.rs:2652-2671).

- Create is hidden on the list and on the panes when `app.vaults.back_to` is set: no second vault is started in the way back's place (`faraday/faraday-core/src/vault/screen.rs:362`, `653`; `faraday/faraday-core/src/vault/panes_screen.rs:~147-148`).
- With no vault file in Files, Vaults opens New vault; Back returns to where Vaults was pressed (fix c1603fb1f).
- `V::CGo` runs `vault_create_check` (`faraday/faraday-core/src/vault/mod.rs:2604-2619`), which calls `create_refusal` (`2622-2650`), in this order:
  1. a stick is attached (`!self.stick.is_empty()`): `vaults_remove_the_first`;
  2. a step other than PHRASES not done: `vaults_finish_step`, and that step opens;
  3. any passphrase empty: `vaults_every_passphrase`; any not `phrase_ok` (typed the same twice, non-empty): `vaults_each_passphrase`;
  4. the chosen key file refused by `create_key::chosen`;
  5. `create_memory_refusal` (`faraday/faraday-core/src/vault/area.rs:3160-3175`): need (`vault_need_mib(mem, new_file_len())`) over free memory: `vaults_choose_a_cost`; free memory unknown and memory over Light (64 MiB): `vaults_memory_unknown_create`; noted through `work::refused`.
  Only then `stretch::defer(Work::Create)`. The primary style of the button is drawing only.
- A too-short passphrase is refused at once, before the session's randomness has arrived (fix f36d75418).
- Passphrase strength (`phrase_bits`, `faraday/faraday-core/src/vault/area.rs:~3302-3307`): bits are measured only while the field holds dice words; typed passphrases are not measured; Practice's fixed `faraday` passphrase is not measured.
- Estimates: `MACHINE_SECONDS` per 64 MiB at 3 passes; `seconds_text` shows one decimal below 9.95 s; 3 passes for every preset except Strong (`CREATE_PASSES`).
- `here_limit_of` (`faraday/faraday-core/src/vault/area.rs:3622-3632`): Light when free memory is unknown, else the largest `CUSTOM_MEMORY` (64..1024 MiB) whose need fits; it falls back to 64 MiB even if that does not fit (a floor, not a refusal).
- A new vault's untouched form starts on Standard, 256 MiB; Light only where free memory is unknown (fix 0b8d99cf8). Only the listed memories (64 MiB to 1 GiB at 3 passes) are offered; any other figure is refused, never written smaller (fix 9bdbb8827).
- Create job (`faraday/faraday-core/src/vault/stretch.rs`):
  - no job without fresh randomness (`app.fresh.pool_is_empty()` gives `Command::RequestEntropy` and `EN.no_randomness`, `263-272`);
  - a memory figure not in `STEPS_KIB` is refused with `EN.vault_cost_not_written` (`~249-260`);
  - the name is trimmed; empty becomes `EN.vaults_vault`; it is cut to `LABEL_MAX` at a character boundary (`~335-346`);
  - lanes are fixed at 4 (`~321`);
  - a chosen key file that cannot be read stops the make (`~303-311`).
- Created result (`faraday/faraday-core/src/vault/stretch.rs:~524-637`):
  - refused if a stick is in (`EN.vaults_remove_the_first`, `532-539`);
  - a drawn key file must reach the Outbox, else `EN.vault_key_file_not_kept` and no vault is kept (`~540-559`);
  - made inside a copy or backup flow: unlocked at once; otherwise it stays locked and picked, and `VaultAction::Open(i)` runs so the passphrase is typed once (F301; fix c5eb530e4).
- Free name (`free_vault_name`, `faraday/faraday-core/src/vault/mod.rs:2652-2674`): a name is taken by a name in the Inbox or Outbox (case-insensitive) or by its `.keyx` companion; it tries `stem.kdbx`, then `stem-2` to `stem-99`, then `stem-100.kdbx` without checking it.
- File stem (`vault_stem`, `faraday/faraday-core/src/vault/area.rs:3331-3366`): ASCII alphanumerics, `-` and `_` kept; any whitespace becomes `-` unless the output already ends in `-`; everything else (non-ASCII letters too) dropped; at most `NAME_MAX` = 40 characters (checked before each character); leading and trailing `-` trimmed; empty gives `vault`; Windows device names (con, prn, aux, nul, com0-9, lpt0-9, any case) get `-vault` (fix ce1266290). The document label is capped at `LABEL_MAX` = 64 bytes at a character boundary.
- Key file on Create (`faraday/faraday-core/src/vault/create_key.rs`): the dialog opens only when Create is open (`~28`); nothing is read or made before Create; a refused chosen file is named and makes nothing (`Bad::KeyFile`, `~71-74`); Make clears the error and closes the dialog (`~37-43`); the offered files are non-vault entries of Files.
- Dice (`faraday/faraday-core/src/vault/dice.rs`): rolls are refused unless `may_load_keys()` (`~74`); the roller unless `roller_offered()` (`~79`); Use fills only an existing passphrase field with a non-empty phrase (`~93-96`); Faraday Demo replaces rolled words with the public `demo::PASSPHRASE`, 0 bits (`~92-101`); fallback cost `(0, 64, 3)` (`~133`).
- `CName` has no `may_load` check, unlike `CFocus` (`faraday/faraday-core/src/vault/act_create.rs:~111-113`).
- The unlock time estimate is worked out on the worker, not the drawing thread (fix d03f74d36).
- A new Create form starts on `SIZES[0]` (Light, 64 MiB, 3 passes) with machines 0 (other PCs) and 4 (Pi Zero 2 W) ticked (`faraday/faraday-core/src/vault/area.rs:2163-2173`, `3658-3678`); the weakest ticked machine sets the memory limit (`weakest_of`, `faraday/faraday-core/src/vault/area.rs:3215-3228`), so the other Pis need no tick. `memory: 512` is only the custom size's value, used when the preset is `CUSTOM` (`faraday/faraday-core/src/vault/area.rs:3007`, `faraday/faraday-core/src/vault/act_create.rs:104-107`).

## Domain calls

- `faraday_keys::vault` (`faraday/faraday-keys/src/vault/mod.rs`): `OpenVaults`, `VaultId`, `SEAL_RANDOM_LEN`, `Held`.
- `faraday_keys::job::StretchJob` (`faraday/faraday-keys/src/job.rs`).
- `faraday_vault::file::{STEPS_KIB, Facts}` and `Cost` (`faraday/faraday-vault/src/file.rs`).
- `faraday_bip::diceware` (`core/faraday-bip/src/`), `faraday_bip::bip39::Mnemonic`.
- `faraday_codec::qr::QrMatrix` (`core/faraday-codec/src/qr.rs`).
- `faraday_storage::memory_available_mib` (`faraday/faraday-storage/src/lib.rs`) through `vaults.read_free()`.

## Fixtures

- `faraday/faraday-samples/data/sample.kdbx`: the sample vault Faraday Demo puts into Files (`faraday/faraday-samples/src/data.rs:~683`).

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| c1603fb1f | Vaults with no vault file in Files did not open New vault; Back did not return to Vaults | No file opens New vault; Back returns to where Vaults was pressed |
| d31f89ac0 | Create a vault opened by a job's answer showed the eye open; the first Hide press did not hide | Every way into Create a vault opens hidden |
| 0b8d99cf8 | A new vault's untouched Create form always started on Light, even where the computer reports its free memory; unlock cost text was not plain | New vault defaults to Standard 256 MiB; Light only where free memory is unknown |
| 9bdbb8827 | Create a vault could write a file with less memory than its label said | Only the memories listed are offered; other values refused, never written smaller |
| ce1266290 | A vault named com0 or lpt0 was written under a Windows device name | Safe file name for Windows device names |
| f672dc9b3 | Create a vault's file line did not show the name the file would get | Shows the name the file gets, numbered when taken |
| be8683f90 | The Create key file chip showed a name the file would not get | The chip shows the name the file will get |
| c5eb530e4 | A new vault and a changed passphrase did not end locked and unlocked once with the new passphrase | They end locked and are unlocked once |
| 456e22c51 | A Your choices row did not open its step as left; later fitting answers were not kept | Rows go forward while answers hold; a change clears those below |
| d03f74d36 | The time to unlock a vault was worked out on the drawing thread, so Create vault, Unlock and other screens froze while it ran | The unlock timing runs on the worker |
| f36d75418 | A KeePass export or Create a vault passphrase under 8 characters waited for randomness before being refused | A too-short passphrase is refused at once, before the seed has arrived |

## Doubts

- The two rows given to this feature were left out as not about it: 8d5708ed9 (a compile fix of the Argon QR Vault window build and a lint) and c2495cf13 (the QR vault tool's note seal). The rows above were given under Inside a vault and moved here.
- c5eb530e4 and 456e22c51 are flagged in history as decision changes as much as repairs.
- The line numbers of the list and Create screens, `faraday/faraday-core/src/vault/mod.rs` actions and `faraday/faraday-core/src/vault/act_create.rs` were not re-verified by the inventory.
- Two parts of the inventory gave different ranges for `free_vault_name` (`~2642-2658` and `2652-2674`); the confirmed one is used.
- `faraday/faraday-core/src/vault/area.rs:~1385` mentions free memory read "a process on macOS" (macOS is not developed).
