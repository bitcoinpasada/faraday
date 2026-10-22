# QR transfer and scanning: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/QR.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header and introduction (from docs/QR.md)

V0.2 · 2026-10-05 · partly built; §6 says what is and what is not.
§7 (2026-10-14, F28) is specified, not scheduled; build only when the owner says.
§8 (2026-10-21) is parity with Gauss's scanner.

Sources read for this file: Faraday OS's docs/gpg-qr-workflows.md and
`lib/faraday/qr_*.py`, `qr-transfer-gui.py` and `ur/` (the Foundation
Devices UR reference, vendored); `faraday-codec`
(`classify.rs`, `ur/`).

## §1 Where each piece stands (from docs/QR.md)

The table as written before the 2026-10-05 build; `docs/QR.md` §1 now
says what is.

`Have` means Faraday already does it. `Port` means the Wallets tab
needs it and nothing here has it yet. `New` means neither does it as needed.

### Transports

| Feature | Faraday OS | Faraday |
|---|---|---|
| BC-UR single part | yes | Have |
| BC-UR multi-part, fountain mixtures written (rateless) | yes | Have |
| BC-UR mixtures decoded, so a scan can start late or miss frames | yes | Have |
| Bytewords and CRC checked against BCR-2020-012 vectors | yes | Have |
| BBQr read: file types `P` (PSBT), `T` (transaction), `U` (text), `J` (JSON) | J only | Port |
| BBQr encodings `2` (base32), `H` (hex), `Z` (DEFLATE + base32) | yes | Port: read all three, write `2` only (§4) |
| BBQr write | J | Port |
| Specter numbered text parts (`pNofM`), written for SeedSigner, chunk size chosen so no frame reads as a PSBT | no | Port |
| Bounds: frame length, part count, total size, unresolved equations | yes | Port Faraday OS's limits (§3) |

### Payloads read

| Payload | Faraday OS | Faraday |
|---|---|---|
| PSBT (binary, base64, `ur:crypto-psbt`) | no | Have |
| `ur:psbt` (the newer registry name) | no | Port |
| Raw transaction hex | no | Port |
| `ur:bytes` | yes | Have |
| `ur:crypto-account` (SeedSigner's key export) | no | Port |
| `ur:crypto-output` (read, never written: it cannot express `<0;1>`) | no | Port, read only |
| `ur:crypto-hdkey` | no | Port |
| `ur:crypto-seed` | no | Refuse, with the reason |
| Descriptor, xpub and SLIP-132, Coldcard JSON, multisig config, BSMS, codex32, SeedQR, CompactSeedQR, words, address, silent payment address | no | Have |
| Faraday file envelope (`faraday-file-v1`: name, kind, size, SHA-256, data) | yes | Port |
| `otpauth://` URI (a TOTP secret, for an entry) | no | New |

### Sending

| Feature | Faraday OS | Faraday |
|---|---|---|
| Animated UR for a transaction by default, BBQr one choice away | no | Port |
| Descriptor: one code while it fits at version ≤ 25 and ECL M or better, otherwise animated | no | Have |
| `ur:crypto-account` byte-identical to SeedSigner's for the same key | no | Port |
| Address codes: the address exactly as printed, not a `bitcoin:` URI, not upper-cased | no | Have |
| Any file up to 256 KiB, in the Faraday envelope, as `ur:bytes` (default) or BBQr `J` | yes | Port: any Outbox file within the limit |
| Frame rate (default 2 per second) and part size settings | yes | Port |
| Public codes hidden while a secret code is on screen; public codes labelled safe to scan | no | Port |

### The scanner itself

| Feature | Faraday OS | Faraday |
|---|---|---|
| Camera choice when there is more than one | yes | Have on Linux and Windows; macOS §8 item 9 |
| Viewfinder with corner guides, and the found code outlined | preview only | Have: brackets, outline, state and parts inside (§8 item 4) |
| Progress for multi-part codes: "4 of 10, missing 3, 7", kept across interruptions | count only | Have, UR included (§8 item 7) |
| Mirrored symbols | zbar | Have |
| Inverted symbols (light on dark) | yes (tested) | Have on the stick and the Pi; desktop §8 item 2 |
| Decode at full frame size; pace the loop by measured decode cost, never by shrinking frames | n/a | Port the rule: §8 item 3 |
| "Too fine for this camera" told apart from "bad picture", with the fix (use the animated form, or a file) | no | Port |
| A code that is not what this screen wants is named, not refused generically ("that is a seed backup, not a transaction") | no | Port the wording |
| A seed scanned where a transaction is expected is never placed in the box | no | Have |
| Camera constraints treated as wishes; a refused request retried with none | n/a | Port the rule to V4L2 |
| QR from a photograph or image file, many at once, stopping when a transfer completes | PNG import | Port: PNG only, decoded in the disk process (§4) |
| Typed or pasted text as a way in | paste | Have Type; Paste on the desktop (§8 item 5), a seed's included (F75); no clipboard on the stick |
| Text files from a stick as a way in | yes | Have, from the Inbox |

## §6 Built 2026-10-05 (from docs/QR.md)

In `faraday/faraday-qr` (BBQr, numbered parts, the envelope, the UR
registry types, a minimal CBOR reader and a single-part UR writer) and
the scanner and QR sheet in `faraday-core`:

- **Read:** BBQr `P`, `T`, `U`, `J` in `2`, `H` and `Z`, within §3's
  limits; Specter `pNofM`; the Faraday file envelope over BBQr `J`,
  `ur:bytes` or one code, checked by re-packing it byte for byte, and
  filed under its own name; `ur:psbt`, `ur:crypto-hdkey`,
  `ur:crypto-account` and `ur:crypto-output` (tags 400–409) as key or
  descriptor text; raw transaction hex.
- **Refused, with the reason:** `ur:crypto-seed`, a private
  `crypto-hdkey`, a seed's words and private keys in Files, a SeedQR or
  CompactSeedQR in Files, and something else where a SeedQR was wanted.
- **Progress:** "BBQr: 3 of 10 · missing 2, 5, 6, …" for BBQr and
  numbered parts; UR keeps its count.
- **Address:** a scanned address or `bitcoin:` URI is checked against the
  first 100 receive and change addresses of each loaded wallet, and not
  filed.
- **Inverted codes:** a PNG is retried inverted, and the stick shell
  inverts every other camera frame.
- **Sending:** the QR sheet chooses UR or BBQr, 2, 3 or 5 frames a
  second, and small, medium or large parts. An Outbox PSBT goes as
  `crypto-psbt` or BBQr `P`; a wallet, key, message or share as text or
  BBQr `U`; any other file within 256 KiB in the envelope, as `ur:bytes`
  or BBQr `J`. A cosigner's `wsh` multisig key in Create shows as
  `ur:crypto-account`, SeedSigner's own CBOR.
- **Tests:** the ten BBQr parts against their own base64; Faraday OS's
  envelopes over BBQr `Z`, written by its `qr_transfer.py`; a
  `crypto-account` read back by `faraday-codec`; the app's scan path
  (`faraday-core/tests/all/scan.rs`).

Built 2026-10-06: Specter `pNofM` written (the QR sheet's **Parts**, for
a descriptor or a key); every code on the QR sheet says whether it is
public or a secret; a secret code is shown only
for a file the person let out after the secret sheet (`docs/FLOWS.md`
decision 6); progress in parts kept when the camera is closed and opened
again; an image's codes read in turn stop once a transfer in parts comes
together.

Built 2026-10-06, the rest of §1's scanner: `faraday/faraday-scanner`, the
scanner that the stick and desktop
shells use, reports what each pass saw of a code (its corners, its
module size, whether it read); the scan sheet outlines the code, green once
read and amber while not, gone ten preview frames after the scanner stops
seeing it, and three passes in a row that find a code under 2.5 pixels a
module and cannot read it say "Too fine for this camera" with the fixes.
The stick shell lists its cameras from sysfs (each video4linux node with
index 0) when the camera opens, the sheet offers the choice when there is
more than one, and the shell reopens on the one chosen. These travel on
Faraday's own storage channel (`StorageEvent::Cameras`, `QrSeen`, and
`Faraday::take_camera`), so no shared API changed. A cosigner's key
scanned while Create waits fills the waiting slot.

Built 2026-10-09, the desktop app's **Transfer** (`docs/FLOWS.md`, QR
transfer tab; `faraday-core/src/transfer/mod.rs`): Send shows a file from
`~/Downloads` (listed by the shell newest first, or dropped on the
window where winit reports drops: X11) as the QR sheet shows an Outbox
file, refused over 256 KiB; Receive is the scanner with the purpose
`Transfer`, which takes anything, routes nothing and has the shell save
each whole file into `~/Downloads` (`StorageCommand::SaveDownload`),
never over a file, saying when it holds a secret. A file read again, or
the device's animated code coming round again, is saved once. The
device's Scan now files an envelope of any kind under its name, as §2's
table says; before, a file Faraday does not read was refused there.
Tests: `faraday-storage/tests/all/transfer.rs`.

Not built: camera choice on the desktop shell (it opens the first camera);
a descriptor scanned while Create waits goes to the Inbox, not into the
flow.

## §7.4 Batch Q1, the rules for the implementing agent (from docs/QR.md)

- **Rules for the implementing agent.** `CLAUDE.md` applies in full: builds and tests on the build machine; every dimension a token; labels, not prose;
  tests for user-facing behaviour only; `just` once at the end; no
  `rm`; no commits; no agents. Nothing in `core/`.

## §8 Parity with Gauss (from docs/QR.md)

The introduction:

Compared on 2026-10-21 with Gauss's camera scanner. Items 1 and 4 to 8
are the core's (`faraday-core`, `faraday-qr`, `faraday-codec`,
`faraday-shell-api`, `faraday-draw`'s tokens); items 2, 3 and 9 are the camera
shells'. DESIGN §4.9 Scanner is the design they meet. No torch (DESIGN
§4.9).

Decided by the owner on 2026-10-21 (the list that was open here):

- **The camera's full picture, in its own 4:3 shape**, not cropped to a
  square. What the scanner reads is what the person sees. DESIGN §4's
  Scanner row says so.
- **No "Read a file" in the scanner.** Files and the Inbox already load
  files; on the stick a scanner opens only with no stick attached, so the
  tile could only close the scanner and open Files. The scanner's band
  holds Paste (desktop only) and Type.
- **Paste is desktop only**, never on the small panel: there is no Paste
  and no dimmed tile there.
- **A seed pastes into a scanner that loads one**, on the desktop
  (owner, F75): its words, SeedQR digits and shares, as typed or
  scanned. Every other scanner still refuses it, and a seed scanner has
  no Paste on a shell without a clipboard.
- **The waits stay**: 6 s for no picture, 4 s for a QR seen and not
  read.
- **Paste's seed-word check stays English only.** Widening it to other
  wordlists is dropped.
