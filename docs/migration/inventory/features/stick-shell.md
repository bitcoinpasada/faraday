# The stick shell

The stick shell is the program that runs Faraday on the device (the PC stick
and the Pi, together Faraday Offline). It builds the app, turns hardware
(framebuffer, input devices, camera, sticks) into events for `faraday-core`,
answers the app's commands, draws whatever `App::parts()` returns onto the
panel, and ends the process at a lock, a restart or a power-off. It has no
screens of its own: no `Screen::*` or `Sheet::*` is named or drawn in it. Code:
`faraday/shells/stick/src/`, with `faraday/shells/stick/src/main.rs` as the loop, boot and lock cycle.
Input devices, pointer and touch, the camera and the dev reports of the same
shell are in `devices-inputs`, `pointer-touch`, `scanner-qr` and
`dev-diagnostics`.

## Screens and sheets

- None. The app's frame is drawn through `Shell::draw` (`faraday/shells/stick/src/main.rs:~1251`) and `flush`, and blanked by `blank` (`faraday/shells/stick/src/main.rs:~1227`).
- A dev notice (`faraday/shells/stick/src/notice.rs`): up to 2 lines of a 5x7 lowercase and digit face (`. - _ : /`; anything else, uppercase included, draws blank), scaled one step per 240 short-side pixels, over the top of the frame, shown `NOTICE_FOR` 3 s after a dev file is written (`written`, `faraday/shells/stick/src/main.rs:~1204`); extra lines are dropped.

## Actions

- None of its own. It answers the app's `Command`s (`Shell::send`, `faraday/shells/stick/src/main.rs:~1630`, feeding `Faraday::event` and `poll_command`; each returned command is handled in order and requests the shell answers are queued at the back). The table (`faraday/shells/stick/src/main.rs:1651-1712`):
  - `Draw` marks the frame dirty; `Vibrate` (no motor), `StoreSettings` and `PreviewSize` are ignored; `Exit` sets `done`.
  - `RequestClipboard` answers `ClipboardUnavailable`; `WriteClipboard` answers `ClipboardNotWritten` (no clipboard).
  - `RequestFile` and `ReadFile` answer `FileUnavailable`; `WriteFile` answers `FileNotWritten` (files go through the disk process).
  - `CameraOn`: tells the app `StorageEvent::Cameras(list)` first, then `camera.on()`; on an error says "camera unavailable: reason" (verbose) and queues `Event::CameraUnavailable`; else creates `Scanner::new(ReadMode::Reduced)` if none.
  - `CameraOff`: `camera.off()`, `scanner = None` (the drop ends the worker and wipes its frame and payload, per its comment).
  - `RequestEntropy`: `entropy()`, then `Event::EntropySource(ENTROPY_SOURCE)` and `Event::Entropy`; on failure it says "no entropy from /dev/random; nothing is sealed until it comes" and sends nothing.
  - `SecureMac` answers `SecureUnavailable`, `StoreSecret` `SecretNotStored`, `LoadSecret` `SecretUnavailable`, `ForgetSecret` `SecretForgotten`: no secure element and no kept blob, so the core offers no stored key.
  - `Orientation(chosen)`: `turn_to`; if the size changed, queues `Event::Display(display_info())`.

## Effects

- Boot (`run`, `faraday/shells/stick/src/main.rs:~2159-2503`): builds the app (`fresh_app`, `~2927-2962`), restores the Inbox and Outbox, scans sticks, runs `main_loop`, then exits as `Exit::PowerOff` (status 0), `Exit::Lock` (core `RESTART_CODE`; the header doc says 75) or `Exit::Restart` (`RESTART_COMPUTER_CODE`) (`~2505-2526`).
- `fresh_app`: `set_memory_reader(faraday_storage::memory_available_mib)`, `set_wake(work::wake(..))`, `medium()`. SD card with HDMI: `DeviceCheck::hdmi_screen().with_edid(screen_block(..))`. Stick medium: `SecureBootAtStart::from_boot_params`, maker, family and model from the sysfs DMI files, `DeviceCheck::machine(cpuinfo, maker, product, chassis).with_edid(from_boot_params)`.
- Reads `/proc/cmdline`, the firmware boot params, DMI and chassis files (`~2161-2222`, `~2946-2957`), `/proc/meminfo` (`total_memory_mib`, `~3140`, for the Argon2id memory recommendation), `/sys/class/drm`, `/sys/bus/usb/devices`, `/sys/class/block`. Opens the framebuffer read and write (`~2202-2208`; created only with `--fb`).
- Entropy (`entropy`, `faraday/shells/stick/src/main.rs:785-798`): 32 bytes from `/dev/random` (`ENTROPY_SOURCE`, `~816`), a blocking `read_exact`. An open failure or short read gives no event; the core then has no session seed: no vault seal, no key, no signing; Home says so.
- Storage serving (`serve_storage`, `~1586-1614`): requests for the disk process wait for the disk lock; when the disk is `busy` this pass skips them rather than hold the frame; the rest are served at once. `Boxes` and `Dirs` are served in process.
- Disk-work thread "disk-work" (`faraday/shells/stick/src/diskwork.rs`): jobs `List { hide_waiting }`, a `Timings` write and a `BootReport` write. If the thread cannot start, the loop does the work itself. Writes go through `write_copies(stick, name, bytes, &[], true, false)`.
- Stick scan (`Shell::scan_sticks`, `faraday/shells/stick/src/main.rs:~2040-2090`) every `STICK_SCAN` 700 ms; with a disk process the look is asked of the worker and taken later (`finish_work`, `~1540-1574`). `sticks_found` (`~2118-2156`) tells the app `StorageEvent::Sticks`, `StickProblem { stick, read_only }`, `Clock { unix_secs }`, `Cameras`, then `Event::Tick`, only on a change unless forced.
- `tell_attached` (`~2096-2114`): new storage hides a not-handed-out stick for `HIDE_WAITING` 2 s and tells `StorageEvent::Attached`; a boot medium change is told once (`BootMedium::Two`, `NotFound`, `Known`). `attached` is called at `faraday/shells/stick/src/main.rs:2101`.
- Clean marker: `/run/faraday-clean/clean` (`put_marker`, `~464-476`; `marker_stuck`, `~478`; `publish_clean`, `~1986-2004`).
- Drawing (`faraday/shells/stick/src/fb.rs`, `faraday/shells/stick/src/present.rs`, `faraday/shells/stick/src/panel.rs`, `faraday/shells/stick/src/writer.rs`, `faraday/shells/stick/src/refresh.rs`, `faraday/shells/stick/src/pace.rs`, `faraday/shells/stick/src/orient.rs`):
  - `Presenter` keeps `last` (core RGBA) and `converted` (panel format) and writes only changed bands; a frame the core says moved is shifted in the kept copies (`move_kept`).
  - Conversion to RGB565, BGR888, RGB888, BGRA8888 or RGBA8888 with a turn of 0, 90, 180 or 270, by tile walk on `available_parallelism()` scoped threads.
  - `Panel` is shared by the loop, the input threads and the writer; 64 rows per lock hold (`CHUNK_ROWS`).
  - Writer thread "fb-write"; vblank waits through `faraday_v4l2::wait_vblank`; refresh measured once after the first write.
  - `faraday/shells/stick/src/pace.rs`: a frame step of 1 to 4 refreshes at 60 Hz (`MOST` 4, `KEEP` 1.15, `SHORTEN` 1.4, a window of 8, coarsen at 5 late, recover after 12, cost capped at `frame * MOST`); fades paced by the median of the last 5 step costs; no clock of its own.
- The CPU's minimum clock is raised while input or drawing runs and dropped a second after (`faraday/shells/stick/src/clock.rs`; only the schedutil governor; a policy whose write fails is dropped, `faraday/shells/stick/src/clock.rs:55, 111-114`). The Pi is left alone.
- Power-off and restart end the disk process and drop the page cache before reboot or power-off; a restart or power-off drops everything a lock would drop (without the seal) and removes every waiting file.

## State and lock

- `Shell.app: Option<Faraday>`: the whole app, with the core's own wiping on drop; dropped before exit.
- Lock, restart and exit (`faraday/shells/stick/src/main.rs:2446-2503`):
  - Normal path: `flush`, `settle`, `end_work`, `serve_storage` (serves what the app queued, such as the lock's box save; a vault in pieces is not waited for), `report(true)`, `write_boot_report(true)`, `camera.off()`. If restarting: `keep_mode(BOXES, app.mode_after_restart())` and `keep_input_counts(BOXES, app.input_carry())`. Then `app = None`. When exiting without restart: sleep `GOODBYE` 2 s (`~290`) and `boxes.forget()`. `blank()` always (no picture left for the firmware, a warm start, or a lock).
  - Panic path (`shell.panic`): `blank()`, `camera.off()`, `app = None`, return `PowerOff`. It skips `flush`, `settle`, `end_work`, `serve_storage`, `report`, `write_boot_report`, `boxes.forget` and the 2 s goodbye.
- Kept across a lock: the Inbox and Outbox (`/run/faraday` boxes), the `mode` file, the `input-counts` file, the start-window file, the `timings-carry` file (dev). Nothing else of the shell; everything in the app is lost with the process.
- `/run` is a tmpfs mounted by init (`faraday/image/common/rootfs-overlay/etc/init.d/rcS:24-25`), so the boxes go at power-off, also on the panic path that skips `boxes.forget()`. In the not-kept rebuild, the shell keeps its own state except what `start_again` resets (`faraday/shells/stick/src/main.rs:1752-1797`): `Panel`, `Presenter`, the disk handle and its queue stay.
- A failed lock save (`faraday/shells/stick/src/main.rs:1727-1797`, `start_again` `~1752-1797`): if `restart_requested` is no longer true after `serve_storage`, `done = false` and the app is rebuilt in the same process. The old app is dropped first; `camera.off()`; `scanner = None`; believed flags false; every hold back to `Pending`; the announce list rebuilt; `usb`, `sticks` and `clean` cleared; a new `fresh_app`; `Event::Display`; `RotationOffered` and `ExternalScreen` sent again; `app.start_unrestarted(carry_unkept_lock)`. Cursor and arrow state and the `Panel` and `Presenter` frame copies are not reset.
- Screen copies: `Panel.shown` and `band`, `Presenter.last` and `converted` hold the pixels of whatever was on screen, which can be seed words or keys. None is `Zeroizing`; `blank()` writes black rows, sets `stale`, clears `arrow` and `notice`, but does not clear `shown` (`faraday/shells/stick/src/panel.rs:476-485`). They are freed when the process ends; the failed-lock rebuild keeps the same `Panel` and `Presenter`.
- Disk work: `Work { request Sender, reply Receiver, thread, stop, in-flight flags }`; `Disk = Arc<Mutex<DiskProcess>>`. Jobs hold only stick id, name and diagnostic text.
- No secret in `Keyboard`, `Cursor`, `Pointer`, `Parser`, `Window`, `Watch`, `Work`, `Timings`, `Report`, `Sampler`, `Beat`, `Pace`.
- Constants: `WAKES` 64 (`~174`), `TICK` 50 ms (`~161`), `NOTICE_FOR` 3 s (`~168`), `GOODBYE` 2 s (`~290`), `HIDE_WAITING` 2 s (`~448`), `STICK_SCAN` 700 ms (`~450`), `DEFAULT_FILES` `/mnt/microsd` (`~316`).

## Reach

- Feeds every app feature through `Event` and `StorageEvent`; serves `storage` through `faraday_storage::{Boxes, Dirs, DiskProcess, Sticks, serve_with}`.
- Reads `devices-inputs` (`DeviceCheck`, the held-back inputs), `scanner-qr` (camera pass from `main_loop`), `pointer-touch` (cursor, touch), `dev-diagnostics` (boot report, timings, `costly::Watch` and `Timed`, used at `faraday/shells/stick/src/main.rs:1263, 1590, 1603`).
- The disk work uses `faraday_core::StickInfo`, `start_window::medium_port`, `bootreport`, `devnodes`, `radios`, `system`.
- Restart after a failed lock save uses `storage`'s `carry_unkept_lock` and `start_unrestarted`.

## Rules and refusals

Mode and carried files:
- `mode_token` (`faraday/shells/stick/src/main.rs:359-370`): only the four `STICK_MODES` (mainnet, testnet, signet, regtest) get a token; `Mode::Demo` returns None. `keep_mode` (`~375-384`): no token removes the file (the next process is Mainnet); write errors are ignored. `mode_at_start` (`~392-401`): an exact byte match to a token, anything else Mainnet; the file is removed whatever it said, so only a lock carries a mode (`faraday/shells/stick/src/main.rs:359-401`).
- `input_counts_at_start` (`faraday/shells/stick/src/main.rs:343-355`): the file is always removed after reading; a missing or unreadable file gives `[0; 3]`; a file that is not three numbers gives `[usize::MAX; 3]` ("door shut"). `keep_input_counts` (`~332`) prints a write error to stderr always.
- Clean marker: removing a missing marker counts as success; `publish_clean` does not write it when the app is not clean or is restarting, removes it when not clean, and says once when it cannot be removed.

Arguments and screen:
- Exit statuses `~2522-2523`; a usage error exits 2 (`~3113-3121`); a run error `ExitCode::FAILURE` (`~3131`); messages only with `--verbose`. A framebuffer open failure says "cannot open {path}: {error}" (`~2208`).
- `parse_args` (`~558-630`): each value flag errors when its value is missing; `--size WxH` parse only; `--depth` 16, 24 or 32; `--pointer-speed` finite and > 0; `--touch-grid WxH` both nonzero; `--native` portrait or landscape; `--mount` 90 or 270 only; an unknown argument is an error; `--fb` sets off-device; `--files` sets files given (`faraday/shells/stick/src/main.rs:576-626`).
- `probe` (`~640-649`): off-device or no framebuffer directory gives None; a framebuffer that cannot be described is an error, never guessed. `geometry` (`~658-683`): zero width or height is "the panel has no pixels"; the probed stride is used only when size and depth match.
- `Geometry::from_sysfs` (`faraday/shells/stick/src/fb.rs:~135-185`) refuses, naming the framebuffer and field: no `virtual_size` or fewer than two numbers, no visible mode, no `bits_per_pixel`, a depth other than 16, 24 or 32, a width or height not `u16` or zero. 24-bit byte order from the `red` and `blue` offsets; a 32-bit fbdev named `vc4drmfb` is RGBA8888; a stride smaller than the packed row is ignored; `visible_mode` needs a width equal to the virtual width, tallest wins. Layout uses the visible size, not the memory size.
- `dpi` (`faraday/shells/stick/src/main.rs:~723-743`): `--dpi` wins; then the EDID size on a screen of 11 inches or less (`orient::edid_dpi`); then `faraday.dpi=` on the command line if > 0; else 160; doubled at 2800 px wide or more. `window_asked` (`~746`): `faraday.window=WxH` or None.
- `medium` (`~412-420`): SD card on arm or when the exchange is `/dev/mmcblk0*`, else Stick. `screen_block` (`~426-428`): an empty EDID is no screen, else the first 128 bytes.
- `display_info` (`~1148-1170`): insets 0, buttons 0, `camera_fixed` false, secure hardware `None`, boot state `Unknown` (no attestation on the board).
- Orientation (`faraday/shells/stick/src/orient.rs`): `Turn` is 0, 90, 180 or 270 only (`Shared::get` falls back to R0 on a bad value); portrait detected when the framebuffer height exceeds its width; Rotation is offered when portrait natively or the EDID diagonal is 6 to 11 inches (`SMALL_MIN` 1524, `SMALL_MAX` 2794 tenths of a millimetre). `turn_for` (`faraday/shells/stick/src/orient.rs:266-273`): R0 when not offered or the chosen orientation is native, else the mount turn. Start is landscape. `MACHINES` (`faraday/shells/stick/src/orient.rs:127`, 19 vendor and product rows with a mount turn) is matched exactly after trimming; an unlisted machine gets R90. `edid_dpi` gives a density only when the EDID pixel size equals the framebuffer mode, the diagonal is at most `SMALL_MAX` and the density is within 96 to 480. `drm_edid` takes the first non-empty EDID of the sorted connected `card*-*` connectors. `stated` (`--native`, `--mount`) is offered only with a mount, default R90. `Shell::turn_to` (`faraday/shells/stick/src/main.rs:~1174-1187`): no change for the same turn; the arrow is kept inside the turned frame.

Drawing:
- `Shell::draw` (`faraday/shells/stick/src/main.rs:~1251-1321`) refuses a frame whose size does not match the panel ("frame does not match the panel; not drawing") or that is short ("short frame; not drawing"); a frame counts toward `--frames` even if no pixel changed; `read_back` (`~1323`) only with timings, 250 ms apart.
- `Presenter`: `move_kept` is ignored when there is no kept frame, dy is 0, w is 0, the move is outside the frame, |dy| >= h, or on a quarter turn; a stale panel or a size or turn mismatch writes the whole frame; a zero-row frame is skipped; a failed write leaves the panel stale; changed bands outside the frame or empty are dropped.
- `Panel`: a poisoned lock is entered anyway; a failed write sets `stale`; the whole frame is taken on a size change or when stale; `window_in` writes black outside the window once; `readback` returns None when stale or on a size mismatch; `set_notice` is a no-op when unchanged.
- Writer thread started only with 2 or more cores, otherwise the caller writes on its own thread (`faraday/shells/stick/src/writer.rs:71-73`); one write in flight (a `debug_assert!` only); vblank wait only when a source exists and the beat is on and measured; a failed vblank wait turns waiting off for the run but the write still goes out; a dead thread raises its panic; `is_for` compares the panel by address only.
- Refresh (`faraday/shells/stick/src/refresh.rs:40-62`): 16 back-to-back waits are measured; the wait is turned off when the median is under 2 ms (`AT_ONCE`) after `TRIES` 3 sets, when the median is outside 8 to 34 ms, or when fewer than 8 of 16 gaps are within 25% of the median; 8 waits in a row off the grid by more than 25% (`BAD_WAITS`) turn it off; gaps over 8 refreshes are pauses; a vblank older than 2 s is no reference; the grid refresh is clamped to 8 to 34 ms; any failed vblank wait turns waiting off.

Sticks and disk work:
- `attached(sys_block, usb_root, boot)` (`faraday/shells/stick/src/attached.rs:22-72, 88-101`):
  - only names `mmcblk<n>[p<m>]` or `sd<letters><digits>` under `/sys/block` count; size 0 or unreadable is skipped; a path that cannot be made canonical is skipped (`faraday/shells/stick/src/attached.rs:90-101, 41`);
  - the boot disk and its partitions are left out only if the record's `diskseq` equals the current one; a missing `diskseq` leaves nothing out; a reused name is not left out (`faraday/shells/stick/src/attached.rs:29-35`);
  - a disk is kept if mmcblk or on a USB path; the doc says the `diskseq` rule covers the Pi's own card too (`faraday/shells/stick/src/attached.rs:~18-23`); the boot record is `Record::Found { partition, diskseq }` (`faraday/shells/stick/src/main.rs:~2097-2100`);
  - USB interfaces: skipped on the boot medium's port; storage only if class 0x08, the interface and port both authorised (a missing `authorized` file counts as authorised), and not `only_empty_disks` (`faraday/shells/stick/src/attached.rs:65-72`);
  - output sorted; sysfs reads only.
- A stick the kernel shows counts as attached even when the disk process does not list it: it blocks passphrase entry and key loads, closes passphrase sheets, and in a process that is not clean brings up the Lock sheet (history fixes 0849ae18d, 65049ffb0).
- Sticks not yet handed over by the grant are kept from the app while it is clean (`hide_waiting`); a process that is not clean is told at once to lock (history fix 9e202fcff). Sticks are hidden while `clean != Some(false)` and `hide_until` is in the future (`faraday/shells/stick/src/main.rs:~2030`). On a disk error the last sticks are kept (`~2010-2016`).
- The device check runs only when `app.devices.read` (`~2053`); a changed EDID is judged again (`screen_changed`, `~2065`).
- Disk work: a second job of a kind is refused while one is out. `for_pass` waits for the disk lock when the loop asked, otherwise takes it only if free; a poisoned lock is taken anyway. A job waits up to `IDLE_WAIT` 15 s (50 ms polls) for a vault in pieces to end, or until `stop`; after that the call is made anyway (`faraday/shells/stick/src/diskwork.rs:254-281`). `take` asserts if the thread died with a job out; `finish` asserts no panic.
- Files writes are refused on the device: `mounted()` is checked first in both writers (`faraday/shells/stick/src/files.rs:80-86`). A file read from a stick's partition goes into a presized wiping buffer and is carried as `Zeroizing` through the shell API's file event.
- Windows and macOS housekeeping files and dot-files are ignored by name; a Faraday write leaves them in place.
- An open secret sheet for an Inbox file on its way to another stick does not make the process unclean.

Kernel and image (from history fixes):
- No kernel output leaves on a serial port; the app runs in its own session with no input.
- Neither RDRAND nor the firmware seed is credited as entropy (both still mixed in); the Pi's hardware RNG is mixed with credit 0 (`rng_core.default_quality=0`); the kernel config check fails a config lacking the required command-line words.
- The PC kernel's default governor is schedutil; the Pi's is schedutil.
- On the Pi, the screen script runs before `mdev -s`, each screen's arguments come only from its own probe, a slow HDMI screen is asked again for two seconds before the panel, a connected HDMI monitor without modes is forced on, and DRM nodes are root-only 0600.

## Domain calls

- `faraday_core::{Faraday, StorageEvent, Mode, STICK_MODES, Medium, BootMedium, SecureBootAtStart, RESTART_CODE, RESTART_COMPUTER_CODE}`, `wallet::Session::on` (`faraday/shells/stick/src/main.rs:~2332`), `work::time_vaults` (`~2383`), `devices::{chassis_laptop, DeviceCheck}`.
- `faraday_paper::edid::{from_boot_params, size, Size}`.
- `faraday_storage::{Boxes, Dirs, DiskProcess, Sticks, serve_with, memory_available_mib}` (`faraday/faraday-storage/src/lib.rs`).
- `faraday_crypto::pin_failure_to_report` (`faraday/shells/stick/src/main.rs:3065`).
- `faraday_v4l2::wait_vblank` (`faraday/shells/v4l2/src/lib.rs`).

## Fixtures

- `faraday/shells/stick/tests/edid/LICENSE-CC-BY-4.0.txt`, `faraday/shells/stick/tests/edid/README.md` and the EDID screens under `faraday/shells/stick/tests/edid/`: `boe-10.99in-2160x1440.edid`, `boe-11.02in-2160x1440.edid`, `cpt-7in-1280x800.edid`, `dell-24in-1920x1080.edid`, `displaylink-7in-1024x600.edid`, `eve-10.1in-1280x800.edid`, `hannstar-7in-800x480.edid`, `mstar-11.6in-1920x1080.edid`, `samsung-10.79in-1920x1080.edid`, `samsung-tv-1920x1080.edid`, `sunplus-tv-1920x540.edid`, `valve-7in-800x1280-placeholder.edid`, `vizta-10.1in-1024x600-wrong-image-size.edid`, `vizta-10.1in-1024x600.edid`, `waveshare-10.1in-1920x1200.edid`, `waveshare-11.9in-2560x1600.edid`, `waveshare-8.9in-2560x1600.edid`, `waveshare-portrait-1080x1920.edid`. Used only by the test module `edid_screens` (`#[cfg(test)]`, `faraday/shells/stick/src/main.rs:103-104`), for the HDMI density table.
- Data in code: the `MACHINES` table (`faraday/shells/stick/src/orient.rs:~127-156`); hand-copied input-event constants (`faraday/shells/stick/src/evdev.rs:~61-128`).

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 90c45fa1d | A stick that took a pulled boot stick's device name was not listed as attached; an empty USB card-reader slot was listed as attached. | The boot disk is left out only while its kernel disk sequence number matches the one recorded at boot; a mass-storage interface counts only if it has no block device yet or one of size above 0. |
| 0aaa0f66d | Regression from 90c45fa1d: the Pi's own SD card, which recorded no disk sequence number, was listed as an attached stick. | With no recorded sequence number the boot partition's disk was left out by its name. |
| 74ec847dc | Ticking a file that might be a secret on a stick visit opened the secret sheet, which counted as a secret held; the process stopped being clean, the stick was taken back and Write failed. | An open secret sheet for an Inbox file on its way to another stick does not make the process unclean; copies between sticks show at once. |
| bab40ada2 | On the PC stick's 6.18 kernel, the Exar serial driver selected a forbidden option, so the image failed its kernel config check. | The config passes the forbidden list; the Exar driver is left out. |
| 9e202fcff | The disk process listed the boot medium before the grant handed it over; the app's first look found no files, so the boot import was never offered. | Sticks not yet handed over are kept from the app while it is clean; a process that is not clean is told at once to lock. |
| 754ef4646 | The stick's kernel silently credited RDRAND and the firmware's seed as entropy (the options the fragment named were removed in Linux 6.2). | Own randomness (F86): neither is credited; both still mixed in; a config lacking the required command-line words fails the check. |
| 3766e236a | Power off and Restart left plain seed files and secrets in the RAM directory, kept the disk process running and kept the page cache. | A restart or power-off drops everything a lock would drop (without the seal), removes every waiting file, ends the disk process and drops the cache; a lock keeps the Outbox secret. |
| 0849ae18d | With a slow or silent disk process, a disk without partitions or a boot-named partition, a passphrase could be typed or a key loaded while a stick was attached; Files and the Pull sheet did not say so. | Attachment comes from the kernel's view of disks and USB interfaces on each scan; an attached stick is said and blocks passphrase entry and key load. |
| a263c8523 | A file read from a stick's partition was read into a plain growable buffer. | Read into a presized wiping buffer, carried as `Zeroizing` through the shell API's file event. |
| 2c29154b1 | The kernel console could put kernel lines or panic text on a serial port; Ctrl-Z or Ctrl-C on serial pins could stop the app. | No kernel output leaves on serial; the app runs in its own session with no input. |
| 65049ffb0 | A stick the disk process does not list but the kernel shows took typing in the Krux and seed passphrase sheets; pulling a card from a USB card slot dropped a key load waiting on the pull. | Any shown stick is treated as listed (passphrase sheets closed, Lock sheet in an unclean process); a waiting load runs when the USB card adapter is out. |
| 879c32838 | The stick laid its screen out for the framebuffer's memory size, not its visible size. | Layout uses the visible size. |
| 8a2acd279 | On a Pi 3B+ the CPU was held at 600 MHz by the powersave governor, so frames took twice as long. | The Pi governor is schedutil and init raises the lowest clock while something moves. |
| 4454f093a | On the dev Pi image a screen found at boot got no screen arguments. | The dev image's app reads the screen the boot found, as the release image does. |
| d841b0eeb | On the Pi the touch and framebuffer nodes stayed root 0600 because the screen script ran after `mdev -s`; HDMI's panel arguments were wrong; one probe's touch name leaked into another screen's arguments. | The screen script runs before `mdev -s`; each screen's arguments come only from its own probe. |
| 26e1b0b2b | A Pi whose HDMI monitor reads connected but gives no modes got no picture; a missing `/dev/fb0` was reported as a refused creation. | The boot script forces the connector on and the next probe reads the EDID; a missing fb0 is reported as missing. |
| f3cba4bde | The Pi 2.8-inch touch overlay was refused every time, so the Pi started on HDMI. | The overlay is a source file configfs takes; it loads and the display overlay applies. |
| 319a0cbeb | A Pi HDMI screen with three or more EDID blocks lost all its modes when a later block's read failed. | A failed block read keeps the blocks before it; vc4 returns no modes rather than an error. |
| dba68b92f | A slow HDMI screen was probed once and missed, so the Pi went to its panel. | The connector is asked again for two seconds before the panel is tried. |
| fffa25f71 | On the Pi the panel's colours were wrong at 32 bits. | A kernel patch makes the framebuffer in the format the shell asks for. |
| a66b5b060 | A Pi with the 2.8-inch panel panicked about 13 s after power-on (vc4 output polling after unbind). | vc4 stops output polling on unbind. |
| bfc91b3f7 | The refresh check's sixteen waits all returned at once, so the wait was switched off for the whole run. | Measured after the first write; a set of instant waits is retried and only a third identical set turns the wait off. |
| d8c93f824 | The 2.8-inch panel's documented 32 MHz pixel clock gave 87 Hz instead of 60 Hz. | 22 MHz gives 59.93 Hz for the 532 x 690 mode. |
| a3d8b6f8d | Attempted fix of the Pi's first CRTC vsync wait by turning TXP off (reverted by 84243f703). | Vsync follows the panel's own pixelvalve. |
| 84243f703 | With TXP off the Pi showed no picture and wrote no boot report. | The first CRTC is the writeback block again; the overlay is removed. |
| 370805fab | Kernel 6.18: the Pi's 32-bit fbdev patch and the polling patch no longer applied. | The fbdev patch is redone (XBGR8888); the polling patch is dropped. |
| e1d2915ff | DRM nodes matched no `mdev.conf` line on the Pi and fell to the default rule. | Node permissions as in F153: mdev lines name the nodes as mdev reads them. |
| b037c01b6 | The disk loop took the disk lock every pass even with no stick request; a poisoned lock was refused. | Lock taken only when a request waits; a busy disk skips that pass's serving; a poisoned lock is still taken. |
| 5c63181a9 | The Pi credited bcm2835-rng with entropy. | The hardware RNG is mixed with credit 0, required by the config check. |
| a08c26305 | The refresh measure called a method a rename had removed, so the stick shell did not build. | The rate is learnt by `Beat::learn`; the shell builds. |
| e6703030d | On a slow laptop the default governor was userspace, leaving the CPU at its boot clock; drawing crates did not vectorise. | Default governor schedutil; drawing crates at opt-level 3. |
| 53bb0130f | On a PC stick a fade started from idle under schedutil came slowly and the frame gate turned fades off. | The minimum clock is raised while input or drawing runs and dropped a second after; the Pi is left alone. |

## Doubts

- Rows left out as not about this feature: 080b60125 (Upgrade's source stick), 14c854851, fdc31f3f8, 42e87e9cc and b9a1f5188 (the boot logo), cd3cb5056, b4723e7f5, fb961492f and 44f36ada8 (the Starter's stick writing and layout), 8dc8e4d11 (download status), 95bc790d4 (Spend's Lock and write), 86ec4713c and acf516aa5 (dev timings and the boot report's sampler; in `dev-diagnostics`), a6c59705c and 425eb6b48 (pointer; in `pointer-touch`), c7fd03a21, ec55eec25 and 87b48f203 (input trust; in `devices-inputs`), 0cabd576c, 0b18f538c, 4832948a3 and 535390cc8 (FAT and vault write-back in the disk process; see `storage`), d5fd12b64 and dad3b6c94 (stick delete sheet and read answers; see `files-inbox`).
- The 0aaa0f66d row says a boot record with no disk sequence number leaves the boot disk out by its name. The code at this commit does not: a record without `diskseq` leaves nothing out (`faraday/shells/stick/src/attached.rs:29-35`), and its doc says the Pi's own card is matched by `diskseq` like any other. A later change is presumed; whether every boot path records a `diskseq` (so the Pi's card is never counted as an attached stick, which would block key loads) was not traced.
- 319a0cbeb's code was not read; 8a2acd279 and d8c93f824 say themselves they were not measured on hardware.
- A failed `CameraOn` leaves the camera list already told; intent unclear.
- `faraday/shells/stick/src/diskwork.rs:~254`'s doc says a job after the wait is refused as busy; no such refusal is in that file.
- Doc and code differ: `faraday/shells/stick/src/fb.rs` `compare_convert_keep` with an empty frame and fw > 0 could hit `chunks(0)`; `faraday/shells/stick/src/present.rs` `rows_converted` `(b + 1 - a)` underflows only if b < a (neither re-checked); `Loop::wait` sets `unwoken` on a disconnected channel and never clears it (not re-checked).
- `edid_dpi` has an upper size bound but no lower one: a screen under 6 inches still gets a density while Rotation is not offered.
- Whether `Shell::say`'s output reaches anyone except with `--verbose` is unclear.
- Struct and enum lines in `faraday/shells/stick/src/main.rs:1-300` are approximate (within about 10 lines).
