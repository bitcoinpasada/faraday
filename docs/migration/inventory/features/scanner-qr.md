# Scanning and QR codes

The camera, the scanner sheet, reading QR codes and showing them. A scanner
(`Sheet::Scan`) is opened by a feature with a purpose (Home's Scan, a form's
own Scan, a check, a restore, a spend); it turns the camera on, reads single
codes, UR parts, BBQr, numbered (Specter) parts and Faraday file envelopes,
classifies what it read, and shows a read card with the ways over (load a key,
sign, collect, decode, keep as a file). On a desktop shell a scanner can also
take pasted text. Pictures on a stick are read for codes the same way
(`StorageCommand::ReadQr`). The QR view (`Sheet::Qr`) shows one code or an
animation of parts for a wallet, key, message, address or file, and can save a
labelled PNG to the Outbox or a PDF on a desktop. The camera itself is the
stick shell's capture thread on the device, the window shell's lens on the
desktop, over V4L2 on Linux and Media Foundation on Windows.

## Screens and sheets

- `Sheet::Scan` (`faraday/faraday-core/src/qr_view.rs`, `faraday/faraday-core/src/scan_key.rs`): the camera picture with the found-code outline, corner brackets, a state line and a parts bar (`faraday/faraday-core/src/screens/scan_frame.rs:~17`, also used by the session pane); it takes no press. `Sheet::IdleWarn` keeps the camera on (`faraday/faraday-core/src/qr_view.rs:~1679`).
- The read card, in place of the picture on the open scan sheet: the kind read and its ways over. Text is masked behind an eye.
- `Sheet::KefRead` (a Krux code read), `Sheet::Pull` (pull the stick first).
- A scanner beside a page (address check) scrolls the page (`faraday/faraday-core/src/motion.rs:~1329`); the session pane can stand a scanner in (wide, Scan sheet, at most one camera).
- State lines: too fine for this camera; no picture (Silent after `NO_PICTURE_MS`, Unavailable, Refused, InUse, NotFound); `scan_hold_square_hint` when a QR stays unread on a large picture (`faraday/faraday-core/src/scan_key.rs:~135`); UR progress naming the missing parts; the decoder's refusal reason.
- `Sheet::Qr` (opened by `QrArea::open`, `faraday/faraday-core/src/qr_view.rs:~814`): one static code or animated frames (UR, BBQr, numbered parts), with format, part size and speed choices.

## Actions

- Purposes (`ScanPurpose`, `faraday/faraday-core/src/qr_view.rs:~36-106`): Files, Intake, Seed, KeyPart, WalletKey(w), GpgPublic, GpgPaperkey, CheckSent, CheckReturned, MessageText, ToolText, SeedCompare, Sweep, OtsHeader, OtsFileHash, MultisigSetup, VaultEntry, CheckCopy(fp), CheckConvert(fp, compact), CheckCodex32, CheckKef(i), Kef, Transfer (Demo).
- `Ask` (Description, Address, Xpub(n), Cosigner, SpendPsbt, Any, Sign, Decode, Invoice) says what a scanner takes (`asked`, `faraday/faraday-core/src/scan_read.rs:~732-741`); Sign, Decode, Address and Invoice take everything. Each scanner is titled for what its opener asks (`Action::ScanFor(Ask)`).
- Scanner presses: `ScanPart`, `ScanSeed`, `ScanKef`, `ScanKey(w)`, `ScanCamera(i)` (choose a camera), `ReadGo` (take a way from the card), `ReadAgain`, `read_again_cancel`.
- Ways over from the card (`Way`, `faraday/faraday-core/src/scan_read.rs:~538-598`): Key, Share, Sweep, Lightning, AddTo, Unlock, MakeVault, Sign, Collect, Decode, Wallet, Restore, Check, Invoice, SignMessage, Note, Keep, UseKey, RestoreGpg, Open, NotEncrypted, Look, CheckAddress, MakeWallet, OpenWallet, SignAddress.
- Paste (Ctrl+V, `paste`, `faraday/faraday-core/src/scan_key.rs:~996`; `pasted`, `~1028`) and Paste words (`entry_paste`, `~889`), on a clipboard shell only.
- QR view: `QrFormat(i)` (wraps over `QrFormat::ALL`), `QrPartSize`, `QrSpeed(ms)` (saved by `settings.set_qr_frame_ms` and `save_boxes`, `faraday/faraday-core/src/qr_view.rs:~985-988`); format and size are ignored for one-code views. `QrPng` and `CodePng` save a labelled PNG to the Outbox (`code_png`, `~1602-1647`). `PdfInbox` and `PdfOutbox` make a PDF (`make_pdf`, `~1101-1116`). `QrSigned`, `QrPart`, `QrWallet(i)`, `QrOutbox(i)`, `GpgSignature`.

## Effects

- `Command::CameraOn` and `CameraOff` (`faraday/faraday-core/src/qr_view.rs:~1019, 1061, 1092`; `camera_on`, `~1667-1682`; Transfer excluded online).
- `Command::RequestClipboard` for paste (desktop).
- `StorageCommand::Print` for a PDF (does nothing offline, no toast); a PNG goes into the Outbox.
- Picture reads: `ReadQr` reads a picture and its codes on one worker thread, one at a time (`faraday/faraday-storage/src/lib.rs:~306-363`, `~906-919`, `~1237`).
- Stick shell camera (`faraday/shells/stick/src/camera.rs`):
  - `on()` starts a capture thread (`--no-camera` refuses with `Err("--no-camera")`; already running is a no-op); `off()` stops and joins; `choose(path)` restarts on a node;
  - the device is found by `faraday_v4l2::find_or_failure` unless chosen; none gives `Message::Unavailable` ("no /dev/video* is a streaming capture device", with a `CameraFailure` Refused, InUse or NotFound);
  - `climb` (`faraday/shells/stick/src/camera.rs:~402-420`) tries `LADDER` 1920x1080, 1280x720, 640x480; the first that streams wins; refused, in use or not found ends the climb; a size giving nothing is `Silent` after `FIRST_FRAME` 2.5 s (the last size waits without limit); all silent is `Climb::Nothing` and the thread sends nothing (the core shows "The camera sends no picture");
  - capture at `FPS` 15, `POLL` 100 ms, `QUEUE` 2; a full channel drops the new frame; frames reduced with `faraday_scanner::{reduce, reduce_factor, reduce_chroma}`; an empty reduced chroma is dropped (grey preview); the sustained rate is printed after 5 s to stderr.
- Stick shell loop (`take_in_camera`, `faraday/shells/stick/src/main.rs:2977-3040`): per pass keeps only the newest `Shot`; reports `Unavailable` once, drops the scanner and sends `Event::CameraFailed(why)` or `CameraUnavailable`; offers the full luma to the scanner (every other frame read inverted) and sends `Event::CameraFrame { luma, chroma }` (reduced); scanner output as `Event::Scanned { bytes }`; `StorageEvent::QrSeen { .., module_tenths }` clamped 0..65535. The camera list and choice go as `StorageEvent::Cameras` and `Faraday::take_camera` (`faraday/shells/stick/src/main.rs:~3048-3055`).
- Window shell lens (`faraday/shells/window/src/lens.rs:45-139`): `Lens::on` sends `StorageEvent::Cameras` on every `CameraOn`, starts the camera, then a `Scanner::new(ReadMode::Full)` if none (`faraday/shells/window/src/lens.rs:45-59`); `Lens::off` stops the camera and drops the scanner (`~64-67`); `chosen` takes a camera chosen on the sheet (`~69-74`); `pump` delivers the newest frame, each code read (`Event::Scanned`, `~125`) and `QrSeen` (`~127-138`). The stick shell's scanner uses `ReadMode::Reduced`.
- Each `CameraOn` first sends the camera list to the app, on the stick and the window shell, and sends it whether or not the camera then starts; a failed start answers `CameraUnavailable` (stick: always that; window: the specific reason when known, else that) (`faraday/shells/stick/src/main.rs:1669-1689`, `faraday/shells/window/src/lens.rs:44-52`, `faraday/shells/window/src/camera.rs:63-69`).

## State and lock

- `ScanState` (`faraday/faraday-core/src/qr_view.rs:111-161`), held in `Camera.scan`:
  - `frame` and `chroma` are `Zeroizing` and wiped again in `Drop` (`faraday/faraday-core/src/qr_view.rs:320-331`);
  - `single` (the last single code's bytes) and `card.code` are `Zeroizing`;
  - `masked` is `Text`, which wipes on drop (`core/faraday-text/src/text.rs:363`);
  - `decoder` (`faraday_codec::ur::Decoder`) wipes what it read on drop (`core/faraday-codec/src/ur/mod.rs:398-401`, `core/faraday-codec/src/ur/fountain.rs:236-237`);
  - `assembler` holds BBQr parts as `Zeroizing` buffers, and the Specter `Collector` has a `Drop` (`faraday/faraday-qr/src/bbqr.rs:175-182`, `faraday/faraday-qr/src/specter.rs:41`).
- `Camera` (`faraday/faraday-core/src/scan_key.rs:~42`) also holds `parts` (a transfer in parts, kept across a camera close), `carried` (a TOTP setup code carried through Unlock), `slot` (a Create slot waiting for a key), the camera list, the chosen camera and a `scanned` counter.
- Lock (`impl AtLock for Camera`, `faraday/faraday-core/src/scan_key.rs:2895`): `holds()` is typed when a carried setup code, a card secret or kept parts exist; `lock()` runs `clear_kept` (drops `parts`, `carried`, `slot`) and sets `scan = None` (drops frame, chroma, decoder, single, card). `scan_drop_at_lock` (`~1490`) closes the sheet and camera. Any lock drops the camera scan whole, before waiting on a seal.
- Leaving a page: `read_carried_tidy` wipes the carried setup code unless on Unlock; `seed_carried_tidy` wipes the carried passphrase unless the Seed scanner or `Sheet::KefRead` is up; `read_card_tidy` and `read_again_end` close the scan when the sheet is gone; `read_again_cancel` wipes the card's code (`~2126`).
- A scan's UR, BBQr and numbered parts are wiped at its end or restart; a new UR animation starts afresh.
- `QrView` (`faraday/faraday-core/src/qr_view.rs:~334-370`) wipes its source, title, subtitle, frames and `whole` on drop (`faraday/faraday-core/src/qr_view.rs:373-400`). Its `secret` flag is set from `wallet::classify` for File sources only. `QrArea` (`~799-805`): the view and a held press; a lock closes the view; `holds()` is NOTHING because its source (vault or key) counts it (`faraday/faraday-core/src/qr_view.rs:1696-1703`).
- Secrets from a scan are never filed (F167): `scanned_secret` (`faraday/faraday-core/src/scan_key.rs:~1100`) zeroizes `item.bytes` after taking the text; TOTP setup codes and vault entries are never filed (`~1148`).
- Camera frames in the shells: the stick's `Queued` frames are `Zeroizing<Vec<u8>>` (`faraday/shells/stick/src/camera.rs:92-118`); the camera crate's `Frame::new` wraps luma and chroma in `Zeroizing` (`faraday/shells/camera/src/lib.rs:26-41`); V4L2's drop zeroes the mapped buffers, then unmaps (`faraday/shells/v4l2/src/lib.rs:549-582`). The stick loop keeps only the newest frame per pass.

## Reach

- Opened by: Home's Scan, every form's own Scan (seed, key part, codex32, Krux), Check my copy and Convert, the GPG restore, Sweep, Sign a message, the OTS tools, multisig setup, vault entries, the address check, Spend and the Spend tab (a PSBT scanned during a spend is checked by its unsigned txid and collected).
- Dispatch order (`scanned_here`, `faraday/faraday-core/src/scan_key.rs:~1261`): message, tools, codex32, GPG restore, sweep, signer check, seed compare, OTS, multisig, then purpose.
- Opens: Add a key, the passphrase sheet, Restore, Spend review (a PSBT to sign has no card), Decode, Sign a message, Lightning, Unlock, Make a vault, Files (Keep).
- The QR view reads `stick::keys_load`, `public_files::wallet_code_head` and `wallet_file_text`, the `message` area, `vaults.store()` (TOTP, reveal), `gpg::paperkey_shown` (`faraday/faraday-core/src/qr_view.rs:1080, 1213, 1561`), `flows.restore_*`, `wallets.spend()` (`nonce_hold`, `out_bytes`, `finished_hex`).
- The boot import and the stick visit read picture files with the same decoder (`boot-import`, `files-inbox`).
- The device check holds the camera's reveals (`devices-inputs`); a scanner waits for a stick and held devices like every key load.
- Windows camera (`faraday/shells/mediafoundation/src/camera.c`): `faraday_mf_list` and `faraday_mf_capture` return 0, 1 (no camera), 3 (failure without HRESULT) or a negative HRESULT; Media Foundation's DLLs are loaded from System32 at call time, never imported (an N edition has no camera); the device is chosen by symbolic link (without case) or the first; the format is the device's own nearest 1280 x 720, asked as NV12 with the reader's video processing; frames are handed over only when width and height are even, rows cover the height and the UV plane is long enough, else the callback gets NULL; a negative stride is refused; every call releases all it made (`:54-70`, `:110-143`, `:196-476`).

## Rules and refusals

Reading (`faraday/faraday-core/src/qr_view.rs`, `faraday/faraday-core/src/scan_read.rs`):
- Scanner refusals (`scan_refusal`, `faraday/faraday-core/src/qr_view.rs:~591-620`): too long, BBQr malformed variants, another transfer, conflict, too large, BBQr text or JSON, envelope error (`ur_feed`, `~285-308`). After a refused BBQr set or a UR message failing its checksum, the next good one is read.
- Too fine: a code under 25 tenths of a pixel per module for 3 passes (`TOO_FINE_PASSES`) says so (`~27-31`, `~624-630`); only a whole-frame pass can say it.
- `ScanCamera(i)`: out of range refused; the list is replaced only if different (`faraday/faraday-core/src/scan_key.rs:~330-364`).
- Classification (`Kind`, `faraday/faraday-core/src/scan_read.rs`):
  - SeedQR is 48 or 96 digits (`~320`); WIF by `is_wif` (`~323`); `nsec1` is Nostr (`~326`);
  - 12, 15, 18, 21 or 24 words on a BIP-39 list: a seed if the checksum holds, else a 24-word aezeed if its checksum holds, else bad checksum (`~337-350`); words from any list Faraday reads are accepted;
  - codex32 secret versus share (`~352-357`); `ur:crypto-seed/` and `ur:seed/` are UrSeed (`~363`); otpauth migration and single otpauth are Totp, other entries are Entries (`~366-375`); paperkey (`~273`);
  - Compact SeedQR is 16 or 32 bytes no text kind took (`~277-289`); a bare xprv is one line without whitespace, `(`, `[`, `]` or `/` (`~308-311`); hex digit text is plain text on every scanner;
  - Krux (KEF) by `kef::recognize` (`~472`), asked only after `secret_kind` (a SeedQR that passes the Krux test is a SeedQR);
  - `sp1` and `tsp1` silent addresses (`~497`); an address is 26 or more characters, no whitespace, a network prefix (`~502-509`); invoices `lightning:`, `lnbc`, `lntb`, `lno1`, `lnr1`; web `http(s)`; a labels file is every line a `{..}` with "type" and "ref" (`~521-527`); printable text, else Unreadable;
  - an enveloped file is always File (`~437-449`);
  - message scanners refuse a key written as text (`~294-303`);
  - a key or wallet file with seed words or a private key beside it is named as holding a secret, not filed, not an account key; an xprv without an origin in scanned text is refused.
- Routing (`Kind::route`, `faraday/faraday-core/src/scan_read.rs:~184-201`): whole words or codex32 load; a codex32 or SLIP-39 part goes to its form; a whole Krux code to a sheet; whole SLIP-39, Seed XOR, parts of words or Krux, a hex seed and an xprv part are `Other`; a whole xprv is a line with the typed form's own refusals.
- Card: `Kind::Wif` offers only Sweep; Aezeed only Lightning; Text is masked behind an eye with Keep, Note and SignMessage (single-key wallet, valid UTF-8); unknown kinds have no ways (`faraday/faraday-core/src/scan_key.rs:~1537`). `KeyWithSecret` is not offered "Use key". Keep is always removed from Home's card.
- `Key`, `Share`, `Sweep`, `Open` and `AddTo` load a key or open a passphrase and wait until the stick is out (`faraday/faraday-core/src/scan_read.rs:~644-649`).

Key loading (`faraday/faraday-core/src/scan_key.rs`):
- `may_load_keys()` gates `ScanPart`, `ScanSeed`, `ScanKef` and `ScanKey` (`faraday/faraday-core/src/scan_key.rs:633, 642, 651, 660`), key-loading `ReadGo` ways (`faraday/faraday-core/src/scan_key.rs:1716`) and seed-file paths (`~1131-1133, 1966, 2453-2475`). The refusal line is `stick::keys_load(medium)` (`faraday/faraday-core/src/qr_view.rs:1080`).
- `ReadGo` with a key-loading way and a stick in returns silently; the card stays, no note (`faraday/faraday-core/src/scan_key.rs:1716`). `ReadAgain` is refused while a stick is in (`~2075`).
- A code left with no seed and not on Seed check is dropped silently after the camera closed (`read_go`, `~1729-1760`).
- A storage device the disk process has not listed but the kernel shows blocks scanning a key and typing a passphrase; a setup code or entry waits for the stick to be out before going into a vault form.
- Scanners turn the camera on only once no stick or held device is in.
- `ScanKey(w)` (`w < session.wallets.len()`): a seed matching no key of the wallet is refused `scan_key_is_not_one` and goes to the passphrase sheet; a key already held says so; a codex32 or SLIP-39 part goes to Add a key for that wallet (`~529, 2318`). It leaves with the wallet (`~209`).
- Copy check (`copy_scanned`, `~2624`): must decode as SeedQR; the seed must still be in the session, else the camera closes silently; only `Matches` closes the camera; once the result has moved on, the camera closes with no answer. Convert (`~2674`): the right key and kind; digits versus compact mismatch refused.
- Codex32 only on the Seed scanner, not while a stick is in (`~1182`). `file_scanned` refuses a non-enveloped Other file (`~2519`). Home's Scan sends a seed or master xprv to Add a key (with a stick in, waits or is refused); a pasted seed is taken as a scanned one. A Krux count above what Krux writes is asked once more (`~1820`). A Krux code read by a scanner a seed flow opened lands where a SeedQR read there lands.
- GPG restore: the public key must be an armoured public key, else refused on the scanner (`faraday/faraday-core/src/qr_view.rs:~1043-1054`); a paperkey scan only inside a restore that is typing and when keys may load (`~1006`); a paperkey read is refused while a stick is in (`~1079-1080`) or outside a restore (`~1083-1085`); it returns silently when not allowed (`~1000-1008`). A scanned GPG key is filed as `{name}-{fp8}.asc`.
- A PSBT scanned during a spend is checked by its unsigned txid and collected; the same file twice changes nothing; another transaction is refused with a toast naming Cancel.

Paste (`faraday/faraday-core/src/scan_key.rs`):
- Only on a clipboard shell (desktop Demo; never the device); no Paste button is drawn on a shell without a clipboard.
- Refused when no scanner pastes, when empty, or when the text is a secret: word-list words (except on seed scanners); `secret_text` Words, xprv, silent spend key, WIF; SeedQR digits of 48 or 96; codex32 or SLIP-39 shares (non-seed scanners). Seed-loading scanners refuse only private keys. Sweep accepts a pasted WIF (F171).
- Unavailable on VaultEntry, CheckCopy, CheckConvert, CheckCodex32, CheckKef, Kef, GpgPaperkey (`~925`).
- Paste words (`~871`, `~744`): clipboard shell, not Mainnet, the Words form, no sheet over the page, keys may load; 12, 15, 18, 21 or 24 words, each on the list, the checksum holding.

QR view (`faraday/faraday-core/src/qr_view.rs`):
- Codes (`Code`, `~408-462`): wallet descriptor, multisig config, BSMS record, Specter DIY command, account keys (plain, BIP-129), wallet keys, silent-payments values, signed message, GPG key and signature, receive address, vanity address, Explore address, pubkey and xpub, Lightning node id, vault sheet. PNG names: `{stem}-descriptor.png`, `xpub-{fp}.png`, `xpub-{fp}-bsms.png`, `message-{addr6}.png`, `lightning-{id8}.png`, `silent-*`, `{name}-specter-diy.png`.
- Refusals (`code_view_inner`, `~1174-1354`; `value_code`, `~1362-1467`): no such wallet; a silent wallet's descriptor while its key waits for backup; config or BSMS without file text; a key BSMS without a BIP-129 record; a cosigner key of a silent wallet; a signed message while its key waits for backup (`message_waits_backup`) or none; a receive address if the wallet is silent, its key waits for backup, or it cannot be derived (F31); a vanity address whose key is held; an Explore value whose key is held (`explore_unheld`); an unreadable node id; a vault sheet with an empty descriptor; the default arm refuses with `code_view_no_address`. A key without origin is labelled, not refused.
- `QrSigned` and `QrPart` are refused while the spend's nonce hold is on (`~905`); an unwritable PSBT toasts `spend_v2_not_written`; `QrWallet(i)` out of range is refused; a `GpgSignature` code is refused while the time is not asked (`~933-934`); `QrOutbox(i)` is refused while the message file waits for its key backup, and unless the file fits a QR (`qr_fits`).
- PNG (`offers_png`, `~779-781`): only public, not secret, a single frame, a PNG name set; a signed message is refused while its signer is held (`~1617-1619`); a public multi-frame view gets a toast.
- `QrView::kef` is refused when the Krux text does not fit one code at version 25. A secret PSBT is shown as UR only; text is one code if it fits version 25, else animated `ur:bytes`.
- A PDF is made only online.

Camera and decoders:
- Picture files: at most 4096 x 4096, refused from the header before decoding; a picture too large for a 32-bit address space is refused; a picture with too many codes is refused at once with a reason.
- A UR stream not resolved within a bound stops and restarts; parts kept per fragment are bounded; held index sets are a few kilobytes; decoding does not recurse; CBOR counts and reservations are bounded. A SLIP-39 share past the set size is refused.
- The vendored QR decoder (`third_party/rqrr/`) has one work budget per frame; arithmetic on crafted input does not wrap; out-of-range numeric or alphanumeric groups fail as DataOverflow; closing the scan sheet ends a search at its next budget check.
- Camera frame helpers (`faraday/shells/camera/src/`): `Frame::from_nv12` is None when the luma plane is shorter than its rows claim or the result does not fit `u16`; missing or short chroma is dropped silently (`faraday/shells/camera/src/lib.rs:~51-77`). `nv12::luma` and `nv12::chroma` refuse a zero size, a small stride or a short plane (`faraday/shells/camera/src/nv12.rs:~17-47`). `scale_factor` and `max_side` are at least 1; `downscale` and `downscale_chroma` return the input unchanged on a factor of 1 or less, a zero result size or a short buffer (`faraday/shells/camera/src/scale.rs:34-37`; `~72-75`). `list()` lists only `/dev/videoN` whose sysfs index is 0; the name falls back to the node name (`faraday/shells/camera/src/lib.rs:~99-121`). Failure reasons `Failure::{Refused, InUse, NotFound}` (`faraday/shells/camera/src/lib.rs:~80-90`).
- V4L2 (`faraday/shells/v4l2/src/lib.rs`): `find` and `find_or_failure` scan `/dev/video0..31` (`~152-186`); the first capturing node wins, colour preferred; no device chooser. Formats GREY, YUYV, NV12 and YU12 only; colour first at or below 1280x720, planar first above (YU12, NV12, then YUYV, GREY at 1080p); the node must report VIDEO_CAPTURE and STREAMING; GREY-only nodes are kept only if there is no colour node; probing uses TRY_FMT; a substituted format is accepted only as a fallback and only if luma-readable; an unusable size (0, or `stride*height > 4096*4096`) is refused; fewer than 2 buffers refused, and a buffer length of 0 or above the maximum refused (`faraday/shells/v4l2/src/lib.rs:497-516`); a bad buffer index ends the thread; damaged and short frames are dropped and requeued; a full channel drops the frame; `set_max_fps(0)` is raised to 1; ENOENT, ENODEV and ENXIO are NotFound, EBUSY InUse, EACCES and EPERM Refused, and the search keeps the strongest (`~156-192`, `~205-212`). `MAX_WIDTH` is 1920 (`faraday/shells/v4l2/src/convert.rs:24`, `faraday/shells/v4l2/src/lib.rs:446-447`). Conversion returns None on a width, height or step of 0, a stride below the row length, a source shorter than claimed, or an overflow; `chroma` is None for GREY.
- Media Foundation (`faraday/shells/mediafoundation/src/lib.rs`): NV12 needs an even width and height; `MAX_WIDTH` 1280; `on_frame` refuses width 0, height under 2, odd dimensions, a stride under the width; a rate cap `MAX_FPS` 10; a full channel drops the frame; a non-OK status sends `Message::Unavailable` once; HRESULT mapping `E_ACCESSDENIED` Refused, in-use codes, not-found codes (`~64-105`, `~295-346`, `~371-404`). `devices()` lists name and link (`~214`).
- Window shell: on platforms other than Linux and Windows every `CameraOn` is refused at run time (`faraday/shells/window/src/camera.rs:~405-437`). A non-frame camera answer drops the scanner (`faraday/shells/window/src/lens.rs:89-92`). Choosing a camera while running restarts it; on failure it only prints (`faraday/shells/window/src/camera.rs:189-198`). A camera's first stream is not restarted; a damaged frame is skipped.
- A scan asked for Any, Sign, Decode, Address or Invoice hands every whole code to the file intake (qr_view.rs:194-199, scan_read.rs:739, scan_key.rs:1526); no caller screens for secrets. The intake does, for all of them: on the Scan sheet, a code that is not an enveloped file and reads as vault entries, seed words, a seed part, a wallet or key file with secret text, or a private-key line is not filed and its bytes are wiped (scan_key.rs:1100-1140).
- Frame caps: V4L2 shrinks by an integer factor so the longer side is at most 1920; Media Foundation caps the width at 1280 (`faraday/shells/v4l2/src/convert.rs:24`, `faraday/shells/mediafoundation/src/lib.rs:61`). With no camera chosen the V4L2 search tries `/dev/video0..31` read-write and takes the first colour capture node, else the first grey-only one (`faraday/shells/v4l2/src/lib.rs:169-190`).

## Domain calls

- `faraday_keys` (identity `fingerprint_of_words` and seed, `is_wif`, `codex32_seed`, `words::from_indices`, SeedQR decoders), `faraday_bip` (bip39, codex32, slip39, aezeed, address prefixes; `core/faraday-bip/src/`), `faraday_intake` (`secret_text`, `seed_in_code`, `seedqr_words`, `arrival`), `faraday_codec` (ur, kef, classify; `core/faraday-codec/src/`), `faraday_qr` (Assembler, Step, `classify_text`, registry, `send::frames`; `faraday/faraday-qr/src/`), `faraday_pgp::read_public_armor`, `faraday_vault` (`otpauth_names`), `faraday_crypto` Compute, `encode_bounded`, `picture::labelled_png`, `sha256_of`.
- Shells: `faraday_v4l2::{Camera, find_or_failure, spawn_reporting, Failure, Size::{FULL_HD, HD, VGA}, Frame}`, `faraday_camera::list`, `faraday_scanner::{Scanner, ReadMode, reduce, reduce_factor, reduce_chroma}` (`faraday/faraday-scanner/src/lib.rs`), `zeroize::Zeroizing`.

## Fixtures

- `faraday/faraday-qr/tests/vectors/README.md`
- `faraday/faraday-qr/tests/vectors/faraday-os-bbqr-z-key.txt`
- `faraday/faraday-qr/tests/vectors/faraday-os-bbqr-z.txt`
- `faraday/faraday-qr/tests/vectors/faraday-os-envelope-2.json`
- `faraday/faraday-qr/tests/vectors/faraday-os-envelope.json`
- `faraday/faraday-qr/tests/vectors/wallets/multisig-5-psbt.png`
- `faraday/faraday-qr/tests/vectors/wallets/multisig-6-psbt-bbqr-part-1-of-10.png` to `multisig-6-psbt-bbqr-part-10-of-10.png` (ten files)
- `faraday/faraday-qr/tests/vectors/wallets/psbts.txt`
- `core/faraday-codec/tests/large_code_payload.txt` (large-code test of the codec, `core/faraday-codec/tests/large_codes.rs`)

The `faraday/faraday-qr/tests/vectors/` files are read by `faraday/faraday-qr/tests/interop.rs`.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 380acde80 | A lock left the camera's scan up: picture, code read, sheet and camera stayed on. | Any lock drops the camera scan whole, before waiting on a seal. |
| 30d954098 | A Krux code read by a scanner a flow opened for a seed did not go where a SeedQR goes; cancelling its passphrase left the person in the flow. | A Krux code lands where a SeedQR read there lands; Cancel leaves the flow. |
| 7cf4bfa99 | A SeedQR or CompactSeedQR that also passes the Krux test opened the Krux sheet at a form's own Scan. | `secret_kind` is asked before the Krux test. |
| 2503d28d5 | The passphrase eye stayed open after a Krux check reached by scan. | The eye closes on the scan path as on Check. |
| 87ac43fd0 | A read card's "Add this key" no longer loaded a seed code. | A seed code loads by Add this key beside the xprv arm. |
| 02980ab51 | Hex digit text was read as a key line, called a private key, asked a passphrase, or loaded by the boot import. | Hex is plain text on every scanner; only a bare xprv is a key line. |
| 5d815e0ec | A descriptor or key after an origin holding an xprv was named a private key; 64-digit hex was refused as secret instead of shown as QR. | Only a bare xprv goes to the xprv form; hex is held only as a file. |
| 81fe89071 | rqrr misread a dense (version 30) code whose data had a corner-square lookalike. | A capstone group must be as far apart as its timing says. |
| 543b2dc6f | A code seen at an angle lost its length check in rqrr. | Each arm is measured in module axes from both corner squares, averaged. |
| 0550cd996 | rqrr's `hypot` linked libm, about 51,600 extra instructions per start. | Square root gives the same length; cost back. |
| 2d49d0d20 | A read card stayed on the session pane after a sheet the page opened ended the scan; a tall card was not cut and scrolled. | The card is kept only under the sheet its Scan again raised; a tall card scrolls. |
| f838f1561 | A UR stream that kept giving new parts never resolved and kept growing. | A stream not resolved within a bound stops and restarts; parts per fragment are bounded. |
| 5d453340c | A very large picture for a QR read could take about 320 MB of the disk process. | A picture is at most 4096 x 4096; a larger one is refused from its header. |
| e83379073 | A picture too large for a 32-bit address space was read at a wrapped size. | Refused as too large from its header. |
| 3bd534888 | A picture with too many QR codes made the disk go silent for a minute. | Refused at once with a reason. |
| 3c3d24679 | A secret's QR text, rows and fountain fragments stayed in memory after the view closed or a PNG was saved. | A secret's QR and its encoder's buffers are wiped with the view. |
| 5e5913398 | Out-of-range numeric or alphanumeric groups decoded as other characters; a capstone sort could panic on NaN; a scan span of 2^30 was not refused. | DataOverflow (ISO/IEC 18004); a total order; a span at the limit is refused. |
| f50a487bb | A scan's UR, BBQr and numbered parts stayed in memory after a scan ended or restarted; SLIP-39 shares had no bound. | A scan's parts are wiped at its end or restart; a share past the set size is refused. |
| 41f645d48 | The bytes decoded from a QR frame (possibly a SeedQR) were not wiped when dropped. | Secret-bearing decoded bytes are wiped when dropped. |
| 66729551c | A pasted seed's words were refused unless English. | Words from any word list Faraday reads are accepted. |
| 73211253c | Home's Scan refused a seed pasted on Faraday Demo. | Home's Scan takes a pasted seed as it takes a scanned one (F75). |
| 6c935d371 | On the stick and the Pi, scanners drew a dimmed Paste button that did nothing. | Paste is drawn only where the shell has a clipboard. |
| 7b969df98 | A scanned key or wallet file with seed words or a private key beside it was filed as an account key or a plain file. | Such a file is named as holding a secret, not filed, not an account key. |
| 31147119b | A setup code or entry read by the scanner was added to a vault form while a stick was attached. | It waits for the stick to be out. |
| db0b428b9 | "Scan the description" opened a scanner titled "Scan what you have". | The scanner is titled for what its opener asks. |
| a9e168e4d | A scanned GPG public key was filed under a name that clashed with other keys. | Filed as `{name}-{fp8}.asc`. |
| 10ed8c8f8 | Check my copy's camera answered when a code arrived after the result had changed. | Once the result has moved on, the camera closes with no answer and no note. |
| 3c1f1d940 | A GPG key's two Scans and the read card's Scan again turned the camera on while a stick or held device was in; Send text let through forms it should refuse; Specter parts lost a trailing space; a non-UTF-8 UR message to sign was accepted. | Scanners wait for the stick and held devices; Send text and scanners fail closed; a message to sign is signed as scanned. |
| ee639a4da | A CompactSeedQR whose entropy bytes happened to be valid text was read as text. | 16 or 32 bytes that no text kind takes are a CompactSeedQR, in the classifier and seed parsers alike. |
| 59f8b56bc | A wallet or key file holding seed words or an xprv in a JSON member, comment, heading or Specter name was filed as ordinary; an xprv without origin in scanned text was not refused. | Such a file is a secret file, refused where not allowed; an xprv without origin is refused. |
| 14a7ba01d | Home's Scan of a PSBT during a spend did not check it against the spend; it went to Files. | Checked by unsigned txid and collected; another transaction refused with a toast naming Cancel. |
| 412040f8b | The Spend tab's PSBT scan had the same gap. | Same txid check; the same file twice changes nothing; a different transaction is refused. |
| c26bc59e4 | A camera, disk read, part file or signature with sizes beyond a 32-bit address space was read at a wrapped size. | Such sizes are refused. |
| 1e0d0545d | The QR decoder's line scan overflowed at the largest or smallest i32. | No overflow; the scan ends at its last point. |
| c934529ce | A crafted picture could stall the scan of a timing pattern. | The scan ends with no read once the frame's work budget is spent. |
| 19ee1e6bb | Scanners carried wrong titles for what they took. | Each scanner titled by its opener's Ask; the seed sheet names form, wallet and Seed XOR. |
| 1c44ba701 | The stick and the Pi asked the camera for 640 x 480, too coarse for a dense QR. | 1920 x 1080 where available; a whole-frame read in both polarities; too fine said only from the whole frame. |
| 3ee74aeb9 | A dense QR did not read in a high-resolution frame at the module size. | A dense code reads at its module size. |
| b94b31e0c | A dense QR held close and tilted 35 degrees or more did not read in enough frames. | Dense codes read at the tilt. |
| 0e8d55d2a | A frame with tens of thousands of corner marks stalled the scanner. | Bounded time with no read; checked arithmetic. |
| 65214c1db | The camera gave no usable frame at the first size and the scanner stayed dead. | A ladder 1920 x 1080, 1280 x 720, 640 x 480, each after 2.5 s without a frame. |
| 9b1ba5f88 | On the Pi camera, 1080p with YUYV gave no frame. | At 1080p planar formats are asked first. |
| 89c183297 | A slow camera showed a false "no picture" at 6 s. | Ladder timing kept below the no-picture time. |
| 77b01a7b7 | Camera ladder and no-picture timing (false "no picture" risk). | 2.5 s first-frame wait at the first two sizes, no limit at the last; no-picture note after 9 s. |
| 63406cdc3 | A crafted camera frame made the QR search repaint regions over and over. | One work budget per frame for all flood fills, evictions and copies. |
| 20a594797 | A reduced copy of a frame could decide a code was too fine. | Only a whole-frame pass can say too fine. |
| b59f02fc7 | A crafted frame could wrap the scanner's arithmetic. | Area in i128, step checked, side in f64, corner in i64 and clamped. |
| f5d745135 | A 16 or 32 character paperkey scanned for Sign a message was not named a paperkey; scanners read 16 or 32-byte values differently. | Same naming in both scanners; only ASCII whitespace at the edges is trimmed. |
| 40ded47d4 | A BBQr set refused at its end or in conflict, or a UR message failing its checksum, left the scan stuck. | The next good BBQr set or UR message is read after a refusal. |
| e4f4f1589 | A hostile animated UR made the decoder hold hundreds of megabytes and recurse; a CBOR message could reserve memory for a declared count. | Held index sets are a few kilobytes; no recursion; CBOR counts bounded. |
| 74cd78c7d | A tall camera frame was shrunk only by width; closing the scan sheet did not stop a crafted frame's search. | Reduced by both dimensions; closing ends the search at its next budget check. |
| b3fddbb4c | The V4L2 camera did not ask for 1280 x 720. | Cameras request 1280 x 720 and pass frames up to 1280 wide whole. |
| 221fdbd22 | The desktop scanner did not read white-on-black codes and cut 1280 x 720 frames. | The desktop reads white-on-black codes and whole 1280 x 720 frames; the stick and Pi keep the reduced read. |
| 93481274f | Scanning a key or typing a passphrase in the Inbox was allowed while the kernel showed storage the disk process had not listed. | Such a device blocks scan and typing. |
| 24e47f389 | Pulling the stick by its pill on the Pull sheet did not turn the camera on for the waiting Scan; Linux capture restarted a silent stream. | A waiting Scan gets its camera on when the stick change is told; a first stream is not restarted. |
| 6b7c05050 | A new UR animation kept the previous one's parts; progress did not name missing parts; a QR seen but never read gave no hint. | A new animation starts afresh; progress names missing parts; the decoder's refusal is shown. |

## Doubts

- Left out: 16b4dcadd (a test-only build cast, not user-visible). The Electrum keystore part of 40ded47d4, the certificate part of e83379073 and the AVFoundation and macOS camera parts of b3fddbb4c and 221fdbd22 are not about this feature (macOS is not developed); b3fddbb4c's files carry an older crate prefix.
- 30d954098 includes the follow-up 67e068d42. 2503d28d5's diff shown was docs only. e4f4f1589 and 74cd78c7d were not inspected beyond their messages. 0550cd996 is a cost fix with no wrong output.
- `Camera::keep_parts` (`faraday/faraday-core/src/scan_key.rs:~297`) has no call site in its file.
