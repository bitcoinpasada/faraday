# A stick that does not say it is Faraday's: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/STICK-IDENTITY.md` on 2026-11-05. Each keeps
its original number; the source keeps its heading.

## 12 Order of work (from docs/STICK-IDENTITY.md)

The labels are the ones decisions and old commits use. E1 is the batch of
`docs/ESTABLISHED.md` that built the FAT32 formatter.

Each batch is its own change, landed when its tests pass; each needs
the one before it, except 2 and 3, which can go in either order.

| Batch | What | Where | Size |
|---|---|---|---|
| 1 | This specification and F182 | §2, §4, §5.1, §10, §11 | Built |
| 2 | The image: first usable sector 34, the boot FAT as Windows leaves it, the build's checks with `sgdisk`, `sfdisk` and `fsck.fat` | §4.1 | Built |
| 3 | The device and the layout: §5.1's rule in `faraday-boot`, `faraday-grant`, `faraday-disk` and `stick-boot.sh`, with fixtures of each shape and one Windows rewrote; `disk-test.py --no-kvm` | §5.1, §5.2 | Built |
| 4 | The Starter: the stick made for its size, the FAT32 formatter (`docs/ESTABLISHED.md` §3.2, batch E1), the boot partition's sum in `MANIFEST` and its check, Clear the whole stick first, the check's lines, keep data's check, Learn's Checking what is written, the guide's Written picture | §4.2 to §4.4 | Built |
| 5 | A dev build and the disk test; the timings; the Windows 11 virtual machine run with real builds, the Pi card's FAT16 boot partition included; what it finds, if anything, is a change of its own | §10's last items | half a day to a day |
