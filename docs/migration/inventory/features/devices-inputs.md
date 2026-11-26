# Devices and input trust

Faraday checks what is connected before it shows a secret, and holds back any
input device it did not see at power-on until the person says it is theirs.
The device check (`DeviceCheck`) recognises screen-capturing or remote devices
(a KVM, a TV screen, a virtual machine, a capture device) and holds a press that
would show a secret under the Devices sheet. Held-back input (F153, F202): a
new keyboard, pointer or touch device is held; its sheet (`Sheet::NewInput`)
shows once it acts; a keyboard is believed by typing a 6-letter code, a touch
device or an alone pointer by pressing targets in order. The stick shell
decides which devices are trusted at start (the start window, fixed ports,
built-in buses), reads every device exclusively, and carries the physical
keyboard map and the panic chord (Super and S held, power-off). It is reached
whenever a device arrives, from the boot notice of USB devices present at
start, and before any press that shows a secret.

## Screens and sheets

- `Sheet::Devices`: the devices that are held (KVM, TV screen, capture device, virtual machine) and the waiting press; its flow also uses `Sheet::Lock`, `Sheet::Qr` and `Sheet::LockAsk` (`faraday/faraday-core/src/devices.rs`).
- `Sheet::NewInput`: a held-back keyboard, pointer or touch device; shown only after it acts. For a keyboard, the 6-letter code to type; for touch and alone pointers, the targets to press in order; Ignore.
- The boot notice: the list of USB devices present at start (`StorageEvent::BootUsb { names }`), shown once, closed by x or Escape after a frame drew it, timed out at 10 s (`BOOT_NOTICE_MS`).
- The reminders sheet before the first secret, which names cameras and capture cards; a TV reminder on laptops with a TV maker's HDMI screen; a keyboard mark in the top bar while a wireless receiver is attached (history fix b111ae501).

## Actions

- `device_continue` asks; `device_let_through` only after the sheet asks; `device_understood` (refused while any item is held); `device_check` (drops gone items, toast `devices_still`) (`faraday/faraday-core/src/devices.rs:753-760`).
- Lock from the device sheet drops the waiting press and any pending mode or write (`faraday/faraday-core/src/devices.rs:~649-677`).
- `device_released` resumes the boot import and runs the pressed action again (`faraday/faraday-core/src/devices.rs:~696-704`).
- On `Sheet::NewInput`: type the code (keyboard), press the targets (touch, pointer), Ignore (kept out until unplugged). A trusted keyboard's Enter lets a pointer in after the Enter line was drawn and 1 s of quiet (`ENTER_QUIET_MS`).
- Close the boot notice (x or Escape).
- Panic chord: hold Super (either) and S for 2 s; the device powers off.
- Physical keyboard keys (`faraday/shells/stick/src/keyboard.rs`): one US layout, no XKB: letters, digits, space, hyphen, passphrase punctuation, Enter and keypad Enter, Backspace and Delete (both `Key::Backspace`), Escape, Tab (Shift+Tab is `BackTab`), arrows, keypad digits (Num Lock ignored). Caps Lock is never read; other keys map to None.
- The central `key()` dispatcher (`faraday/faraday-core/src/inputs.rs:~1277-1451`) routes keys to sheets, the guide's find, scroll, pass fields, entry, wallet rename, message, copy check, Escape and Paste.

## Effects

- In: `StorageEvent` input events (NewInput, InputTyped, InputTapped, InputUsed, InputGone, InputTrust, BootUsb); out: the `decisions` queue (`app.poll_input_decision()` returns (id, believed)).
- `session.keys.add_mnemonic` and `add_words`, wallet rename in `session.wallets` (key loading helpers in `faraday/faraday-core/src/inputs.rs`).
- Device discovery reads USB devices and the screen from the shell (`usb_devices`, `screen_changed`).
- Any capture device arriving takes every secret on show off the screen; Show brings them back once the device is gone and checked.
- The stick shell (`faraday/shells/stick/src/`):
  - discovery (`Watch`, `faraday/shells/stick/src/evdev.rs:~747-757`) scans `/sys/class/input/event*` (`~385-394`), again every `RESCAN` 2 s (`~358`) or 250 ms at start (`~362`);
  - one thread "input" per device and one "input-look" thread for the scans (`~812-822`);
  - `faraday_v4l2::grab_input` (EVIOCGRAB) on every device, so the kernel terminal never receives the keys;
  - a believed mouse's move is applied to the arrow and repainted on the input thread before the main loop hears (`~672-679`);
  - the start window file `start-window` is written in the boxes directory through `.start-window.tmp` and a rename.
- Power-off on the panic chord is init's (`poweroff -f` inittab line, `faraday/shells/stick/src/main.rs:~2483-2485`).

## State and lock

- `DeviceCheck` (`faraday/faraday-core/src/devices.rs:~285-320`) is `Public`: a lock keeps the device list. `over_import` remembers an Import sheet it covered.
- `InputsArea` (`faraday/faraday-core/src/inputs.rs`) is `Public`. Its counts (kept out, beside presses, wrong letters) are carried across a lock through the `input-counts` file and clamped on restore (`faraday/faraday-core/src/inputs.rs:~373-383`). Wrong letters count for the whole session and across a lock.
- The code and targets come from `fresh.draw(b"faraday input code", ..)` and `fresh.draw(b"faraday input target", ..)`; none before the seed arrives (`faraday/faraday-core/src/inputs.rs:~1063-1085`).
- Shell holds: `Pending | Believed | Ignored` per device, a `believed` `AtomicBool` in `Direct`. A restart in process after a failed lock save resets the flags and every hold to Pending (`faraday/shells/stick/src/main.rs:~1758-1763`).
- `Window { last, changed_at, closed, trusted: Vec<PathBuf>, laptop }` (start window); nothing secret; the file is its only persistence across a lock.
- `Keyboard { left/right shift, left/right meta, s, panic }` (all booleans) and `PanicTimer { since: Option<Instant> }` in `Shell.panic_timer`; no key material.
- Typed characters in the shell:
  - `buf[..n]` is zeroized after each read and `held.spare_capacity_mut()` after each drain (`faraday/shells/stick/src/evdev.rs:665-666`);
  - the `Wake` channel (`faraday/shells/stick/src/wakechan.rs:60-125, 170-200`): a bounded ring (`WAKES` 64) for the input threads, camera and core worker. `Slot::of` replaces a `Key::Char` with `'\0'` and holds it as u32 in `typed`; `take` zeroizes `typed`; `State::drop` zeroizes all slots; `WakeRx::drop` pops everything (wiping) and marks the receiver gone.

## Reach

- Holds presses of every feature that shows a secret (vault pages behind an eye, TOTP QR, seed QR, GPG Show as QR, Show SeedQR, New key, Back up, tools).
- Gives back the boot import's Import sheet (`boot-import`) through `over_import`.
- Reads the stick shell's events (`stick-shell`) and `storage` events; the start window uses `evdev::port_fixed` (`faraday/shells/stick/src/start_window.rs:196`); evdev uses `start_window::Window::admit` (`faraday/shells/stick/src/evdev.rs:897`).
- Pointer and touch decoding: `pointer-touch` (`crate::pointer`, `touch`, `hid::ranges`, `orient::Shared`).
- `may_load_keys` (self-test passed and no stick in the way, `faraday/faraday-core/src/inputs.rs:~1095`) is read by Files, the Inbox and the boot import.

## Rules and refusals

Device check (`faraday/faraday-core/src/devices.rs`):
- A virtual machine is held and is never let through (`faraday/faraday-core/src/devices.rs:471-473, 742-757`).
- KVM and TV screens are held; receivers are never held (`~388-402`).
- `device_let_through` only after the sheet asks (`faraday/faraday-core/src/devices.rs:753-760`); `device_understood` is refused while any item is held.
- `device_holds` holds only presses that show a secret: listed actions, pull actions, a vault page showing a secret behind an eye, a TOTP QR or a seed QR (`~598-643`). Hiding a field never waits.
- A waiting press is dropped when the sheet it waited under is replaced.
- Reminders only while a press waits (`~708-714`).
- Recognition: KVM names; Linux Foundation HID with mass storage; Bolt and other receivers by ID or a name heuristic (`~173-201`); VM names QEMU, VMware, VirtualBox, Hyper-V, Xen, Parallels or the hypervisor flag (`~233-258`); chassis lists (`~269-282`).
- A keyboard found at start-up or on a laptop is trusted; a PS/2 keyboard appearing later on a desktop asks for its code.

Held-back input (`faraday/faraday-core/src/inputs.rs:53-86`):
- `CODE_LEN = 6`, `CODE_TRIES = 3`, `KEPT_OUT_LIMIT = 3`, `TAP_TRIES = 3`, `BESIDE_LIMIT = 9`, `WRONG_LETTERS_LIMIT = 54`, `TARGETS_MAX = 16`, `ENTER_QUIET_MS = 1000`, `BOOT_NOTICE_MS = 10_000`; alphabet `b"acdefhjkmnprtvwxy"` (17 letters); a code letter is `byte % 17`.
- A keyboard that types three codes' worth of letters (18) without the code is ignored until unplugged.
- The door closes at 3 kept out, 9 beside presses or 54 wrong letters: later keyboards, touch devices and alone pointers are ignored on arrival with a toast ("New keyboards are ignored until Faraday starts again."). Devices the person ignored do not count.
- A trusted keyboard's keys stay on the device's sheet while it is up (F202).
- The test for an alone pointer is whether anything trusted can point; the arrow shows from the sheet's first frame.
- `load_key`: an Inbox text file, UTF-8 only; on a Restore back, a key that is none of the wallet's is refused.

Trust at the shell (`faraday/shells/stick/src/`):
- Roles (`faraday/shells/stick/src/evdev.rs:~240-264`): keyboard = EV_KEY + KEY_A + KEY_Z; mouse = EV_REL + REL_X + REL_Y; touchpad = EV_ABS + ABS_X + ABS_Y + (BTN_TOOL_FINGER or INPUT_PROP_POINTER) and not DIRECT; touchscreen = not a touchpad and (ABS_MT_POSITION_X or DIRECT). A device with no role is not opened (`~327-329`).
- The named panel (`--touch-name`) is the touchscreen only on a trusted bus; the first match wins and other devices' touchscreen flag is cleared; a USB device of that name is not the panel (`~431, 445-457`).
- A node already held by an input thread is not reopened (`~433, 892-894`); a node that now holds another sys path, or no longer exists, ends the old entry (`~889-891`), so a re-plugged device is read again. Candidates that cannot be opened now are found at the next look (`~422-437`).
- `trusted_by_bus` (`faraday/shells/stick/src/evdev.rs:483-494`): trusted on bus 0x18 (I2C) and 0x19 (host); on USB (0x03) only when `port_fixed` (every device and hub up to the root reads `fixed` in `removable`, `~496-521`); any other bus untrusted.
- `device.trusted |= admit(&device.sys)` (`faraday/shells/stick/src/evdev.rs:897`) for newly seen devices.
- Start window (`faraday/shells/stick/src/start_window.rs:14-150`): at power-on, devices seen while the window is open are trusted and recorded; it closes at `CAP` 8 s, or when the USB list has been quiet `QUIET` 1.5 s with the shell up `MIN_UP` 3 s. After it closes, only recorded sys paths are trusted, or on a laptop any device under an `i8042` path. `Window::load`: an absent file is power-on (opens); unreadable is closed, nothing trusted; a wrong header, no trailing newline or any non-absolute path is closed, nothing trusted. A save failure is logged to stderr only, and the next process would reopen the window.
- Start window tick (`faraday/shells/stick/src/main.rs:~1046-1085`): nothing when closed; no watch closes at once; when ready, one more rescan, close, save, then `usb::devices` and `start_window::names(...)`, which excludes hubs, devices with no HID (3), mass storage (8) or video (14) interface, devices on a hard-wired port, and the boot device's port. `BootUsb` only if any; nothing said when there are none.
- `usb::devices` (`faraday/shells/stick/src/usb.rs:35-59`): ports are names without `:`; a device without readable `idVendor` and `idProduct` is skipped; a bad interface class, subclass or protocol hex becomes 0; manufacturer and product name default empty.
- Reading (`read_loop`, `faraday/shells/stick/src/evdev.rs:~630-692`): a grab error other than ENOTTY means the device is not read at all; ENOTTY (a regular file for `--input`) is read as is (`faraday/shells/stick/src/evdev.rs:637-645`). A read of 0 or an error ends the read loop and sends `Wake::InputEnded(id)` (`~696-706`).
- `decode` (`~163-178`) refuses a size under 8 or a short buffer; sizes other than 8 or 16 give (0, 0) timestamps.
- Held devices (`faraday/shells/stick/src/evdev.rs:~670-684`, `faraday/shells/stick/src/main.rs:1915-1941`): an untrusted device's events are wrapped `Wake::Held(id, HeldInput)`. Pending: a typed `Key::Char` goes as `StorageEvent::InputTyped { id, ch }`; any other key once as `InputUsed` (only while not yet used); a touch Down as `InputTapped { id, x, y }`; anything else is dropped. Ignored drops all; Believed passes through. A held pointer with no aim target sends only `InputUsed` once; with targets the cursor moves and a press goes as `InputTapped`, and the cursor keeps no press.
- `trust()` (`faraday/shells/stick/src/main.rs:~1847`): without a watch (`--input`), pointing is trusted and typing is not. `alone()`: nothing trusted can point; the arrow shows only then or while a held pointer aims (`sync_arrow`, `~1887`). `tell_trust` sends `InputTrust { pointer, keyboard }` on change. Unplugging (`input_ended_inner`, `~1116-1138`) drops the hold, tells the app `InputGone`, and hides the arrow when nothing can point.
- `--input [kind:]PATH` (`input_device`, `~691-713`; kinds touch, mouse, pad, keys; a bare path is touch, trusted) makes a file stand in for a device and disables the watch (`~2224-2230`).
- Every input device, held-back ones too, is taken exclusively while the app runs. The kernel's terminal keymap is empty; the image build fails if it maps a key.
- `Wake` channel: `send` blocks while full; `try_send` refuses when full or the receiver is gone; a send after the receiver is gone returns `Rejected`; a ring size of 0 is read as 1 (`faraday/shells/stick/src/wakechan.rs:~144`).

Physical keyboard and panic chord (`faraday/shells/stick/src/keyboard.rs:185-262`):
- Only `EV_KEY` events; value 2 (repeat) makes a stroke only for Backspace, Tab, BackTab and arrows; unknown values are dropped.
- Super (either) never types; while Super is held key-downs are dropped and key-ups still pass; a key-up reads Shift again; Shift makes no stroke.
- Panic chord: Super and S held. `Stroke::Panic(true/false)` is sent only on change and wins before the drop rule; `PanicTimer` keeps the first time while held; `PANIC_HOLD` 2 s. `stop_after_input` (`faraday/shells/stick/src/main.rs:2966`) sets `panic` and `done` when due; `run` then takes the panic path (`stick-shell`).

## Domain calls

- `faraday_paper::edid` (`display`, `tv_maker`, `hdmi_or_dvi`) in the device check.
- `faraday_core::devices::UsbDevice { port, vendor, product, manufacturer, name, interfaces }` built by the shell.
- `faraday_v4l2::grab_input` (`faraday/shells/v4l2/src/lib.rs`), `zeroize::Zeroize`, `faraday_shell_api::{Key, Event, TouchPhase}`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 77ffe1d08 | A capture device arriving took secrets off screen only on the vault item page; New key, Back up and the tools' secrets stayed visible. | Any capture device arriving takes every secret on show off the screen; Show brings them back once the device is gone and checked. |
| 0405ffacd | A keyboard or mouse unplugged and plugged back within two seconds was taken for the device whose input thread had ended and never reopened. | An ended input entry goes when another device holds its node (by sysfs path); a re-plugged device is read again. |
| b93cf328a | With a KVM attached, Show as QR on a GPG key's page waited under the Devices sheet; when a new device brought Devices up again, the waiting press ran and showed the code. | A waiting press is dropped when the sheet it waited under is replaced. |
| 7e8a2b210 | With a KVM attached, Show as QR on a TOTP secret, the eye on a masked field, and Show SeedQR could show a secret without a hold. | Those reveals wait under the Devices sheet and carry on after the device is gone and checked; hiding a field never waits. |
| 72ec79ea9 | A PS/2 keyboard that appeared after start-up on a desktop was trusted without asking for its code. | A keyboard found at start-up or on a laptop is trusted; a PS/2 keyboard appearing later on a desktop asks for its code. |
| b111ae501 | A wireless receiver's keyboard mark disappeared; a laptop with a TV maker's HDMI screen lost the TV reminder; cameras and capture cards were missing from the sheet before a secret. | Mark shown while the receiver is attached; TV reminder for laptops; the pre-secret sheet names cameras and capture cards. |
| d96707c2f | A keyboard that unplugged before its last try could be replaced by a new one for fresh codes (wrong letters reset). | Wrong letters count for the session and across a lock; new keyboards are ignored after the limit. |
| 1e283e222 | With a device's approval sheet up, a trusted keyboard's keys reached the form under the sheet, so the keyboard could not reach Ignore. | A trusted keyboard's keys stay on the device's sheet (F202). |
| 7bf9d8633 | A held-back keyboard was accepted when its last six letters matched the code, with no count of wrong attempts. | A keyboard that types 18 letters without the code is ignored until unplugged. |
| fb272f048 | A device that unplugs and plugs itself back got a fresh code and 18 more letters each time. | Once three keyboards in a session are kept out for a wrong code, every new keyboard is ignored until Faraday starts again; devices the person ignored do not count. |
| c7fd03a21 | A keyboard behind its own hub on an external USB port was treated as built in, so it skipped the proof that a person is at the screen. | A device counts as built in only if it and every hub above it are fixed; the root hub ends the walk. |
| ec55eec25 | Keyboards and pointers, held-back ones too, were not taken exclusively, so the kernel's terminal received typed keys. | Every input device is taken exclusively while the app runs. |
| 87b48f203 | The kernel terminal keymap was not empty, so keys typed between app runs were queued for the terminal. | The kernel is built with an empty terminal keymap; the image build fails if it maps a key. |

## Doubts

- Left out: 0b6e31d67 (non-canonical base64 refused; a parser fix, not about input devices). 90c45fa1d and 0aaa0f66d (which disks count as attached sticks) are about the stick shell's attachment and are carried in `stick-shell`'s Fixes.
- c7fd03a21, ec55eec25 and 87b48f203 came with the stick shell's rows and are carried here.
- b111ae501's files were not inspected beyond its message; it also carries a Create a vault text change not about this feature.
- `usb_devices` has no production caller inside `faraday-core`; the host is presumed to call it (`faraday/faraday-core/src/devices.rs:504`, unverified).
- `devices::chassis_laptop` has no caller in its own file.
- `Direct.believed`'s first value for a trusted device is set by the caller and was not seen.
- The code letter `byte % 17` is slightly biased.
- The keyboard doc (`faraday/shells/stick/src/keyboard.rs:~25`, `~242`) says the chord "powers off at once"; the code waits a 2 s hold.
- `faraday/faraday-core/src/inputs.rs:~1080-1081` indexes 4 bytes of a `fresh.draw` result, assumed at least 4.
