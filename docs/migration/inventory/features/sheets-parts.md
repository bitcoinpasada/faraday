# Sheets and shared parts

The sheets drawn over screens (lock, power, idle, devices, scan, QR, new input, guide, secret out, wallet confirm, published words and others) and the shared screen parts that many pages use (buttons, kind rows, word cells, dice and card pads, the QR copy grid, the seed word grid, time zone rows, caution lines). Sheets open from any screen; the parts are drawn inside pages and sheets of every product.

## Screens and sheets

- Sheets (`faraday/faraday-core/src/screens/sheets.rs`), plain drawing functions; the `Sheet` enum lives in `faraday/faraday-core/src/lib.rs`.
  - Lock: `lock_sheet` (`:~558`), `lock_ask_sheet` (`:~604`), `remove_keys_sheet` (`:~694`), `kept_sheet` (`:~453`) with `kept_sections` (`:~398`), `write_out_sheet` (`:~724`).
  - Devices: `devices_sheet` (`:~282`), `devices_buttons` (`:~236`). Stick: `pull_sheet` (`:~354`).
  - Inbox: `potential_sheet` (`:~184`), `potential_body` (`:~30`).
  - Scan: `scan_sheet` (`:~781`), `scan_demo_offers` (`:~772`), the read card (`:~1076, ~1166, ~1188, ~1208`).
  - QR: `qr_sheet` (`:~1229`), `qr_sheet_compact` (`:~1367`).
  - Inputs: `new_input_sheet` (`:~1493`), `new_input_target` (`:~1595`).
  - Idle: `idle_warn_sheet` (`:~1705`), `locked_sheet` (`:~1809`). Power: `power_sheet` (`:~1871`).
  - Guide: `guide_sheet` (`:~2324`), `guide_body` (`:~2008`), contents, words, tabs, `GuideLook`.
  - Secrets: `secret_out_sheet` (`:~2412`), `secret_keeps` (`:~2531`), `plain_sheet` (`:~2555`, F129); wallet confirm (`:~2618`, F254); published words (`:~2699`).
- Shared parts (`faraday/faraday-core/src/screens/parts.rs`):
  - Buttons: `next_button_when`, `buttons_and_next_when`, `pin_button`, `stepper`, `info_button`.
  - Rows: `kind_rows`, `kind_list`, `option_list` (the Kind card with More kinds), `first_addresses` (`:~601`), `descriptor_row` (`:~963`), `file_row`, `made_place`, `made_state`, `file_choices`, `written_and_checked`.
  - Word cells: `typed_cells` (`:~1184`), `keyed_cells` (`:~1298`), `keyed_pills` (`:~1411`), `word_keys` (`:~1466`).
  - Pads: `dice_pad`, `coin_pad`, `card_pad`, `camera_row`. QR copy grid `copy_grid` and `CopyGrid` (`:~1975`). Seed words `seed_word_grid` (`:~2252`). Time zone `tz_row`, `tz_steppers`.
  - Empty-tool card `nothing_to_work_on` (`:~2297`), `ending` (`:~2378`). Busy bars `working_bar`, `busy_bar` (`:~23-49`).
  - Caution lines `never_real_seed`, `never_below`, `keys_read_as_us_line`.
- Shared helpers (`faraday/faraday-core/src/ui.rs`): `container_of` (`:~26`), `icon_tile` (`:~41`), `fill_pressed` (`:~55`); trait `CoreUi` with `fit_secret` (`:~86-118`, shortens with `…` and returns a wiping `SecretText`), `slider` (`:~94, ~131-177`), `no_passphrase` (`:~98, ~218`, the "No passphrase" pill).

## Actions

- Sheets: Cancel, NotNow, Lock, LockUnsealed, PowerOff, PowerAsk, HoldPowerOff, Restart, HoldRestart, RemoveKeysHold, Scan, `ScanFor(Ask::SpendPsbt)`.
- Devices: `DevicesCheck`, `DevicesContinue`, `DevicesHold`, `DevicesUnderstood`.
- Inbox: `PotentialType`, `TypeAgain`, `Kind`, `Make`.
- Scan: `ScanCamera(i)`, `ScanPaste`, `ScanSample`, `ReadShow`, `ReadGo(way)`, `ReadAgain`, `EntryWallet(wi)`, `SpendBackup(TypeWords)`.
- QR: `QrFormat`, `QrSpeed`, `QrPartSize`, `QrPng`.
- Inputs: `InputIgnore`, `InputUse`, `InputTarget`.
- Secrets: `SecretForm`, `SecretVault`, `SecretUnprotected`, `SecretHold`, `Vault(KeepTick)`. Wallet: `WalletHold`, `WalletCancel`. Published: `PublishedKeep`, `PublishedRemove`, `PublishedHold`.
- Guide: `GuideTabGuide`, `GuideTabHow`, `GuideContents`, `GuideHere`, `GuidePage(i)`, `GuideWord(i)`, `GuideClose`, `TypeHere(Here::GuideFind)`.
- Hold buttons (`vault::screen::hold_button_in`): power off, restart, remove keys, save unencrypted, add wallet, publish hold, devices continue, lock with a vault.
- Slider: `Action::Slide(id, n)` per stop; `lo` and `hi` swapped if reversed; value clamped (`faraday/faraday-core/src/ui.rs:~131-177`).

## Effects

- None of their own; every outward step is an action handled elsewhere.
- Camera only through actions. Clipboard only offered as `ScanPaste` or `PasteWords` (`faraday/faraday-core/src/screens/sheets.rs:~957`), gated on `app.scan_pastes()`.
- Secrets shown while drawing are borrows: `app.reveal(key, Reveal::CopyByHand)` (`faraday/faraday-core/src/screens/parts.rs:2258`) returns `faraday_keys::Revealed<'_>`, a borrow of the key store's own wiping text, neither `Clone` nor `Debug`; no copy is made (`faraday/faraday-keys/src/session.rs:250-263`, `faraday/faraday-core/src/drawing/allowed.rs:38`).
- Draw-time writes: `app.guide_scroll(max, scroll, section)` (`faraday/faraday-core/src/screens/sheets.rs:~2078`), `app.enter_line(d.id)` (`faraday/faraday-core/src/screens/sheets.rs:~1655`), `app.public_qr(g)` (`faraday/faraday-core/src/screens/sheets.rs:~1916`, the Secure Boot guide QR made once), `state.wrap_keep` (`faraday/faraday-core/src/screens/sheets.rs:~2190, ~2225`).

## State and lock

- Own no state. They read the owning areas: lock and idle, secrets, inbox, inputs, devices, scan, QR view, guide.
- Per-frame values only: `Cells`, `OptionRow`, `CopyGrid` (a borrowed QR matrix), `SheetLine`, `GuideLook`.
- `OnceLock` caches of static text (idle minute labels, scale labels).
- What a lock wipes is each owning area's; every lock ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`).

## Reach

- Opened by the screen frame's sheet dispatch (`screen-frame`).
- Lock and idle sheets (`lock`), secrets out and plain saves (`secrets`), the scanner and read card (`seeds`, `add-key`), the QR sheet (`public-files`), randomness rows (`randomness`), the guide sheet (`guide`), wallet confirm (`wallets`), published words.
- Sheets reach `randomness::{lock_seals_changes, row_in}`, `catalog::{search_field, words_in}`, `scan_frame::draw`, `pass_field::*`, `vault::screen::*`, `inbox::*`, `spend_backup::*`, `stick::pull_what`, `scan_read::*`, `inputs::*`, `idle::lock_at`, `secrets::*`, `guide::Lines`, `version::release`, `QrView`.

## Rules and refusals

- Devices (`faraday/faraday-core/src/screens/sheets.rs:~236-349, ~374-375`): none held gives Cancel and Understood; a refusing device gives only Power off, an error-tone line and no Continue hold; an asking device gives a hold "Continue anyway" (F136). Reminders are listed; due reminders come before a secret follows a pull (F24).
- Potential sheet (`faraday/faraday-core/src/screens/sheets.rs:~101-121, ~137-143, ~195-218`):
  - The passphrase takes effect only for seeds (not an xpub) whose first source takes one.
  - The field is shown if `pass_is_public` or `may_load_keys()`, else a stick field.
  - Make is primary only when `(may_load_keys || xpub) && !pass.differ() && !pass_eye.waits()`.
  - A fixed-path xpub gets no kind choice.
- Kept and wiped sheet (`faraday/faraday-core/src/screens/sheets.rs:~400-468`): holds are filtered from pressed actions; a mode change replaces the Then text; items kept nowhere else are in the error tone.
- Lock sheets (`faraday/faraday-core/src/screens/sheets.rs:~547-654`):
  - The seal nonce row only on LockAsk when `lock_seals_changes`.
  - Lock: "Scan instead" only with a camera and a spend PSBT waiting.
  - Lock ask: a seal waiting gives LockUnsealed only; unsaved vault changes that would be lost give LockUnsealed; changes short give "lock without saving" plus Lock; Back to the vault only when a vault is open.
  - Remove every key: `RemoveKeysHold` only (`faraday/faraday-core/src/screens/sheets.rs:~694-720`).
  - Write-out sheet: with no stick, a plug-in notice (`faraday/faraday-core/src/screens/sheets.rs:~735-736`).
- Scan (`faraday/faraday-core/src/screens/sheets.rs:~782-1135`):
  - Nothing drawn without a scanner.
  - The camera row only with several cameras.
  - Type the words only for a wallet-key scan, or an intake with `may_load_keys()`.
  - Paste only when not compact and `scan_pastes()` (F75).
  - The "never a real seed" line for the purposes Seed, KeyPart, WalletKey, Intake, Sweep.
  - Intake shows Done instead of Cancel. The read card replaces the picture.
  - Read card: secret kinds in the warning tone, text masked behind an eye, nothing copied; a key-loading way dimmed while a stick is in; Done only once filed.
- QR (`faraday/faraday-core/src/screens/sheets.rs:~1279-1340`):
  - The Parts format only for text or key sources.
  - A one-code QR hides format, speed and part size.
  - PNG only when `offers_png()` (public sources).
  - A secret QR is tagged "whoever scans" in the error tone; a public one "safe to scan" in the OK tone.
- New input (`faraday/faraday-core/src/screens/sheets.rs:1527-1535, 1626-1635`):
  - Keyboard: Use only for a pointer, not a keyboard (`:~1511-1517`). A "no code" error when it is a keyboard, there is no code and `randomness_gives_none()`.
  - Touch or pointer: an error when no target fits, or no spots with no randomness. Tries left `TAP_TRIES - missed` when missed > 0 (`:~1646-1650`). The Enter line only when `input_enter_lets_in()` (`:~1627, ~1653`).
- Idle warning (`faraday/faraday-core/src/screens/sheets.rs:~1706-1765`): shown only when `idle::lock_at(idle_lock_min())` is some. Sealed vaults are listed (changes > 0), with a seal line or "not sealed". No power-off line when the off minutes are 0 or online (`faraday/faraday-core/src/screens/sheets.rs:1764-1765`). Only Keep working.
- Locked sheet: the title is Locked or Not locked by `idle_followed_lock()` (`faraday/faraday-core/src/screens/sheets.rs:~1812-1817`). At most 5 Outbox names (`faraday/faraday-core/src/screens/sheets.rs:1811`); secret files in the error tone (`:~1853-1859`). A power-off countdown when `idle_off_at()` is some (`faraday/faraday-core/src/screens/sheets.rs:1840-1846`). Only Continue.
- Power (`faraday/faraday-core/src/screens/sheets.rs:~1872-1916`): Restart only when `offers_restart()`; a hold when the Outbox is not empty, a vault has changes, or something is kept nowhere; Quit online; the Secure Boot reminder and BIOS key row; the guide code only from a non-empty Secure Boot guide.
- Guide sheet (`faraday/faraday-core/src/screens/sheets.rs:~1972-2376`): tabs only when `how_offered()`; search hides the tabs while the keyboard is up; an empty search shows "no matches" plus Clear find; a held guide fills the frame, no box, no Done; Done is `GuideClose` inside the guide, else `Cancel`.
- Secret out (`faraday/faraday-core/src/screens/sheets.rs:~2413-2603`):
  - Plain secrets go to `plain_sheet`.
  - Save into a vault only when `keep != None`; "already in vault" when the vault holds the key or words; with no open vault, a vault way button.
  - Plain (F129): hold `SecretHold` to save unprotected; Put it in a vault only when `to_vault`.
- Wallet confirm (F254, `faraday/faraday-core/src/screens/sheets.rs:~2618-2693`): danger rows in the error tone; "of N" only when N > 1; Add by hold `WalletHold`; Cancel `WalletCancel`.
- Published words (`faraday/faraday-core/src/screens/sheets.rs:2704-2722`): sample keys give Remove only; keys written online give a hold to keep and no Keep button; else Keep (secondary) and Remove (primary).
- Shared parts (`faraday/faraday-core/src/screens/parts.rs`):
  - `never_real_seed` and `never_below` only where `app.seed_line` is set (the desktop) (`:~52-76`); `never_real_seed_if` only when typed (`:~80-93`).
  - `kind_rows` shows kinds 0 and 4 until More kinds (`:~296-333`).
  - A typed word not in the list is drawn in the error tone unless partial or the list has it (`:~1126-1130`); at most 8 candidates; "no word starts" in the error tone (`:~1258-1285`).
  - A suit is disabled with no rank, or when the card is already drawn (`:~1818-1825`).
  - `first_addresses`: when `receive_held(wl)` the addresses are replaced by `RECEIVE_WARNING` (`:~611-619`); only receive rows open a code (`:~659-662`).
  - `written_and_checked` is true only when `visit.out` is empty and the receipt holds files all verified (`:~1049-1056`).
  - Bring in from a stick only where `!stick.is_empty() && !holds_secret()` (`faraday/faraday-core/src/screens/parts.rs:2307`).
  - `size_text` uses decimal units (1 KB = 1000 B) (`:~1007-1017`).
  - `tz_row` shows "UTC" plus offset, or "Not set" (`:~1607-1617`).
  - The dice pad is one row of six if the touch size fits, else two rows of three (`:~1702-1706`).
  - The seed word grid is 4 columns wide, 2 on compact (`:~2252-2288`).
- Since the history fixes: every sheet opens at its top each time it opens, on wide and compact layouts.

## Domain calls

- `faraday/faraday-keys`: `Reveal::CopyByHand` (`faraday/faraday-core/src/screens/parts.rs:2258`).
- `core/faraday-entropy`: `RANKS`, `SUITS`, `card_index`, `CardDraws`, `FrameStats` (in `faraday/faraday-core/src/screens/parts.rs`).
- `core/faraday-codec`: `qr::QrMatrix` (`faraday/faraday-core/src/screens/parts.rs:~1947`, `faraday/faraday-core/src/screens/sheets.rs:~447`).
- `faraday/faraday-guide`: `glossary`, `guide_groups`, `GuidePage` (in `faraday/faraday-core/src/screens/sheets.rs`).
- `faraday/faraday-strings`: `bip::bip39_error`, `zones::ZONES`, `EN`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| c7f92d46c | A sheet on a wide screen took the page's scroll, and a reopened sheet kept its last scroll position, which could hide its first lines. | Every sheet opens at its top each time it opens, on wide and compact layouts. |

## Doubts

- Which `Sheet` variant opens which function here is decided in `faraday/faraday-core/src/screens/mod.rs:~503-585`; the mapping was not recorded sheet by sheet.
- Stale doc: the `lock_ask_sheet` doc sits above `qr_speed_labels` (`faraday/faraday-core/src/screens/sheets.rs:~594-598`).
- Literal strings outside `EN`: `"-"` and `"+"` (`faraday/faraday-core/src/screens/parts.rs:~1648-1652`), `"UTC"` (`faraday/faraday-core/src/screens/parts.rs:~1625, ~1673`), the `"••••••"` mask (`faraday/faraday-core/src/screens/parts.rs:~1111, ~1342`). Whether these count under the strings rule is not decided.
- `EntryCandidate(n as u8)` would truncate past 255 candidates (`faraday/faraday-core/src/screens/parts.rs:~1456`); arithmetic at `faraday/faraday-core/src/screens/parts.rs:~1935, ~1993` is unchecked but out of reach for QR sizes.
- `QR_PARTS[1]` is the default part size (`faraday/faraday-core/src/screens/sheets.rs:~1340`), a magic index.
- c7f92d46c is mostly a Secure Boot change; only the sheet-scroll part is a fix of this feature.
- Whether the local `Text` copies in typed word cells (`faraday/faraday-core/src/screens/parts.rs:~1266, ~1314-1315`) can hold secret words after the screen is done was not checked.
