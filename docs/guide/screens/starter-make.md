# Faraday Starter: Make

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-starter/src/draw/home.rs`, `faraday/faraday-starter/src/draw.rs`, `faraday/faraday-starter/src/draw/boot_file.rs`, `faraday/faraday-starter/src/lib.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Starter page:** `Make`, `Which`, `Disks`, `Mode`, `Confirm`, `Writing`, `Done`
**Group:** Starter

**Opened from:** **Make** on the rail or the bar. Its steps open from
**Make a Faraday stick** (on Make, on Home's checklist, or as Home's
card button) and **Make a Pi card** (on Make).

Make writes Faraday onto a USB stick, or onto an SD card for a
Raspberry Pi, one step a page: **For which computer?** (only when the
Starter carries two stick images), **Which stick?** or **Which card?**,
**What to write?**, **Confirm**, **Writing**, then **Written** or **Not
written**. The walkthrough is [Get Faraday on a stick](../walk/get-faraday.md).

## Make

*Picture to come: `starter-make.png`, Make with its two cards.*

| Card | Line | Button |
|---|---|---|
| **Faraday stick** | **No Faraday stick attached**, or "Faraday stick attached · 0.1.8" | **Make a Faraday stick**: the primary button until a Faraday stick is attached. Opens [For which computer?](#for-which-computer) or [Which stick?](#which-stick-or-which-card) |
| **Pi card** | "Version 0.1.8", the Pi image's, or **Not in this Starter** | **Make a Pi card**: opens [Which card?](#which-stick-or-which-card). Grey when this Starter carries no Pi image |

## Make a Faraday stick

The pages below have no rail: the top app bar holds the back arrow,
the name of the page it goes back to, and the page's name.

### For which computer?

Shown only once the Starter carries both stick images.

| Control | What it does |
|---|---|
| **A PC or an Intel Mac** | the stick for a PC or an Intel Mac: opens Which stick? |
| **A Mac with M1 or M2** | the stick for a Mac with Apple silicon: opens Which stick? |
| **This computer** | at the end of the row that fits this computer. Nothing is chosen for you |

### Which stick? or Which card?

*Picture to come: `starter-make-which-stick.png`, Which stick? with two sticks listed.*

**Which stick?** for a stick, **Which card?** for a Pi card. Every disk
attached is a row, Faraday sticks first, then the others by size,
smallest first. The list follows disks as they are plugged in and
pulled out. No disk is ever chosen for you.

| Control | What it does |
|---|---|
| a disk's row | its maker and model (**Disk** when it reports neither), what is on it, and its size ("32 GB", "512 MB") at the right. A press opens What to write? for it. A USB disk has the stick's mark, the computer's SD card reader the card's, any other disk the drive's |
| "Faraday stick · 0.1.8" | the disk holds Faraday, this version; on Windows "Faraday stick · version not read" until it is read |
| **Empty** | nothing is on the disk |
| "1 volume, no files", "BACKUP, files on it", "2 volumes, files on them" | its volumes, by name where they have one, and whether they hold files |
| **Smaller than the image: needs …** | dimmed: the disk is too small, and cannot be pressed |
| **Used by this computer: mounted at …**, **Used by this computer: swap**, **Used by this computer: another device is built on it**, **Holds this computer's system** | dimmed: the computer uses the disk, and it cannot be pressed |
| **Not checked: what uses it could not be read** | dimmed: it cannot be pressed |
| **For a Raspberry Pi 3B, 3B+ or Zero 2 W with the Waveshare 2.8 inch screen** | on Which card?, the Pi and screen the card is for |
| **Cameras: a USB webcam, or Camera Module v1 or v2** | on Which card?, the cameras Faraday reads on the Pi |

| State | What the page shows |
|---|---|
| no disk attached | **No USB stick attached · 256 MB or larger**; on Which card?, **No SD card or USB stick attached · 256 MB or larger** |
| this Starter carries no image | **Faraday Starter carries no stick image**, or **Faraday Starter carries no Pi image**, in the error tone, and no list |
| this computer cannot write a disk | **This computer cannot write a stick from here**, in the warning tone; the rows cannot be pressed |

### What to write?

*Picture to come: `starter-make-what-to-write.png`, What to write? for an empty stick.*

The chosen disk and its size are the first line ("SanDisk Ultra ·
32 GB"). A ring is filled on the choice ticked.

| Control | What it does |
|---|---|
| **New Faraday stick** (**Everything on it is lost**) | the whole stick is written. **New Faraday card** on a Pi card. On a disk that is not a Faraday stick it is the only choice, and starts ticked |
| **Replace Faraday, keep data** (**Only the boot partition is written**) | shown only on a Faraday stick: the boot partition is written; the data partition, with its vaults and settings, is not read and not written |
| **This computer can read and change the data partition while the stick is attached.** | under Replace Faraday, keep data |
| **Replacing Faraday on a stick** | under it: opens that Learn page |
| **Already this build** | in place of the second line, in the warning tone: the stick holds this build already, and keep data cannot be ticked |
| **The image is a boot partition alone** | in place of New Faraday stick's second line, in the warning tone, and it cannot be ticked: a developer's image that holds only a boot partition, written only over a Faraday stick, keeping its data |
| **Reading the stick's version** | on Windows, after keep data is ticked on a stick whose version was not read: Windows asks once for administrator permission, and Continue waits for the answer |
| **Continue** | the primary button once a choice is ticked (and, for keep data, the version read): opens Confirm |

The reason the version could not be read is in the error tone above
the buttons.

#### Boot file

For a stick for a PC or an Intel Mac, under the choices:

| Control | What it does |
|---|---|
| **Boot file** | the section's label |
| **The release's (unsigned)** | ticked at first: the release's own boot file |
| **One I signed** | a boot file signed with your own db key, taken from the `.efi` files in Downloads. Continue waits until one is taken |
| **BOOTX64.EFI signed with your db key** | under it until a file is taken |
| "In …" | the folder searched, over its `.efi` files, a row each: a press checks the file against this release |
| **Checking against this release** | on the row being checked |
| "No .efi file in …" | when the folder holds none |
| **Certificate name** | the taken file's signer, as its certificate names it (**None** when it names none) |
| **Certificate SHA-256: compare it with your db key's** | the signing certificate's SHA-256 |
| **File** | the taken file's name |
| "BOOTX64.EFI: This boot file is not signed" | in the error tone, a refused file and why: **This is not this release's boot file**, **This boot file is not signed**, **This boot file has no signature that holds**, **This boot file holds more than the release's program and one signature** |

The walkthrough is [Your own Secure Boot keys](../walk/secure-boot-keys.md).

### Confirm

*Picture to come: `starter-make-confirm.png`, Confirm, before the stick is written.*

| Control | What it does |
|---|---|
| **Stick** (**Card** for a Pi card) | the disk's maker and model |
| **Size** | its size |
| **Device** | the system's name for the disk |
| **Serial** | its serial number, or **None** |
| **On it** | what is on it now, as Which stick? says |
| **Writes** | **New Faraday stick**, **New Faraday card** or **Replace Faraday, keep data** |
| **Boot file** | **The release's (unsigned)**, or **One I signed** with its **Certificate name** and **Certificate SHA-256: compare it with your db key's** |
| **Build** | the Faraday version written, or **Not known** |
| **SHA-256 of the image** | the image's sum, in groups; **SHA-256 of this image** for a developer's image |
| **Erase and write** | for a new stick or card: the primary button. The system's own permission prompt follows (Windows: administrator; Linux: the password, where asked) |
| **Write** | the same, for keep data |
| **Hold to write the older release** | in place of **Write** when keep data puts an older Faraday over a newer one: held to the end, writes; let go before, nothing happens |
| **Choose again** | shown when the disk is no longer the one confirmed: back to the disk list; **Erase and write** is then grey |

| Line | Shown when |
|---|---|
| **Not Faraday Starter's own image** | in the warning band: a developer's image |
| "This stick holds a newer Faraday: 0.1.9", then **Only to go back to an older release.** | keep data over a newer stick |
| **Writes a dev build, with a serial console** | keep data writing a developer's build |
| **Not the stick confirmed: it was removed or changed** (**Not the card confirmed: it was removed or changed**) | in the error tone: the disk was pulled out, or another took its place |
| the reason the write was refused | in the error tone |
| **This computer cannot write a stick from here** | in the warning tone; the write button is grey |

### Writing

*Picture to come: `starter-make-writing.png`, Writing, part way.*

No way back while the disk is written.

| Control | What it does |
|---|---|
| "SanDisk Ultra · 32 GB" | the disk being written |
| the bar | how much of the image is written, then read back |
| "Written 1.2 GB of 2.0 GB" | while writing |
| "Reading back · 1.2 GB of 2.0 GB" | while the written disk is read back and compared |

### Written or Not written

| Result | What the page shows |
|---|---|
| **Written** | the tick in the ok tone. **Keep the stick in to start this computer from it. To start another computer, pull it out.** (a Pi card: **The card can be pulled out.**). For a stick, **In the boot menu**, "something like UEFI: SanDisk Ultra, Partition 1": roughly the name this computer's boot menu gives the stick, when the stick reports its maker or model. **Start this computer from the stick** (primary; on an Intel Mac **Start this Mac from the stick**) opens [Start this computer from the stick](starter-learn-and-boot.md#start-this-computer-from-the-stick); **Done** goes to Home. A Pi card has **Done** alone |
| **Not written** | the cross in the error tone, and the reason in the error tone ([what each says](../walk/get-faraday.md#16-writing-then-written)). **Try again** goes back to the disk list; **Done** goes to Home |

The boot menu name follows this computer's maker: "UEFI: …, Partition
1" for most, "EFI USB Device (…)" for Lenovo's IdeaPad family and
Framework, "USB Hard Drive - …" for HP.

**Why:** the Learn pages [Replacing Faraday on a stick](../../learn/faraday/boot/replace-faraday.md)
and [When the stick does not start](../../learn/faraday/boot/not-starting.md).
