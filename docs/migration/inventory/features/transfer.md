# Sending files and text as codes

Transfer is only in the online app: Faraday Demo, and the desktop app the
Starter holds. It moves files and text between an online computer and the
offline device by QR codes, and does not use a stick:
- **Send** lists the computer's Downloads folder. It shows a picked or
  dropped file as a run of codes.
- **Receive** reads codes with the computer's camera. It saves each whole
  file into Downloads.
- **Send text** shows typed or pasted text as one QR code, unless the text
  is or holds a secret.

Nothing received goes to the Inbox or the session. It is reached from the
sidebar (`Screen::Transfer`), from the Demo's `--transfer` option, from a
file dropped on the window, and from the Starter's Transfer press.

## Screens and sheets

- `Screen::Transfer` (`faraday/faraday-core/src/transfer/screen.rs`):
  - Send: the Downloads folder's files with their sizes. A file over the limit has a warning-coloured row that can still be pressed.
  - Receive: a button. Notes of saved files (`saved`), "already saved" and refusals.
  - Open the folder: only when the shell can open folders.
- `Sheet::Scan` with `ScanPurpose::Transfer`: the Receive camera.
- The codes of a file: `QrView` (`faraday/faraday-core/src/qr_view`), with part notes.
- Send text card (`faraday/faraday-core/src/transfer/text.rs`, `faraday/faraday-core/src/transfer/screen.rs`):
  - A field that grows from 3 to 8 lines (`faraday/faraday-core/src/transfer/screen.rs:~134`, `~148`, `~177-180`).
  - A check line (`~190-214`): `TooLong`, `Secret(k)`, `Partial`, `SeedHex`, `Inside(k)` or `Fine`.
  - Paste, Clear, and Show as QR. Show is primary only when the text can be sent (`~233`).
  - The QR is shown as text (`QrView::text`, titled `EN.transfer_text_sheet_title`).
- The "Hold to save without encryption" sheet for a received secret (`secrets::Plain`).

## Actions

- `TransferSend(i)`: picks file `i` (`transfer_pick`, then `send`).
- Drag a file over the window, then drop it. `StorageEvent::Hovered` marks the drop target. `StorageEvent::Dropped { path, size }` opens Transfer and sends the file.
- `TransferReceive`: opens the camera. Skip is taken only while that scan is open.
- `TransferOpenFolder`: only if the shell can open folders.
- `TransferText(TextPress)`: Paste, Clear, Show.
- `TypeHere(Here::TransferText)` puts the typing focus on the text field.
- Keys in the text field (`transfer_key`): characters, Enter (a newline), Backspace.
- On a received secret: the hold "Hold to save without encryption" saves it.

## Effects

Line numbers are in `faraday/faraday-core/src/transfer/mod.rs` unless named.
- Storage commands:
  - `ReadHost { path }` (`~231`).
  - `SaveDownload { name, bytes }`; the bytes are moved (`~381-386`).
  - `OpenDownloads` (`~414`).
- Storage events taken: `Hovered`, `Dropped`, `HostRead`, `HostReadFailed`, `Saved`, `SaveFailed` (`~486-499`). The listing arrives through `transfer_listed` (`~544-551`).
- Clipboard: `Command::RequestClipboard { kind: FileKind::Text }`, only when the shell has a clipboard (`~457`).
- Camera: `camera.open` and `Command::CameraOn` (`~561`, `~563`).
- Secret hold: `plain_ask(secrets::Plain { .., then: PlainThen::Transfer(name, data) })` (`~576-581`).
- Digest: `faraday_crypto::sha256_whole` over name, extension and bytes, used to drop a repeat (`~339`).
- The host side, in `faraday/faraday-storage/src/lib.rs`:
  - The Downloads listing, `read_host` and `save_download` (`~730-863`, `~1264-1304`).
  - A saved file never overwrites another. It becomes `name (2)`, `name (3)` and so on, up to 999 tries with `create_new` (`:832-863`).
- In Faraday Demo:
  - The Downloads listing thread feeds the screen every 700 ms (`faraday/shells/demo/src/lib.rs:~278-302`).
  - A dropped file is sent (`faraday/shells/demo/src/lib.rs:~305-312`, `faraday/shells/demo/src/main.rs:~349-360`).
- In the Starter, the Downloads listing is rescanned every 700 ms and sent only on change (see `starter-app`).

## State and lock

- `TransferState` (`faraday/faraday-core/src/transfer/mod.rs:~40-64`):
  - `dir`.
  - `files: Vec<(Text, u64)>`.
  - `opens`.
  - `reading: Option<Text>`: the path asked for.
  - `refused`.
  - `received: u32`.
  - `saving: VecDeque<bool>`.
  - `last: Option<[u8; 32]>`: digest of the last file saved or held; cleared on `receive_begin`.
  - `saved: Vec<(Text, bool)>`: names or paths of saved files. Not wiped at lock; it holds no secret.
- `TransferArea.hovered`.
- `TransferArea.text: SendText` (`faraday/faraday-core/src/transfer/text.rs:~29-35`):
  - `text: Text`, up to `HOLD` = 4 × `MAX_TEXT` bytes.
  - `check: TextCheck`.
  - `over`.
  - The text may be a seed or a key someone typed.
  - `Text::clear` wipes the whole buffer (`core/faraday-text/src/text.rs:301-303`), and `Text` wipes on drop.
  - `Debug` prints only the length, the check and the flag.
- Bytes read from a file and received bytes are wiped after use, or kept in `Zeroizing` (`faraday/faraday-core/src/transfer/mod.rs:~106-108`, `~155-180`, `~250-263`, `~326`, `~340`, `~375`, `~589`).
- Lock: `impl AtLock for TransferArea` (`faraday/faraday-core/src/transfer/mod.rs:653-661`). `holds` is `Holds::NOTHING` and `lock` is `clear_text()`. The lock then restarts the app: the Demo's restart, or the Starter's fresh Demo.
- Leaving `Screen::Transfer` clears the text, in the Demo only (`faraday/faraday-core/src/tools/mod.rs:~763-766`).

## Reach

- Opened from: the sidebar of the online app, the Demo's `--transfer` (`faraday/shells/demo/src/main.rs:~455-457`), a file dropped on the window, and the Starter's Transfer (`Action::Nav(Screen::Transfer)` in the held app).
- Uses:
  - `qr_view::{qr_source, part_note, transfer_name, scan_refusal, ur_note, ur_feed, UrFed, QrView, open_qr}`.
  - `wallet::{read_psbt, classify}`.
  - `secrets::{Plain, PlainThen, Exposure}`.
  - `crate::arrival` (`faraday/faraday-core/src/transfer/mod.rs:86`), `seed_in_code`, `scan_read::secret_kind`.
  - `at_lock::{AtLock, Holds}`, `text_check::{MAX_TEXT, TextCheck, check}`, `camera`, `screens::{size_text, section_label}`.
- Host side: `faraday-storage` (see `storage` and `desktop-shells`).

## Rules and refusals

Line numbers are in `faraday/faraday-core/src/transfer/mod.rs` unless named.
- Online app only:
  - `transfer_open` (`~427`).
  - `transfer_text_here` (`~441`), which also needs `Screen::Transfer` and no sheet open.
  - `transfer_pick` (`~479`), `transfer_dropped` (`~510`), `transfer_receive` (`~555`), `open_folder` (`~413`), `transfer_hovered` (`~505`).
- Send:
  - Refuses a file over `MAX_SEND` (= `faraday_qr::envelope::MAX_FILE`) with `transfer_larger_than`.
  - Refuses an empty file with `transfer_is_empty`.
  - Lines 215-232.
- A read answer for a path not asked for is dropped and its bytes wiped (`~249-252`).
- A read over the limit reads `MAX_SEND + 1` bytes and is refused (`~254-257`).
- A failed read shows the base name and the reason (`~275-281`).
- Receive, repeats: a file with the same name, extension and digest as the last one is not saved again (343-348). The digest is stored before the secret check. A declined secret is not offered again; reading it again gives the "already saved" note.
- Receive, secrets (356-364). The file is not saved but held for "Hold to save without encryption" when it is any of:
  - a registry `Private` or `Seed` (`~163-168`);
  - a `wallet::classify` result of `Exposure::Secret`;
  - a `seed_in_code` match;
  - a `scan_read::secret_kind` match.
- UR keys, from the history fixes (`faraday/faraday-qr/src/registry.rs`):
  - A UR key or account is refused where the Blockchain Commons specs refuse it:
    - a non-boolean hardened flag;
    - an empty key path;
    - a network other than mainnet or testnet;
    - a coin type other than 0 or 1;
    - a partial depth;
    - source fingerprint 0;
    - a one-step origin mismatch.
  - A private `crypto-hdkey` or `ur:seed` is a secret and waits for the hold. This holds even where the registry refuses it for another reason.
  - A CBOR map with a key given twice is refused (RFC 8949 §5.6), also when the same key number is written in a longer form (`core/faraday-codec/src/cbor.rs`).
- Kinds of saved file:
  - Bytes that are not UTF-8 are kept as a PSBT if they parse, else as `bin` (`~103-110`).
  - A UR of a kind not read, or one the registry refuses as not its shape, is kept as CBOR (`~172`).
  - A transaction is saved as `.txt` (`~85-90`).
  - A file without a name is saved as `received-{n}.{ext}` (`~349-355`).
  - A saved secret is said to be one (`~397-401`).
- `transfer_skip` only while the camera scan is open (`~600-604`).
- `transfer_paste` needs the clipboard (`~456`). An empty paste is ignored (`~470`).
- `Show` opens the QR only if `sendable()` (`~628-634`).
- `SendText` (`faraday/faraday-core/src/transfer/text.rs`):
  - Control characters other than Enter are refused (`~93`, `~106`).
  - Past `HOLD`, a character or Enter is dropped (`~94-101`).
  - A longer paste is cut at a character boundary and flagged `over`, which shows `TooLong` until `clear()` or a new `put()` (`~115-128`).
  - `sendable()` needs non-empty trimmed text and `TextCheck::Fine` (`~71-74`).
- Send text refuses what could carry a secret, and names the real kind (history fixes; `faraday/faraday-core/src/transfer/text.rs`, `faraday/faraday-core/src/scan_read.rs`, `faraday/faraday-intake/src/lib.rs`):
  - a word or key glued to a label;
  - a UR part of a secret-holding type, or a `ur:bytes` holding a secret;
  - bare hex of 32, 40, 48, 56 or 128 digits;
  - SeedQR digits in fours;
  - zero-width characters;
  - a paste longer than the field.
  - Bare hex of a seed's length is named as hex, not as a CompactSeedQR.
  - The classifier also refuses `ur:seed/` and `otpauth-migration://` exports.
- A paste longer than the Send text field holds is cut and the text is refused (too long) until the field is cleared; typing or Backspace does not lift it (transfer/text.rs:31-34, 90-131).

## Domain calls

- `faraday_qr::{envelope::MAX_FILE, Arrived, classify_text, Step, registry::{read, Read, Error}}`.
- `faraday_codec::ur::{is_ur, Message}`.
- `faraday_intake::Kept`.
- `faraday_crypto::{sha256_whole, work::Compute}`.
- `faraday_text::Text`.
- `faraday_shell_api::{Command, Key, FileKind}`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 620d45068 | A UR key or account was accepted with defaults where the Blockchain Commons specs refuse it. A UR with private key data was refused as "not registry shape" rather than as private. A private `crypto-hdkey` or `ur:seed` was saved to Downloads without the hold once its CBOR no longer showed a key. | A UR key or account is refused where the specs refuse it. A private key or seed on Receive is a secret and waits for Hold to save, also where the registry refuses it for another reason. |
| f211af7f6 | A CBOR map with a key given twice was accepted with the first value, so a `crypto-hdkey` with two key entries showed the first key. | A map with a duplicate key is refused (RFC 8949 §5.6), also in a longer form. The registry type is refused as not in its shape. |
| f28995a18 | Send text let through a word or key glued to a label, a secret UR part, a `ur:bytes` holding a secret, bare hex of 32/40/48/56/128 digits, SeedQR digits in fours, zero-width characters, and a paste over the field. The line named the wrong kind. The device classifier accepted `ur:seed/` and `otpauth-migration://` exports. | Send text refuses what could carry a secret, and the refusal names the real kind. |
| 078d38150 | Bare hex of a seed's length was reported as a CompactSeedQR. | The refusal names the kind the text actually is. |

## Doubts

- Two history rows given to this feature are left out because they are about other features:
  - 71cc2c003: a PSBT output to another own wallet shown as change. That is a signing review's "Transfer" kind, not this feature.
  - f0c7f6145: the disk process kept a vault transfer's pieces. That is the storage layer's background transfer.
- The storage layer's background transfer of vault files over one piece (16 MiB, `faraday/faraday-files/src/proto.rs:~37`) shares the word "transfer". It belongs to `storage`.
- `transfer_dropped` has a stale-looking doc comment above `transfer_hovered` (`faraday/faraday-core/src/transfer/mod.rs:~501-503`).
- Only the lines marked plain above were checked against the code.
