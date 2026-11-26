# Public wallet files

A wallet's public files are the files that carry its public description to
other wallets, and to paper. They include descriptors, multisig configs,
backup and share sheets, BSMS records, Core import, Electrum, Caravan,
Specter DIY and Wasabi files, and QR codes.

Faraday makes them one kind at a time (from Files or the wallet), or all
together from the backup checklist. It puts them in the Outbox ("For the
stick"), then shows a toast.

The guided backup flow's plan also decides which stick carries a key's
public files. It is for the person backing up a wallet on the offline
device. It is reached from the Backup checklist (`Screen::Backup`) and from
Files.

## Screens and sheets

- No screen of its own. Its rows are drawn in the Backup checklist and in Files.
- File kinds (`PublicFile`, `#[repr(u8)]`, `faraday/faraday-core/src/public_kind.rs:~13`). The wire values are fixed at 1 to 15:

  | value | kind |
  |---|---|
  | 1 | Descriptor |
  | 2 | MultisigConfig |
  | 3 | BackupSheet |
  | 4 | SplitShares |
  | 5 | WalletJson |
  | 6 | Bsms |
  | 7 | CoreImport |
  | 8 | DescriptorQr |
  | 9 | CoreBefore29 |
  | 10 | Electrum |
  | 11 | Caravan |
  | 12 | SpecterDiy |
  | 13 | Wasabi |
  | 14 | SpecterDiyQr |
  | 15 | SpDescriptor |

- The slow kinds, which run as worker jobs, are 2, 4 and 14 (`ROWS`, `faraday/faraday-core/src/public_kind.rs:49-70`).

## Actions

These are in `faraday/faraday-core/src/public_files.rs`:

- `public_out` (`~762`) makes one kind and puts it in the Outbox.
- `public_out_job` (`~783`) is the worker path for the slow kinds.
- `public_remove` (`~798`) removes a file by name. It saves the boxes if one was removed.
- The checklist: `files_start` (`~721`) and `checklist_start` (`~729`).
- `files_made` (`~627`) receives the worker's answer.

## Effects

- Files are written to the Outbox through `put_outbox`, which wraps the bytes in `Zeroizing` (`faraday/faraday-core/src/inbox.rs:~2098`).
- A toast follows.
- Worker jobs run for the slow kinds.
- The PDFs of the backup sheet are made with `faraday_paper::pdf::sheet`.

## State and lock

- `Inputs` holds the public policy and fingerprints, and no seed.
- `Made` holds the file bytes, which are public.
- `FilesWork` and `FilesDone` are owned copies on the worker, one job at a time.
- Nothing is wiped explicitly. Nothing secret is held.
- The plan's choice of stick is `Plan.public_stick` (`faraday/faraday-core/src/backup_flow/mod.rs:~448-458`, `~1533-1552`, `public_slot`), with `PublicOn::{Own, Place, Files}`. It is kept with the backup flow's state.

## Reach

- Called from the Backup checklist (`faraday/faraday-core/src/wallet_backup.rs`, `faraday/faraday-core/src/screens/backup.rs`), Files and the guided backup flow.
- Reads `restore::checked_config` and `restore::sheets_rebuild`.
- The secret sheet (`secrets`) carries the `SpDescriptor` kind.

## Rules and refusals

Making files (`faraday/faraday-core/src/public_files.rs`)
- `from_wire` takes only the values in `ROWS` (`faraday/faraday-core/src/public_kind.rs:49-70`).
- `Inputs.blank` (`~140`, `~328`) means no public files at all. It is set for a silent record that a loaded key contradicts.
- `Inputs.held` (`~168`, `~372`) is set for a silent wallet whose key waits for backup. It means no descriptor code, and the sheet is made with `held`.
- The address of a held silent wallet has no picture (`~774-778`). The sheet carries no QR code of it while the key waits for backup.
- A multisig config is refused with `backup_config_rebuilds_another` if `restore::checked_config` does not rebuild it (`~88-94`).
- A split sheet set is refused with `backup_sheets_rebuild_another` if `restore::sheets_rebuild` fails (`~95-101`).
- A refusal returns no files and shows a toast.
- A PDF that cannot be made gives the toast `public_made_no_pdf`, and is still named (`~375`).
- `SpDescriptor` is in `ROWS` but makes nothing here. It goes out through the secret sheet.

Worker answers
- `files_made` (`~629-643`) drops the answer when any of these is true:
  - the screen or sheet changed;
  - the wallet id is gone;
  - the backup is not open for that wallet, except for `Then::Out`.

Which stick carries a key's public files (`faraday/faraday-core/src/backup_flow/wallet_files.rs`)
- `public_stick_at` (`~226-235`) picks the first stick, in `t.key()` order, at a place with no secret file.
- `public_clash` (`faraday/faraday-core/src/backup_flow/wallet_files.rs:242-264`) reports the first secret file that would clash. It replans a local clone of the flow (no key material: `faraday/faraday-core/src/backup_flow/mod.rs:1015-1052`), dropped on return; its one caller keeps only the file names and the stick's name (`faraday/faraday-core/src/backup_flow/wallet.rs:971-990`).
- `is_secret_file` (`~217-221`) counts these as secret files:
  - a seed's words;
  - a SeedQR;
  - a passphrase file;
  - a codex32 string kept as a file.

## Domain calls

- `faraday_bip`: `bsms`, `core_import`, `WalletPolicy`.
- `faraday_paper::pdf::sheet`.
- `faraday_qr`: `specter`, `send::parts_frames`.
- `faraday_intake`: `specter_diy::command`, `electrum::file`, `caravan::file`, `single_key::wasabi_file`.
- `faraday_crypto::work::Compute`.

## Fixtures

None.

## Fixes

None. See Doubts.

## Doubts

- The three fix rows given to this feature were moved to wallets: 1590a5825, d89080ffc and b0010427e. They concern SLIP-132 versions and BSMS records being read when a wallet file loads, not the files made here.
- Two encoders count Specter parts: `specter::encode` and `send::parts_frames`. No check shows that they agree.
- Every line number in `faraday/faraday-core/src/public_files.rs` and `faraday/faraday-core/src/backup_flow/wallet_files.rs` is unconfirmed.
- The silent-wallet QR rule's line in the Backup screen is approximate. It is probably in `faraday/faraday-core/src/public_files.rs`, not at `faraday/faraday-core/src/screens/backup.rs:~1668-1675`.
- Public files left without a slot (`faraday/faraday-core/src/backup_flow/steps.rs:~478-482`) are not attached to a slot and not removed from the Inbox. Whether they stay visible in Files is unclear.
