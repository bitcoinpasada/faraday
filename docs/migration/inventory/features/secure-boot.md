# Secure Boot keys

Secure Boot keys let a person make their own Platform Key (PK), Key
Exchange Key (KEK) and signature database key (db), keep them in a vault, write
the enrolment files for a computer's firmware, and sign a Faraday release's
boot program (`BOOTX64.EFI`) with their own db key. It is for a person who
wants their computer to start only Faraday boot programs they signed. The keys
are made in the backup flow (the Secure Boot kind), and kept as a type-8
record in a vault. Enrolment, signing and checking are reached from that
record in the open vault. Apart from that, the device reads at start whether
Secure Boot was on, and the power-off sheet reminds when it was off. The
Starter's "One I signed" boot file and its stick check use the same
signature reading (see `starter-make-stick`).

## Screens and sheets

- Secure Boot record in the open vault (`secure_boot_rows`, `faraday/faraday-core/src/vault/screen.rs:3337-3398`):
  - Owner GUID, then the common names of the PK, KEK and db certificates (`shown`, `faraday/faraday-core/src/secure_boot/mod.rs:51-62`). The GUID is written as text by `guid_text` (`faraday/faraday-core/src/secure_boot/mod.rs:27-40`).
  - The line `vault_enrolment_adds`.
  - A choice of two buttons: Windows compatible, or Own keys only.
  - A warning line in warning colour: `vault_own_keys_only_warning` or `vault_yours_with_microsoft`.
  - The backup row (how many places of the plan are written; Continue the backup).
  - When a wait is selected (key not yet fully backed up): "Back up first" in place of the enrolment hold.
  - Otherwise: the own-randomness row, then the hold "Hold for enrolment files" in warning tone.
  - An action "Sign or check" (`SbImages`).
- Sign or check page (`faraday/faraday-core/src/vault/screen/save.rs:~380-490`):
  - Title `vault_sign_or_check`.
  - Each Inbox file whose name ends in `.efi`, any case (`is_image`, `faraday/faraday-core/src/secure_boot/mod.rs:65-67`). Each has Sign (or "Back up first" while a wait is selected) and Check.
  - When none is there: `vault_no_efi_image`, naming where files come from.
  - The last check's line (`sb_checked`).
  - "Back to the keys".
- Sign page for a picked image (`faraday/faraday-core/src/vault/screen/save.rs:~380-420`):
  - The release check's line: `secureboot_release_boot` with the version, or the refusal in error colour.
  - Only when the check passed: the own-randomness row and "Hold to sign".
  - Back.
- Backup flow, Secure Boot kind: the same owner GUID and PK/KEK/db name rows (`faraday/faraday-core/src/backup_flow/screen.rs:1403-1408`).
- Power-off sheet: a Secure Boot reminder line, a BIOS key and a guide address code when the reminder applies (`faraday/faraday-core/src/screens/sheets.rs:~1873`; `faraday/faraday-core/src/secure_boot/state.rs:44-74`).

## Actions

`VaultAction` (`faraday/faraday-core/src/vault/area.rs:~4108-4118`):
- `SbOwnOnly(bool)`: chooses the enrolment policy (`faraday/faraday-core/src/vault/act_state.rs:51`).
- `SbEnrol`: a hold. It asks for the time first when the time is not yet asked (`time_asked`); then `sb_enrol` runs (`faraday/faraday-core/src/vault/mod.rs:924-928`).
- `SbImages`: opens or closes the Sign or check page. It also clears the picked image and the last check line (`faraday/faraday-core/src/vault/act_state.rs:52-56`).
- `SbPick(k)`: picks Inbox image `k`. It keeps the release check found then (`sb_release_now`) in `sb_sign` (`faraday/faraday-core/src/vault/mod.rs:930`).
- `SbHoldSign`: a hold. It signs the picked image (`faraday/faraday-core/src/vault/mod.rs:931-934`).
- `SbCheck(k)`: checks image `k`'s signature against the selected db certificate (`faraday/faraday-core/src/vault/mod.rs:935`).
- While the selected key waits for its backup, `SbEnrol`, `SbPick` and `SbHoldSign` open the backup flow (`flow_resume_selected`) and do nothing else (`faraday/faraday-core/src/vault/mod.rs:886-892`). `SbCheck` is not under this guard.
- Backup flow, Secure Boot kind: Make runs `sb_build(owner)` with the first typed field as the owner name (`faraday/faraday-core/src/backup_flow/steps.rs:2224-2228`).

## Effects

- Randomness:
  - `fresh(b"secure boot")` gives the key seed.
  - `fresh(b"secure boot enrolment")` is drawn when keys are made and when enrolment files are written.
  - `fresh(b"secure boot signing")` is drawn when an image is signed.
  - These are at `faraday/faraday-core/src/secure_boot/mod.rs:156-170`, `:241`, `:268`.
  - `faraday/faraday-core/src/fresh.rs` counts `SbEnrol` and `SbHoldSign` as presses that draw (`faraday/faraday-core/src/fresh.rs:~949-950`, `~994`).
- Clock: `clock_now()` for making keys and for enrolment.
- Outbox:
  - Enrolment files go to the Outbox one by one (`put_outbox`), with the toast `secureboot_enrolment` naming the count (`faraday/faraday-core/src/secure_boot/mod.rs:244-251`).
  - A signed image goes to the Outbox under the image's own name, with the toast `secureboot_signed`. The sign state is then reset (`sb_reset`) (`faraday/faraday-core/src/secure_boot/mod.rs:275-284`).
- Making keys in the backup flow returns a `Built`: the record, the owner GUID as subject, a name (the owner, or the GUID when no owner was typed), and the enrolment files for the outbox. The backup flow writes them with the plan (`faraday/faraday-core/src/secure_boot/mod.rs:152-182`).
- Check: a toast `<name>: <what>` and the same line kept in `sb_checked` (`faraday/faraday-core/src/secure_boot/mod.rs:306-317`).

## State and lock

- In `VaultsState` (`faraday/faraday-core/src/vault/area.rs:2555-2566`):
  - `sb_owner: Text`: the owner name being typed.
  - `sb_own_only: bool`.
  - `sb_images: bool`.
  - `sb_sign: Option<(usize, Checked)>`: the image picked and its release check.
  - `sb_checked: Option<Text>`: the last check line.
  - None of these holds a secret.
- The private keys (PK, KEK, db) are in the open vault's type-8 record (`faraday_vault::records::kind::SECURE_BOOT`). The core reaches them only through `vaults.store()`: `sb_enrolment` and `sb_sign_image` in `faraday/faraday-keys/src/vault/reveals.rs:~515`, `~539`.
- The seed and the made key in `sb_build` are locals handed to `faraday_keys::secure_boot::create`.
- `SecureBootAtStart` (Unknown, Off, On) is `at_lock::Public`: it holds nothing a lock wipes (`faraday/faraday-core/src/secure_boot/state.rs:9-20`).
- A lock closes the vaults with the rest of the vault area. It then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`). So no Secure Boot key stays reachable after a lock.

## Reach

- Opened from: an open vault's Secure Boot record (the vault feature), and the backup flow (Secure Boot kind).
- Reads: the Inbox (images, `SHA256SUMS`, `SHA256SUMS.asc`), the open vault's selected record, the clock, `selected_waits`.
- Opens: the backup flow (`flow_resume_selected`, "Back up first").
- The power-off sheet reads `secure_boot_reminder`, `bios_key` and `secure_boot_guide`.
- `bios_key` reads `faraday_guide::maker`. `secure_boot_guide` reads `crate::guide::guide_id` with "windows" and `guide_address`. This is the same address the Starter shows for the same computer (`faraday/faraday-core/src/secure_boot/state.rs:54-74`).
- The Starter uses `faraday_sb::pe::{transplant, signer}` and `common_name` for its "One I signed" boot file and stick judging (see `starter-make-stick`).

## Rules and refusals

- Make keys (`sb_build`):
  - Refused with `secureboot_this_computer` when the clock is not set (`faraday/faraday-core/src/secure_boot/mod.rs:153-155`).
  - Refused with `no_randomness` when either random draw is missing (`:156-170`).
  - The owner name is trimmed (`:~159`).
  - A `faraday_sb` error is shown by `error_text`: `secureboot_error_key`, `secureboot_error_image` or `secureboot_error_not_release` (`:92-98`).
- Enrolment policy: `Policy::OwnKeysOnly` when `sb_own_only`, else `Policy::Windows` (`faraday/faraday-core/src/secure_boot/mod.rs:235-239`). Enrolment files are written only through a hold. The warning line is in warning colour (`faraday/faraday-core/src/vault/screen.rs:3359-3394`).
- Enrol (`sb_enrol_refusal`, `faraday/faraday-core/src/secure_boot/mod.rs:187-193`):
  - Silent when a wait is selected (the press opens the backup flow first, see Actions).
  - Silent when the open vault has no Secure Boot record selected.
  - The clock text when the clock is not set.
  - Then `no_randomness` if the draw fails (`:241-243`).
- Sign (`sb_sign_refusal`, `faraday/faraday-core/src/secure_boot/mod.rs:206-216`):
  - Silent without a selected key, while a wait is selected, or without image `k`.
  - A toast for a release refusal.
  - The hold re-checks the Inbox as it is when held, not only as it was when picked (`:257-266`).
- Release check (`sb_release_now`, `faraday/faraday-core/src/secure_boot/mod.rs:119-140`; `refusal_text` `:101-110`):
  - The image's SHA-256 is the one taken when it came into the Inbox. An Inbox item without one is `NotImage`.
  - `SHA256SUMS` and `SHA256SUMS.asc` are looked up in the Inbox by name, any case.
  - The release must be at least as new as the running Faraday version.
  - Refusals:
    - `NotImage`.
    - `NoSums`: `SHA256SUMS` missing.
    - `NotSigned`: `SHA256SUMS.asc` missing or not valid.
    - `Older { version, running }`.
    - `NotListed { version }`.
- Release keys are those built into `faraday_sb::release::KEYS`. The test release key is added only under the `testkit` feature, never in a shipped binary (`faraday/faraday-core/src/secure_boot/mod.rs:81-86`).
- Check (`sb_check`, `faraday/faraday-core/src/secure_boot/mod.rs:291-318`):
  - Silent without a selected Secure Boot record, without image `k`, or without a db certificate in the record.
  - Results: Valid (`secureboot_signed_by_vault`), OtherSigner, Altered, Unsigned, Unreadable (`secureboot_its_signature`), NotRead. A crate error is shown by `error_text`.
- Only files ending `.efi` (any case) are listed as images (`faraday/faraday-core/src/secure_boot/mod.rs:65-67`).
- `faraday_sb` reading of an EFI file, from the history fixes (`faraday/faraday-sb/src/pe.rs`, `faraday/faraday-sb/src/cms.rs`, `faraday/faraday-sb/src/x509.rs`):
  - Overlapping sections, sections inside the headers, or sections near the end of the address space are refused as not an image.
  - The signature is decoded and the image hashed once, however many certificates there are.
  - The signature is read as firmware reads it.
  - Two message digest attributes, or one with two values, read as unreadable.
  - A certificate time with an hour, minute or second out of range is refused.
- Secure Boot at start (`faraday/faraday-core/src/secure_boot/state.rs:22-36`): byte `0x1ec` of `/sys/kernel/boot_params/data` is 2 for Off and 3 for On. Any other value, or a file too short, is Unknown.
- Reminder on the power-off sheet (`secure_boot_reminder`, `faraday/faraday-core/src/secure_boot/state.rs:44-49`). It shows only when all of these hold:
  - `Medium::Stick`.
  - Not a shell with stick pills (Faraday Demo).
  - The maker does not start with "Apple".
  - Secure Boot was not read as On.
- `bios_key` is given only with the reminder, and only when a guide maker page matches (`:54-59`).

## Domain calls

- `faraday_keys::secure_boot::create(seed, blind_seed, owner, policy, now)` (`faraday/faraday-keys/src/secure_boot.rs:~31`).
- `faraday_keys::vault` store: `sb_enrolment(id, at, seed, policy, now)` and `sb_sign_image(id, at, seed, release, bytes)` (`faraday/faraday-keys/src/vault/reveals.rs:~515`, `~539`).
- `faraday_keys::vault::{RecordView, VaultId, sb_field}`.
- `faraday_sb::release::{check, KEYS, SUMS, SUMS_SIGNATURE, Refusal, Release, ReleaseKey}` (`faraday/faraday-sb/src/release.rs:~120`).
- `faraday_sb::pe::check(pe, cert)` (`faraday/faraday-sb/src/pe.rs:~399`).
- `faraday_sb::{Check, Policy, Error, common_name}` (`faraday/faraday-sb/src/lib.rs:~234`).
- `faraday_vault::records::kind::SECURE_BOOT`.
- `faraday_crypto::hex::encode_string`.
- `faraday_guide::maker`.

## Fixtures

- `faraday/faraday-sb/tests/vectors/cert.der`
- `faraday/faraday-sb/tests/vectors/cert.sha.der`
- `faraday/faraday-sb/tests/vectors/make.sh`
- `faraday/faraday-sb/tests/vectors/tiny.efi`
- `faraday/faraday-sb/tests/vectors/tiny.osslsigncode.efi`
- `faraday/faraday-sb/tests/vectors/tiny.sbsign.efi`
- `faraday/faraday-sb/tests/vectors/tiny.sha1.efi`
- `faraday/faraday-sb/tests/vectors/tiny.sha384.efi`

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| e5b23ba54 | An EFI file with overlapping sections, sections starting inside the headers, or sections near the end of the address space was hashed thousands of times or stopped the program, including on a 32-bit build. | Such a file is refused at once as not an EFI image. |
| 7e51da5a3 | Enrolment files were not written under a hold. The warning line did not say that some computers may not start once enrolled (Microsoft's third-party certificate is in neither list), or that Restore Factory Keys undoes it. | Enrolment files are written only through a hold. The warning is in warning colour and names the risk and the undo. |
| c475d936b | An EFI file with very many certificates re-read and re-hashed its signature once per certificate. | The signature is decoded and the image hashed once. |
| b66adefb2 | The check read an EFI signature differently from firmware: a certificate table not ending the file, a signer id naming another certificate, other content types, two signers. | The signature is read as firmware reads it. The checks match what firmware accepts (also in the Starter). |
| ee5976a9c | An Authenticode signature with two message digest attributes, or one attribute with two values, was read as valid. | Read as unreadable (RFC 5652 11.2). |
| 76aaee791 | A certificate time with an hour, minute or second out of range was accepted. | Out-of-range times are refused. |

## Doubts

- Three history rows on this feature are left out: 0a54cebd5, 0b277a8cf and e23a27b4e. They concern timing behaviour of the RSA signing and its dependencies, which the inventory does not record. One fact from 0a54cebd5 is kept under Effects: making keys, enrolment and signing each draw fresh system randomness.
- 76aaee791 was filed under the Starter in the history. It changes `faraday/faraday-sb/src/x509.rs`, so it is placed here.
- Earlier notes said Enrol "refuses silently when a wait is selected". In the code, the press first goes to the backup flow (`faraday/faraday-core/src/vault/mod.rs:886-892`). The silent refusal in `sb_enrol_refusal` is only reached from other callers, such as `sb_enrol_draws`.
- The earlier caller line `faraday/faraday-core/src/backup_flow/screen.rs:1404` draws the record (`shown`). Keys are made from `faraday/faraday-core/src/backup_flow/steps.rs:2226`.
- Not read: how `faraday_keys` wipes the seed, the made key and the blinding seeds once `create`, `sb_enrolment` and `sb_sign_image` return. Also not read: the exact files the enrolment writes.
- Not read: whether `sb_owner` and the sign and check fields are cleared when the vault closes without a lock. The process restart at lock clears them in any case.
