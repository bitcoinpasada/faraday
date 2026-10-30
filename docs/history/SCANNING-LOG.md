# Scanning: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/SCANNING.md` on 2026-10-30, when the last of its open work was built. Each keeps its original number; the source keeps a one-line stub under the same heading.

## Header (from docs/SCANNING.md)

**Kind:** Spec
**Status:** Batches S1 to S5 built (2026-10-30), an LND aezeed told apart by its own checksum and handed to Lightning (§10 item 3), unrecognised text masked behind its eye on the read card (§10 item 8), a codex32 secret into Restore's or Create's slot (§4.3), and a copy check's "This is another seed's SeedQR" (§4.3), but for what the Open line names; the owner's choices decided (§10, 2026-10-29)
**Open:** the intake's Scan the description titled for it (§7.1). Not scheduled

## 3. What happens today

Read from the code and checked by feeding each kind to each scanner in
a test (2026-10-29, on main at the commit that made Home's Scan take a
pasted seed). Scanners that open the same purpose behave alike, except
where a flow on screen takes the arrival (noted).

Legend:

- **Seed sheet**: the camera closes, Add a key opens behind, and the
  seed read sheet asks how the key loads: "SeedQR read", the line
  "Without a passphrase · {fp}", **Use without a passphrase**, **Add a
  passphrase**, **Cancel** (from a wallet's or the Spend tab's scanner,
  also **Not this one**). Using it lands on Wallets: "Key {fp} is loaded.
  No wallet uses it." with **Make a wallet from this key**, **Add
  another key** and **Restore an existing wallet**, or on the wallet
  that uses it.
- **Filed**: the camera closes, the code is put in Files as
  `scanned-N.{ext}` with its kind, and the toast says "scanned-N.txt is
  in Files"; Home, Restore, the Spend tab and a stick visit stay on
  screen, any other screen moves to Files. Files then offers the kind's
  action on its row (Sign, Load the wallet, Decode, Import into a vault,
  Add to vault as a note, Check the signature, Load this key) or none
  (an account key, a seed part, a GPG key).
- **Line "…"**: the state line on the picture's band says it; the camera
  stays on and nothing changes.

### 3.1 The matrix

| Kind | Home, Files, Tools (`Files`) | Restore, Create (`Files`, in the flow) | Add a key's Scan a SeedQR (`Seed`) | A form's Scan (`KeyPart`, codex32 form) | A wallet's Scan a key (`WalletKey`) | What do you have? (`Intake`) | TOTP (`VaultEntry`) | Copy checks |
|---|---|---|---|---|---|---|---|---|
| Words, SeedQR, CompactSeedQR | Seed sheet; with a stick in, line "Remove the stick…" | Seed sheet: leaves Restore or Create for Add a key | Seed sheet; from New key, its Key card | Line "Not a codex32 string: string holds a character outside printable ASCII" | Loads at once if it is a missing key ("Key 2 added · 1 more"); "Key 2 · already here"; else the seed sheet with "{fp} is not one of {wallet}'s keys" | Seed sheet, the camera back after; with a description in, as the wallet's scanner | Line "Not a TOTP setup code" | SeedQR: compared, "matches" or "word {k} differs"; never loaded |
| Words that fail their checksum | Filed as Text | Filed as Text | Line "Not a SeedQR or a CompactSeedQR" | Line, a codex32 parse error | Line "Not a SeedQR or a CompactSeedQR" | Filed as Text, line "scanned-1.txt is in Files" | Line "Not a TOTP setup code" | Line "Not a SeedQR" |
| Seed XOR part | as a whole seed: Seed sheet | as a whole seed | as a whole seed | Seed XOR form: taken | as a whole seed | as a whole seed | Line | compared as a seed |
| codex32 secret | **Filed** as a seed part | **Filed** | Loads at once: "Key {fp} added", Wallets | Taken: loads | Line "Not a SeedQR or a CompactSeedQR" | Loads at once: "Key {fp} added" | Line | codex32 check: compared box by box |
| codex32 share | **Filed** as a seed part | **Filed** | codex32 form opens with it in | Taken | Line "Not a SeedQR…" | Add a key's codex32 form, back to the Spend tab once whole | Line | codex32 check: compared |
| SLIP-39 share | **Filed** as a seed part | **Filed** | Line "Not a SeedQR or a CompactSeedQR" | Line, a codex32 parse error (on the SLIP-39 form: taken) | Line "Not a SeedQR…" | Add a key's SLIP-39 form | Line | Line "Not a SeedQR" |
| `ur:crypto-seed`, private `hdkey` | Line "That is a seed (ur:crypto-seed). Faraday takes a seed as its words or a SeedQR, in Add a key" | same | Line "That is a code for Files, not a seed" | Line, parse error | same as Add a key | Line, as Home | Line | Line |
| Extended private key | Line "That is a private key. Faraday takes keys as words or a SeedQR, in Add a key" | same | Line "Not a SeedQR or a CompactSeedQR" | Line, parse error | Line "Not a SeedQR…" | Line, as Home | Line | Line |
| WIF private key | **Filed** as Text | **Filed** | Line "Not a SeedQR…" | Line, parse error | Line | **Filed**, line "scanned-1.txt is in Files" | Line | Line |
| Nostr secret key | **Filed** as Text | **Filed** | Line "Not a SeedQR…" | Line | Line | **Filed** | Line | Line |
| TOTP setup code | **Filed** as Entries ("Import into a vault") | **Filed** | Line "Not a SeedQR…" | Line | Line | **Filed** | New entry's form filled (title, account, TOTP), back to Codes or Vault contents | Line |
| Vault entries | Filed as Entries | Filed | Line | Line | Line | Filed | Added to the open vault: "Added 3 entries" | Line |
| PSBT | Filed ("Sign"); one of the spend under way is collected into it | Filed | Binary: line "Not a SeedQR…"; base64 or UR: "That is a transaction, not a seed. Scan it from Files" | Line | as Add a key | "A transaction to sign · kept for later" | Line | Line |
| Finished transaction | Filed ("Decode") | Filed | Line | Line | Line | Filed | Line | Line |
| Wallet description | Filed ("Load the wallet") | Restore: taken as its description; Create: a descriptor of one key fills the slot waiting | Line "That is a public wallet or key, not a seed: it cannot sign" (another network's: "Not a SeedQR…") | Line | as Add a key | "Wallet description read", the wallet opens | Line | Line |
| Split sheets | Filed | Restore: its description, rebuilt once whole | Line | Line | Line | "Part {k} read", rebuilt once whole | Line | Line |
| Account key | Filed, no action on its row | Create: the slot waiting; Restore: "scanned-1.txt is a cosigner's key" | Line "That is a public wallet or key…" | Line | as Add a key | "A public key: it shows the wallet but cannot sign" | Line | Line |
| Address, BIP-21, silent payment address | Line: "That address is Savings's receive address 4", "…not among the first 100 of any loaded wallet", "That is an address. Load a wallet to check whether it is yours", "That address is not on {network}" | same | Line "Not a SeedQR…" | Line | Line | Line, as Home | Line | Line |
| Signed message | Filed ("Check the signature") | Filed | Line | Line | Line | Filed | Line | Line |
| GPG public key | Filed (Restore from its paperkey in its ⋮) | Filed | Line | Line | Line | Filed | Line | Line |
| Labels, Lightning invoice, web address, text | Filed as Text ("Add to vault", as a note) | Filed | Line "Not a SeedQR…" | Line | Line | Filed | Line | Line |
| Not readable | Line "Not a PSBT, a descriptor, an xpub or a signed message" | same | Line "Not a SeedQR…" | Line | Line | Line | Line | Line |

Transfer's Receive takes every kind and saves it into Downloads; a
secret only past **Save without encryption?**'s hold (F129). It is the
far end of a link from the device and keeps that rule (§4.3).

### 3.2 What is wrong with it

1. **Secrets filed by the camera.** Home's, Files', Restore's, Create's
   and the Spend tab's scanners put into Files a codex32 secret or
   share, a SLIP-39 share, a WIF private key, a Nostr secret key, words
   that fail their checksum and a TOTP setup code. `docs/QR.md` §2 says
   a seed never goes into Files, a codex32 string goes to its slot, and
   a TOTP code to a new entry in the open vault; FLOWS says a private
   key is named, not taken. The same text pasted on Faraday Demo is
   refused (`pasted_secret`), so the camera and Paste disagree.
2. **No way over.** A code read in the wrong scanner gets at best a line
   naming it ("That is a transaction, not a seed. Scan it from Files");
   the person must cancel, find the other scanner and scan again. From
   the codex32 form, the TOTP scanner and the copy checks it gets less.
3. **Lines that name nothing.** Add a key's scanner says "Not a SeedQR
   or a CompactSeedQR" to a binary PSBT, a private key, a SLIP-39 share
   (though it takes a codex32 string), an address and a TOTP code; the
   forms' scanner answers a descriptor with "Not a codex32 string:
   string mixes upper and lower case".
4. **Home's Scan hides its result.** A PSBT, a wallet description or a
   signed message read from Home leaves a toast and Home as it was; the
   next step is in Files, two presses away, and an account key has no
   action there at all.
5. **Flows left behind.** A seed read at Restore's or Create's scanner
   takes the person out of the flow to Add a key, though Restore and
   Create each have a slot for it.
6. **The seed sheet's title** says "SeedQR read" for words and a
   CompactSeedQR alike, and a Seed XOR part loads as a whole seed with
   no word that it might be a part.
7. **A wallet's Scan a key** takes no codex32 string or SLIP-39 share,
   though Add a key and the Spend tab do.
8. **One title for fourteen buttons.** Every `Files` scanner says "Scan
   a QR code", whether Restore asked for a description or Spend for a
   PSBT; the band's one line is all that differs.
9. **`docs/QR.md` §2 is not what is built** for TOTP codes, codex32
   strings and "any other text" (shown, with **Save as a note**).

## 9. Batches

- **S1, the classifier and nothing secret filed** (§4.6, §3.2 items 1,
  3): one classifier; every secret kind named or taken, never filed;
  the forms' scanner names a kind that is not its form's. Tests as §8's
  second item.
- **S2, the read card and the ways over** (§4.2, §4.3, §5): the card,
  its kept code and its wiping; the ways over from every scanner but
  Home's.
- **S3, after Home's Scan** (§5.4), with the session building Home's
  Scan as the universal entry; `docs/QR.md` §2's table made true.
- **S4, flows keep their arrivals** (§3.2 items 5, 7): Restore's and
  Create's scanners take a seed into their slot; a wallet's Scan a key
  takes codex32 and SLIP-39.
- **S5, titles and the seed sheet** (§6, §7.1): the scanner titles by
  opener (`ScanPurpose::Files` carries what was asked for), the seed
  sheet's titles, its wallet line and **It is a Seed XOR part**.

Each batch updates `docs/QR.md` §2, DESIGN §4.9 and §5 (the sheets),
`docs/FLOWS.md` (Home, Add a key) and the guide's scanner pages, and is
timed in a late-flow session where it adds work to drawing
(`docs/SPEED.md` §7): the card's facts are worked out at the read, not
while drawing.

## 10. Decided (owner, 2026-10-29)

Each as recommended, but item 1, which the owner changed for a TOTP
code read with no vault at all.

1. **A TOTP setup code** goes into the open vault's new entry form,
   never Files. With a vault in Files but none open, **Unlock {vault}**
   carries it through the unlock (one page, wiped by a lock, Back or
   Cancel there, §5.3). With no vault at all, the way over is **Make a
   vault**, and the code is not carried: the card says "Not kept · scan
   it again once the vault is open", the code is wiped at the press, and
   the person scans it again once the vault is made and open (§4.5).
2. **Words that fail their checksum** are named, never filed.
3. **An LND aezeed** is told apart by its own checksum and handed to
   Lightning, tested against LND's vectors.
4. **A Seed XOR part**: the seed sheet offers **It is a Seed XOR part**
   for 12 and 24 words, a text button; loading it as a seed stays the
   default.
5. **Home's Scan of a public kind** shows the read card with the kind's
   action, and **Done** in place of Cancel (§5.4).
6. **The Spend tab's intake** names a kind it does not use and stays on
   the Spend tab (§5.5).
7. **A seed read at Restore's or Create's scanner** goes into the flow's
   slot.
8. **Unrecognised text** on the card is masked behind its eye.
9. **A seed read at a scanner that loads none** is handed to the seed
   sheet, behind the same stick gate as Add a key.
10. **The camera** is off while the card shows, and back on at **Scan
    again**.
11. **Scanner titles** name what each opener asked for (§7.1).

## 7.4 The seed sheet: its last paragraph (from docs/SCANNING.md)

Retired once the card replaces them: `what_instead_of_a_seed_*`,
`scanned_not_a_psbt`, `scanned_is_in_files` (from Home's Scan),
`read_code_is_seedqr`, `read_code_that_may`.
