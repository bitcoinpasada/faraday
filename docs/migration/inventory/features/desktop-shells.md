# The desktop window shells

The desktop shells carry the online app on a desktop computer (Linux and
Windows). There are four layers:
- **The window shell** (`faraday/shells/window`): the window, its drawing and
  pacing, mouse, touch and keyboard input, paste, file drop, the camera and
  randomness.
- **The host** (`faraday/faraday-host`): lays a `Faraday` app out at the
  window size or at a preview size, routes input between the app and the
  host's own strip, menus and sheets, and draws both.
- **The storage client** (`faraday/faraday-storage`): carries out the app's
  storage commands on the computer, such as the Downloads folder, opening a
  folder, and the disk process.
- **The camera crates** (`faraday/shells/camera`, `faraday/shells/v4l2`,
  `faraday/shells/mediafoundation`).

Faraday Demo (`faraday/shells/demo`) and the Starter (`faraday/shells/starter`)
are built on these layers. The Demo's own strip, menus, Samples sheet and
command line are in `practice-demo`. The Starter's window is in `starter-app`.
Nobody reaches this feature directly: it is what the desktop programs run on.

## Screens and sheets

- No screen of its own. The window shows the app's frame. In the Demo, the host's strip runs along the window's foot (see `practice-demo`).
- Window identity: app id, X11 class, a PNG icon, and Windows icon resource 1 (`faraday/shells/window/src/view.rs:~31-73`).
- On Wayland the window frame is drawn without title text, so no font is embedded (history fix 3dfef1d31).
- Window size and density go to the app as `Event::Display`, on open, on resize and on a change of scale (`faraday/shells/window/src/view.rs:~533-556`, `faraday/shells/window/src/run.rs:~171-179`, `~239-252`).

## Actions

- Mouse (`faraday/shells/window/src/input.rs:~78-277`):
  - Left press, move and release give `Event::Touch`. Moving gives `Event::Hover` and `Event::HoverEnd`.
  - Back and forward buttons give `Event::Back` and `Event::Forward`.
- Touch: only the first finger counts.
- Wheel: `Event::Wheel` in 48 px steps a notch.
- Trackpad: `Event::Scroll`, then `ScrollEnd`.
- Keys:
  - Backspace, Delete, Enter, Escape, the arrows, Home, End.
  - Tab and Shift+Tab (BackTab), Space, single printable characters.
  - Ctrl+A (SelectAll) and Ctrl+V (`Key::Paste`) (`faraday/shells/window/src/input.rs:~236-244`).
  - Shift press and release give `Event::Shift`.
  - Losing focus lifts a held press.
- Paste: the app's Paste sends `Command::RequestClipboard { kind }`. It is answered from the system clipboard as a file of that `FileKind`, with the text unchanged (`faraday/shells/window/src/clipboard.rs:~246-255`). The clipboard is never written.
- Drag a file over the window: `StorageEvent::Hovered` marks the drop target. Drop it: `StorageEvent::Dropped { path, size }` (`faraday/shells/demo/src/main.rs:~339-360`, `faraday/shells/demo/src/lib.rs:~305-312`).
- Press and drag on the empty room of the app's top strip: moves the window. A double-click maximises or restores it (`faraday/shells/window/src/drag.rs:~1-88`, `faraday/shells/window/src/run.rs:~255-270`).
- Close the window (`faraday/shells/window/src/run.rs:~215-224`).
- The host's routing of a press (`faraday/faraday-host/src/lib.rs`):
  - A press in the app's frame is `Event::Touch` at the frame's own pixel (`~606-629`).
  - A press outside an open menu closes the menu and presses nothing (`~601-605`).
  - A press on a host control is held as `Held::Host`. It acts on Up only if the pointer is still over that control (`~598-600`, `~632`).
- The host's routing of scrolling:
  - A drag or the wheel over an open menu or sheet scrolls its rows (`faraday/faraday-host/src/scroll.rs:~96-147`, `faraday/faraday-host/src/lib.rs:~1094-1105`).
  - On the strip it scrolls sideways, only where the strip is wider than the window (`faraday/faraday-host/src/scroll.rs:~153-193`, `faraday/faraday-host/src/lib.rs:~1108-1113`).
  - Over the app's frame it goes to the app (`faraday/faraday-host/src/lib.rs:~1114-1120`).
- The host's routing of keys:
  - Keys are dropped while a host menu or sheet is open (`faraday/faraday-host/src/lib.rs:1078-1079`). Otherwise they go to the app.
  - `Event::Back` closes an open menu, else goes to the app (`:1128`).
  - `Event::Focus` is taken by the host (`~1076`).
- `--no-camera` on the window shell's command line (`faraday/shells/window/src/camera.rs:~88-91`, `178-181`).

## Effects

- Drawing (`faraday/shells/window/src/view.rs:~77-191`, `~263-386`):
  - A window with a softbuffer surface. Each app frame is copied in and presented with damage, so only changed bands are redrawn.
  - Ticks every 50 ms when idle, and a frame apart while something moves. The refresh grid adapts (`faraday/shells/window/src/pace.rs:~15-36`, `~125-139`, `~181-189`).
- The window: `drag_window` and `set_maximized` (`faraday/shells/window/src/drag.rs:~56`, `~73`).
- Randomness: `getrandom::fill`, 32 bytes, given with the source's name (`faraday/shells/window/src/lib.rs:~43-50`).
- Camera (`faraday/shells/window/src/camera.rs:~273-383`):
  - On Linux, `faraday_v4l2` on its own thread. On Windows, Media Foundation `Capture::start`.
  - Events `StorageEvent::Cameras` and `QrSeen` (`faraday/shells/window/src/lens.rs:~46-51`, `~127-138`).
  - Camera messages are printed to stderr.
- v4l2 (`faraday/shells/v4l2/src/lib.rs`):
  - Opens `/dev/videoN` read-write for N in 0..32, even when probing.
  - Ioctls: QUERYCAP; TRY_FMT (640×480 per format when probing); S_FMT; REQBUFS (4 MMAP); QUERYBUF; mmap; QBUF and DQBUF; STREAMON and STREAMOFF.
  - Polls every 100 ms.
- Media Foundation:
  - The C side is `faraday/shells/mediafoundation/src/camera.c` (`faraday_mf_list`, `faraday_mf_capture`).
  - It is built and linked with `mfuuid` and `ole32` only on Windows (`faraday/shells/mediafoundation/build.rs:~8-21`).
  - The capture thread is named "camera".
- Clipboard (`faraday/shells/window/src/clipboard.rs:~60-218`):
  - A child process, `pbpaste`, `wl-paste` or `xclip`: stdin null, stdout piped, no shell, killed after 2 s.
  - On Windows, `OpenClipboard` and `GetClipboardData`.
- Library loading: `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` before anything else (`faraday/shells/window/src/system32.rs:~21-49`). It is called from the Demo (`faraday/shells/demo/src/main.rs:~635-640`) and the Starter.
- Build (`faraday/shells/window/build.rs`), for target Windows GNU only:
  - Makes delay-load import libraries, so winit's `SetWindowTheme`, `DwmEnableBlurBehindWindow` and `DwmSetWindowAttribute` load from System32 at first use.
  - No effect at run time.
- Waking on work: the winit user event (`wake_on_work`, `Faraday::set_wake`, `faraday/shells/window/src/lib.rs:62`).
- Host (`faraday/faraday-host/src/lib.rs`):
  - `Command::Draw` on most events. `Command::PreviewSize` (`~578`).
  - A canvas for the window and one for the strip (`~112`, `~116`, `~764-791`).
  - `Fading::begin` copies the pixels under a fading menu (`faraday/faraday-host/src/screen.rs:~1190-1206`).
  - `Host::peek` takes the app with `mem::take`, runs frames, and puts it back (`~407-438`).
- Storage client (`faraday/faraday-storage/src/lib.rs`; detail in `storage`):
  - File reads, writes (temp file then rename; `create_new` for downloads), deletes and zero-fills, folder reads.
  - Memory reads: `/proc/meminfo`, `/sys/fs/cgroup/memory.max` and `memory.current`, and Windows `GlobalMemoryStatusEx`.
  - Disk FIFOs, with a conversation token of 8 bytes from `/dev/urandom` (`~1426-1440`).
  - Timeouts: `ASK` 15 s, `ASK_WRITE` 120 s, `ASK_READ_PART` 120 s, `ASK_PICTURE` 60 s (`1413-1423`).
- Opening a folder or address (`faraday/faraday-storage/src/lib.rs:~867-887`, `~1307-1320`):
  - `OpenDownloads` opens Downloads.
  - `OpenInstitute` opens the fixed address `https://nakamotoinstitute.org` (`~1309`).
  - Each runs `xdg-open`, or `explorer.exe` from `SystemRoot`, with stdio null, and is waited for on a thread.
- `Hash`: a file's SHA-256 only (`faraday/faraday-storage/src/lib.rs:~938-955`, `~1221-1235`).
- None in this area: a clipboard write, a printer other than a file write, or the network.

## State and lock

- Window shell:
  - `Core`, `Shell<P>`, `View`, `Shown` (row hashes, not pixels), `Pacer`, `Pointer`, `TitleDrag`.
  - `Lens`: a `Camera` and an `Option<Scanner>`. The scanner exists only while the camera is on.
  - `Camera` / `Shared<B>`.
  - No secret state.
- Clipboard text is held in `Zeroizing<String>` and copied into `Text`, which wipes (`faraday/shells/window/src/clipboard.rs:~13`, `~30`, `~99-103`, `~246-254`).
- Camera frames: `faraday_camera::Frame` holds `luma` and `chroma` as `Zeroizing<Vec<u8>>` (`faraday/shells/camera/src/lib.rs:26-41`).
- v4l2 `Camera` (`faraday/shells/v4l2/src/lib.rs:~266-283`):
  - Fields: the file, format, geometry, `max_fps` (default 10), `buffers`, `streaming`, `stats`.
  - On drop: the stream stops, each mapped buffer is zero-filled and unmapped, then the file closes (`faraday/shells/v4l2/src/lib.rs:549-582`).
- Host `Host` (`faraday/faraday-host/src/lib.rs:~99-206`):
  - Owns `app: Faraday`.
  - `window`, `preview`, `canvas`, `strip`, `hits`, `menu`, `leaving`, `labels`, scroll and drag fields, `out: VecDeque<Command>`, `bare`.
  - The host has no lock handler.
- `Labels` (`faraday/faraday-host/src/labels.rs:1-23`) holds only control labels and menu rows. It holds no typed text.
- Storage client:
  - Nothing is wiped at lock. `Boxes` are kept across a lock (`faraday/faraday-storage/src/lib.rs:~637-640`).
  - `ForgetBoxes` (`:641-672`) is for an app that ends for good: `Memory` boxes are cleared; `Dir` boxes zero-fill each file in `inbox`, `outbox` and `kept`, then remove it.
- Leaving the scan sheet: `CameraOff`, then `Lens::off`. The camera stops and the scanner is dropped.
- A lock in the app ends in a restart. Each program replaces the app: the Demo with `Demo::restart`, the Starter with `restart_demo` (see `practice-demo`, `starter-app`).

## Reach

- Used by: Faraday Demo (`faraday/shells/demo`), the Starter shell (`faraday/shells/starter`), and the stick shell for the camera crates.
- `faraday-storage` uses `faraday_core::{Faraday, StickInfo, StickMake, StorageCommand, StorageEvent, Files, Kept, OnTaken, Unrenamed}` (`faraday/faraday-storage/src/lib.rs:~14`). It also uses `faraday_core::file_menu::sha256` (`:946`, `:1226`) and `faraday_qr::envelope::MAX_FILE` (`:823`).
- `faraday-host` calls the app:
  - `set_hosted`, `press`, `event`, `frame`, `poll_command`, `fades`, `take_changed`, `canvas`, `theme`, `reduce_motion`.
  - `stick_pills`, `shell_sticks`, `vaults_offer_pulses`.
  - `demo_roller_now`, `demo_hosted_now`, `keygen`, `sample_area`, `samples`, `samples_on`.
  - `selftest_passed`, `no_stick_in`, `medium`, `busy`, `busy_marks`, `mode`, `now_ms`.
- `faraday-host` also uses `faraday_core::work::core_count` (`faraday/faraday-host/src/lib.rs:775`, `:786`).
- The window shell's `faraday/shells/window/src/lens.rs` calls `Faraday::storage` and `take_camera` (`faraday/shells/window/src/lens.rs:71`).
- Opens: `transfer` (file drop), and the paste of every typed field.

## Rules and refusals

- Window drawing (`faraday/shells/window/src/view.rs:~121-132`, `~172-175`):
  - A zero-size window is not drawn.
  - A failed resize or `buffer_mut` skips the frame.
  - A failed present forgets the buffers, so the next frame is given whole.
- Window failures: a failure of the window, context or surface is printed and the loop exits (`faraday/shells/window/src/view.rs:~83-89`, `faraday/shells/window/src/run.rs:~196-199`). An icon PNG that is not 8-bit RGBA is left out (`faraday/shells/window/src/view.rs:~68-70`).
- Display: width is clamped to 320..65535, height to 240..65535, density to 80..640 dpi (`faraday/shells/window/src/view.rs:537-545`). A resize to zero is not told to the app (`faraday/shells/window/src/run.rs:~244-252`).
- Input (`faraday/shells/window/src/input.rs`):
  - Within 500 ms after a finger lifts, a mouse event echoed from the touch is ignored (`:43`, `~54-73`). While a finger is down, the left button and cursor moves are ignored too, so a tap presses once.
  - Only the first finger counts (`~85-101`).
  - Positions are clamped to 0..65535 (`~121-125`).
  - A scroll is sent only in whole pixels, and the wheel is clamped to i16 (`~189-203`).
  - Key releases are ignored (`~230-232`). Only a single non-control character is a key (`~254-277`).
- Drag (`faraday/shells/window/src/drag.rs`):
  - The window moves after more than 4 px (`:15`).
  - A double-click is two presses within 400 ms (`:18`).
  - Maximise only if the window is resizable (`~55`).
  - A press off the strip's empty room clears the drag (`~47-50`).
  - It works only when the app says `moves_window()`, and not for a finger echo (`faraday/shells/window/src/run.rs:~260-268`).
- Camera (`faraday/shells/window/src/camera.rs`):
  - `--no-camera` refuses every `CameraOn` with "--no-camera", and the app is told `CameraUnavailable` (`178-181`).
  - A running camera is not started twice (`~182-184`).
  - At most two frames are queued (`~231-234`).
  - On Linux, a refusal, a camera in use, or a camera not found becomes `CameraFailure` (`~249-283`).
  - Off Linux and Windows: "this shell captures on Linux and Windows only" (`~424-426`).
  - A camera that ended on its own is turned off, so the next `CameraOn` tries again (`~200-211`).
  - A damaged or short Linux buffer is skipped, not fatal (history fix 6a6c3caf6).
- Lens (`faraday/shells/window/src/lens.rs`):
  - Any camera answer that is not a frame drops the scanner before it is sent (`89-92`).
  - A frame that shrinks to zero size is not drawn (`~115-122`).
  - `CameraOn` sends the camera list each time. A scanner is made only if there is none (`45-59`).
- Clipboard (`faraday/shells/window/src/clipboard.rs:18-130`, `~246-254`):
  - Text over 1 MiB (`CAP`) is refused. Each tool gets 2 s (`LIMIT`) and is then killed.
  - Nothing is given for output that is over the cap, late, from a failed tool, from a non-zero exit, or not UTF-8.
  - Empty or no text is `ClipboardUnavailable { kind }`.
  - Tool order: macOS `pbpaste`; Wayland `wl-paste` then `xclip`; else `xclip` then `wl-paste`.
  - On Windows, data over twice the cap in bytes, or text over the cap, is refused (`~201-216`).
- Randomness: 32 bytes with the source's name, or nothing (`faraday/shells/window/src/lib.rs:~44-50`).
- Pacing (`faraday/shells/window/src/pace.rs:~16-36`, `faraday/shells/window/src/view.rs:~101-106`):
  - Tick 50 ms when idle; frame 16.667 ms.
  - The grid has at most 4 steps. It keeps a step at 1.25 room and shortens at 1.6 room.
  - A gap over 100 ms is a rest.
  - The window is paced only on Wayland.
  - Idle draws nothing (history fix edf6122bc).
- Library loading: a failure of `system32_only` on Windows is returned to the caller, which must stop (`faraday/shells/window/src/system32.rs:~21-49`). The Starter refuses to run then (`faraday/shells/starter/src/main.rs:~822-825`).
- Build (`faraday/shells/window/build.rs:~32-55`): only for Windows GNU. It panics if `llvm-dlltool` cannot start or fails. The machine type is fixed as `i386:x86-64` (`~47`).
- Host (`faraday/faraday-host/src/lib.rs`):
  - A `Display` with zero width or height is ignored, and so is the same window display again (`1043-1051`).
  - The preview size is refused under `bare` (`278-282`). The preview scale is clamped to `[PREVIEW_SCALE_MIN, 1.0]` (`~520`).
  - `bare` sets `set_hosted(false)`: no strip, strip height 0, no focus border (`~262-265`, `~335`, `~447-449`, `~926-928`).
  - Keys are ignored while a menu or sheet is open (`1078-1082`). `Event::Back` closes the menu (`1128`).
  - `ScrollEnd` goes to the app even with a menu open (`1122-1127`).
  - Under an open menu or sheet, scroll and wheel move only the rows under the pointer; nothing reaches the app (`~1094-1105`).
  - The wheel moves the strip only where `strip_max > 0` (`~1108-1113`).
  - The scroll step divides by `self.f()`, which the source comment says is never zero (`1088-1092`).
  - Hover over the app is ignored under a menu or over a host control (`~988-990`). A press reaches the app only where there is no host control and no menu (`~594-597`).
  - `peek` panics (`expect`) when the app has no display yet (`~409-410`).
- Storage client: see `storage`. Examples:
  - Folder sticks refuse a name (`faraday/faraday-storage/src/lib.rs:426-428`).
  - `ask` refuses with `storage_stick_busy` while a transfer is in flight, and with `storage_disk_silent` when the disk process is dead (`1867-1872`).
  - `save_download` never overwrites a file (`832-863`).
- Window layout, from history fixes (`faraday/faraday-core/src/lib.rs`):
  - A window whose wide layout would fall below three quarters of the screen's density takes the one-column layout at that density.
  - The layout is the same at any compositor scale above 1.5.

## Domain calls

- `faraday_files` (from the storage client):
  - `listing`, `vault_names`, `read_whole`, `write`, `write_vault`, `finish_vault_writes`, `read_qr_png`, `write_file`, `wipe`, `rename`, `checked`, `checked_for_write`, `foreign`.
  - `Place::{space, zero_free, make_empty}`.
  - `proto::{Request, Response, send, receive, PIECE, Problem}`.
  - Constants and types: `MAX_VAULT_READ`, `MAX_PIXELS`, `MAX_READ`, `OnTaken`, `WriteRefused`, `Unrenamed`, `Wiped`, `Picture`.
- `faraday_crypto::work::Compute`. `faraday_qr::envelope::MAX_FILE`. `faraday_text::{Text, text!}`.
- `faraday_scanner`: `Scanner::new(ReadMode::Full)`, `reduce_factor`, `reduce`, `reduce_chroma`, `offer`, `poll`, `poll_seen` (`faraday/shells/window/src/lens.rs:~57-137`).
- `faraday_camera`: `Frame`, `Failure`, `list`, `scale::{chroma_size, scale_factor, downscale, downscale_chroma}`.
- `faraday_v4l2`: `Camera::open(path, Size::HD)`, `find_or_failure`, `spawn`.
- `faraday_mediafoundation`: `Camera::start`, `Message`, `devices`.
- Outside crates: `getrandom`, `png`, `winit`, `softbuffer`.
- None of faraday-bip, -psbt, -codec, -entropy, -keys, -vault, -pgp, -sb, -kef, -ots, -plan, -paper, -sqlite, -core-wallet, -payload, -intake or -forms is called directly.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 3dfef1d31 | Every Linux desktop build carried the Cantarell font through the Wayland frame's title, and that font's notice put two email addresses into each Linux binary. | No personal information in a published build. The Wayland frame is drawn without title text, so no font is embedded. |
| 8593487be | The desktop app scaled its wide layout to a half-screen window on a scaled laptop screen, so text drew at about half size. | A window whose wide layout would fall below three quarters of the screen's density takes the one-column layout at that density. |
| 6a6c3caf6 | A window's layout followed the compositor's scale above 1.5. Fades were switched off by one slow frame. The Linux camera ended capture on a damaged or short buffer. | Layout is the same at any scale above 1.5. One slow frame does not turn fades off. A damaged buffer is skipped. (The camera restart part was later reverted by 24e47f389.) |
| cafdf37c7 | Scrolling under a finger in the Starter and Faraday Demo: a tap pressed twice, and the cursor moves the system makes from a touch acted as presses. | The window shell ignores the left button and cursor moves while a finger is down and for 500 ms after. A tap presses once. |
| 58dc296d7 | Windows programs could load libraries from outside System32. | Libraries load from System32 only, and the build checks it. |
| 1f4c7b61d | Windows programs did not call `SetDefaultDllDirectories` first. | The DLL search is set before any load, and the build checks the configuration. |

## Doubts

- Three history rows were moved here from sibling features of this group, because they change the window shell:
  - cafdf37c7, from `practice-demo`.
  - 58dc296d7 and 1f4c7b61d, from `starter-app`.
- The resources file named earlier as `faraday/shells/build/windows/resources.rs` does not exist. The file is `faraday/build/windows/resources.rs`.
- Library loading is limited to System32 on every platform. Whether the call does nothing off Windows was not read.
- File drop is X11 only, per a source comment (`faraday/shells/window/src/run.rs:~216-217`). Not tested.
- `faraday/shells/window/src/camera.rs:189-198`: choosing another camera while one runs restarts it. On failure it only prints; the app is not told and may believe the camera is on.
- The host passes `ScrollEnd` to the app with a menu open, and drops keys while a sheet is open, so the Samples sheet cannot be driven by keyboard. Whether this is intended is unclear.
- The v4l2 width cap is 1920 in code (`faraday/shells/v4l2/src/convert.rs:24`) and 1280 in a doc comment (`faraday/shells/v4l2/src/lib.rs:446-447`). Media Foundation's cap is 1280, and its comment says "same as the V4L2 shell's".
- The v4l2 crate doc says 640×480 and "no device chooser yet", while the code takes a wanted size. `search()` opens every `/dev/video*` read-write, which also touches nodes that are not cameras.
- Whether `faraday_draw::Canvas` wipes its pixels was not read. The host's canvases show whatever is on screen.
- Not read by the inventory:
  - `faraday/shells/mediafoundation/src/camera.c`.
  - `faraday/shells/v4l2/src/abi.rs:~421-452`.
  - The window shell's Windows build script.
  - Which action sends `ForgetBoxes` and `Print`.
  - Which feature sends `OpenInstitute`.
- Storage: the early `?` returns in `Boxes::save` drop `first_error` and stop later subfolders, against the doc at `faraday/faraday-storage/src/lib.rs:~577-583`. `MakeEmpty` reports `next: 2, total: 0` (`1137-1139`), which looks like a placeholder.
