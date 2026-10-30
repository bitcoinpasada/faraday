# QR transfer and scanning

**Kind:** Reference
**Status:** Built

Faraday has one scanner and one QR sender, shared by the Wallets tab
and the QR transfer tab. The sender's codes are made by `faraday-qr`'s
`send` module;
each app writes the line under them in its own words. The target is the union of what Faraday OS's QR
transfer does and what the Wallets tab needs. This file lists every feature, where it comes from, and its
state.

## 1. Where each piece stands

What Faraday does now, beside what Faraday OS's QR transfer does (the
other half of the union this file set out). `Not built` marks what is
open.

### Transports

| Feature | Faraday OS | Faraday |
|---|---|---|
| BC-UR single part | yes | yes |
| BC-UR multi-part, fountain mixtures written (rateless) | yes | yes |
| BC-UR mixtures decoded, so a scan can start late or miss frames | yes | yes |
| Bytewords and CRC checked against BCR-2020-012 vectors | yes | yes |
| BBQr read: file types `P` (PSBT), `T` (transaction), `U` (text), `J` (JSON) | J only | yes |
| BBQr encodings `2` (base32), `H` (hex), `Z` (DEFLATE + base32) | yes | read all three, write `2` only (§4) |
| BBQr write | J | `P`, `U`, `J` |
| Specter numbered text parts (`pNofM`), read, and written for SeedSigner (the QR sheet's **Parts**, for a descriptor or a key), chunk size chosen so no frame reads as a PSBT | no | yes |
| Bounds: frame length, part count, total size, unresolved equations | yes | Faraday OS's limits (§3) |

### Payloads read

| Payload | Faraday OS | Faraday |
|---|---|---|
| PSBT (binary, base64, `ur:crypto-psbt`, `ur:psbt`) | no | yes |
| Raw transaction hex | no | yes |
| `ur:bytes` | yes | yes |
| `ur:crypto-account`, `ur:crypto-hdkey`, `ur:crypto-output` (tags 400–409), as key or descriptor text; `crypto-output` read, never written: it cannot express `<0;1>` | no | yes |
| `ur:crypto-seed`, a private `crypto-hdkey` | no | refused, with the reason |
| Descriptor, xpub and SLIP-132, Coldcard JSON, multisig config, BSMS, codex32 (a Codex32QR/v1-48 code among them, `docs/CODEX32.md` §10.1; Add a key's Scan a SeedQR opens the codex32 form with one), SeedQR, CompactSeedQR, words, address, silent payment address | no | yes |
| Faraday file envelope (`faraday-file-v1`: name, kind, size, SHA-256, data) over BBQr `J`, `ur:bytes` or one code, checked by re-packing it byte for byte and filed under its own name | yes | yes |
| `otpauth://` URI (a TOTP secret, for an entry) | no | yes |

### Sending

| Feature | Faraday OS | Faraday |
|---|---|---|
| A transaction as animated UR (`crypto-psbt`) by default, BBQr `P` one choice away | no | yes |
| Descriptor: one code while it fits at version ≤ 25 and ECL M or better, otherwise animated | no | yes |
| `ur:crypto-account` byte-identical to SeedSigner's for the same key (a cosigner's `wsh` multisig key in Create) | no | yes |
| Address codes: the address exactly as printed, not a `bitcoin:` URI, not upper-cased | no | yes |
| Any Outbox file up to 256 KiB, in the Faraday envelope, as `ur:bytes` (default) or BBQr `J`; a wallet, key, message or share as text or BBQr `U` | yes | yes |
| Frame rate (2, 3 or 5 per second; 2 by default) and part size (small, medium, large) | yes | yes |
| Every code on the QR sheet says whether it is public ("Public · safe to scan") or a secret; a secret code only for a file let out through the secret sheet (`docs/FLOWS.md` decision 6) | no | yes |
| Receive addresses, Explore's values, the node id, a recovery sheet's descriptor and the TOTP secret as codes | no | yes: §7 |

### The scanner itself

| Feature | Faraday OS | Faraday |
|---|---|---|
| Camera choice when there is more than one | yes | yes: stick (sysfs), Linux, Windows and macOS desktop |
| Viewfinder with corner guides, and the found code outlined | preview only | brackets, outline, state and parts inside (§8 item 4) |
| Progress for multi-part codes: "4 of 10, missing 3, 7", kept across interruptions and when the camera is closed and opened again | count only | yes, UR included (§8 item 7) |
| Mirrored symbols | zbar | yes |
| Inverted symbols (light on dark) | yes (tested) | yes: every other frame inverted on the stick, the Pi and the desktop (§8 item 2); a PNG retried inverted |
| Decode at full frame size; pace the loop by measured decode cost, never by shrinking frames | n/a | yes (§8 item 3) |
| "Too fine for this camera" told apart from "bad picture", with the fix (use the animated form, or a file) | no | yes: three passes under 2.5 pixels a module (`TOO_FINE_TENTHS`); §8 item 8 |
| A code that is not what this screen wants is named, not refused generically ("That is a transaction, not a seed") | no | yes |
| A seed scanned where a transaction is expected is never placed in the box | no | yes |
| Camera constraints treated as wishes | n/a | yes: the V4L2 shell uses the size and format the driver grants |
| QR from a photograph or image file, many at once, stopping when a transfer completes | PNG import | PNG only, decoded in the disk process (§4) |
| Typed or pasted text as a way in | paste | Type everywhere; Paste on the desktop (§8 item 5), a seed's included (F75); no clipboard on the stick or the Pi |
| Text files from a stick as a way in | yes | yes, from the Inbox |
| A scanned address or `bitcoin:` URI checked against the first 100 receive and change addresses of each loaded wallet, not filed | no | yes |
| A descriptor scanned while Create waits goes into the flow | n/a | yes: a descriptor of one key with its origin, at the wallet's kind and on its network, fills the waiting slot as a cosigner's key does; one of several keys, or one that cannot be that key, stays in Files and the toast says why (`docs/FLOWS.md`, the Keys card) |
| A code drawn by hand scanned back to check it, never loaded | n/a | yes: a SeedQR on its copy item (`CheckCopy`), a codex32 string's Codex32QR on its item (`CheckCodex32`, `docs/BACKUP-FLOW.md` §17.24); the grid to copy it from is marked block by block or row by row (owner, 2026-10-24) |

## 2. Routing

Every successful scan is classified once, then goes where it belongs. The
scanner reached from a screen that wants one kind (Sign wants a PSBT) still
classifies everything, and names a wrong arrival instead of refusing it
generically. The QR transfer tab's **Receive** and Home's **Scan** want
anything. Every scanner opens only with no stick attached, as Add a key
does: a frame may picture a seed. A stick plugged in while the camera is
on is held back until it is off (`docs/history/PLAN.md` §5.4).

| Arrived | Goes to |
|---|---|
| PSBT, raw transaction | The transaction step of the wallet it belongs to (`docs/WALLETS.md` §2) |
| Descriptor, policy, multisig config, BSMS, account key (`crypto-account`, `crypto-hdkey`, xpub) | A new wallet card, or the piece a wallet in progress is missing |
| SeedQR, CompactSeedQR, words | Add a key, added as its own **Scan a SeedQR** adds them; never Files or the Inbox |
| codex32 string, Codex32QR, SLIP-39 share | Add a key's form for it, the part in, waiting for the rest of its set (F167); never Files |
| WIF private key | On Home's Scan and the Sweep sheet's own scanner, the Sweep sheet (`docs/INTEROP.md` §5 I11, F171); on any other scanner, named: Sweep a private key, in Tools, takes it; never filed |
| Private key: xprv, Nostr `nsec` | Named: Add a key takes seeds as words or a SeedQR; never filed (F167) |
| Words of a BIP-39 list that are no English BIP-39 seed (checksum failing, aezeed, Electrum) | Named; never filed (F167) |
| `otpauth://` URI, vault entries | A new entry, filled in, or the entries, in the open vault; with none open, the scanner says to unlock one, and What do you have? says it is not used there; never filed (F167) |
| Armoured GPG public key, read by Restore a GPG key's **Scan the public key** | Files, as `{name}-{fp8}.asc` (the name Make gives it), and the restore opens on it; one that is not restored here is filed and the sheet says why (`docs/GPG-PAPERKEY.md` §3) |
| GPG paperkey: paperkey's base16 lines, or its raw form | Taken only by a GPG restore's **Scan the paperkey**, at Type the paperkey; any other scanner names it, "That is a paperkey. Restore a GPG key takes it at Type the paperkey, not Files", and files nothing (F167) |
| Faraday file envelope | The Inbox, after its SHA-256 is checked |
| Codes read from an image file on a stick | Routed one by one by the rows above, as if scanned |
| Address, silent payment address | **Is this address mine?** |
| Any other text | Shown as text, with **Save as a note** in the open vault |

A scanner opened for one kind (Add a key's **Scan a SeedQR**, a form's
**Scan**, a wallet's **Scan a key**, a vault's **Scan a setup code**)
takes what it is for and names anything else on its line by what it is,
"A transaction to sign · not used here", "A private key · not used here"
(`docs/SCANNING.md` §4.6); What do you have? names a secret it does not
use the same way. Add a key's **Scan a SeedQR** takes a SLIP-39 share to
its form as it takes a codex32 string; a form's **Scan** takes the other
share form's part onto that form and a whole seed to the seed sheet, and
keeps its parser's line for a code that starts as its own part does.

Those scanners, and a scanner for anything opened by a flow that asks
for one kind (Restore's **Scan it**, Create's **Scan its xpub**, the
seeds card's **Scan**, Spend's **Scan a copy**, the Spend tab's **Scan
it** and **Scan the PSBT**), show the read card in place of the picture
for a kind they do not take (`docs/SCANNING.md` §5): what it is, which
one, and the way over to the flow that takes it, which receives what
was read: **Add this key** (the seed sheet), **Add this share** (its
form), **Sweep a private key**, **Add to {vault}**, **Unlock {vault}**
(the code carried through the unlock, the entry's form opened filled
once the vault is open, wiped once Unlock is left) or **Make a vault**
(the code not kept), **Sign it**, **Add to the signatures**, **Decode
it**, **Load the wallet**, **Restore the wallet**, **Check the
signature**, **Save as a note**, **Keep in Files**. Tools' **Sign a
transaction**, **Decode a transaction** and **Check an address** take
every kind as Home's Scan does.

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
   file on a stick is never decoded by the app. `faraday-disk`
   (`docs/history/PLAN.md` §4.3), which is unprivileged and holds no secret, decodes the
   image and the QR codes in it and passes the app the decoded text,
   each code arriving exactly as a camera scan would, and beside it the
   picture's bytes as they are (2026-10-08). The app keeps those bytes as
   an Inbox file, to be written to another stick as the same `.png`
   (`docs/history/PLAN.md` §5.4), with the exposure of what the codes held; it never
   parses them. A picture whose codes cannot be read still comes in, as
   one with no code read. Several images at
   once are read in turn, one at a time off the shell's own thread (the
   disk process answers other requests meanwhile; `docs/SPEED.md` §9), and
   reading stops when a multi-part transfer completes. Images are tried at about 1,200 pixels on the
   long side first and at full size second; one over
   4096 × 4096 pixels (about 16.8 megapixels, `MAX_PIXELS`) is refused
   from its header, before its pixels are decoded: it covers phone and
   5K-screen screenshots and 12-megapixel photographs, and keeps one
   picture to about 135 MB of the disk process on a 1 GB Pi. The app gains
   no image parser and no new kind of input.
2. **BBQr `Z`: read, never written.** Reading needs a DEFLATE decoder in
   the app, where scanned text is decoded: `miniz_oxide` (pure Rust), with
   the decompressed size bounded at Faraday OS's 360 KiB before any output is
   kept. Faraday writes BBQr as `2` (and reads `H`), so nothing it
   sends needs a decompressor.

3. **Pictures written: labelled PNGs** (2026-10-09). A wallet's
   descriptor and each share of a split backup go to the Outbox as a
   PNG: the code, black on white in whole-pixel modules, with plain
   label lines under it (the wallet's name, shape, fingerprints,
   checksum or share number), drawn on an off-screen canvas in the
   app's fonts and written as eight-bit greyscale in stored deflate
   blocks (`faraday_codec::png::grey_png`, no compressor, a few hundred
   KB). Writing a picture is allowed in the app; reading one stays in
   the disk process (item 1). A descriptor past one code (version 25
   at ECL M) is one picture per BBQr part, which the reader assembles
   from the pictures in turn. The writer, `faraday_core::picture`,
   takes any code, title and label lines.

4. **What is offered as a code and as a picture** (2026-10-09). Every
   public file a flow makes is offered three ways where it can be: to
   the Outbox, **Show as QR** beside it, and a labelled **PNG** to the
   Outbox. A code is offered where a wallet or a person reads that
   content from one; a picture where it is one static code worth
   keeping on paper or showing to another device later. One mechanism
   serves every flow: a `QrView` carries the label lines and the file
   name of its picture, and whether its content is public; the QR sheet
   shows **PNG to the Outbox** while the view is one static code of
   public content, and a flow's own **PNG** builds the same view and
   writes its picture without opening the sheet.

   | File | Show as QR | PNG |
   |---|---|---|
   | Descriptor, shares | yes | yes (item 3) |
   | Multisig config (Coldcard, Keystone, Passport scan it) | yes | yes |
   | BSMS descriptor record | yes | yes |
   | Cosigner's key, `xpub-{fp}.txt`, and its BSMS key record | yes | yes, labelled `Key {fp} · {kind} · {path}` |
   | Silent payments record `silent-{fp}.txt`, and the address on its sheet | yes | yes, labelled `Silent payment address · {fp}` |
   | Signed message | yes | yes |
   | GPG public key, detached signature | yes | yes, as one code |
   | GPG revocation certificate | no: the secret sheet only (F32) | no |
   | PSBT part, signed PSBT, transaction hex | yes | no |
   | Wallet .json, Bitcoin Core import | from Files | no |
   | Backup, share and blank PDFs | no | no |
   | Vaults, Secure Boot keys and signed images | from Files where they fit | no |

   Why not the rest: a PSBT is handed over in animated parts, and a set
   of part pictures is clumsier than the file; a wallet .json and
   Bitcoin Core's import file are read by software from a file, and
   nothing scans them; the PDFs are the printed sheets themselves;
   vaults, keys and signed images are sealed or binary, too large for
   one code. A secret never gets either: it leaves only through the
   secret sheet (since F28, an entry's TOTP secret also as a code, §7). Files' own **Show as QR** shows any Outbox file that
   fits, as before, and offers no picture. A public file past one code
   is shown animated and its **PNG** says it is more than one code;
   only the descriptor is written as one picture per part.

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

Moved to `docs/history/QR-LOG.md`.

## 7. QR codes for public values, and the TOTP secret (F28)

The decision is `docs/DECISIONS.md` F28, its presses amended by F123
(a press, never a hold); this section is what the app does.
`docs/FLOWS.md` and the code are the record where they say more.

The owner's ask: "offer QR codes wherever it would be convenient for
users inside Faraday". Every screen named here is Faraday's own, in
`faraday-core`, drawn the same on the stick, the Pi and Faraday
Demo. Nothing in `core/` changes.

### 7.1 What is decided

**The rule for public values.** A public value gets **Show as QR**
where another device or app reads it, and **PNG** as well where it is
one static code given out or kept more than once (§4 item 4). A value
read by eye (fingerprints, hashes, GUIDs) or taken from a stick
(firmware enrolment) gets neither.

**The rule for secrets.** A secret gets no QR. It reaches a code only by
these doors: its encrypted form carries the code in its place (the
ncryptsec); a file let out through the secret sheet is shown
by Files, marked Secret (`docs/FLOWS.md` decision 6); a seed's SeedQR in
the backup's checklist; and the two exceptions the owner has made,
each because the secret's usual destination takes it by scanning and
reads no encrypted form of it: SeedQR and the TOTP secret (below). The
Starter's BitLocker recovery key (`docs/STARTER.md` D18) was a third
until `docs/DECISIONS.md` F33 (2026-10-14) removed it: no app of
Faraday's shows that key, as a code or otherwise.

**Added** (rows 1 to 6 are new codes; the rest is recorded so the
rule's reach is written down):

| # | Screen | Value | Public or secret | Before F28 | Decided | Why |
|---|---|---|---|---|---|---|
| 1 | Wallets: the wallet card's first receive address (wide and small panel); the first addresses of Create's Check, Restore's Check and the chart sheet's wallet; the Spend tab's pages (**Receive address 0** and **Check the money**); Vanity address's result | A receive address | Public | No | **Show as QR**, no PNG | A payer's phone or online wallet takes the address from the screen that derived it. A silent payments wallet's address is the one its loaded key gives at the record's account, both of its keys checked; while that key is not loaded it is the record's, marked "Not checked", and where a loaded key with its fingerprint gives other keys there is none. No PNG: a picture kept invites reuse; the silent payments address is the one meant for giving out many times, and it has one |
| 2 | Explore | The extended key at the path | Public | No | **Show as QR**, no PNG | Another wallet reads an xpub at a path to watch or compare |
| 3 | Explore | The address and the public key at the path | Public | No | **Show as QR**, no PNG | Compared with another wallet's at the same path by scanning |
| 4 | Lightning node key | Node id | Public | No | **Show as QR** and **PNG** | A peer or a Lightning wallet takes a node id by scanning, and it is given out more than once |
| 5 | Vault contents, a recovery sheet (record type 5) | The sheet's descriptor | Public | No | **Show as QR** and **PNG** | The record has no action that loads it; a code is the only way to move its descriptor short of typing it |
| 6 | Vault contents, an entry | TOTP secret, its `otpauth://` URI | Secret | No | **Show as QR**: the QR sheet marked Secret; no PNG, no file | An authenticator app takes it by scanning; its only other way in is typing a 32-character key. Faraday already reads the same URI in (§2) |
| 7 | The signing steps' **Addresses · Compare** | Receive 0/0, 0/1, change 1/0 | Public | No | No | Shown mid-transaction to compare with the online wallet, not to receive |
| 8 | Any screen | A change address | Public | No | No | Nobody pays to a change address |
| 9 | Make a new key | Its first address | Public | No | No | A key made here receives nothing until its backup is done (`docs/history/NEW-WALLET.md` §14.3); the card shows "Back up before you receive" instead |
| 10 | Vault contents, a wallet (record type 3) | Descriptor | Public | No | No | **Open in Wallets** is one press, and has **Show as QR** and **PNG** |
| 11 | Key rows, Compare, wallet cards, a key's vault page | Fingerprint | Public | No | No | Eight characters, read by eye |
| 12 | A GPG key's vault page | Fingerprint, subkey, user IDs | Public | No | No | The certificate's code carries them |
| 13 | Secure Boot keys' vault page | Owner GUID, certificate names, certificates | Public | Files, where they fit | No | Firmware enrols them from a stick |
| 14 | (retired) | Was the APK signing key's certificate (F99) | — | — | — | — |
| 15 | Nostr key, a saved Nostr entry | npub; ncryptsec | Public; sealed | Specified (`docs/NOSTR.md`, F27) | As specified | — |
| 16 | Spend, after signing | Transaction id | Public | No | No | The online wallet needs the signed transaction, which has its code |
| 17 | Vault contents, an entry | Username, URL | Kept in the vault, shown in clear | No | No | Typed where they are used |
| 18 | Vault contents, an entry; a note | Password; the note | Secret | No | No | Nothing scans a password; a note leaves through the secret sheet if at all |
| 19 | Vault contents, a Bitcoin key | Seed words | Secret | No | No | The backup's checklist is the seed's door, SeedQR included |
| 20 | BIP-85 | Child words, hex, WIF, xprv, Base64 and Base85 passwords | Secret | Not on the screen; **Out unprotected…** writes, past the secret sheet, child words as `{fingerprint}-words.txt` (the words alone, F166), a password as `password.txt` (itself alone, F172) and the rest as `bip85-{fp}-{app}-{index}.txt`, and Files shows it as a secret text code (the path line, then the value, for the rest) | No; no SeedQR of child words (owner, 2026-10-14) | A child seed is a seed and a child password is a password |
| 21 | Nostr key | nsec | Secret | Specified: never (F27) | No | Its ncryptsec carries the code |
| 22 | Lightning node key; Silent payments | Private key, aezeed; scan key | Secret | Files only, past the secret sheet | No | Imported from a file or typed by the software that uses them |
| 23 | GPG, Secure Boot key | Paperkey; private keys | Secret | No | No | Paper, or never leave the vault |

**Placement**, in each screen's own idiom:

- **Receive address rows** (row 1) open their code when pressed, with a
  chevron at the row's end, as Create's descriptor row opens the
  wallet's code (`screens::descriptor_row`). No separate button, so a
  list of addresses and a 240 × 320 panel take no extra room. A change
  address row is not pressable. While the wallet holds a key whose
  backup is not done (`Faraday::wallet_held`), no receive row is
  pressable: "Back up before you receive" is shown, as today. Vanity
  address's found address is pressable the same way; its key's backup
  is checked the same way.
- **Explore's three rows** (rows 2 and 3) are pressable the same way.
- **Lightning node key** (row 4): **Show as QR** and **PNG**, a row of
  their own under **Node id**, before **Made**; on a small panel under
  the id, as a GPG key's rows put them.
- **A recovery sheet's page** (row 5): **Show as QR** and **PNG** in the
  page's actions, under **Descriptor** and its note, as a GPG key's page
  puts its codes.
- **An entry's TOTP secret** (row 6): **Show as QR**, a button under
  the masked **TOTP secret** row, shown when the URI reads as TOTP; an
  HOTP or unreadable one shows none. No hold shows a secret (§4.10 of
  `docs/DESIGN.md`, F123): a press opens the sheet, Done or leaving
  closes it. Only the entry's own page opens it, with no sheet over it.
- A pressable row is a touch target tall at least (48 dp), its text in
  its middle; the chevron is `ROW_CHEVRON` in an `ICON_SMALL` box
  (`screens::row_press`, `screens::address_press`; Explore's
  `code_row`).

**What each code holds:**

- A receive address and Explore's address: the address exactly as the
  screen prints it, by `Session::address_shown` or `Session::shown`
  (§1, Sending): not a `bitcoin:` URI, not upper-cased. In Practice
  that is the testnet address the screen prints (F81). The QR sheet
  shows the address whole under the code (`QrView::whole`), to read
  against the code.
- Explore's public key and extended key, and the node id: the text the
  screen shows (hex for the keys, the xpub as printed).
- A recovery sheet: its descriptor as the record keeps it, as
  `Code::Descriptor` writes a wallet's, the whole text under the code.
- The TOTP secret: the stored URI as it is (a bare key is stored as one,
  `docs/TOTP.md` §4.3), so the authenticator gets the issuer and the
  account. One plain-text code.

**Pictures.** The node id's PNG is `lightning-{first 8 hex of the node
id}.png`, labelled "Lightning node id", the node's kind as the screen
names it (`ldk-node · {fp}` or `LND`) and "Public: names the node,
spends nothing". A recovery sheet's PNG is `{name}-descriptor.png` from
the sheet's name (`file_stem`), labelled with the wallet's shape, the
descriptor's checksum and "Public: watch only, spends nothing", and
past one code the BBQr parts, one picture each, as a wallet's
descriptor (§4 item 3). Both go For the stick as public files, and the
row's **PNG** says "{name} waits for the stick", as the others do.

**The TOTP secret on screen.** The QR sheet says "Secret · whoever
scans this can use it" under it, as for a secret file. It offers no
**PNG** (`QrView::offers_png` refuses a secret) and writes nothing to
the Outbox. Its text is wiped when the sheet closes (`QrView`'s
`Drop`). A lock, by hand or idle, closes the sheet first
(`lock_now` drops the view, which today it leaves to the process's
end), then ends the process. It is the one code Faraday shows of a secret that has not passed the
secret sheet: the hold is its acknowledgement.

**Code.** `Code` (`faraday/faraday-core/src/qr_view.rs`) has a variant
for each public value: `Receive(wallet, index)`, `VanityAddress`,
`ExploreAddress`, `ExplorePublicKey`, `ExploreXpub`, `NodeId` and
`VaultSheet`, each made into its view by `Faraday::code_view` when
**Show as QR** or **PNG** is pressed, never while drawing; the address
views carry no picture, so `CodePng` of one does nothing. The TOTP
secret has no `Code`, so `CodePng` cannot reach it: the vault action
`VaultAction::TotpQr` reveals the stored URI through
`Reveal::VaultEntry(Part::Totp)` and opens it as `QrView::secret`.

**Size.** A value is one code while it fits version 25 at error
correction M (about 1,000 bytes; `faraday-qr`'s `send`). An address is
26 to 62 characters, an xpub about 111, a public key or node id 66, an
`otpauth://` URI 100 to 300, a single-key descriptor about 150 to 200:
each one code. A multisig descriptor on a recovery sheet may pass
version 25; the QR sheet then shows it animated, as any descriptor, and
its PNG goes as parts.

### 7.2 Code

Moved to `docs/history/QR-LOG.md`.

### 7.3 Decided, with nothing open

- The TOTP secret as an `otpauth://` QR: yes, opened by a press (F123),
  on the QR sheet marked Secret, no PNG and no Outbox file (owner,
  2026-10-14). F27's "No Entry gains a QR of any other field" now names
  it as the exception.
- BIP-85 child words as a SeedQR: no (owner, 2026-10-14). The BIP-85
  screen shows no code of any output.
- A receive address as a PNG: no. A node id as a PNG: yes.

### 7.4 Batch Q1

Moved to `docs/history/QR-LOG.md`.

## 8. Parity with Gauss

Built (F75 for a seed's paste). Items 1 and 4 to 8 are
the core's; items 2, 3 and 9 are the camera shells'. DESIGN §4.9
Scanner is the design they meet. The scanner has no torch.
The comparison that led here: moved to `docs/history/QR-LOG.md`.

1. **A part of another UR message starts that message again.** The UR
   decoder answers a part of another message (another type, another part
   count, another checksum) with `Mismatch`. That part goes to an empty
   decoder. If the empty decoder reads it, it replaces the old one and the
   state line says "A new QR started; reading it from the start". If it
   does not (a damaged frame), the frame is dropped and the message being
   read stays as it was. Once a message is whole, the decoder is left
   empty, so the next animation read on the same sheet (What do you
   have?) starts from nothing. Any other refusal says "That QR is not a
   readable UR part". No error's Debug name reaches the screen. The
   Files scanner and Transfer's Receive share this code (`ur_feed`).
2. **White-on-black QRs on the desktop.** The window shell's `Lens`
   offers every other frame inverted, as the stick does.
3. **Resolution.** The desktop shells ask for about 1280×720, and the
   scanner reads the full frame, paced by what a decode costs. The Pi
   stays at 640×480.
4. **Viewfinder.** Corner brackets over the camera's picture: white
   (`pal::VIEWFINDER`) while looking, green (`pal::VIEWFINDER_READ`)
   while a read QR is in view (the ten preview frames the outline lasts).
   The outline round a found QR follows its corners as the scanner
   reports them, taken to the picture as drawn (the frame scaled to fill
   the picture and its longer side cropped, as the preview is; no shell
   mirrors the preview, so neither is the outline): warning
   (`Role::Warning`) when found, ok (`Role::Ok`) when read,
   `VIEWFINDER_OUTLINE_STROKE` thick with rounded corners, edged
   `VIEWFINDER_OUTLINE_EDGE` each side in the role's on-colour
   (`OnWarning`, `OnOk`), so one of the two reads over any picture
   (`found_outline`). The state line and the parts bar are drawn inside the
   picture, at its bottom, on a translucent dark band
   (`pal::VIEWFINDER_BAND` at `VIEWFINDER_BAND_ALPHA`), in white. Nothing
   is above the picture but the title. The state line shows "Starting the
   camera" before the first frame, then "Hold the QR in front of the
   camera", then the last thing said. Material's linear progress
   indicator (`Ui::progress`) shows the parts read of a UR, BBQr or
   numbered-parts transfer. Sizes: `VIEWFINDER_BRACKET_SHARE`
   of the short side for an arm, `VIEWFINDER_BRACKET_STROKE`,
   `VIEWFINDER_BRACKET_INSET`, `VIEWFINDER_BAND_PAD` and `PROGRESS_H`.
5. **Paste.** A Paste button beside the scanner's other ways in, where
   the scanner takes anything: Files, Transfer's Receive and What do you
   have?. The scanners that load a seed (a SeedQR, a key part, a
   wallet's key, and Home's Scan) have it only where the shell has a
   clipboard (F75); the copy check and the TOTP scanner have none. The button is dimmed
   where the shell has no clipboard. `Faraday::clipboard` is set by a shell
   that answers `RequestClipboard` with the clipboard's text (the desktop
   app); it is never set on the stick or the Pi. A press asks
   `RequestClipboard { kind: Text }`. Text holding a secret is refused
   and named: a seed's words (12 to 24 English words, checksum or not),
   a SeedQR's digits, an extended private key, a WIF private key, a
   codex32 string or a SLIP-39 share. A scanner that loads a seed (the
   three above, Home's Scan and What do you have?), where the shell has a clipboard,
   takes the seed's own forms instead (F75): words (in any case, any
   spacing), a SeedQR's digits, and a codex32 string or SLIP-39 share
   where the scanner takes one; they load as the same code scanned
   would, through the same intake and `faraday-keys` path; Home's Scan
   takes a codex32 string and a SLIP-39 share pasted, and a TOTP setup
   code, and sends each where the same code scanned goes (F167). A
   private key is still refused there; the Sweep sheet's scanner, where
   the shell has a clipboard, takes a WIF private key pasted, as typed or
   scanned (F171), and refuses every other secret. Other text goes where
   a QR read by
   the camera goes (`scanned`), parts included, and is wiped once
   routed. Empty text or `ClipboardUnavailable` puts "Nothing to paste"
   on the state line for `NOTE_FLASH_MS` (2 s). A shell with no
   clipboard (the stick, the Pi) has no Paste on any scanner, and no
   dimmed tile stands in for it. Paste on the desktop is live: the window shell reads the text
   through the platform's own tool (`wl-paste`, `xclip`, `pbpaste`; the
   Windows API on Windows; `docs/deps/clipboard.md`), run only on a press,
   and sets `Faraday::clipboard`. Ctrl+V (Command+V on macOS) is
   `Key::Paste` and does what the tile does; with the scanner closed it
   is Paste words where the words form has the keyboard (F75), and
   nothing anywhere else.
6. **Camera failures.** If no frame arrives `NO_PICTURE_MS` (6 s) after
   the sheet opens, the square says "The camera sends no picture". A
   frame arriving later clears it. Every Linux shell skips a frame the
   driver marks damaged or writes short (`faraday-v4l2`) rather than
   ending the capture, and the desktop shell on Linux sends
   `CameraUnavailable` when a capture ends on its own.
   `faraday-shell-api` has
   `Event::CameraFailed(CameraFailure)`, which a shell that knows why
   sends in place of `CameraUnavailable`. Its kinds and lines are
   `Refused` "The camera was refused", `InUse` "The camera is in use by
   another app" and `NotFound` "No camera found". `CameraUnavailable`
   still works and says "No camera, or the camera was refused". After a
   failure the sheet stays open with the line in the square, the ways in
   live and Cancel; it no longer closes with a toast. Faraday's scanner
   has no Read a file: files come in from a stick through Files (open,
   below).
7. **UR progress names the missing parts.** "UR: 5 of 9 · missing 2,
   7", the line BBQr's parts get (`part_note`): at most eight numbers,
   then "…". The fountain decoder knows which fragments it has resolved.
   `faraday-codec`'s `Decoder::missing` lists the rest, numbered from 1,
   which are the animation's first frames. A fragment resolved from
   mixtures counts as read.
8. **The picture is the problem.** A QR that the scanner sees (`QrSeen`)
   at `TOO_FINE_TENTHS` or more for `POOR_PICTURE_MS` (4 s) without
   reading, never out of view for more than ten preview frames, gets the
   line "Hold it square-on, in focus, with its white margin in the
   frame". A read, a pass too fine, or the QR out of view starts the wait
   again. The scanner reports all of this.
9. **macOS camera choice.** The window shell lists the AVFoundation
   cameras, and choosing one reopens the camera on it.

The owner's answers to the list once open here: moved to
`docs/history/QR-LOG.md`.
