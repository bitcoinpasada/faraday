# Decisions

Faraday's own decisions, numbered F1 onward, newest last.
`docs/PLANNING.md` is OpenSigner's and is kept identical to upstream; its
§15 and §16 are OpenSigner's decisions. F1 to F4 were first written as
PLANNING §16.141 to §16.144 and moved here on 2026-10-10, when upstream
took §16.141 for its own entry.

### F1 The boot import, and one line of fact on its sheet (2026-10-08)

**Why.** At boot the app read the boot stick's vault files into the
Inbox, and pulling the stick opened the first locked vault's passphrase
prompt. Anything else on the stick needed a stick visit before the pull,
and a PNG's QR codes were read one picture at a time behind a **Read the
QR codes** button, with its checkbox greyed out. The owner's ask: at
boot, copy everything on the stick into memory, say so and ask for the
stick to be removed; then offer the vaults to unlock; then list the
wallets the whole set holds, with their keys and whether they can sign,
and the files, to choose from; one button imports the choice and wipes
the rest.

**What it does** (`faraday-core/src/boot_import.rs`, its sheet in
`boot_import_screen.rs`; README, "Starting a session: the boot import";
`PLAN.md` §5.4). The first look at the boot stick in a power-on (the
condition the settings file is read under) copies every file: vaults
into the Inbox, where the vault screens unlock them, the rest into a
holding area beside it, a PNG as what its codes hold. `Sheet::Import`
over Home says what was copied and asks for the stick to be removed;
once it is out, it lists the vaults with **Unlock**, the wallets found
in the files and the open vaults once each by descriptor with "Can
sign", "k of n keys here · m more needed" or "Watch-only" and the files
that carry them, the keys no wallet uses, and every file for the Inbox.
**Import** loads through the Inbox's loader and the vaults' own, moves
the chosen files into the Inbox, and drops the rest, wiped as it drops.
Pulling the boot stick no longer opens a passphrase prompt; a lock drops
what was not imported; a later insertion is an ordinary visit. On a
visit, a PNG is ticked like any file and read on Import, and the **Read
the QR codes** button is gone.

**The owner's decision: one line of fact on a working sheet.** The sheet
carries, above its buttons, "Files not chosen are wiped from memory. To
bring one in later, insert the stick again." It says what the button
does and what follows from it, in plain statement, which PLANNING
§16.46 allows anywhere; it is not an explanation of why, which PLANNING
§16.37 keeps in Learn.

### F2 Faraday upgrades another Faraday stick, and never signs inside the upgrade (2026-10-08)

**Why.** Upgrading a stick meant writing the new image over the whole
stick, which empties its data partition, or replacing `BOOTX64.EFI` on
another computer, which puts the vaults on that computer: it can copy
them to guess at the passphrase offline, delete them, and plant files. The
owner's ask: Faraday copies itself onto a stick that already holds
Faraday and a data partition, and the data partition is not touched.

**What it does** (`PLAN.md` §5.5; built for the PC
2026-10-09, the Pi to follow). Settings →
**Upgrade a Faraday stick**, in the clean state: the stick Faraday
started from is read, then its boot partition is written raw over the
target's, read back and compared. The source is accepted only if it
carries the running kernel's release string, into which the build puts
the Faraday version and commit; every stick has the same partition
UUIDs, so unplugging and replugging the boot stick changes nothing. A
new process, `faraday-boot` (`ofboot`, uid 203), does the copy and reads
no FAT; the app can only ask it to copy, never hand it bytes.
`faraday-grant` hands it partitions named `OSKBOOT` only while the app
publishes an upgrade marker.

**The owner's decisions.**

- *The exception to "no boot partition is handed out" is small.* That
  rule (`PLAN.md` §4.3) is there so a Faraday compromised while
  running cannot plant itself on a stick. Here the app never supplies
  the bytes; a source stick that was tampered with already controls the
  machine it booted; and without Secure Boot any computer the stick is
  plugged into can rewrite the boot partition anyway.
- *No signing inside the upgrade.* Signing there would have the app hand
  `faraday-boot` a file of its own making, checked by Authenticode
  digest against the source, and `faraday-boot` would parse PE and write
  FAT. Instead an owner with Secure Boot signs each release once through
  a spare stick (sign on the vault stick, copy the signed file onto the
  spare's boot partition on a computer, boot the spare, upgrade the
  vault stick from it), so the vault stick never meets an online
  computer. A Learn page and the README take the person through it.
- *A boot-partition-only flash for development* (`just
  faraday-stick-boot`), so a test stick keeps its vaults across builds.
  It is not offered to users: it puts the stick on the build computer.

**As built** (2026-10-09). The release string is
`<kernel>-faraday-<version>+<commit>` (`.dirty` after a tree that
differs from the commit). The copier also lists the boot partitions it
has with the release string found on each, for the screen's versions,
and forgets the source when the flow ends; its pipe carries no file and
has a 4 KiB frame limit. A target that already carries the running
release is not written; a newer one is warned about and can be. The
Learn page is Faraday's own, outside OpenSigner's set
(`docs/learn/faraday/`). The QEMU test drives the copier from the dev
console, since no input in QEMU reaches the app (`PLAN.md`
§4.6).

### F3 A multi-choice list beside Choice, for the backup's plan (2026-10-09)

**Why.** Faraday's backup became a plan, then a checklist of only what
the plan needs (`docs/WALLETS.md` §5, the owner's proposal A).
The plan's questions each take more than one answer: the seeds on paper
and in a vault, the wallet description on a sheet and in software,
several programs at once. Choice (DESIGN §4.2) checks one row; a row of
chips for more is ruled out on every class.

**Decision** (owner: ok). DESIGN §4.2 gains **Multi choice**: the same
full-width rows as Choice, a checkbox at each row's start instead of the
accent check, any number ticked, Continue under the list, a row that may
not be ticked dimmed and inert. Nothing has to be ticked, so Continue is
never dimmed for it. Several lists may share a page, each under its own
label, where they answer one question (the software and its form). On a
small panel the plan asks one question per page, as every step flow
pages there. It is built in Faraday's core (`Ui::multi_list`); `osk-ui`
gets a component when an OpenSigner screen needs one.

### F4 On the Pi, "SD card" where the app says "stick" (2026-10-09)

**Why.** The Pi has no USB storage (PLAN.md §3), so every medium it sees
is an SD card, yet the app called it a stick on every screen: Home's
status line, Stick visit, the Pull and Lock sheets, the vault screens,
the boot import, the family guide.

**Decision** (owner: go-ahead). Faraday's core keeps
`Medium { Stick, SdCard }` (`faraday-core/src/medium.rs`), told to it by
the shell at start; nothing in the core looks at the architecture, so the
snapshot shell renders either wording at any size and the desktop app
stays "stick". The stick shell says SD card in its Pi build (armv7) or
when the image recorded the exchange partition on `/dev/mmcblk0`
(`/etc/opensigner/exchange`); the snapshot shell takes `--sd-card`.
Every string naming the medium takes its words from `Medium`: the noun
("stick" / "SD card"), the plural, the article form ("a stick" / "an SD
card"), the sentence-start forms ("Stick" / "SD card", "A stick" / "An SD
card"), the full name where the kind matters ("USB stick"), the visit's
name ("Stick visit" / "SD card visit") and the boot medium's ("Boot
stick" / "Boot SD card"); no string builds "a {noun}" by hand. The boot
medium's name moved out of `faraday-storage` into the app, which names a
`boot` medium itself whatever its volume label. Spelling: "SD card",
capital SD and lowercase card, "SD cards", never "card" alone; so the
PC's Files foot now reads "Waiting for a stick" where it read "Waiting
for a stick or card". Code names and file names are unchanged
(`StickInfo`, `stick_settings.rs`, `faraday-settings.txt`).

The medium's glyph follows it: `Icon::Drive` on the PC, `Icon::SdCard` on
the Pi, Font Awesome 6 Free Solid `sd-card` (U+F7C2) added back to the
icon face after PLANNING §16.91 removed `MicroSd` as unused. `osk-ui` is
shared with upstream OpenSigner: the new variant, its place in `Icon::ALL`, its
code point and name in `core/osk-ui/src/widgets/icon.rs`, the
`tools/fontbake` entry and the rebaked `core/osk-ui/assets/icon.outl` are
an upstream change, offered to OpenSigner with this one.

Not settled here: the family guide's "Start from the SD card" card still
walks through a PC's boot menu, which a Pi does not have; its steps for a
Pi are the owner's to write. The online desktop app's mainnet warning
still says "booted from a stick", since it names the PC image to use, not
a medium the app sees.

### F5 Release, test release and dev builds, each named in the release string (2026-10-10)

**Why.** Every build since 0.1.0 carried `-faraday-0.1.0+<commit>`, so a
build from `main` claimed the published release's number, and a dev
image and a release image of one commit carried the same string: the
upgrade read them as the same build. Builds made again with different
uncommitted changes also read as the same build and were refused. The
owner rebuilds often to test, the upgrade among them, and wants each
kind of build named for what it is.

**What it is.** The part of the kernel release string after
`-faraday-`, which the app also shows in About:

| Build | String | Shown as |
|---|---|---|
| Release | `0.2.0` | 0.2.0 |
| Test release (the default) | `0.1.0+ef24784b48d8.test` | 0.1.0 test release (ef24784b48d8) |
| Dev (console, serial login) | `0.1.0+ef24784b48d8.dev` | 0.1.0 dev (ef24784b48d8) |

A tree that differs from its commit adds `.dirty-` and eight hex digits
of a hash of the differences before the kind
(`0.1.0+ef24784b48d8.dirty-3fa9c1d2.test`, shown as "0.1.0 test release
(ef24784b48d8, changes 3fa9c1d2)"), so two builds with different
changes differ and the same changes give the same string. The commit and
the kind follow `+`, which in semantic versioning names a build of a
version, not an earlier version.

**The owner's decisions.**

- *A published release carries its number alone.* One version is
  published once; its commit is in the signed tag and in `SHA256SUMS`.
- *Test release is the default; a release is asked for* (`just
  release=1 …`), and a release build refuses a dev image, a version
  override and a tree that differs from its commit. A release built by
  mistake without the flag reads as a test release, which shows before
  it is published.
- *A test or dev build may claim another version number*
  (`version=0.9.0`), so the upgrade's warning about a newer Faraday can
  be tried between test builds.
- *The upgrade warns before writing a dev build over a stick that does
  not hold one*: the target gains a serial console and login. It can
  still be written.

The upgrade's other rules are F2's: the same string is refused, a
higher version number is warned about, anything else is written.
Sticks made between F2 and this entry (`0.1.0+<commit>`, no kind) show
as before, "0.1.0 (<commit>)". This resolves the audit's finding that a
dev image and a release image could not be told apart.
