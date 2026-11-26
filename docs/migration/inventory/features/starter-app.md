# The Starter app

Faraday Starter is the desktop program a person runs first, on the online
computer they will later start Faraday on. It runs on Linux and Windows. It
can:
- say whether this computer can start a Faraday stick;
- make the stick (`starter-make-stick`);
- help start from it (`starter-start`);
- install itself for the user;
- print blank backup templates;
- show the guide;
- open Faraday Demo inside its window;
- send and receive files as codes (`transfer`) through the desktop app it holds.

The app is in two parts:
- The pages and rules are in `faraday/faraday-starter/src/`. This crate does no I/O.
- The shell that carries out its commands is `faraday/shells/starter/src/`, on the window shell (`desktop-shells`).

The program starts at Home. `--uninstall` opens it on the Installed page with
Uninstall pressed.

## Screens and sheets

- Pages (`Page`, `faraday/faraday-starter/src/lib.rs`):
  - Home, Make, Tools, Guide.
  - Which, Disks, Mode, Confirm, Writing, Done (see `starter-make-stick`). Start (see `starter-start`).
  - Install, Installed, About, Appearance.
  - Templates, TemplateWords, TemplateCopy, TemplatePlaces, TemplateForm, TemplateLength, TemplateWorksheets, TemplateSize.
- Sheets (`Sheet`): Restart(Firmware | Stick), Uninstall, LetThrough, How, Guide, StartGuide.
- Frame (`faraday/faraday-starter/src/draw/nav.rs`; `faraday/faraday-starter/src/lib.rs:~2595`):
  - The destinations Home, Make, Tools and Guide sit in a rail, a bottom bar or a top bar, by size class (`Class`: Compact, Medium, Expanded).
  - A top app bar with the install or update action.
  - A menu with Uninstall (which opens `Page::Installed`) and Remove entry.
  - A Guide icon on task pages.
  - A window close button, on Linux only.
- Home (`faraday/faraday-starter/src/draw/home.rs` `home_panes`, `home_cards`):
  - Two panes at Expanded (This computer; the cards), one column otherwise.
  - Cards: Faraday Demo (Linux, Windows), Update, Remove boot entry, and Settings (Appearance, About).
  - The "This computer" card shows the checks, a summary, the checklist, and one button.
- Install / Installed: the places installed (App, Menu, Icon, UninstallEntry, Settings). `Sheet::Uninstall` confirms.
- Appearance (theme, mode, contrast) and About (text size) (`faraday/faraday-starter/src/appearance.rs`, `faraday/faraday-starter/src/draw/appearance.rs`).
- Guide destination: rows from `GUIDE_PLACES`. The Guide sheet. The How sheet (Cryptography) with blocks for `starter:write`, `starter:confirm`, `starter:writing`, `starter:done` and `starter:start`, each block once (`faraday/faraday-starter/src/how.rs:10-16`).
- Templates under Tools (`faraday/faraday-starter/src/template.rs`, `faraday/faraday-starter/src/sheets.rs`, `faraday/faraday-starter/src/sheets/dice.rs`):
  - **Words**: 12 or 24 words; copy words, SeedQR or both; places from 1 to `PLACES_MAX`.
  - **Codex32**: length 48, 74 or 127; copy as String, Codex32QR, or four grids (grids only at 48); worksheets (none at 127).
  - **Krux**: size 25, 29, 33 or 37.
  - The dice worksheet is two landscape pages.
- Faraday Demo inside the Starter: drawn under the Starter's top bar (`faraday/faraday-starter/src/demo.rs`).
- The held desktop app's Transfer screen (`Screen::Transfer`), shown in the Starter's frame.
- Drawing is `faraday/faraday-starter/src/draw.rs` (no `draw/mod.rs`) and its submodules `appearance`, `boot_file`, `clear_first`, `guide`, `home`, `how`, `images`, `name`, `nav`, `row`: `frame` draws one of the 23 pages in a scrolling column (`COL_W = 720`, `CONTENT_MAX` on the four destinations) under the top app bar, beside a rail or above a bottom bar by window class, then the menu and the open sheet (6 variants) over a backdrop; with a held desktop app only the held chrome is drawn (`faraday/faraday-starter/src/draw.rs:63-182`).

## Actions

- Navigation: `Home`, `MakePage`, `ToolsPage`, `GuideDestination`, `Menu`, `CloseMenu`, `Close`, `Back`, `Appearance`, `About`.
  - Back and Escape are Back.
  - Forward opens again the page Back left, but not from a confirmation and not with a menu or sheet up.
  - Back during a write is ignored.
- Install: `Install`, `InstallNow`, `Installed`, `Uninstall`, `UninstallNow`, `RemoveEntry`.
- Appearance: `Theme` (only on Appearance), `TextSize` (only on About).
- Guide: `Guide`, `GuidePage(n)`, `How`.
- Templates: `Templates`, `TemplateChoice`, `TemplateWords`, `TemplateForm`, `TemplateLength`, `TemplateCodexCopy`, `TemplateWorksheets`, `TemplateSize`, `TemplateCopy`, `PlacesFewer`, `PlacesMore`, `PrintTemplate`, `SaveTemplate`, `OpenTemplate`.
- `OpenDesktop` (Open Faraday Demo) and `Transfer`.
- Each action maps to a numeric `Action::Host` code (`code`, `faraday/faraday-starter/src/lib.rs:~455`; `from_code`, `~563`).
- Scrolling (`faraday/faraday-starter/src/scroll.rs`):
  - Drag, fling and wheel scroll the page or the open sheet.
  - A press that moves past `SLOP` presses nothing. A press on a coasting page only stops it.
  - A control acts on release only if the press is still on it.
  - Hold actions act only when held to `HOLD_MS`, never on release.
- Starter command line (`faraday/shells/starter/src/main.rs:~851-893`): `--image FILE`, `--downloads DIR`, `--size WxH`, `--timings`, `--uninstall`, `--version`, `--help`.
- A dropped file goes to Transfer when the Demo is open. Otherwise it goes to the make flow (`faraday/shells/starter/src/main.rs:~286-303`).

## Effects

All of these are `StarterCommand`s that the shell carries out (`faraday/faraday-starter/src/api.rs:~1025-1168`; `faraday/shells/starter/src/main.rs:~319-547`). The Starter's own
time is `now_ms` from host ticks.
- `Install` and `Uninstall`:
  - Linux (`faraday/shells/starter/src/linux/install.rs`):
    - A copy of the running file goes to `.local/bin` under the home folder, mode 0755. The icon and a `.desktop` file are mode 0644. Each is written beside its target as `.new`, then renamed.
    - It removes legacy names only if they are Faraday's own: Faraday Demo's installed copy and its cache (`faraday/<dotted numbers>` only), and the settings file.
  - Windows (`faraday/shells/starter/src/windows/install.rs`):
    - A per-user copy goes to `%LOCALAPPDATA%\Programs\Faraday Starter`.
    - A Start menu `.lnk` that holds LinkInfo only (`faraday/shells/starter/src/windows/shortcut.rs`).
    - An HKCU Uninstall key.
    - The running copy cannot be deleted, so it is renamed aside. Uninstalling the running copy schedules a deferred deletion with `cmd.exe`, retrying for 30 s with `ping` (`~465-491`). It runs from the `AFTER_EXIT` static in `exiting` (`faraday/shells/starter/src/windows/win.rs:~73`, `~262-276`).
- `RemoveBootEntry`, on Linux: `pkexec efibootmgr --delete-bootnum` for every `Faraday stick` firmware entry (`faraday/shells/starter/src/linux/boot.rs:300-315`). After Uninstall the entry is removed. If that is declined, the event is `BootEntryKept` (`faraday/shells/starter/src/main.rs:~352-367`).
- `KeepTextSize`: writes `settings.txt` with one `text=` line. It is written to `.settings.txt.new`, then renamed (`faraday/shells/starter/src/settings.rs`). It is read at start (`faraday/shells/starter/src/main.rs:~922-927`).
- `SaveTemplate`: saves the PDF to Downloads with `faraday_storage::save_download` (`faraday/shells/starter/src/main.rs:~392-400`).
- `PrintTemplate`: writes the PDF to a temp `faraday-print-` folder and opens it in the system viewer, never printing it (`faraday/shells/starter/src/pdf.rs:~232-276`).
- `OpenTemplate`: opens the saved PDF.
- `OpenDemo` (`faraday/shells/starter/src/main.rs:~227-258`):
  - Builds a `faraday_demo::Demo` and `desktop_app_in` with `Mode::Demo` and the `Program::StarterApp` paths.
  - A failure is `DesktopStarted(Err)`, naming the folder.
- `DemoClosed`: drops the Demo.
- `Close`: closes the window.
- Theme at start (`faraday/shells/starter/src/scheme.rs`):
  - Linux: the portal `org.freedesktop.appearance color-scheme` on the session bus, waiting 700 ms. Then `gsettings color-scheme`, then `gtk-theme`.
  - Windows: `AppsUseLightTheme`.
- The held app (`faraday/shells/starter/src/main.rs`):
  - Its Downloads listing is read again every 700 ms (`RESCAN`, `~75`, `~764-786`) and sent only on change (`relist`, `~620-639`).
  - Its `RequestEntropy` is answered with `faraday_window::entropy()`. `CameraOn` and `CameraOff` work as in the window shell (`~156-165`, `~198-199`).
- Child processes: `xdg-open`, `explorer.exe`, `cmd.exe`, `ping`, `gsettings`, `pkexec`. Also the installed copy with `--version`, which is given 5 s. `gsettings get org.gnome.desktop.interface color-scheme|gtk-theme` is waited for with no limit (`faraday/shells/starter/src/scheme.rs:77-87`); it runs at most twice, once at start, only when the portal said nothing.
- Threads: a Downloads watcher and one per slow command.
- No clipboard, no network. Diagnostics go to stderr only.
- The Starter's own state keeps no file (`faraday/faraday-starter/src/api.rs:8-9`); the settings file (`text=` line) is the shell's, written on `KeepTextSize` only while a Starter is installed; a write failure is printed to stderr and the size lasts for this run (`faraday/shells/starter/src/main.rs:369-376`).

## State and lock

- `Starter` (`faraday/faraday-starter/src/lib.rs:~934-940`):
  - Made once by `Starter::new`, it lives as long as the process.
  - It holds the whole desktop app, `held: Box<Faraday>`, with `online = true` and `held = true`.
  - Fields include `making`, `template`, `disks`, `concerns`, `let_through`, `bitlocker`, `include_key`, `other_confirmed`, `installed`, `dev`, and `Held` (the Demo host, its size, touch).
- A change of page (`go`, `~1991`) clears `sheet`, `menu`, `forward` and `offset`, and keeps the rest.
- A page whose content changes in place keeps the last-pressed item on screen. A new page opens at the top.
- The Starter has no lock, restart or reset path. Nothing in the crate wipes or resets it.
- `Template`, `Scroller` and `Held` hold no secret. Template sheets hold no secret.
- Theme: not kept. Text size: kept only while a Starter is installed (`settings.txt`). With nothing installed, `text_kept` goes back to Standard but `text` stays.
- `Shell` (`faraday/shells/starter/src/main.rs:~84-120`):
  - `starter`, `system: Arc<dyn Platform>`, `finder`, `image`, channels, `writing`, `close_held`, `key_sheets`, `host`, `listed`, `cameras`.
- `boxes`: `Boxes::Memory`, three empty lists at start (`faraday/shells/starter/src/main.rs:990-994`): the held app's Inbox files, Outbox files and `kept` (its own kept entries, such as `backup-gate`, `backup-flow`). The held app's `SaveBoxes` replaces them, `ForgetBoxes` clears them, `restore` hands them back (`faraday/faraday-storage/src/lib.rs:590-599, 642-651, 676-686`). Held in memory only; nothing reaches a file; gone at exit.
  - `dirs`, `demo`.
- The Demo inside the Starter:
  - A lock in the Demo calls `restart_demo`. A fresh Demo app is built from `demo.restart(&old.app)`, in the mode the old one chose (`faraday/shells/starter/src/main.rs:260-273`).
  - `DemoClosed` sets `Shell.demo = None`.
  - `exiting` sets `demo = None` and `core.view = None` (`~378-381`, `~744`).
  - Dropping the host is the Demo's only wipe in the Starter.
  - The Demo's data is test data.
- At exit, key sheets are wiped (see `starter-start`).
- A lock in Faraday Demo inside the Starter rebuilds the Demo app with the mode, boxes, attached sticks, preview size, clipboard and listings the Demo host keeps; the fresh app takes the Starter's theme and text size (`faraday/shells/starter/src/main.rs:262-273`, `faraday/faraday-starter/src/demo.rs:55-70`, `faraday/shells/demo/src/lib.rs:317-335`).

## Reach

- Opens:
  - `starter-make-stick` (Make; the card's Make).
  - `starter-start` (the card's Start, Firmware, Stick and Rescue buttons).
  - `practice-demo` (Open Faraday Demo).
  - `transfer` (`Action::Nav(Screen::Transfer)` in the held app, `show_held`, `faraday/faraday-starter/src/lib.rs:~2027`; leaving presses Cancel and Nav Home).
  - The guide (`faraday_guide`).
- Uses from `faraday_core`:
  - `Action`, `Faraday`, `Screen::Transfer`.
  - `wallet_backup::{sheet_blank, BlankCopy, structural_dark}`, `codex32_backup::{QR_SIDE, printed}`.
  - `motion::{Motion, Region, SLOP}`, `work::core_count`.
  - `strings::EN`, `names`, `BUILD`, `version_line`, `Program`, `Mode`, `parse_version_output`, `ui::{Theme, TextSize}`.
- Uses `faraday_shell_api` (`impl App for Starter`, `faraday/faraday-starter/src/lib.rs:~2835`), `faraday_host::Host`, `faraday_demo`, `faraday_storage`, `faraday_window`.
- Events reach the held app when it shows: keys, touch, wheel and ticks. The arm at `faraday/faraday-starter/src/lib.rs:2893-2899` passes every event the Starter does not handle itself (including the answers to what the held app asked: entropy, camera, files) to the held app. This arm is about events, not commands. Held-app commands: the shell answers `Draw`, `RequestEntropy`, `CameraOn`, `CameraOff` and `Exit` (not while a restart is requested); any other command is dropped with no answer (`faraday/shells/starter/src/main.rs:152-162`). The Starter does not turn on the held app's clipboard flag, so Transfer's paste emits no request.
- The snapshot example presses and reads the drawn state: `press`, `where_offered`, `drawn_texts`, `drawn_progress`, `drawn_targets`, `drawn_ink`. These are hooks in non-test code.

## Rules and refusals

**Home checks** (`Starter::checks`, `faraday/faraday-starter/src/decide.rs:~337`):
- Start mode: UEFI is good, legacy is bad.
- ARM adds a bad processor row.
- Secure Boot off is good. On or unread is a step.
- A usable camera is good; otherwise a step.
- A BitLocker row appears only on Windows with protection On.
- A Mac gets no checks.

**Summary** (`faraday/faraday-starter/src/decide.rs:~402`):
- Cannot: not UEFI, cannot start, or any restart hold.
- CanWithoutCamera: the camera is None or not usable.
- Otherwise Can.

**Checklist** (`Facts::checklist`, `faraday/faraday-starter/src/decide.rs:~616`):
- Items: MakeStick, Start, SecureBootOn.
- The checklist stops after Make a stick when the computer cannot start a stick (ARM, or a Mac with 32-bit firmware or Apple silicon). Turn Secure Boot back on (a row only for a non-Mac with Secure Boot off) comes after that point, so it is hidden too; on Windows that means an ARM computer (`faraday/faraday-starter/src/decide.rs:592-599, 631-647`).
- The Secure Boot item comes back only on a computer that is not a Mac, with Secure Boot off.
- `next_step` highlights one item.
- "Use a device" applies on Windows, with UEFI and Secure Boot read as off.

**Card button** (`Starter::card`, `faraday/faraday-starter/src/lib.rs:~1078`):
- Make: no stick, or the computer cannot start one, or not UEFI and not a Mac.
- Rescue: Windows with another encryption product open.
- Stick: Secure Boot off and the stick restart has not failed.
- Otherwise Firmware. A Mac gets StartMac.

**Install**:
- `Install` and `InstallNow` send `Install`, but not while installing (`faraday/faraday-starter/src/lib.rs:~2365`).
- `Uninstall` opens `Sheet::Uninstall` only on `Page::Installed`, with an installed copy.
- The menu's Uninstall row opens the Installed page; it does not uninstall (`faraday/faraday-starter/src/draw/nav.rs:84-88`, `faraday/faraday-starter/src/lib.rs:2368`).
- Update is offered when the running copy is newer by `faraday_paper::build::version_number`. A copy with no version counts as older (`~2579`).

**Install in the shell**:
- Linux refuses with `Reason::InTheWay` when the target exists and does not answer `--version` as the Starter within 5 s. Nothing is written before that check (`faraday/shells/starter/src/linux/install.rs:287-295`).
- Linux `Dirs::from_env` needs an absolute `HOME`, else `NoHome`.
- Linux uninstall keeps an App file that is not the Starter.
- Windows refuses `InTheWay` in the same way (`faraday/shells/starter/src/windows/install.rs:~286-290`). A file or registry failure is `Reason::File`.
- Windows removes legacy files only if they answer `--version` as their program.
- Windows removes cache folders only if all their dotted parts parse as u32. `.exe.old` and `.exe.new` are removed first.
- Windows does not schedule the deferred deletion if the folder name has `"`, `%`, `^`, CR or LF. Spawn errors are ignored.

**Remove boot entry**:
- Offered on Linux only, with the entry present and no Faraday stick attached (`entry_offered`, `faraday/faraday-starter/src/lib.rs:~1213`).
- With no `efibootmgr`, or no entry found, the result is Ok (`faraday/shells/starter/src/linux/boot.rs:300-315`).

**Appearance**:
- The theme choice is ignored off Appearance, and the text size off About (`faraday/faraday-starter/src/appearance.rs:40-75`). Both redraw the held app and the Demo at once.
- `keep_text` sends `KeepTextSize` only while a Starter is installed and the held size differs (`faraday/faraday-starter/src/appearance.rs:96-102`).
- The settings file is read only while installed. An unknown size gives None. Uninstall removes the file.

**Theme at start** (`faraday/shells/starter/src/main.rs:~919-920`):
- Only `Some(true)` gives Dark (`faraday/shells/starter/src/scheme.rs:~23-28`).
- The portal answers 1 for dark and 2 for light.
- `gsettings color-scheme` answers `prefer-dark` or `prefer-light`. A `gtk-theme` name ending `-dark` or `:dark` is dark.
- On Windows, `AppsUseLightTheme` 0 is dark.
- The colour stays Cobalt.

**Guide**:
- `guide_offered` (`faraday/faraday-starter/src/guide.rs:58-97`): not while the held app shows, not in the Demo, and only on the Which, Disks, Mode, Confirm, Writing, Done and Start pages.
- `guide_open` does nothing while any sheet is open.
- `guide_press` closes the Guide sheet, else opens it if offered.
- `how_press` toggles `Sheet::How` only when no sheet, or How, is up (`faraday/faraday-starter/src/how.rs:35-40`).
- A How press while Faraday Demo is open is consumed there (`faraday/faraday-starter/src/demo.rs:139-147`); while the held app shows it only ends the held app's view (`faraday/faraday-starter/src/lib.rs:2152-2154, 2672-2691`). The row is drawn only on the Guide page (`faraday/faraday-starter/src/draw/guide.rs:14-26`).

**Sheets and menu** (`faraday/faraday-starter/src/lib.rs:~2153-2175`):
- While a sheet is up, only these are taken: Cancel, RestartNow, UninstallNow, LetThrough, How, Guide, StartGuide(BootEntry), and Back on the How, Guide and StartGuide sheets.
- The menu does not open under a sheet.
- No control is pressed while the menu is open.

**Templates** (`faraday/faraday-starter/src/template.rs:192-223`):
- `set_chars` refuses lengths other than 48, 74 and 127.
- It resets the copy to String when the length is not 48, and turns worksheets off at 127.
- Save is refused while `saving`. Open is offered only after a saved path.
- The PDF's cost takes `&Compute`.
- The Places count stops at the rows the PDF's places page prints (13).

**Template sheet files** (`faraday/faraday-starter/src/sheets.rs:~288-367`):
- A kind other than codex32 or krux: `pdf_not_a_faraday_sheet`.
- Characters other than 48, 74 or 127: `pdf_c32_length`.
- A Krux side other than 25, 29, 33 or 37: `pdf_krux_size`.
- Defaults: 48 characters, side 33, 1 place.
- The checksum worksheet is only for 48 or 74. The translation and derivation pages only for 48.
- Krux QR finder, timing and alignment squares are drawn as a code has them.

**Demo inside the Starter** (`faraday/faraday-starter/src/demo.rs`):
- Offered only off macOS (`offers_desktop`).
- `open_demo_asked` sends `OpenDemo` unless the Demo is already open.
- While it is open, every Starter bar action other than Back is dropped. Back closes it (`DemoClosed`) and returns Home.
- Presses, moves and releases on its frame go to the Demo. Ticks go to both. Drops on the Starter side are discarded.
- The Demo size is sent only when it changes.
- `answer_demo` runs at most four rounds of Demo commands a tick (`faraday/shells/starter/src/main.rs:~187`).
- A file dropped while the Demo is open goes to its Transfer. The Demo branch passes the path as given. The Starter branch uses `std::path::absolute` (`faraday/shells/starter/src/main.rs:~286-303`).
- The lens camera is chosen only when the Demo is not shown (`~163-165`).

**Window and command line** (`faraday/shells/starter/src/main.rs`):
- The window is at least 640 × 400 (`~675`).
- `close_requested` is held during a write (`~699-710`; see `starter-make-stick`).
- It refuses to run if library loading cannot be limited to System32 (`~822-825`).
- An unknown argument exits 2 (`886-892`). `--size` needs `WxH`.
- `--version` and `--help` print and exit.
- The print folder is `faraday-print` under `$HOME`, or under `USERPROFILE` (`~962-965`).
- The window close button is drawn on Linux only.

**Tools in the crate's examples**:
- `faraday/faraday-starter/examples/icon.rs`: `icon OUT.png` draws the 256×256 app mark, the shield alone on transparency. Wrong usage exits 2.
- `faraday/faraday-starter/examples/snap.rs`: `[--tour] WxH[@DPI] OUT_DIR [THEME]`, with env `FARADAY_SNAP_TEXT`. It writes PNGs, `cut.txt` and sample PDFs. A bad size, theme or text size exits 2. A `@DPI` that does not parse becomes 160 without a word (`faraday/faraday-starter/examples/snap.rs:1170-1172`).
- Install ends by removing the cache's release folders (`X.Y.Z`); a folder that cannot be removed makes the whole install (Windows `faraday/shells/starter/src/windows/install.rs:306`, Linux `faraday/shells/starter/src/linux/install.rs:305`) and the whole uninstall return an error. On Windows uninstall this comes before the folder removals and before the exit-time removal command is returned (`faraday/shells/starter/src/windows/install.rs:441`); on Linux it is the last step (`faraday/shells/starter/src/linux/install.rs:377`).

## Domain calls

- `faraday_paper::build::{version, version_number, describe}`, `faraday_paper::pdf::{sheet, raw_document, PLACES_MAX}`.
- `faraday_bip::codex32::{checksum_table, translation_symbols, pair_symbol, gf_mul, charval, CHARSET, INDEX_ORDER, SYMBOL_GLYPHS}`; `faraday_bip::keys::Network::Mainnet` (blank words sheet only).
- `faraday_guide::{MAKERS, GUIDES, maker, guide_id, guide_address, page_of, how::screen_blocks, guide_page, step}`.
- `faraday_codec::qr::encode`.
- `faraday_crypto::work::Compute` (cost token; the snapshot example mints a no-op one).
- `faraday_storage::save_download`.
- `faraday_ui`, `faraday_draw`, `faraday_strings`.
- No calls into faraday-keys, -vault, -psbt, -sb, -kef, -pgp, -entropy, -sqlite or -core-wallet from the Starter crate.

## Fixtures

None. The crate compiles in data that is not in the fixture list:
- `faraday/faraday-starter/src/macs.tsv` (173 rows).
- The 19 files in `docs/guide/start/`.
- `faraday/shells/starter/assets/icon.png`.

The codex32 tables cite `tools/vectors/codex32-booklet/tables.json` in comments only.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| bbf1971b3 | The Starter's icon drew the shield on a dark rounded square. | The icon is the shield alone on transparency. |
| f3c81f060 | The Places list kept counting past the 13 rows the PDF's places page holds, and More never stopped. | The on-screen count matches what the PDF prints; More stops at the printed rows. |
| 130803464 | The Krux code sheets printed a QR code's fixed squares as solid black blocks. | Finder, timing and alignment squares are drawn as a code has them. |
| ce10d1c41 | The mouse forward button did nothing after Back; Back from Mode cleared the chosen disk; Back from Confirm could be reopened by Forward. | Forward reopens the page Back left, but not from a confirmation; Back from Mode keeps the chosen disk. |
| 2c934614e | A page whose content changed in place lost the place of the person's last press. | The page keeps the last-pressed item on screen within its new length; a new page opens at the top. |

## Doubts

- Twenty history rows given to this feature were moved to their own features in this group:
  - Eleven to `starter-make-stick`: disks in use, 512-byte sectors, GPT checks, keep data, version read, write progress, a write lost at uninstall, the image layout, and `stick-boot.sh`.
  - Seven to `starter-start`: the key sheet's folder and contents, restart from the stick, and the BitLocker fields.
  - Two to `desktop-shells`: System32 library loading.
- bbf1971b3 also changed a launcher icon of an Android app since dropped.
- Looks dead or odd:
  - `StartGuide::ALL` lacks Maker.
  - `Template::pdf` has identical Codex32 and Krux branches.
  - `from_code` maps an unknown argument 42 to Places.
  - Several doc comments do not describe their function (leftover "Costs ..." lines in `faraday/faraday-starter/src/notes.rs`, `faraday/faraday-starter/src/sheets.rs`, `faraday/faraday-starter/src/scroll.rs`, `faraday/faraday-starter/src/template.rs` and `faraday/faraday-starter/src/draw/nav.rs`).
- `Step::Replace` is never built in `plan` (`faraday/shells/starter/src/windows/install.rs:~146`).
