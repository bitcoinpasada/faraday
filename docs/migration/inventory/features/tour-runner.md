# The tour runner

The tour runner is the snapshot tool `faraday-snapshot`
(`faraday/shells/snapshot/`): a command-line program for developers that
builds a `Faraday` app with no hardware, drives one tour (or the default
tour) through it, and writes a PNG per step. It answers the app's storage,
entropy and exit commands itself, checks after presses that no planted test
secret is still on screen after a Lock, a hide or leaving a page or sheet,
and writes a report of touch targets under the touch floor. It is reached
from the command line and from the repository's render recipes; it is not
shipped in any app.

## Screens and sheets

None of its own. It draws the app's frames off screen
(`Tour::save`, `faraday/shells/snapshot/src/tour.rs:529`) and the bare
canvases some tours make (`save_canvas`,
`faraday/shells/snapshot/src/tour.rs:547`).

## Actions

- Command line: `faraday-snapshot [--sd-card] [--network NAME] WxH[@DPI]
  TESTKIT_DIR OUT_DIR [TOUR]` (`faraday/shells/snapshot/src/main.rs:165`).
  - `--sd-card` selects `Medium::SdCard`; the default is `Medium::Stick`
    (`faraday/shells/snapshot/src/main.rs:143-148`).
  - `--network` takes mainnet, testnet, signet or regtest
    (`faraday/shells/snapshot/src/main.rs:150-156`) and sets it through
    `testkit::use_network`.
  - `WxH` is the window in pixels; `@DPI` defaults to 160
    (`faraday/shells/snapshot/src/main.rs:191`).
  - `TESTKIT_DIR` is the kit folder (the stick's files); `OUT_DIR` is where
    pictures and reports go (created if missing).
  - `TOUR` is a name from `TOURS`
    (`faraday/shells/snapshot/src/tours/mod.rs:102-229`); with no name the
    default tour runs (`faraday/shells/snapshot/src/main.rs:127-131`). There
    is no tour named "default".
- Environment:
  - `FARADAY_SNAP_THEME=<theme>[,<mode>[,<contrast>]]` presses
    `Action::Theme` (`faraday/shells/snapshot/src/main.rs:91-95`).
  - `FARADAY_SNAP_TEXT=standard|large|larger` presses `Action::TextSize`
    and creates an empty `cut.txt` in the output folder
    (`faraday/shells/snapshot/src/main.rs:100-110`).
- Run setup (`run`, `faraday/shells/snapshot/src/main.rs:69`):
  `Faraday::new()`, the medium set, `display(size, dpi)`, then
  `offer_rotation`: on the 7-inch panel (1024x600 or 600x1024 at 170 dpi,
  landscape natively) and the 10-inch panel (800x1280 or 1280x800 at 150 dpi,
  portrait natively) it sends `RotationOffered{native}`, and when the render
  is portrait it presses `Rotation(Portrait)`
  (`faraday/shells/snapshot/src/main.rs:40-66`). The session is moved to
  the test network with `testkit::move_to(..., testkit::net())`
  (`faraday/shells/snapshot/src/main.rs:86`). Then the theme and text size,
  the `Tour` built (`faraday/shells/snapshot/src/main.rs:112`),
  `leaks::plant()` (`faraday/shells/snapshot/src/main.rs:127`), and the
  tour run.
- A `Compute` token is minted for the tool, with an
  `#[expect(clippy::disallowed_methods)]` naming it a mint
  (`faraday/shells/snapshot/src/main.rs:195-200`), and passed to costly
  tours through `run_named`
  (`faraday/shells/snapshot/src/tours/mod.rs:238`).

### The `Tour` driver (`faraday/shells/snapshot/src/tour.rs`)

- `pump` (`faraday/shells/snapshot/src/tour.rs:163`) drains app commands:
  - `RequestEntropy` is answered with `EntropySource("getrandom(2)")` and 32
    bytes of `0x5a ^ counter`; the counter advances per request, so runs
    repeat and each request differs
    (`faraday/shells/snapshot/src/tour.rs:168-175`).
  - `Command::Exit` calls `restart`
    (`faraday/shells/snapshot/src/tour.rs:176-177`).
  - Storage commands: `Read` from the kit folder; `Write` answered
    `Written`, or `NameTaken` (with the on-stick name and sha256) when the
    target stick folder already holds a file of that name (compared without
    case) and `on_taken` is `Ask`
    (`faraday/shells/snapshot/src/tour.rs:~197-226`); `Print` writes into
    the output folder; `ReadQr`, `ReadHost`, `SaveDownload` (fails "no
    Downloads folder" without a host), `StickSpace` (6.65 GB free of 7.95
    GB), `SaveBoxes` (stored into `t.boxes`), `Hash`. `OpenDownloads`,
    `OpenInstitute`, `ForgetBoxes`, `Wipe`, `Rename`, `NameStick`,
    `ZeroFree` and `MakeEmpty` are ignored.
- `restart` (`faraday/shells/snapshot/src/tour.rs:334-368`) stands for a
  lock or power cycle (see State and lock).
- `press` (`faraday/shells/snapshot/src/tour.rs:384-410`):
  - Before a Board press other than Customize, Software, SeesKeep or
    SeesUndo on a plan that is not custom, it presses `Board(Customize)`
    first.
  - After any Board press that opens `Sheet::BoardSees` it presses
    `SeesKeep` (the std tour shows both itself).
  - It runs the leak check around the press (`leak_before`, `leak_after`)
    and pumps.
  - When `go_on` is set and the Devices sheet is open with reminders, it
    presses `DevicesUnderstood`.
- `tick` (`faraday/shells/snapshot/src/tour.rs:377`): draws, advances 1000
  ms, pumps.
- Shots: `shot(name)` is `shot_after(name, 4000)`
  (`faraday/shells/snapshot/src/tour.rs:493-526`): pump, advance, tick,
  draw twice, `settle`, save. `shot_moving` (`faraday/shells/snapshot/src/tour.rs:506`) ticks `ms` and
  saves without settling. `shot_unanswered` (`faraday/shells/snapshot/src/tour.rs:515`) leaves commands
  unanswered.
- `save` (`faraday/shells/snapshot/src/tour.rs:529`): a counter `n`;
  writes `NN-name.png` only when `pictures` is set; collects touch targets
  under the floor when `targets` is set (`note_small`,
  `faraday/shells/snapshot/src/tour.rs:567`: the floor is
  `tokens::touch(window)` scaled by dpi/160); when `FARADAY_SNAP_TEXT` is
  set, records text cut at a side or drawn over other text in `cut.txt`
  (`note_cut`, `faraday/shells/snapshot/src/tour.rs:597`).
- `stick()` (`faraday/shells/snapshot/src/tour.rs:124`): the kit folder's
  files as one stick labelled "TESTSTICK" (not a boot stick), vaults found by
  content (`faraday_codec::vault_file::is_vault_file`), plus a fake
  "IMG_0412.jpg" of 2,300,000 bytes
  (`faraday/shells/snapshot/src/tour.rs:145`). The stick's id is the kit
  folder's path text. `sticks(on)` (`faraday/shells/snapshot/src/tour.rs:472`) puts it in or takes it
  out.
- `copy_wallets` (`faraday/shells/snapshot/src/tour.rs:414-416`) copies
  wallet descriptor files in from the stick, and does nothing while a key is
  held.
- `load_kit` (`faraday/shells/snapshot/src/tour.rs:447`) copies wallets,
  opens each kit's `<id>-wallet.txt` plus `musig-wallet.txt` from Files and
  confirms them, types test key 0 if no test key is held, selects wallet 0
  and goes to WalletsStart.
- `type_key(n)` (`faraday/shells/snapshot/src/tour.rs:439`) types test seed n and presses `EntryAdd`.
  `decode(c)` (`faraday/shells/snapshot/src/tour.rs:480`) decodes QR codes from the current frame.
- Other helpers in `faraday/shells/snapshot/src/tour.rs` (lines 620 to 1010,
  names only): `test_key_1`, `copy_digits`, `type_words`, `bmap`, `bshape`,
  `bput`, `boff`, `vault_stick`, `vault_place`, `keep_vault`, `files_zone`,
  `description_file_at`, `seed_files_at`, `shares_on_paper`, `to_checklist`,
  `scroll_to`, `device_key`, `scroll`, `open_made`, `type_text`,
  `flow_at_make`, `flow_made`, `key_text_of`, `not_now_load`, `laptop`,
  `write_png`, `display`; `backup_as` is at `faraday/shells/snapshot/src/tour.rs:119`.

## Effects

- `NN-name.png` per shot in the output folder.
- `targets.txt` in the output folder, one line per counted target
  (`targets::write`, `faraday/shells/snapshot/src/targets.rs:62`, called at
  `faraday/shells/snapshot/src/main.rs:215`). Each line names the tour (or
  "default"), the step, the screen, the sheet, the action, the target's size,
  the floor and the drawn rectangle (`faraday/shells/snapshot/src/targets.rs:~47-58`).
- `cut.txt` in the output folder when `FARADAY_SNAP_TEXT` is set.
- Files the app prints, answered `Print` commands, written into the output
  folder; files the app writes to the stick, written into the kit folder.
- Exit codes: 2 for a usage error (wrong argument count, a bad size or
  dpi, an unknown `--network` value, an unknown tour name); failure when
  `testkit::use_network` fails, when a tour returns `Err`, when the leak
  check found anything, or when `targets.txt` cannot be written; success
  otherwise (`faraday/shells/snapshot/src/main.rs:141-226`).

## State and lock

- `Tour` (`faraday/shells/snapshot/src/tour.rs`) holds the app, the output
  and kit folders, the shot counter `n`, the clock `now`, the sticks list,
  the entropy counter `answers`, the dpi, a host, the saved `boxes`,
  `go_on`, `pictures`, the collected `targets`, the tour name, the restart
  count and the `leaks` found
  (`faraday/shells/snapshot/src/main.rs:112-128`).
- `restart` (`faraday/shells/snapshot/src/tour.rs:334-368`):
  - counts the restart;
  - reads the queued storage commands and keeps the last `SaveBoxes`
    (inbox, outbox, kept) in `t.boxes`, through `testkit::plain_files` and
    `testkit::plain_kept`;
  - builds a fresh `Faraday::new()`: the whole session, its keys, vaults
    and sheets are gone;
  - restores the medium, the test network, the `laptop()` device set on the
    Stick medium, the display, and the theme if it differs.
  - Settings and the scheme survive; secrets do not. The sticks list
    `t.sticks` is kept by the `Tour` but not re-sent by `restart`. A later
    `StorageEvent::Restored` (or a tour's `only(...)`) reloads the boxes.
- Secrets held: only test data, the test seeds and vectors planted by
  `leaks::plant` and whatever the tours type. They go at a `restart` with
  the app; the planted forms stay registered for the thread
  (`faraday_core::leaks::plant`).
- `Tour::restart` keeps the lock's saved boxes, rebuilds the app with the same medium and network, display and theme, and does not tell it of the sticks; a tour that needs a stick after a restart sends `Sticks` (or `Tour::sticks`) and `Restored` from `Tour::boxes` itself (`faraday/shells/snapshot/src/tour.rs:334-364`, `:472-475`).

## Reach

- Runs every tour of the Tours feature (`run_named`,
  `faraday/shells/snapshot/src/tours/mod.rs:238`; `names`,
  `faraday/shells/snapshot/src/tours/mod.rs:232`).
- Reads the app through `faraday_shell_api::App` (`press`, `event`,
  `frame`, `poll_command`, `poll_storage`) and through accessors on
  `Faraday` (`board_plan`, `device_reminders`, `sheet`, `screen`, `theme`,
  `shown_forms`, `kept_images`).
- The ratchet file of touch targets is `tools/touch-targets.counts`, held by
  `just lint-touch-targets` (`faraday/shells/snapshot/src/targets.rs:1-8,
  ~74-77`).
- The leak check's rules are `docs/SECRET-LEAKS.md` §5; the touch-target
  rules are `docs/M3-CHECKS.md` §2 and §3.
- Sizes run: `just faraday-snapshots` runs the default tour at 13 window sizes, each at standard and Larger text; `just faraday-snap SIZE [TOUR]` one tour at one size; `just lint-touch-targets` every tour at 240x320@143, 480x640@286, 1024x600@170 and 1280x800@160 (`faraday/faraday.just:195-258`, `faraday/shells/snapshot/src/targets.rs:79-90`, `justfile:175-185`).

## Rules and refusals

- Command line:
  - `--network` with a value that is not a network name: "--network takes
    mainnet, testnet, signet or regtest", exit 2
    (`faraday/shells/snapshot/src/main.rs:150-156`).
  - Argument count other than 4 or 5 after the flags are taken out: the
    usage line with the tour names, exit 2
    (`faraday/shells/snapshot/src/main.rs:163-168`).
  - A size that is not `WxH`: "size must be WxH[@DPI]", exit 2
    (`faraday/shells/snapshot/src/main.rs:170-179`).
  - A dpi that is not a number: "dpi must be a number", exit 2
    (`faraday/shells/snapshot/src/main.rs:185-191`).
  - An unknown tour name: refused with the list of names, exit 2
    (`faraday/shells/snapshot/src/main.rs:200-204`).
  - An unknown word in `FARADAY_SNAP_THEME` is ignored silently
    (`faraday/shells/snapshot/src/main.rs:91-95`); an unknown word in
    `FARADAY_SNAP_TEXT` is an error "standard, large or larger"
    (`faraday/shells/snapshot/src/main.rs:100-110`).
- Leak check (`faraday/shells/snapshot/src/leaks.rs`):
  - `plant()` (`faraday/shells/snapshot/src/leaks.rs:39-62`) registers the
    test secrets' text forms: each `TEST_SEEDS` phrase ("the phrase of test
    key N"), the 24-word BIP-39 vector (`abandon` x23 `art`,
    `faraday/shells/snapshot/src/leaks.rs:27-29`), three BIP-93 codex32 strings (`faraday/shells/snapshot/src/leaks.rs:32-36`), a
    vault item password "correct-horse-battery-staple", a TOTP secret
    "JBSWY3DPEHPK3PXP", a vault note "Recovery codes".
  - A frame is drawn and checked after a press only when the frame before
    showed a planted form, or the core kept an image holding one, or the
    press locked (`faraday/shells/snapshot/src/leaks.rs:182-237`).
  - Events, in order of precedence (`faraday/shells/snapshot/src/leaks.rs:~207-217`):
    - "Lock": a restart happened, or the action is `Lock` or
      `LockUnsealed`, and the `LockAsk` sheet is not showing. Nothing
      planted may still show.
    - "a hide": the action's name contains hide, eye, reveal, cover,
      conceal or signtext (`faraday/shells/snapshot/src/leaks.rs:24`). What the frame showed before the
      thing was opened may stay.
    - "leaving a page": the screen changed. Nothing planted may show.
    - "leaving a sheet": the same screen, the sheet changed or closed. What
      the page under the sheet showed (from its frosted copy) may stay.
  - Both the frame's text (`shown_forms`) and every kept image (frost, fade
    frames; `kept_images`) are checked
    (`faraday/faraday-core/src/fade.rs:801, 845`). A hit is appended to
    `t.leaks` with the tour, step, action, event and screen.
  - The press log is capped at 256 entries (`faraday/shells/snapshot/src/leaks.rs:184`); it is cleared
    on a lock or a change of screen.
  - Any hit fails the whole run with "a secret is left after Lock, a hide or
    leaving a page (docs/SECRET-LEAKS.md §5)" and the list
    (`faraday/shells/snapshot/src/main.rs:132-137`).
  - This is the only wipe or lock check the tours make; it covers the
    planted forms only.
- Touch targets (`faraday/shells/snapshot/src/targets.rs`):
  - `short` counts a target when a side is under the floor and a neighbour
    trimmed that side; a side only its clip trims is not counted
    (`faraday/shells/snapshot/src/targets.rs:22-24`).
  - `excepted`: on a window under 600 dp wide, `Osk`, `EntryKey`,
    `EntryKeyBack` and `EntryShift` are exempt; `Codex32(Cell(` is always
    exempt (`faraday/shells/snapshot/src/targets.rs:30-43`).
  - The floor is 44 dp on the compact and short layout and 48 dp elsewhere
    (`faraday/shells/snapshot/src/targets.rs:1-4`), scaled by dpi/160 (`note_small`,
    `faraday/shells/snapshot/src/tour.rs:567`).
  - The counts in `tools/touch-targets.counts` only fall.

## Domain calls

- `faraday_core::Faraday`, `Action`, `Medium`, `Sheet`, `Screen`,
  `board::BoardPress`, `ui::Theme::from_words`, `ui::TextSize::from_id`,
  `devices::DeviceCheck` (`faraday/faraday-core/src/`).
- `faraday_core::leaks::plant` (`faraday/faraday-core/src/leaks.rs:33`),
  `Faraday::shown_forms` and `Faraday::kept_images`
  (`faraday/faraday-core/src/fade.rs:801, 845`).
- `faraday_core::testkit` (`faraday/faraday-core/src/testkit`):
  `use_network`, `move_to`, `net`, `TEST_SEEDS`, `test_words`,
  `plain_files`, `plain_kept`, `settle`.
- `faraday_bip::keys::Network::from_name`.
- `faraday_crypto::work::Compute` and `faraday_crypto::sha256`
  (`core/faraday-crypto`).
- `faraday_codec::vault_file::is_vault_file` (`core/faraday-codec`).
- `faraday_storage` (`faraday/faraday-storage`): `read_qr_png`,
  `read_host`, `save_download`.
- `faraday_text::Text` (`core/faraday-text`); `faraday_draw::geom::Rect`,
  `tokens::touch` (`core/faraday-draw`).
- `faraday_shell_api`: `App`, `Event`, `Orientation`, `EntropyBytes`.

## Fixtures

None of the kept fixture list directly. It reads the kit folder given on the
command line (the test kit's files) and `tools/touch-targets.counts` for the
ratchet test.

## Fixes

None.

## Doubts

- `faraday/shells/snapshot/src/tour.rs` lines 620 to 1010 (scroll helpers,
  `vault_place` and others) were not verified by the inventory; only names
  and lines of definitions are given.
- The ratchet test over `faraday/shells/snapshot/src/targets.rs` and the `every_tour_runs_to_its_end`
  test are test code and were not read.
- The `Write`/`NameTaken` line range (`faraday/shells/snapshot/src/tour.rs:~197-226`) and the leak
  events' range (`faraday/shells/snapshot/src/leaks.rs:~207-217`) were not re-checked line by line.
