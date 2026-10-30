# Scanning: what each scanner takes, and the way over

**Kind:** Reference
**Status:** Built; the owner's choices (2026-10-29) are in `docs/history/SCANNING-LOG.md` §10

Every scanner in Faraday reads every kind of code, and each scanner
takes only some kinds itself. This document says what each scanner
does with each kind: it takes what it is for, hands everything Faraday
takes elsewhere to the flow that takes it with what was read kept, and
names the rest (§4, §5). Home's
**Scan** is the scanner that takes every kind (`docs/QR.md` §2); every
other scanner sends a kind it does not take to where Home's Scan would
send it.

It extends `docs/QR.md` (§2 Routing, §8 the scanner itself), F75 (a seed
pasted on Faraday Demo, `docs/decisions/F75.md`), DESIGN §4.9 (Scanner),
§4.10 (Secrets) and §4.11 (Records and results), and `docs/FLOWS.md`
(Home, Add a key, the Spend tab).

## 1. The scanners

One sheet (`Sheet::Scan`, drawn by `scan_sheet` in
`faraday/faraday-core/src/screens/sheets.rs`), opened for a purpose
(`ScanPurpose` in `faraday/faraday-core/src/qr_view.rs`); one routing
function for what it reads (`Faraday::scanned` in
`faraday/faraday-core/src/lib.rs`, `read_code` beside it, and
`faraday/faraday-core/src/scan_key.rs`).

| Purpose | Opened from (button) | Title | Ways in under the picture |
|---|---|---|---|
| `Files` | Home's **Scan** (floating, every panel); Files' **Scan a QR code**; Tools' **Scan a QR code**, **Check an address**, and **Sign a transaction** and **Decode a transaction** with nothing to sign or decode in Files; Restore's **Scan it** and **Scan descriptor**; Create's **Scan its xpub** on a key slot; Restore's seeds card's **Scan** (a cosigner's box); Spend's **Scan a copy** at Finish; the Spend tab's **Scan it** and **Scan the PSBT** | "Scan a QR code", from every one of them | Paste (desktop only), Cancel |
| `Seed` | Add a key's **Scan a SeedQR**; New key's Start, **Scan a SeedQR**; a Restore slot's **Scan a SeedQR**; Tools' **Scan a SeedQR** | "Scan a SeedQR" | Paste (desktop), Cancel |
| `KeyPart` | Add a key's **Scan a QR code** on the codex32, SLIP-39 and Seed XOR forms | "Scan a QR code" | Paste (desktop), Cancel |
| `WalletKey(w)` | a wallet's **Scan a key for {wallet}** and **Scan a key**; Spend's Signers **Scan a key**; the Spend tab's signers **Scan a key**; Add a key opened for a wallet | "Scan a key for {wallet}", over a line per missing key ("Key 2 · 1A2B3C4D · Place 2") | Type the words instead, Paste (desktop), Cancel |
| `Intake` | the Spend tab's **Scan the QR codes you have**, **Scan more**, **Scan the description** | "Scan what you have", over the question's hint | Type the words instead, Paste (desktop), **Done** |
| `VaultEntry` | Codes' **Scan a setup code**; Vault contents' **Scan a code** (Entries) | "Scan a QR code" | Cancel |
| `CheckCopy(fp)` | the backup's copy item **Check my copy**; At a glance's **Scan my copy** | "Scan your copy" | Cancel |
| `CheckCodex32` | a codex32 item's **Scan my copy** | "Scan your copy" | Cancel |
| `Transfer` | Transfer's **Scan with the camera** (Faraday Demo only) | "Receive into Downloads" | Paste, Cancel |
| `GpgPublic` | Restore a GPG key's **Scan the public key** (`docs/GPG-PAPERKEY.md` §3) | "Scan the public key" | Paste (desktop), Cancel |
| `GpgPaperkey` | a GPG restore's step 4, **Scan the paperkey**, while no line is checked (`docs/GPG-PAPERKEY.md` §4.5) | "Scan the paperkey" | Cancel |

Ways in that are not the camera: Paste on Faraday Demo
(`Faraday::pasted`, routed as a scan after `pasted_secret`); a PNG on a
stick, read at a stick visit or Import (`Faraday::qr_read`: a SeedQR
copied in as `{stem}-words.txt` and loaded when the stick is removed,
anything else routed as the `Files` scanner routes it, with no sheet). A
file dropped on Faraday Demo's window goes to Transfer's Send, never to a
scanner. The device has no clipboard and no file picker.

The scanners that load a key open only with no stick in, and a stick
plugged in while the camera is on is held back until it is off
(`docs/QR.md` §2).

## 2. The kinds

What a code can be, in the order the classifier (§4.6) tells them
apart, and whether it is a secret (DESIGN §4.10, Mask policy).

| Kind | Read as | Secret |
|---|---|---|
| A seed's words | 12 to 24 English BIP-39 words, checksum holding (`faraday_intake::seed_in_code`) | yes |
| SeedQR | 48 or 96 digits | yes |
| CompactSeedQR | 16 or 32 bytes that are not UTF-8 | yes |
| Words that fail their checksum | 12 to 24 words of a BIP-39 list, checksum failing: a mistyped seed, an Electrum seed | yes |
| LND aezeed | 24 words of the English list whose BIP-39 checksum fails and whose own version byte and CRC-32C hold | yes |
| Seed XOR part | a valid BIP-39 mnemonic; nothing tells it from a whole seed | yes |
| codex32 secret | a codex32 string with share index `s` | yes |
| codex32 share | a codex32 string with another index | yes |
| SLIP-39 share | a SLIP-39 mnemonic | yes |
| `ur:crypto-seed`, a private `crypto-hdkey` | the UR registry | yes |
| Extended private key | `xprv` and its SLIP-132 and testnet forms, with or without an origin | yes |
| WIF private key | `5…`, `K…`, `L…`, `c…`, `9…` with a checksum | yes |
| Nostr secret key | `nsec1…` | yes |
| Silent payments spend key | BIP-392's `spspend1q…` (`tspspend1q…`), alone or in `sp()` (`faraday_intake::secret_text`) | yes |
| GPG paperkey | paperkey's base16 lines with a first line numbered 1, or its raw form (`faraday_keys::gpg::is_paperkey`) | yes |
| TOTP setup code | `otpauth://` URI | yes (its `secret=`) |
| Vault entries | `field: value` lines, several `otpauth://` URIs | yes |
| PSBT | binary, base64, `ur:crypto-psbt`, `ur:psbt`, BBQr `P` | no |
| Finished transaction | raw hex, BBQr `T` | no |
| Wallet description | descriptor, BIP-388 policy, Coldcard and Electrum files, multisig config, BSMS (BIP-129), `ur:crypto-output` | no |
| Split sheets of a wallet | a multisig config with keys left off | no |
| Account key | `[fp/path]xpub`, SLIP-132, `ur:crypto-account`, `ur:crypto-hdkey` | no |
| Address | an address, a `bitcoin:` URI (BIP-21), a silent payment address | no |
| Signed message | address, signature and text | no |
| GPG public key | ASCII armour | no |
| Faraday file envelope | `faraday-file-v1`, any of the above as a file | as its contents |
| Labels | BIP-329 JSON lines | no |
| Lightning invoice, web address | `ln…`, `https://…` | no |
| Text | anything else printable | may be (recovery codes) |
| Not readable | bytes that are none of these | — |

A UR, BBQr or numbered-parts transfer is read part by part ("UR: 5 of 9
· missing 2, 7") and the whole is then one of the kinds above.

## 3. What happens today

Moved to `docs/history/SCANNING-LOG.md`.

### 3.1 The matrix

Moved to `docs/history/SCANNING-LOG.md`.

### 3.2 What is wrong with it

Moved to `docs/history/SCANNING-LOG.md`.

## 4. What each scanner does with each kind

### 4.1 The rule

Every scanner reads every kind (one classifier, §4.6), and does one of
three things with what it read:

- **Takes** it: the kinds the scanner is for.
- **Hands it over**: a kind Faraday takes, but elsewhere. The scanner
  shows the read card (§5) with the way over to the flow that takes it,
  and that flow receives what was read: nothing is scanned twice.
- **Names** it: a kind Faraday takes nowhere, or one this scanner must
  never pass on. The read card names it, with **Scan again** and
  **Cancel** only.

Never, from any scanner: a secret filed in Files or the Inbox (§3.2 item
1), shown, put in a note or a vault entry other than its own kind's; a
copy check loading what it compares.

### 4.2 Where each kind goes

The destination of each kind, the same from every scanner that hands
it over. Home's Scan takes every kind straight to its destination, and
from Home the read card (§5.4) shows what came and the
destination's action.

| Kind | Destination, and the way over's label |
|---|---|
| Seed (words, SeedQR, CompactSeedQR, codex32 secret) | the seed sheet (§6); inside a flow with a slot waiting for a key (Restore's slots, Create's key slot, a wallet's missing key), that slot. **Add this key** |
| codex32 share | Add a key's codex32 form with the string in. **Add this share** |
| SLIP-39 share | Add a key's SLIP-39 form with the share in. **Add this share** |
| Seed XOR part | the seed sheet, which offers **It is a Seed XOR part** (§6) |
| Words that fail their checksum | an LND aezeed (its own checksum holds): Lightning, the aezeed typed. **Use in Lightning**. An Electrum seed: named (`docs/INTEROP.md` §5 I7). Anything else: named |
| WIF private key | the Sweep sheet (`docs/INTEROP.md` §5 I11, F171). **Sweep a private key** |
| Extended private key, `ur:crypto-seed`, private `hdkey`, Nostr secret key, silent payments spend key | named |
| GPG paperkey | a GPG restore's Scan the paperkey only, which needs its public key first; every other scanner names it ("That is a paperkey. Restore a GPG key takes it at Type the paperkey, not Files") and files nothing (F167) |
| TOTP setup code | a new entry's form in the open vault, filled, as the TOTP scanner fills it. **Add to {vault}**; with a vault in Files but none open, **Unlock {vault}**, the code carried through the unlock and the form opened filled once it is; with no vault at all, **Make a vault**, and the code is not kept: the card says "Not kept · scan it again once the vault is open", and the code is wiped at the press (§4.5) |
| Vault entries | added to the open vault. **Add to {vault}**; the same **Unlock {vault}** and **Make a vault** cases as a TOTP setup code |
| PSBT | filed, then the spend (`StartSpend`). **Sign it**; one of the spend under way, **Add to the signatures** |
| Finished transaction | filed, then Decode. **Decode it** |
| Wallet description | filed, then loaded. **Load the wallet**; in Restore, its description |
| Split sheets | filed, then Restore's description. **Restore the wallet** |
| Account key | Create's or Restore's slot waiting for one; else filed. **Keep in Files** |
| Address, BIP-21, silent payment address | **Is this address mine?**: the card's fact is the line `address_note` gives |
| Signed message | filed, then Check a message. **Check the signature** |
| GPG public key, Faraday file envelope | filed (an envelope under its own name, then as its contents). **Keep in Files** |
| Text | the card shows it masked (§4.10), with **Save as a note** in the open vault and **Keep in Files** |
| Labels, Lightning invoice, web address | named |
| Not readable | named |

### 4.3 Each scanner

| Scanner | Takes | Hands over | Names |
|---|---|---|---|
| Home's Scan | every kind, to §4.2's destination | — | as §4.2 |
| Files, Tools' Scan a QR code | as Home's | — | as §4.2 |
| Tools' Check an address, Sign, Decode | as Home's, titled for what they ask (§7.1) | — | as §4.2 |
| Restore's Scan it and Scan descriptor | a wallet description, split sheets; a seed or a codex32 secret into the first slot with no seed (after the seed sheet); an account key into its slot | every other kind Faraday takes | as §4.2 |
| Create's Scan its xpub | an account key or a descriptor of one key, into the slot; a seed into the slot, as the slot's key (after the seed sheet) | every other kind | as §4.2 |
| Restore's seeds card's Scan | an account key into the box | every other kind | as §4.2 |
| Spend's Scan a copy; the Spend tab's Scan it and Scan the PSBT | a PSBT of the spend | every other kind | as §4.2 |
| Add a key's Scan a SeedQR | a seed; a codex32 secret, loaded; a codex32 or SLIP-39 share, to its form | every other kind | as §4.2 |
| A form's Scan (codex32, SLIP-39, Seed XOR) | its own form's part; the other share form's, switching to that form with it in; a seed, to the seed sheet | every other kind | as §4.2; a string that starts as its form's does keeps the parser's line ("Not a codex32 string: …") |
| A wallet's Scan a key | a seed or a codex32 secret, matched to the wallet's missing keys; a codex32 or SLIP-39 share, to its form opened for the wallet (`EntryWallet`), back to the wallet once the key is whole | every other kind | as §4.2 |
| What do you have? | a seed, shares, a description, split sheets, an account key, a PSBT | none: the camera stays on and the intake is not left (§10 item 6) | every other kind on its line, as in §5.5; a secret is not filed |
| TOTP | a TOTP setup code, vault entries | a seed, a share, a PSBT, a wallet description, a signed message | as §4.2 |
| Copy checks | the copy, compared, never loaded | none | every other kind on the line; a SeedQR of another seed: "This is another seed's SeedQR" |
| Transfer's Receive | every kind, into Downloads (F129 for a secret) | none | — |
| Restore a GPG key's Scan the public key | a GPG public key, filed as `{name}-{fp8}.asc` and its restore opened (one not restored here filed, the sheet back saying why); a code in parts gathered first, as Home's | none | a seed, a private key, a paperkey or another secret, by the line Home's Scan names it with, nothing filed; every other kind: "Not a public key Faraday reads", nothing filed |
| A GPG restore's Scan the paperkey | a paperkey's base16 lines or raw form, into the restore's lines, all at once | none: a paperkey is a secret read only here | a paperkey with a line that does not hold: the line's refusal ("CRC on line {n} does not match"); every other kind: "Not a paperkey"; with a stick in, the line asking for it to be removed; nothing filed or kept |

A PNG read from a stick routes as Home's Scan does, without the card:
what has a destination is filed or loaded as the visit's rules say, and
a secret it may not file is named in the visit's log.

### 4.5 A secret read at a scanner that asked for something else

The owner's question (2026-10-29): what a scanner that asked for one
thing, say "Scan the PSBT" in a spend, shows when a TOTP setup code or
a SeedQR is read there. The same holds at every typed scanner (§7.1)
and at Add a key's, a form's and the TOTP scanner for a kind they do
not take.

- **A TOTP setup code.** The camera goes off and the read card shows
  "Read · A TOTP setup code" with the warning glyph, and the fact
  "{issuer} · {account}" from the code's label; never its `secret=`, and
  nothing is filed. The way over: with a vault open, **Add to {vault}**
  (the new entry's form, filled); with a vault in Files but none open,
  **Unlock {vault}**, the code carried through the unlock, the form
  filled once it opens; with no vault, **Make a vault**, with the line
  "Not kept · scan it again once the vault is open", the code wiped at
  the press and Create a vault opened empty of it.
- **A SeedQR** (or a CompactSeedQR, a seed's words, a codex32 secret).
  The card shows "Read · A SeedQR" with the warning glyph and the fact
  "Key {fp}", the key's fingerprint without a passphrase; never a word.
  The way over is **Add this key**, which opens the seed sheet (§6);
  with a stick in it is dimmed with "Remove the stick first", and the
  stick is held back while the card holds the code. Where the flow on
  screen has a slot for a key (Restore, Create, a wallet's missing key),
  the seed goes to that slot instead (§4.3).
- **A share** (codex32 or SLIP-39): "Read · A codex32 share", the fact
  "Share {index} of {id} · {k} needed", **Add this share**, to its form;
  the same stick gate.
- In every case **Scan again** wipes the code and turns the camera back
  on in the same scanner, with its title, so the person carries on with
  the PSBT; **Cancel** wipes it and closes the scanner onto the page it
  was opened from, the spend where it was. The way over leaves the
  spend in progress (F41): Home's lead tile, "Carry on the spend", takes
  the person back.

| Read at | A TOTP setup code | A SeedQR, words, CompactSeedQR, codex32 secret | A codex32 or SLIP-39 share |
|---|---|---|---|
| Home's Scan, Files, Tools' Scan a QR code | straight to the vault's form (no card) with a vault open; else the card: **Unlock {vault}** or **Make a vault** (not kept) | straight to the seed sheet | straight to its form |
| A typed `Files` scanner: Scan the PSBT, Scan a transaction, Scan an address | card: **Add to {vault}**, **Unlock {vault}** or **Make a vault** (not kept); Scan again, Cancel | card: **Add this key** to the seed sheet, behind the stick gate; Scan again, Cancel | card: **Add this share**; Scan again, Cancel |
| Restore's Scan the description, Create's Scan Key {n}'s xpub | card, as above | taken into the flow's slot, after the seed sheet | card: **Add this share** |
| Add a key's Scan a SeedQR | card, as above | taken | taken, to its form |
| A form's Scan a share | card, as above | to the seed sheet | taken; the other kind's switches form |
| A wallet's Scan a key | card, as above | taken, matched to the wallet's keys | taken, to its form for the wallet |
| What do you have? | line "A TOTP setup code · not used here"; not filed; the camera stays on | taken | taken |
| The TOTP scanner | taken | card: **Add this key** | card: **Add this share** |
| The copy checks | line "Not a SeedQR" / "Not a codex32 string" | compared, never loaded | codex32 check: compared |

### 4.6 One classifier

One classifier (`scan_read`: `secret_kind` for a secret on the one
code read, `public_kind` for anything else once a transfer in parts is
whole) reads a code once and says what it is, and every scanner, Paste
and a PNG's codes go through it, so the camera and Paste name and take
the same things. The read card's facts are worked out once, at the
read (§5.2). It costs what its parsers cost (a mnemonic's checksum, a
PSBT read, a wallet read) and says so in its first line.

## 5. The read card

### 5.1 Where it shows

In the scanner sheet, in place of the camera's picture, when a code is
read that the scanner hands over or names, and on Home's Scan after
every read whose destination is a press away (§5.4). The camera turns
off while it shows; **Scan again** turns it back on. A stick plugged in
while a card holds a secret is held back, as it is while the camera is
on, until the card is gone.

Checked against Material 3's Dialogs (a basic dialog: headline,
supporting text, actions; "purposefully interruptive, so they should be
used sparingly") and Snackbar ("shouldn't interrupt", gone after a
timeout or a press elsewhere), in Material's component docs for
Android, and against Common buttons for the emphasis of each action.
The card is the scanner sheet changed in place, not a dialog stacked
over it: the choice was made by scanning, and a second modal surface
would ask the person to answer twice. A snackbar's one action is gone
after `TOAST_SHOW_MS`, too short to read a way over, and would leave a
secret held for its life.

### 5.2 What it holds

Under the sheet's title (unchanged, so the person sees which scanner
they are in):

- **Read**: a fact (§4.11, `kv`), its value the kind in body large:
  "A transaction to sign", "A codex32 share", "A TOTP setup code".
- One or two facts that say which one it is, worked out when the code
  was read and kept with the card (drawing reads them): a seed's key
  "Key 73c5da0a" (the fingerprint without a passphrase, as the seed
  sheet shows it); a share's "Share C of NAME · 2 needed"; a TOTP code's
  "Example · account 1"; a PSBT's "2 inputs · Savings" (its
  wallet when one loaded spends it); a wallet description's shape "2 of
  3 · P2WSH"; an account key's "73c5da0a · m/48h/1h/0h/2h"; an address's
  `address_note` line; text masked behind its eye (§4.10).
- Nothing of a secret's value. A secret kind carries the warning
  glyph in `WARN` beside its kind (DESIGN §4.11, Warning).

At its foot (DESIGN §4.13): the way over, filled, at the end; **Scan
again**, a text button, at the start; **Cancel** beside it. A named
kind has no way over. A way over that cannot be taken now is dimmed
with the line under it that says what would allow it (DESIGN §4.11,
Dimmed): "Remove the stick first", the one case, while a stick is in
and the code is a key or a share.

### 5.3 What it keeps

The code read, held in `Zeroizing` memory on the card's state, handed
to the destination at the press and wiped there, as the destination
wipes what it takes. **Scan again**, **Cancel**, a press beside
the sheet, the Back key, a lock and the idle lock wipe it. Nothing on
the card is filed until the way over is pressed; **Keep in Files**
files a public kind.

A way over from inside a flow leaves the flow in progress (F41): its
lead tile and Flows in progress carry it back.

The code goes on past the card only into the page the way over opens
and only for as long as that page is open: the seed sheet, a form, a
vault's new entry form, and **Unlock {vault}**'s page, after which the
form opens filled. A lock, the idle lock, Back or Cancel there wipe it.
**Make a vault** carries nothing: making a vault is a flow of several
steps (`docs/FLOWS.md`, Vaults), and a secret held through it would
outlive the card by minutes, so the code is wiped at the press and the
card says so first (§4.5).

### 5.4 After Home's Scan

Home's Scan takes every kind (§4.3). Where its destination is a page
the person opens with one press, the card shows with that press in
place of the toast "scanned-1.psbt is in Files": a PSBT, a finished
transaction, a wallet description, split sheets, a signed message,
text, a TOTP code with no vault open. The code is in Files already
(public kinds) and **Done** replaces **Cancel**. Where Home's Scan goes
straight on (a seed to the seed sheet, a share to its form, a TOTP code
into the open vault's form, an address's line), no card.

### 5.5 The Spend tab's intake

The intake reads one code after another until **Done**, and its line
under the picture says what each was ("Wallet description read"). It
shows no card: a kind it does not take is named on its line, and a
secret is not filed ("A TOTP setup code · not used here", "A private key
· not used here").

## 6. A seed read

What the person sees when a seed is read, from any scanner that loads
one:

- The seed sheet's title names the form read: "SeedQR read",
  "CompactSeedQR read", "Words read"; a codex32 secret loads with no
  passphrase sheet (BIP 93 has none).
- Its lines: "Without a passphrase · 73c5da0a"; and where a loaded
  wallet has that key, "Key 2 of Savings", worked out when the seed was
  read (`wallet_key_found` over the loaded wallets).
- Its actions: **Use without a passphrase** (filled), **Add a
  passphrase**, **Cancel**; from a wallet's or the Spend tab's scanner,
  **Not this one**. For 12 or 24 words, a text button **It is a Seed XOR
  part**, which opens Add a key's Seed XOR form with this part in (§10
  item 4).
- After **Use without a passphrase**: Wallets, with the key
  under Keys without a wallet and **Make a wallet from this key**, **Add
  another key**, **Restore an existing wallet**; or the wallet that uses
  it.

## 7. Strings

All in `faraday/faraday-strings/src/en.rs`. Labels, values and actions;
no explanation.

### 7.1 Scanner titles

| Name | Text | For |
|---|---|---|
| `scan_title_description` | Scan the description | Restore's Scan it, Scan descriptor; the intake's Scan the description |
| `scan_title_xpub` | Scan Key {}'s xpub | Create's slot |
| `scan_title_cosigner` | Scan a cosigner's key | Restore's seeds card |
| `scan_title_psbt` | Scan the PSBT | Spend's Scan a copy, the Spend tab's Scan it, Scan the PSBT, Tools' Sign a transaction |
| `scan_title_transaction` | Scan a transaction | Tools' Decode a transaction |
| `scan_title_address` | Scan an address | Tools' Check an address |
| `scan_title_part` | Scan a share | a form's Scan |
| `scan_title_totp` | Scan a setup code | the TOTP scanner |

### 7.2 The read card

| Name | Text |
|---|---|
| `read_label` | Read |
| `read_seed_words` | A seed's words |
| `read_seedqr` | A SeedQR |
| `read_compact_seedqr` | A CompactSeedQR |
| `read_codex32_secret` | A codex32 secret |
| `read_codex32_share` | A codex32 share |
| `read_slip39_share` | A SLIP-39 share |
| `read_bad_checksum` | Words that fail their checksum |
| `read_aezeed` | An LND aezeed |
| `read_private_key` | A private key |
| `read_xprv` | An extended private key |
| `read_nostr_secret` | A Nostr secret key |
| `read_spend_key` | A silent payments spend key |
| `read_ur_seed` | A seed (ur:crypto-seed) |
| `read_totp` | A TOTP setup code |
| `read_entries` | Vault entries |
| `read_psbt` | A transaction to sign |
| `read_transaction` | A finished transaction |
| `read_wallet` | A wallet description |
| `read_split_sheets` | Split sheets of a wallet |
| `read_account_key` | An account key |
| `read_address` | An address |
| `read_payment_request` | A payment request |
| `read_silent_address` | A silent payment address |
| `read_message` | A signed message |
| `read_gpg` | A GPG public key |
| `read_file` | A file: {} |
| `read_labels` | Labels (BIP-329) |
| `read_invoice` | A Lightning invoice |
| `read_web` | A web address |
| `read_text` | Text |
| `read_unreadable` | Nothing Faraday reads |
| `read_not_here` | {} · not used here |
| `read_key_fact` | Key {} |
| `read_share_fact` | Share {} of {} · {} needed |
| `read_psbt_fact` | {} inputs · {} |
| `read_another_seed` | This is another seed's SeedQR |

### 7.3 The ways over

| Name | Text |
|---|---|
| `read_go_key` | Add this key |
| `read_go_share` | Add this share |
| `read_go_xor` | It is a Seed XOR part |
| `read_go_lightning` | Use in Lightning |
| `read_go_vault` | Add to {} |
| `read_go_unlock` | Unlock {} |
| `read_go_sign` | Sign it |
| `read_go_collect` | Add to the signatures |
| `read_go_decode` | Decode it |
| `read_go_wallet` | Load the wallet |
| `read_go_restore` | Restore the wallet |
| `read_go_check` | Check the signature |
| `read_go_note` | Save as a note |
| `read_keep` | Keep in Files |
| `read_again` | Scan again |
| `read_need_no_stick` | Remove the stick first |
| `read_go_make_vault` | Make a vault |
| `read_not_kept` | Not kept · scan it again once the vault is open |
| `read_totp_fact` | {} · {} |

### 7.4 The seed sheet

| Name | Text |
|---|---|
| `seed_read_words_read` | Words read |
| `seed_read_compact_read` | CompactSeedQR read |
| `seed_read_key_of` | Key {} of {} |


## 8. Tests

What a person scanning would notice, through `Event::Scanned` and
Paste, on the purposes of §1:

- For each scanner, each kind it hands over shows the card with its way
  over, and the press lands in the destination with the code in it, no
  second scan (a PSBT read at Add a key is signable; a SLIP-39 share
  read at Add a key's scanner is on the SLIP-39 form; a TOTP code read
  at Home is in the open vault's new entry form).
- No scanner puts a secret kind of §2 in Files or the Inbox, by camera,
  by Paste or from a PNG; the camera and Paste name and take the same
  kinds.
- **Scan again**, **Cancel**, a lock and the idle lock leave nothing of
  the code; the destination takes it only at the press.
- A seed read at Restore's or Create's scanner fills the flow's slot and
  the flow stays on screen.
- A wallet's Scan a key takes a codex32 secret that is one of its keys,
  and sends a codex32 share to its form and back.
- The copy checks never load a key, and a SeedQR of another seed says
  so.
- A TOTP setup code and a SeedQR read at Scan the PSBT show the card
  naming them and no secret; Scan again leaves the camera on that
  scanner and Cancel on the spend; with no vault, **Make a vault**
  opens Create a vault and the code is gone (the vault's form is empty
  once the vault opens); with a vault locked, **Unlock {vault}** opens
  its form filled.
- With a stick in, a seed's card dims **Add this key** with "Remove the
  stick first", and the stick is held back while the card holds it.
- A codex32 secret, a codex32 share and an aezeed read in each scanner
  against BIP 93's and LND's published vectors.

## 9. Batches

Moved to `docs/history/SCANNING-LOG.md`.

## 10. Decided (owner, 2026-10-29)

Moved to `docs/history/SCANNING-LOG.md`.
