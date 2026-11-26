# One-time codes

The TOTP codes screen shows a live one-time code for every vault entry that
holds a TOTP secret, across all open vaults, and the vault's masked secrets
(TOTP secret, password, SeedQR, paperkey) are drawn behind an eye. It is for
the person who keeps their authenticator secrets in a vault on the offline
device or in Faraday Demo. It is reached from Vault contents (the TOTP codes
button and an entry's code row), and its rows open the entry in the vault.

## Screens and sheets

- Codes `Screen::Codes` (`faraday/faraday-core/src/codes/screen.rs:~183` desktop, `~234` compact): one row per vault record with a TOTP secret across all open vaults: the live code, seconds left, a progress bar, or a badge (HOTP, Unreadable, Set the time first).
- Top row: local time and zone with Set the time (`Action::OpenSetTime`, `faraday/faraday-core/src/codes/screen.rs:~29-40`).
- Search bar (`SearchIn::Codes`, `~166-171`), shown only with rows or a query.
- Scan a setup (`VaultAction::ScanEntry`) and Add an entry (`Action::CodesAdd`); the inline `codes_form` when `vaults.form` is set (`~203-225`, `~255-282`).
- Empty state: "search none" with a query, else "no TOTP entries" (`~175-181`).
- On an entry's item page (`faraday/faraday-core/src/vault/screen/codes.rs`): the TOTP code row (`entry_code_row`), seconds left (`left_of`, `~18-20`), HOTP and Unreadable badges (`NotTotp::Hotp/Unreadable`, `~63-71`), Set the time when there is no clock (`Action::OpenSetTime`, `~50-61`), Show as QR (`V::TotpQr`), the Add an entry form.
- Masked secret (`faraday/faraday-core/src/vault/screen/secret.rs:~8-79`, `~168-195`): 24 dots until the eye is pressed; the value is built into a `SecretText` only while shown (`~61`). `seedqr_box` / `code_with` (`~114-166`) draw a SeedQR or code behind an eye (`V::ShowSeedQr`) with `crate::screens::qr_code`.

## Actions

- `Action::CodesOpen(vault, record)`: a row press opens the entry.
- `Action::OpenSetTime`, `VaultAction::ScanEntry`, `Action::CodesAdd`, typing in the search bar.
- `V::TotpQr`, `V::PaperkeyQr`, `V::ShowSeedQr` on the item page; `V::FormSave`, `V::FormCancel` on the form.
- Presses are handled by the vault area's glue (`faraday/faraday-core/src/codes/mod.rs:~1-6`): `open_codes`, `open_code_row`, `codes_add_entry`, `codes_act` (`faraday/faraday-core/src/vault/mod.rs:~3003-3036`).

## Effects

- Non-held path `codes_rows`: `app.vaults.store().totp(ov.id, at, now, Reveal::VaultEntry(Part::Totp))` reveals the entry's secret and computes one HMAC per row (`faraday/faraday-core/src/codes/mod.rs:~152-163`).
- Held path `codes_rows_held` (drawing, `~114-116`) reads `app.vaults.codes_held()` (`~125`): no reveal, no compute at draw.
- Codes are cached by `vault_codes_refresh` (`faraday/faraday-core/src/vault/mod.rs:~4274`) and `vault_code_refresh` (`~4338`); one HMAC is counted per code (`work::count(Work::TotpCode)`, `~4306`, `~4376`). The held code on the item page is `entry_code_held()`, a borrow made once per period (`faraday/faraday-core/src/drawing/allowed.rs:~85`).
- Clock: `app.clock_now()`, `app.tz_min()`; for TOTP only `clock_now().filter(clock_checked)`.
- No write, camera (except Scan a setup), or clipboard.

## State and lock

- `CodeKind` (`faraday/faraday-core/src/codes/mod.rs:~19-39`): `Code { grouped: Text, left, period }`, `Hotp`, `Unreadable`, `NoClock`, `NotHeld`. It holds the shown code in clear in a `Text` (which zeroizes on drop). `CodeRow` is built per draw and not kept.
- The held codes are `Vaults.code: Option<CodeHeld>` and `Vaults.codes: Option<CodesHeld>` (`faraday/faraday-core/src/vault/area.rs`). They are dropped when the screen is left or when `lock_pending()` (`faraday/faraday-core/src/vault/mod.rs:~4274`, `~4338`).
- `edit` clears `code`/`codes` (`faraday/faraday-core/src/vault/area.rs:~1751-1761`). `hide_secrets` at a lock keeps `code` (taken and put back); `close_all` at the seal's end drops `code` and `codes` (`faraday/faraday-core/src/vault/area.rs:640-651`, `2932-2941`).
- A lock drops the vault key's SeedQR (fix 92e00f6f0). The codes screen has no lock logic of its own; the hold is the vault's.

## Reach

- Opened from Vault contents (`vault-contents`): `Action::OpenCodes` and an entry's code row.
- Opens Set the time (`Action::OpenSetTime`), the camera (Scan a setup), the entry form and the vault entry.
- `shows_clock` is true for Settings, Set the time, Codes, VaultContents' Entries category and Sign (`faraday/faraday-core/src/vault/mod.rs:~2916-2928`).
- Uses `crate::vault::screen::{search_bar, codes_form}`, `crate::vault::{tz_offset_text, time_of_day_secs, local_secs, Focus, SearchIn, VaultAction}`, `crate::screens::bar_head_back`, `crate::compact::{M, screen::finish}`, `crate::drawing::View`, `codes::totp_qr_button`.

## Rules and refusals

- One row per entry with a TOTP secret (`faraday/faraday-core/src/codes/mod.rs:~1-2`, `~128`). Held path: a record the hold says has no row gets none; one not yet held keeps a row with `NotHeld` (`~133-138`, `~170`). Non-held path: an unreadable TOTP gets no row (`~157-158`).
- HOTP: no code, a badge (`~30-31`, `~178`). Unreadable: a badge (`~32-33`, `~179`).
- No code is drawn unless the clock is known and checked (`clock_now().filter(clock_checked)`, `faraday/faraday-core/src/vault/screen/codes.rs:32`). Clock unknown: badge "Set the time first", no code worked out (`faraday/faraday-core/src/codes/mod.rs:~34-35`, `~171`).
- Opening Codes, or a vault for Codes, with no clock checked this power-on opens Set the time first (`faraday/faraday-core/src/vault/mod.rs:~3003-3010`, `~1383-1385`).
- Code comparison is constant time through `subtle::ConstantTimeEq` (`faraday/faraday-core/src/codes/mod.rs:~41-57`); `Debug` prints kind and seconds only, never the code (`~69-84`).
- Seconds left = `period - (now mod max(1, period))` (`faraday/faraday-core/src/codes/mod.rs:140-146`); at an exact boundary it reads the full period, not 0.
- Account: the URI label's account, else field 2 (username), else empty (`~94`, `~166-168`); title fallback `codes_entry` (`~165`); the vault name is shown only when more than one vault is open (`~96`, `~124`, `~186-190`).
- Progress clamped to 0..1, minimum bar 6 dp (`faraday/faraday-core/src/codes/screen.rs:~126-134`). Scan is filled-primary only with no rows and no query (`~209-213`, `~264-268`). Compact: the time row is hidden while search has focus; no buttons in the search view (`~239-282`).
- TOTP input (`faraday/faraday-vault/src/totp.rs`): a Steam otpauth URI (`encoder=steam` or `encoder=STEAM`) is Unreadable and no code is read from it (fixes 83a6a67e5, 5c3897094); a repeated key-URI parameter is ambiguous and not read; parameter names are read only in lower case without percent-escapes (fix 62d9f8c1d); every TOTP form is read (see `vault-unlock`, fix 5836c4d64).
- An entry's TOTP field left unchanged stays as it was even if it does not read; a changed one is checked as any typed input (`faraday/faraday-core/src/vault/mod.rs:~2129-2160`; fix 5c3897094). A value that is not a TOTP key is refused with `vaults_not_a_totp_key`; a scanned text that is not a setup code gives `vaults_not_a_totp_setup` (see `vault-contents`).

## Domain calls

- `faraday_keys::totp::NotTotp`, `faraday_keys::totp::normalise` (`faraday/faraday-keys/src/totp.rs`).
- `faraday_keys::{Reveal::VaultEntry, Part::Totp}` (`faraday/faraday-keys/src/session.rs`, `faraday/faraday-keys/src/parts.rs`).
- `OpenVaults::totp` (`faraday/faraday-keys/src/vault/reveals.rs`).
- `faraday_vault::records::field::{TITLE, USERNAME}` (`faraday/faraday-vault/src/records.rs`); `faraday_vault::totp` (`faraday/faraday-vault/src/totp.rs`); `faraday_vault::entries::otpauth_names` (`faraday/faraday-vault/src/entries.rs`).
- `subtle`, `faraday_text` (`core/faraday-text/src/text.rs`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 92e00f6f0 | Locking did not drop the vault key's SeedQR; a secret stayed reachable in app state after a lock | A lock drops the vault key's SeedQR; words, paperkey and TOTP codes are computed once and held while shown |
| 74995c2e2 | With the clock unknown TOTP codes were worked out at a fixed time and shown | With no clock the codes say "Set the time first"; no code is worked out |
| 83a6a67e5 | A Steam otpauth URI was read with encoder=steam ignored, showing a 6-digit code Steam refuses, with no warning | A Steam URI is Unreadable; Faraday reads no code from it |
| 5c3897094 | Saving an entry whose TOTP field did not read said "Not a TOTP key" on every save, so only blanking the field could get out, losing the key | An unchanged TOTP field stays as it was; a changed one is checked as typed input; encoder=STEAM refused like encoder=steam |
| 62d9f8c1d | An otpauth URI with a repeated parameter was read, taking the last secret, digits, period or algorithm | A repeated key-URI parameter is ambiguous and not read; parameter names read only in lower case without percent-escape |

## Doubts

- Rows left out: e50c59a3c (the by-hand copy grid of a Krux code; not one-time codes); 6b6f01d22 (held keys' address and key codes and the power-off hold) was moved to `lock`.
- 92e00f6f0's file list was partly cut.
- `faraday/faraday-core/src/codes/mod.rs:~118-119`: the doc of `Given` is stale ("Set the time" is not a case); `faraday/faraday-core/src/codes/screen.rs:~227` has an unused `h`. Who calls `codes_rows` (the non-held path, one HMAC per row) is not known.
- "HOTP" is hard-coded English (`faraday/faraday-core/src/vault/screen/codes.rs:~65`).
- Code line numbers in `codes/` other than `140-146` are not inventory-verified.
