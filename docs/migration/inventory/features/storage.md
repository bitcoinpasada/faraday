# Storage and the disk process

All disk work of the app goes through one protocol: the app pushes
`StorageCommand`s and receives `StorageEvent`s. On the device, a separate
disk process serves FAT sticks over a FIFO and the app never opens a stick
itself; on the desktop (Faraday Demo, the Starter), a stick is a folder. The
same protocol keeps the Inbox, Outbox and kept entries across a restart (the
boxes), saves downloads and prints on a desktop host, and carries input-device
and camera events from the shell. It has no screens of its own; every feature
that reads or writes a file uses it. Code: `faraday/faraday-core/src/storage_api.rs`
(the app side), `faraday/faraday-storage/src/lib.rs` (the client and folder
sticks), `faraday/faraday-disk/`, `faraday/faraday-files/` and
`faraday/faraday-fat/` (the disk process and the FAT code).

## Screens and sheets

- None. Results reach `faraday-core` as `StorageEvent`s; refusals reach the person as the screens' toasts and lines (`EN.storage_*` strings).
- In Faraday Demo, stick pills stand for the boot stick and backup sticks as folders (`faraday/shells/demo/src/lib.rs:~97-129, 345-381`); pressing a pill plugs the stick in or pulls it (`~373-381`).

## Actions

- None of its own. Commands are pushed by other features (`StorageCommand`, `faraday/faraday-core/src/storage_api.rs`):
  - `Read`, `ReadQr`;
  - `Write`: read back and compared; `OnTaken::Ask` returns `NameTaken`, `Beside` writes a free name, `Over(sha)` replaces only a file with that SHA-256; `replace` only if the person chose it; vault `copies`;
  - `Wipe`: zeros written and read back first; `WipedName` if the bytes are in use;
  - `StickSpace`, `ZeroFree`, `MakeEmpty`;
  - `Rename`: refused where the name exists; vaults with a common copy hash are the same vault;
  - `NameStick`: a FAT-style name, capitals, at most 11 (from the doc only);
  - `Hash`;
  - `Print` (online desktop only), `ReadHost`, `SaveDownload` (never over a file; `name (2).ext`), `OpenDownloads`, `OpenInstitute`;
  - `SaveBoxes`, `ForgetBoxes`.
- Who sends: `ForgetBoxes` from `exit_forgetting` (power off with nothing waiting, Power sheet's power off and restart, their hold forms, idle power-off, failed self-test) (`faraday/faraday-core/src/action.rs:1317-1326`, `faraday/faraday-core/src/state.rs:2532-2556`, `:2664`); `Print` from the Inbox and Outbox PDF buttons, only online (`faraday/faraday-core/src/qr_view.rs:891-902`, `:1101-1113`); `OpenInstitute` from the site name in the Settings About note, only online (`faraday/faraday-core/src/screens/settings.rs:614-640`, `faraday/faraday-core/src/action.rs:1696-1701`).

## Effects

- Commands go out on `out.storage_out`, drained through `storage_pending` and `poll_storage` (`faraday/faraday-core/src/storage_api.rs:~1065-1072`).
- Events come back through `Faraday::storage` (`~1084-1094`), which runs `drain_jobs`, `note_here`, `stick::events::storage_event`, `event_settled`, `note_visit`, `storage_left`, `drop_forged_wallets`, `silent_vaults_catch_up`.
- Events: Read, ReadFailed, Written, WriteFailed, NameTaken, Wiped, WipedName, WipeFailed, Zeroing, ZeroFailed, Renamed, StickNamed, Hashed, QrRead, Cameras, QrSeen, Clock, Memory, NewInput, InputTyped, InputTapped, InputUsed, InputGone, InputTrust (held-back input devices, `devices-inputs`), BootUsb, HostFiles, Hovered, Dropped, HostRead, HostReadFailed, Saved, SaveFailed, Printed, PrintFailed, StickProblem (not FAT32 as Faraday reads it, or read-only), Restored, NotKept.
- Sticks (`Sticks` trait, `faraday/faraday-storage/src/lib.rs:~125-217`):
  - `Dirs`, folder sticks (`~367-370`);
  - `DiskProcess`, the FIFO client (`~1341-1487`, `~1955-2043`, `~2109-2308`); the serving loop `serve_with` (`~896-933`);
  - `stick_info` and `stick_info_after` (`~24-73`): a listing with vault detection, reading 4 bytes of files of vault length; the previous vault list is reused when name and size match (`~46-60`);
  - `DiskProcess::list` (`~1908-1952`): sticks with make, model and size; unusable and not-yet-handed-out sticks;
  - erase paths `StickSpace`, `ZeroFree`, `MakeEmpty`, `Wipe` (`~1103-1172`);
  - naming `NameStick` to `StickNamed` (`~1207-1218`);
  - vault write-back over a vault's own copy by SHA-256 (`~94-102`, `~1075-1102`, `~1987-2042`), replying `Written`, `NameTaken` or `WriteFailed`; rename replies `Taken` or `SameVault` (`~1173-1206`).
- Boxes (`faraday/faraday-storage/src/lib.rs:~560-725`, `~1321-1331`) keep the Inbox, Outbox and kept entries: `Dir(PathBuf)` on the device (a RAM directory), `Memory { inbox, outbox, kept }` on the desktop. `backup-gate` and `backup-flow` are written first and atomically (`~547`, `~609-610`).
- `Print` writes bytes to the host's print folder (`~1238-1263`); Faraday Demo's Make PDF on a sheet in the Inbox or Outbox saves there (`faraday/shells/demo/src/lib.rs:~339-343`).
- `ForgetBoxes` (`forget`, `faraday/faraday-storage/src/lib.rs:641-672`): clears the memory boxes; zero-fills, then removes the directory's files.
- Faraday Demo keeps the boxes in memory and carries them over a lock (`faraday/shells/demo/src/lib.rs:~329-330`).
- `MakeEmpty` success is answered as a zeroing step at cluster 2 of 0: the app marks the partition emptied (the backup flow's copies on it are gone), keeps its known total, and asks the next zeroing step (`faraday/faraday-storage/src/lib.rs:1135-1143`, `faraday/faraday-core/src/stick/wipe.rs:391-436`).

## State and lock

- Wipe status of command and event bytes: `Files`/`Kept`, `Read`, `QrRead`, `Restored` and `Write` bytes are `Zeroizing`. `HostRead`, `Print` and `SaveDownload` bytes are plain `Vec<u8>` (a received file or a PDF).
- `Debug` for `StorageCommand` prints names and lengths only.
- Restart after a lock whose save failed (`carry_unkept_lock`, `faraday/faraday-core/src/storage_api.rs:1167-1190`; `start_unrestarted`, `1196-1202`):
  - the Inbox and Outbox are moved (not copied) into `Carried`, which holds their bytes until `start_unrestarted`;
  - a mode change keeps nothing but the mode;
  - the new session is `Session::on(mode.network())`;
  - the new app is tainted for life: no stick is handed out until a restart that works; it toasts `toast_lock_unrestarted`.
- `save_boxes` (`faraday/faraday-core/src/storage_api.rs:~1147-1160`) keeps the Inbox, Outbox and kept entries in the shell's keeping across a restart; `ForgetBoxes` removes them at a real end (power-off).
- A lock does not drop the boxes, also during a vault transfer (history fix b6c4143a9).
- A new app starts a clean session with the disk process after a lock (history fix 3dc2acf7c).
- `HostRead`, `Print` and `SaveDownload` carry plain `Vec<u8>` and are online only, never reaching the disk process (`faraday/faraday-storage/src/lib.rs:1013-1024`). `SaveDownload` can carry a secret file received by Transfer, moved out of `Zeroizing` (`faraday/faraday-core/src/transfer/mod.rs:369-387`). `Print` carries a `FileKind::Sheet` file's PDF (`faraday/faraday-core/src/qr_view.rs:1101-1110`). `HostRead` carries any computer file picked for Transfer. Their `Debug` prints no bytes (`faraday/faraday-core/src/storage_api.rs:959-968`).

## Reach

- Used by every feature that reads or writes: Files and the Inbox (`files-inbox`), the boot import (`boot-import`), vaults, backups, the stick shell's disk work (`stick-shell`), the scanner for `ReadQr` (`scanner-qr`), devices for input events (`devices-inputs`).
- `file_menu::sha256` is called from the storage client (`faraday/faraday-storage/src/lib.rs:946, 1226`). `MAX_FILE` is read at `faraday/faraday-storage/src/lib.rs:823`.
- Held stick is delivered only when the camera is off (`release_held_stick`, `faraday/faraday-core/src/storage_api.rs:~1097-1118`).

## Rules and refusals

App side (`faraday/faraday-core/src/storage_api.rs`):
- A file over `MAX_FILE_LEN` (vault) or `MAX_READ` (others) is refused `visit_file_too_large`; a vault is also refused by `vaults.read_refusal` when this computer lacks memory (`~159-171`).
- A file is a vault by its contents; a non-vault `.kdbx` is a plain file.
- `NOT_OPEN` ("stick-not-open", a copy of the disk process's string) is reported in the app's words (`~996-1060`).
- With no stick listed, the secrets close and a process that is not clean locks (`~1122-1145`).
- Secret-bearing bytes in commands are compared in constant time (`~701-863`).

Client (`faraday/faraday-storage/src/lib.rs`):
- Folder sticks cannot be named: `EN.storage_name_folder` (`faraday/faraday-storage/src/lib.rs:426-428`).
- One disk request at a time: `ask` refuses with `EN.storage_stick_busy` while a transfer piece is in flight, and with `EN.storage_disk_silent` once the answer thread has ended (`faraday/faraday-storage/src/lib.rs:1867-1872`).
- Timeouts: `ASK` 15 s, `ASK_WRITE` 120 s, `ASK_READ_PART` 120 s, `ASK_PICTURE` 60 s (`faraday/faraday-storage/src/lib.rs:1413-1423`). Late answers are dropped by sequence number (`~1894-1896`, `~1519-1539`). A stalled transfer piece times out to `storage_disk_silent` (`~1735-1737`).
- A disconnected answer channel sets `dead`; later asks fail at once (`~1499-1502`, `~1899`). After the answer thread fails nothing more is written; waiting requests fail at once.
- Malformed or unexpected answers are errors (`EN.storage_answer_malformed`, `EN.storage_answer_*`): `~1744`, `~1761-1763`, `~1794-1796`, `~1805-1807`, `~1892`, `~1950`, `~2021`, `~2040`, `~2093`, `~2147`, `~2158`, `~2170`, `~2181`, `~2294`, `~2306`.
- `accept_part` (`faraday/faraday-storage/src/lib.rs:2050-2071`) refuses a read piece when the first piece's total exceeds `MAX_VAULT_READ`, when the total changes between pieces, when a piece has the wrong length (`want` = min(remaining, PIECE)), or when memory cannot be reserved.
- Background transfer only for names ending `.kdbx` (case-insensitive); writes only when longer than one piece; everything else goes one frame at a time (`faraday/faraday-storage/src/lib.rs:2185-2192`).
- Host-only commands (`Print`, `ReadHost`, `SaveDownload`) refuse without a host (`~1240`, `~1266`, `~1282`; `Print` uses `EN.storage_no_print`). `OpenInstitute` and `OpenDownloads` do nothing unless `host.opens` (`~1308`, `~1315`).
- `save_download`: the name passes `checked`; it never overwrites (`create_new`) and gives up after 999 tries with `EN.storage_no_free_name` (`faraday/faraday-storage/src/lib.rs:832-863`).
- `read_host` reads at most `MAX_FILE + 1` bytes (`faraday/faraday-storage/src/lib.rs:823`). The downloads listing omits OS files, folders and non-files (`~802`, `~807`); links are followed (`~806`); a link to a folder is omitted.
- Boxes save: a temporary file, then a rename; names pass `checked_for_write`; `backup-gate` and `backup-flow` first; files no longer listed and stray temporary files are removed (`~551-558`, `~603-630`). A bad name or a full disk does not stop the other files; the first error is returned. Restore drops a leftover temporary file of `backup-gate` or `backup-flow` (`~694-703`).
- `Boxes::save` returns early with `?` on `create_dir_all` and `read_dir` (`faraday/faraday-storage/src/lib.rs:605`, `623`), before `first_error`, so later subfolders are not saved in that case.
- `MakeEmpty` reports `next: 2, total: 0` (`faraday/faraday-storage/src/lib.rs:1137-1139`).
- Memory: `None` if `/proc/meminfo` or `MemAvailable` is unreadable or does not fit `u32` (`~443-444`, `~456-459`); a cgroup limit lowers it (`~447-450`); a cgroup `max` or unreadable text gives no limit (`~465-471`); on Windows `GlobalMemoryStatusEx` must return non-zero (`~537-539`).
- Picture reads one at a time (`~310-313`).

Disk process and FAT code (from history fixes, files named):
- A FAT32 top folder is capped at the format's 65536 entries (refused as damaged); the FAT is read in 64 KiB pieces; cluster numbers are limited to 28 bits; a file read follows only the clusters its size needs; a cluster loop is refused (`faraday/faraday-fat/src/lib.rs`).
- Zeroing leftover space covers each file's whole clusters, past its end too (`faraday/faraday-fat/src/lib.rs`).
- A stick listing reads only a candidate's first four bytes; an interrupted vault write's part file keeps the `.fvlt` suffix, so no planted part can become `autorun.inf` (`faraday/faraday-fat/src/lib.rs`, `faraday/faraday-files/src/lib.rs`).
- A vault write-back pulled at its rename ends with one copy under the asked name; a kept copy's chain must end where its size says (`faraday/faraday-files/src/lib.rs`).
- A locked (write-protected) card that cannot be opened is listed read-only (`faraday/faraday-disk/src/lib.rs`).
- A name with hidden characters is refused; save time is bounded on a stick of many vault-sized files or a looping chain.
- A piece read gets the same 120 s allowance as a write.
- Power-off waits only for disk-process requests, not for all jobs.
- `Boxes::save` writes subfolders in the order kept, inbox, outbox; a failed file write does not stop the others and the first such error is returned at the end; a failure to create or list a subfolder returns that error at once (`faraday/faraday-storage/src/lib.rs:602-630`).
- A stick's name: spaces at the ends are trimmed; empty means no label; at most 11 characters; letters, digits, spaces and `` ! # $ % & ' ( ) - @ ^ _ ` { } ~ ``; lower case is written in capitals; the boot-layout label and an unchanged name are refused (`faraday/faraday-fat/src/lib.rs:2044-2065`, `faraday/faraday-core/src/file_menu.rs:1092-1110`, `faraday/faraday-disk/src/lib.rs:903-907`).
- `DiskProcess::ask` refuses a request made while a transfer is in flight, at once and sending nothing, with "the stick is busy"; a dead disk process answers "silent" first (`faraday/faraday-storage/src/lib.rs:1855-1873`).

## Domain calls

- `faraday_codec` size limits, `faraday_text`, `faraday_shell_api::Command`, `subtle` (constant-time compare), `zeroize` (`faraday/faraday-core/src/storage_api.rs`).
- `faraday_fat` (`faraday/faraday-fat/src/lib.rs`), `faraday_files` (`faraday/faraday-files/src/lib.rs`, `faraday/faraday-files/src/proto.rs`), `faraday_disk` (`faraday/faraday-disk/src/lib.rs`, `faraday/shells/demo/src/main.rs`).
- `file_menu::sha256` from `faraday-core` (`faraday/faraday-storage/src/lib.rs:946, 1226`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 3dc2acf7c | A new app's talk with the disk process did not start clean after a lock; a failed answer thread left requests waiting. | A new app starts a clean session; a failed answer thread fails waiting requests at once. |
| afb9fd7b8 | After the answer thread failed the app kept writing to the disk process; an answered transfer did not end as answered. | After the answer thread fails nothing more is written; an answered transfer ends as answered. |
| f24b7a9b5 | A request during a piecewise vault write or read waited out the 15 s timeout. | Such a request fails at once: "the stick is busy writing". |
| 37ca3de30 | A request during a vault transfer blocked the app; power-off waited on the stick's jobs. | Requests wait until the transfer ends; power-off waits only for disk-process requests. |
| b6c4143a9 | A lock during a vault transfer dropped the boxes; power-off waited for all jobs. | Boxes stay across a lock; power-off waits only for disk requests. |
| 7d69118a0 | A locked (write-protected) card that could not be opened was listed as waiting. | Such a card is listed read-only. |
| 5bff59ae4 | A crafted stick could grow a FAT32 top folder without limit, make the disk process read the whole table into memory, or loop when following a file's clusters. | Top folder capped at 65536 entries (refused as damaged); FAT read in 64 KiB pieces; cluster numbers limited to 28 bits; reads follow only the clusters the size needs; a loop is refused. |
| 01fddabbd | Zeroing a stick's leftover space worked on the wrong range (file size, past end, whole clusters). | Leftover zeroing covers each file's whole clusters, including past its end. |
| b2c70ea9f | A stick with hundreds of vault-sized files, a looping file chain, or hidden characters in a name made a save slow or a name look like another. | Save time is bounded; a name with hidden characters is refused; the hidden-entries refusal says Zero the free space removes them. |
| 2ee34b490 | A vault write-back pulled at its rename, beside a second copy, was refused on every retry and never ended with one copy under the asked name; a kept copy whose chain did not end where its size said could be the one kept. | One copy ends under the asked name after a rename; a kept copy's chain must end where its size says. |
| 730295e09 | A 16 MiB piece read from a slow USB 2 stick got 15 s, so it failed with "the disk is silent" while a write had 120 s. | A piece read gets the same 120 s allowance as a write. |
| 0cabd576c | Zeroing free space while keeping files could zero a cluster shared by two entries, so a vault saved over that cluster was not actually zeroed. | Zeroing is refused when two files share a cluster, with a message saying so. |
| 0b18f538c | A crafted hidden part file whose header sizes reach the end of a 32-bit address space was walked forever, so the Pi's disk process stopped answering. | The vault-file check adds each offset with a check and reports the file as not whole. |
| 4832948a3 | Windows and macOS housekeeping files on a stick's data partition were listed, read and counted as the person's files. | Those system files (and dot-files) are ignored by name; a Faraday write leaves them in place. |
| 535390cc8 | A vault written back to a stick could lose the good copy when the stick was pulled or dropped writes; a delete or wipe could free or zero clusters another file, folder or the root used. | The new copy is read back before the original is deleted; a delete frees only clusters no other name uses; a write-back under a name the stick cannot hold is refused first. |
| 11967b85c | A FAT stick's listing read each candidate file in full; an interrupted vault write's part file could land under a name a planted part could use, so a part not ending `.fvlt` could become `autorun.inf`. | A listing reads only a candidate's first four bytes; a vault part keeps the `.fvlt` suffix. |

## Doubts

- c10f56835 (the Starter's stick writer reading the partition table with unaligned O_DIRECT reads, `faraday/shells/hub/src/writer.rs`) is about the Starter's stick writing, not this protocol; left out.
- 0cabd576c, 0b18f538c, 4832948a3 and 535390cc8 came with the stick shell's rows and are carried here.
- b2c70ea9f's files were not inspected; only its message is carried.
- `(bytes.len() - 1)` in `send_piece` would underflow for an empty file; looks unreachable, not checked.
- `StorageEvent`'s doc says it carries bytes in wiping buffers (`faraday/faraday-core/src/storage_api.rs:203-206`), but `HostRead.bytes` is a plain `Vec<u8>` (`faraday/faraday-core/src/storage_api.rs:499`).
