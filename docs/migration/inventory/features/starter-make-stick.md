# Making a Faraday stick

The Starter's Make flow writes a Faraday release image to a USB stick (for a
PC or an Apple silicon Mac) or to an SD card (for the Pi). It runs on Linux
and Windows. It is reached from the Starter's Make destination and from the
"This computer" card's Make button. The flow finds release images in
Downloads and among dropped files, lists the disks it may write, and writes
in one of three ways:
- **Whole**: the whole stick.
- **Keep data**: the boot partition only, which keeps a Faraday stick's data
  partition.
- **Clear first**: zero every sector, then a whole write.

The person can put a boot file they signed in place of the release's, and
can name the stick. The shell reads every byte back after writing. A Windows
stick can also be checked against its release ("Check this stick").

There are two halves:
- The pages and rules are in the `faraday-starter` crate (`faraday/faraday-starter/src/`).
- The writing is in the Starter shell (`faraday/shells/starter/src/`). On Windows it runs in an elevated copy of the program.

## Screens and sheets

The pages are `Page::Which`, `Disks`, `Mode`, `Confirm`, `Writing` and `Done`.
- **Which computer** (`StickFor`: `Pc` or `AppleSilicon`): asked only when both kinds are offered and no developer image stands in (`faraday/faraday-starter/src/lib.rs:~1277`). On a Mac the choice comes from `faraday/faraday-starter/src/macs.rs` `stick_for`.
- **Disks** (`decide::offers`, `faraday/faraday-starter/src/decide.rs:95-120`):
  - Faraday sticks first, then by size, then by id. A disk's number is kept by its id (`seen`).
  - A refused disk shows why: `InUse` or `TooSmall`.
  - "Check this stick" on a Faraday stick (`StarterAction::CheckStick`, `faraday/faraday-starter/src/draw/home.rs`).
- **Mode**:
  - Whole or Keep data, with each refusal named.
  - "Clear the whole stick first" with a time estimate (`faraday/faraday-starter/src/clear.rs`, `faraday/faraday-starter/src/draw/clear_first.rs`).
  - The image section (`faraday/faraday-starter/src/found.rs`, `faraday/faraday-starter/src/draw/images.rs`): images found; images to download (wanted names not found); images refused (`Mismatch`, `OtherRelease`).
  - The boot file choice: the release's, or "One I signed" (`faraday/faraday-starter/src/boot_file.rs`, `faraday/faraday-starter/src/draw/boot_file.rs`). It lists the `.efi` files in Downloads. For the file chosen it shows the signer name and fingerprint, or the refusal.
  - The optional volume name field (`faraday/faraday-starter/src/name.rs`, `faraday/faraday-starter/src/draw/name.rs`).
- **Confirm** (`faraday/faraday-starter/src/draw.rs` `confirm`):
  - The disk, mode, image and clear-first estimate.
  - If the disk changed since it was confirmed: a "not confirmed" line, with only Choose again and Write shown.
  - Write, which is a hold in some cases (see Rules).
- **Writing**: progress, and the bytes written out of the total. The total is the boot partition's length for Keep data.
- **Done**:
  - For a stick, the boot-menu entry text to look for (`boot_menu_entry`, `faraday/faraday-starter/src/decide.rs:~158`):
    - Lenovo IdeaPad and Framework: `EFI USB Device (...)`.
    - HP: `USB Hard Drive - ...`.
    - Others: `UEFI: ..., Partition 1`.
  - For a Pi card: Done only.
  - On failure: the reason, Try again, and Done.
- Check this stick's answer on Make's stick card. A failed check says why.

## Actions

- `Make(Target)`, `StickFor`, `Disk(n)`, `Mode`, `ClearFirst`, `Image(n)`, `BootChoice(bool)`, `BootFile(n)`, `Name` (typing), `Continue`, `Write`, `Again`, `CheckStick`, `KeepDataGuide`.
- Each action maps to a numeric `Action::Host` code (`faraday/faraday-starter/src/lib.rs:~455` `code`, `~563` `from_code`).
- Most actions are taken only on their own page:
  - `Disk` on Disks.
  - `Mode` and `Continue` on Mode.
  - `Write` on Confirm.
- A file dropped on the window (when the Demo is not open):
  - It joins the image finder's candidates (`StarterCommand::FindImages { dropped }`).
  - It is checked as the signed boot file (`CheckBootFile { path, source }`) (`faraday/shells/starter/src/main.rs:~286-303`).
- Starter command line (`faraday/shells/starter/src/main.rs:~851-893`):
  - `--image FILE` offers a developer image (`~894-911`).
  - `--downloads DIR` sets the folder searched.

## Effects

The Starter crate does no I/O. Its effects are `StarterCommand`s that the shell
carries out (`faraday/faraday-starter/src/api.rs:~1025-1168`), and it reads
`StarterEvent`s back. For this feature:
- `Write(WriteOrder { source, disk, device, serial, size, mode, clear_first, boot_file, name })`.
  - It sets `writing` and goes to `Page::Writing`.
  - The shell runs it on its own thread (`faraday/shells/starter/src/main.rs:578-585`).
  - Events back: `WriteProgress`, `Written`.
- `FindImages { dropped }`: hashes candidate files. Event back: `ImagesFound`.
- `ListBootFiles` and `CheckBootFile`. Events back: `BootFiles`, `BootFileChecked`.
- `ReadVersion { disk, serial, size, source }`: reads which Faraday a stick holds. Event back: `VersionRead` (`faraday/shells/starter/src/main.rs:~448-479`).
- `JudgeBoot`: judges a stick's boot partition. Event back: `BootJudged` (`faraday/shells/starter/src/main.rs:~505-537`).
- Raw disk writes:
  - Linux: through udisks2 on the system bus. It unmounts every mount of the disk, then `OpenDevice` read-write with `O_EXCL|O_SYNC`, then `Rescan` (`faraday/shells/starter/src/linux/mod.rs:~335-384`, `faraday/shells/starter/src/linux/bus.rs:~287-315`).
  - Windows: an elevated copy writes `\\.\PhysicalDriveN`, after locking and dismounting its volumes (`faraday/shells/starter/src/windows/win.rs:~989-1009`).
- Read-back: the disk is opened again after the write and compared (`faraday/shells/starter/src/writer.rs:~401-494`, `~600-613`).
- Randomness: a 256-byte pool from `getrandom::fill`, used for fresh disk and partition GUIDs, the volume serial and the MBR signature (`faraday/shells/starter/src/writer.rs:~686-708`).
- Windows elevation:
  - `ShellExecuteExW` with the verb `runas`, hidden, keeping the process handle (`faraday/shells/starter/src/windows/sys.rs:~499-529`).
  - A named pipe carries the order, the progress and the answer (`faraday/shells/starter/src/windows/pipe.rs`).
- Disk watchers: a udisks2 signal thread on Linux. A poll every 1.5 s on Windows (`faraday/shells/starter/src/windows/win.rs:~455`). Event: `Disks`, or `NoWriter`.

## State and lock

- In the Starter (`faraday/faraday-starter/src/lib.rs`):
  - `Making`: target, chosen disk id, mode, the confirmed disk id, serial and size, progress, result, `boot: BootPick`, `name: NameField`, `clear_first`, `stick_for`.
  - `Found`: `dropped` paths, `findings`, `wanted`, `chosen`.
  - None of it is secret, and none of it is wiped.
  - A new Make replaces `making`. A page change keeps it (`go`, `faraday/faraday-starter/src/lib.rs:~1991`).
- The name is a plain `String` (not secret).
- In the shell:
  - Everything for a write is made for that call and dropped at its end: `Identity`, `Job`, `Source`, `Random`, `Patch`, `Revert`, `Writing` with a 4 MiB `AlignedBuf`, `Comparing`, `Capture`, `Table`, `Sparse`. All of it is public bytes: the release, and IDs.
  - `Shell.writing` is cleared on `Written` (`faraday/shells/starter/src/main.rs:~591-595`).
  - `Finder` keeps a `Memo` map of file hashes, never cleared.
  - `Shell.image` lives as long as the window.
  - The version cache is per disk.
  - Boot-file bytes are in a plain `Vec<u8>`; they are not secret.
- The Starter has no lock. Its state lasts as long as the process.

## Reach

- Opened from: the Starter's Make destination and the This computer card (`starter-app`).
- Opens: the guide sheet on Which, Disks, Mode, Confirm, Writing and Done (`starter-app`), and the Keep data guide.
- Done leads to the Start page (`starter-start`).
- Reads the signed boot file through `faraday_sb`, as `secure-boot` does.
- The disk list and the Faraday-stick test are shared with the Start page's restart from the stick and with Remove boot entry.

## Rules and refusals

**Disk list** (`faraday/faraday-starter/src/decide.rs:95-120`):
- USB disks are always listed. An SD card slot (`SdReader`) only for a Pi card. Other buses are never listed.
- `InUse`: swap, held, unchecked, system, or mounted outside `/run/media` and `/media` (`faraday/faraday-starter/src/api.rs:~209-227`).
- `TooSmall`: the disk is smaller than the image's bytes.
- A disk can be tapped only on `Page::Disks`, when it is not refused and `no_writer` is false (`faraday/faraday-starter/src/lib.rs:~2239-2246`).

**Linux disks** (`faraday/shells/starter/src/linux/udisks.rs:~308-406`):
- A drive with no whole-disk block, or of size 0, is skipped.
- The whole-disk block has no Partition interface and no CryptoBackingDevice.
- Bus: `usb` is USB. `mmcblk` with an SD card is `SdReader`. Anything else is Other.
- `fixed` is false when the disk is hotplug or its sysfs `removable` is 1.
- A Faraday stick is a `gpt` partition table that passes `gpt::layout`. Its boot partition is `parts[0]`, which is safe because the layout requires exactly two partitions (`faraday/shells/starter/src/linux/udisks.rs:376-382`, `faraday/shells/starter/src/gpt.rs:63-65`).
- In use (`faraday/shells/starter/src/linux/udisks.rs:~243-269`):
  - a mount point outside `/run/media/` and `/media/` is `Mounted`;
  - active swap is `Swap`;
  - a kernel holder of the disk or of a partition is `Held`;
  - unreadable holders are `Unchecked`.
- The data partition is never mounted or read.
- The watcher sends the list again on udisks signals (2 s timeout) and sleeps 2 s after an error.

**Windows disks** (`faraday/shells/starter/src/windows/disks.rs`):
- A disk is listed only if its descriptor and geometry answered and its size is above 0 (`~405-414`).
- Bus: USB is USB. SD/MMC removable is `SdReader`. Anything else is Other.
- A disk holding the Windows folder, the system partition or the page file is `InUse::System`. When the Windows volume cannot be found, every disk is `InUse::System` (`~373-393`, `~481-484`).
- These partitions read as encrypted and hold the disk (`~343-349`, `~429-461`):
  - a volume with no file system name;
  - a partition with no Windows volume, other than Microsoft Reserved, such as a Linux file system, LUKS or swap.
- A Faraday stick is one with GPT that passes `gpt::layout` (`~474-475`).
- MBR types 0x00, 0x05 and 0x0F, and partitions of length 0, are skipped.

**No writer**: the `Elsewhere` platform reports `NoWriter` once, and `write` refuses with `NoWriter` (`faraday/shells/starter/src/platform.rs:~213-219`).

**"Unlike a stick" holds on Confirm** (`faraday/faraday-starter/src/decide.rs:~69-75`). Write becomes a hold for:
- a fixed disk;
- a disk over `STICK_MAX_BYTES` = 128,000,000,000 (`faraday/faraday-starter/src/decide.rs:43`);
- a disk that is not a Faraday stick and has an encrypted volume or a file system outside `stick_fs`: vfat, fat*, msdos, exfat, ntfs, udf, empty.

**A Faraday stick is erased only after a hold.** Keep data is listed first, and the line names the data partition as lost (history fix eca68fbed).

**Mode** (`mode_refusal`, `faraday/faraday-starter/src/lib.rs:~1548`):
- Keep data is refused with `NotYet` or `NotFaraday` on a disk that is not a Faraday stick, and on a Pi card.
- Keep data is refused with `Same` when the stick already holds this build.
- Whole is refused with `BootOnly` for a boot-only image.
- Keep data waits for a version read (`keep_unread`).
- Continue needs a chosen disk, a mode that is not refused, `boot_ready` and `name_ready`.

**Keep data, same build** (`faraday/faraday-starter/src/decide.rs:192-204`):
- `Fit::Same` becomes `Fits` when the stick's boot partition sum is not the release's. So a keep-data write that stopped part way is not refused as the same build.
- An image with no build id gives `Fits`.
- A stick newer than the image makes Write a hold (`writes_older`).

**Clear first** (`faraday/faraday-starter/src/clear.rs:25-44`):
- Offered only for `Target::Stick` with `Mode::Whole`.
- Toggled only on `Page::Mode` and when offered. Ordered only if offered and ticked.
- The estimate (`clear::minutes`) assumes 8 MB/s, rounded up.
- The shell refuses clearing for anything but a whole write: `Reason::Device`, "clearing first is for a whole write" (`faraday/shells/starter/src/writer.rs:843-849`).

**Image finder, in the Starter** (`faraday/faraday-starter/src/found.rs`):
- It offers only names with the prefix `faraday-stick-` or `faraday-pi-`, with duplicates removed by image SHA-256.
- The stick offers only the kind Which answered.
- `image_chosen` ignores a number past the list, and matches by name and build (`faraday/faraday-starter/src/found.rs:183-197`).
- `found_image` is the chosen image, else the first offered.

**Image finder, in the shell** (`faraday/shells/starter/src/finder.rs`, `faraday/shells/starter/build.rs`):
- The release list is built in. It is never read from Downloads or from beside a file.
- `FARADAY_STARTER_IMAGES` is read at build time (`faraday/shells/starter/build.rs:~23`, `~38-49`). The path is canonicalised. An unreadable path panics, and a list that does not parse stops the build.
- With no list built in, nothing is found and only `--image` is offered (`faraday/shells/starter/src/finder.rs:~31-38`, `~111-113`).
- `faraday/builds.toml` gives the release image names when no list is built in (`faraday/shells/starter/src/finder.rs:~42`). The names are `faraday-stick-<v>-x86_64-uefi.img.xz` and `faraday-pi-<v>.img.xz` (`~50-53`).
- Which files are opened (`faraday/shells/starter/src/finder.rs:157-194`):
  - A file is stat-ed only if its name is classified (`NameClass`). Only plain files are taken, and folders are not entered.
  - A name of another release gives `OtherRelease` without opening the file.
  - A listed name whose `.xz` SHA-256 differs gives `Mismatch`.
  - Hashing is remembered by path, size and modification time.
  - Dropped files join the candidates (`~143-147`).
- `found_job` needs the name, the `.xz` SHA-256 and the image SHA-256 to match a list line. It splits a name on `/` and `\` (`~60-81`).

**Developer image** (`faraday/shells/starter/src/image.rs:~31-72`):
- `Image::hash` reads the whole file, decompressing `.xz`. An unreadable file is refused.
- `boot_only` is decided by the name suffix `-boot.vfat`.
- An `.xz` file gets no build ID.
- `--image` without a file, or with a bad hash, exits 2 (`faraday/shells/starter/src/main.rs:~855-858`, `~906-909`).

**Signed boot file, "One I signed", in the Starter** (`faraday/faraday-starter/src/boot_file.rs`):
- `BootChoice(true)` lists the `.efi` files in Downloads.
- Only one check runs at a time. A result for a path other than the one being checked is ignored.
- `check_boot_file` does nothing unless on Mode, offered, not already signed and idle. It also returns early with no image source (`faraday/faraday-starter/src/boot_file.rs:108-122`).
- `boot_ready`: Continue is blocked until a signed file is chosen. `may_write` covers `no_writer` false, no mode refusal, not `keep_unread`, `boot_ready` and `name_ready` (`faraday/faraday-starter/src/boot_file.rs:44-62`).
- The order carries the file only when it is signed and offered.

**Signed boot file, in the shell** (`faraday/shells/starter/src/signed.rs`):
- `signed::list` lists `.efi` files of any case in a folder, newest first, without entering folders (`~158-178`).
- The signer name has control and bidi-control characters removed and is at most 64 characters (`~109-119`). The fingerprint is shown for comparing with the person's db key.
- Reading refuses (`~84-100`, `~200-202`, `~241-243`):
  - anything but a plain file, such as a reparse point: `BootFileRead`;
  - a file over `MAX_BOOT_FILE` (= `faraday_boot::MAX_PARTITION`): `BootFileLarge`;
  - bytes that differ from what was chosen (SHA-256): `BootFileChanged(path)`.
- Transplant results map to refusals (`~133-153`):
  - `Transplant::Release`: `ReleaseBootFile(Image)`.
  - `Other`: `BootFileOther`.
  - `Unsigned`: `BootFileUnsigned`.
  - `Layout`: `BootFileLayout`.
- Signer results map to refusals: `Unsigned`, `Broken`, `NotRead` and others give `BootFileUnsigned`, `BootFileBroken`, `BootFileNotRead` and `BootFileOther`.
- `CheckBootFile` with no release gives `NotCarried` (`faraday/shells/starter/src/main.rs:~541-542`).
- On Windows the bytes are sent once over the pipe (`faraday/shells/starter/src/windows/boot_block.rs:~36-93`):
  - `send` refuses a file over `MAX_BLOCK` and writes nothing.
  - `receive` refuses a length over `MAX_BLOCK` before reading (`Refused::Large`).
  - A short pipe is `Short`. A SHA-256 that differs is `Sum`.
  - The elevated side checks the file before anything is unmounted (`faraday/shells/starter/src/windows/win.rs:~747-754`).

**Name** (`faraday/faraday-starter/src/name.rs:24`, `39-53`, `121`):
- Optional, for Whole writes only.
- At most 32 typed characters. Control characters are ignored.
- Refused: a label over 11 characters (`starter_name_long`, the FAT `Long` error), any other FAT error, and a boot partition's label.
- Lowercase is written as capitals and trailing spaces are trimmed. An empty field gives no name order.
- The shell needs Whole and a whole-stick image, else `NotStickImage(StickFault::Name)` (`faraday/shells/starter/src/writer.rs:~673-680`). The label must be FAT-writable, not empty and not a boot partition's label (`faraday/shells/starter/src/ids.rs:~537-546`).

**Confirm and Write** (`faraday/faraday-starter/src/lib.rs:~2620` `write`; `faraday/faraday-starter/src/scroll.rs:161-172`):
- Write needs a confirmed disk, a mode, an image, and `may_write`.
- Write is a hold (`HOLD_MS`) when `writes_older`, `writes_unlike` or `writes_over_stick`.

**Writing**:
- Back is ignored.
- Progress and `Written` are ignored unless a write was asked (`faraday/faraday-starter/src/lib.rs:~1799-1805`).
- Closing the window is held during a write (`hold_close`, `faraday/faraday-starter/src/lib.rs:~2612`). The shell closes when the write reports (`faraday/shells/starter/src/main.rs:308-316`).
- The Writing page stays and shows the result, even when an uninstall ends while the write runs.

**Done**: when a virtual machine is attached, Start is not primary and a Start press is ignored (`faraday/faraday-starter/src/lib.rs:~2311`).

**Shell write order** (`faraday/shells/starter/src/writer.rs:~505-633`):
1. No matching job: `Written(Err(Reason::NotCarried))` (`faraday/shells/starter/src/main.rs:578-585`).
   - The job comes from `Source::Found` through `finder::found_job`, or from `Source::Image` only when its path and SHA-256 equal the developer image (`faraday/shells/starter/src/main.rs:~565-572`).
2. The same disk (device, serial, size), else `Reason::Changed` (`~982-991`, used at `~514`).
3. A disk in use: `Reason::InUse`.
4. Whole only: a boot-only source is `BootOnly`. An image larger than the disk is `TooSmall` (`511-531`).
5. A found image whose SHA-256 differs from the list: `ImageChanged` (`~318-332`).
6. A developer image whose SHA-256 and size differ from Confirm's (`~334-346`).
7. Without a signed boot file, the boot partition SHA-256 must equal the list's `boot_sha256`, else `BootSum` (`~296-314`).
8. The block size must be a power of two from 512 to 4096, else `Device` (`~535-541`).
9. A stick needs 512-byte logical sectors, else `SectorSize` (`~812-830`). An unreadable size is refused. On Windows an unknown sector size gives 0, which is refused (`faraday/shells/starter/src/windows/win.rs:~983-987`).
10. The identity is read again after open, else `Changed` (`~549-550`, `~997-1007`).

**Variants** (`faraday/shells/starter/src/writer.rs`):
- **Whole stick** (made, never patched): GPT and a FAT32 data volume, fresh disk and partition GUIDs, a volume serial, and the optional name.
- **Pi card** (patched IDs, `~644-672`): the source SHA-256, with the patches taken out, must equal the list's, else `BackSum` (`~717-750`).
- **Keep data** (`~620-625`, `~791-808`, `~1009-1090`):
  - Only the boot partition is written.
  - The table must be a Faraday stick's, else `NotStick`.
  - The table is read before and compared after, else `TableChanged`.
  - The same build is refused with `SameBuild` (`~1101-1119`).
  - On Windows the elevated side reads the stick's release again and refuses the same build (`faraday/shells/starter/src/windows/win.rs:~766-777`).
- **Clear first**: zeroes every sector, only with Whole.

**Read-back** (`faraday/shells/starter/src/writer.rs`):
- The disk is reopened after `drop(disk)` and compared: `BackEnds`, `BackDiffers`.
- Windows error 1110 (media changed) gets one retry. Any other failure stops the write.
- The table is written last when `table_last` (`~614-618`, `~906-928`).
- A flush failure is `Flush`.

**Progress** counts only the bytes that reach the stick. The end is sent once.

**Stick layout** (`faraday/shells/starter/src/gpt.rs:62-91`):
- Exactly two partitions (`Partitions(n)`). Type GUIDs are ESP, then basic data, compared without case.
- The boot partition is at 1 MiB. Shorter than 64 MiB at that offset is `BootSmall`. Any other offset or size, including larger, is `BootPlace`.
- The data partition is at `DATA_AT`, at least 128 MiB, and ends within the usable end (`DataPlace`).
- Names, IDs and labels are not checked.
- `StickFault` lists what a partition table must show (`faraday/faraday-starter/src/api.rs:~388-441`).

**GPT reading** (`faraday/shells/starter/src/gpt.rs:~182-286`):
- Sector size 512 is tried, then 4096.
- Checked: the signature; header size from 92 up to the sector; the header CRC; entry size from 128 to 4096; entry count from 1 to 1024; the entries CRC.
- The entries must lie inside the disk, and the table sectors clear of partitions.
- Each used entry must lie inside the usable range.
- The backup header and entries must lie inside the disk, past the usable area.
- The keep-data check is `faraday_boot` (`~315-361`).

**Capture and ids** (`faraday/shells/starter/src/ids.rs`):
- A stick is never patched, only made: `fresh` refuses (`~185-186`).
- Making a whole stick needs (`~243-281`):
  - 512-byte sectors, 32 entry sectors, and a header of 92 to 512;
  - a disk big enough;
  - the MBR signature and a protective 0xEE entry (`NotGpt`);
  - used entries only the data or boot one (`BootPlace`);
  - the data partition found (`DataPlace`).
- Gaps are zeroed (`~311-317`), so sectors 34 to 2047 hold no old marker.
- FAT serial rules: `NoSerial` (`~386-398`).
- A Pi MBR needs 0x55AA and exactly two FAT partitions of 64 and 128 MiB, in order (`CardLayout`, `~682-704`).
- Random serials and the MBR signature are never zero. GUIDs are v4 (`~710-729`).
- Any incomplete stretch makes `done()` fail (`~330-338`).
- Randomness failure is `Reason::Random` (`faraday/shells/starter/src/writer.rs:~694`).

**Linux write**:
- An unmount failure is `Unmounting`. Opening read-write fails as `OpenWrite`. A direct read falls back, and fails as `OpenRead`.
- A disk that is gone is `Reason::Gone` (`faraday/shells/starter/src/linux/mod.rs:~343-348`).

**Windows write**:
- Only the `Bus` kinds the Starter offers. `Bus::Other` is refused as `Changed` (`faraday/shells/starter/src/windows/win.rs:~972-979`).
- Volume lock and dismount get up to 10 tries, 300 ms apart, else `Unmounting` (`~989-1009`).

**Write events** belong to the write asked for. Stale answers are ignored: `VersionRead` for a stick no longer attached, or arriving during a write (`faraday/faraday-starter/src/lib.rs:~1704`, `~1936-1942`).

**Version** (`faraday/shells/starter/src/stick.rs:~27-86`):
- `stick::release_at` reads `faraday-build.txt` on the boot partition (limit `MAX_RELEASE*2`), else the kernel file. A failure gives None.
- An unread version says "version not read" and does not switch off Keep data's rules.
- On Windows the file is opened keeping its access date; the `SetFileTime` result is ignored. Keep data's version read goes through the elevated `--read-version` (`faraday/shells/starter/src/windows/win.rs:~164`, `~588`, `~809`).
- The cache per disk is read again only when the serial or size changes, or after `forget`:
  - Linux: `faraday/shells/starter/src/linux/mod.rs:~230-267`.
  - Windows: `faraday/shells/starter/src/windows/win.rs:~455-483`, stored only when a boot volume exists.
- On Linux, `boot_sum` hashes the whole boot partition with no cap. `boot_bytes` is capped by `MAX_PARTITION`: one byte more is refused.

**Check this stick and judging**:
- `boot_check` (`faraday/faraday-starter/src/decide.rs:~225`):
  - Equal sums: `Matches`.
  - No image sum, or the stick sum unread: None.
  - Differing sums: `MatchesSigned` when the stick is signed, else `Differs`.
- `can_check_stick` (`faraday/faraday-starter/src/lib.rs:~1355`): Windows only; not while reading; a stick attached with a release sum; not yet judged.
- `judge_boots` sends `JudgeBoot` only on Linux, once per stick, and not while one is in flight (`faraday/faraday-starter/src/decide.rs:~454-499`).
- The answer is filed under the release it was asked for. A stick that could not be judged says so and is not offered again.
- Judging in the shell (`faraday/shells/starter/src/main.rs:~505-537`):
  - An unreadable or changed partition answers "not signed".
  - No release to judge by answers "not judged".
  - The stick is signed only if its SHA-256 equals the listed sum and `signed::is_release_signed`.
  - On Windows the stick's signed state is `judgement` (`faraday/shells/starter/src/windows/win.rs:~870-878`).

**Elevated copy on Windows: arguments** (`faraday/shells/starter/src/windows/elevated.rs`):
- The first argument must be `--write` or `--read-version`. The count must be even.
- An unknown key, a duplicate key or a missing required key is refused.
- `--pipe`: `\\.\pipe\faraday-starter-` plus 1 to 64 hex digits or dashes (`43`, `371-381`).
- `--disk`: `\\.\PhysicalDriveN`, N from 0 to 999, with no leading zero. `--device` must equal `Disk N`.
- `--serial`: at most 256 characters, no control characters.
- `--size`: digits only, at most 20, above 0.
- `--mode`: `whole` or `keep-data`.
- SHA-256 values: 64 lower-case hex digits; upper case is refused.
- `--name`: checked by `ids::label_of`.
- `--clear-first yes`: only with `whole` (`168-172`).
- `--found` and `--image`: absolute (a drive or `\\server\`), at most 1024 bytes, no control characters.
  - `--found` excludes `--image` and needs `--img-sha256`.
  - `--image` needs `--image-size`.
- The source keys of `--read-version` need `--sha256`.
- Exit code 2 for a parse failure, 3 when the pipe cannot be opened (`faraday/shells/starter/src/windows/win.rs:~693-697`, `~740-745`, `~810-814`).

**Elevated copy on Windows: the pipe** (`faraday/shells/starter/src/windows/sys.rs:~387-432`):
- Its DACL `D:P` gives full access only to the account's SID and to Built-in Administrators.
- One instance only (`FILE_FLAG_FIRST_PIPE_INSTANCE`). Remote clients are rejected.
- The pipe is duplex.
- Its name is the prefix, the pid and a nanosecond time (`faraday/shells/starter/src/windows/win.rs:~334-344`).
- The access check is the client PID: the answering process must be the elevated child (`faraday/shells/starter/src/windows/win.rs:311`, `~547-551`, `~639-643`).

**Elevated copy on Windows: the protocol** (`faraday/shells/starter/src/windows/pipe.rs`):
- A line is at most `MAX_LINE` 8192 bytes (`26`, `592`), UTF-8, tab-separated.
- Only the escapes `\\ \t \n \r` are taken.
- Refused: a missing or extra field, a bad number, and an unknown message, fault, reason or release-fault name.
- The progress stage is `clearing`, `writing` or `reading-back`. `signed` is `yes` or `no`.
- There are 36 release-fault names and 28 reasons without a payload.

## Domain calls

- `faraday_payload::{ImageList::parse, ImageList::classify, NameClass, ImageKind::of_name, sha256_of, hash_image, write_image, write_payload, ImageSum, Sha256Sum}`.
- `faraday_boot::layout::{BOOT_SECTORS, BOOT_START, DATA_SECTORS, DATA_START, SECTOR, is_boot_label}`, `faraday_boot::{MAX_PARTITION, MAX_RELEASE, find_release}`.
- `faraday_fat::{Region, format32, BootSector::parse, boot_sector_serial, boot_sector_label, Volume::{open, set_label, into_disk}, read_in, rewrite_in, label_bytes, Error, BadName}`.
- `faraday_files::fat_text`.
- `faraday_sb::pe::{transplant, signer, Signer, Transplant}`, `faraday_sb::common_name`.
- `faraday_paper::build::{build_id, compare, version, version_number, describe, Fit}`.
- `faraday_crypto::{crc32, hex::{encode_string, decode_exact, decode_vec}, work::{Compute, Work::FileHash}}`.
- Outside crates: `sha2`, `getrandom`, `rustbus` (udisks2), `windows_sys`.

## Fixtures

- `faraday/shells/starter/tests/udisks/attached.txt`
- `faraday/shells/starter/tests/udisks/faraday-stick.txt`
- `faraday/shells/starter/tests/udisks/hard-drive-and-emmc.txt`
- `faraday/shells/starter/tests/udisks/in-use.txt`
- `faraday/shells/starter/tests/windows/laptop.txt`
- `faraday/shells/starter/tests/windows/usb-hdd.txt`

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 4bac34f9b | The Starter's Windows build did not compile (a match arm against a two-field variant). | The build compiles; the progress loop ignores Release messages with both fields. |
| d5c3c19fc | On Windows a first whole write to a blank stick could fail its read-back with error 1110 (media changed). | The read-back is opened afresh and retried once when Windows says the media changed; any other failure still stops the write. |
| bfd2f2d5f | On Windows a locked BitLocker To Go volume made from a FAT stick read as holding no files, so no hold was taken. | A volume with no file system name is encrypted and shows no files; a hold is asked. |
| c87c26777 | A GPT or MBR partition with a Linux file system, LUKS or swap read as no files, so a tap wrote over the disk. | Such a partition reads as encrypted and asks for the hold. Microsoft Reserved is left out. A Faraday stick is still known by its layout. |
| eca68fbed | Every Faraday stick counted as holding data, erasing one was not behind a hold, and Replace Faraday listed the erase first. | A Faraday stick is erased only after a hold; keep-data is listed first; the line names the data partition as lost. |
| 698c7b8e0 | The Starter took write progress and results not asked for, and did not re-read the disk after opening it. | Write events belong to the write asked for; the disk is re-read after opening. |
| 0e7f27169 | A Windows stick's Check this stick answer was not tied to its release, and a failed check did not say why. | The answer belongs to the release it was asked for; failures are stated. |
| c2dab4804 | On Linux the check answer was not filed under the release asked for, and a stick that could not be judged was offered the check again. | The answer is filed under the requested release; such a stick says so and is not offered again. |
| dba7f1240 | On Windows an unreadable stick version read "0.1.0 or earlier", which switched off Keep data's rules. | It says "version not read"; Keep data reads it once through an elevated read; the same build is refused and a downgrade warned. |
| 983987833 | On Linux the Starter would unmount and write a disk mounted at /mnt, or one with LVM, RAID, an open encrypted volume or unreadable holders. | Only /run/media and /media count as the desktop's mounts; the kernel's holders of the disk and each partition are checked. |
| 0bd375106 | Keep data showed the image size as written while only the boot partition reached the stick. | Progress counts only bytes that reach the stick; the end is sent once; the boot partition's length is the total. |
| 99fb53b07 | Disks with 4096-byte sectors were not refused, and a made stick kept an old ISO's marker in sectors 34 to 2047. | Only 512-byte-sector disks; an unreadable size refused; those sectors zeroed. |
| 9ff31a057 | A GPT whose backup header lies inside a partition was accepted. | The backup header and entries must lie inside the disk, past the usable area. |
| 20e7cc8a2 | A GPT with entries inside a partition, or partitions outside the usable area, was accepted. | Header entries and every partition lie in the usable area; a table Windows rewrote still reads. |
| 276f53d56 | A header whose entries run to the disk's end was tested with a wrong checksum; the backup region was not always written. | The checksum is of the bytes read; the backup region is always written. |
| 03a672d06 | `faraday/image/stick-boot.sh` did not check the target's primary header, entries, sizes or checksums. | The script refuses a bad signature, header size, checksum or out-of-bound entries. |
| 039a45e4d | The PC stick image did not match what Windows leaves (first usable sector, FAT[1], FSInfo hint). | The image is left as Windows leaves it, checked with sgdisk, sfdisk and fsck.fat in the build. |
| e48785ec0 | A stick write was lost when the uninstall ended while it ran; the Writing page did not stay. | The Writing page stays and shows the result; closing the window waits for the write. |
| dc63f6353 | A keep-data write that stopped part way left the new version at the boot partition's start, so Again refused keep-data as the same build. | Keep-data is refused as the same build only when the stick's boot partition is already the release's. |

## Doubts

- Eleven history rows were moved here from `starter-app`, because they are about making a stick: dba7f1240, 983987833, 0bd375106, 99fb53b07, 9ff31a057, 20e7cc8a2, 276f53d56, 03a672d06, 039a45e4d, e48785ec0 and dc63f6353.
- 4bac34f9b is a compile fix, not checked further.
- The files changed by c87c26777 and eca68fbed are not confirmed.
- `faraday/shells/starter/src/writer.rs:~698-708`: an error refilling the random pool is ignored (`let _ =`). It would matter only if the pool ran out; about 56 bytes are used for a stick and 12 for a card, out of 256.
- On Linux, `boot_sum` has no size cap (`faraday/shells/starter/src/linux/mod.rs:~283-301`). It relies on the layout check's fixed 64 MiB boot partition, which is an inference not read there.
- On Windows, `pipe_connected` returns `Ok(false)` on a successful connect (`faraday/shells/starter/src/windows/sys.rs:~441-451`). Not checked whether this is by design.
- `ConnectNamedPipe` errors other than already-connected or no-data are all read as "not connected".
- On Windows the program waits for the child's end in a busy loop with no timeout (`faraday/shells/starter/src/windows/win.rs:~319`, `~577`, `~654`, `~192`).
- Pipe names are predictable. The guard is the client-PID check and the DACL.
- `NameClass::Unrelated` at `faraday/shells/starter/src/finder.rs:~169` is an unreachable duplicate arm.
- `faraday/faraday-starter/src/draw/images.rs` casts `n as u16` with no bound (`faraday/faraday-starter/src/draw/images.rs`). `faraday/faraday-starter/src/boot_file.rs` guards the same cast.
- `clear_first_order()` is called twice in `write`.
- `StarterAction::code` maps an unknown Item to MakeStick without a word.
- Ship in non-test code: the `Recorded` stand-in, `udisks::Objects::parse`, and the Windows flag `--record-disks` (`faraday/shells/starter/src/windows/win.rs:~678-684`).
- `faraday/image/stick-boot.sh` mirrors keep-data; it was not read. Docs named: `docs/STICK-IDENTITY.md` §5.1, `docs/STARTER.md`.
- Lines in `faraday/faraday-starter/src/draw/*.rs` were partly given as local and partly as cumulative numbers. Check them again before use.
