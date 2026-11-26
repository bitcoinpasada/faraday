# Practice and Faraday Demo

Faraday Demo is the Faraday app in a desktop window, for trying Faraday on an
online computer with test data. It runs on Linux and Windows. It runs
standalone (`faraday/shells/demo`) or inside the Starter (Open Faraday Demo,
see `starter-app`). It starts in `Mode::Demo`: an online app, on testnet, in
the Practice style. Its session takes only the published samples.

Practice is the mode in which the session makes no secret a person could take
away (`makes_secrets()` is false). A press that would make one is refused with
one toast, and a "Use faraday" chip fills passphrase fields. Around the app,
the Demo host draws a strip along the window's foot. The strip holds:
- folder "sticks" to plug and unplug;
- a preview size;
- dice and codex32 aids;
- the Load samples sheet.

A lock in the Demo replaces the app with a fresh one in the same window.

## Screens and sheets

- The app's own screens, with the actions Practice refuses drawn dimmed (`demo_dimmed`, `faraday/faraday-core/src/demo.rs:86`). Eyes, Show buttons, and the scan-key and descriptor outputs are drawn as usual, and answer a press with the toast.
- The "Demo only" card with **Use faraday** under an empty passphrase field (`pass_chip_draw`, `faraday/faraday-core/src/demo.rs:263`). A field holding `faraday` in Practice is drawn unmasked in its code box (`pass_shows`, `:227-235`).
- Host strip (`strip_area`, `faraday/faraday-host/src/lib.rs:~334`; drawn by `faraday/faraday-host/src/screen.rs:~643`):
  - Stick pills (`Action::StickPill(n)`, `faraday/faraday-host/src/screen.rs:~166-182`, `~913`). The boot stick (n = 1) flashes while a step waits for it (`faraday/faraday-host/src/lib.rs:~840`, `faraday/faraday-host/src/screen.rs:~940-947`).
  - Aids: `DemoItem` buttons and lines; the codex32 "Use a sample"; lines about the hosted demo (`faraday/faraday-host/src/screen.rs:~203-242`).
  - The head is the mode's name (`~416`).
  - The dice roller: the computer's die, Roll/Stop, "Rolls per click". It covers `Roller::KeyGen` and `Roller::Vault` (`~277-318`, `~1025-1044`, `~1212`).
  - Size, Samples and More.
- Menus (`HostMenu`):
  - Sticks ("Sticks · N", `faraday/faraday-host/src/screen.rs:~1003`).
  - Size: the preview sizes (`~322`).
  - More, Rolls, Samples.
  - A menu is as wide as its widest row needs, up to the window margins.
- Load samples sheet (`samples_sheet`, `faraday/faraday-host/src/screen.rs:~1275`):
  - The sample passphrase (`faraday_core::samples::PASSPHRASE`).
  - Select all or Unselect all.
  - The wallets and keys, each with a tick box.
  - A warning to pull the medium.
  - Not now, and Load ticked.
- Top bar: the mode pill on Testnet and in Practice. The name FARADAY DEMO is drawn on one line where it fits, else DEMO sits under FARADAY (history fixes).
- `--panel` bare window: no strip and no preview (`faraday/faraday-host/src/lib.rs:~262-265`, `~280`, `~335`).

## Actions

- Any press Practice refuses: one `practice_refused` toast, and nothing else (`faraday/faraday-core/src/action.rs:1411-1415`).
- `Action::PassChip(field)` fills the empty passphrase fields with `PASSPHRASE = "faraday"` (`demo_pass_chip`, `faraday/faraday-core/src/demo.rs:318-395`). A pair's repeat hands the press of its first field, so one chip fills both where they are empty. It covers:
  - entry; keygen; the Inbox's potential key;
  - seed read; seed check;
  - Create's slots; a vault's phrases, unlock, prompt and entry passwords;
  - the backup check; the LND passphrase; the Core wallet tool.
- Strip (`faraday/faraday-host/src/lib.rs`):
  - A stick pill: plugs or pulls that folder stick.
  - `HostPress::Menu(Sticks | Size | More | Rolls | Samples)`: pressing the open menu closes it (`~562-572`). Samples also presses `Action::Samples` on the app (`~565-569`).
  - `HostPress::Size(..)`: closes the menu, sets the preview and sends `Command::PreviewSize` (`~574-579`).
  - `HostPress::App(action)`: an aid (`~554-561`).
  - `Action::Codex32(C32::Sample)` (`faraday/faraday-host/src/screen.rs:~215`).
  - Roll, Stop, Per and Count, through `Action::KRoller` or `Action::Vault(VaultAction::Roller(..))` (`~281-318`).
- Samples sheet:
  - `Action::SampleTick(Pick::Wallet | Key)` (`faraday/faraday-host/src/screen.rs:~1448`).
  - `Action::SamplesAll` (`~1380`) and `Action::SamplesLoad` (`~1479`). Not now (`~1489`).
  - Ticking and Select all keep the sheet open. Load and Not now close it (`faraday/faraday-host/src/lib.rs:~556-559`). The sheet's own surface presses nothing (`~573`).
- Demo command line (`faraday/shells/demo/src/main.rs:~540-632`):
  - `--text standard|large|larger`: presses `Action::TextSize(t)` (`~452-454`, `~527-528`, `559-565`).
  - `--transfer`: opens `Screen::Transfer`.
  - `--sticks DIR` and `--downloads DIR`.
  - `--size WxH`.
  - `--panel INCHES`: the app at a device screen's true size, in a window that cannot be resized (`~108-128`, `~402-423`, `~481-490`).
  - `--aspect W:H`, `--ppi`, `--timings`, `--help`/`-h`, `--version`.
- Also in the Demo: lock (restart), quit, Make PDF on a sheet, scan on and off, add randomness, paste, drop a file (see `desktop-shells`, `transfer`).

## Effects

- Practice: a refused press shows the toast only. The chip writes `faraday` into the fields' own state.
- Demo start (`faraday/shells/demo/src/lib.rs`):
  - Creates the sticks folder.
  - Writes the marker `.faraday-sticks` (text "BOOTSTICK and STICK-2 made").
  - Makes `BOOTSTICK` once, and `STICK-2` on every start (`455-466`, `~141-146`).
- Plugging a backup stick's pill creates its folder, empty; errors are ignored. The boot stick's folder is never created there (`374-381`).
- A listing thread reads Downloads and lists the cameras every 700 ms. It stops on `Drop` (`~131-135`, `~399-418`).
- The Demo sets the samples: `Session::on(mode.network())` and `set_demo(&SAMPLES)` with `faraday_samples::SAMPLES` (`~438-441`).
- `serve_with(app, boxes, Some(host), dirs)` (`~339-343`).
- A dropped file is stat-ed (`~306`).
- An EDID read of `/sys/class/drm/*/edid`, only with `--panel` and no `--ppi` (`faraday/shells/demo/src/main.rs:~132-155`, `~606`).
- Preview: `Command::PreviewSize` sets a preview size in dp (`faraday/shells/demo/src/lib.rs:~126-128`, `~202-205`). It is offered at start as `Event::PreviewOffered` (`faraday/shells/demo/src/main.rs:~182-184`, `~443-446`), but not in panel mode.
- The icon is `faraday/shells/starter/assets/icon.png`. The Windows resources come from `faraday/shells/demo/build.rs`, which includes `faraday/faraday-core/src/names.rs` and `faraday/build/windows/resources.rs`.
- A count of fades is printed to stderr (`faraday/shells/demo/src/main.rs:~362-384`).
- With `--timings` the Demo prints a once-a-second "fades" line (started, reduced, steps, median and worst gap, bands, size) on stderr after the frame's timings line; no release check (`faraday/shells/demo/src/main.rs:362-384`).

## State and lock

- Practice keeps no state of its own. `makes_secrets()` is `!session.keys.demo()` (`faraday/faraday-core/src/demo.rs:20-22`). The demo session is set up with `Session::set_demo` on Testnet.
- `Demo` (`faraday/shells/demo/src/lib.rs:~97-129`):
  - `sticks_dir`, `host`, the `listing` receiver, `stop`, `latest`, `listed`, `cameras`.
  - `boxes: Boxes` (Memory), `dirs`, `attached: BTreeSet<String>`.
  - `seen`, `toggles`, `served`, `next_scan`, `clipboard`, `preview`.
- Host state (strip, menus, scroll, `Fading`): see `desktop-shells`.
- Lock: `Command::Exit` with a restart asked serves, then restarts instead of exiting (`faraday/shells/demo/src/lib.rs:~206-210`, `faraday/shells/demo/src/main.rs:~288-306`, `~319`). `Demo::restart` (`faraday/shells/demo/src/lib.rs:317-335`):
  - It resets `dirs` (pictures) and clears `seen`, `toggles` and `served`.
  - It builds a new app from `old.mode_after_restart()`, so the mode the old app was in is kept (`:323`, `~425-444`).
  - It keeps `boxes`, `preview`, `attached`, `stop` and the listing thread.
  - `restarted` feeds the new app `boxes.restore()`, a clone of the in-memory Inbox, Outbox and kept list. It rescans, clears `listed` and `cameras`, and sends them again.
- The secrets of the old app go with the old app. The kept boxes stay through a lock. The Demo is a test area: what it holds is test data.
- A lock keeps, from the old app, only its mode (`Mode::Demo` brings the demo samples again); the host keeps the boxes, the attached sticks, the preview size, the clipboard and the listings; the input counts are not carried (`faraday/shells/demo/src/lib.rs:317-335`, `:425-444`).

## Reach

- Practice's refusals are checked in `act_inner` (`faraday/faraday-core/src/action.rs:1411`). They reach:
  - Seed XOR; new key as SLIP-39 shares; the backup flow (Make, Restore).
  - BIP-85; Vanity; Lightning; Silent payments; Convert; the vault from those screens.
- The chip fills fields of entry, keygen, inbox, seed read, seed check, create, vaults, backup check, lightning and core wallet.
- The Demo opens `transfer` (`--transfer`, file drop) and the samples (`faraday_samples`).
- The Starter opens the Demo inside its window (see `starter-app`). The Starter shell's `restart_demo` builds the new Demo from `demo.restart(&old.app)`.
- The host reaches into the app; see `desktop-shells` Reach.

## Rules and refusals

- `demo_refuses` is false whenever `makes_secrets()` is true (`faraday/faraday-core/src/demo.rs:27-30`). In Practice it refuses (`:31-79`):
  - `BXor` (Seed XOR parts).
  - `KWay` and `KMix` in SLIP-39 mode. `KNext` in SLIP-39 mode on the SOURCE or ENTER step.
  - `KLock`, unless the key was typed, words just made are a sample's own, or a sample of that length is still left to hand out (`:47-51`).
  - `Flow(Make)` for a flow that is not a wallet's (GPG or Secure Boot keys).
  - `Flow(Restore(Sheet | From | Restore))`.
  - `PShow`, `PVault`, `PLoad`, `POut` (every BIP-85 output).
  - `Vanity(Dial(Passphrase))`.
  - `LShow`, `LVault`, `LOut`.
  - `SScanVault`, `SScanOut`, `SDescriptorOut`.
  - `Convert(SetSource)`.
  - `Vault(CreateFrom | OpenFrom | ListFrom)` from Bip85, Lightning or Silent.
- A refused press shows one `practice_refused` toast and nothing else (`faraday/faraday-core/src/action.rs:1411-1415`).
- The chip is offered only in Practice (`demo_passphrase`, `faraday/faraday-core/src/demo.rs:204-206`; `pass_chip` `:255-258`). It is never offered on the KeePass export prompts (`Kdbx`, `KdbxCheck`), because `faraday` is under their minimum (`prompt_chip`, `:306-313`).
- In Practice `faraday` is not measured for strength (`pass_is_public`, `:221-223`). A Create a vault passphrase that is `faraday` in place of rolled words is said to be so (`dice_made_public`, `:211-217`).
- A public sample is refused on Mainnet, by every way a key comes in. Testnet, Signet and Regtest take it. The same refusal reaches a sample's BIP-85 child, the Lightning sample seeds, and a sample's words typed as Core Lightning words (history fixes; `faraday/faraday-keys/src/session.rs`, `faraday/faraday-keys/src/vault/reveals.rs`).
- A mode change asks first when a file waits in From the stick or For the stick (history fix, `faraday/faraday-core/src/idle.rs`).
- Demo command line (`faraday/shells/demo/src/main.rs`):
  - `--sticks` and `--downloads` need a directory, else exit 2.
  - `--text` must be a known `TextSize` (`559-565`).
  - `--panel`: a diagonal of 1.0 to 13.0, with an `in` suffix allowed (`566-572`).
  - `--aspect W:H`: both above 0 (`573-582`).
  - `--ppi`: 50 to 1000 (`583-586`).
  - `--size WxH`: numbers only, no range check; default 1280 × 800.
  - An unknown argument is refused with exit 2.
  - `--panel` with no `--ppi` and no EDID value is refused with exit 2. The panel aspect defaults to 3:4 below 4 in and 9:16 at 4 in and over (`604-614`).
  - `--version` is answered first.
  - It exits if the System32 restriction fails, if there is no display, or if `Demo::open` fails.
- Sticks (`faraday/shells/demo/src/lib.rs`):
  - The boot stick folder is made only once, guarded by the marker (`455-466`).
  - The app never moves, renames or removes files in stick folders.
  - A stick folder renamed or removed counts as pulled (`~355-356`).
  - Stick numbers are 1 to 255. A stick index that does not fit `u8` gets no pill (`faraday/faraday-host/src/screen.rs:~173`).
- Lists are sent only on change: the Downloads listing, and the sticks unless forced.
- Window: panel mode holds the app bare. Otherwise the window is at least 640 × 400 (`faraday/shells/demo/src/main.rs:~430`). The preview does not move the window. A change of scale keeps the panel's pixel size (`~481-489`).
- A dropped file that cannot be stat-ed is sent with size 0 (`faraday/shells/demo/src/lib.rs:~306`).
- Strip (`faraday/faraday-host/src/screen.rs`):
  - No pills unless `app.shell_sticks` (`~166-171`).
  - The boot stick waits only when asked and inserted (`~124`, `~185`).
  - The codex32 aid shows only on the codex32 key step when `sample_offered()` (`~207-213`). An aid already listed is skipped (`~223`).
  - Roll is disabled without room (`~281-294`). "Rolls per click" shows "Number" when typed and empty (`~298-307`).
- Strip shedding order (`fit`, `faraday/faraday-host/src/screen.rs:~405-495`):
  - The sticks fold into "Sticks · N".
  - Size drops its name.
  - The aids move into More, and Samples joins More.
  - `keep_aids` keeps the primary press first (`~501-529`).
  - More shows only aids that are not Off (`~664-665`, `~1048-1053`).
- Samples sheet (`faraday/faraday-host/src/screen.rs`):
  - It warns to pull the medium (`EN.pull_medium_to_reason`) when the self-test passed and no stick is in (`~1287-1292`).
  - Load can be pressed only when a sample is ticked, the self-test passed and no job runs; else it is Off (`1480-1484`). A running job shows progress.
  - A loaded sample shows ticked and dimmed, and takes no press (`~1524-1534`).
  - Tick lists stop after 255 rows (`1444-1447`).
  - "Unselect all" while any is ticked, else "Select all" (`~1375-1379`).
  - A wallet label that is `None` shows empty (`~1397-1398`).
- The Use a sample chip on the empty Add a key word grid sits in cell 1 after its number, and keeps every numbered cell. The Demo passphrase chip draws no caret until the person presses in or types (history fixes).
- The vault prompt's `faraday` chip is offered only where `prompt_chip()` holds, which is not for the KeePass export or its check (`Purpose::Kdbx`, `KdbxCheck`; `faraday` is under their minimum); the chip's press (`demo_pass_chip`, demo.rs:318) tests only that the session is Practice (demo.rs:306-313, 284-289).

## Domain calls

- `faraday_samples::SAMPLES`.
- `faraday_core::wallet::Session::{on, set_demo}`.
- `faraday_core::samples::{PASSPHRASE, Pick, Roller, label, Row}`.
- `faraday_core::fresh::roller::screen::{die_alone, per_choices, ROW_H}` (`faraday/faraday-host/src/screen.rs:1221`).
- `faraday_core::keygen::kstep::CODEX32`, `faraday_core::codex32::C32::Sample`, `faraday_core::vault::VaultAction`.
- `faraday_core::{BOOT_FOLDER, Faraday, StickInfo, StorageEvent, Mode, Action, Screen, ui::TextSize, names::MENU_DESKTOP_ID}` (`faraday/shells/demo/src/lib.rs:30`).
- `faraday_storage`, `faraday_window`, `faraday_host::AppHost`.

## Fixtures

These are compiled into `faraday_samples::SAMPLES`, which the Demo sets as its sample source:
- `faraday/faraday-samples/Cargo.toml`
- `faraday/faraday-samples/data/bump-original.psbt`
- `faraday/faraday-samples/data/bump-replacement.psbt`
- `faraday/faraday-samples/data/core-wallet-encrypted.dat`
- `faraday/faraday-samples/data/core-wallet-plain.dat`
- `faraday/faraday-samples/data/hsm-secret`
- `faraday/faraday-samples/data/hsm-secret-encrypted`
- `faraday/faraday-samples/data/hsm-secret-words`
- `faraday/faraday-samples/data/inheritance.psbt`
- `faraday/faraday-samples/data/krux-code.kef`
- `faraday/faraday-samples/data/legacy.psbt`
- `faraday/faraday-samples/data/musig.psbt`
- `faraday/faraday-samples/data/nested.psbt`
- `faraday/faraday-samples/data/sample.kdbx`
- `faraday/faraday-samples/data/savings.psbt`
- `faraday/faraday-samples/data/signer-elsewhere-returned.psbt`
- `faraday/faraday-samples/data/signer-elsewhere-sent.psbt`
- `faraday/faraday-samples/data/signer-returned-proprietary.psbt`
- `faraday/faraday-samples/data/signer-returned.psbt`
- `faraday/faraday-samples/data/signer-sent.psbt`
- `faraday/faraday-samples/data/spending.psbt`
- `faraday/faraday-samples/data/taproot-multisig.psbt`
- `faraday/faraday-samples/data/taproot.psbt`
- `faraday/faraday-samples/data/timestamp-header.hex`
- `faraday/faraday-samples/data/timestamp.ots`
- `faraday/faraday-samples/data/tree.psbt`
- `faraday/faraday-samples/data/vault-legacy.psbt`
- `faraday/faraday-samples/data/vault-nested.psbt`

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 5c196d803 | The desktop app moved, renamed or removed a tester's stick folders (BOOTSTICK, STICK-N, old names) at start. | The desktop never moves, renames or removes anything in its stick folders. |
| edf6122bc | Faraday Demo at rest drew 22 to 49 frames a second with nothing moving, and its fades came and went. | Idle draws nothing, only changed rows are copied, and fades continue only while running. |
| 1d2bae2ad | In the Demo's menus at phone width, a row's name ran into its trailing text. | A menu is as wide as its widest row needs, up to the window margins. |
| a05417db4 | The Demo offered a stick pill past the 255th, so a pill could select another stick's number. | Stick numbers are 1 to 255; no pill selects another stick. |
| 223c1c330 | The Demo passphrase chip drew a caret under it before any press or typing. | No caret under the chip until the person presses in or types. |
| ce8c45ae7 | The side panel of Spend, Back up and Create a vault, and a board's list, drew under the header pills, so the Load samples and mode pills stood over "Your choices". | Side panels start below the header's row of pills (`SIDE_PANEL_TOP`). |
| 58e461841 | In the compact app bar, FARADAY DEMO's name ran past the bar. | The name is on one line where it fits; else DEMO sits under FARADAY at 0.7 of its height. |
| ac6667107 | A mode change emptied files waiting in From the stick or For the stick without asking. | A mode change asks first when a file waits in either box. |
| df069121a | On Testnet and in Practice the mode pill cut page names beside it. | The pill is on the top bar's second line, and the page name keeps the first. Back is shortened to the place's name where it would not fit. |
| 27d1dd469 | The mode pill covered the page name's descenders. A Back label that could not show its first word was drawn as "T…". | The pill stands below the name's line. A Back label that cannot show its first word is left off. |
| 17fe78482 | The mode pill's press target was smaller than a touch target, and so was the open card header under Change rows on a compact layout. | The press target is a full touch target kept inside the bar. |
| 5513530f1 | Off mainnet, the mode pill sat in the wrong place on the top bar and a long name ran under it. | The pill is at the end of the top bar's first line, centred on the name, at every size. |
| 8b0ab819f | A Practice sample seed typed in on Mainnet was kept; only a sample from a vault was refused. | A public sample is refused on Mainnet by every way a key comes in. Testnet, Signet and Regtest take it. |
| c9c0e9199 | On Mainnet a sample's BIP-85 child and the Lightning sample seeds were not refused. | The same Mainnet refusal reaches derived and Lightning samples. |
| da9321358 | A sample seed's words typed as Core Lightning words were not refused on Mainnet. | The same Mainnet refusal for Core Lightning words; Testnet reads them. |
| 532e6f1ab | The empty Add a key word grid drew the Use a sample chip over the whole first row, hiding the numbered cells. | The grid keeps all its numbered cells; the chip sits in cell 1 after its number. |

## Doubts

- History row cafdf37c7 (a tap pressed twice when scrolling under a finger) was moved to `desktop-shells`, because it changes the window shell's input.
- 5c196d803 names `faraday/shells/desktop/src/main.rs`, the Demo's earlier path.
- The file changed by 223c1c330 is not confirmed.
- Only the `SIDE_PANEL_TOP` uses are confirmed as the fix in ce8c45ae7.
- c9c0e9199 and da9321358 were read from their commit messages only.
- `--size` has no range check. What a size of 0 does depends on `Core`.
- A doc line at `faraday/shells/demo/src/main.rs:~66` says the stick app starts on mainnet, while the Demo is on testnet.
- The host's public lookups for drivers and test scripts have no user-facing effect: `record_ink`, `ink`, `canvas_bytes`, `strip_texts`, `strip_actions_for`, `where_offered`, `offered_area`, `menu_area`, `app_area`, `sheet_area`, `list_scrolled`, `open_menu_at_rest`.
