# STARTER build log: the batches and what changed from v0.1, as written at the time

**Kind:** History
**Status:** Kept as written; section numbers are the spec's when it was written. The current spec is `docs/STARTER.md`.

## 11. Batches

| Batch | What | Needs | Order |
|---|---|---|---|
| R | Release groundwork: the glibc 2.31 container for the desktop app; `.xz` images; the new names; the Linux `.tar.gz`; `SHA256SUMS` with the `.img` and inner-file lines; the Starter's payload build; the xz decoder and its `docs/deps` note; README "Verifying a release" for Windows and the two Linux steps | decided | first |
| L | The Starter on Linux: crates (§2), the D-Bus crate and its `docs/deps` note, Home as a checklist, Make a stick through udisks2 (with what each tried distribution asks for), the stick's version read from FARADAYBOOT and the keep-data rules, Make a Pi card for the one image, Install and uninstall, Try the desktop app, Transfer and Learn, About, `--image` for developers; then `tools/flash-stick` removed and the README's mention of it with it | decided | after R |
| G | Guides: the general Windows and Linux pages and §7.2's other pages with their sources; detection on Linux; Restart into the BIOS on Linux; the stick's power-off reminder (§7.5); the first group of maker pages, several families at once (D6, F14); the website item in its own repository | decided | after R, independent of L and W |
| W | The Starter on Windows: window, install and uninstall, disks and writing, detection, the firmware buttons, Transfer Send; Defender run; the report on what Windows does to a written stick (D11). Its BitLocker guard, marker, logon reminder and acceptance were built and are removed by F33a | decided; D11 is answered by this batch's report | after L; needs G's general Windows page only |
| F33a–F33d | No BitLocker step: removal in the Starter, the stick's power-off line, the guides' last step (§6.5) | decided | any time |
| C | Windows camera for Transfer Receive, shared with the desktop app; the desktop app on Windows (§2's changes) and its install | built (HANDOFF item 76): C1, the desktop app on Windows with its install, its Try and a payload folder per system; C2, the camera on Media Foundation. To be tried on Windows. C1's install, Try and payload desktop app superseded by F30 (`docs/DEMO.md` DM2): the desktop app on Windows stays as a developer's build | after W |
| P | Pi: one image for the 2.8 inch panel, Make a Pi card tried on hardware; profiles, boards and other sizes when a second screen is tried | decided (D1, D2, D3, D5) | after L, independent of G, W and C |
| S | Keeping Secure Boot on (§7.7) | D9 | last |
| T | Blank templates (§3.8): the row, the screen, the PDF saved into Downloads, from `faraday-core`'s existing sheet and PDF code | decided (F31) | after L; before the backup flow's batch B1 (`docs/BACKUP-FLOW.md` §14), whose Templates item points here |
| MAC A3a, A3b | The Starter's macOS module, and Home on a Mac, without a Mac (`docs/MAC.md` §8, HANDOFF item 73) | decided | after L (owner, 2026-10-13) |
| MAC A3c, D | The Starter built and tried on a Mac; the desktop app on macOS (batch D superseded by F30: dropped from the plan) | a Mac (MAC M7) | after A3b |

## 13. What changed from v0.1

Changed on 2026-10-13 by F17 (owner): the BitLocker step on Windows is
by default the recovery key in hand (where it is also kept, Show the
recovery key without typing it back, **I have my recovery key**, which
opens the firmware steps for the session and changes nothing on the
computer); suspending is **Extra safe: suspend BitLocker**, the guard
as built, offered and never required. §0, §3.1's table, §3.4 step 1,
§6's requirement, §6.2, §6.5 and §6.6 (runs 7 and 8) follow it. The
guides say to choose the stick from the boot menu on the restart that
saves Secure Boot off, and to turn Secure Boot back on before Windows
starts again.

Changed on 2026-10-14 by F21 (owner): the BitLocker step is optional.

Changed on 2026-10-14 by F33 (owner): there is no BitLocker step. §6 is
now Secure Boot back on: what the Starter does on Windows, no Resume
for any release, what stays
(another encryption product, Linux's TPM line), batches F33a to F33d,
what is removed, and the guides' new text. §0, §2, §3.1 (no BitLocker
item; *Use a device* only with Secure Boot reported off), §3.1a, §3.1b, §3.4 step 4, §5.1,
§5.2, §5.4, §7.1, §7.2, §7.3, §7.5 (the power-off line, its place,
tone and states; the idle power-off unchanged), §7.7, §10.2, §10.3,
§11 and §12 follow it. Specified, not built.

Changed on 2026-10-19 by F49 (owner): the Start page offers, never
requires, Temporarily suspend BitLocker (for the next 3 times Windows
starts), Resume BitLocker, and the recovery key on the printed sheet
behind a tick box; the sheet gains lines to circle the boot menu key
and the BIOS key used (F47). §0, §3.1b, §3.4, §6 (rewritten: §6.1's
when it is not enough, §6.2, §6.3 what is not brought back, §6.5's
F49 batch and tests, §6.7's pointer) and §12's D7 follow it. Built.

From v0.1:

- Home is a checklist of the path, with live state, not a menu (D12).
- Linux writes through udisks2 without root; pkexec is the fallback
  only (D13). The Linux download is a `.tar.gz` (D14).
- Which stick never pre-selects a disk, shows what is on each disk
  and the Faraday version a stick holds, and keeps the device path and
  serial for Confirm. Keep data refuses the same build and warns about
  an older one (D15).
- The guard shows and confirms the key before suspending, hides the
  key while the groups are typed back, offers a QR code and a photo
  for the phone, and offers Save only when there is somewhere to save
  (D16, D18). It names the two cases it cannot pass (no administrator
  account, a managed computer).
- No SYSTEM startup task. A marker file, Resume BitLocker offered only
  with Secure Boot on, and a per-user logon reminder for an installed
  copy (D17). Uninstall handles a standing suspension.
- §6.6 tests Pro in a virtual machine and Home on a computer of the
  owner's, with the owner (D19).
- §3.4 shows only the steps that apply, including the case of Secure
  Boot already off, and puts the phone QR on the restart confirmation.
- §7.6 ships the general pages plus tried makers (D6). §7.7 notes that
  enrolling a key also changes PCR 7.
- The desktop app comes to Windows in batch C (D20); its install check
  box is on both systems. `HOME` and `mlock` get their Windows
  equivalents.
- One Pi screen for now; profiles and other sizes wait for a second
  screen (D1, D5). The Pi image name carries the board and, for now,
  the panel.
- Payload files are checked when used, not at start. Linux installs to
  `~/.local/bin`. Batch W no longer waits on D11; G is independent of
  L and W; P of G, W and C.
- `tools/flash-stick` is replaced by the Starter's `--image` and
  removed in batch L (D21).
- §10.2 writes down what the website and README say when an antivirus
  blocks the file.

Folded in on 2026-10-13 from `docs/MAC.md` §6 (F13), when the owner
picked up the Mac work with the Starter:

- §1.1: the macOS Starter, the macOS desktop app, the Apple Silicon
  stick image and the start-up file, each with its batch.
- §2: the `macos` module (disks through `diskutil`, writing through
  `authopen`, unmounted rather than ejected after a write so the stick
  stays listed), type-checked and tested without a Mac until there is
  one.
- §3.1: Home on a Mac, on any system: no Secure Boot line or firmware
  restart; the Intel Mac's checklist; one line for a Mac that cannot
  start Faraday; the Apple Silicon checklist for later.
- §3.2: *For which computer?*, once there are two stick images.
- §3.4: the Intel Mac's steps, none of them a button.
- §5.2: install on macOS. §7.1: the facts on macOS and the Mac table.
- §10.2: ad-hoc signed, not notarized; *Open Anyway*. §10.3: the
  Apple Silicon start-up volume.
- §11: MAC batches A3a, A3b, A3c and D.

Changed on 2026-10-15 by F30 (`docs/DEMO.md` §10, batch DM2): no
`faraday-desktop-` file in §1.1; the payload is the two images on each
system (§1.3), about 36 MB; Home has no Try the desktop app (§3.1);
Install is one press (§3.5); the Starter installs, lists, runs and
removes no desktop app and keeps no cache (§5.2 to §5.4); C1's Starter
parts and MAC batch D are superseded (§11, §12).

Sections below moved out of `docs/STARTER.md` on 2026-10-22; the source keeps a
one-line stub under each heading.

## Header (from docs/STARTER.md)

**Kind:** Spec
**Status:** Built; the Mac parts (`docs/MAC.md`) are not

Spec v0.2 · 2026-10-12 · decided by the owner in discussion
(`docs/DECISIONS.md` F11 and F12); since F33 (2026-10-14) no BitLocker
step, and since F49 (2026-10-19) BitLocker's suspension, Resume and the
key on the sheet offered, never required, §6. v0.2 is v0.1 with the
owner's answers to a review of it: §12 records each decision; what
changed is in the build log. No decision stays open: D11 was taken on
2026-10-19.

## §0 Rules for the implementing agent (from docs/STARTER.md)

- `CLAUDE.md` applies in full: every build and test on the build machine; every dimension a token; labels, not
  prose, on working screens (explanations go in Learn pages); tests for
  user-facing behaviour only; `just` once at the end of a batch; no
  `rm`; no commits; no agents.
- Batches in the order of `docs/history/STARTER-LOG.md` §11, except where it says a batch is independent;
  one review and one completion report each. A batch that needs an
  owner decision from §12 does not start until the owner has confirmed
  it.
- There is no BitLocker step (F33, §6): the step is turning Secure
  Boot back on before Windows starts. Since F49 the Start page offers,
  never requires, to suspend BitLocker for three times Windows starts,
  to resume it, and to print its recovery key on the sheet; it shows no
  key on screen and keeps none. Another encryption product
  closes the firmware steps until its rescue is confirmed (§6.4).
- The Windows builds are tried on a Windows computer the owner
  provides. §6.6 says which of its runs may happen on an owner's
  computer (the Home run, with the owner) and which never do (anything
  that resets firmware keys or leaves Secure Boot off).
- The implementing agent never runs a writer against a disk it has not
  been given for the purpose.
- Nothing in the Starter reaches a network. No HTTP client, no update
  check, no telemetry, no crash reporting. The one link it shows (the
  guides on the website, §7.4) is a QR code a phone opens.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## §6.5 Batches (F33) (from docs/STARTER.md)

Specified, not built. Each is one review and one completion report;
F33a, F33b and F33d may land together and be built once. Every batch
changes `faraday-strings` or the Learn pages, which `faraday-core`
carries, so each counts for every app: Faraday (the desktop app, the
stick and the Pi) and Faraday Starter.

| Batch | What | Needs |
|---|---|---|
| F33a | Removal in the Starter (§6.6's Starter list), with no Resume kept (§6.3); Home and the Start page as §3.1 and §3.4 now say; *Use a device* only with Secure Boot reported off | none |
| F33b | Dropped with the Companion (F34); `faraday_qr::bitlocker` is removed with the Starter's use of it | none |
| F33c | The stick's power-off reminder (§7.5): the line, its tone and place, the unread state; the idle power-off unchanged | none |
| F33d | The guides' new text (§6.7), `bitlocker.md` removed from `faraday-guides`; the website's `start/bitlocker` page goes at its next build, in its own repository | none |

**Tests**, user-facing:

- The stick's power-off sheet shows "Turn Secure Boot back on in the
  BIOS before starting Windows." when the stick started with Secure
  Boot off, and when the state was not read; not when it started with
  Secure Boot on. The Pi's power-off sheet and Faraday Desktop's never
  show it (`faraday-core/tests/all/secure_boot_reminder.rs`, whose
  "unknown says nothing" test changes to the rule it now encodes).
- Every guide's last numbered step begins "Turn Secure Boot back on"
  (the Windows, Linux and maker pages) (`faraday-guides`).
- Nothing in the Starter asks for or holds a recovery key unless
  BitLocker reads on (F49 replaced F33's "on Windows with a protected
  drive" here): with BitLocker not read, Home and the Start page show no
  BitLocker item, row or key, and the only thing sent to the computer
  is the Start page's reading of BitLocker; no marker or logon entry
  ever.
- On Windows, **Restart to choose a device** and the card's **Restart
  from the stick** are shown with Secure Boot reported off, and hidden
  with it reported on or unread (`UEFISecureBootEnabled` 1, or no
  value), against recorded registry answers.
- Another encryption product still closes the firmware steps until its
  rescue is confirmed.

**F49, built 2026-10-19** in one batch: `faraday-starter` (`api.rs`'s
`BitLocker`, `Protection`, `RecoveryKey`, `BitLockerStep`, the events
and commands; `lib.rs`, `draw.rs`, `notes.rs`, `examples/snap.rs`),
the shell (`windows/bitlocker.rs`, `sys.rs`'s WMI and property store,
`win.rs`'s elevated verbs, `pipe.rs`'s reasons, `platform.rs`,
`main.rs`, `pdf.rs`'s key sheets), `faraday-strings`, the guides
(§6.7). Tests, user-facing:

- With BitLocker on, the Start page offers Temporarily suspend
  BitLocker and the tick box, between Write down and the restart; the
  restart is offered as without it (`faraday-starter/tests/all/bitlocker.rs`).
- Suspend asks once ("Waiting for administrator permission"); after it
  the row reads "Suspended · turns back on by itself after Windows
  starts 3 times" with Resume BitLocker, and keeps the count when the
  page is shown again; Resume returns to On; a refusal says why.
- With BitLocker off, not read, or not on Windows, none of it shows and
  the sheet has no key.
- The sheet carries the key only when ticked: the drive and the 48
  digits, nothing else; no document information, path, computer name
  or key ID; no QR code (no filled squares); the key is never drawn.
- The elevated copy's verbs against recorded WMI answers
  (`shells/starter/tests/all/bitlocker.rs`, `tests/windows/*.wmi`, restored
  from before F33 and adapted): the key read and nothing changed, no
  recovery password added; Suspend with `DisableCount` 3 on operating-
  system volumes only, read back with 3 left; a managed computer's
  refusal leaves nothing half done; a drive already suspended is left as
  it is; Resume read back; the pipe's lines round-trip and are wiped;
  the real virtual-machine recordings read as on, suspended (no count)
  and not encrypted.
- The guides say the key may be asked for with Secure Boot back on and
  that suspending first avoids it, and no longer say it is not needed
  (`faraday-guides`).

Only real Windows can show: the elevation prompt on an administrator's
and a standard account, the shell property and WMI on Windows Home with
Device encryption, a managed computer's refusal code, how Windows
counts the starts, and whether the viewer holds the key
sheet open when the Starter exits.

## §6.6 What is removed (from docs/STARTER.md)

- **Faraday Starter** (`faraday/faraday-starter`): the BitLocker page,
  Home's BitLocker item, its note, states and "Key shown", the Start
  page's BitLocker row, the guard's steps (show, type back, suspend,
  done, resume) and the TV hold on Show the recovery key, in `lib.rs`,
  `decide.rs`, `api.rs`, `draw.rs` and `examples/snap.rs`; the
  dependency on `faraday-qr` for the key; `tests/all/bitlocker.rs`,
  `tests/unreadable_bitlocker.rs`, and the BitLocker cases of
  `tests/all/starter.rs` and `tests/all/concern_holds.rs`.
- **The Starter's shell** (`faraday/shells/starter`):
  `src/windows/bitlocker.rs`; the BitLocker reading and WMI calls in
  `win.rs`, `sys.rs` and `mod.rs`; `--bitlocker` and `--remind` in
  `main.rs` and `pipe.rs`; the logon Run entry in `install.rs`; the
  key file Save; `platform.rs`'s BitLocker facts; `tests/all/bitlocker.rs`,
  the BitLocker cases of `tests/all/windows.rs`, and the recorded WMI
  answers in `tests/windows/` that serve only them. The VeraCrypt
  driver check stays.
- **Shared:** faraday/faraday-qr/src/bitlocker.rs, its `pub mod`, its
  `zeroize` use where only it needs it, and `tests/bitlocker.rs`; the
  BitLocker cases of `faraday-core/src/testkit.rs`.
- **Strings:** every `starter_` string in
  `faraday/faraday-strings/src/en.rs` used only by the above (about 70:
  the guard, the page, the states, the key, **BitLocker first**).
- **Learn:** docs/learn/faraday/boot/bitlocker.md and its entry in
  `faraday-guides`; the lines §6.7 lists.
- **Docs:** `docs/history/BITLOCKER-ACCEPTANCE.md` is superseded (its header
  says what remains). `tools/winvm/` stays: it is how the Windows
  Starter is tried, and a Windows with BitLocker on is the computer the
  guide is checked on.

## §7.5 The stick reminds: its old status line (from docs/STARTER.md)

(owner, 2026-10-12, D8; made the step's backstop by F33, 2026-10-14.
Built as a muted line in batch G; F33's changes are batch F33c,
specified, not built.)

## §3.1d Building it (from docs/STARTER.md)

**Building it** (planned; Sonnet from this section, then the owner's
look on Linux and Windows at a narrow and a wide window):

1. **N1, the frame.** The tokens; the size class in `faraday-starter`
   (replacing `compact`'s single 600 dp test); the rail, the bar and the
   top app bar as `faraday-ui` components (DESIGN §4.1); `Page` gains
   `Make` and `Tools`; the frame draws the rail or the bar on the four
   destinations and the back-arrow bar on task pages.
2. **N2, the pages.** Home's This computer card with the checklist
   inside it, the other cards in the grid; Make and Tools; the ⋮ menu;
   Install and Update in the bar. Strings in `faraday-strings`.
3. **N3, Transfer and Learn** drawn under the top app bar (§2's held
   `Faraday` given the space below it; batch R's way of holding it is
   kept).
4. **Tests** (user-facing): at 400 dp the bar shows the four
   destinations and at 700 dp and 1300 dp the rail does; each opens its
   page; a task page shows neither and Back returns to the destination
   and its scroll; every screen has one primary button; the ⋮ menu
   lists Uninstall only while installed and Remove the boot entry only
   while offered; Install and Update appear in the bar as §3.5 says;
   Larger text (F85) at 700 dp takes the compact layout.
