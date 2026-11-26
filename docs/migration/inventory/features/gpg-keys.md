# GPG keys

GPG (OpenPGP) keys made and kept in a vault: a primary key with a signing
subkey, its revocation certificate kept with it, renewing its term, exporting
the public key, making and exporting a revocation, signing files from Files,
and restoring a key from a paperkey. It is for the person who keeps a GPG
identity on the offline device. It is reached from Vault contents' GPG keys
category (rows, Make a key, GPG Restore) and from the backup flow, which
makes a new GPG key and restores one.

## Screens and sheets

- GPG rows on a key's item page (`gpg_rows`, `faraday/faraday-core/src/vault/screen.rs:~3197`): fingerprint, subkey, user IDs, validity, revocation status and kept certificate, the paperkey behind an eye (made only while shown, `~3226-3235`), the renew term (`V::GpgRenewTerm`) and Renew (disabled with no term, `~3279-3291`), Export public, Export revocation, Make a revocation (not read-only), Sign a file (`V::GpgSignPick`). With a backup waiting, Renew and Sign are replaced by "Back up first" (`~3267-3277`, `~3326-3330`) and `backup_row` "Continue the backup" (`~3295-3303`, `~3405`). A hold marks the key revoked (`V::GpgHoldRevoke`, `~2962-2974`).
- GPG key form and revocation form in the form pane (revocation detected by field index 3, `faraday/faraday-core/src/vault/screen.rs:~2481`); Save reads "Make the key" or "Make the revocation". Make a revocation asks GnuPG's reasons (fix bd1a3959d).
- Sign pane (`sign_pane`, `faraday/faraday-core/src/vault/screen.rs:~2693`) and `Sheet::GpgSign` ("Sign with {key}"): tick files (`GpgTick`, `GpgTickAll`); `SignText(i)` shows a small text file's text before the hold (`~2827-2830`); a QR/PNG per signature named with the first 8 bytes of the file hash (`2753`); a "signed" tag when the Outbox holds `<file>.asc` (`~2746-2749`); Sign is primary only when a file is ticked (`~2844`); only the hold on the sign sheet signs (`~2835-2836`).
- GPG Restore button in Vault contents' GPG keys category (`backup_flow::gpg_restore::contents_button`, `faraday/faraday-core/src/vault/screen.rs:~2018-2022`).
- GPG restore sheet `Sheet::GpgRestore` and the restore flow's Make card (`gpg_restore_screen::make_body`, `faraday/faraday-core/src/backup_flow/screen.rs:~1241-1243`). In Practice, restore, sheet, from and scan are dimmed (`faraday/faraday-core/src/backup_flow/steps.rs:~2384`).
- Paperkey QR (`V::PaperkeyQr`).

## Actions

- Vault: `GpgYears`, `GpgExport`, `GpgExportRevocation`, `GpgRevocationPage`, `GpgReason(u8)` (clamped to 3), `GpgRenewTerm`, `GpgHoldRevoke`, `GpgRenew`, `GpgSignPick`, `GpgTick(i)`, `GpgTickAll`, `SignText(i)`, `GpgSign`, `GpgHoldSign`, `PaperkeyQr`, `Reveal` (paperkey eye).
- Restore: `RestorePress::{Sheet, From(k), Restore, Scan, ScanPaperkey}` (`faraday/faraday-core/src/backup_flow/steps.rs:~2386-2428`). `R::From(k)` starts a restore flow from Inbox file `k`. Typed key lines go to `flows.restore_key` only on the flow screen with no sheet open (`~2430`).

## Effects

- Make: `faraday_keys::gpg::create` builds the key; randomness `fresh(b"gpg primary")`, `fresh(b"gpg subkey")` (`faraday/faraday-core/src/vault/mod.rs:~3154-3155`); expiry from `gpg_years` (`~3163-3169`). The revocation certificate is kept with the key (record type 7, field 8) and never written to a stick at Make (fix bd1a3959d).
- Export public: an armoured public key to the Outbox.
- Export revocation: only through the secrets sheet (`Keep::None`), as `{name}-{FPR}.rev` in GnuPG's layout, never public, never a code or a picture (`faraday/faraday-core/src/vault/mod.rs:~3320-3332`; fixes bd1a3959d, ca3aaf584).
- Sign: an armoured signature `{file}.asc` per ticked file to the Outbox, and a QR/PNG of each signature.
- The camera for Scan a paperkey (restore).
- Clock reads `clock_now`, `clock_checked`, `checked_now` (`faraday/faraday-core/src/vault/mod.rs:~2902-2911`).

## State and lock

- GPG fields in `Vaults` (`faraday/faraday-core/src/vault/area.rs`): `sign_wraps`, `sign_seen` (text of files chosen to sign; wiped by `drop_sign_text`), `sign_shown`, `paperkey: Option<(usize, Held<Text>)>` (`2588`).
- `faraday/faraday-core/src/gpg.rs`: texts are wiped by `Seen::drop_all` and `SignWrap` drop_all; the paperkey comes as `Held<Text>` from faraday-keys.
- The paperkey is dropped when the item page or selected record changes (`tidy_shown`, `faraday/faraday-core/src/vault/area.rs:621-634`) and at the seal's end (`close_all`). `sign_shown` is cleared when not on the signing page or the page is not on screen. At a lock `VaultArea::lock` calls `drop_sign_text` (`faraday/faraday-core/src/vault/area.rs:4477-4486`); a lock wipes and drops the signing page's kept texts (fix 838ea6938).
- Restore state `RestoreFrom` in `BackupFlow.restore` (`faraday/faraday-core/src/backup_flow/mod.rs:~1047`), kept as a `restore` line (file and hex, `~1749-1751`). The typed lines are wiped at a lock by `restore_wipe` (`faraday/faraday-core/src/backup_flow/gpg_restore.rs:583-589`); the certificates stay with their flows. The backup flow area's lock wipes only these lines (`faraday/faraday-core/src/backup_flow/area.rs:790-799`).
- The revocation certificate is a secret (F32): made and kept inside faraday-keys; it leaves only through a named reveal (fix ca3aaf584). The paperkey is lent only for the GPG paperkey reason (`Reveal::GpgPaperkey`; see `secrets`, fix f8fea5987).

## Reach

- Opened from Vault contents (`vault-contents`) and the backup flow (`faraday/faraday-core/src/backup_flow/steps.rs`, `faraday/faraday-core/src/backup_flow/gpg_restore.rs`).
- Sign a file reads Files (Inbox) and the Outbox; the signing sheet uses the secrets sheet for a revocation export (`secrets`).
- Reads the clock (Set the time); `gpg_restore_finish` needs the vaults open (`vault-unlock`).
- File classification treats a file that is only armoured OpenPGP public-key or signature blocks as public (`secrets`).

## Rules and refusals

- `gpg::usable` (`faraday/faraday-core/src/gpg.rs:151-173`) refuses signing with a revoked key, an expired key (`now >= created + expiry`), or a primary key or subkey created after now.
- `small_text` (`faraday/faraday-core/src/gpg.rs:~274-282`) refuses empty text, more than 2048 bytes (`TEXT_MAX`), non-UTF-8, or control characters other than newline, CR and tab. Names keep ASCII alphanumerics only (`~118-126`). `DEFAULT_YEARS = 3`, `YEAR = 31_536_000`, `SKEW = 5`.
- `gpg_clock` refuses with `gpg_this_computer_s` when there is no clock (`faraday/faraday-core/src/vault/mod.rs:~3101-3104`). `GpgRenew` is refused while `time_asked()` (`~902`). A GPG form action needs `time_asked` checked (`~2018-2021`).
- Make: a key needs a name or an email (`gpg_a_key_needs_a_name`, `faraday/faraday-core/src/vault/mod.rs:~3149`); in the backup flow it is refused only when both name and email are empty (`faraday/faraday-core/src/backup_flow/steps.rs:2165`). Fresh randomness is needed, else `no_randomness` (`faraday/faraday-core/src/vault/mod.rs:~3159-3161`). A non-revocation GPG form with no record is refused (`~2013-2016`); a GPG key made new is the flow's.
- `gpg_mark_revoked` is idempotent (`~3378-3396`).
- `gpg_renew` (`~3403-3475`): refused while `selected_waits()`; with no term (`gpg_choose_the_term`); on a revoked key (`gpg_revoked`); on a clock conflict (`gpg_time_conflict`, when the last signature is later than now by `gpg::SKEW`); term 0 means no expiry; a name collision gives `gpg_exists`.
- Signing:
  - `.asc` files are never signable; exactly one signable file is ticked by default (`~3537-3543`).
  - `gpg_sign_sheet_open` is refused when not signing, when the key differs, or when nothing is ticked (`~3627-3639`).
  - `gpg_sign_sheet_hold` acts only on `Sheet::GpgSign` with a known clock (F63).
  - `gpg_sign_ticked` (`~3655-3713`): refused while `selected_waits`; a key mismatch resets the page; `gpg::usable` must pass; a file whose bytes changed is not signed (`gpg_changed`); an existing `{file}.asc` is not overwritten (`gpg_exists`); a failed armoured signature is skipped with no message (`~3705-3708`).
  - A signature is made only over the file as it was when its code was made (fix da6c56035).
  - A key waiting for backup signs nothing (`backup_flow_first`, `~3277-3278`).
- Revocation: export only through the secrets sheet (`~3320-3332`); `gpg_keep_revocation` is refused when there is no room (`~3212-3216`); `gpg_make_revocation` does not replace the kept one and needs a known clock (`~3358-3361`); a revocation needs a reason (default Unspecified).
- A GPG file replaced in the Inbox by one of the same name and size has its hash computed again (fix abe702257).
- Restore:
  - Paperkey lines: `push` accepts hex, spaces and one colon (`faraday/faraday-core/src/backup_flow/gpg_restore.rs:286-298`).
  - `gpg_restore_finish` needs every needed vault open and the key built from the lines (`faraday/faraday-core/src/backup_flow/steps.rs:~2412-2428`).
  - Not in Practice (`~2384`).

## Domain calls

- `faraday_keys::gpg::create` (`faraday/faraday-keys/src/gpg.rs`).
- `faraday_keys::vault::{gpg_facts, gpg_paperkey, gpg_revocation_kept_at}` (`faraday/faraday-keys/src/vault/mod.rs`, `faraday/faraday-keys/src/vault/reveals.rs`), with `Reveal::GpgPaperkey`.
- `faraday_pgp::{Reason, read_public_armor, Armor, fingerprint_text, signature_time, utc_date}` (`faraday/faraday-pgp/src/lib.rs`); paperkey parsing in `faraday/faraday-pgp/src/paperkey.rs`.
- `faraday_vault::records` (`faraday/faraday-vault/src/records.rs`): the GPG record and its revocation field.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| da6c56035 | A GPG signature could be made over a file changed since its code was made | A signature is made only over the file as it was when the code was made |
| 08d4ec447 | Faraday's own GPG public keys, revocation certificates and signatures, copied between sticks, were read as text and asked "This may be a secret" | A file that is only armoured OpenPGP public-key or signature blocks is a public file |
| bd1a3959d | GPG revocation certificates were written to the stick at Make, and exported not as GnuPG does | The revocation certificate is kept with the key, never written to a stick at Make; export only through the secrets sheet as {name}-{FPR}.rev in GnuPG's layout; Make a revocation asks GnuPG's reasons |
| 5ab39ba4b | At 1280 x 800 a GPG key's page drew Backup over its Renew row's second line, and Files' line ran past the For the stick column | Terms wrap onto a second row, and Files' line wraps |
| abe702257 | A GPG file replaced in the Inbox by one of the same name and size kept its old cached hash | The hash is computed again when the Inbox's files change |
| ca3aaf584 | A GPG key's revocation certificate was handled as an ordinary output rather than a secret | A revocation certificate is secret (F32); it leaves faraday-keys only through a named reveal |

## Doubts

- The code of `faraday/faraday-core/src/backup_flow/gpg_restore.rs` and `faraday/faraday-core/src/backup_flow/gpg_restore_screen.rs` was only partly read by the inventory (the `push` rule and `restore_wipe`); the restore flow's other rules are missing.
- The vault-side GPG lines in `faraday/faraday-core/src/vault/mod.rs` were not re-verified by the inventory.
- Whether the "name or email" refusal in `faraday/faraday-core/src/vault/mod.rs:~3149` and the backup flow's "both empty" refusal (`faraday/faraday-core/src/backup_flow/steps.rs:2165`) are the same rule seen from two places, or two different rules, is not settled.
- `GpgReason` is clamped to 3 without the count of reasons being visible.
- da6c56035's file list shows only `faraday/faraday-core/src/vault/mod.rs`. bd1a3959d's file list was not fully read.
- `gpg::SKEW` is unused in `faraday/faraday-core/src/gpg.rs` itself; `gpg_renew` uses it.
- The comment "puts nothing For the stick" in `faraday/faraday-core/src/backup_flow/gpg_restore.rs` is stale or garbled.
