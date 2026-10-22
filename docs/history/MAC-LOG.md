# Faraday on Macs: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/MAC.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header (from docs/MAC.md)

**Kind:** Spec
**Status:** Partly built; a Mac is needed for the rest

Spec v0.1 · 2026-10-12 · decided by the owner in discussion
(`docs/DECISIONS.md` F13; every decision in §9 taken). Not built. The
owner picked up the Mac work on 2026-10-13 with the Starter, Faraday
Starter: batch A3 is split so its part that needs no Mac goes first
(§8; HANDOFF item 73). F13 supersedes F11's "no macOS
app": Faraday now targets Macs across the stick, the desktop app and the
Starter. F30 (2026-10-14) stops publishing the desktop app, so batch D,
the desktop app on macOS, is dropped from the plan (§5).

- **The Starter** runs on macOS. The desktop app is not published on
  any system since F30, and its macOS build is a developer's (§5).

§5's first lines read:

Not published, and dropped from the plan (owner, 2026-10-14, F30): no
release carries a macOS desktop app, and batch D is not built.

## §0 Rules for the implementing agent (from docs/MAC.md)

- `CLAUDE.md` applies in full: builds and tests on the build machine; no `rm`; no commits; no agents; `just`
  once at the end of a batch.
- Batches in §8's order; one review and one completion report each.
- **§2.2 and §4.3 are requirements, not goals.** If a batch finds one
  cannot be met (m1n1 cannot be built without its USB proxy, U-Boot
  cannot be built without a command line, the preseeded Secure Boot
  starts an unsigned image in QEMU, the kernel takes an initrd from
  outside), it stops and reports. It does not weaken the requirement
  or work around it.
- Every claim about a Mac model in a document a person reads (README,
  Learn pages, the website's text) cites its source: Apple's
  identification pages or Asahi's feature-support pages (§7). Tries on
  hardware find bugs; no published text says which models were tried
  (§9, M6).
- Nothing in the Starter reaches a network itself. The one download
  §4.5 needs goes through macOS's own `/usr/bin/curl` (F13).
- Every pinned outside source (kernel, Asahi patches, m1n1, U-Boot,
  Apple's restore-file parts) has a `docs/deps/` entry: where from,
  commit or version, SHA-256, licence, how many lines Faraday carries,
  and who reviewed the diff.
- Completion report: items done with their tests and whether they pass;
  what was left out and why; docs changed; renders to look at.

## §6 The Starter on macOS, and what changes in STARTER.md (from docs/MAC.md)

Folded into `docs/STARTER.md` on 2026-10-13 (`docs/history/STARTER-LOG.md` lists each). The
app is now Faraday Starter (F16), so its files below are
`Faraday-Starter-X.Y.Z-macOS.zip` and `Faraday Starter.app`, installed
at `~/Applications/Faraday Starter.app`. One change from this list:
after a write the Starter unmounts the stick (`diskutil unmountDisk`)
rather than ejecting it, so the stick it just made stays listed and
Home goes on to the next step (HANDOFF item 73, A3a).

- **§1.1 Release files:** `Faraday-X.Y.Z-macOS.zip` (the Starter,
  universal, `Faraday.app`, ad-hoc signed);
  `faraday-stick-X.Y.Z-arm64-apple.img.xz`;
  `faraday-mac-boot-X.Y.Z.bin`. Every Starter (Windows, Linux, macOS)
  carries the Apple Silicon image and the start-up file template in its
  payload. No Starter carries a desktop app (F30).
- **§2:** a `macos` module in `faraday/shells/starter` beside `linux` and
  `windows`, implementing its `Platform` trait
  (`faraday/shells/starter/src/platform.rs`, committed c3519bc);
  `restart_into_firmware` has no Mac form and returns its refusal, and
  the Apple Silicon setup and removal (§4.5–§4.6) are additions the
  trait gains for Macs only. Disks from `diskutil list -plist` and `diskutil info
  -plist` (external, physical, protocol USB or SD); unmounted with
  `diskutil unmountDisk`; opened through `/usr/libexec/authopen` (the
  system's own password prompt hands back a descriptor for
  `/dev/rdiskN`, read and write), so no part of the Starter runs as root;
  after writing and reading back, `diskutil eject`, because macOS
  mounts every FAT volume it sees and writes `.fseventsd` and other
  folders to it (Faraday never lists, reads or counts them, nor the
  `._` and `.DS_Store` files a Mac writes on a data partition:
  `docs/STARTER.md` §4.4). Facts (§7.1): maker Apple, `sysctl hw.model`,
  `uname -m` and `sysctl.proc_translated`, T2 from the model
  identifier, Startup Security where NVRAM shows it (batch A3 checks
  `AppleSecureBootPolicy`). Built on a Mac, as §5.
- **§3.1 Home:** on a Mac the checklist's start item is the Mac's own:
  *Start this Mac from the stick* (Intel), or *Set up this Mac to start
  Faraday*, then *Start Faraday* (Apple Silicon, §4.5), with *Remove
  Faraday from this Mac* in the rows once set up (§4.6). An M3, M4 or
  M5 Mac: one line, "This Mac cannot start Faraday yet"; Make a stick
  stays, for another computer.
- **§3.2 Make a stick:** a first choice, *For which computer?*: "A PC
  or an Intel Mac" or "A Mac with M1 or M2". Defaults to nothing, as
  the disk (D12's rule), with this computer's kind named on the row
  that fits it. Keep data refuses the other kind (§4.8).
- **§3.4 and §7:** the Intel Mac guide (§3.1–§3.2: Option key; T2
  Startup Security and putting it back; firmware password) and the
  Apple Silicon setup guide (§4.5–§4.7), as Learn pages
  `docs/learn/faraday/boot/mac-intel.md` and `mac-apple-silicon.md`,
  ids `mac-intel` and `mac-apple-silicon` in `learn::BOOT`, published
  at `faradayoffline.app/start/<id>`. *Restart into the BIOS*
  has no Mac form; the Intel page's step is "Shut down, then hold
  Option".
- **§5.2 Install on macOS:** `~/Applications/Faraday.app`; per user,
  no administrator. Uninstall removes exactly that. No desktop app is
  installed (F30).
- **§10:** §10.2's unsigned rule extends to macOS (ad-hoc signature,
  not notarized; Open Anyway). §10.3 adds: the Faraday start-up volume
  and the shrunk macOS container, and the remove action that takes both
  back.
- **§11:** batches A3 and D below join STARTER's list after L.

## §8 Batches: the built and dropped rows (from docs/MAC.md)

| Batch | What | Needs | Order |
|---|---|---|---|
| D | The desktop app on macOS (§5) | dropped (owner, 2026-10-14, F30): not published; a developer with a Mac may build it | not built |
| A3a | The Starter's macOS module (§6): disks, writing through `authopen`, facts, install; type-checked for both Mac targets and tested on Linux against hand-written answers (HANDOFF item 73) | decided | now (owner, 2026-10-13); needs no Mac |
| A3b | Home, the Start page and Make a stick on a Mac (Intel Macs; Apple Silicon says it cannot start Faraday yet); the Mac table in `faraday-starter`; the Intel Mac Learn page (moved here from I) | decided | after A3a; needs no Mac |
