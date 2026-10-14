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
  confirms by typing part of it back (`docs/STARTER.md` §6).
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
  notarized, for the reason F11 gives for Windows.
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
  computer.
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
  across a lock. `faraday-pgp` reads the armor and the packet headers
  only (`PLAN.md` §4.4).
- A revocation certificate is one of them. `PLAN.md` §5.2 already lists
  revocation certificates in the Outbox as public. GnuPG advises keeping
  one private, because whoever holds it can revoke the key; the owner
  judged it public.
- Anything else stays as it was: a private key block, text around the
  block, armor that does not read, or any secret-key packet inside is
  text or a file Faraday does not read, which may be a secret.
