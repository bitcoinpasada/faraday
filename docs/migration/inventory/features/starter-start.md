# Starting from the stick

The Starter's Start page helps a person start their computer from a Faraday
stick. It is reached from the "This computer" card on Home and from Done after
a stick is written. It shows:
- the steps for this computer and its maker (boot menu key, BIOS key, Secure Boot setting);
- a QR code of the guide's address for a phone;
- printable start notes;
- on Windows, BitLocker: suspend or resume protection, and print the recovery key on the notes;
- things attached that a person should know about: screens that record, KVMs, wireless receivers, a virtual machine.

It restarts the computer into its firmware setup, or (on Linux, and on
Windows through the advanced start menu) straight from the stick. Restart is
held while a risk is open.

## Screens and sheets

- Start page (`faraday/faraday-starter/src/draw.rs` `start`, `mac_start`):
  - The start steps.
  - The start guides (`StartGuide`).
  - The notes block.
  - The BitLocker block.
  - The phone QR code of the guide address (`faraday_codec::qr::encode`, `faraday/faraday-starter/src/lib.rs:~1043`), made once per change of computer (`phone_codes_make`).
- Mac Start:
  - No BitLocker block.
  - T2 steps unless `t2 == Some(false)`.
  - A T2-only warning when `t2` is unknown.
- Concern block (`concern_block`, `faraday/faraday-starter/src/draw.rs`):
  - For each thing attached: Continue anyway, which is not drawn for a virtual machine (`faraday/faraday-starter/src/draw.rs:582-590`).
  - Check again, always drawn.
  - The receiver line (`receiver_line`, `faraday/faraday-starter/src/lib.rs:~1117`): a receiver warning, or a Bolt line when only Bolt receivers are attached.
- `Sheet::LetThrough`: a hold to let one concern through.
- `Sheet::Restart(RestartTo::Firmware | Stick)`:
  - Restart now.
  - When a key is needed, Restart now is a hold instead (see Rules).
  - Cancel.
- `Sheet::StartGuide`: start guide pages:
  - StartWindows, StartLinux, NoStart, KeepData, Maker(name), SecureBootBitLocker, ScreensThatRecord, WhatElseRuns, BootEntry, MacIntel (`faraday/faraday-starter/src/start_guide.rs`).
  - The pages are `include_str!` of the 19 files in `docs/guide/start/`.
  - An Apple maker gets the Mac guide. A maker page is used only if `guide_id` finds one. macOS on a maker that is not Apple gets the Linux page.
- Notes PDF (`faraday/faraday-starter/src/notes.rs`):
  - The boot-menu key, the BIOS key, the Secure Boot setting, and the steps.
  - Circle-the-key lines, except on a Mac.
  - With the key tick, each recovery key: the drive, the eight groups four to a line, and the protector id.

## Actions

- `Start`: opens the Start page.
- `Restart`: restart into firmware; opens the restart sheet.
- `RestartStick`: restart from the stick; opens the restart sheet.
- `RestartNow` and `Cancel`.
- `Rescue`: on Start, sets `other_confirmed`, meaning another encryption product's rescue key is confirmed.
- `CheckAgain`: sends `StarterCommand::CheckAgain`.
- `ContinueAnyway(n)`, then `LetThrough` (a hold).
- `IncludeKey` (tick), `PrintNotes`.
- `SuspendBitLocker`, `ResumeBitLocker`.
- `StartGuide`.
- The Start page reads BitLocker protection each time it is shown.

## Effects

Commands the shell carries out (`faraday/shells/starter/src/main.rs:~319-547`), each slow one on its own thread:
- `RestartIntoFirmware`:
  - Linux: `systemctl reboot --firmware-setup` (`faraday/shells/starter/src/linux/mod.rs:~100`).
  - Windows: `shutdown /r /fw /t 0`, elevated (`faraday/shells/starter/src/windows/win.rs:~185`).
- `RestartFromStick { device }`:
  - Linux (`faraday/shells/starter/src/linux/boot.rs:~271`): one `pkexec` prompt to set `BootNext` with `efibootmgr`, then `systemctl reboot` as the user.
  - Windows: `shutdown /r /o /t 0`, unelevated (`faraday/shells/starter/src/windows/win.rs:~202`).
- `ReadBitLocker`, `BitLocker(Suspend | Resume)`, `ReadRecoveryKey`: WMI calls in an elevated copy, answered over the named pipe (`faraday/shells/starter/src/windows/bitlocker.rs`).
- `PrintNotes { name, pdf, with_key }`: writes the PDF and opens it in the system viewer (see Rules for the key sheet's folder).
- `CheckAgain`, and a check every 3 s by itself (`CONCERNS_EVERY`, `faraday/shells/starter/src/main.rs:~382-384`, `~791-810`). Event back: `Concerns`.
- Computer facts at start. Event back: `Computer`.
  - Linux (`faraday/shells/starter/src/linux/mod.rs:~390-429`):
    - dmi `sys_vendor`, `product_name`, `product_family`;
    - `/sys/firmware/efi`;
    - the SecureBoot EFI variable, byte 4;
    - the architecture;
    - a `crypttab` fourth field `tpm2-device`, meaning a TPM-unlocked disk;
    - the camera.
  - Windows (`faraday/shells/starter/src/windows/registry.rs:~84-117`):
    - BIOS strings;
    - UEFI only when `GetFirmwareType` is 2;
    - Secure Boot `Some(true/false)` from its DWORD, in UEFI only;
    - ARM for machine types 0xaa64, 0x01c4 and 0x01c0.
- Concerns read:
  - Linux sysfs (`faraday/shells/starter/src/linux/devices.rs`): `drm card*/status` and `edid`, USB devices, dmi, and the `/proc/cpuinfo` hypervisor flag.
  - Windows (`faraday/shells/starter/src/windows/devices.rs`).
- Camera check:
  - Linux sysfs (`faraday/shells/starter/src/linux/camera.rs`).
  - Windows (`faraday/shells/starter/src/windows/devices.rs:~221-256`).
- Events in: `Computer`, `Concerns`, `BitLocker`, `BitLockerDone`, `RecoveryKey`, `NotesOpened`, `RestartFailed`, `StickRestartFailed`, `OtherEncryption`.

## State and lock

- In the Starter (`faraday/faraday-starter/src/lib.rs`): `concerns`, `let_through`, `bitlocker`, `include_key`, `other_confirmed`. These are kept across a change of page (`go`, `~1991`). There are also flags: `key_printed`, `key_pending`, `key_reading`.
- Secret: the BitLocker recovery key.
  - `RecoveryKey.key` is `Zeroizing<String>`. `Debug` prints only the drive (`faraday/faraday-starter/src/api.rs:103-111`, `179-183`).
  - `StarterEvent::RecoveryKey(Result<Vec<RecoveryKey>, Reason>)` holds the keys until it is taken.
  - The `Vec<RecoveryKey>` is moved into `print_notes` and dropped at its end. It is not kept in `Starter` (`faraday/faraday-starter/src/lib.rs:1192-1210`, `1910-1920`).
  - `Notes::sheet` wipes its copies of the row labels and values after the PDF. It returns `Zeroizing<Vec<u8>>` (`faraday/faraday-starter/src/notes.rs:324-328`).
  - `StarterCommand::PrintNotes.pdf` is `Zeroizing<Vec<u8>>` (`faraday/faraday-starter/src/api.rs:1094`).
- In the shell:
  - The key-sheet PDF is wiped after it is written (`pdf.zeroize()`, `faraday/shells/starter/src/main.rs:410-430`).
  - The sheet's folder is recorded in `Shell.key_sheets`, only when the write succeeded.
  - `bitlocker::Value::Str` is `Zeroizing<String>`. The `reply` and `run` buffers are `Zeroizing<Vec<u8>>`, and each line is wiped after decoding (`faraday/shells/starter/src/windows/bitlocker.rs:~901-970`).
  - A top-level `VT_BSTR` is wiped on drop (`faraday/shells/starter/src/windows/sys.rs:685-696`).
- The key sheet file goes when the program ends:
  - `remove_key_sheets` runs in `exiting` and after `run` (`faraday/shells/starter/src/main.rs:744-746`, `1001`).
  - `sweep_key_sheets` at start removes what an earlier run left (`faraday/shells/starter/src/main.rs:914`).
  - It is not wiped at a Demo lock or when leaving the Demo.
- The Starter has no lock. Its state lasts as long as the process.
- `StarterCommand` derives `Debug` (`faraday/faraday-starter/src/api.rs:1024`), so `PrintNotes`' `Zeroizing` PDF, which carries the recovery key, would print in full. Nothing outside tests formats a command: the shell matches each one (`faraday/shells/starter/src/main.rs:410-430`), and its prints carry no command.

## Reach

- Opened from: Home's "This computer" card (`starter-app`), and Done after writing (`starter-make-stick`).
- Reads: the disk list and Faraday-stick test (`starter-make-stick`) for `RestartStick`, and the computer facts, which Home's checks also read.
- Opens: the guide sheet and start guides (`starter-app` guide).
- The device's power-off sheet shows the same guide address for the same computer (`secure-boot`).
- Home's checklist and card choose between Make, Rescue, Stick and Firmware from the same facts (see `starter-app`).

## Rules and refusals

**Restart holds** (`restart_holds`, `faraday/faraday-starter/src/lib.rs:1103-1110`):
- A virtual machine always holds; it is never let through.
- A TV or KVM holds unless let through.
- A receiver or Bolt receiver never holds.

**Restart and RestartStick**:
- `Restart` (firmware) only on `Page::Start`, not on a Mac, not `protected_open`, and with no holds.
- `RestartStick` also needs `use_a_device` and a stick attached (`faraday/faraday-starter/src/lib.rs:2326-2333`).

**Restart now**:
- With holds present, `RestartNow` closes the sheet without restarting (`faraday/faraday-starter/src/lib.rs:2335`).
- Otherwise it sends `RestartIntoFirmware` or `RestartFromStick { device }`.

**When the restart needs the key** (`restart_needs_key`, `faraday/faraday-starter/src/lib.rs:1162-1169`): it is true when no key was printed this run and BitLocker is unread, or offered with protection On. `RestartNow` is then a hold, only on a Restart sheet (`faraday/faraday-starter/src/scroll.rs:161-172`). Put the other way: while BitLocker is on, not suspended or unreadable, and no recovery-key sheet has opened, Restart is a hold.

**Let through** (`faraday/faraday-starter/src/lib.rs:2521-2533`):
- `ContinueAnyway(n)` opens `Sheet::LetThrough` only for a concern that is present, not yet let through, and not a virtual machine.
- `LetThrough` is always a hold. It records the id in `let_through`.
- The sheet closes when the concern goes away.
- Concerns are never let through for a virtual machine.

**Concerns** (`faraday/faraday-starter/src/concern.rs`):
- A screen is judged from its EDID only, as TV or KVM.
- A KVM is gadget vendor 0x1d6b with keyboard and disk interfaces, or a name `names_kvm` knows.
- The Bolt receiver is 046d:c548. Receivers are 046d:c52b, c532, c52f, c534, c539 and c548 (`55-67`, `90-100`).
- A HID keyboard or mouse with a receiver word, or "2.4G", is a receiver.
- A virtual machine (`123-137`) is any of:
  - a hypervisor word in the maker or model;
  - Hyper-V: maker MICROSOFT plus "virtual machine";
  - the CPU guest flag. This can return `Some("")` when the maker and model are empty.

**Windows concerns** (`faraday/shells/starter/src/windows/devices.rs`):
- The internal panel is never a concern.
- Analogue TV outputs (1, 2, 3, 8, 14) are TVs.
- A hypervisor is a concern unless this is the Hyper-V root partition (`~179-183`).
- USB devices with no hardware ID are skipped.

**Camera check** (Usable, NotUsable, None or Unknown):
- Linux (`faraday/shells/starter/src/linux/camera.rs`):
  - It needs an uncompressed VS format (subtype 0x04, at least 27 bytes, at least one frame), `uvcvideo`, an index 0 capture node, and a streaming interface of class 0x0e, subclass 0x02.
  - An unreadable index counts as capture.
  - MJPEG and frame-based formats do not count.
  - Each physical device is counted once.
- Windows (`faraday/shells/starter/src/windows/devices.rs:~221-256`):
  - The `Camera` class always counts. The `Image` class counts only with `usbvideo`.
  - A camera is usable only if it is USB and `usbvideo`. A compressed-only camera is taken as usable.
  - An unreadable list is `Unknown`.

**Mac** (`faraday/faraday-starter/src/macs.rs:206-242`, table `faraday/faraday-starter/src/macs.tsv`):
- `start`:
  - A maker that is not Apple: None.
  - ARM or Apple silicon: NotYet.
  - 32-bit EFI: Firmware32.
  - Otherwise: Intel { t2 }.
- `stick_for`:
  - An Apple silicon stick only for M1 or M2.
  - A PC stick for Intel, or for an unknown computer that is not ARM.
  - None for Firmware32 or other ARM.
- A malformed `macs.tsv` row panics at first use.

**BitLocker in the Starter** (`faraday/faraday-starter/src/lib.rs`):
- Offered only on Windows, not on a Mac, with protection On or Suspended (`bitlocker_offered`, `~1137`).
- Suspend only when On, Resume only when Suspended. Both only on the Start page, and only with nothing under way (`step_open`, `~1175`).
- `SUSPEND_STARTS` = 3: suspension lasts three starts.
- The `IncludeKey` tick is taken only on Start, with BitLocker offered.
- With the tick, Print sends `ReadRecoveryKey`.
  - An empty key list gives `Reason::NoRecoveryKey` (`~1915`).
  - The PDF is made only on `Ok(keys)`, while on `Page::Start` and with `include_key` (`1910-1920`).
- `Notes::for_computer` returns None for a 32-bit-EFI Mac, Apple silicon or ARM, so nothing is sent (`~1194`).

**Recovery key parsing** (`RecoveryKey::parse`, `faraday/faraday-starter/src/api.rs:~120-152`):
- The drive is `X:`.
- The GUID is 8-4-4-4-12 hex, with braces optional, stored in capitals.
- The key is eight groups of six digits joined by dashes.

**BitLocker in the shell, Windows** (`faraday/shells/starter/src/windows/bitlocker.rs`, WMI through the `Wmi` trait):
- Arguments are exactly `--bitlocker key|suspend|resume --pipe <pipe>`.
- `shell_protection`: 1, 3 and 6 are on; 5 is suspended; anything else is unknown. 2 is not trusted as "not encrypted".
- `protection_of`: 1 is on. 0 asks `GetConversionStatus`: 0 is none, any other value is suspended.
- `read_key` refusals: `NotRead`, `NotEncrypted`, `NoRecoveryKey`, refused.
- Every numerical-password protector's ID and key is printed.
- Suspend (`436-470`):
  - Uses `DisableCount = SUSPEND_STARTS`.
  - On a refusal it resumes the volumes already suspended in the call, and returns `Managed(ret)`.
  - If any target still has protection other than 0 afterwards, it undoes the call and returns `Managed(0)`.
- Resume refuses when `EnableKeyProtectors` fails. It does not undo the earlier volumes (`~475-500`).
- VeraCrypt is detected by a registry key only (`faraday/shells/starter/src/windows/win.rs:~110`).
- A declined elevation gives `Declined` for an admin, and `NoAdmin` otherwise (`faraday/shells/starter/src/windows/win.rs:~299-300`, `~618-619`).
- The pipe's client must be the elevated child (`faraday/shells/starter/src/windows/win.rs:311-316`). A failure is reported as `GetNamedPipeClientProcessId` with code 0.
- On Linux BitLocker is `Unsupported` (platform default).
- `record_bitlocker` reads no recovery password (`faraday/shells/starter/src/windows/win.rs:~715-717`).

**Key-sheet folder** (`faraday/shells/starter/src/pdf.rs:~45-146`; `121-146`):
- It is under the user's private runtime folder, else under temp. The runtime folder must be absolute, a real directory and not a link, owned by the user, with no group or other bits.
- The folder is mode 0o700, with a random name from `/dev/urandom`. It is checked private afterwards, else removed with `Reason::KeyFolder`.
- The file is made with `create_new`, mode 0o600. A file name with no name part is refused.
- Off Unix, `write_in` makes no private-folder or owner check (`~145`).

**Key-sheet wipe** (`remove_key_sheets`, `faraday/shells/starter/src/pdf.rs:~153-194`):
- Each regular file is overwritten with zeros and removed, then the folder.
- It never follows a link. Each file is checked, once open, to be the one listed.
- The sweep at start (`~209-228`) removes only private `faraday-key-` directories. Off Unix, every `faraday-key-` folder is taken as the Starter's own.

**Print** (`faraday/shells/starter/src/pdf.rs:~232-276`):
- Print PDFs go to a `faraday-print-` folder in temp, made with `create_dir`, which fails if it exists.
- They are opened with `explorer.exe` from `SystemRoot` on Windows, or `xdg-open` elsewhere. There is no print verb.
- A missing viewer is `Reason::Missing`.

**Restart from the stick, Linux** (`faraday/shells/starter/src/linux/boot.rs`):
- The device must be `/dev/` plus alphanumerics, else `NoStick` (`152-158`). An empty GUID is `NoStick`.
- `BootNext` is set to the Faraday entry that matches the partition GUID. Otherwise a new entry is made with `--create-only`, never in BootOrder.
- Each stick has its own entry. A restart touches only the attached stick's entry, with one pkexec run, and keeps other sticks' entries.
- No free number is `BootEntry`. A pkexec exit of 126 or 127 is `Declined` (`262`).
- Privileged tools are found only in `/usr/sbin`, `/usr/bin`, `/sbin` and `/bin`. Each must be a regular, executable file, owned by uid 0, not group- or world-writable. PATH is never used (`161-192`). A missing tool is `Missing(name)`.
- A non-zero exit of the restart command gives `Restart`. The default platform (`Elsewhere`) refuses with `Unsupported`.

## Domain calls

- `faraday_paper::pdf::{notes, raw_document, pdf_text}`, `faraday_paper::edid::{display, names_kvm, words}`.
- `faraday_codec::qr::encode` (the guide address QR).
- `faraday_guide::{MAKERS, GUIDES, maker, guide_id, guide_address, page_of, guide_page, step}`.
- `faraday_crypto::work::Compute` (only as a cost token for the PDF).
- `zeroize::{Zeroizing, Zeroize}`.
- In the shell: `windows_sys` (WMI, registry), `rustbus`. No call into keys, vault, psbt or sb from these steps.

## Fixtures

- `faraday/shells/starter/tests/efibootmgr/faraday-two-stale.txt`
- `faraday/shells/starter/tests/efibootmgr/faraday-verbose.txt`
- `faraday/shells/starter/tests/efibootmgr/one.txt`
- `faraday/shells/starter/tests/efibootmgr/several.txt`
- `faraday/shells/starter/tests/windows/control-panel.wmi`
- `faraday/shells/starter/tests/windows/data-only.wmi`
- `faraday/shells/starter/tests/windows/managed.wmi`
- `faraday/shells/starter/tests/windows/no-password.wmi`
- `faraday/shells/starter/tests/windows/not-encrypted.wmi`
- `faraday/shells/starter/tests/windows/pro-resume.wmi`
- `faraday/shells/starter/tests/windows/pro-suspend.wmi`
- `faraday/shells/starter/tests/windows/pro.wmi`
- `faraday/shells/starter/tests/windows/suspended-converted.wmi`
- `faraday/shells/starter/tests/windows/two-passwords.wmi`
- `faraday/shells/starter/tests/windows/two-systems.wmi`
- `faraday/shells/starter/tests/windows/undone.wmi`
- `faraday/shells/starter/tests/windows/vm-on.wmi`
- `faraday/shells/starter/tests/windows/vm-suspended.wmi`

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 63c1b7eea | On Windows the BitLocker step could not finish: the elevated copy's pipe was inbound only, so switching it to wait mode was refused. Suspend failed because the code used RebootCount, not DisableCount. | The pipe is duplex, with the access list unchanged; the WMI call uses DisableCount, so suspension happens. |
| 5d818fb5b | Uninstall could leave the install folder (the after-exit command used `timeout`). After Uninstall, Home showed a drive Faraday had suspended as "Protected". | Uninstall finishes, waiting with `ping`; a suspended drive reads "Suspended" whoever suspended it. |
| 343a31117 | On Windows, Restart ran with BitLocker on or unread and no recovery key shown, so the restart was not held. | While BitLocker is on, not suspended or unreadable, and no recovery-key sheet has opened, Restart is a hold. |
| 6cc0ab0f2 | The BitLocker group fields had no cursor: typing went to the end and Backspace removed the last digit. | Home and End move the cursor in the focused field; typing and Backspace act at it; Ctrl+A selects the digits. |
| 14d8f76a0 | A key sheet was written to a folder other users could read, and the sweep could touch files the Starter did not own. | A key sheet goes only to a fresh random-named folder, mode 0700, owned by the person; the file is new with mode 0600; the sweep touches only its own files. |
| 0ea78ac6f | Restart from the stick on Linux kept a boot entry naming the boot partition by GUID, so it started nothing for another stick or a remade one. System tools were found on PATH and unowned files were run. | The entry is replaced for each stick; only root-owned programs in system folders run. |
| 1927556c7 | Restart from the stick on Linux deleted entries for other sticks and could leave extra prompts; the listing lacked device paths. | A restart touches only the attached stick's entry, with one pkexec run; other sticks' entries are kept. |
| 70162b5b3 | The key sheet did not print the key protector's ID; the after-exit `cmd.exe` got a wrong switch. | The protector's ID is printed with its key. |
| a687c8692 | The key sheet printed only one numerical-password protector and its key. | Every numerical-password protector's ID and key is printed. |
| 153b7b72c | The key sheet's read-again loop did not compile; an empty key list did not say there was no recovery key. | An empty list says no recovery key; each ID is paired with its key. |

## Doubts

- Seven history rows were moved here from `starter-app`, because they are about the Start page, restart or BitLocker: 6cc0ab0f2, 14d8f76a0, 0ea78ac6f, 1927556c7, 70162b5b3, a687c8692 and 153b7b72c.
- 76aaee791, a Secure Boot certificate time, was moved to `secure-boot`.
- 63c1b7eea and 5d818fb5b name files under `faraday/shells/hub` and `faraday/faraday-hub`, the Starter's earlier names.
- A key sheet whose `write_all` fails after the file is made is not recorded in `key_sheets`. It is left until the next start's sweep.
- On Windows, `tpm_unlock` is fixed to `false` (`faraday/shells/starter/src/windows/registry.rs:114`), so Windows never reports a TPM-unlocked disk. Intent unclear.
- Windows takes compressed-only cameras as usable, and Linux does not. Whether this is by design is unclear.
- `concern::virtual_machine` can return an empty name. Whether callers handle it is unknown.
- `systemctl reboot` after pkexec runs as the user. Not checked whether logind allows it.
- Other pkexec dismiss codes are not known.
- WMI `Next` waits with `WBEM_INFINITE`, with no bound. Several unrelated WMI failures are all reported as `NOT_FOUND` (`faraday/shells/starter/src/windows/sys.rs:~1157`, `~1177`, `~1206`, `~1316`).
- `--record-bitlocker` ships in the Windows binary (`faraday/shells/starter/src/windows/win.rs:~678-684`). Whether a user can reach it is unclear.
- The doc comment at `faraday/shells/starter/src/pdf.rs:~5-6` names `open` for macOS, but the code uses `xdg-open`.
