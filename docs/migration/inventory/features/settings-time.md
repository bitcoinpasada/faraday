# Settings and the clock

Settings (mode, session, time, randomness, appearance, display, remembered data, About), the settings file on the boot stick that carries a person's choices across power-offs, Set the time by hand (the app keeps its own offset against the shell's clock), and the version line. Settings is a sidebar tab and a menu page; Set the time opens from Settings or from any screen whose action needs a checked clock, and returns there.

## Screens and sheets

- Settings (`Screen::Settings`, `faraday/faraday-core/src/screens/settings.rs`, `faraday/faraday-core/src/screens/settings/rows.rs`).
  - Sections (`parts()`, `faraday/faraday-core/src/screens/settings.rs:~132`): Mode, Session (lock, remove keys, power), Time, Randomness, Appearance, Display, Remembered, About.
  - On the compact layout a list (`listed`, `:~507`) opens a section as its own page (`SettingsSection(Some(k))`) or the mode sheet (`ModeSheet`).
  - About: the version, the self-test result, the link to `nakamotoinstitute.org` (`faraday/faraday-core/src/screens/settings.rs:~614`).
- Set the time (`Screen::SetTime`, `faraday/faraday-core/src/screens/set_time.rs`, logic in `faraday/faraday-core/src/set_time.rs`): Date and Time fields, a zone row, Set, Keep this time, Back. The zone list replaces the fields when `set_time.zones()` (`TzList(false)`, `TzSet(min)`).
- The Settings row on Files: offered when a setting differs from what the boot stick holds.

## Actions

- Settings: `Theme`, `TextSize`, `Motion(MotionStyle)`, `Speed(MotionSpeed)`, `LockAsk`, `RemoveKeys`, `PowerAsk`, `IdleLock(min)`, `IdleOff(min)`, `OpenSetTime`, `Own(fresh::page::Act::Open)`, `Scale(pct)`, `Rotation(Orientation)`, `SealAmounts(bool)`, `NonceHistory(bool)`, `SelfTestRun`, `OpenInstitute` (online only).
- Set the time: `SetTimeSet`, `SetTimeKeep`, `SetTimeBack`, `SetTimeFocus`, `TzList`, `TzStep`, `TzSet`.
  - Type the date `YYYY-MM-DD` and the time `HH:MM` (24 h); only digits, `-` and `:` are typed; Tab swaps fields; Backspace; Escape goes back, or closes the zone list.

## Effects

- Set: turns the typed local time into an offset against the shell's clock (`vaults.set_clock_offset`), marks the clock checked, toasts and saves the boxes. It never writes the computer's clock (`set_time_set`, `faraday/faraday-core/src/set_time.rs:411-432`).
- The settings file: `StorageCommand::Read` of the file at first look; written to the boot stick through Files; `save_boxes`.
- `Command::Orientation` for Rotation.
- `SelfTestRun` runs the self-test (toast; result shown in About).
- `OpenInstitute` opens the link, online only.
- The desktop shell sends `StorageEvent::Clock { unix_secs }` at each rescan (`faraday/shells/demo/src/lib.rs:~264-269`).
- `--version` on the desktop app prints its version line before any other setup (`faraday/shells/demo/src/main.rs:~641-652`).

## State and lock

- `SetTimeArea` (two `TextBox`es, focus, `back_to`, sheet, zones) is `at_lock::Public`: nothing wiped. The clock offset and the checked flag live in `vaults` and are kept.
- `Settings` (`faraday/faraday-core/src/stick/settings.rs`) is `at_lock::Public`, "kept across a lock" (`faraday/faraday-core/src/stick/settings.rs:92`).
- No secrets.
- Every lock ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`); the settings and the clock offset are carried through the saved boxes.

## Reach

- Opens Set the time, Add randomness (`randomness`), the mode sheet (`modes::screen::settings_row`, `faraday/faraday-core/src/screens/settings.rs:196, 685`), Lock and Power (`lock`).
- Set the time is opened by `time_asked` from actions that keep a time: GPG key creation and signatures (`gpg-keys`), Secure Boot certificates, the TOTP codes (`totp-codes`).
- Read by the motion and page turn (`motion`, `pane-container`), the idle timers (`lock`), the search (`menu-pages`).
- Reaches `fresh::page::Act::Open`, `stick::settings::{IDLE_LOCK_CHOICES, IDLE_OFF_CHOICES}`, `stick::settings_choice::*`, `panes::*`, `version_label()`, `faraday_shell_api::Orientation`.

## Rules and refusals

- Set the time (`faraday/faraday-core/src/set_time.rs`):
  - The date must be a real calendar date, 2020 to 2099; the time `HH:MM`, 24 h (`faraday/faraday-core/src/set_time.rs:225-255`).
  - The Set button needs a zone chosen (`tz_known`) and a ready date and time (`set_time_ready`, `faraday/faraday-core/src/set_time.rs:379-381`).
  - Enter does not need the zone: `Typed::Set` goes straight to `set_time_set`, so Enter sets with the default offset (`faraday/faraday-core/src/set_time.rs:186-196, 462-465`).
  - Keep this time marks the clock checked only if a clock reading exists; it sets no offset and needs no zone (`faraday/faraday-core/src/set_time.rs:442-449`).
  - With no offset, a clock below `CLOCK_FLOOR` is "not known".
  - `time_asked` opens Set the time and says so when the clock is not checked (`:~362-375`).
  - On the screen, Set is primary only when `set_time_ready()` (`faraday/faraday-core/src/screens/set_time.rs:~176, ~227`); Keep only when a clock is known and the zone is known (`faraday/faraday-core/src/screens/set_time.rs:179, 230`).
  - Zone limits -12:00 to +14:00 are applied in `settings.step_tz`.
- Settings screen (`faraday/faraday-core/src/screens/settings.rs`):
  - Lock and Remove every key only when `holds_secret()`; Remove styled Danger; the Power off button reads Quit online (`faraday/faraday-core/src/screens/settings.rs:228-240`).
  - The panic-key hint only offline (`:~243-245`).
  - The idle Lock timer is always shown; the Power off timer only offline (`faraday/faraday-core/src/screens/settings.rs:654-669`).
  - Ignored inputs give a warning line (`:~253-261`).
  - Speed is dimmed when Motion is Off (`:~66`).
  - Rotation only where `rotation_offered()` (`:~365`).
  - Scale choices 75, 90, 100 (Auto), 110, 125, 150, 200 (`faraday/faraday-core/src/screens/settings.rs:325`).
  - Idle 0 reads Never (`:~319-320`).
  - Remembered amounts and signatures: "In open vault" or "Until power off" (`:~421-446`).
  - About: self-test Passed, or Failed with a count, or not run (`:~476-483`); the link pressable only online (`:~616-618`).
  - The compact Mode card only on compact layouts (`:~176-178`).
  - Search: a card matches only when every word of the find is among its drawn words (`faraday/faraday-core/src/screens/settings/rows.rs:~97-104`).
- Settings file on the boot stick (`faraday/faraday-core/src/stick/settings.rs`):
  - Name `faraday-settings.txt`, header `faraday-settings 1`, at most 4096 bytes, UTF-8; anything else is ignored whole (`faraday/faraday-core/src/stick/settings.rs:26-32, 499-503`).
  - Lines are `key=value`; unknown keys are ignored.
  - Keys: `scale` (in `SCALES`), `rotation`, `theme`, `mode`, `contrast`, `text`, `motion` (the legacy full and reduced become Smooth and Off), `speed` (Quick 250 ms, Medium 400 ms, Relaxed 600 ms), `guided` (ignored), `qr-ms` (in `QR_SPEEDS`), `idle-lock` (2, 5, 10, 30, 0 min), `idle-off` (10, 20, 30, 60, 0 min), `tz` (clamped to -720..=840 minutes).
  - `idle-lock=0` and `idle-off=0` (never) are refused when read from a stick (`faraday/faraday-core/src/stick/settings.rs:365-380`). A person may still choose Never in Settings.
  - Only a chosen motion and a chosen zone are written (`body`, `:~272-295`).
  - The Files row only when a setting differs from the boot stick's (`settings_changed`, `:~564`). A read from a stick that is not the boot stick is ignored.
  - `idle_off_at`: power-off counts only when `idle_off_min > 0`, not online, not lockable, the Outbox empty and not on a flow screen (`faraday/faraday-core/src/stick/settings.rs:414-420`; also `faraday/faraday-core/src/state.rs:2655-2671`).
  - Rotation is offered only when the shell says it can turn; the same orientation does nothing (`:~455, ~602-605`).
  - `layout_scale` drops a `Command::RequestEntropy` added by a display call (`:~483-491`).
- Version (`faraday/faraday-core/src/version.rs`):
  - `version_label` (`:~118-123`) is `faraday_paper::build::describe(build)` when `FARADAY_BUILD` is set (`faraday/faraday-core/src/version.rs:120`), else the crate version marked local.
  - `release()` (a `OnceLock`); `is_release` = a build ID with no `+`.
  - `Program::{StarterApp, DesktopApp}` names (`:~47-62`).
  - `parse_version_output` (`:~89-100`): the first line, trimmed, must be the program name (or an old name) followed by a space and a non-empty build string, else `None`. The Starter uses it to tell its own files from the desktop app's.
  - Compile-time refusal: feature `handwriting` with a build ID not ending `.dev` does not compile (`:~35-43`); no ID (cargo test or clippy) is let through (`:~31-34`).
- Since the history fixes: a time is recorded or shown (seals, a new vault, a KeePass export, the write receipt, vault summaries, a sealed QR picture) only once the clock is checked (`checked_now`); the desktop reads Quit and shows the power-off timer row only where it applies; zone rows wrap their city names.

## Domain calls

- `faraday/faraday-paper`: `build::describe` (`faraday/faraday-core/src/version.rs:120`).
- `core/faraday-shell-api`: `Orientation`.
- `faraday/faraday-strings`: `zones::ZONES`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 7aac0e62f | The time zone list cut a zone's city names with an ellipsis instead of showing them. | Zone rows wrap their cities onto as many lines as needed and grow. |
| c3083b919 | Seals, a new vault, a KeePass export, the write receipt, vault summaries and a sealed QR picture recorded or showed the time from a clock the person had not checked. | A time is recorded or shown only once the clock is checked (`checked_now`). |
| 79790e50b | On the desktop app (online) Settings and the Power sheet said "Power off", and Settings showed a "Power off after timer" row, though the app does not power off; it closes. | The desktop reads Quit and says Faraday closes; the timer row is shown only where it applies. |

## Doubts

- Set the time: Enter sets with no zone chosen, and Keep marks the clock checked with no zone, while the Set button needs one. The button and the keys disagree (confirmed in code).
- The zone is documented as a multiple of 15 minutes, but `set_tz` only clamps (`faraday/faraday-core/src/stick/settings.rs:~215-218`).
- Why `layout_scale` drops `Command::RequestEntropy` is not known.
- The Settings search words omit some drawn strings (`settings_loaded`, `settings_ignored_until`, remembered, self-test, `own_status`); a likely gap.
- `nakamotoinstitute.org` is a literal outside `EN`.
- 79790e50b also dropped the Pi's QR sheet PNG button; whether that part was a fault is unclear.
- Two figures for the idle power-off conditions were checked in two places (`faraday/faraday-core/src/stick/settings.rs` and `faraday/faraday-core/src/state.rs`); which one the app uses at each call was not traced.
