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

### F6 The backup before the spend (2026-10-10)

**Why.** A key made in Faraday exists nowhere else until its backup is
done, and a passphrase added here exists only in memory. Back up's
checklist could be passed item by item without doing any of them; a
vault item saved its seeds and the description by separate buttons, so
part of it could be left out; and a plan could leave a vault's stick at
no place (the owner's screenshot: a single key whose words were
unticked, its only copy in Vault 1, Vault 1's stick nowhere). The
owner's ask: "we need to force users to follow the backup flow they
already selected", and for a vault item, "make ONE button that saves
everything they indicated they wanted saved in the vault into the
vault; do not allow continue until they've done that".

**What it does** (`docs/NEW-WALLET.md` §14; `faraday-core/src/held.rs`;
`docs/WALLETS.md` §5, "The backup before the spend").

- *Which keys.* A key is made here when its words came from New key, or
  when a passphrase was added to it in Create. It stays held until a
  backup checklist that keeps its seed, and that passphrase, is
  complete, every item done. Keys restored from words, a SeedQR or a
  vault with nothing pending are already backed up and sign at once.
- *What waits.* Signing a transaction with a held key is refused; the
  wallet card's first button is **Finish the backup first**, and Who
  signs lists the key as "Waiting for its backup". Signing a message is
  not a spend and is not held. Receiving is warned, not blocked: every
  address shown for the wallet carries "Back up before you receive".
- *The checklist forces the plan chosen.* An item's button is its next
  action until it is done, then **Continue**; the items after it open
  only once it is done. A vault item has one **Save into {vault}**,
  which saves every seed, passphrase and the description the plan puts
  in that vault. Every vault's stick has a place, and a plan that keeps
  no copy of a key made here is not made into a checklist. **Change the
  plan** stays the way out of an item: a simpler plan is allowed, and
  its own checklist must still be completed.
- *Across a lock.* Which keys are held is kept by fingerprint, never by
  a secret: bit 1 of the key record's flags in the vault beside the
  seed, and a `held` line in the backup plan's record (`docs/VAULT.md`
  §7). A key loaded from a vault that says so is held again. The seed
  and the passphrase are written nowhere new.

**The owner's decision: one line of fact on the plan.** The plan's first
card says "A key made here signs only once this backup is done.", only
when the wallet has a key made here. It states what follows from the
plan, as F1's line does, and explains nothing.

**What it leaves.** A vault locked before the checklist completes
keeps its flag: a key loaded from it later is held again, until a
checklist that keeps it is completed with that vault open.


### F7 TOTP codes, and a clock the person can set (2026-10-11)

**Why.** A vault entry holds a TOTP secret, scanned, typed or imported,
but Faraday showed only the secret (`PLAN.md` §6.4: "it does not
generate TOTP codes"). The owner's ask: "a TOTP viewer inside Faraday,
so we can scan TOTP QR codes or load the keystring and see the codes",
with the time set by hand, and a time zone in `faraday-settings.txt`
"so the time could be correct from boot". A code is right only when the
clock is: the Pi has no real-time clock and starts at 1970, and the
stick reads the PC's hardware clock as UTC, which a PC running Windows
keeps in local time.

**What it does** (`docs/TOTP.md`).

- *Codes.* A **TOTP codes** screen lists a code for every entry with a
  TOTP secret in the open vaults, live, with the time left; the entry's
  detail shows its code too. A bare Base32 key is accepted where a
  `otpauth://` URI was, and stored as one. The secrets stay in the
  vault (type 6, field 6); nothing new is stored outside one.
- *The time.* The time can be set by hand, in local time, to the
  minute. It is kept in the app as an offset from the computer's clock,
  survives a lock, and is gone at power-off. A clock before 2020 counts
  as not known.
- *The zone.* A fixed offset from UTC (−12:00 to +14:00, 15-minute
  steps) in Settings and in `faraday-settings.txt` (`tz=+02:00`). Every
  time shown is local; every time written into a format stays UTC.

**The owner's decisions.** The secrets live in the vault. The zone is a
fixed offset, not a named zone: no time zone database in the image, and
the person changes it at daylight saving; the time shown on the codes
screen makes an hour's error visible. Where the clock is not known, the
codes screen opens Set the time first.

**Decided with them** (architect). The time is asked for only when the
codes screen is opened, never at boot or anywhere else (owner). On the
stick and the Pi, opening it asks even when the clock is known, with
**Keep this time** one press away, since a known clock can still be
hours off; once answered, it is not asked again that power-on; the desktop app's clock is
its host's and is not asked. The code is shown plainly; the secret stays
hold-to-reveal. Faraday never writes the computer's clock.

**What it leaves.** HOTP codes, Google Authenticator's export QR and
Steam's codes are not read. File dates on a stick still come from the
system clock, 1970 on the Pi.

### F8 Scan to sign, and a Spend tab that asks for data (2026-10-11)

**Why.** In the demo's backup start the owner could not see how to bring
in the second signer: "Add a key" opened a page for typing, the chart's
"Add its key" links asked them to know which key their paper was, and
scanning was a second press on the next page. The Spend tab, meant for a
first-time spender, asked them to classify their envelope before
anything was read.

**What it does** (`docs/SCAN-TO-SIGN.md`).

- *Scan a key.* Where a wallet is short of keys, one **Scan a key**
  opens the camera and takes any of its missing keys; Faraday works out
  which. A seed whose key matches a missing slot loads with no
  passphrase question; one that matches nothing asks for its passphrase.
- *Can spend.* A wallet with enough keys here shows a green **Can
  spend** pill on its card, its list row and Who signs.
- *Names.* Home's ways in read **Make a new wallet** and **Restore an
  existing wallet**.
- *The Spend tab's intake.* **Scan the QR codes you have** and **Type
  the words you have**, a vault's Unlock only when one is found, and
  the next question worked out from what came in: anything else after
  one seed, the description after several, the wallet's shape when
  there is none.
- *A guessed wallet is checked online.* Continue only through **Yes, my
  money shows**; **No money shows** tries a passphrase, Taproot, nested
  SegWit and legacy in turn.
- *GPG.* Sign a file signs several files ticked in a list.

**The owner's decisions.** All of the above (2026-10-11), with: one
seed and "That's everything" opens native SegWit; seeds without a
description build standard multisig only with every key in hand;
Restore on the Wallets tab stays the expert flow for now.

### F9 Writing over a file on a stick, and an Add glyph (2026-10-11)

**Why.** A stick visit never wrote over a file: a taken name became
`name-2.ext` without a word, so a second copy of a wallet or a PSBT
piled up beside the first. And the vault's add actions hid behind a
faint **Add…** row.

**Decision** (owner: HANDOFF item 68; `docs/FLOWS.md` decision 58).
A visit's write whose names are taken on the stick asks first:
**Replace**, **Keep both** or **Cancel**. Replace is
`faraday_files::write_replacing`: the new bytes under a hidden name,
read back and compared, then the old file removed and the new one
renamed, as a vault write-back does; `StorageCommand::Write` and the
disk process's `Request::Write` carry `replace`. A vault is never
written over another vault, and the same vault is always written back,
whatever the choice; neither alone brings the sheet, so the everyday
vault write-back is not asked about.

The vault's add rows carry `Icon::Add`, Font Awesome 6 Free Solid
`plus` (U+F067). `osk-ui` is shared with upstream OpenSigner, so, as
with F4's `SdCard`, the variant, its place in `Icon::ALL`, its code
point and name, the `tools/fontbake` entry and the rebaked
`core/osk-ui/assets/icon.outl` are an upstream change.

### F10 The name as a wordmark (2026-10-11)

**Why.** The headers set "Faraday" in Noto Sans SemiBold, the face of
every label on the screen, so the name beside the mark read as one more
label.

**Decision** (owner, from mockups of three faces). The name is a
wordmark: FARADAY in Archivo Expanded SemiBold (wdth 125, wght 600),
hand-kerned, each A's crossbar cut free of its legs. The keyhole shield
beside it is unchanged. The outlines are SVG path data in
`faraday/faraday-ui/assets/wordmark.txt`, with Archivo's SIL Open
Font License beside them; `Ui::wordmark` fills them in the text colour,
so every theme draws the name as it draws its text, and no font is
added to the bake. The sidebar sets its capitals 10 units tall, which
leaves the widest network pill (Regtest) its gap; the Pi's header, with
more room, 11. The boot logo's 5×7 pixel name is unchanged.

### F11 The Starter: one download that makes sticks and cards (2026-10-12)

**Why.** Getting Faraday onto a stick or a Pi card, and a laptop to
start from it, took a terminal, `dd`, a firmware menu and a Secure Boot
setting; on Windows a wrong step can leave the computer asking for a
BitLocker recovery key nobody has. The release offered only the plain
files.

**Decision** (owner, 2026-10-12; the spec is `docs/STARTER.md`).

- A separate app, the Starter, shown to people as Faraday, is the main
  download: `Faraday-X.Y.Z-Windows.exe` and `Faraday-X.Y.Z-Linux`. It
  makes a Faraday stick or a Pi card, guides starting a computer from
  the stick and putting its Secure Boot back, transfers files by QR
  code, opens the desktop app, and installs itself to the Start menu or
  app menu, or runs without installing. It has no network code.
- The desktop app stays a testing app, opened on its own or from the
  Starter. Installing the Starter on Linux asks whether to install the
  desktop app too, unticked. There is no Windows desktop app.
- The plain files stay in every release, under the Starter, with new
  names (`faraday-desktop-…`, `faraday-stick-….img.xz`,
  `faraday-pi-….img.xz`). Images are published as `.xz` only, with the
  `.img` inside listed in `SHA256SUMS` as well.
- No dev images are published, plain or inside the Starter; developers
  build them and write them with the Starter's `--image`.
- Replacing Faraday on a stick while keeping its data partition is
  offered to everyone, not only developers. This changes F2, which kept
  the boot-partition-only flash for development ("It is not offered to
  users"). The Starter
  never reads or writes the data partition, and its Learn page says
  that plugging the stick into an online computer is what exposes it,
  and that Settings → Upgrade a Faraday stick avoids that.
- Nobody who follows the Starter's steps is ever asked for a BitLocker
  recovery key: before any firmware step it suspends BitLocker until
  Secure Boot is back on, and shows the recovery key, which the person
  confirms by typing part of it back (`docs/STARTER.md` §6). (Changed
  by F17 and F21; superseded by F33: the step is turning Secure Boot
  back on.)
- The Windows Starter is not code-signed: a certificate names a
  verified person or company, and Faraday is published under a
  pseudonym, for privacy. Windows' first-run warning is documented, with
  how to check the file against `SHA256SUMS` and its signature instead.
- The Pi image aims at as many Pis and screens as can be tried: the
  layout already follows the screen's size and density; the screen
  that cannot be detected is chosen when the card is made.

### F12 The Starter, reviewed: what the owner changed in v0.2 (2026-10-12)

**Why.** A review of `docs/STARTER.md` v0.1 found places where the spec
made the person do more than needed, two security holes in the
BitLocker guard's resume path, a test the spec could not run, and a
Windows exclusion with no technical ground. The owner answered each
point the same day; `docs/STARTER.md` v0.2 carries the answers (its §12,
D12 to D21) and §13 lists what moved.

**Decisions** (owner, 2026-10-12).

- Home leads with the next step for this computer's state (make a
  stick, the BitLocker step, restart into firmware, turn Secure Boot
  back on, resume BitLocker), read live, with the menu rows below.
- The Linux Starter writes sticks through udisks2 and never runs as
  root; the Linux download is a `.tar.gz`, so the executable bit
  survives and nobody runs `chmod`.
- No disk is ever chosen for the person. Keep data reads the stick's
  Faraday version from its boot partition and applies F2 and F5's
  rules: the same build refused, an older one warned about.
- The guard shows and confirms the recovery key before suspending, and
  hides it while the person types two groups back. The phone gets it as
  a plain-text QR code or a photo of the screen. No startup task
  resumes BitLocker: the Starter offers Resume only with Secure Boot
  on, with a per-user logon reminder for an installed copy. Windows
  Home is proved on a Home computer of the owner's, since a virtual
  machine cannot turn Device encryption on; Pro in a virtual machine.
- Maker pages only for families tried on a computer; the general pages
  carry everyone else.
- The desktop app comes to Windows (changing F11's "there is no
  Windows desktop app"): nothing in it is Linux by design, and the
  camera module the Windows Starter needs serves both.
- One Pi screen for now, the 2.8 inch panel; panel profiles and other
  sizes wait for a second screen.
- `tools/flash-stick` was removed, in batch L, when the Starter wrote a
  developer's own image (`--image`); the window and its launcher were a
  stopgap.

### F13 Faraday on Macs: the stick, the desktop app and the Starter (2026-10-12)

**Why.** F11 said there is no macOS app, and the stick started only on
PCs. Intel Macs can start the PC stick with little change, and Apple
Silicon Macs can start Faraday through Asahi Linux's boot chain. The
owner asked for Mac support across the board, provided none of the
security properties Faraday relies on is weakened.

**Decision** (owner, 2026-10-12; the spec is `docs/MAC.md`). This
supersedes F11's "no macOS app".

- Intel Macs with 64-bit EFI start the PC stick. Its kernel gains
  `applespi` (2015–2017 MacBook keyboards, reachable only through the
  firmware-described device) and OHCI; `bcm5974` is forbidden, for
  `HID_APPLE`'s reason. T2 Macs are used with Startup Security lowered
  and are told to set it back; their built-in keyboard needs t2bce,
  which is not carried, so they take a USB keyboard.
- M1 and M2 Macs start a new image, `arm64-apple`, through a start-up
  file sealed by the Mac's recovery system: m1n1, the device trees and
  U-Boot in one file, with nothing loaded from the Mac's disk, no
  command line, no USB debug mode and no initrd from outside. M3 waits
  for Asahi's release; M4 and M5 for m1n1.
- There is no project key. At setup the person chooses: any Faraday
  stick starts, or only one signed with their own Secure Boot key,
  preseeded into U-Boot. Nobody holds a key that starts Faraday on
  every set-up Mac.
- Sealing is typed by the person in Recovery's Terminal, after checking
  the file's SHA-256 against a value from outside that Mac (the release
  page on a phone, or Faraday's Secure Boot screen), because setup is
  done from a macOS that may be compromised.
- Apple's restore-file parts the start-up volume needs are downloaded
  by the Starter through macOS's own `curl`, pinned by SHA-256; the Starter
  carries no network code.
- Asahi's two keyboard drivers are the only out-of-tree code, in the
  Apple Silicon kernel only. The M2 trackpad, which needs a firmware
  file copied from macOS, is not supported.
- The desktop app and the Starter run on macOS, ad-hoc signed and not
  notarized, for the reason F11 gives for Windows. (Owner, 2026-10-14,
  F30: the desktop app is not published on macOS either, and MAC batch
  D is dropped; developers can still build it.)
- The Starter can remove the start-up volume again, for privacy.
- The README lists the supported Macs by model identifier, each claim
  cited, as Expected or Not supported. Nothing published says which
  models were tried: those are the owner's own Macs.
- There is no Mac to build on yet: the Intel Mac work, the Apple
  Silicon image and its start-up file go ahead; the macOS apps and the
  Apple Silicon setup wait for one.

### F14 Maker pages are published several families at once (2026-10-12)

**Why.** STARTER D6 published a maker page only for a family tried on a
computer. The computers tried are the owner's, and their make and model
are personal information: a page for one family alone would tell which
maker the owner has, and a page naming the model it was tried on would
tell more.

**Decision** (owner, 2026-10-12). Maker pages are published several
families at a time, never one alone, each checked against the maker's
own documentation. No page names a model, and nothing published says
which families were tried (`docs/STARTER.md` §7.2, §7.6, D6).

### F15 Faraday Companion: an Android app with no network access (2026-10-12)

*Superseded by F34 (2026-10-15): the Companion is dropped.*

**Why.** The device and the Starter both hand things to a phone, and
the phone has nothing of Faraday's to take them: the Pi's codes are its
only way out and a webcam reads its panel badly, the Starter's guides
are on the website, and the BitLocker recovery key goes into a notes app
that may sync it. An app that also checked balances or broadcast would
need the network, and the owner does not want Faraday to become an
online wallet.

**Decision** (owner, 2026-10-12; the spec is `docs/COMPANION.md`).

- An Android app, **Faraday Companion**, offline only. Its manifest
  asks for the camera and no network permission, which anyone can
  check on the APK; the release build is reproducible.
- It reads the device's codes and shows codes for it, keeps what
  arrives, keeps wallet descriptions and BitLocker recovery keys, and
  carries the Learn pages, opening the Starter's guide addresses as
  its own pages.
- It never takes a seed, a private key, an encrypted backup, a sealed
  vault or a TOTP secret, by any way in, and never signs.
- A kept recovery key is offered for deletion once the person is back
  in Windows: a key Windows did not ask for is not needed there, and
  the Starter shows it again the next time Faraday is set up on that
  computer. (Superseded by F33: the Companion keeps no BitLocker key.)
- Online use, if ever, is a separate app, so this one never gains the
  network permission.
- Wallet descriptions and BitLocker recovery keys open only after the phone's
  own unlock, their file's Keystore key bound to it.
- Its core is its own, without Faraday's vault, signer or key loaders,
  checked from its dependency list by a lint.
- Published on the release page and on F-Droid under the project's
  name, application ID `app.faradayoffline.companion`, signed with an
  APK key the owner generates and keeps. Not registered with Google's
  developer verification, for the pseudonym; the install route on a
  phone that asks is documented. No iPhone app in phase 1. Every point
  is `docs/COMPANION.md` §12, D1–D10.

### F16 The hub app is Faraday Starter; the desktop app is Faraday Desktop (2026-10-12)

**Why.** STARTER D10 named the hub app plain "Faraday" to people. Installed
beside the desktop app, two menu entries began "Faraday", and the
desktop app's "Faraday desktop (testing)" was cut off in a launcher.
"Faraday Hub" was chosen first and withdrawn the same evening for its
unwanted associations, and "Setup" the same evening for "Starter":
the app is what a person opens each time Faraday is to start on this
computer (the stick made, BitLocker suspended, Secure Boot turned off,
the restart into the stick), not only the first time.

**Decision** (owner, 2026-10-12).

- The hub app is **Faraday Starter**: its window title, menu entry, Start
  menu shortcut, installed name and uninstall entry, and the release
  files `Faraday-Starter-X.Y.Z-Linux.tar.gz` and
  `Faraday-Starter-X.Y.Z-Windows.exe`. In the code it stayed `faraday-hub`
  at first (owner, 2026-10-14: the code was renamed to `starter`
  throughout, so the crates, modules and paths read `faraday-starter`,
  `faraday-starter-shell` and the like, matching the public name).
- The desktop app is **Faraday Desktop**, with no "(testing)": it opens
  on testnet, so its purpose shows; its menu description says test
  seeds only.
- Their headers show the wordmark with the second word beside it,
  STARTER or DESKTOP, in the same Archivo Expanded (wdth 125) at weight
  400, in the grey of secondary text, so FARADAY stays the name. The
  outlines join the wordmark's in `faraday-ui/assets/`; STARTER's A
  has its crossbar cut free of its legs, as FARADAY's A's do.
- The stick, the Pi card and the project remain **Faraday**, with the
  wordmark alone.

### F17 The BitLocker step is the recovery key; suspending is the extra-safe choice (2026-10-13)

*Superseded by F33 (2026-10-14): there is no BitLocker step.*

**Why.** STARTER §6 made suspending BitLocker the one way through to the
firmware steps on Windows. BitLocker checks Secure Boot only when
Windows starts: a person who turns Secure Boot off, starts the stick,
and turns Secure Boot back on before Windows starts again is never
asked for the key, as a computer of the owner's showed. The prompt
comes when Windows starts while Secure Boot is off, most often on the
restart that saves the firmware setting when the stick is not chosen
in time, or when the firmware's keys change. Suspending covers those
slips, at the cost of an elevation prompt, a typed-back key, a logon
reminder and a Resume step for everyone. §6.6's runs in a virtual
machine (2026-10-13) showed the suspension and the resume work.

**Decision** (owner, 2026-10-13).

- On Windows with a protected drive, the BitLocker step by default is
  the recovery key in hand: where it is kept, **Show the recovery
  key** for whoever wants it written down (one elevation prompt), and
  **I have my recovery key**, which opens the firmware steps. Nothing
  on the computer changes.
- **Extra safe: suspend BitLocker** on the same page leads to the
  guard as §6 built it: the key shown and typed back, the suspension,
  the marker, the logon reminder and Resume. It is offered, never
  required, and is unchanged.
- The guides say, at the step that turns Secure Boot off, to choose
  the stick from the boot menu before Windows starts, and to turn
  Secure Boot back on before starting Windows.

### F18 Firmware sketches in Faraday Starter and the Companion (2026-10-13)

*The Companion's half is superseded by F34 (2026-10-15): the sketches are in the Starter only.*

**Why.** The maker pages describe the firmware's menus in words. A
picture of the screen to look for helps more, but the makers' own
screens are theirs, and differ by model.

**Decision** (owner, 2026-10-13).

- The maker pages carry sketches of the firmware's screens (where
  Secure Boot is, the save step, the boot menu), drawn by Faraday's
  own code from the makers' simulators and manuals to look like a
  firmware setup screen: square corners, firmware colours (the screen
  colour nearest the family's own, black where unsure) and a text-mode
  font. Never a maker's screenshot, logo or name. Each carries a line
  under it, in Faraday's own text, saying it is a mock-up and the real
  screen may look different.
- One sketch serves a family of menus; none names a model (F14).
- Only Faraday Starter and the Companion draw them. The stick, the Pi
  and Faraday Desktop show the pages without them.

### F19 Audits from experts and AIs, published with Faraday's response (2026-10-13)

**Why.** Nobody outside the project has reviewed Faraday. AI audits are
now cheap and will arrive in many shapes; without one shape, they cannot
be compared, and a summary written by the project would read as the
project grading itself. Critical findings must not arrive in public.

**Decision** (owner, 2026-10-13; the process is
`docs/AUDIT-SUBMISSIONS.md`).

- The website gets an Audits page, linked from the bar, inviting audits
  from experts and AIs, with a request that people follow or paste into
  their prompt so every audit comes back in the same shape: two Markdown
  files, `audit.md` and `private.md` (critical and high findings), a
  summary of at most 150 words, personal information redacted, and a
  SHA-256 hash of `private.md`.
- Submissions go through GitHub only: a public issue with `prompt.md`,
  `audit.md` and the hash of `private.md`; `private.md` in a private
  vulnerability report. Nothing by email.
- Each entry shows the submitter's summary, unedited, Faraday's verdict
  on each finding, and the full prompt and output. Faraday replies
  within 15 days; `private.md` is published once fixed, its hash showing
  it is the file that was sent.

### F20 Faraday Desktop's mainnet is the demo; test networks take any seed (2026-10-14)

**Why.** Faraday Desktop runs on an online computer. It started on
testnet and took any seed on every network, mainnet included, so
nothing stopped a funded seed being typed into it. The demo website
already took only public seeds on mainnet; the two differed.

**Decision** (owner, 2026-10-14).

- Faraday Desktop starts on mainnet as the demo. On mainnet it takes
  only the eleven public seeds (`faraday-core/src/demo.rs`: each a word
  repeated twenty-four times), shows every address with a checksum
  broken in one character so no wallet sends to it, and New key adds
  the next public seed not loaded in place of the key it made. The demo
  website is the same app on the same rules.
- The passphrase fields stay on the Key card in the demo; what is typed
  there is ignored, and the public seed is added with none. The card
  says which public seed Lock in adds, and which it added.
- Testnet, signet and regtest take any seed and show real addresses.
  Going to one, chosen from Network or by loading a test wallet or
  transaction, first shows a warning: for testing only, this computer
  is online, never enter a seed that holds funds. Cancel stays on
  mainnet; once acknowledged, it is not shown again that session. The
  old warning before mainnet is gone.
- Mainnet is refused while a key other than a public seed is loaded.
- A shell can fix the session's network (`Session::fix_network`): the
  Network row offers only that one, and a wallet or transaction on
  another is refused. The website fixes it at mainnet, so it never
  takes any other seed.
- The stick and the Pi are unchanged: any seed on every network, real
  addresses.
- The demo refuses to load a seed other than a public one, but the words
  have been typed on an online computer by then. On a test network the
  warning is the only guard: an offline app cannot tell a funded seed
  from a test one.

### F21 The BitLocker step is optional (2026-10-14)

*Superseded by F33 (2026-10-14): there is no BitLocker step.*

**Why.** F17 made the recovery key in hand the default BitLocker step,
and still put it before every firmware step: on Windows with an
encrypted drive, the Starter's first request after Make a Faraday
stick was for the recovery key. Turning Secure Boot off to start
Faraday, and back on before Windows starts, never makes Windows ask
for that key (F17's reason). A first request for a key a person does
not need reads as a warning that something could go wrong.

**Decision** (owner, 2026-10-14).

- On Windows, the BitLocker step no longer comes before the firmware
  steps. Home's card goes from **Make a Faraday stick** to **Restart
  into the BIOS**; the Start page's firmware buttons are offered
  whatever BitLocker's state. Another encryption product (§6.4)
  still closes them until the person says they have its rescue disk
  or password.
- Home's checklist keeps a BitLocker item, unnumbered, never the next
  step, titled **Back up the BitLocker recovery key**, its state
  "Optional". A short note under it says Secure Boot goes off, then
  back on before Windows starts, so Windows does not ask for the
  recovery key, and a backup is for a start where Secure Boot is
  still off. Under the note, the link **What Secure Boot and BitLocker
  are**, which leaves Home's card.
- This note is the one explanation on a working screen, an exception
  to the rule in `CLAUDE.md`, made by the owner so the step reads as
  optional. It is on Home and on the BitLocker page, and nowhere else.
- The BitLocker page is titled **BitLocker recovery key**, leads with
  the same note, then **Also kept in**, **Show the recovery key** as
  the primary button, and **Extra safe: suspend BitLocker** as before.
  **I have my recovery key** goes: nothing waits on it. Once the key
  has been shown this session, Home's item reads "Key shown".
- The extra-safe suspension, its marker, the logon reminder and
  Resume are unchanged.
- The Learn pages "What Secure Boot and BitLocker are" and "BitLocker
  and starting from a stick" are shortened, and say plainly that
  turning Secure Boot off for the time it takes to start Faraday is
  safe. The maker pages say the recovery key is optional.

### F22 An armored GPG public key or signature is public (2026-10-14)

**Why.** A person copying their GPG public key and revocation
certificate (`.asc`, the files Faraday's own GPG flow writes) from one
stick to another was told "This may be a secret": `wallet::classify` read
them as plain text, and text may be a secret. They are public.

**Decision** (owner, 2026-10-14).

- A file that is nothing but one ASCII-armored OpenPGP block of type
  `PGP PUBLIC KEY BLOCK` or `PGP SIGNATURE`, whose packets are all public
  kinds, is a public file: written as it is, ticked by Select all, kept
  across a lock (`PLAN.md` §4.4).
- *Amended 2026-10-14 (F32).* `faraday-pgp` reads every packet to its
  exact end, not only its tag: a public key or subkey is a version-4
  public key with nothing after its last field, a user ID is UTF-8, a
  signature is a version-4 signature with well-formed subpackets. A user
  attribute, a trust packet or any other tag refuses the block. A secret
  packet whose tag was changed to a public one fails to parse. A key
  block starts with a public key packet. The armour headers are the five
  GnuPG accepts (Version, Comment, Hash, Charset, MessageID); a header
  value or a user ID that is a seed's words, an extended private key or a
  share makes the file not public. A missing or wrong CRC does not refuse
  the block (RFC 9580) and does not make it public.
- *Amended 2026-10-14 (F32).* A revocation certificate is not public:
  whoever holds it can revoke the key, and GnuPG advises keeping it
  private. It is a signature in a key block with no key before it, so it
  does not read as public; and its file, `{name}-{FPR}.rev`, has GnuPG's
  text before it and a colon before its dashes. It goes out only through
  the secret sheet.
- Anything else stays as it was: a private key block, text around the
  block, armor that does not read, or any secret-key packet inside is
  text or a file Faraday does not read, which may be a secret.

### F23 The PC stick's kernel is 6.18 longterm, ahead of upstream's (2026-10-14)

**Why.** Upstream's board file pins the PC stick's kernel to 6.6.84
while the 6.6 longterm series is at 6.6.158, so the stick missed about
seventy stable releases of fixes. And on the stick Faraday could not
tell whether its screen is a TV (STARTER §3.1a): DRM is forbidden, so
nothing reads the screen's EDID. From Linux 6.19 the EFI stub reads it
from the firmware and puts its base block in `boot_params`, with no
graphics driver. Faraday had decided to add nothing to the kernel
until upstream moved it; waiting left both open.

**Decision** (owner, 2026-10-14).

- Faraday builds the PC stick (`x86_64-uefi`, release and dev) on
  mainline 6.18 longterm, 6.18.56 to start, and follows its stable
  releases. Its image overlay replaces the board's
  `buildroot.fragment` with a copy that differs in the tarball, its
  sha256 (from kernel.org's signed `sha256sums.asc`) and
  `BR2_LINUX_KERNEL_PATCH`; the board's `linux.fragment` and its
  `kernel.required` and `kernel.forbidden` lists are checked as
  before.
- Five mainline commits from 6.19 are carried as patches in
  `faraday/image/overlay/boards/x86_64-uefi/linux-patches/`, each the
  commit's diff unchanged: 9d805709d897, ae42b9c5ddda, 17029cdd8f9d,
  e41ef37ddfe7 and 5688e977ed2b. Nothing else is patched, and DRM
  stays forbidden.
- The stick reads the EDID's base block from
  `/sys/kernel/boot_params/data` and names a screen whose name names a
  TV or a KVM (STARTER §3.1a). `faraday-core`'s `edid` module holds
  the rules the Starter and the stick share.
- The Pi's kernel is unchanged: upstream's Raspberry Pi tree.
- Upstream is asked to move the board to 6.18 and carry the five
  commits (`local/for-upstream/kernel-6.18-edid.md`). When its board
  has both, Faraday's copy and its patches go.
- `faraday/image/run-build.sh` cleans a variant's Buildroot tree when
  its kernel source (tarball, hash, patches) changes: the toolchain's
  headers are the kernel's, and Buildroot does not notice a new
  tarball in a tree it has built.

### F24 The stick's reminders come before the first secret, not at start (2026-10-14)

**Why.** Every start of the stick carried "On a TV? Stop: a smart TV
can record its screen." on Home, a wireless receiver added a line
there, and a desktop was asked at start whether its screen is a TV.
A line that is always there stops being read, and none of it matters
until a secret is about to be shown or typed.

**Decision** (owner, 2026-10-14).

- What the stick found (a KVM by its USB identity, a screen whose EDID
  names a TV or a KVM, a virtual machine) is named at start as before
  (STARTER §3.1a), with Check again and Continue anyway, and a virtual
  machine is still refused.
- What it did not find but may capture a secret is a reminder: the TV
  line, unless the screen's EDID settled it (F23), and the wireless
  receiver's line, or the Bolt's, while one is attached. Neither is on
  Home, and the question whether the screen is a TV goes.
- The first action that would put a secret on the screen or have one
  typed opens one sheet, **Before a secret is shown**, with what was
  found and the reminders due; with only reminders, **Cancel** and
  **I understand**. Continue anyway goes on from the reminders too.
- When the action also needs a stick pulled, the reminders are on the
  sheet asking for it, and pulling the stick goes on from them: one
  sheet, never two in a row.
- A reminder seen does not come back this session; a new one comes at
  the next secret.
- The Starter is unchanged: it warns ahead of time, on its Home.

### F25 The Companion's APK signing key is made, kept and used in Faraday (2026-10-14)

**Why.** COMPANION D7 left the Companion's APK signing key to the
owner, generated and kept outside Faraday. That is the one key whose
loss cannot be repaired: Android installs an update only when it is
signed by the key that signed the installed copy. A keystore kept
elsewhere is a key none of Faraday's rules cover, while the release's
OpenPGP key and the Secure Boot keys are already made, kept and used in
a Faraday vault. Android accepts no Ed25519 APK signature, so the GPG
key cannot sign the APK itself.

**Decision** (owner, 2026-10-14, taking the recommendations of
`docs/APK-SIGNING.md`, which is the spec; its §12 records the
answers).

- The key is a third kind beside GPG and Secure Boot: made on a Faraday
  stick or Pi, kept in a vault as record type 12 (`docs/VAULT.md` §7),
  used there. Its private part never leaves the vault. The vault's
  sealed copies on sticks are its backup. Without Faraday, given the
  passphrase, `tools/vault/open.py --field 12.1 --out FILE` writes the
  key alone to a mode-0600 file for `apksigner`, on an offline
  computer, by the person's own hand.
- RSA-3072 with PKCS#1 v1.5 and SHA-256: 128-bit strength for a key that
  cannot change, on `faraday-sb`'s existing RSA, with no new
  dependency, and deterministic, so its output can be compared byte for
  byte with `apksigner`'s.
- One key per app: each key is bound to one package, typed at Make
  (`app.faradayoffline.companion` filled in) and shown on its page;
  Sign an APK refuses an APK of another package. Another app gets
  another key, as Android advises.
- A self-signed certificate naming the project only ("Faraday
  Companion"), with no expiry. Its SHA-256 is shown on the key's page,
  the Companion's About, README and F-Droid's `AllowedAPKSigningKeys`.
- APK Signature Scheme v2 and v3, as `apksigner` writes them for
  `minSdkVersion` 26; no v1, no v4, no rotation.
- **Sign an APK** reads an unsigned, aligned APK copied in on a stick
  (on the Pi, an SD card), shows its file name, size, SHA-256, package,
  version and minimum Android, and writes the signed APK For the stick
  as `faraday-companion-X.Y.Z-android.apk`. It refuses a file that is not
  an APK, is already signed, is not aligned, or needs v1, and any ZIP or
  manifest that two readers could read two ways (manifest attributes
  known by resource ID, as Android knows them). The signed
  file is the unsigned one with the APK Signing Block inserted and one
  offset changed, so F-Droid can copy its signature onto its own build.
- The review signs only once the person has entered the SHA-256 of an
  independent second build of the same source, made on another
  computer, and it matches the file's (at least 32 hex digits): the
  build machine is online, and a signature on a bad APK cannot be
  withdrawn.
- Each signature is blinded with fresh bytes of the system's, and
  checked against the certificate before anything is written, so a
  faulty signature never leaves the device.
- A key made in a session signs nothing until the vault holding it has
  been written to two sticks with different FAT serials, as a Bitcoin
  key made here spends nothing until its backup is done.
  Replaced by F31: the key is made after its places' sticks are read
  and its vaults opened, and signs once every place (two or more) is
  written by the shared backup flow, a second copy refused on the stick
  holding the first and confirmed by a hold, with no FAT serial; the
  same wait covers GPG and Secure Boot keys.
- Faraday Desktop offers it too, as it offers GPG and Secure Boot. The
  real key is made on the stick of a published release that has the
  feature. Which vault holds a key is the person's choice, with no
  default steering it; the Learn page says that a slot holding one does
  not open in builds up to 0.1.2, and that one passphrase then guards
  everything in that slot.
- The code is a new `no_std` crate, `faraday/faraday-apk`, judged by
  `apksigner`: its signed APK is byte for byte the one `apksigner sign`
  makes with the same key.
- A file that is exactly one PEM `CERTIFICATE` block is public, as F22
  made an armored OpenPGP public key.
- COMPANION D7 is replaced; COMPANION §2, §3, §4.8, §8 and §10 batch C
  follow. The certificate is committed as `faraday/companion-cert.pem`
  once the key is made, as `faraday/release-key.asc` is.

### F26 Files of any size on a stick visit, imported whole or only for signing (2026-10-14)

**Why.** A stick visit read at most 18 MiB of a file, a figure set
because a vault with four 4 MiB slots is the largest thing Faraday
writes. A file to sign can be larger: a release artifact, an APK, a
disk image. It was listed and could not be ticked. Lifting the cap
means large files in the memory a vault unlock also needs.

**Decision** (owner, 2026-10-14; the spec is `docs/LARGE-FILES.md`).

- No cap on reading: FAT32's largest file, 4 GiB less one byte, is the
  only size limit. `MAX_READ` and `READ_MAX` go.
- A file up to 64 MiB comes in as before, without asking. A larger one
  is listed unticked with its size; ticking it offers **Import the whole
  file** or **Import only what signing needs**. The boot import reads
  none over 64 MiB and counts them.
- A whole import is refused where the memory left would be less than
  the largest need of the vaults this session knows, or, with none
  known, of the largest memory that fits, Create's custom memories up to
  4 GiB included; it is warned within 128 MiB of that line. A vault's
  need is its Argon2id memory, twice its file and 100 MiB, from one
  function Unlock's check also uses. The memory is `MemAvailable` read
  once at power-on and kept across locks, less a ledger the app keeps of
  everything it holds twice (its copy and `/run/faraday`'s) and every
  read or product in progress; a reservation that succeeds is not taken
  as proof. Every copy counts, the signed APK made from a whole file
  included; GPG signs from a hash state and copies nothing. The visit's
  total of smaller files follows the same rule. An unlock refused for
  memory names the large files held and offers to delete them.
- The Inbox and Outbox stop being cloned and rewritten whole at every
  save: items are shared buffers, only what changed is written, each
  box saves on its own.
- A signing-only import streams the file once and keeps its name, size,
  OpenPGP SHA-256 and SHA-512 states (the SHA-256 shown is the first of
  them finished), and for an APK the chunk digests, central directory,
  end record and manifest fields, all from that one pass: a central
  directory that streams differently from the copy read first keeps
  nothing. In Files it signs and does nothing else. A lock keeps it, in
  a directory and format of its own that no file is read from.
- A GPG signature over it is an ordinary `.asc`. A signed APK from it is
  written on a later visit by copying the unsigned file from that
  stick with the signing block inserted, each part checked against the
  first read before it is written, under a temporary name renamed when
  read back whole. A source that changed writes nothing. A file under
  the output name is always the whole output; a stick pulled in the
  instant of FAT's rename can leave neither name, which FAT does not
  allow Faraday to prevent.
- The disk process gains reads and writes in parts of up to 4 MiB and
  lists one stick's files per answer; the app hashes every part itself
  and drives the copy. `MAX_FRAME` becomes 5 MiB. The disk process
  names the one temporary file itself, and no request renames or
  removes any other file. It keeps its place in a file's cluster chain
  between parts, refuses a chain that loops, and abandons an open write
  when the stick goes or a new app process connects. The privilege
  split is unchanged.
- File names with control or format characters, or over 230 bytes, are
  listed and cannot be ticked, on every visit.
- The Pi does the same by SD card visit; Faraday Desktop has the same
  behaviour, with the computer's memory now, and no refusal where the
  system gives no figure.
- Specified, not built. Until batch L1, `README.md`, `PLAN.md` and
  `docs/FLOWS.md` give today's 18 MiB.

### F27 Nostr keys from the seed, by BIP-85 application 128002' (2026-10-14)

**Why.** A Nostr identity is one secp256k1 key, and losing it loses the
identity. BIP-85 2.1.0 (2026-08-02) adds application 128002', which
derives Nostr keys from a Bitcoin seed at
`m/83696968'/128002'/{identity}'/{account}'`, so a seed Faraday already
backs up can back up Nostr keys too. OpenSigner lists the application
as open (`docs/PLANNING.md` §16.114).

**Decision** (owner, 2026-10-14, revised the same day after a review;
the spec is `docs/NOSTR.md`).

- BIP-85 128002' only. No NIP-06 (`m/44'/1237'/…`): NIP-06 is applied
  by a Nostr app to the words it is given, and its safe use is with
  words that hold only Nostr, which Faraday already makes as a BIP-85
  child seed. Faraday applying NIP-06 to the main seed gives nothing
  over 128002'.
- The nsec is the first 32 bytes of the 64 bytes of BIP-85 entropy, as
  a secp256k1 secret key, Bech32 `nsec` per NIP-19; the npub is its
  x-only public key, Bech32 `npub`. Identity and account are each 1 or
  more, since the BIP reserves 0, and start at 1/1, filled in.
- It is a seventh application on Faraday's BIP-85 screen, and a Tools
  tile, **Nostr key**, opens that screen with Nostr chosen, on the
  device only: Faraday Desktop has neither, on any network, and nor
  does the demo. Desktop runs online and the derivation ignores the
  network, so a test network would show a real Nostr key there; the
  demo's public seeds would give keys anyone can derive. The screen
  shows the path, the npub with Show as QR and PNG, and the nsec,
  shown as BIP-85's WIF and extended private key are, with no **Out
  unprotected**. The plain nsec is never a QR or a picture, in the
  tool, the vault or Files. **Find a pair** lists the npubs of
  identities 1 to 5 at account 1 and of accounts 1 to 5 at an
  identity, public values only, so a forgotten pair is found by its
  npub.
- **Save into {vault}** writes an Entry: the npub as its username, the
  nsec as its password, the path and the key's fingerprint in its
  notes. No new record type. The seed's words and the two numbers make
  the nsec again without the vault; the numbers are written down with
  the seed's backup. An Entry holding an nsec is not exported for
  KeePass.
- **Export encrypted** writes the nsec as NIP-49 `ncryptsec1…` under a
  password typed twice, of at least 12 characters (scrypt LOG_N 16,
  XChaCha20-Poly1305, key-security byte `0x02`), as text, a QR, a PNG
  and a sealed file For the stick. It is the only way the nsec leaves
  the device, apart from the sealed vault file.
- Nothing that can leave the device carries the key's fingerprint or
  an identity or account number: files and labels are named by the
  npub's first 13 characters. The fingerprint stays inside the vault.
  It is in every PSBT and descriptor a person shares, so on a file
  name it would tie the Nostr identity to the wallet, and two
  identities to each other.
- A saved Nostr entry (an Entry whose password is an nsec and whose
  username is its npub) offers Show as QR and PNG for the npub and
  Export encrypted on its page, so a saved key goes to a phone without
  being derived again. No Entry gains a QR of any other field than
  its TOTP secret's (F28).
- A file holding an nsec is a secret kind with no code and no way to
  the Outbox. Faraday reads no ncryptsec it did not make (recommended,
  for the owner to confirm).
- Faraday Companion refuses `nsec1` and `ncryptsec1`, as it refuses
  other private keys and sealed files, in the same batch.
- The derivation is Faraday's (`faraday-core/src/nostr.rs`, the reading
  rules in `faraday-intake/src/nostr.rs`), over `osk-bip`'s public key
  derivation; `core/` is not edited. No new crate enters the graph.
- Signing Nostr events is a later item.
- Specified, not scheduled: it is built when the owner says.

### F28 QR codes for public values wherever another device reads them, and for the TOTP secret (2026-10-14)

**Why.** The owner's ask: "offer QR codes wherever it would be
convenient for users inside Faraday". Faraday reaches the world by QR
and by stick, and many public files already have **Show as QR** and
**PNG** (`docs/QR.md` §4 item 4). An audit of the screens found public
values a person moves to another device by retyping them: receive
addresses, Explore's keys, the Lightning node id, a recovery sheet's
descriptor. An entry's TOTP secret has one destination, an
authenticator app, which takes it by scanning.

**Decision** (owner, 2026-10-14; the spec is `docs/QR.md` §7).

- A public value gets **Show as QR** where another device or app reads
  it, and **PNG** as well where it is one static code given out or kept
  more than once. A value read by eye (fingerprints, hashes, GUIDs) or
  taken from a stick (firmware enrolment) gets neither.
- Five places gain a code: a receive address, wherever a wallet's
  receive addresses are shown (the wallet card, Create's and Restore's
  Check, the chart sheet, the family pages, Vanity address's result);
  Explore's extended key, and its address and public key; the Lightning
  node id; a recovery sheet's descriptor in Vault contents.
- A receive address of a wallet with a key made here and not yet backed
  up is neither shown nor encoded; "Back up before you receive" stands
  in its place until the hold is released (F31).
- A receive address has no PNG: a picture kept invites reuse. The node
  id and a recovery sheet's descriptor have one, For the stick.
- The code holds the value exactly as the screen prints it; on Faraday
  Desktop's mainnet demo an address keeps its broken checksum (F20).
- A secret gets no QR. It reaches a code only as its encrypted form,
  through the secret sheet and Files, or as a seed's SeedQR in the
  backup's checklist. The exceptions are those whose usual destination
  takes the secret by scanning and reads no encrypted form of it:
  SeedQR, the Starter's BitLocker recovery key (STARTER D18), and now
  an entry's TOTP secret. (F33 removed the BitLocker key: the
  exceptions are SeedQR and the TOTP secret.)
- The TOTP secret's code is the stored `otpauth://` URI, opened by a
  hold under the **TOTP secret** row, on the QR sheet marked Secret,
  with no PNG and no Outbox file. F27's line on Entry QRs names it.
- BIP-85 shows no code of any output; no SeedQR of child words.
- Every screen touched is Faraday's own (`faraday-core`), the same on
  the stick, the Pi and Faraday Desktop. OpenSigner's files and
  strings are not edited.
- Specified, not scheduled: it is built when the owner says, as one
  batch (`docs/QR.md` §7.4, Q1).

### F29 A copy by hand is checked by its form: typed words, or a scan of a SeedQR (2026-10-14)

**Why.** The backup's copy item drew the SeedQR grid for every seed and,
whenever a camera was listed, could be checked only by scanning that
grid, though the presets copy the words alone. With no camera the
words' four-digit numbers were typed back, compared digit by digit as
they were typed, with the seed allowed on screen above the field.

**Decision** (owner, 2026-10-14; the spec is `docs/NEW-WALLET.md`
§14.7).

- A words copy is checked by typing the whole seed back as words, with
  the seed not on screen. Each word may be typed whole, as a prefix
  only one word starts with, or as its four-digit number.
- A SeedQR copy is checked by scanning the drawn grid, as before; with
  no camera, by typed words. A seed copied both ways needs both checks.
- The grid is drawn on the copy item, and printed on the blank
  template, only when the plan copies that seed as a SeedQR.
- The typed check uses Add a key's word entry, takes the seed's own
  list and word count, shows the word being typed and masks the words
  taken, compares once the last word is in, and names the first word
  that differs; the words before it stay and typing resumes there.
- The BIP-39 passphrase is not asked: it has its own check on the
  chart, by the fingerprint it makes.
- A SeedQR copy can be dropped for the words: in the plan, by **Keep
  the words only** on its item, and at the item's foot after three
  scans without a match. The plan is edited directly; if the seed's
  words were already checked the item is done. The grid drawn so far
  stays on the map, marked drawn in part, until destroyed.
- Each seed here keeps its own copy item. The typed words are wiped on
  a match, on leaving the check, and on a lock.
- Specified, not built.

### F30 No released Faraday Desktop; the demo makes no secrets (2026-10-14)

**Why.** Faraday Desktop runs on an online computer, and released, it
offered every flow there: GPG and Secure Boot keys, BIP-85 passwords,
shares of a seed. The website runs the same app in a browser and is a
better way to try Faraday than a download. F20 kept funded seeds off
the demo's mainnet, but the demo still made keys and passwords, shown
on an online computer before being dropped or kept.

**Decision** (owner, 2026-10-14; the spec is `docs/DEMO.md`).

- Releases stop publishing Faraday Desktop:
  `faraday-desktop-X.Y.Z-linux-x86_64`,
  `faraday-desktop-X.Y.Z-windows-x86_64.exe`, and the planned macOS
  file. Faraday Starter drops Try the desktop app and the desktop app in
  Install, and its payload carries the images only.
- The desktop app's code and builds stay, supported for developers and
  unpublished; the README says how to build it, in one short section.
  A developer build starts on mainnet as the demo and keeps F20's test
  networks with any seed. It is never released.
- The website demo, the desktop shell compiled to WebAssembly in its
  own repository, is the way to try Faraday.
- The demo (a session set up by `Session::set_demo` and on mainnet)
  opens every tab and screen, and every action that would make a secret
  is disabled: SLIP-39, Seed XOR and codex32 shares, GPG, Secure Boot
  and APK keys, every BIP-85 output, a FROST deal, a vanity passphrase,
  a Lightning node's private key and a scan key leaving Faraday. Bitcoin use with the
  eleven public seeds stays fully open, as F20 has it.
- New key keeps every source, dice, coins, cards, hex, camera, the
  generator and a mix, and its result is replaced by the next public
  seed, as F20 has it. Generate with dice for a vault passphrase keeps
  its rolls, and its words are replaced by the demo's public passphrase,
  a constant beside the public seeds. Neither result is kept.
- A disabled action is DESIGN §4.11's dimmed control with no reason
  line, and does nothing when pressed; the demo's Learn page, first in
  the info button's list on those screens, says what the demo does not
  make. This is an exception to `docs/FLOWS.md` decisions 8 and 10, for
  these actions in the demo only.
- The decisions that offer a flow "in Faraday Desktop" (F11, F12,
  F13, F25, F26) now apply to developer builds and the demo; `docs/DEMO.md` §8 and §11 list each change. F27 stands.
- Decided with it (owner, 2026-10-14, `docs/DEMO.md` §13): typing a
  password or TOTP secret into a vault Entry stays open; the LND cipher
  seed row of Lightning node key is dimmed in the demo; a desktop app
  installed by an earlier Starter is left in place, and the README says
  how to remove it; the desktop app on macOS (MAC batch D) is dropped
  from the plan, and developers can still build it.
- Built (2026-10-15, before 0.1.3): `docs/DEMO.md` §12's batches DM1,
  DM2 and DM3, with the README, `docs/STARTER.md` and the demo's Learn
  page. DM4, the website's, is in its own repository.

Numbering: F30 was the next free number on `main` on 2026-10-14. Work in
another worktree may add a decision; whichever is committed second takes
the next number, and `docs/DEMO.md` follows it.

### F31 Decide where a key will be kept, then make it: one backup flow for every secret Faraday makes (2026-10-14)

**Why.** A Bitcoin wallet has its backup checklist (F6); nothing else
Faraday makes had one, and the checklist itself left every stick write
to the stick visit. A stick is read only with nothing secret open. A key
made first would have to be locked into some vault before the stick
holding the vault it belongs in could be read, and a new seed held only
in memory would be wiped by that lock. APK batch 1 counted two sticks by
FAT volume serial, which sticks written from one image can share.

**Decision** (owner, 2026-10-14; the spec is `docs/BACKUP-FLOW.md`).

- One flow for the APK signing key, GPG keys, the Secure Boot keys,
  Nostr keys and a Bitcoin wallet's keys, in this order: where it will
  be kept (the plan: places, which keep a stick, which vault each stick
  holds); bring in each stick the plan names, one at a time, with
  nothing secret open, checked for use (FAT32, room, writable, not a
  boot partition), its vaults read, its place confirmed by a tap or,
  where two sticks read alike, a hold; unlock the existing vaults and
  make the new ones, each with its own passphrase; make the key straight
  into every vault the plan keeps it in; lock once; write each stick by
  place, "Insert the stick for Vault 1 (Place 1)", refusing a stick that
  does not read as it did; done, then unlock. Restore takes the same
  steps with its seed typed or scanned. A Bitcoin wallet's order is the
  next bullet.
- **A Bitcoin wallet asks about vaults first and plans after its keys**
  (owner, 2026-10-15; it replaces the card that asked where the keys
  would be kept before any key existed). Make a new wallet and Restore
  ask, after the kind, "Will you use vaults as part of this backup?". No
  goes straight to the keys. Yes asks how many vaults (one or more, no
  upper bound) and sets each up, one at a time, inside the flow: its
  stick is inserted with nothing secret open and checked; a stick that
  holds vaults offers which existing vault this is or a new vault beside
  them, an empty one a new vault; every stick is read before any vault
  is unlocked; then the existing vaults are unlocked and the new ones
  made. The keys are made into memory. The plan (which keys go into
  which vault, the places, the paper forms, SeedQR, files, the
  passphrase, the wallet description) is Back up's, after the keys, with
  the vaults set up as its vaults; a vault the plan keeps at more places
  than the stick it was set up on is written to new sticks at the write
  step, and a vault the plan wants that was not set up takes the extra
  round. Nothing about places, key-to-vault ticks or paper is asked
  before the keys. The checklist and Write to sticks are as built in
  B1b.
- The plan decides what each stick holds before any stick is inserted.
  An empty stick holds only what the plan puts there; a vault has one
  copy per place the plan keeps it at, with no upper bound. A vault's
  later copy is refused on a stick holding an earlier one and needs the
  hold "This is a second stick" (or "another stick"). Faraday cannot prove two sticks are
  different; that is the extent of the check.
- The flow is driven by the plan and narrows nothing it can say: every
  form (vaults on sticks, paper, steel, shares, sheets, a stick of
  files, watch-only software, any mix). Only places whose plan includes
  a stick are asked for; a plan with no stick has no stick step and no
  lock for writing. Paper keeps the checklist's items and F29's check.
- Places: two or more for a key that cannot be made again from seed
  words (APK, GPG, Secure Boot), the count never below two; one or more
  for a wallet; any number, none included, for a Nostr key. Places are
  added or removed at any time; the check lines inform and do not block.
- A wallet whose secrets all sit at one place, including a seed and its
  passphrase at the same place, shows under its check lines, in `WARN`,
  "Every secret of this wallet is at {place}. Separate places are
  strongly recommended." A single key with no passphrase also shows "A
  passphrase kept at another place than the seed is recommended." with
  **Add a passphrase**. A passphrase's default place keeps no copy of
  its seed in any form. Nothing is forced.
- Signing waits until every place the plan gives the key is written,
  for the keys that cannot be made again (APK, GPG, Secure Boot). The
  owner's reason, which departs from following GnuPG's behaviour for
  GPG: a key that cannot be made again, whose revocation is kept in the
  same vault, must exist in two places before its signatures are out. A
  Nostr key waits for nothing. A Bitcoin key keeps F6's rule, the hold
  moving to the flow's state at the lock. A key in a vault unlocked from
  a stick signs at once, so existing keys are not held.
- The person stays in the flow: the stick visit page is never shown, at
  any step. The Bitcoin checklist's every stick write (vault copies,
  public files, seed files, the PDFs to print) happens inside it, and
  after the checklist nothing waits For the stick.
- The flow's state (the plan, what each stick read, the step, the
  written copies' SHA-256) is not secret and is kept across a lock under
  `/run/faraday`; it is gone at power-off. The vault carries no mark of
  the flow. A power-off between writes leaves the copies already
  written, and a key unlocked from one of them signs at once: the
  person chose the places, and can write the rest or stop there.
- No cap on places or copies: a vault, or any one key, is kept at as
  many places as the person adds, one copy per place. A copy of the same
  vault not seen this session is replaced only after a hold. Writes made
  on the stick visit page do not count toward the flow.
- Each key's account key file and BIP 129 key record are among the
  public files. Public files may share a stick with a vault copy, as the
  person chooses. The blank template is printed from Faraday Starter;
  no template is written to a stick.
- **Change the plan** is at every step. After the key is made, a place
  with a new vault on an empty stick, or another copy of a planned
  vault, needs no extra round; a stick holding an existing vault the key
  should go into takes one guided round inside the flow (lock, read the
  stick, unlock both vaults, copy the key across, lock). A place removed
  before its write drops its vault where the vault was new; one removed
  after its write stays on the map until destroyed. The minimums hold.
- F25's two sticks of different FAT serials are replaced by this flow.
- Built: batch B1a (everything but the Bitcoin wallet) for the APK key,
  GPG keys and the Secure Boot keys, and batch B1b for the Bitcoin
  wallet and Restore, with the choices `docs/BACKUP-FLOW.md` §16
  records; B1b leaves out the two choices for a key kept only on paper
  before an extra round (the owner's decision is asked for there) and
  the Nostr key's part.
- No coins arrive at a key before its backup (owner, 2026-10-15). A new
  key lives only in memory from Keys until Back up saves it, and the
  idle lock wipes it after its warning. So the idle lock is unchanged,
  and every receive address of a wallet holding such a key, and any code
  of one, is not shown until the backup is done (`docs/NEW-WALLET.md`
  §14.3, F28).

### F32 GPG follows GnuPG's behaviour (2026-10-14)

**Why.** Two reviews of the GPG feature found places where Faraday did
what GnuPG does not, or was silent where GnuPG speaks. The owner's rule
(2026-10-14): for each issue, copy what GnuPG does as closely as
possible, and where its behaviour is in doubt, check it with the real
`gpg` rather than guess.

**Decision** (owner, 2026-10-14). Each behaviour below was checked with
GnuPG 2.4.8 in a throwaway `GNUPGHOME`, with `--faked-system-time` for
every time; the tests in `faraday/faraday-pgp/tests/gnupg.rs` and
`faraday/faraday-core/tests/gpg.rs` run `gpg` itself and fail where it is
missing, unless `FARADAY_SKIP_GNUPG=1`.

- *Revocation certificate kept with the key.* GnuPG 2.1 and later write
  one at key generation to `openpgp-revocs.d/<FPR>.rev`, beside the
  keyring, not to where files are published. Faraday keeps one in the
  key's vault record (`docs/VAULT.md` §7, type 7, field 8) and writes
  nothing to the Outbox. A key with none makes one when its page is
  first opened with the clock known, or when **Export the kept
  revocation** is pressed. After Make, the result says where it is kept.
- *The `.rev` file.* `{name}-{FPR}.rev`, laid out as GnuPG's: the
  sentence "This is a revocation certificate for the OpenPGP key:", the
  key as `gpg --list-keys` prints it (`pub   ed25519 2026-05-28 [C]
  [expires: …]`, the fingerprint on its own line, `uid` lines; dates UTC,
  checked with `TZ` set to three zones), GnuPG's paragraphs, then the
  armour with `Comment: This is a revocation certificate` and a colon
  before the first dashes. `gpg --import` takes nothing from it until the
  colon is removed (checked: "no valid OpenPGP data found"; after, "revocation
  certificate imported"). It goes through the secret sheet and has no
  code, picture or Select all.
- ***Make a revocation*** (`--generate-revocation`). GnuPG's reasons by
  its menu numbers: 0 no reason, 1 compromised, 2 superseded, 3 no longer
  used (the packet codes are 0, 2, 1, 3; checked by reading the
  subpacket of a certificate made with menu 2), and an optional
  description. The new certificate goes out as a `.rev` file; the kept
  one is not replaced, as GnuPG keeps no copy of one it generates.
  **Hold to mark the key revoked** marks the record (field 9), as
  importing its own revocation marks a key in GnuPG.
- *Signing a file* (`--armor --detach-sign`). The output is
  `{file}.asc`. GnuPG in batch mode refuses to overwrite ("signing
  failed: File exists", checked), so Faraday does too, and so a
  signature never replaces the certificate when a file is named like it.
  The signature is made over exactly the bytes shown: the page lists each
  file's name, size and SHA-256 and the text of a small text file, and
  the ticks are keyed by name and hash, so a stick visit that replaces a
  file leaves it unticked. The page names the signing key as
  gpg-agent's pinentry does (checked by recording its prompt): user ID,
  key ID, creation date and the main key's ID. Signing takes a hold, as
  Secure Boot signing does. Changing the key, the category or the vault
  clears the page. The signature's code and picture follow the file by
  its hash.
- *Keys it will not sign with.* An expired or revoked key: GnuPG says
  "Unusable secret key" (checked with `-u`). A key made after the clock:
  "key … was created N seconds in the future (time warp or clock
  problem)", seconds below a day, then whole days (checked at 1, 59,
  86399, 86400, 172800 seconds and more).
- *Renewing* (`--quick-set-expire`). The term is asked, with the current
  one shown, and is never silently "never". The default for a new key is
  GnuPG's: `--quick-generate-key` with no expiry gives 3 years
  (checked: 94,608,000 seconds). A year is 365 days (checked: `2y`
  counted from the clock is 730 days). The new term is counted from the
  clock, and stored from the key's creation. GnuPG signs one second after
  the key's last self-signature when the clock is within 4 seconds of it
  and refuses ("Time conflict") from 5 seconds before it (checked at 1
  to 600 seconds); Faraday does the same. The record's certificate and
  Export change together.
- *The user ID* is whatever `gpg --quick-generate-key` takes: it trims
  the string and refuses only an empty one ("Invalid user ID"). Checked
  with no name, a name only, an address only, an address with no `@`
  domain, `<>`, spaces, a comment, colons, a newline, a tab, a control
  character, non-ASCII letters and strings of 255, 256 and 300
  characters; all were accepted. Faraday trims and refuses empty.
- *Backup.* GnuPG has no backup gate. Here the owner departs from
  GnuPG (F31): a GPG key made in Faraday cannot be made again from seed
  words, and its revocation is kept in the same vault, so signing waits
  until the shared backup flow has written every place its plan gives
  it (two or more). A key in a vault unlocked from a stick signs at once.
- *Faraday Desktop* offers GPG as it does today; it is an ordinary
  online computer, as GnuPG runs on.
- *Paperkey.* Computed only while it is held to show, and wiped after;
  the seeds, secret MPIs and the paperkey buffer are wiped. Checked
  by a test that compares it with `paperkey`'s output, with seeds that begin
  with zero bytes.
- *Reading armour* is F22's amended list.
- *Not changed.* Hash SHA-512 for signatures, as GnuPG 2.4 chooses for
  Ed25519; primary key flags certify only, with the signing subkey.

### F33 Secure Boot back on is the step; no BitLocker recovery key in the Starter or the Companion (2026-10-14)

**Why.** F17 and F21 kept a BitLocker step in Faraday Starter (Show the
recovery key, its QR code and Save, an extra-safe suspension with a
marker, a logon reminder and Resume) and a store for the key in Faraday
Companion, with a text format shared between them (STARTER batch W).
BitLocker checks Secure Boot only when Windows starts (F17): Windows
started with Secure Boot back on asks for nothing. The owner's reasons:
turning Secure Boot back on is simple and it is necessary. People must
not leave Secure Boot off, and must not turn BitLocker off to avoid the
prompt; making "Turn Secure Boot back on" the step, with no
recovery-key machinery beside it, ensures that. Offering to copy a
recovery key onto a phone is irresponsible.

**Decision** (owner, 2026-10-14; the spec is `docs/STARTER.md` §6,
`docs/COMPANION.md` §4.6 and §4.7, `docs/STARTER.md` §7.5).

- *Removed from Faraday Starter:* the BitLocker page and Home's
  BitLocker item (**Back up the BitLocker recovery key**, its note and
  states); **Show the recovery key**, its QR code, "Or take a photo of
  this screen.", Save and the key file; **Extra safe: suspend
  BitLocker**, the two groups typed back, the suspension, the marker,
  the logon reminder (`--remind` and its Run entry) and **Resume
  BitLocker**; Uninstall's BitLocker paragraph; the elevated
  `--bitlocker` verbs and the WMI reading of BitLocker; the drive's
  BitLocker line on This computer; the Start page's **Back up the
  recovery key** row; the TV and remote-session hold on showing the key.
  The one explanation F21 allowed on a working screen goes with the
  note, so `CLAUDE.md`'s rule holds without exception.
- *Removed from Faraday Companion:* **BitLocker keys** (COMPANION
  §4.6): the Home row, the store's keys, the Record, **Back in
  Windows** and its sheet, **Delete the key**, the scanner's route for
  recovery-key text, and the Learn page's section on the key.
- *Removed between them:* the recovery-key text format
  (`faraday_qr::bitlocker`), written by the Starter and read by the
  Companion (STARTER batch W, COMPANION batch B).
- *Removed from the guides:* the Learn page "BitLocker and starting
  from a stick" (`bitlocker`), the Companion's **BitLocker first**, and
  every line that offers a backup of the key or a suspension.
- *Every string, Learn page and test that exists only for these.*
  `starter_` and `companion_` strings used by nothing else go from
  `faraday/faraday-strings/src/en.rs`; tests that state the removed
  behaviour go (`faraday-starter/tests/bitlocker.rs`,
  `unreadable_bitlocker.rs`, the shell's `tests/bitlocker.rs` and its
  recorded WMI answers, `faraday-companion/tests/bitlocker.rs`,
  `faraday-qr/tests/bitlocker.rs`), and tests that touch them in
  passing are changed to the rule they now encode.
- *In their place:* every guide's last numbered step is **Turn Secure
  Boot back on**, in the BIOS, before Windows starts; the Companion
  ticks it off like the others (COMPANION §4.7). Faraday Starter's Home
  lists **Turn Secure Boot back on** whenever Secure Boot is off.
- *Windows' Use a device only with Secure Boot reported off* (owner,
  2026-10-14: decided). **Restart to choose a device** and, on Windows,
  **Restart from the stick** are shown only when Windows reports Secure
  Boot off at that moment (`UEFISecureBootEnabled`, read without
  elevation, as the Starter's This computer reads it), and hidden when
  it reads on or cannot be read. The stick then starts with Secure Boot
  already off, and the power-off line still applies. The guides' lines
  on it stay, conditioned the same way. Linux keeps **Restart from the
  stick** (STARTER §3.1).
- *At the stick's power-off* (STARTER §7.5): on a PC stick that started
  with Secure Boot off, or with its state not read, the power-off sheet
  leads with "Turn Secure Boot back on in the BIOS before starting
  Windows." in the warning tone, above the guide's code. It needs no
  press of its own: **Power off** is the press. The Pi and Faraday
  Desktop never show it; a Mac follows `docs/MAC.md` §3.2. It is one
  line of instruction for the action the person takes next, at the
  moment of the press, as F1's line of fact is; the reason stays in
  Learn.
- *The risk, said plainly.* A person who forgets and starts Windows
  with Secure Boot still off is asked for the BitLocker recovery key,
  and Faraday offers nothing for it.
- *Where Windows keeps the key* (owner, 2026-10-14: decided). The
  Windows guide and each maker page carry one line of fact: for a
  computer set up with a Microsoft account, the key is on the
  account's recovery-key page, aka.ms/myrecoverykey; otherwise it is
  wherever it was saved when BitLocker was set up. It is a pointer, not
  a flow: nothing is scanned, saved or copied.
- *Kept:* another encryption product (VeraCrypt or a vendor's) still
  closes the firmware steps until its rescue disk or password is
  confirmed, and the Linux guide's line for a disk unlocked by the TPM
  stays (STARTER §6). Neither shows or keeps a key.
- *Keys already kept in a Companion debug build.* Nothing migrates: no
  Companion was released. The store's format moves to version 2,
  without keys; a version 1 store is deleted, with its wrapped key and
  its counts, at the first start of the new build.
- *Supersedes:* F11's bullet on the guard; F12's bullets on the
  guard's order, the key on a phone and Resume; F15's BitLocker parts
  (keeping keys, their deletion, their unlock); F17 whole; F21 whole;
  F28's exception for the BitLocker key (the exceptions are SeedQR and
  the TOTP secret); STARTER D7, D16, D17, D18 and D19; COMPANION D5,
  and D4 for BitLocker keys. `docs/BITLOCKER-ACCEPTANCE.md` is
  superseded (its header says what remains).
- *No Resume* (owner, 2026-10-14: decided). The suspension is removed
  entirely; no release keeps Resume BitLocker or reads the 0.1.2
  marker. There are no users yet (STARTER §6.3).
- *The idle power-off is unchanged* (owner, 2026-10-14: decided). It is
  not withheld when Secure Boot is off. It shows no sheet, so its
  reminder is the guides' last step (STARTER §7.5).
- *Batch S* (owner, 2026-10-14: decided). When S is scheduled is
  decided then. When it enrols Faraday's key, its guide tells the
  person to have the recovery key ready, with the same pointer; nothing
  more (STARTER §7.7).
- *The Companion refuses a recovery key* scanned, pasted or shared in,
  as it refuses a seed (owner, 2026-10-14: decided; COMPANION §4.6).
- Specified, not built: STARTER §6.5's batches F33a to F33d.

### F34 Faraday Companion is dropped (2026-10-15)

**Why.** The Companion (F15) was to take the device's codes to a phone
and keep what a phone should keep. The owner dropped it, at least for
the time being: the Starter's guides carry the start-from-stick steps,
with a sheet to write them down; the website's guide pages open on any
phone from the Starter's QR code (STARTER §7.4); and QR transfer happens
between the device and the wallet program directly. Nothing was
released in it.

**Decision** (owner, 2026-10-15).

- *Dropped:* the app, `faraday-companion`, `faraday-companion-ffi`, the
  Android shell with its fastlane metadata, the container that built
  its APK, its two lints, `docs/COMPANION.md`, its strings, its Learn
  page, the APK build in `faraday.just` and `stage-release.sh
  --companion`, and what in `faraday-intake` only it used (the secret
  classifier). The wordmark's COMPANION suffix goes with them.
- *Supersedes:* F15 whole; F18's Companion half (the sketches stay in
  the Starter); the Companion's parts of F25 (its name and package filled
  in), F27's refusals (NOSTR §6.7) and F33 (its BitLocker store).
- *Stays:* **Sign an APK** (F25, `docs/APK-SIGNING.md`): Faraday still
  makes, keeps and uses an APK signing key for any app. Its name and
  package fields start empty, where the Companion's name and package
  were filled in. The release carries no APK.
- *Bringing it back* is a revert. The last commit that has the
  Companion is `d870d52`.
- Tracker: #113 does the removal; it closes #16, #18, #76 and #111
  (#111, reserved for this number before the decision, closed as not
  planned).

### F35 A wallet's backup plan is asked place by place (2026-10-15)

**Why.** Back up asked where each kind of thing goes: "The seeds go"
(words, SeedQR, vaults, a file), with a section per vault ticking its
seeds, then "The wallet description goes". A person thinks of a backup
as places: what goes in the drawer at home, what goes in the safe
deposit box. Asked by kind, they had to work out each place's contents
from several lists.

**Decision** (owner, 2026-10-15; the spec is `docs/BACKUP-FLOW.md`
§17).

- The plan's cards are: Start from (the presets); the number of places;
  a card per place; the wallet description, whole or in shares with
  the slider of keys left off; where the description goes, place by
  place; the software; and the whole map, which the person approves
  with Make the checklist.
- A place's card is a list of what it keeps: seed words by hand, SeedQR
  by hand, each vault, a seed as a file, a passphrase. Ticking a row
  opens a popup that asks which seeds; for a vault, which seeds go into
  that vault. A vault kept at two places is one vault with two sticks.
- Where the description goes is chosen per place: on paper, as a file,
  or in a vault whose stick that place keeps; with shares, which shares
  each place keeps.
- A file is kept at a place, on a stick there, and counts in the checks
  as that place's.
- What goes into a vault is the person's choice, a share included
  (owner, 2026-10-15): a share is kept in a vault as a new record, type
  13 (VAULT §7, BACKUP-FLOW §17.6).
- One wallet, one checksum (owner, 2026-10-15): every wallet Faraday
  reads is kept with its hardened steps written `h`, as Faraday writes
  them when it makes a wallet, whether it came with `h` or `'` (BIP-380
  allows both, and the checksum covers the text). So a wallet rebuilt
  from shares, read from a multisig config or typed in shows the same
  descriptor and checksum as when it was made.
- Specified, not built: BACKUP-FLOW §17.

### F36 Numbered stick pills on every device (2026-10-15)

**Why.** A backup asks for several sticks in turn. On a device a person
holding a stick cannot tell from the stick which of the backup's sticks
it is, and Faraday Desktop's pills existed only on the desktop.

**Decision** (owner, 2026-10-15; the spec is `docs/BACKUP-FLOW.md` §18).

- The numbered pills are drawn on the PC stick and the Pi too, while a
  backup uses sticks: a pill per stick, lit only when the stick in is
  recognised as that one, edged when it is the one wanted now; no press
  on a device.
- The boot stick is Stick 1, still called Boot stick; a backup's sticks
  are numbered from 2, on every device.
- When a stick is first taken for its place, its card says "Mark it as
  Stick 2": the person marks it as they like, so the number on screen
  and the stick in hand agree.
- The pills are larger: 28 high with 13-point text, 24 and 12 on the
  Pi's small panel.
- Specified, not built.


### F37 The backup plan is a board of places (2026-10-15)

**Why.** F35's place cards asked through tick lists and popups, which
the owner found confusing; a person thinks of a backup as things put in
places, and wants to see it that way.

**Decision** (owner, 2026-10-15; the spec is `docs/BACKUP-FLOW.md` §19).

- A card for each place, its name at the top; in each, its sticks (each
  with a vault on it or not, and files), its paper; beside the cards a
  list of the seeds, passphrases, the description, its shares (the
  partial descriptions, always offered for a multisig, with the keys
  left off each share beside them), the vaults and watch-only software. The person drags things into places,
  or taps to place them.
- A place may keep more than one stick; a stick carries at most one
  vault, and may carry files beside it.
- Each thing at a place has its forms: on paper words and SeedQR; on a
  stick a words text file and a SeedQR picture, the description as text,
  QR picture and the software's file.
- A vault keeps records only (a key, its passphrase, the wallet, a
  share); its contents screen shows a key's SeedQR and a wallet's QR
  and saves them to a stick as files when asked.
- Watch-only software is a place: sticks, and the description scanned
  in.
- The board starts empty, with one place; + and × add and remove places.
  Presets stay as a way to fill it.
- Unwise plans are warned about, never refused; only a seed kept
  nowhere holds the checklist.
- Desktop and the PC stick at any width: a narrow window stacks the
  places in one scrolling column, the list in a sheet, tap to place.
  Not the Pi for now.
- A passphrase may be kept in a vault without its seed (a new vault
  record), and every form the board keeps is read back by Restore;
  each file is read back from its stick right after it is written.
- After the lock, paper and files stay as made (§17.8); vaults change.
- A vault the plan makes with no stick, or holding nothing, still holds
  Make the checklist beside a seed kept nowhere (owner, 2026-10-16).
- Built: F1, the model (2026-10-16); F2, the board (F2a in Back up,
  F2b in the guided flow and a narrow window, with BACKUP-FLOW §19.14's
  boxes and layout; 2026-10-16). F3 not built.

### F38 Access layers (2026-10-16)

*Withdrawn (owner, 2026-10-19): the access layers were removed from Faraday; a backup is planned on the board from The standard way (`docs/BACKUP-FLOW.md` §19.6).*

**Why.** A backup may serve several access layers, each reaching the
wallet by its own route with its own knowledge; arranging places by
hand so that each layer can spend and no stranger can is hard.

**Decision** (owner, 2026-10-16; the spec is `docs/BACKUP-FLOW.md` §20).

- An access layer is a list of what it is able to spend with: places,
  and the vault passphrases and passphrases known to it.
- The person sets the layers first; Faraday works out the board so that
  every layer can spend, making as large as it can the number of places
  a stranger who knows nothing can find and still not spend.
- No layers that must not spend; the stranger is the rule against.
- Conflicts are explained, each with what would resolve it.
- Each layer's instructions are shown on screen to be copied by hand,
  never printed or written to a file.
- A row at the top of the board shows one view at a time (All, each
  layer, the stranger), with that view's checks under the board: for a
  layer, whether it can spend, the places it needs and the places it
  can lose.
- Layers and places are named by number ("Access layer 1", "Place 1");
  the person may rename both in the app, while a vault is open, and the
  names are kept only inside the vaults (as place names are, F35).
- A layer has one or more routes, each "any k of" its places; Faraday
  offers each layer a paper route (optional, with a warning without
  one); no balance-only layers; no passphrase is insisted on.
- Back up opens on a short set of questions, one card each; Faraday
  works the board out from the answers and shows the result in words.
- Create a wallet starts with the questions and suggests the wallet
  from the answers (single-key and standard multisig quorums only, for
  now); **I know what I want** chooses the kind directly.
- **The standard way** skips the questions and the board everywhere: the
  seeds to write down and the description to show, save or print, as
  a plain checklist; F31's hold lifts when each seed's copy is checked.
- Specified, not built; after F37's F3.

### F39 One page colour; steps are sections, not cards (2026-10-16)

**Why.** White step cards on a grey-blue page put two colours under
everything, and the board's boxes had to stand out against both; the
page looked busy.

**Decision** (owner, 2026-10-16; `docs/DESIGN.md`).

- Every page's working area is one colour, the theme's surface (white
  in the light theme). The sidebar keeps its own shade.
- A step flow's steps are flat sections divided by a thin rule, not
  cards: no fill and no box, the step's number, title and value on its
  row as before.
- Applies across the app: Create, New key, Restore, Back up, the
  checklist and every flow drawn with step cards; and to the right-hand
  column, divided from the page by the same rule.
- Built with BACKUP-FLOW §19.15 (F2c, 2026-10-16).


### F40 The plan is what the places keep (2026-10-16)

**Why.** In the owner's test of the board (F37), a vault whose places
had been removed still stood in The plan with what it held, and
held Make the checklist, until its chip was taken off the list; and
**+ stick** did nothing when pressed.

**Decision** (owner, 2026-10-16; `docs/BACKUP-FLOW.md` §19.16).

- The plan is what the places keep. A vault no place keeps a stick of
  stays on the board's list, with what it holds, and is left out of The
  plan, the checklist and the checks until it is put on a place again,
  its chip dim on the list. A vault that exists already, at no place,
  is warned of but holds nothing back; its × takes it out of the plan
  and leaves it dim on the list, to be put back.
- A press on **+ stick** adds an empty stick at the place, the board's
  until something is dropped on it; while a chip it takes is held,
  **+ stick** is drawn with a solid edge.
- No line of explanation on the board: the box does what its label says.

### F41 A flow in progress is always one press away (2026-10-16)

**Why.** Leaving a flow for Settings left no visible way back to it,
though its state was kept.

**Decision** (owner, 2026-10-16; `docs/FLOWS.md`, Flows in progress).

- Every flow in progress has a row in the sidebar, under the tab it is
  opened from, with its name and open step; a press returns to it as
  it was left. The small Home leads with a tile for each.
- The same for every flow: wallet, vault and tool flows alike.

### F42 Restart beside Power off, with the BIOS key (2026-10-16)

**Why.** Turning Secure Boot back on means opening the BIOS at the next
start; the power sheet offered only Power off.

**Decision** (owner, 2026-10-16; `docs/FLOWS.md`, Restart).

- The power sheet on the PC stick and the Pi offers **Restart** beside
  **Power off**, with the same holds and the same wiping.
- Where the Secure Boot reminder shows, the sheet also shows this
  computer's **BIOS key**, from its maker page.
- Not **Restart into the BIOS**: that writes a firmware variable, and
  the stick writes nothing to the computer. Restart wipes as Power off
  does, since a restart keeps the memory powered.

### F43 The cursor moves by the screen's scale (2026-10-16)

**Why.** On the stick one mouse count moved the cursor one panel pixel,
so the mouse was slow on a 3200×1800 screen and fine on 1920×1080 and
below.

**Decision** (owner, 2026-10-16; `docs/MOTION.md` §3.2).

- One count moves the cursor by the whole-number scale the arrow is
  already drawn at, times `--pointer-speed`: a dense screen crosses like
  the desktop it is scaled to. A touchpad the same.
- Still no acceleration. This replaces upstream's "one device unit is
  one pixel" (PLANNING §16.94) on the stick.
### F44 Add a key is the New key flow (2026-10-19)

**Why.** Add a key offered only boxes for words held; New key in Create
offered the length, the randomness and the rest. Two screens for one
job had drifted apart.

**Decision** (owner, 2026-10-19; `docs/NEW-WALLET.md` §15).

- Add a key and New key are one flow, opening on **Start**: Make a new
  key, Type the words, Scan a SeedQR, and Other forms (SLIP-39 made or
  typed, codex32, Seed XOR).
- A typed or scanned key takes a passphrase on its Key card; a new key
  on its own still has none (decision 4).
- The Entry screen stays for a key asked for by its fingerprint or a
  wallet's missing key.

### F45 Keys from a vault in Create; a new wallet from one you have (2026-10-19)

**Why.** A key in a vault had to be loaded on the vault's page before
Create would offer it, and no wallet could be made from the keys of one
already held.

**Decision** (owner, 2026-10-19; `docs/NEW-WALLET.md` §16; tracker
#149, #131 folded in).

- The Keys card lists each open vault's keys under its name; a press
  loads the key and fills the slot. A locked vault is one Unlock row,
  returning to the card.
- A wallet's card offers **New wallet from this one**: Create filled
  from it, its account, script type, quorum and keys all changeable,
  starting at the next unused account.

### F46 Access layers: layers first, a paper route for every layer (2026-10-19)

*Withdrawn (owner, 2026-10-19): the access layers were removed from Faraday; a backup is planned on the board from The standard way (`docs/BACKUP-FLOW.md` §19.6).*

**Why.** The owner wants the backup's layers to choose the wallet, and
every layer able to get in on paper when sticks fail.

**Decision** (owner, 2026-10-19; `docs/BACKUP-FLOW.md` §20.12).

- The layers and the backup's parts come first; Faraday suggests the
  wallet's shape (single key, single key with a passphrase, 2 of 3,
  3 of 5) that meets every layer with the largest stranger margin.
- Every layer has a paper route; a placement without one is a
  conflict, not a warning.
- Layers share paper: one copy at a place serves every layer whose
  route holds it.
- A route says how many locations give full access and what kinds of
  backup each holds (seeds and their form, the description none, part
  or whole, a vault), every location of the route alike. From that
  Faraday suggests the wallet's shape (how many keys, how many spend);
  the person makes or picks every key with Create's existing ways
  (New key's dice and the rest, a loaded key, a vault's key); Faraday
  only places each key's seed (owner, 2026-10-19;
  `docs/BACKUP-FLOW.md` §20.13).

### F47 Three things to write down before the restart, and a sheet to print (2026-10-19)

**Why.** Once the computer restarts into the BIOS the Starter is no
longer on screen, and with the Companion dropped (F34) nothing else
carries what the person needs there.

**Decision** (owner, 2026-10-15 and 2026-10-19; `docs/STARTER.md`
§3.4, §6.7, §7.6; tracker #114).

- The Start page shows, before **Restart into the BIOS**, three rows:
  the **boot menu key**, the **BIOS key** (to get back in afterwards
  and turn Secure Boot on before Windows starts) and where the
  **Secure Boot setting** is in this maker's BIOS. They come from the
  maker table, which carries all three and is checked against the
  maker guides; a maker without a page reads Not known, and the sheet
  leaves a line to write on. An Intel Mac gets the Mac's equivalents:
  Option at power-on, and on a T2 Mac Command-R and the Startup
  Security Utility.
- **Print** writes the sheet as a PDF with Faraday's own PDF writer and
  opens it in the system's own viewer, where the person prints or
  saves it: offline, with no printing code or service of the Starter's
  own, on Windows, Linux and macOS, as Blank templates' Print (F31).
  The sheet holds the three things, the steps in order (Secure Boot
  off, start from the stick, Secure Boot back on before Windows
  starts) and the one line below; nothing else. The PDF carries no
  document information, path, user or computer name.
- **The keys used** (owner, 2026-10-19): the sheet also has a line each
  to circle the key that worked, so it is plain later which one did:
  "Boot menu key used (circle one)" and "BIOS key used (circle one)",
  each with this maker's key from the table first, then every other
  key the maker table names in a fixed order (Esc, the function keys
  by number, Fn with one, then the rest as the table has them), then
  "Other:" and a line to write on. A maker without a page gets the same
  lines without a key first; an Intel Mac's sheet has neither (its keys
  are fixed).
- **One sheet** (owner, 2026-10-19, with F49): on Windows with BitLocker
  on, a tick box beside Print, "Include the BitLocker recovery key",
  unticked by default; only when it is ticked does the sheet carry the
  system drive and its 48-digit recovery key. No second Print.
- **If the setting cannot be found**, the page, the sheet and every
  guide's firmware step say plainly: exit the BIOS without saving
  changes, and Windows starts as before.

### F48 The recovery key, framed plainly; no Microsoft account row (2026-10-19)

**Why.** The BitLocker page of 0.1.2 said the key was also kept in a
Microsoft account on every computer, including one signed in with a
local account (tracker #87). F33 removed that page; the guides'
pointer still opened on the Microsoft account.

**Decision** (owner, 2026-10-19; `docs/STARTER.md` §6.2, §6.7).

- The guides frame the key plainly: with Secure Boot off, Windows
  cannot start without it; it is not needed, since the last step turns
  Secure Boot back on before Windows starts; to see it anyway, where it
  is on the computer (Control Panel's BitLocker Drive Encryption, or
  `manage-bde` on Windows Home). The Microsoft account's page
  (aka.ms/myrecoverykey) is named only as a condition: "on a computer
  signed in with a Microsoft account".
- The Starter shows no Microsoft account row, so it reads no account
  type: there is nothing left to gate on it.
- Supersedes F33's wording of *Where Windows keeps the key*; the
  pointer is still a pointer, not a flow.

### F49 The Starter offers to suspend BitLocker and to print its recovery key (2026-10-19)

**Why.** F33 took the BitLocker step out of the Starter on the ground
that Windows started with Secure Boot back on asks for nothing. That
holds in the common case only: BitLocker unseals its key from the TPM
when the boot measurements (PCR 7 with Secure Boot on) match those it
sealed to, and a firmware that resets its Secure Boot keys or databases
when Secure Boot is turned off and on (Setup Mode, factory keys, a dbx
update lost), BitLocker sealed to other PCRs, or Windows started while
Secure Boot is still off, asks for the recovery key. Faraday never
touches the internal drive; that does not prevent it.

**Decision** (owner, 2026-10-19; `docs/STARTER.md` §6). Partly reverses
F33 and F48's "it is not needed".

- On Windows with BitLocker on the system drive, the Starter's Start
  page offers, never requires: **Temporarily suspend BitLocker** (for
  the next 3 restarts, after which Windows turns it back on by itself;
  `manage-bde -protectors -disable C: -RebootCount 3` or WMI's
  equivalent, elevated) and **Resume BitLocker** (turns it back on at
  once). Suspended, the state reads "BitLocker suspended · turns back on
  by itself after Windows starts 3 times", with Resume BitLocker beside
  it. The count is kept by Windows and counts the times Windows starts,
  not every restart: the BIOS and the stick do not run Windows. When
  protection resumes, by the count or by Resume, the key is sealed
  again to the computer's state at that moment, so a firmware whose
  Secure Boot keys changed does not ask for it. How Windows counts is to
  be confirmed on the Windows test virtual machine (tracker #157); the
  count stays 3 (owner).
- **The recovery key on the one sheet** (owner, 2026-10-19, amending
  "Print the recovery key"): no separate Print. The Start page's Print
  (F47) prints one sheet; beside it, a tick box "Include the BitLocker
  recovery key", unticked by default; only when it is ticked does the
  sheet carry the drive and its 48-digit key, read through the elevated
  copy. No QR code, no key file, no key shown for a phone or on screen
  (F33's reason stands). The PDF with the key is written to a temporary
  folder of its own, which the Starter overwrites and removes when it
  exits, and sweeps at its next start if a run left one.
- Neither is a step Faraday needs: the steps stay Secure Boot off,
  start from the stick, Secure Boot back on.
- The guides say plainly that Windows may ask for the recovery key
  even with Secure Boot back on, and that suspending BitLocker first
  avoids it.

### F50 OSKDATA keeps its drive letter; a computer's own files are ignored by name (2026-10-19)

**Why.** Windows gives a Faraday stick's data partition a drive letter,
and the GPT "no drive letter" attribute could take it away (STARTER
D11, open since 2026-10-12). With the letter, a person can put a PSBT on
the stick from Windows; Windows and a Mac then write their own files
there too (`System Volume Information`, `$RECYCLE.BIN`, `desktop.ini`,
`Thumbs.db`; `.Spotlight-V100`, `.fseventsd`, `.Trashes`, `.DS_Store`,
`._` AppleDouble files), which the boot import counted as other files
and a visit offered to copy.

**Decision** (owner, 2026-10-19, tracker #30; `docs/STARTER.md` §4.4,
D11). OSKDATA keeps its letter. Faraday reads what a person puts there
as any file on a stick; a name beginning with a dot, or one of
`System Volume Information`, `$RECYCLE.BIN`, `RECYCLER`, `desktop.ini`
and `Thumbs.db` ignoring case, is never listed, read or counted, by
name and never by the hidden or system attribute. A write leaves those
files where they are. Built as `faraday_files::foreign`, used by every
listing of a stick and of the desktop app's Downloads folder.

### F51 A flow is left by Cancel inside it (2026-10-19)

**Why.** A flow in progress has a sidebar row to return to (F41) and no
way to end it.

**Decision** (owner, 2026-10-19; `docs/FLOWS.md`, Flows in progress,
Cancel). **Cancel** at the top of every flow with a row ends it, asking
first only when something would be lost; keys, vaults and the Outbox
are never lost by it. Not a hold on the sidebar row: a press there only
returns to the flow.

### F52 At most one place per key; partial descriptions by one button (2026-10-19)

**Why.** Places beyond the wallet's keys add nothing the plan can check,
and handing a multisig description out as shares one by one is a puzzle
a person should not have to solve.

**Decision** (owner, 2026-10-19; `docs/BACKUP-FLOW.md` §19.19). A plan
has at most N places, N the wallet's keys; a single key has one place.
An extra copy is made outside the plan (copied between sticks, or by
hand) and the plan does not track it. Partial descriptions are added to
the whole backup by one button, **Add partial descriptions to this
backup**: Faraday chooses how many keys each share leaves off (M − 1
where it can) and which share goes to which place, so any M places
rebuild the description and, where it can, no place sees the balance
alone. Shares are no longer placed one by one, and the "Keys left off
each share" stepper goes. Problems are warnings, never refusals (§19.5).
