# Faraday on Macs

**Status:** spec v0.1 · 2026-10-12 · decided by the owner in discussion
(`docs/DECISIONS.md` F13; every decision in §9 taken). Not built. The
owner picked up the Mac work on 2026-10-13 with the Starter, Faraday
Starter: batch A3 is split so its part that needs no Mac goes first
(§8; HANDOFF item 73). F13 supersedes F11's "no macOS
app": Faraday now targets Macs across the stick, the desktop app and the
Starter.

What it adds:

- **Intel Macs** start the PC stick (`x86_64-uefi`) as it is, with one
  added keyboard driver and the guides to reach it.
- **Apple Silicon Macs** (M1 and M2 first) start a new stick image,
  `arm64-apple`, through a start-up file the Starter installs once on
  the Mac and Apple's own recovery system seals.
- **The desktop app** and **the Starter** run on macOS.

Asahi Linux's work (m1n1, its U-Boot and kernel trees) is what makes
Apple Silicon possible, and this document uses its names. Faraday
departs from Asahi in one place on purpose: Asahi loads the second half
of its boot code from a FAT partition on the Mac's disk, unchecked, and
says it has no secure boot. Faraday puts every byte that runs before its
kernel inside the sealed start-up file (§4.2).

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: builds and tests
  through `local/remote-run.sh`; no `rm`; no commits; no agents; `just`
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

## 1. Which Macs

| Mac | Stick image | Built-in keyboard | Built-in pointer | Firmware steps |
|---|---|---|---|---|
| 2006 and some 2007 models (32-bit EFI) | none: cannot start a 64-bit EFI file | — | — | — |
| Intel laptops 2008–2014, and MacBook Air and the 2015 MacBook Pro models whose keyboard is on USB | `x86_64-uefi` | yes, generic HID | no (`bcm5974` not built, §3.3) | Option key; firmware password if set |
| Intel laptops 2015–2017 with the SPI keyboard (MacBook 2015–2017, MacBook Pro 2016–2017) | `x86_64-uefi` | yes, `applespi` | yes, `applespi` | Option key; firmware password if set |
| Intel Macs with the T2 chip (2018–2020) | `x86_64-uefi` | no (t2bce not carried, F13): a USB keyboard | no | Startup Security Utility, then back (§3.2) |
| Intel desktops (iMac, Mac mini, Mac Pro) | `x86_64-uefi` | USB keyboard | USB mouse | as the laptops of their year |
| M1, M1 Pro, Max, Ultra | `arm64-apple` | laptops: yes (Asahi patch) | no (keyboard only; a USB mouse can be approved) | one-time setup (§4.5) |
| M2 family | `arm64-apple` | laptops: yes (Asahi patch) | no (its trackpad needs a firmware file copied from macOS, F13) | one-time setup |
| M3 | later: Asahi's support is downstream only, near a first release | | | |
| M4, M5 | not possible yet: m1n1 does not start on them | | | |

The README carries this as an exact list of model identifiers (§7).
A USB keyboard or mouse on any Mac goes through the stick's approval
code, as on a PC (`PLAN.md` §4.6).

## 2. Security

### 2.1 What holds on a Mac

| Property | PC | Intel Mac | Apple Silicon |
|---|---|---|---|
| No network or radios | no drivers | same image | no Wi-Fi or Bluetooth driver built |
| The computer's own disk invisible | no NVMe or SATA driver | same image | Apple's NVMe driver not built (`kernel.forbidden`) |
| Device DMA confined | Intel/AMD IOMMU, strict | same image | Apple's DART IOMMUs, strict |
| Storage rule, disk process, USB authorisation, input trust | `PLAN.md` §4–§5 | same code | same code; the built-in keyboard trusted by its bus (§4.1) |
| What runs before Faraday's kernel | the firmware | the firmware | Apple's boot ROM and iBoot, then the sealed start-up file (§4.2) |
| The stick's kernel checked before it runs | only with Secure Boot on and the owner's key enrolled (`PLAN.md` §8, STARTER §7.7) | never: Mac firmware takes no custom keys, and a T2 Mac must be set to No Security | when the owner chose their own key at setup (§4.4) |
| Changed on the computer | Secure Boot off while used | T2: Startup Security lowered while used | a separate start-up volume, sealed per OS; macOS keeps Full Security |
| Firmware and other processors | out of scope (`PLAN.md` §4.5) | same | same: Apple's processors run Apple-signed firmware loaded before Faraday |

### 2.2 Requirements on every Apple Silicon start

- Nothing that runs before Faraday's kernel is read from the Mac's own
  disk except Apple's own signed files and the sealed start-up file.
  No FAT partition, no settings file, no variables file, no second
  stage.
- Nothing reaches the kernel but the stick's `BOOTAA64.EFI`: no initrd
  from outside, no command line from outside, no device tree from the
  stick.
- No way into the boot code from the keyboard or a cable: no
  interrupt key, no command line, no USB debug mode.

### 2.3 Setup trusts nothing macOS wrote

The start-up file is written by the Starter in macOS, and sealed in
the Mac's recovery system, which Apple signs and which macOS cannot
change. A compromised macOS could swap the file between the two. So
the person types the sealing commands in Recovery's Terminal
themselves, and before sealing compares the file's SHA-256, computed
there by Recovery's own `shasum`, with a value from outside that Mac
(§4.5 step 6). No script on a volume macOS wrote takes part in sealing
(F13).

## 3. Intel Macs

### 3.1 Starting

Hold Option (Alt) at power-on; the stick shows as "EFI Boot". The
stick's GPT layout and its one file, `EFI/BOOT/BOOTX64.EFI`, are what
Mac firmware starts; no `bless`, HFS+ volume or hybrid MBR. Older
firmware is pickier about sticks: a 2008–2012 Mac is tried before
batch I is done, and what it finds goes into the code or the Learn
page, not into a list of tried models (§7).

The 6.6 kernel's EFI stub still reads the firmware's framebuffer on
Macs whose firmware offers UGA instead of GOP (2006–2010). 6.14 removed
that path; the kernel update that crosses 6.14 checks the Macs in
`drivers/firmware/efi/sysfb_efi.c`'s table and says what it found.

On the dual-GPU MacBook Pros, the stub's `apple_set_os` runs by itself
(firmware vendor "Apple", models listed in `x86-stub.c`). Batch I
records which GPU drives the panel on one if one is tried.

### 3.2 Firmware

- **Firmware password** (Intel Macs before T2): it blocks the Option
  key. The Learn page says to enter it or remove it in Recovery.
- **T2 Macs** (MacBook Pro and MacBook Air 2018–2020, iMac 27-inch
  2020, iMac Pro, Mac mini 2018, Mac Pro 2019; Apple's list): in macOS
  Recovery (Command-R), Utilities → Startup Security Utility, with the
  Mac's administrator password: Secure Boot **No Security**, and
  **Allow booting from external or removable media**. Medium Security
  is not enough: it accepts only Apple's and Microsoft's Windows
  signatures. Afterwards, the same utility: **Full Security** and
  **Disallow booting from external media**.
- The stick's power-off reminder (STARTER §7.5) on a Mac: `sys_vendor`
  "Apple Inc." and a T2 model identifier (`product_name`) give "Startup
  Security is lowered. Set it back in macOS Recovery: Startup Security
  Utility." Any other Intel Mac gets no Secure Boot line: its firmware
  has none, and the `boot_params` byte saying "off" is not a setting
  anyone changed.

### 3.3 The PC kernel's changes

In `faraday/image/overlay/boards/x86_64-uefi/`:

- `linux.fragment` and `kernel.required`: `KEYBOARD_APPLESPI` with what
  it needs (`SPI`, `SPI_PXA2XX`, `SPI_PXA2XX_PCI` through `PCI` and
  `COMMON_CLK`, `LEDS_CLASS`, `CRC16`; `MFD_INTEL_LPSS_PCI` is built
  already). It attaches only to the device the firmware describes
  (ACPI `APP000D`); nothing on USB reaches it.
- `USB_OHCI_HCD` and `USB_OHCI_HCD_PCI`: the 2008–2010 Macs with
  NVIDIA chipsets put full-speed USB, the built-in keyboard included,
  on OHCI. A host controller driver, bound by PCI class, reachable by
  no USB device's IDs.
- `USB_UHCI_HCD` too (found 2026-10-13, orchestrator, while writing
  the Intel Mac Learn page): the stick builds only xHCI and EHCI, and
  on the Intel chipsets of 2006 to 2010 a full- or low-speed device on
  a root port (the built-in keyboard, a USB keyboard plugged straight
  in) is handed to EHCI's UHCI companion controller, so Faraday sees
  no keyboard there. From 2011 the chipset's own hub matches rates and
  EHCI alone serves it. Bound by PCI class, as OHCI. Until batch I
  (which waits for the kernel move), the Learn page says a keyboard on
  a pre-2011 Mac may not be found and a USB 2.0 hub between works.
- `kernel.forbidden`: `MOUSE_BCM5974`, with `HID_APPLE`'s reason: it
  binds by Apple's USB vendor and product IDs, which any device can
  claim. Pre-2015 trackpads stay off (F13).
- `applespi.iso_layout` and `applespi.fnmode`: whether the stick's
  keyboard-layout setting can set them at run time
  (`/sys/module/applespi/parameters/`) or only on the built-in command
  line is checked; an ISO keyboard's swapped key (left of Z, below Esc)
  is tried on an ISO Mac; until then the README says ISO keyboards are
  expected to swap that key.

### 3.4 Input trust

`faraday/shells/stick/src/evdev.rs`'s `trusted_by_bus` trusts i8042,
I2C, host and hard-wired USB. It adds **SPI** (`BUS_SPI`, 0x1C), trusted
when no ancestor of the input device in sysfs is a USB device: an SPI
keyboard is wired inside the case and nothing plugged in reaches that
bus. `applespi` (Intel) and Asahi's SPI HID (M1) report `BUS_SPI`.
What M2's DockChannel HID reports is read on a Mac before its rule is
written (§4.1). Tests: a stand-in sysfs, as `evdev.rs`'s own tests use,
with an SPI keyboard trusted and a USB device reporting `BUS_SPI`
through a USB ancestor not.

## 4. Apple Silicon

### 4.1 The stick image

A new board, `faraday/image/overlay/boards/arm64-apple/`, beside
`x86_64-uefi`, with the same parts (`linux.fragment`,
`kernel.required`, `kernel.forbidden`, `cmdline.txt`, `genimage.cfg`,
`board.conf`, `buildroot.fragment`).

- **Layout:** as the PC stick: GPT, OSKBOOT (FAT32, ESP type) holding
  only `EFI/BOOT/BOOTAA64.EFI`, then OSKDATA. Same partition names and
  sizes, so `faraday-grant`, `faraday-disk`, `faraday-boot` and the
  Starter's keep data work unchanged. The image is
  `faraday-stick-X.Y.Z-arm64-apple.img.xz`.
- **Kernel:** mainline, pinned to one release that has what Apple
  Silicon needs (USB on the Type-C ports from 7.0, the SMC and its
  power-off from 6.18, the SPI controller from 6.15), plus Asahi's two
  keyboard drivers as patches pinned to one Asahi commit: SPI HID (M1
  laptops) and DockChannel HID (M2 laptops, needing no firmware file).
  Nothing else from Asahi's tree. `EFI_STUB`, built-in command line
  with `CMDLINE_FORCE`, initramfs built in, `noinitrd` on the command
  line. 16K pages, as Asahi runs.
- **Required:** `APPLE_AIC`, `APPLE_DART` (strict), `APPLE_RTKIT`,
  `APPLE_MAILBOX`, `MFD_MACSMC` and `POWER_RESET_MACSMC` (power off at
  exit; without them `poweroff -f` may leave RAM powered),
  `SPI_APPLE`, `USB_DWC3_APPLE`, `PHY_APPLE_ATC`, `TYPEC_TPS6598X`,
  `USB_STORAGE`, `HID_GENERIC`, `USB_HID`, the two keyboard drivers,
  `FB_SIMPLE` (the framebuffer iBoot and m1n1 leave on, as
  `simple-framebuffer`).
- **Forbidden:** as the PC's list where it applies, and `NVME_APPLE`,
  `BRCMFMAC`, every Bluetooth driver, `DRM` (as on every Faraday
  image), `APPLE_DCP` and the GPU, the camera ISP, `EFIVAR_FS`, the
  vendor HID drivers.
- **The app:** `aarch64-unknown-linux-musl`, static, built as the PC
  stick's is. `osk-crypto`'s `pinned.rs` takes the page size from
  `sysconf(_SC_PAGESIZE)` on Linux where it is a constant today, so a
  16K kernel pins whole pages. The stick shell's `cfg!(target_arch =
  "arm")` that means "the Pi" stays true only for 32-bit ARM; aarch64
  says "stick".
- **No `boot_params`:** the power-off reminder (STARTER §7.5) has nothing
  to say on Apple Silicon: macOS was never lowered.
- **Input:** M1's keyboard as §3.4. M2's: its `bustype` and sysfs path
  are read on an M2 Mac; it is trusted only if, like SPI, nothing
  plugged in can produce it. The trackpad is not built on either.
- **Ports:** the Type-C ports work; USB-A ports on desktops, the
  iMac's two ports furthest from the power cable and the M1 Max Mac
  Studio's front ports do not, and a hub with an empty SD slot can
  reset the Mac in U-Boot. The Learn page says so.

### 4.2 The start-up file

One file, built per release by `just faraday-mac-boot`, and the only
code of Faraday's that runs before its kernel:

```
m1n1 (stage 1, built without chainloading)
+ the device trees for every supported Mac (from the pinned kernel tree)
+ U-Boot (gzip, its Secure Boot variables preseeded, §4.4)
```

m1n1 takes appended payloads by design; U-Boot's own Apple
documentation installs exactly this concatenation as the custom kernel.
The recovery system records its hash in the Mac's LocalPolicy for that
volume (the `coih` field), so a changed byte fails to start, and
changing it takes the owner's credentials in recovery with the power
button held.

- `faraday/faraday-macboot`: a crate with no I/O that builds the file
  from the release's template and a key choice (§4.4), reads a file
  back (which key it trusts, which release), and gives its SHA-256. The
  Starter and Faraday both use it, so the hash a Faraday shows for an
  owner's key is the hash the Starter writes.
- The release publishes the no-check file,
  `faraday-mac-boot-X.Y.Z.bin`, and lists it in `SHA256SUMS`. Every
  image and the Starter carry the template.
- **The device trees live in the start-up file**, not on the stick
  (§2.2). A kernel update that needs newer device trees needs a new
  start-up file: each release states the start-up file version it
  needs (`faraday-macboot`'s version number, in the file and in the
  image), the stick says on start when the Mac's file is older than it
  needs, and the Starter offers to redo the setup. A release that keeps the
  version needs nothing on the Mac.

### 4.3 Requirements on the start-up file

Each is checked by the build (configuration lines asserted, as
`check-kernel-config.sh` does for kernels) and recorded in
`docs/deps/`:

- **m1n1:** stage 1 with its payloads; no chainload; no `chainload=`
  or ESP variable read. When it cannot start its payload, it powers the
  Mac off: it never falls into its USB proxy mode, which takes commands
  from another computer over the cable. Batch A2 establishes how m1n1
  behaves on failure at the pinned commit before anything else; if the
  proxy cannot be left out, it stops (§0).
- **U-Boot:** no command line (`CMDLINE` off) or, if bootstd needs it,
  no way to reach it (`BOOTDELAY` -2 and no `AUTOBOOT_KEYED`
  bypass); no environment storage (`ENV_IS_NOWHERE`); no NVMe
  (`NVME_APPLE` off); UEFI the only boot method, from USB mass storage
  only (no extlinux, no boot scripts, no network); no initrd loading
  (`EFI_LOAD_FILE2_INITRD` off); `EFI_VARIABLE_NO_STORE`, so no
  variables file is read or written.
- **The kernel** (§4.1): `CMDLINE_FORCE`, `noinitrd`; the EFI stub
  drops any initrd the device tree names.
- **The license:** m1n1 is MIT, U-Boot GPL-2.0; the release notes and
  `docs/deps/` give the source of both at the pinned commits, as the
  kernel's.

A QEMU test, `just faraday-mac-boot-check`, builds U-Boot's
`qemu_arm64` with the same configuration fragment and the same preseed
mechanism and shows: an image signed with the seeded db key starts; the
same unsigned, or altered by one byte, does not; an initrd beside it on
the stick is not loaded; a key press at start reaches no command line.
It is the arm64 twin of `faraday/tools/sb-ovmf-check.py`.

### 4.4 Which stick the file starts: two modes

Chosen by the person at setup (owner, F13). There is no project key:
nobody holds a key that would start Faraday on every set-up Mac.

- **No check.** U-Boot is built without Secure Boot and starts any
  `BOOTAA64.EFI` on a USB stick. The file is the published one,
  identical for everyone on a release. What this leaves open is the
  same as a PC with Secure Boot off: a computer the stick was plugged
  into could replace its Faraday.
- **My own key.** U-Boot with `EFI_SECURE_BOOT` and its variables
  preseeded: the person's PK, KEK and db (the `.esl` files Faraday's
  Secure Boot writes, `PLAN.md` §8), and an empty dbx. Only a
  `BOOTAA64.EFI` signed with their db key starts; every release is
  signed by them through the spare stick, as on a PC (`PLAN.md` §8,
  §5.5). `faraday-macboot` writes the variables into a reserved,
  fixed-size region of the U-Boot payload in U-Boot's own preseed
  format; no compiler runs at setup. Changing or revoking the key is a
  new setup.

Faraday's Secure Boot screen (on any Faraday, PC or Mac) gains, for a
key in an open vault: "Mac start-up file for this key", its SHA-256,
from the template the image carries. That is the value the person
compares in recovery (§4.5 step 6).

The usual path to my own key: set up with no check, start Faraday on
the Mac, make the keys in a vault and sign the release, then redo the
setup with the key. That second setup is checked against the value
the first one's Faraday shows.

### 4.5 Setup, from the Starter

"Set up this Mac to start Faraday" (§6) offers it on an M1 or M2 Mac
(`sysctl hw.model`, and `sysctl.proc_translated` so a Rosetta copy is
not fooled), on macOS 13.5 or later (Asahi's requirement for its stub),
with 3 GB free. Each step is a screen; nothing happens before the
person presses its button.

1. **What it does** (Record): makes a 2.5 GB start-up volume named
   Faraday beside macOS, shrinking macOS's container to make room;
   downloads parts of an Apple restore file from Apple (the size, and
   `updates.cdn-apple.com`); afterwards, one visit to recovery. One line:
   "Back up this Mac first." The Learn page says why shrinking a
   container can fail (Time Machine local snapshots) and what to do.
2. **Which stick it starts** (Choose): *Any Faraday stick* (no check),
   or *Only Faraday signed with my key*, which takes the `PK.esl`,
   `KEK.esl` and `db.esl` dropped on the window, and shows the db
   certificate's fingerprint.
3. **Download** (Progress): `/usr/bin/curl --proto =https --tlsv1.2`
   for each pinned URL, byte ranges as asahi-installer reads them;
   every extracted file checked against the SHA-256 in the Starter's
   manifest, and refused otherwise. The URLs, ranges and sums come from
   asahi-installer at its pinned commit, recorded in `docs/deps/`. The
   files are Apple's and Apple-signed; iBoot checks them again at every
   start.
4. **Make the volume** (Progress): `diskutil apfs resizeContainer`,
   `addContainer` / `addVolume` with Preboot, Recovery, Data and System
   roles as asahi-installer makes them, the downloaded parts copied in,
   and the start-up file written. Through the Authorization Services
   prompt (administrator password); nothing of the Starter's runs as root
   beyond that one command each.
5. **Recovery** (Record): the instructions, and a QR code of them for
   a phone: shut down; hold the power button until Options appears;
   choose Faraday; in its recovery, Utilities → Terminal.
6. **In recovery, typed by the person** (owner, F13): `shasum -a 256`
   on the start-up file's path, compared with the value on the release
   page on their phone (no check) or with Faraday's Secure Boot screen
   (my own key) — the Starter shows the value too, but the guide says the
   phone's or the Faraday's is the one to trust; then the security
   policy and `kmutil configure-boot` commands asahi-installer's
   `step2.sh` runs at the pinned commit, with the owner's credentials
   when asked. Not `bless --setBoot`: macOS stays the Mac's default, and
   Faraday starts only when chosen. The exact commands are in the Learn
   page and on the QR code; batch A3 copies them from `step2.sh` and
   tries them on a Mac before the page is published.
7. **Done:** to start Faraday, plug in the stick, hold the power button,
   choose Faraday.

The Starter's Home (STARTER §3.1) on an Apple Silicon Mac shows "Faraday
start-up volume: none", or "installed: any Faraday stick" or "installed:
your key, fingerprint …", read from the volume's file through
`faraday-macboot`; the line says it is what the volume holds, which is
what was sealed only if the setup's check passed.

### 4.5a Explained, and the person's to keep or remove (owner, 2026-10-12)

Every screen of §4.5, and Home's line about the start-up volume, has a
small link, **About the Faraday start-up volume**, to a Learn page that
says in plain words: what the volume is (a small start-up volume beside
macOS, holding Apple's own start-up code and one sealed file that
starts a Faraday stick); why an Apple Silicon Mac needs it (it starts
only what its security policy allows, and the policy is set in
recovery, by the person); what it can and cannot do (it starts a stick
only when the person holds the power button and chooses Faraday;
macOS stays the Mac's default; it reads nothing of macOS's data); what
setup automates and the one part it cannot (the recovery commands,
§4.5 step 6, which the person types by design); and that keeping it
or removing it is the person's choice: kept, Faraday starts next time
with no setup; removed (§4.6), nothing of Faraday stays on the Mac
except what only recovery can see, for anyone who does not want the
Mac to show that Faraday was used. Home offers **Remove Faraday from
this Mac** whenever the volume is there. On an Intel Mac there is no
volume or entry to explain: the stick starts from the Option-key menu,
and the link says so. As in STARTER §3.1c for Linux's boot entry.

### 4.6 Removing it

"Remove Faraday from this Mac" (owner, 2026-10-12, for privacy):
`diskutil apfs deleteContainer` on the Faraday container, then macOS's
container grown back (`resizeContainer … 0`). It needs the
administrator password, not recovery. The Options screen no longer
shows Faraday. What may remain is the volume's entry in the Mac's
security policy storage, which only recovery can see; batch A3 checks
and the Learn page says what it found. Setting up again later repeats
§4.5 in full, recovery included.

### 4.7 Each start

Plug the stick into a working Type-C port, hold the power button,
choose Faraday. The Mac starts Apple's code, then the sealed file, then
the stick's Faraday, and from there it is Faraday as on a PC: the stick
is pulled out before a vault is opened (`PLAN.md` §5). Power off at
exit through the SMC.

### 4.8 Upgrades

The stick's own upgrade (`PLAN.md` §5.5) works unchanged: it copies the
running Faraday's boot partition, and only to a stick of the same kind,
since it requires the running kernel's release string. The Starter's
*Replace Faraday, keep data* refuses to write one kind's boot partition
over the other's: it reads which EFI file the stick's OSKBOOT holds
(`BOOTX64.EFI` or `BOOTAA64.EFI`) through `faraday-fat`.

With my own key, every release's `BOOTAA64.EFI` is signed by the owner
before the vault stick starts it. `faraday-sb` already signs any PE32+
whatever its machine type; the first-signing build record (`PLAN.md`
§8) gains the arm64 image.

## 5. The desktop app on macOS

A testing app with test seeds only, as on Linux and Windows.

- Built on a Mac (Apple's SDK licence allows its use only there) by
  `just faraday-mac-desktop`, as a universal binary (x86_64 and
  arm64), macOS 11 or later, in `Faraday Desktop.app` with
  `NSCameraUsageDescription`. Ad-hoc signed, not notarized (owner, F13,
  for the reason F11 gives for Windows). Released as
  `faraday-desktop-X.Y.Z-macos.zip`.
- Changes: the camera through `opensigner-avfoundation`, which the tree
  carries; `memory_available_mib` from `sysctl hw.memsize` where it
  reads `/proc/meminfo`; folders opened with `open` where `xdg-open` is
  used; `HOME` as on Linux (`~/faraday-sticks`, `~/Downloads`). `faraday-grant`
  and the other stick-only crates are not built for macOS (they call
  Linux-only interfaces); the Mac recipes build the packages they need
  by name.
- `pin-pages` uses `mlock` as on Linux; the 16K page on Apple Silicon is
  handled already.
- First open: macOS refuses an unnotarized app; System Settings →
  Privacy & Security → Open Anyway. The README and the Starter's About say
  so, and how to check the zip against `SHA256SUMS` and its signature.

## 6. The Starter on macOS, and what changes in STARTER.md

Folded into `docs/STARTER.md` on 2026-10-13 (its §13 lists each). The
app is now Faraday Starter (F16), so its files below are
`Faraday-Starter-X.Y.Z-macOS.zip` and `Faraday Starter.app`, installed
at `~/Applications/Faraday Starter.app`. One change from this list:
after a write the Starter unmounts the stick (`diskutil unmountDisk`)
rather than ejecting it, so the stick it just made stays listed and
Home goes on to the next step (HANDOFF item 73, A3a).

- **§1.1 Release files:** `Faraday-X.Y.Z-macOS.zip` (the Starter,
  universal, `Faraday.app`, ad-hoc signed);
  `faraday-desktop-X.Y.Z-macos.zip`; `faraday-stick-X.Y.Z-arm64-apple.img.xz`;
  `faraday-mac-boot-X.Y.Z.bin`. Every Starter (Windows, Linux, macOS)
  carries the Apple Silicon image and the start-up file template in its
  payload; the macOS Starter carries the desktop app.
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
  folders to it. Facts (§7.1): maker Apple, `sysctl hw.model`,
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
- **§5.2 Install on macOS:** `~/Applications/Faraday.app` and, if
  ticked, `~/Applications/Faraday Desktop.app`; per user, no
  administrator. Uninstall removes exactly those and the cache
  (`~/Library/Caches/faraday/X.Y.Z/`).
- **§10:** §10.2's unsigned rule extends to macOS (ad-hoc signature,
  not notarized; Open Anyway). §10.3 adds: the Faraday start-up volume
  and the shrunk macOS container, and the remove action that takes both
  back.
- **§11:** batches A3 and D below join STARTER's list after L.

## 7. Supported hardware, published

The README gains **"Which Macs"**, the list a person checks before
buying or trying, and the website's download page links to it.

- One row per Mac model identifier (`MacBookPro14,1`, `Mac14,2`, …),
  with its marketing name and year, grouped as §1's table: image,
  built-in keyboard, built-in pointer, camera, what to set in the
  firmware, and status.
- **Status** is one of: *Expected* (its chip, keyboard and firmware are
  ones Faraday supports, from Apple's and Asahi's documentation, cited)
  or *Not supported* (with the reason). There is no *Tried* (§7.1).
- Every identifier comes from Apple's identification pages ("Identify
  your MacBook Pro model" and the others) and, for Apple Silicon,
  Asahi's feature-support pages, cited under the table with the date
  read. 32-bit EFI models are listed as not supported with how to check
  (`ioreg -l -p IODeviceTree | grep firmware-abi`).
- The same list is what the Starter reads to say "This Mac cannot start
  Faraday yet": one table in the code (`faraday-starter`), with a test that
  the README's rows and the table agree.

### 7.1 No "Tried" column

A "Tried" mark would name the Macs Faraday was run on, and those are
the owner's own, which the owner's rules treat as personal information
(owner, M6). So the list never says which models were tried: every
status comes from cited documentation. Tries on hardware still happen
and fix what they find; a model found not to work despite its
documentation becomes *Not supported* with the reason, which names a
fault, not whose Mac showed it. The same holds for the Learn pages:
the Mac pages end with their documentation sources and the date read,
not a model tried on.

## 8. Batches

| Batch | What | Needs | Order |
|---|---|---|---|
| I | Intel Macs: the PC kernel's changes (§3.3), SPI input trust (§3.4), the Mac power-off line (§3.2) with the Intel Mac guide's QR code, the README's Intel rows from A3b's table, with the test that they agree (§7) | decided | independent of the Starter batches; the README rows after A3b |
| D | The desktop app on macOS (§5), its recipe and release file | a Mac to build on (§9, M7) | waits for a Mac |
| A1 | The `arm64-apple` stick image (§4.1), with the Asahi keyboard patches and their `docs/deps` review | decided | after I |
| A2 | The start-up file (§4.2–§4.4): m1n1 and U-Boot pinned and built under §4.3, `faraday-macboot`, the QEMU check, the published no-check file | decided | after A1 |
| A3a | The Starter's macOS module (§6): disks, writing through `authopen`, facts, install; type-checked for both Mac targets and tested on Linux against hand-written answers (HANDOFF item 73) | decided | now (owner, 2026-10-13); needs no Mac |
| A3b | Home, the Start page and Make a stick on a Mac (Intel Macs; Apple Silicon says it cannot start Faraday yet); the Mac table in `faraday-starter`; the Intel Mac Learn page (moved here from I) | decided | after A3a; needs no Mac |
| A3c | The Starter on a Mac: the bundle and its recipe, every answer recorded on a Mac, the device checks; then the Apple Silicon setup and removal (§4.5–§4.6), tried on an M1 or M2 Mac with the owner; the Apple Silicon README rows | a Mac to build on and an M1 or M2 Mac to try (M7) | after A3b, and A2 for Apple Silicon; waits for a Mac |
| A4 | Faraday's Secure Boot screen showing the start-up file's hash for a key (§4.4); the arm64 build record | decided | after A2 |

Tests state what a person sees or can do: the SPI keyboard trusted and
a USB device faking the bus not (§3.4); the Mac power-off line for a T2
and a non-T2 model string; `faraday-macboot` building the same file for
the same key every time, refusing `.esl` files that do not parse, and
reading back the key it was given; the QEMU check (§4.3); the Starter's
*For which computer?* and keep data refusing the other kind; the Mac's
facts mapped to the right Home line for an Intel, T2, M1, M2 and M4
model string; the README table and the code's table agreeing.

## 9. Decisions

Taken by the owner on 2026-10-12 in discussion, except where marked.

- **M1 Macs supported:** Intel Macs with 64-bit EFI on the PC stick;
  M1 and M2 on a new image; M3 when Asahi's support is released; M4
  and M5 when m1n1 starts on them.
- **M2 The key the start-up file trusts:** chosen at setup: none (any
  Faraday stick) or the owner's own. No project key.
- **M3 Apple's files:** downloaded by the Starter through macOS's own curl
  from Apple, pinned by SHA-256; the Starter carries no network code.
- **M4 Out-of-tree code:** Asahi's two keyboard drivers, in the Apple
  Silicon kernel only. No t2bce; no trackpad firmware. On the PC
  kernel, `applespi` and OHCI; `bcm5974` forbidden.
- **M5 Sealing:** typed by the person in Recovery's Terminal after
  checking the file's SHA-256 against a value from outside the Mac.
- **M6 No "Tried" in anything published:** the Mac list's status is
  Expected or Not supported, from cited documentation; no published
  text names a model tried on (§7.1).
- **M7 No Mac yet:** batches I, A1, A2 and A4 go ahead without one; D
  and A3 wait until a Mac is available to build on (Apple's SDK
  licence allows it only on a Mac) and an M1 or M2 Mac to try.
- **M8 Signing on macOS:** ad-hoc, not notarized; Open Anyway.
- **M9 Removing the start-up volume:** offered by the Starter, for privacy
  (owner).
