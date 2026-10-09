# QR transfer and scanning

**Status:** v0.2 · 2026-10-05 · partly built; §6 says what is and what is not.

Faraday has one scanner and one QR sender, shared by the Wallets tab
and the QR transfer tab. The target is the union of what Faraday OS's QR
transfer does and what the Wallets tab needs, built on what OpenSigner
already has. This file lists every feature, where it comes from, and its
state.

Sources read for this file: Faraday OS's `docs/gpg-qr-workflows.md` and
`lib/faraday/qr_*.py`, `qr-transfer-gui.py` and `ur/` (the Foundation
Devices UR reference, vendored); OpenSigner's `osk-codec`
(`classify.rs`, `ur/`) and `opensigner-core/src/scan.rs`.

## 1. Where each piece stands

`Have` means OpenSigner already does it. `Port` means the Wallets tab
needs it and nothing here has it yet. `New` means neither does it as needed.

### Transports

| Feature | Faraday OS | OpenSigner | Faraday |
|---|---|---|---|
| BC-UR single part | yes | yes | Have |
| BC-UR multi-part, fountain mixtures written (rateless) | yes | yes | Have |
| BC-UR mixtures decoded, so a scan can start late or miss frames | yes | yes | Have |
| Bytewords and CRC checked against BCR-2020-012 vectors | yes | yes | Have |
| BBQr read: file types `P` (PSBT), `T` (transaction), `U` (text), `J` (JSON) | J only | no | Port |
| BBQr encodings `2` (base32), `H` (hex), `Z` (DEFLATE + base32) | yes | no | Port: read all three, write `2` only (§4) |
| BBQr write | J | no | Port |
| Specter numbered text parts (`pNofM`), written for SeedSigner, chunk size chosen so no frame reads as a PSBT | no | no | Port |
| Bounds: frame length, part count, total size, unresolved equations | yes | partly | Port Faraday OS's limits (§3) |

### Payloads read

| Payload | Faraday OS | OpenSigner | Faraday |
|---|---|---|---|
| PSBT (binary, base64, `ur:crypto-psbt`) | no | yes | Have |
| `ur:psbt` (the newer registry name) | no | no | Port |
| Raw transaction hex | no | no | Port |
| `ur:bytes` | yes | yes | Have |
| `ur:crypto-account` (SeedSigner's key export) | no | no | Port |
| `ur:crypto-output` (read, never written: it cannot express `<0;1>`) | no | no | Port, read only |
| `ur:crypto-hdkey` | no | no | Port |
| `ur:crypto-seed` | no | no | Refuse, with the reason |
| Descriptor, xpub and SLIP-132, Coldcard JSON, multisig config, BSMS, codex32, SeedQR, CompactSeedQR, words, address, silent payment address | no | yes | Have |
| Faraday file envelope (`faraday-file-v1`: name, kind, size, SHA-256, data) | yes | no | Port |
| `otpauth://` URI (a TOTP secret, for an entry) | no | no | New |

### Sending

| Feature | Faraday OS | OpenSigner | Faraday |
|---|---|---|---|
| Animated UR for a transaction by default, BBQr one choice away | no | UR only | Port |
| Descriptor: one code while it fits at version ≤ 25 and ECL M or better, otherwise animated | no | yes (its own rule) | Have |
| `ur:crypto-account` byte-identical to SeedSigner's for the same key | no | no | Port |
| Address codes: the address exactly as printed, not a `bitcoin:` URI, not upper-cased | no | yes | Have |
| Any file up to 256 KiB, in the Faraday envelope, as `ur:bytes` (default) or BBQr `J` | yes | no | Port: any Outbox file within the limit |
| Frame rate (default 2 per second) and part size settings | yes | partly | Port |
| Public codes hidden while a secret code is on screen; public codes labelled safe to scan | no | partly (secret panels) | Port |

### The scanner itself

| Feature | Faraday OS | OpenSigner | Faraday |
|---|---|---|---|
| Camera choice when there is more than one | yes | no (first `/dev/video*`) | Port |
| Viewfinder with corner guides, and the found code outlined | preview only | square viewfinder | Port the outline |
| Progress for multi-part codes: "4 of 10, missing 3, 7", kept across interruptions | count only | count only | Port |
| Mirrored symbols | zbar | rqrr (`MirroredGrid`) | Have |
| Inverted symbols (light on dark) | yes (tested) | no | Port |
| Decode at full frame size; pace the loop by measured decode cost, never by shrinking frames | n/a | shell decodes each frame | Port the rule |
| "Too fine for this camera" told apart from "bad picture", with the fix (use the animated form, or a file) | no | no | Port |
| A code that is not what this screen wants is named, not refused generically ("that is a seed backup, not a transaction") | no | routing by kind | Port the wording |
| A seed scanned where a transaction is expected is never placed in the box | no | yes | Have |
| Camera constraints treated as wishes; a refused request retried with none | n/a | V4L2 format negotiation | Port the rule to V4L2 |
| QR from a photograph or image file, many at once, stopping when a transfer completes | PNG import | no | Port: PNG only, decoded in the disk process (§4) |
| Typed or pasted text as a way in | paste | Type, Paste | Have Type; no clipboard on the stick |
| Text files from a stick as a way in | yes | Read a file | Have, from the Inbox |

## 2. Routing

Every successful scan is classified once, then goes where it belongs. The
scanner reached from a screen that wants one kind (Sign wants a PSBT) still
classifies everything, and names a wrong arrival instead of refusing it
generically. The QR transfer tab's **Receive** and Home's **Scan** want
anything. Every scanner opens only with no stick attached, as Add a key
does: a frame may picture a seed. A stick plugged in while the camera is
on is held back until it is off (`PLAN.md` §5.4).

| Arrived | Goes to |
|---|---|
| PSBT, raw transaction | The transaction step of the wallet it belongs to (`docs/WALLETS.md` §2) |
| Descriptor, policy, multisig config, BSMS, account key (`crypto-account`, `crypto-hdkey`, xpub) | A new wallet card, or the piece a wallet in progress is missing |
| SeedQR, CompactSeedQR, words | Add a key, added as its own **Scan a SeedQR** adds them; never Files or the Inbox |
| codex32, encrypted backup | The slot of the wallet it belongs to, or **Keys without a wallet**; then **Save to vault** |
| Private key, xprv | Named: Add a key takes seeds as words or a SeedQR |
| `otpauth://` URI | A new entry, filled in, in the open vault |
| Faraday file envelope | The Inbox, after its SHA-256 is checked |
| Codes read from an image file on a stick | Routed one by one by the rows above, as if scanned |
| Address, silent payment address | **Is this address mine?** |
| Any other text | Shown as text, with **Save as a note** in the open vault |

## 3. Limits

From Faraday, which fails closed on each: 256 KiB file, 360 KiB envelope or
decompressed data, 4,096 characters per frame, 1,295 source fragments,
128 unresolved fountain equations. Inconsistent headers, conflicting
duplicates, malformed encodings, bad lengths or hashes, and parts from two
different transfers all refuse. A SHA-256 match shows a file arrived
intact, not who sent it.

## 4. Decided 2026-10-04

1. **QR from images: PNG only, in the disk process only.** (JPEG was
   dropped on 2026-10-05: it needs a decoder crate.) An image
   file on a stick is never handed to the app. `faraday-disk`
   (`PLAN.md` §4.3), which is unprivileged and holds no secret, decodes the
   image and the QR codes in it and passes the app only the decoded text,
   each code arriving exactly as a camera scan would. Several images at
   once are read in turn, and reading stops when a multi-part transfer
   completes. Images are tried at about 1,200 pixels on the
   long side first and at full size second; one over
   40 megapixels is refused rather than left to appear hung. The app gains
   no image parser and no new kind of input.
2. **BBQr `Z`: read, never written.** Reading needs a DEFLATE decoder in
   the app, where scanned text is decoded: `miniz_oxide` (pure Rust), with
   the decompressed size bounded at Faraday OS's 360 KiB before any output is
   kept. Faraday writes BBQr as `2` (and reads `H`), so nothing it
   sends needs a decompressor.

## 5. Tests

- **The conformance corpus.** `qr-testing/` and
  `qr-testing-testnet/` hold the test vectors, including ten BBQr
  parts, single-code PSBTs up to version 32, and `bad-*` PSBTs that must be
  refused for stated reasons. Its `seeds/` folder holds dummy seeds and is
  kept apart.
- **Faraday OS's interoperability tests**: Faraday OS's sender to Faraday OS's
  receiver and back, for `ur:bytes` and BBQr `J`, including mixtures-only
  late starts with loss, duplicates and reordering.
- **SeedSigner and Sparrow formats**: the `crypto-account` vector checked
  against `urtypes` and SeedSigner's encoder; the `pNofM` frame rule
  against SeedSigner's `decode_qr.py` patterns.
- **Camera**: inverted, mirrored, too-fine and blurred frames from the
  test kit, decoded at full size. No claim of physical camera success is
  made from synthetic frames, as Faraday OS's own notes say.

## 6. Built 2026-10-05

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
  `crypto-account` read back by `osk-codec`; the app's scan path
  (`faraday-core/tests/scan.rs`).

Built 2026-10-06: Specter `pNofM` written (the QR sheet's **Parts**, for
a descriptor or a key); every code on the QR sheet says whether it is
public or a secret; a secret code is shown only
for a file the person let out after the secret sheet (`docs/FLOWS.md`
decision 6); progress in parts kept when the camera is closed and opened
again; an image's codes read in turn stop once a transfer in parts comes
together.

Built 2026-10-06, the rest of §1's scanner: `faraday/faraday-scanner`, a
copy of `opensigner/shells/scanner` at c418768 that the stick and desktop
shells now use, reports what each pass saw of a code (its corners, its
module size, whether it read); the scan sheet outlines the code, green once
read and amber while not, gone ten preview frames after the scanner stops
seeing it, and three passes in a row that find a code under 2.5 pixels a
module and cannot read it say "Too fine for this camera" with the fixes.
The stick shell lists its cameras from sysfs (each video4linux node with
index 0) when the camera opens, the sheet offers the choice when there is
more than one, and the shell reopens on the one chosen. These travel on
Faraday's own storage channel (`StorageEvent::Cameras`, `QrSeen`, and
`Faraday::take_camera`), so no upstream API changed. A cosigner's key
scanned while Create waits fills the waiting slot.

Not built: camera choice on the desktop shell (it opens the first camera);
a descriptor scanned while Create waits goes to the Inbox, not into the
flow.
