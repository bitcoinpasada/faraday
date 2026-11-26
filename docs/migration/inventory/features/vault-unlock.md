# Unlocking a vault

Opening a locked vault file with its passphrase and, where it has one, its
key file, the checks made before the work starts, and the memory a vault kind
needs on this computer. It is for the person on the offline device or in
Faraday Demo. Unlock is reached from a locked vault's card on the Vaults
list, from a flow that waits on a vault (backup, copy and move, boot
import), and from Tools. After an unlock the vault opens on its contents,
usually with the Load sheet over it (see `vault-contents`).

## Screens and sheets

- Unlock `Screen::Unlock` (`unlock`, `faraday/faraday-core/src/vault/screen.rs:~435`; `unlock_compact`, `~739`): lists only locked vaults (`f.open.is_none()`, `faraday/faraday-core/src/vault/screen.rs:456-462`); with none, "no locked vault" and Create (`~461-473`, `~762-775`). The picked vault has a passphrase box, a key-file row, and the Unlock button.
- When `app.may_load_keys()` is false (`~450`, `~511`, `~528`, `~751`, `~812`, `~830`): a stick banner and a stick field in place of the passphrase box, and the Unlock button drawn disabled.
- While `Work::Unlock` runs, a working line replaces the button (`~531`, `~846`).
- A "type the passphrase" caption when the picked vault is the one just made (`~618-629`).
- Key file dialog `Sheet::KeyFile` (`faraday/faraday-core/src/vault/unlock_keys.rs`): candidates are Files entries that are not vaults; Choose is disabled with no files.
- Older copy dialog `Sheet::OlderCopy` ("Open this copy?", `OpenHold`): stops an unlock that finds an older copy or a differing seal count (`faraday/faraday-core/src/vault/copy.rs:~403`; `faraday/faraday-core/src/vault/load_screen.rs:~361-396`; seal counts in `vault-seal`). Open goes on; Keep locked keeps it locked and restores the remembered summary.
- The Load sheet `Sheet::UnlockLoad` follows a successful unlock: see `vault-contents`.
- The caution "Never type a seed that holds real bitcoin here." is not shown on Unlock while nothing can be typed (see `secrets`).

## Actions

- `Open(i)` (`faraday/faraday-core/src/vault/mod.rs:~474-499`): an open vault goes to `VaultContents` with `current` set; a locked one to `Screen::Unlock` with the pick.
- `OpenFrom(i, back)`, `ListFrom(back)`.
- `Pick(i)` (`faraday/faraday-core/src/vault/screen.rs:~505`, `~806`): clears the unlock parts and the error; focus goes to the passphrase only if `may_load` (`faraday/faraday-core/src/vault/act_state.rs:~26-31`).
- `FocusPassphrase` (`~518`, `~819`), typing, `Action::PassEye`.
- `Unlock` (`~554`, `~869`): runs `vault_unlock_check`.
- `Back` / `unlock_back` (`~569`, `~889`): goes to the boot import's `OPEN` sheet, the flow's back or `Screen::Vaults`. It drops the prompt and unlock parts, clears the passphrase and form, and cancels the secret (`faraday/faraday-core/src/vault/mod.rs:~608-634`).
- Key file: `KeyFileAsk`, `KeyFilePick`, `KeyFileChoose`, `KeyFileCancel`, `KeyFileDrop`. The key file is read only at Unlock. Picking another vault clears the held parts (`faraday/faraday-core/src/vault/mod.rs:~442-446`).
- `OlderOpen` / `OlderBack` on the older copy dialog.
- Keys (`vault_key`, `faraday/faraday-core/src/vault/mod.rs:~2768-2890`): Enter and Tab move between fields or save on the last; Escape clears focus and form (not on the backup flow's Make form, `~2804`; it does not clear `create` or `load`).

## Effects

- The unlock job (`Job::Stretch`, `faraday/faraday-core/src/vault/stretch.rs:~193-237`) runs on the worker after `stretch::defer(Work::Unlock)` (`faraday/faraday-core/src/vault/mod.rs:~1243`).
- The used key file is removed from Files and `save_boxes()` is called (`faraday/faraday-core/src/vault/unlock_keys.rs:~103-104`).
- Free memory is read through `vaults.read_free()` from one small file, at most twice a second (`faraday/faraday-core/src/vault/mod.rs:~1168-1190`; `memory_tick` refused within 500 ms of the last read). `memory_available_mib` (`faraday/faraday-storage/src/lib.rs:~431-471`, `~490-541`) gives the free memory for key derivation; `None` means only Light's memory may be used. On Windows it reads available physical memory (fix 52bc6f29d); the figure respects the control group limit: min(machine, cgroup) less use (fix 07451e073).
- After the unlock, `vault_unlock_go_on` (`faraday/faraday-core/src/vault/mod.rs:~1266-1396`):
  - from Tools it opens the vault's category;
  - keys and wallets set not to load are unchosen;
  - a vault just made opens on its contents;
  - unreadable items give a toast;
  - with `back == None` and not just made, the Load sheet offers loads;
  - `Codes` with no clock checked this power-on goes to Set the time (`~1383-1385`);
  - `unlock_landed` returns when a sheet is open;
  - the boot import goes on through `import_unlocked` / `import_after_unlock`.
- Unlock lands in the vault with the Load sheet over it; Cancel returns to a waiting flow (F280, amending F62; fix 7c001ec3c).

## State and lock

- Unlock state lives in `Vaults` (`faraday/faraday-core/src/vault/area.rs`): `pick`, `passphrase: PassField` (Vault kind), `unlock_error`, `unlock_follow`, `unlock_key` (name only), `key_files_used`, `unlock_tried`, `unlock_made`, `working`, `just_made`, `back_to`, `older: OpenHold`, `typed_shown`, memory fields.
- Key-file parts are held in faraday-keys (`UnlockParts`); the key file's key is read and held there on every path, and a lock clears it (fix c6c2ac552).
- `StretchWork.job` holds the passphrase as `SecretText` while the job runs (wiped inside faraday-keys, not checked).
- `clear_unlock` (`faraday/faraday-core/src/vault/area.rs:~534-539`) clears unlock parts, passphrase, `typed_shown`, `unlock_error`.
- Leaving Unlock empties the passphrase field (`pass_field::left`); a stick arriving empties it (`pass_field::stick_arrived`). A typed passphrase is cleared when nothing that takes it is up (fix d247a5a2d).
- At a lock `VaultArea::lock` clears the unlock `passphrase` (`faraday/faraday-core/src/vault/area.rs:4477-4486`); `holds().typed` counts it (`4461-4487`).
- Stale answers (an unlock whose file no longer matches the picked file, or Unlock left) are dropped and wiped (`faraday/faraday-core/src/vault/stretch.rs:~398-414`).
- `key_files_used` records a key file's name before the key file is accepted (`faraday/faraday-core/src/vault/unlock_keys.rs:~76`); nothing in the inventory clears it; it is read at `faraday/faraday-core/src/vault/area.rs:~511` and by Delete files (`faraday/faraday-core/src/stick/wipe.rs`, "used this power-on").
- `older` (`OpenHold`: lines, vault id, sheet, a public summary, `faraday/faraday-core/src/vault/seals.rs:484-495`) is dropped once its vault is not open (`faraday/faraday-core/src/backups/mod.rs:4205-4216`). At a lock it goes with the app, the not-kept rebuild included.

## Reach

- Opened from the Vaults list (`vault-list-create`), flows (backup, copy and move, boot import's `OPEN` sheet), Tools, the Codes screen (`totp-codes`).
- Opens Vault contents and the Load sheet (`vault-contents`), Set the time, the older copy dialog.
- Reads seal counts and summaries (`vault-seal`, `faraday/faraday-core/src/vault/seals.rs`).
- Uses the passphrase field (`secrets`) and `unlock_keys::draw`.

## Rules and refusals
- A vault made with a chosen key file reads the file's parts again for an unlock waiting on it; if that read fails the vault is made without parts and no message is shown (faraday/faraday-core/src/vault/stretch.rs:565-567, 625-629).

- `vault_unlock_check` (`faraday/faraday-core/src/vault/mod.rs:1212-1246`), in this order:
  1. `!may_load_keys()`: `vaults_remove_to_unlock` with the medium noun. `may_load_keys` is "self-test passed and no stick in the way" (`faraday/faraday-core/src/inputs.rs:1095`), so the same line also shows before the self-test has passed.
  2. Empty passphrase: `vaults_type_the_passphrase`.
  3. Key-file parts refused: `Bad::KeyFile`.
  4. `unlock_memory_refusal` (below).
  5. Only then `stretch::defer(Work::Unlock)`; an empty error text is cleared.
- Memory rule for Unlock (`open_refusal`, `faraday/faraday-core/src/vault/area.rs:3062-3075`): need = `vault_need_mib(memory MiB, file length)` (memory plus twice the file, rounded, plus 100 MiB for the app). Refused with `vaults_needs_this_computer` when need is over free memory. With free memory unknown and memory over 64 MiB (Light): `vaults_memory_unknown_unlock`. Without a known free memory only Light's memory unlocks and makes vaults, and the reason is said (fix 6415d884a). One memory rule serves Unlock, Create's check and This computer (fix d247a5a2d).
- Reading a file (`read_refusal`, `faraday/faraday-core/src/vault/area.rs:3139-3152`): need (`Need::Read`, twice the length) over free memory is refused; with free memory unknown a vault file over 4 MiB gives `visit_file_memory_unknown`; non-vault files are never refused.
- Unlock job (`faraday/faraday-core/src/vault/stretch.rs`):
  - no job when no file is picked; a key-file refusal stops it;
  - refused with the medium attached (`EN.vaults_remove_to_unlock`, `~427-442`);
  - the same vault already open is refused, naming the open one (`EN.vault_already_open`, `453-461`);
  - a wrong passphrase gives `EN.vault_error_wrong_key_file` when a key file was tried, else `EN.vault_error_wrong` (`~505-513`); the wrong passphrase reads "Did not open", a memory refusal says what it is (fix 2862380b9);
  - documents needing more than the free-memory figure are refused (`~139-188`);
  - an unreadable key file shows its error under the buttons (fix c5bcd5b03).
- Typing (`vault_key`, `faraday/faraday-core/src/vault/mod.rs:~2768-2890`): typing is taken only on Unlock, Create, VaultContents, SpendBackup, Codes, BackupFlow, the boot import sheet and the passphrase-seeds sheet; nothing secret is typed while a stick is attached (the key is taken and dropped, `~2796-2799`).
- Key file (`faraday/faraday-core/src/vault/unlock_keys.rs`): refusals Empty, Database, Damaged (`KeyFileError`) and file gone (`EN.vault_key_file_gone`). A KeePass key file is refused unless it means one thing, as in KeePassXC: no unexpected elements inside KeyFile, no repeated or misplaced Meta/Version/Key/Data, no namespaced root (fix 8f341f1d9).
- Key file of a just-made vault:
  - the vault keeps the key file it was made with for the next unlock (fix c5bcd5b03);
  - a kept key file reaches only that vault's Unlock, and only while Unlock is on screen for it, through a stale answer too (fixes 66f190cd4, b849764be, 8e3297902);
  - a key file is not used up by a try the stick refused (fix ce9db710e).
- Opening a KDBX file: it is never refused for its contents (a damaged Faraday item, a bad plan or amount, a duplicate field or UUID, a missing format or seal count); such an item is named Unreadable and written back unchanged; only format limits, a wrong passphrase, damage or malformed XML refuse it (F194; fix 5836c4d64). The file parser ignores the bytes of the inner header's end field, as KeePass and KeePassXC do (fix 4678c4713). A passphrase record (type 14) is readable (fix d247a5a2d).
- Older copy: an unlock that finds a copy with a lower seal count than the highest seen, or the same count with different bytes, stops at the older copy dialog; it never refuses (see `vault-seal`).
- Panes: an unopened vault reads "needs a medium" when keys may not be loaded (`faraday/faraday-core/src/vault/panes_screen.rs:~75-87`).

## Domain calls

- `faraday_keys::job::StretchJob`, `Stretched` (`faraday/faraday-keys/src/job.rs`).
- `faraday_keys::vault::{OpenVaults, VaultId, UnlockParts}` (`faraday/faraday-keys/src/vault/mod.rs`).
- `faraday_vault::file::{Facts, inspect, Error}` (`faraday/faraday-vault/src/file.rs`); `faraday_vault::kdbx::keyfile` (`faraday/faraday-vault/src/kdbx/`).
- `faraday_storage::memory_available_mib` (`faraday/faraday-storage/src/lib.rs`).

## Fixtures

- `faraday/faraday-samples/data/sample.kdbx`: the sample vault Faraday Demo offers, unlocked with Practice's passphrase `faraday` (`demo::PASSPHRASE`).

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 6415d884a | A computer that does not report free memory unlocked every vault kind, and could make vaults it cannot open | Without a known free memory only Light's memory unlocks and makes vaults, and the reason is said |
| 52bc6f29d | The low-memory warning for Argon2id vaults was missing on Windows | Available physical memory is read on Windows, so creating or unlocking a vault that needs more is warned about there too |
| 07451e073 | Argon2 free memory did not respect the control group limit (also: a vault's stick place listed repeatedly in a plan) | Argon2 memory = min(machine, cgroup) less use |
| d247a5a2d | The vault file parser ignored record type 14; every slot of a new vault had the same label; a passphrase typed for Unlock stayed after Unlock was left; the memory rule was stated two ways | The passphrase record is readable; each slot names its vault; a typed passphrase is cleared when nothing that takes it is up; one memory rule |
| 7c001ec3c | The Vaults list's Unlock sent the person to Files, Wallets or Wallets start instead of the vault; an unlock a flow waits on had no way back | Unlock lands in the vault with the Load sheet over it; Cancel returns to the waiting flow |
| 66f190cd4 | Pressing another vault beside Unlock handed a just-made vault's key file to that vault's unlock; a stale answer could too | A kept key file reaches only that vault's Unlock |
| b849764be | The same key-file leak through a stale unlock answer | The key file is given back only while Unlock is on screen for that vault |
| 8e3297902 | A stick put in during an unlock wiped the key file held for a just-made vault instead of giving it back | The held key file is kept through a stale answer while its Unlock is on screen |
| ce9db710e | A stick put in during unlock or seal did not keep the key file when the passphrase was right | A key file is not used up by a try the stick refused |
| c5bcd5b03 | A vault made with a key file showed "passphrase wrong" on its first unlock straight after Create; a key file that could not be read at Unlock ended the unlock without a message | The just-made vault keeps its key file for the next unlock; an unreadable key file shows its error |
| c6c2ac552 | The key file's key and a typed YubiKey secret were kept outside faraday-keys on some paths, so a secret lived in the app's own state after its screen | The key-file key is read and held in faraday-keys on every path, and a lock clears it |
| 8f341f1d9 | A KeePass key file with unexpected elements inside KeyFile, repeated or misplaced Meta/Version/Key/Data, or a namespaced root was accepted | A key file is refused unless it means one thing, the same as in KeePassXC |
| 5836c4d64 | A KDBX file was refused for its contents, and TOTP forms other than one were not read | A KDBX file is never refused for its contents; every TOTP form is read |
| 4678c4713 | A KeePass-family vault saved by KeeWeb was refused as damaged | The inner header's end field bytes are ignored, as KeePass and KeePassXC do |

## Doubts

- Five rows given to this feature were left out as not about it: e27d7ab7c (pinned memory pages in the crypto crates), 20c971b3b (a macOS build fix), ded96c7ca, 827c40bca and e548edb71 (the nonce history kept in a vault; they belong to the nonce history feature).
- The rows above other than 6415d884a were given under Inside a vault and moved here.
- 7c001ec3c may be a behaviour change more than a repair (its wording says "lands").
- 5836c4d64 also amends an earlier decision on refusing KDBX contents.
- The Unlock screen and `faraday/faraday-core/src/vault/mod.rs` action line numbers are not inventory-verified.
- `faraday/faraday-core/src/vault/mod.rs:~1126-1138`: the match arms for `Work::Unlock/Create/Kdbx/KdbxCheck` look unreachable (only `FlowMake` runs there).
