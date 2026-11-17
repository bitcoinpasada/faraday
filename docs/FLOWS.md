# Flows

**Kind:** Reference
**Status:** Built; the code is the record where it differs

Every flow, where it starts, and where it leaves the person.

The rule this file checks: **no flow ends on a dead end.** Each one
finishes on a screen that shows what happened and offers the next thing a
person would do. Where a flow cannot go on (no vault open, no file in
Files, a stick still attached), it says what is missing and offers the way
to get it.

`docs/DECISIONS.md` holds the decisions; `docs/VAULT.md` the vault format;
`docs/QR.md` the scanner.

## Decisions this pass made

Each decision still in force is
stated below where it applies, tagged with its number ("decision 61").

## Words

**Keys, seeds, xpubs** (decisions 7 and 53). What is loaded and signs is
a **key**, named by its fingerprint; a **seed** is the words that make it
(words, a SeedQR, a share of one). An account's public key with its
origin, `[fingerprint/path]xpub`, is an **xpub**, and a wallet's
description is a **descriptor** (or its split sheets). No file name,
label or sentence calls an xpub or a descriptor a key; the test stick's
files are named `seed-…` and `xpub-…`. The lists of what is loaded say
Keys: the sidebar's **Keys**, Files' **Keys with no wallet**, the Spend
tab's **Keys with no wallet loaded**, Restore's **Keys** card, counts
such as "2 of 3 keys here" and **Load 2 keys**. The words stay seeds:
Scan a SeedQR, I have the seed, "Key 1 seed", the seed sheet. A key
loaded with a passphrase is marked **passphrase** at the end of its
sidebar row. A key's xpub is its own: **Its xpub**, "waiting for its
xpub file", **Scan its xpub** (decision 58).

**From the stick, For the stick** (decision 1). Files' two halves are
named by direction, **From the stick** and **For the stick** ("From the
card or stick", "For the card or stick" on the Pi), on Files, the stick pane, the
sheets and every button; no screen says "Inbox" or "Outbox", which
remain the code's names only.

## Rules every flow keeps

**What loads a key asks for the stick to be pulled** (decision 8). Keys
load only with no stick attached, but nothing that loads one is shown
disabled for it: Add a key, Make a new key, Load this key, a Tools tile
that adds or makes a key, opening a seed from Files or a backup. Pressed
with a stick in, each opens a sheet, "Pull the stick to add a key" (or
to make a key, …), the stick's label, and **Cancel**. The sheet closes
the moment the last stick goes, and what was pressed carries on; Cancel
closes it with nothing done.

**Every scan waits for the stick to be out** (decision 9). A camera
frame may picture a seed, so the scanner opens only with no stick
attached: Scan, Scan a SeedQR, Scan a key, Scan your copy and a vault
entry's Scan, pressed with a stick in, open the same sheet, "Pull the
stick to scan", and the camera turns on once the stick is out. A stick
plugged in while the camera is on is held back, nothing read from it,
and arrives as any stick does once the camera is off. Signing by QR
alone needs no stick.

**Whatever needs a vault leads into making or unlocking one, and back**
(decision 10). Where a flow needs an open vault and none is, its button
is **Make a vault** when no vault file is in Files, **Unlock {name}**
when one is, and **Unlock a vault** (the Vaults list) when several are;
never a disabled button or a line to go to Vaults. Create a vault,
opened this way, carries a chip at its head, "Then: {flow}", and its way
back returns to the flow. Making the vault goes straight to Unlock with
it picked: the passphrase is typed once more, which shows the person has
it. Unlocking needs the stick out: with one in, the Pull sheet asks first
("Pull the stick to unlock"). Once unlocked, the flow is back on the step
it was on, with the vault's button live. The places: Tools' **GPG key**,
**Secure Boot keys** and **KeePass export**
("Make a vault first" / "Unlock a vault first" on the tile, pressable;
back to that category of the vault unlocked, or to Tools when the way
back is taken); BIP-85, the Lightning node key and the Silent payments
scan key (**Save into {vault}**); the secret sheet (a seed from the
backup's files item), which closes on the way, keeps its
secret, and opens again on return; the backup's vault items; Files (an
entries file, a file added to a vault); Vault contents with
none open; and Restore's seeds route, which offers Unlock only, as a new
vault holds no seed. Going back anywhere on the way, or leaving for
another page, ends it: a secret held for it is dropped, wiped, as Cancel
on the sheet does.

The one exception (F30, `docs/DEMO.md` §7.1): while `Session::demo()` is
true, an action listed in DEMO §4 is drawn dimmed, with no reason, and
does nothing. It is the only action in Faraday dimmed for a condition the
person cannot change on the device. A Tools tile that leads to one of
these screens is not dimmed: "Make a vault first" and "Unlock a vault
first" still lead into the vault and back, and the screen reached shows
its Make action dimmed.

**A SeedQR asks for its passphrase after it is read** (decision 52).
Wherever a SeedQR (or a seed's words in a code) is scanned (Add a key,
Home's Scan, a slot of Create or Restore, the Spend tab's words) the
camera closes and the sheet, titled for what was read (**SeedQR read**,
**CompactSeedQR read**, **Words read**), shows the key it makes
**Without a passphrase**, "Key 2 of Savings" where a loaded wallet has
it, with **Use without a passphrase**, **Add a passphrase** and Cancel,
and for 12, 18 or 24 words **It is a Seed XOR part**, which opens Add a
key's Seed XOR form with them in (`docs/SCANNING.md` §6). A codex32
secret, which holds no passphrase, read at a slot of Create or Restore
loads on Add a key's codex32 form with no sheet and goes into the slot,
the flow on screen after. Add a passphrase shows **Passphrase** and
**Passphrase again**, the key they make once they match (**With this
passphrase**), and **Use with this passphrase**; two that differ load
nothing. With Restore's description known, a key that is not one of its
keys is said on the sheet, which stays for the passphrase to be typed
again. The words wait in memory, never shown, and Cancel or a press
beside the sheet wipes them. A key already loaded, or a vault's seed,
taken into a slot of Restore or the Spend tab with words and no
passphrase opens the same sheet: a vault may keep a seed apart from its
passphrase.

**Step cards** (decisions 16 and 34). In Sign a transaction, Back up and
Create a vault a step card with a default opens closed and shows its
value and **Change** in place of the chevron; the flow opens on the first
card without one. Make a new wallet opens on Kind, open, with nothing
ticked (owner, 2026-11-09); a Tools tile that ticks a
kind opens the card after it. New key opens on its first step, whatever
opened it.
A card that asks one thing (a kind, Yes
or No, a format, a size) closes on the choice and opens the next, and a
press on the option already chosen does too (F103); **Continue** stays
under a card of several values (steppers, ticks, typing) and where the
press is the act.

**Back within a flow** (F104). In Restore, Sign a transaction, the Spend
tab, Sign a message, Create a vault, BIP-85, Convert a seed, Silent payments, Vanity,
Make a new wallet, Back up a wallet, the backup flow and Check a signer,
Back opens the step before the open one with its choices as they were,
and the way back is named after it; Back from the first step leaves the
flow as F77 says. A running step (a vault being made, a search under
way, a stick being written) takes no Back. Once a wallet is made from its seeds or
from Make a new wallet's keys, or a key here
has signed, or a MuSig2 round is open here, Back goes back no further
than the step after it. Escape is Back on BIP-85, Convert a seed and Vanity.

**Krux code** (planned; `docs/KEF.md` §15.2): a Tools tile, not a calculator, that
seals a typed text or a loaded wallet's descriptor as a Krux code.

**Convert a seed** (`docs/SEED-CONVERSION.md` §6). Tools' tile opens From, the
loaded keys, the open vaults' keys not loaded and **Add a key**, which returns
here; a vault's key, pressed, loads and stays loaded, as under Make a new
wallet's Keys; with one key loaded, To opens on it. A key shown on its chart
sheet or on Explore has **Write it in another form**, which opens the flow on
that key, To open; Done returns to the screen it came from. To lists the
master seed in hex and the xprv, and for a key of words the words, SeedQR,
CompactSeedQR, hex entropy, word numbers (`1 (0)` a line), 4-letter starts and
Seed XOR parts (English words only); for a key of seed bytes the SLIP-39 1 of 1 and k of n (16 or 32 bytes; 64
bytes dimmed) and codex32. Seed XOR, SLIP-39 k of n and codex32 open a Settings card between To
and Result (Parts; k and n, or SLIP-39's groups, groups needed and each group's shares needed and made, a plus that would need more randomness than one press can be given disabled and doing nothing; Make; with no randomness ready a toast says so), a codex32 set of 2 or more taking a new identifier on each Make,
and their Result is a set: one eye, the parts numbered, Check my copy against
every part (a SLIP-39 set of groups says which group), Save all shares, one file each, with one warning for the set and every part its own file named for its number and the count (`-1-of-3`), never into a vault; Practice dims Seed XOR
and SLIP-39 k of n and goes from codex32's secret straight to Result;
SeedQR and CompactSeedQR for 12 or 24 words only; a key on another list is
offered the words, entropy and numbers, with a **Word list** fact on the last
two, and a key with no words none of them. A press under To works out
the value, on the tick after a frame (a code is scanned back before it shows), and Result opens with it masked, its
Facts, **Check my copy**, **Save as a file…** and **Done**; a code's Result
has the code in place of the text, no Check my copy, and Save writes its
picture, labelled as the backup's SeedQR is. Check my copy takes
the copy typed back, as dots, and on **Compare** or Enter says whether it
matches (word numbers on the 1 to 2048 numbers alone, or every pair as shown, its two numbers agreeing); a difference wipes what was typed. **Save as a file…** opens the Secret
out sheet (`docs/DESIGN.md` §4.10) with the value alone, and nothing is written
until the sheet is answered. Both work in Practice, whose keys are public seeds.
A dimmed row under To takes no press. Leaving Result, or the flow, drops the
value and what was typed.

From also has **A SLIP-39 set** (`docs/SEED-CONVERSION.md` §8): an entry for one
share at a time, held with Enter or **Use this share** (a refusal names its
reason and adds nothing), under it the set's identifier, the groups needed and
per group the shares needed and the numbers typed; **Start again** forgets
them, and **Continue** opens To. Practice and an attached stick refuse it.
To then offers **Add a share**, **Replace a lost share** and **Rotate the set**,
each dimmed with the shares or groups it still needs until the set allows it.
Add and Replace open a Settings card of the group (where several have a quorum)
and the share number, 1 to 16; Rotate's has per group a count of shares lost or
exposed, then a press for each typed share destroyed and one for all the
others held, then the shape of the new set. At the count's limit the card says
the set may be in other hands and offers nothing to make. The Result is the
share or set, with no Fingerprint. The held set and the shares made drop on
leaving the flow, on choosing a key as the source and on lock.

**Mouse and typing** (decision 47). On the desktop the mouse's back
button does what Back does (F77, F104: within a flow the step before,
else the screen seen before, as it was seen; Escape where there is no
back link, a sheet's Cancel over a sheet), and its forward button opens
again the step or screen back left, as it was left, until anything else
is opened. In a line of typing (typed text, a masked
passphrase, New key's box of rolls or flips) a press puts the caret
where it lands, a drag selects what it goes over, Shift with a press or
Left and Right extends the selection, and a second press selects all;
Backspace and Delete take the selection, a character replaces it. In
dots a selection is by position; nothing is copied out of a masked
field. Rolls or flips deleted in the box leave the key's entropy with
them. An empty BIP-39 passphrase field (New key's Key card, Create's Add
a passphrase, Add a key) shows a **No passphrase** pill at its start,
with the caret after it while focused; a vault's passphrase field has
none.

**The time first** (decision 58). GPG and Secure Boot ask
for the time first where TOTP codes does (`docs/TOTP.md` §1.3).

## Sidebar

**Check** stands right under Home, on the rails and first on the small
panel's Home menu (F255; `docs/DESIGN.md` §5, Check). **Learn** follows Tools (decision 13); the small panel's Home grid
carries it. The sidebar's status block carries the session, Files and
the sticks, which open Files on the stick; its **Files** entry opens Files
and says in plain words what the files are: **n files from the stick**
(brought in), **n files to write to the stick** (waiting to be written)
or **0 files**. There is no strip of stages.
The mode shows only as an off-mainnet pill that opens the
mode sheet (decisions 14 and 33; F81): no coloured bar along the top on
a test network; the sheet is also Settings' Mode row, the only door on
mainnet.

## Files

**One Files screen holds what came from the stick, what goes to it and
the stick itself** (decision 1, F235). Every file copied in carries the action that uses it: a
PSBT **Sign in Wallets** (a signed PSBT no key here is left to sign,
and a finished transaction, **Nonce check**: see Nonce check below), an
entries file **Import into a vault**, a vault file **Unlock**, a received
file **Show as QR**. Both halves are emptied at power-off. Each half
scrolls on its own, with its own scrollbar, under the pointer (decision
55). A `BOOTX64.EFI` carries no action there: it is signed from its
key's page in the vault (**Secure Boot** → **Owner keys** → **Sign or
check an image**).

**For the stick fills itself** (decision 24). A public file a flow makes
goes For the stick the moment it is made: the signed PSBT and the
finished transaction at Finish (once Review is gone through for that
PSBT, also when it was signed elsewhere), the PSBT with this pass's signatures
while more are needed, the backup checklist's template, sheet or shares
and public files when the checklist is made (made again when the word
count or the shares change), a GPG public key or signature (a revocation
certificate is a secret and goes through the secret sheet, F32), a
signed message, a sealed vault at lock. No button puts a file out: the
row reads "For the stick" with **Remove** in its ⋮, and after a Remove
the flow's button makes it again. The secret sheet stays the one way a
secret goes For the stick.

**A write leaves a receipt** (decision 25). When a write to the stick is
answered, what it sent leaves For the stick and becomes the receipt,
shown under For the stick as "Written to {label} at {time}" with a row
per file, "verified" or why not. It is kept across a lock until
power-off and replaced by the next write; nothing on it is written
again. A vault written stays in Files as the stick now holds it. With
nothing loaded, Home leads with "Written to {label}" and the file names,
opening Files; a backup checklist item counts a file on the receipt as
done.

### A ⋮ on every file; Rename (decision 61, F54)

Every file row, in Files, with the stick pane, keeps the one action it
is for as a visible button and puts the rest behind a ⋮ at its right
end, as the backup board's chips do (`docs/BACKUP-FLOW.md` §17.20: a
popup under the ⋮, the sheet on the small panel, keyboard reachable, a
press outside closes it). One module, `menu.rs`, draws and keys every ⋮
menu.

1. **Files, both columns.** The visible button is the row's main
   action: Sign, Unlock or Contents, Load the wallet, Load this key,
   Import into the vault, Decode, Check the signature, Add to
   signatures, the shares' restore, Make PDF for a
   sheet From the stick, Add to vault where it is the only action. The
   ⋮ holds, in this order and only where the row has them: **Add to
   vault** (where it is not the visible one), **Show as QR**, **Make
   PDF** (For the stick), **Check a signer** (a PSBT's row and a
   finished transaction's row, after **Show as QR** where it has one;
   `docs/SIGNER-CHECK.md` §11.2), **Check this setup** (a wallet
   file of more than one key, after **Show as QR** where it has one;
   `Screen::MultisigCheck`, Check a multisig setup), **Rename** (For the stick only),
   **Details**, **Remove**. There is no visible **Remove** button. A row with no main
   action shows only the ⋮. A vault action (Add to vault, Import, Sign
   with key) is not shown, as a button or in the ⋮, while no vault is
   open; the vault's own row keeps its Unlock (F60).
2. **Rename, For the stick.** A popup, "Rename {name}": a field holding
   the name before its extension, the extension shown fixed after it
   (`.kdbx` for a vault; every file keeps its own), and **Rename** and
   **Cancel**. Refused, with a line in the popup and Rename disabled: an
   empty name, one starting with a dot, `/`, `\`, `:`, `*`, `?`, `"`,
   `<`, `>`, `|` or a control character, a name over 255 characters with
   its extension, and a name another file For the stick has, ignoring
   case. Spaces at the ends are dropped; the name unchanged leaves Rename
   disabled with no line. The new name is the one the next visit writes
   it under; the stick's **Already on the stick** sheet then works as
   usual. A vault For the stick whose same vault (by identity,
   `docs/VAULT.md` §6) is on the stick under another name is written over
   that file and the file takes the new name (or `name-2.kdbx` where
   another file has it); otherwise a vault written over its same vault
   keeps the name the stick has for it. Renaming a vault's file also
   renames the file the open vault seals back into. Renaming a vault's
   file never changes the vault's own name (the open vault's
   **Rename**), nor the reverse.
3. **From the stick has no Rename**: its files are Faraday's copies of
   files on a stick, renamed on the stick itself (item 4).
4. **On the stick (the stick pane).** Every file row Faraday lists has a
   ⋮ with **Rename**, **Delete from the stick** and **Details**; the
   computer's own files (decision 54) and the settings file have none.
   **Rename** is the same popup, renaming the file on the stick in place:
   the disk process renames its directory entry, lists the stick again,
   and Files shows "{old} renamed {new}", or why not. Refused, nothing
   written, where the name is on the stick already, ignoring case:
   "{name} is already on this {stick}"; where that file is the same vault
   by identity, "{name} on this {stick} is this same vault" (one copy is
   deleted first). The popup checks only the name's form; whether the
   name is taken is the disk process's answer. **Delete from the stick**
   opens the Delete files sheet (decision 54) with this file alone
   chosen; a vault file there needs the hold of decision 4. The **Delete
   files** button stays, for many files, the free space and the whole
   partition.
5. **Details**, a sheet: the name, what it is (the row's kind line), its
   size in bytes, and its SHA-256 in groups of eight, two lines of four;
   nothing to press but Close. For a file on the stick the disk process
   hashes it and only the hash reaches the app ("Reading…" until then,
   "Not read: …" over 18 MiB).
6. **Names elsewhere.** Backups and the backup flow name a copy by the
   file it was written as; a copy renamed on the stick is named anew from
   the next read of that stick, which the rename itself is.

## Public and secret

**Public and secret leave by different doors** (decision 6). Every file
on its way out is public (anyone may read it: descriptors, wallet files,
account keys, PSBTs, certificates), sealed (a vault or an encrypted
backup) or secret (anyone holding it can use what is in it). Public
files go For the stick as they are. A secret goes into the open vault by
default and leaves only inside the vault file; the one way For the stick
unprotected is the secret sheet, which names the file, what it is and
who can use it; its **Put it for the stick unprotected** opens **Save
without encryption?**, and only its **Hold to save without encryption**,
held to the end, lets the file out (F129). For the
stick lists the three apart, Public, Sealed and Unprotected secrets, a
stick visit tags each row, and an unprotected secret is not ticked for
writing until the person ticks it in Files. A secret let out this
way is marked as one whatever its text reads as (a node key, a child
seed's words), and the mark is kept across a lock. A seed's words file
is a secret by its kind. The code refuses a secret on the plain path For
the stick, so a new flow cannot skip the sheet.

A wallet's seed goes into a vault as a key record (`kind::KEY`, as
Vaults saves it), never a note; as a file it is the words or the SeedQR
picture, one picked on the sheet, behind its own acknowledgement
("anyone who copies the stick or sees this file can spend these coins"),
and never holds the BIP-39 passphrase. A seed's SeedQR never goes
through the QR sheet's **PNG for the stick**, which is for public
content. Seeds are copied by hand, each copy checked by its form (F29): a
words copy by typing the whole seed back as words with the seed not on
screen, a SeedQR copy by scanning the drawn grid (by typed words where
there is no camera); **Keep the words only** turns a SeedQR copy into
words by hand.

## New key and Add a key

Codex32's flows are `docs/CODEX32.md`: making shares (§6) is built,
below. **Type codex32 strings** (the codex32 form, on Start's Words card
and on the Add a key screen) takes one string at a time, typed on the
codex32 keyboard or scanned; each is checked at once, and one whose
checksum fails is named by its index ("Share C · checksum fails",
§8.1). The same string again is "Already in"; another at an index
already in shows both, their differing boxes marked, with **Keep the
one held** · **Replace it** (§8.5). A string is read as given: mixed
case is refused. **Check a string** on the form (§8.3) checks one
string typed or scanned: "Checksum holds" or "Checksum fails", what it
is, and whether it is of the set typed in; nothing is loaded or kept.
The form carries the error correction (§8.4,
`faraday/faraday-core/src/codex32_correct/`): the tick **Suggest
corrections**, off whenever the form opens, which puts `?` on the
keyboard; a failing string's one proposal, box by box, four to a page,
in place of the field; **I mistyped**, **The paper is damaged** and
**Unreadable on the paper**; the changed boxes typed again from the
paper, taken only on an exact match; "Repaired, not checked" under a
repaired string; and the key's mark ("Share entry corrected", "Share
repairs unchecked") kept by fingerprint for the session. **Scan a
SeedQR** takes a Codex32QR code and opens the codex32 form with it in
(§10.1). Once a string of a split set is in, the form offers **Make the
rest** and **Add a share** (§8.1 step 2, §8.2); **Check a string** shows
its checksum worksheet's final row; and the mark's line stands beside the
fingerprint on Wallets and on New key's Key card.

**Make a new key is New key, opening on Start** (decision 60, F44); the **Add a
key** tile opens the Add a key page instead, which takes a key held (words, a
SeedQR, codex32), in Tools and on Wallets start. New key's
first card is **Start**, open with no choice ticked (owner, 2026-10-29):
under **Make a new key**, **BIP-39 words** and **Codex32**, the same; under **Add a key you have**, **Type the
words**, **Scan a SeedQR** and **Type codex32 strings**; and under
**Other forms** (closed) **Make SLIP-39 shares**, **Type SLIP-39
shares**, **Type Seed XOR parts**; closed, it shows the choice ("Make a
new key · Codex32") and **Change** until the key is locked in. Opened
for a single key's slot (F240) Make a new key offers only the Backup
card's form, **BIP-39 words** for Words and **Codex32** for Shares, and
**Change** reopens the same list; opened for a multisig cosigner's slot
it offers **BIP-39 words** only (no Codex32, and **Make SLIP-39 shares**
is not offered there); in both, every way to add a key you have is
offered. On its own (Tools, Add a key) every choice is offered
(`docs/CODEX32.md` §6.9). A key typed is Start · **Words** · Key, one scanned
Start · Key: Words is Add a key's form drawn by its own code (count,
boxes or keyboard, lists; the other forms' fields with **Add this one**
and **Scan a QR code**), whose Continue opens Key once the words are
whole with a checksum that holds or the parts are enough; Key shows the
words behind **Show words**, **Passphrase** and **Passphrase again**
(none for codex32), and **Lock in**, which loads the key as Add a key
does (not marked made here) and fills a Create slot, and leaves the
flow by itself, with no Words or Quiz. Add a key on Wallets and its
Tools tile (one tile for making and for typing a key, owner,
2026-10-24) open with Start open and nothing ticked, and so does New key
from a Create slot (F103): a press on a choice takes it and opens the
next card, and nothing moves on before one is pressed. **Change** on
Start reopens it with its choice ticked, where a press on a choice, the
ticked one included, moves on by itself. The SLIP-39, codex32 and Seed XOR tiles open with their
choice made and Start closed. The page is titled **Add a key**, and
**New key** for a Create slot and from the Add a key screen's Make a new
key, which opens with Start closed on it. The Add a key screen (Home,
below) stays for a key asked for by name: a slot's fingerprint, a
wallet's missing key, Restore's seed slots, and a seed scanned from Home
or the Spend tab. New key keeps nothing when its screen is left, so it
has no sidebar row (Flows in progress). Opened from Tools, a key made, typed
or scanned in ends on a result card, not a toast: the key's name and
fingerprint, **Make a wallet with this key** (filled), **Another** (New key
again, from Start) and **Done** (back to Tools); none of the key's secret is on
it. Opened from Make a new wallet or Add a key it returns there as before.

**Make a new key** (decisions 34 and 38, F103). Once Start has a
choice, Length opens, no count chosen; pressing a count takes it and opens
Randomness, and pressing a way opens the entry card, with no Continue
(a mix keeps its Continue: its sources are ticked first; SLIP-39's
Length keeps its Continue: its steppers are values). Length offers no
choice of SLIP-39: shares are chosen on Start, under Other forms, **Make
SLIP-39 shares**, which opens Length with the share counts. New key's BIP-39 cards are Length · Randomness ·
*entry* · **Key** for a Create slot, and Length · Randomness · *entry* ·
Key · Words · Quiz on its own (Tools, Seeds); SLIP-39 shares keep Length
· Randomness · *entry* · Check · Words · Quiz. For BIP-39 there is no
Check card: once the last entry is in, the entry card shows the counts
and the verdict ("Fair: no cautions", or each caution in `WARN` with
**Roll again**, or the source's word, beside the foot button; a caution
is not a refusal); this device's generator, the camera and a mix as a
whole show none. The foot button appears with the last entry and reads
**Continue** where the words showed as they came in, else **Make the
words**; for a Create slot a line under the words so far says Back up
shows them again. **Key** shows the words, for a Create slot
**Passphrase** and **Passphrase again** (masked, empty is none; unequal
ones lock nothing in), and **Lock in**, which adds the key under the
self-test gate, fills the slot, and on a single-key Create makes the
wallet and marks Keys and Check done. Lock in moves on by itself (F103): it returns to Create (on Back
up for a single key, on Keys otherwise) or, on its own, opens Words,
whose head shows the fingerprint, the source and the key text; the Key
card, reopened, shows **Locked in** and the fingerprint to read, with no
Continue. Length, Randomness and the entry card then open to be read
only. The Quiz's last button reads **Done**. A key added with a
passphrase is a seed with a passphrase to Back up's plan, which asks
where it goes.

**The roller** (F96, `docs/CODEX32.md` §5.6; Faraday Demo only, never
the stick, the Pi or the website). Beside the dice entry of every dice
way of the randomness step stands one die, drawn as the dice pad's row
(DESIGN §4.3, Dice roller): **Rolled by this computer** over it, **Roll**
beside it, and **Rolls per click**: **1** (the default) · **5** · **10** ·
**To the end** · a number typed. The pad and its entries stay for real
dice, and the person uses either or both. A click's values land on the
entries as presses would, from this computer's randomness (`fresh`), all
at once in the frame that shows the click; the die then tumbles a moment
(200 ms), not at all under Reduce motion (MOTION). The entropy line reads "simulated dice", or
"simulated dice and your dice" where both made the entries; the roller
makes no value a real die did not, and the flow is otherwise unchanged.

**Make codex32 shares** (`docs/CODEX32.md` §6, §7). Start's Codex32
opens the flow's own cards: **Scheme** (Length 48 · 74 characters; Split
2 of 3 · 3 of 5 · Other, with a threshold stepper 2 to 9 and a strings
stepper k to 31 · None, the secret alone, not offered for a wallet's key (F240); for k of 4 or more a line that
the booklet publishes no translation sheet for it), **Identifier** (four
characters typed on the codex32 keyboard, or **Suggest**; one used by
the strings typed in Add a key is refused), **Entropy** (Roll dice ·
Type the characters, the latter with the line "Characters from dice or
a worksheet, not chosen"), the entry, **Characters**, the top app bar
titled "Share A · character 7 of 26" while it is open (F146), **Worksheets**, **Strings** and **Strings made**. The entry shows
the booklet's sheets filling as each value is pressed: the share's
string in numbered boxes, the dice de-biasing sheet (five tracks, the
pairs, an equal pair struck and rolled again, the bits and the tree's
path to the character), and the checksum worksheet's rows that hold the
column being entered, or a translation worksheet chosen from the strip
(Share D, Secret); expanded windows side by side, medium stacked,
compact one panel at a time (Dice · Checksum · String) under the pad. A
press on a cell opens its working. Backspace takes back the last roll,
then the last character. Worksheets reads each sheet back whole, a tab
each, ticked once shown (on a compact window **Previous** and **Next**).
Strings shows each string in groups of four, or its Codex32QR code at 48
characters, to be typed back ("Box 17 differs") or skipped with a second
press. Strings made shows the split, length, identifier, fingerprint and
entropy, **Check on another device** (the fingerprint, the identifier,
the split and the sets to try), and **Lock in**, which loads the key as
a key made here and fills a Create slot; **Done** then leaves the flow.

**Words typed** (decision 58). Add a key's words take a count first,
**12 · 15 · 18 · 21 · 24** above the boxes, 12 chosen; typing past the
boxes still grows them to 24; Clear starts the count over.

## Home

**Start** (decisions 11 and 50). Home carries no status cards: the
sidebar's status block carries them. A full-width lead tile shows only
while something is under way or waiting, the first job that applies: a
spend under way ("Carry on the spend"), a PSBT in Files ("Sign {psbt}",
after a lock cycle too, decision 3), an import waiting ("Import from
{label}"), what the last visit wrote with nothing loaded ("Written to
{label}"), a locked vault ("Unlock {name}"), a vault changed since it
was written ("Write {name} to a stick"). Then half-width tiles, two to a
row: **Make a new wallet** (Create), **Restore an existing wallet**
(Restore), **Spend from a backup, step by step** (the Spend tab) and
**Check a hardware wallet**, "Its seed, addresses and signatures"
(Check, `docs/DESIGN.md` §5; F255), always; then **Backups** once a
wallet is known. Vaults has no tile: it is in the sidebar, the rail, the
bar and the Pi's Home menu, and Create a vault is on Vaults. A stick
attached and not yet visited leads Home as "Files on {stick}", opening
Files; Home has no Stick visit tile. Create,
Restore and the Spend tab opened from Home have **Home**
as their way back, and from Wallets, **Wallets**.

**Add a key** (make, type or scan a seed) is on Wallets and in Tools →
New key on its Start card (decision 60). The Add a key screen, for a key
asked for by name and a seed scanned from Home: **Scan a SeedQR** first, full width over the words (primary until a word is
typed, when **Add key** is), a form (BIP-39 words, with the count
**12 · 15 · 18 · 21 · 24** above as many boxes, 12 first; SLIP-39
shares, codex32, Seed XOR parts), **Make a new key**, and "From a stick ·
plug it in" (with a stick attached, **Copy files in** → Files on the stick; a key
file copied in loads when the stick is pulled). **Scan** floats in the
bottom-right corner of Home on every panel → the scanner (with a stick
in, the sheet asks for it to be pulled first). Home's Scan takes
everything a scanner takes and sends it to its flow (F167; `docs/QR.md`
§2): a SeedQR, a CompactSeedQR or a seed's words go to Add a key and are
added as **Scan a SeedQR** adds them; a codex32 string (a Codex32QR
code's) or a SLIP-39 share opens Add a key on its form with the part in;
a TOTP setup code opens a new entry, filled in, in the open vault, or,
with none open, the read card offers **Unlock {vault}** (the code
carried through the unlock) or **Make a vault** (the code not kept); none of these goes into Files, from Home's Scan or any
other scanner (Files', Restore's, Create's, the Spend tab's). Transactions, wallet
files, account keys, signed messages and GPG keys go to Files and the
flow waiting for them, as before; where the next step is a press away
(a PSBT to sign, a transaction to decode, a wallet to load, split
sheets to restore, a message to check, text to save as a note or, with a
single-key wallet loaded, to sign as a message), the
read card shows that press with **Done**, in place of the toast
(`docs/SCANNING.md` §5.4). A private key (an xprv, a WIF or a Nostr
secret key) and words that fail a BIP-39 checksum are named and never
filed, since Add a key does not take them; 24 words that are an LND
aezeed by its own CRC-32C show the read card with **Use in Lightning**
(Lightning node key, the words typed in). On Faraday Demo what Home's
Scan takes pasted is what it takes scanned (F75). Add a key's **Scan a
SeedQR** takes a seed, a codex32 string and a SLIP-39 share (each to its
form), and a form's **Scan** its own part, the other share form's (onto
that form) and a whole seed (to the seed sheet); either names anything
else by what it is on the read card, with the way over to the flow that
takes it ("A transaction to sign", **Sign it**; `docs/SCANNING.md` §4,
§5). The
Wallets tab carries **Add a key** last among its ways in, "Keys first,
wallets later".
A key added with no wallet loaded lands on Wallets, whose card says
"Key 73c5da0a is loaded. No wallet uses it." with **Make a wallet from
this key** (Restore's Seeds card with the key in its slot: single key,
native SegWit, account 0 → **Make the wallet**), **Add another key** (the
same card on a 2-of-2 multisig) and **Restore an existing wallet**.
Several such keys: one line each, the buttons on the one picked. Back
from the seeds card returns to Wallets with the key still loaded.

## Session

**Boot.** Firmware loads the system → the boot medium's data partition
listed, nothing of it read but each file's name and size (F130), nothing
of it in Files yet → the import sheet over Home (decision 27, F60):
"Read from {label}", the files counted by their names in one line ("1
vault · 2 PSBTs · 4 other files, by their names"), every vault and file
with a checkbox and what it is by its name ("PSBT, by its name ·
1.2 KB"), all ticked (a seed's file too), **Unselect all** / **Select
all** above them →
- **Import** (disabled while nothing is ticked, and while the stick is
  out: "Put the stick back in to import") → the ticked files, and no
  other, read from the stick, a file over 18 MiB failing its read, a PNG as what its QR codes hold; the sheet lists them, "3 of 5
  files read", its ticks fixed → once all are answered, what they held
  comes in, with "Imported 1 wallet · 2 keys · 5 files" (and "· 1 not
  read" when the stick went first): the files From the stick, their
  wallets and keys through Files' loader (a seed of any kind, with the
  stick in, loads as it is pulled), the vaults into
  Files, locked → with no vault, Home, leading with **Sign {psbt}** when
  one came; with vaults, each in turn on the sheet: its name, "Pull the
  stick to continue" (Pi: "Pull the SD card to continue") while it is
  in, then its passphrase field with **Unlock** and **Cancel** →
  **Unlock** → **Load from {vault}** (F62; none when it holds no wallet
  or key, or all are loaded): a card per wallet with its seeds there,
  the Keys card, each ticked from its Load at unlock → **Load ticked**
  loads what is ticked (anything loaded already, a key from a file
  among them, shows Loaded and is not loaded twice), **Not now** loads
  nothing → the next vault, or, after the last,
  Wallets, on the first wallet loaded that can sign here, with "Savings
  loaded from vault.kdbx" (decision 43; Home when no wallet loaded); a
  wrong passphrase → stays, says so; **Cancel** → the vault stays in
  Files, locked → the next vault, or Home;
- **Later**, or a tap beside the sheet → Home, nothing read, nothing
  brought in; Home leads with **Import from {label}** (after Import,
  **Unlock {name}**), which opens the sheet again; with the stick still
  in, Files on the stick brings in what it ticks;
- the stick pulled before a choice → the sheet stays as it is, Import
  waiting for the stick; put back, the sheet comes up again over Home,
  ticks kept;
- nothing on the medium but the settings file → no sheet; Home.

**Unlock.** Choose a vault (its memory need shown first) → passphrase →
- opens → Home, whose lead is the next step of what is in Files (a PSBT
  in Files: "Sign {psbt}"; decision 3);
- wrong passphrase → stays, says so; no lockout (an offline copy can be
  attacked without the device anyway);
- needs more memory than is free → said before the passphrase field;
  choose another vault;
- a stick is attached → "Remove the stick to unlock", which clears itself
  when the stick goes.

**Lock** (sidebar, a vault's page, or 30 minutes idle at first, set in
Settings, F58; a warning comes up to 5 minutes before an idle lock) →
the lock sheet (decision 28, below) → vaults sealed For the stick → the
Wallets session wiped → app restarts → Unlock. The idle power-off comes
60 minutes after the last input at first (F58), counted across the lock,
so 30 minutes after an idle lock; it comes only with nothing For the
stick: with anything there, or a backup flow not finished (decision 59),
it stays on Unlock and lists what is waiting. While the backup flow is
on screen waiting for a stick, there is no idle power-off. From Make a
new wallet begun until its keys are sealed into their vaults (or its
backup is done when no stick is written) there is no idle lock, and the
sidebar says "Auto-lock off until backup complete" (F58); the person's
own Lock asks first and ends Make a new wallet as its Cancel does. The
idle power-off is not withheld when Secure Boot is off; it shows no
sheet, so its reminder to turn Secure Boot back on is the guides' last
step (`docs/STARTER.md` §7.5, F33).

**The lock, write-out and power-off sheets lead with what is kept**
(decision 28): the stick's Lock sheet, the Lock button's sheet with a
vault open (owner, 2026-10-24; its Then reads "Faraday restarts on
Unlock", its buttons **Back to the vault** (**Cancel** with no vault
open) and **Lock**, or **Lock
without sealing** under the line saying the system has given no
randomness, and a **Save** beside each key or wallet in no vault, which
puts it in the vault open now), the write-out sheet and the power sheet. Each lists **Kept** first: one line per vault with its
currency, and one for the files For the stick; then **Wiped from
memory**: one line per seed (a key with no name of its own as "Seed
{fingerprint}") and wallet, with where else it is ("in {vault}", "copy
checked", "in Files" for a wallet whose file is there) or "nowhere
else", that line alone in the danger tone and listed first; then
**Then**. At power-off a vault never written or changed since, and the
files For the stick, are under Wiped from memory, in the danger tone. The
write-out sheet adds **Dropped from Files** before Then: each Inbox
file a lock drops (`docs/SECURITY-MODEL.md` "Stateless"); its Then reads "Lock,
and the visit writes them" with the stick already in.

**Power off** → the sheet listing what is kept and what is wiped
(decision 28); when files wait For the stick or a vault has unsaved
changes, it lists them and needs a hold (decision 5): **Hold to power
off** (**Hold to quit** in the desktop app) and **Hold to restart**, each
filling while held; with nothing to lose they are plain presses → off. On a PC stick that started with Secure Boot
off, or with its state not read, the sheet leads, under its title and
before Kept, with "Turn Secure Boot back on in the BIOS before starting
Windows." in the warning tone, and the QR code of this computer's guide
under it (`docs/DECISIONS.md` F33, `docs/STARTER.md` §7.5). It asks for
no press of its own: Power off is the press. Not on the Pi, not on
Faraday Demo; a Mac follows the `mac-intel` guide. The sheet also offers
**Restart** (below, Restart; F42).

## The stick in Files

On the Pi every screen says "card or stick" for "stick" (it reads its SD
card and USB sticks alike): **Card or stick**, and **Lock and use card or stick**, "Remove the card or stick"
(`docs/DECISIONS.md` F216; its "Visit" in the top bar and Home's button are gone with the stick visit's own screen, F235). The card the Pi started from stays "SD card". The flows
below use the PC's words.

**A stick or card inserted while unlocked** → **Lock to use this stick**
(what is kept, then what is wiped from memory and where else it is,
then what happens; decision 28) →
- **Lock and use stick** → lock → Files on the stick;
- **Not now** → nothing is handed out; a banner says "Remove the stick to
  keep working" until it goes.

**A stick or card inserted while locked, or with no vault open** → Files
on the stick directly.

**The stick in Files** (F235). A stick open to Faraday, attached with
nothing held, is a pane of Files: the stick's files beside Files' own
lists from expanded windows up, and primary tabs **In Faraday · On
{stick}** below that, the arrival opening the On tab. Files' rows take a
checkbox for writing while it is open, and the buttons below stand under
the lists. Rail, bar and status strip keep their destinations (the rails six with Check, the bar five; F255); the
stick has no page of its own, and "a stick visit" is the act of having a
stick open. With something held the stick is not open: Files shows "Lock
to use {stick}" and no pane.
- **In Faraday** (the lists):
- **In Faraday** (left): chosen files written, read back, compared:
  those For the stick, then those from it under **From the stick** (below).
  What is written and matched leaves For the stick for the receipt
  (decision 25). When a file to write has a name the stick already
  holds (ignoring case), the sheet **Already on the stick** comes first,
  a row each, with **Replace** (written over the old file: the new bytes
  under a hidden name, read back, then the old file removed and the new
  one renamed to its name), **Keep both** (written beside it as
  `name-2.ext`; the row says "{name} · Keep both: {name-2}") and
  **Cancel** (nothing written, Files stays as it was). The sheet decides nothing for a vault, which brings none, nor does the
  settings file. A sealed vault replaces a file proved a copy of it, or is
  written under its own name where no file has it; a file nothing proves
  under that name is the question **Overwrite** (a hold) or **Save beside**
  as `name-2.kdbx`, one file in turn, and Cancel leaves it For the stick
  (F203, `docs/VAULT.md` §6). A failed comparison
  leaves the file For the stick and says which. The first row is the
  settings, `faraday-settings.txt`, which is not a file For the stick: ticked
  on the boot stick when the settings differ from what the boot stick
  holds, unticked on any other stick, and written over the settings
  file already there.
- **Write** while the process is no longer clean (a key loaded, a
  secret typed or the camera on since the stick came in) writes nothing
  and opens the write-out sheet, as Write it to a stick does from Files:
  **Lock** or **Not now**. The disk process refuses a stick to such a
  process; that refusal reads "not open to Faraday until it locks" on
  Files.
- **From the stick**: every file copied in, unticked, written from Files,
  where it stays for the next stick: a copy from one stick to another,
  one after the other or both in at once. A file the stick shown holds
  already (by name and size) is listed greyed, tagged **On this stick**
  (**On this card or stick**), with a disabled checkbox that cannot be ticked:
  a file just copied in shows here at once, and no "Copied" line says
  it. A public or sealed file (a GPG public key or signature
  included, F22; a revocation `.rev` file is not one, F32) is ticked like a file For the stick, and
  Select all ticks it. A secret kind (words, seed
  parts, entries), text and a file Faraday does not read
  open the secret sheet instead, never ticked by Select all: text, words,
  seed parts and entries may go into the open vault as a note ("This
  may be a secret" heads the sheet for text and other files); its
  unprotected button opens **Save without encryption?**, whose hold
  ticks the row (F129). Nothing goes For the stick. A picture is what its codes hold.
- **On the stick** (right): every file it reads ticked but a seed's (by
  its name: words, SeedQR, BIP-85), one over 18 MiB and one already From
  the stick by name and size (decision 29); **Unselect all** clears
  them. A file whose name and size match a file From the stick is
  tagged **In Faraday**. Chosen files, any of them; a file Faraday
  reads as nothing it knows comes in as a File, for **Sign a file** or
  QR **Send**, and is dropped at a lock. A vault file copied in appears
  on Vaults, locked. A
  PNG is ticked like any other file: the disk
  process decodes it, and each code it finds is routed as a scan would be
  (`docs/QR.md` §2), with a multi-part transfer showing its progress
  across images; the picture itself comes in too, as it came, to be
  written to another stick. A JPEG's codes are not read; it comes in as
  a File. A file over 18 MiB is listed and cannot be ticked.
  A
  long read shows its progress and **Stop**. A pending signed file (§7
  there) is a row For the stick, ticked when this stick holds its
  source, else "Needs {source name} on this stick"; it is written with
  progress and **Stop**, and "{source name} changed since it was read.
  Nothing written." where a part differs.
- **Rename stick** (`docs/STICK-IDENTITY.md` §6), a quiet button left of
  the sticks' chips (beside Delete files on a small panel): the Rename popup for the stick's own name, its FAT label
  of at most 11 characters, written in capitals, the line under the
  field saying how it is written, empty for no name; a boot partition's
  name (EFI, bootfs, FARADAYBOOT) is refused. The visit's line says
  what was written.
- **Delete files** (decisions 4, 54 and 55), a quiet button on the
  stick card over its Import buttons, and each row's ⋮ **Delete from the
  stick**. Its sheet, "Delete files from {stick}", has the band **Danger
  zone: what is deleted here cannot be undone**, then three choices:
  **Chosen files** (the stick's files, ticked on a list, each zeroed,
  read back and removed with its name), **The free space** and **The
  whole partition**, each saying how much it writes and about how long on
  a typical stick, the whole partition with **One pass** or **Two
  passes**; then that only the files partition is written and why, that
  Faraday only ever writes there so zeroing it reaches everything Faraday
  wrote, and what zeros do not reach (a stick's or SD card's controller
  can keep old copies of blocks; a secret ever saved to the stick
  unencrypted is surely removed only by destroying the stick); and one
  button naming the action ("Delete 2 files with zeros"), or the hold
  below, and Cancel. Where a vault file is
  among those chosen the button is a hold, "Hold to delete 1 file with
  zeros": changing a passphrase makes a new vault, and the old file,
  still openable with the old passphrase, stays on the stick until
  deleted. Zeroing the free space or the whole partition is a hold too,
  "Hold to zero the free space", "Hold to zero the whole partition"
  (F131): what it writes over cannot be read back, and a press alone
  starts nothing. The disk process does the work; a file's answer is a line on
  the visit ("{file} deleted, zeros over it first", "{file} removed by name,
  its bytes left" where the file's clusters are in use or the
  stick's folders could not all be read, or why not). The free
  space is zeroed in steps of about 8 MB, each asked for on the tick
  after the last one's answer, read back, the first step also clearing
  deleted directory entries and kept files' last clusters past their
  ends. The whole partition is first made an empty volume with its
  layout and label, then zeroed the same way, once per pass. A sheet
  shows the share done and the time left, with **Stop**; Stop, a failure
  or a pull ends it with a line in Files. A whole-device wipe is not
  offered: it would need the whole device, outside the stick process's
  boundary. A computer's own files (dot-files and Windows' own,
  `docs/STARTER.md` §4.4) are never offered or deleted one by one, and
  the boot partition is never touched.
- The title's chip says where pulling the stick goes (decision 30):
  "Then: Unlock" when a vault was sealed for it (For the stick, or on the
  last write's receipt), "Then: {flow}" when the stick came in during
  one, else "Then: Home" →
  **Remove the stick** → once gone → **Unlock**, with the vault sealed for
  the stick, or the flow it came in during; with neither, the person stays
  on Files, and the chip and the pane go.

**A stick pulled during a write** → the write is reported failed, the file
stays For the stick, and Files says to put the stick back.

## Vaults

**Vaults overview**: each vault in memory with its state (open, locked)
and its currency and source; an open one with its size and cost, a
locked one with what it held when last seen open; **Create a vault** at
the top.

A vault's currency (decision 26), computed from the receipt, leads its
row: "Never written" (sealed For the stick, on no receipt), "On {label} ·
current" (the receipt wrote the bytes it has now), "Changed since
written" (open with unsaved changes, or sealed bytes that differ). A
vault copied in from a stick and not changed reads "On {label} ·
current" for the stick it came from, remembered across a lock, or
"Unchanged" when that stick is not known. A changed vault makes Home
lead with "Write {name} to a stick".

A locked vault says what it held (decision 23, `docs/VAULT.md` §11): what
it held when last seen open is kept across a lock until power-off; its
row goes by its name and Home's lead is "Unlock {name}", both with the
line "Savings 2 of 3 · key 9a6a2580 · 12 entries · seen 14:02"; with
nothing remembered, "Unlock to see what it holds".

- open vault → **View contents** → Vault contents;
- locked vault → **Unlock** → passphrase sheet → Files, with **Load
  from {vault}** over it (F62): **Load ticked** → Wallets on the first
  wallet loaded that can sign here (Files when only keys loaded); **Not
  now** → Files, the vault open, nothing loaded. No sheet when the vault
  holds no wallet or key, or all are loaded already.
- opened from a flow that needs a vault (decision 10): **Unlock** →
  the flow, on the step it was on; the way back returns to it.

**Create a vault** (step cards: Size, then Name and passphrase;
**Customise** puts where it opens, unlock cost and space per passphrase
in Size's place) → **Create vault** → Unlock with the new
vault picked, "Type the passphrase once more to open it", sealed copy
For the stick → **Unlock** → Vault contents, "Nothing in it yet", a
button per add action and **Write it to a stick** (Files) (decision
22). Opened from a flow that needs a vault (decision 10), it shows
"Then: {flow}" by its title, and the unlock goes back to the flow.

**Generate with dice** (a passphrase's card on Name and passphrase): the
list (long, short, short 2), the rolls typed with the words they make,
and the strength so far. On Faraday Demo the roller of New key (above;
F96, CODEX32 §5.6) stands under the rolls with the same **Rolls per
click**; **To the end** rolls as far as a passphrase of 77 bits (six
words of the long list), and at most 20 words' worth of rolls in a
click. The roller's values are marked as this computer's: once **Use
these words** has filled the passphrase, its strength line reads "{} bits
from simulated dice + …", or "from simulated dice and your dice". Changing
the list starts the rolls over, the roller's marks with them.

Size (decisions 21 and 51) is **PCs only** (512 MiB) or **PCs and a
Raspberry Pi** (64 MiB, the default), both 3 passes, so the flow opens on
Name and passphrase; a size pressed moves on to it, as a preset cost
and a space per passphrase do on Customise's cards (F103), Custom's
memory and passes keeping their Continue. The Name field's caption says the name is what it
holds, for anyone who sees the stick. The size rows, the cost card and
the summary panel state one memory figure, the vault's unlock memory
(the Argon2id memory); the panel's row is **Unlock memory**. What this
computer has free is shown only where it falls short. Customise's Where
card has **This computer** ticked by default and untickable, for a vault
that asks less than this computer can give; its row reads the most
unlock memory it gives a vault (the largest custom memory that fits in
its free memory beside the app's 100 MiB, at most 1 GiB) with its free
memory in parentheses: "1 GiB (1.2 GiB free)". Another machine gives
half its memory. The cost suggested is the largest preset the weakest
chosen machine gives.

**Vault contents**: the kinds it holds (only those a slot holds
something of), then a row per add action, the Add icon in the accent
colour (decision 58): Save a key, Save a wallet, Add an entry, Add a
note, Make a key, Make keys; Save a key lists every key loaded and not
in the vault, of words or of the master seed a codex32 or SLIP-39 key
came from, and saves it as a key record (`docs/VAULT.md` §7, type 2:
the seed payload and, for a codex32 key, its set's facts in field 5);
a key record's words, an entry's password, TOTP secret and notes, a note and a
paperkey each show behind their eye (DESIGN §4.10);
an empty vault shows them as buttons under "Nothing in it yet", then
**Write it to a stick** → items → details. Unlocked, it opens on Wallets
where it holds one (decision 42): a wallet's item shows the plan's three
check lines, then the wallet's chart (DESIGN §4.16) read from the backup
up: the places, vaults, files and software of the plan this vault keeps
for it, each with what it holds and what it gives whoever finds it
alone; the keys under them, a seed the vault keeps reading "In this
vault"; the wallet at the foot. Then its descriptor and **Open in
Wallets**. Each node opens its sheet (Wallets tab, the chart's sheets);
an action there loads the wallet, and its seeds the vault keeps where
keys may load, first. While a wallet's item is shown the kinds and items
columns narrow, so the chart has room for five nodes on a line at the
desktop window's size. A wallet with no plan in this vault: its wallet
and key nodes, and "No backup plan in this vault". On a small panel the
item's page stacks the same nodes in that order, with no lines.
- **Add an entry** → form (title, username, password, URL, notes, TOTP
  secret typed or **Scan**; **Generate with dice** is Create a vault's,
  for its passphrases, not an entry's) → saved → the entry
  selected in the list; vault marked unsaved.
- Entries list → **TOTP codes** → the codes screen (below).
- An entry with a TOTP secret → **Show as QR** under **TOTP
  secret** → the QR sheet with the stored `otpauth://` URI, marked
  Secret, no PNG, nothing For the stick; **Done** closes it, and so does
  a lock (F28, `docs/QR.md` §7). An HOTP or unreadable secret has none.
- **Import entries** (from a file in Files) → preview: entries found, lines
  skipped with the reason → choose the vault → import → Entries list.
- **Edit** → the same form → back to the item.
- **Hold to delete from vault** → gone; the next item selected.
- **Copy to another vault…** or **Move to another vault…** → the sheet of
  open vaults, locked ones (**Unlock {vault}**) and **New vault with this
  item**, one that cannot take it dimmed with why → Move's dialog that it
  will be gone from this vault → a toast naming the vault (`docs/VAULT.md`
  §7.2).
- A Bitcoin key or wallet → **Open in Wallets** or **Don't load at unlock** / **Load at unlock** (F62: a wallet's, and a key's in no wallet of the vault; a key in one reads "Loads with {wallet}").
- A GPG key → **Show as QR** under its masked paperkey (F215: the
  QR sheet marked Secret, no PNG, no file); **Export public key** with
  **Show as QR** and **PNG** on its row; **Export the kept revocation** and **Make a revocation** (the
  secret sheet, no code or picture); **Sign a file**, whose list shows
  each file's SHA-256 and a small text file's text, with **Show as QR** and
  **PNG** on each file's row, and **Sign**, whose sheet shows the files
  and the signature time and holds to sign (F63); **Renew** with a term
  chosen; **Hold to mark the key revoked** (F32).
- Secure Boot keys → their page in the vault: **Owner keys**, the
  policy, **Hold for enrolment files** and **Sign or check an image** (below).
- A recovery sheet → its descriptor, with **Show as QR** and **PNG** in
  its actions (F28, `docs/QR.md` §7): `{name}-descriptor.png` For the
  stick, labelled with the wallet's shape and checksum.

A vault's passphrases, unlock cost and space are fixed when it is made:
the app has no action that changes them or makes a new vault from an
open one, and none that removes a vault from the session short of
power-off. A vault file is deleted from a stick by the stick visit's
Delete files (its hold).

**TOTP codes** (`docs/TOTP.md` §5): from a Tools tile or
the Entries list's own button. No vault open → Make or unlock one, same
as a vault category; the clock not checked this power-on → **Set the
time**, which returns to the codes. **Now** (local time with seconds and
the zone) and **Set the time** at the top, then one row per entry with a
TOTP secret in every open vault: its code, grouped, with a progress bar
for the time left in its period; an HOTP secret shows an **HOTP** badge
and no code, one that does not read shows **Unreadable**. A row opens
the entry in its vault, where the **Code** row above **TOTP secret**
shows the same code live, or **Set the time** while the clock is not
checked. **Scan a setup code** and **Add an entry** at the foot.

## Backups

**Backups** is reached from Home's second tile (with no stick attached,
once any wallet is known; on the small panel, a tile after Wallets) and
from the wallet card's backup line. It lists every wallet known, once
each by descriptor checksum: the loaded wallets, then those in an open
vault, then those a locked vault was seen to hold (`docs/VAULT.md` §11).
Under each, its backup map a row per place, as the plan draws it (each
place, the vault, a stick of files, the watch-only software, the seeds
on their own devices), each with what it holds, its tag (secret, sealed,
public) and its state:

- a paper place: **Checked** once a copy of each seed it keeps matched
  here, typed back or scanned; the first check of a seed counts for the
  first place that keeps it, the second for the second, and so on; else
  **Not checked**; where a date is kept for it, "Checked 14 months ago"
  (with **To check** in `WARN` when a copy there is due, `docs/BACKUP-FLOW.md`
  §18.3) in place of both. A place that keeps only a sheet reads its file.
- the vault: the vault file holding the wallet and where it stands,
  "vault.kdbx · for the stick", "vault.kdbx · on STICK, 14:02",
  "vault.kdbx · changed since written"; "Not in a vault" when none holds
  it; "A vault is locked" when a locked vault not seen open this
  power-on might.
- public files: "On {label}, {time}" once the last visit wrote and read
  them back, **For the stick** while they wait, "Not made"; seed files
  For the stick read "For the stick, unprotected".
- the software: its files' state, else **Shown** once the descriptor
  was shown in the backup.
- the seeds on their own devices: "Not here".

A wallet with no plan reads **No backup plan** with **Back up** (on a
loaded wallet). A locked vault's wallet with a plan in that vault and no
record here reads "Plan · In {vault} · locked". What a plan holds is
recorded when its checklist is made, and each check as it matches; the
records are kept across a lock and gone at power-off. A place's name
shows only from the plan an open vault keeps, else "Place 1". On the
small panel one wallet a page, with **Previous** and **Next**.

The wallet card's line, under its shape: "Backup: paper ×2 checked ·
vault.kdbx · for the stick" (paper "1 of 2 checked" while one is not), or
"Not backed up"; a press opens Backups on that wallet.

## Backup flow

The flow's cards, steps and rules are in `docs/BACKUP-FLOW.md` (§4, the steps; §15, the plan by place).

## Wallets tab

Every flow, entry point and step, and the rules this file's rules rest on, are in `docs/WALLETS.md` (§11, the rules).

## Spend tab

The Spend tab's pages and rules are in `docs/SPEND-FROM-BACKUP.md` (§4, the pages; §8, registering a wallet and MuSig2 spends).

## GPG

Every GPG action needs an open vault (the secret key lives there). With
none open, Tools' **GPG key** tile reads "Make a vault first" (no vault
file) or "Unlock a vault first" and leads into Create a vault or Unlock,
which come back to the vault's GPG keys (decision 10). Making a key is
the exception (F31): **Make a key** opens the backup flow (decision 59),
whose steps decide and open the vaults first.

**Make a key** (the backup flow's steps 1 to 3 (F31): two places or
more, their sticks, their vaults; then name and email, expiry with 3 years chosen, as `gpg
--quick-generate-key` makes a key with none given) → created in every vault the plan keeps it in → after the flow's lock
and writes by place, the key's page: fingerprint, signing subkey, user IDs, validity, **Revocation
certificate** ("Kept with the key · made {date}"), **Paperkey** (behind its eye
as every secret on the page; made only while it shows). The public key goes For the stick as the
key is made (decision 24). The revocation certificate does not: it is kept
in the key's record, as GnuPG keeps `openpgp-revocs.d/<FPR>.rev` beside the
keyring (F32). A key made earlier, with none kept, makes one when its page
is first opened with the clock known.

**Signing waits for the backup** (F31). A GPG key made in this session
cannot be made again from seed words, and its revocation is kept in the
same vault, so until the backup flow has written every place its plan
gives it (two or more), **Export public key** (the file again, **Show as QR**, **PNG**:
`{name}-{fp8}.png` with the user ID and fingerprint under the code).

**Export the kept revocation** and **Make a revocation** → the secret
sheet, a file `{name}-{FPR}.rev` in GnuPG's layout (its explanation, the
key, then the armour with a colon before its dashes, so that `gpg
--import` refuses it until the colon is taken out). **Make a revocation**
first asks the reason as `gpg --generate-revocation` does, 0 no reason, 1
compromised, 2 superseded, 3 no longer used, and an optional description.
**Hold to mark the key revoked** marks the key revoked here: it signs and
renews nothing after.

**Sign a file** → the page names the key as gpg-agent's pinentry does
(user ID, signing subkey ID and main key ID, creation date) and lists the
files copied in, each with its name, size, SHA-256 and, when small, its
text → tick → **Sign** → the sheet **Sign with {user ID}**: the files
ticked (name, size, start of the SHA-256), **Signature time** (the
clock's, with **Set the time**, which returns to the sheet), **Hold to
sign** and **Cancel**; only that hold signs, and with the clock not
known it is disabled (F63) → a detached signature `{file}.asc` For the
stick. A file replaced by a stick visit after it was ticked is unticked.
A signature is not written over a file of that name. An expired or
revoked key, or a clock before the key was made, signs nothing and says
why in GnuPG's words. Each file's row also shows the signature as a code
(**Show as QR**) or writes it as `{file}.asc.png` (**PNG**). No file
copied in: "Copy the file in on a stick visit first." Changing the key,
the category or the vault closes the page and clears its ticks.

**Renew** → the current term is shown, a new term is chosen (1, 2, 3, 5
years or never) → the updated public key For the stick, and the key's
record changes with it. Refused when the clock is more than 4 seconds
before the key's last signature.

**Restore from a paperkey** (`docs/GPG-PAPERKEY.md`) → the sheet
**Restore a GPG key**: a row per public key in Files, its user ID and
fingerprint, one not restored here dimmed with why, and **Scan the
public key**, whose code is filed in Files as `{name}-{fp8}.asc`; a
public key's ⋮ in Files offers **Restore from its paperkey**. A row
pressed → the backup flow, titled Restore a GPG key: the public key and
the bytes line 1 of its paperkey must read, then Make's steps 1 to 3,
then **Type the paperkey**, each line checked by its CRC-24 as it ends,
or **Scan the paperkey** (paperkey's lines or its raw form) while no
line is checked → **Restore the key**, which makes every signature of
the key again and saves the record into every vault of the plan; it
signs at once. Then Make's lock, writes and Done.

**Move a key to a vault on another machine** (F215;
`docs/GPG-PAPERKEY.md` §12) is this restore, with only a camera between
the machines: the old vault's **Show as QR** on **Export public key**
for **Scan the public key**, then at step 4 the **Show as QR** under the
masked paperkey, one code on the QR sheet marked Secret, for **Scan the
paperkey**. The old vault keeps its copy until the person deletes it. For
two vaults open on one machine, the vault's copy of an item is the way.

## Secure Boot

Needs an open vault for the keys. With none open, Tools' **Secure Boot
keys** tile leads into Create a vault or Unlock and back, as GPG's does;
**KeePass export** the same, to the vault's entries.

**Make keys** → the backup flow (decision 59): two places or more,
their sticks, their vaults → PK, KEK, db generated into every vault the
plan keeps them in → lock, the sticks written by place → Secure Boot
screen, with the **Backup** row until every place is written. **Signing
waits for the backup** (F31): keys made in the flow cannot be made again
from seed words, so until every place is written, **Enrolment
files** (its updates are signed by PK and KEK) reads **Back up first**,
and picking an image to sign or holding **Hold to sign** opens the flow;
**Check** does not wait. Keys in a vault unlocked from a stick sign at
once.

**Hold for enrolment files** → policy (Windows-compatible or own keys only; the
warning line says what may not start and that Restore Factory Keys in the BIOS undoes it) → hold → the
files For the stick. Enrolling them is done in the firmware's setup
screen; Learn has no page on it yet (per-maker guides are planned,
`docs/STARTER.md` §7.7).

**Sign or check an image** → the `.efi` files copied in, a row each
with **Sign** and **Check**. **Sign** → needs the file copied in (none: "Copy
BOOTX64.EFI in from a stick first", with what a stick visit is) → the
release check (F134): the release's `SHA256SUMS` and `SHA256SUMS.asc`
copied in beside it, the signature verified with the release key built
in, the file listed; a refusal named in place of the hold → **Hold to
sign** → signed file For the stick, under the same name. The Starter
writes it onto a stick as **One I signed** (Learn's **Replacing Faraday
on a stick**).

**Check** → under the list, what the image's signature is against the
vault's db certificate.

## APK signing key

Removed (F99).

## Explore

(F28, F76; `docs/QR.md` §7.) From a key's **Explore
this key** or the Tools tile → the path and the script type → the
**Address**, **Public key** and **Extended key** at that path, each a
row that opens its code when pressed (the value as printed, whole under
the code, no PNG) → **Wallets**.

### Make a wallet at the path

Finding a person's address at a path of their key means a
wallet is there, so Faraday makes that wallet from where the person
found it, without Create's or Restore's cards. Two ways in, one
action:

- **Explore**, at a loaded key's path in a single-key account: wherever
  Explore offers **Addresses** and **Find an address** (the path ends
  `…/0/i` or `…/1/i`, or every level of it is hardened, and the key is
  loaded) and the path is of BIP-44, BIP-49, BIP-84 or BIP-86 on the
  network's coin type, the same row carries **Make a wallet**; on Find
  an address it stands under a search's matches as well, once the search
  has ended with one. The account is the one the Addresses page names
  (the path without its chain and index), at any account number, and
  its script always the one the purpose names (BIP-44 legacy, BIP-49
  wrapped segwit, BIP-84 native segwit, BIP-86 Taproot), whatever the
  path page's script shows. An account is a standard one when its path is
  three hardened levels: the purpose, the network's coin type and the
  account number.
- **Check an address**, when its answer is a find in a loaded key's
  standard account that no loaded wallet is (`docs/QR.md` §2: BIP-84,
  BIP-86, BIP-49 or BIP-44, account 0): the read card's way over, and
  the answer card on Check an address's page, is **Make a wallet**,
  whether the first hundred found it or **Look further** did. A file of
  several addresses lists its finds and offers it on none of them; the
  address checked alone offers it.
- **Check an address**, when its answer is an address of a single-key
  wallet whose key is loaded here, or of a loaded key's standard account:
  the answer's way over is **Sign a message with this address**. A
  receive address of a wallet opens Sign a message on that wallet and
  index; any other opens it on the key's path, the Address card showing
  the address and its path. Its Address card is done and the message
  next. Nothing else offers it: a multisig or watch-only wallet, a
  wallet whose key is not loaded.

The steps:

1. **Make a wallet** pressed. Faraday writes the descriptor Create's
   Make the wallet writes for that key, account path and script, the
   single key with its origin `[fingerprint/path]` and `/<0;1>/*`
   (`create::NewKind::descriptor`; from the address check, the account
   the search held, which is that descriptor already), and adds it as
   Create adds its wallet, its source "Made here".
2. **The first address is checked** before the wallet is kept: the
   wallet's receive address 0 is worked out from its descriptor, and
   the key's own address at `<account>/0/0` from the key itself, as
   Explore's path page works it out; they are the same address or
   nothing is added and the press's place reads "Its first address is
   not the one this key gives at m/84h/0h/3h: not added", in the error
   colour.
3. **Wallet made**: a dialog over the page it was pressed on, headed
   with the wallet's name, holding the descriptor row and the wallet's
   first addresses, the same parts as Create's Check, Restore's Check
   and the chart's Check addresses (`descriptor_row`, `first_addresses`:
   the first two receive addresses and the first change address), the
   first receive address opening its QR sheet; its actions **Done**
   (back to the page, with the wallet loaded) and **Open the wallet**
   (the wallet's card in Wallets, its way back the page it came from,
   F146). A key recorded as a BIP-85 child seed shows **A BIP-85 child
   seed** after it, once for that wallet, as Create does (F166).

The default name is the key's name as Explore's key row and the sidebar
give it (`bip85::key_name`) and the account's path: "Main · m/84h/0h/3h",
"73c5da0a · m/84h/0h/0h". The card's **Rename** changes it as for any
wallet.

**A wallet already made** for that key, path and script is opened, not
made twice: a loaded single-key wallet whose one key has the key's
fingerprint and the account's path as its origin, and whose script is
the script, whatever its name and wherever it came from (made here, a
file, a vault). The action then reads **Open the wallet** in place of
**Make a wallet** and opens that wallet's card, no dialog; which wallet
it is is found by comparing each loaded wallet's key origin and script,
with no key derived.

A wallet made here is one more wallet added in the session: its card
offers **Save to vault** and **Back up**, and a lock lists it if it was
not saved. It passes no confirmation sheet: it has one key, loaded
here, with no other key path (a single-key `tr()` has no script tree,
F132), and every part of it Faraday worked out from that key, so
nothing in it came in from outside (the planted-wallet sheet's gate is
a wallet of several keys or a `tr()` key path of a key not its own).
That holds at any account number (account 3).

What is refused, the action not drawn:

- Any purpose but BIP-44, BIP-49, BIP-84 and BIP-86, the standard
  single-key purposes: a multisig purpose (BIP-45, BIP-48, BIP-87)
  needs its cosigners and goes through Create; BIP-352's wallet goes
  through Silent payments; a path under any other purpose, or none
  (`m/0h/0h`), is not a standard account.
- A coin type that is not the network's: `m/84h/1h/…` on mainnet,
  `m/84h/0h/…` on testnet or signet.
- A path in no account (it ends neither `…/0/i`, `…/1/i` nor all
  hardened), and a key not loaded.

Explore's Addresses page is unchanged: it still lists the addresses of
BIP-87 and the other paths it lists today; only the button is withheld.

Nothing of the key's secret is in the wallet, the dialog or the name:
the descriptor holds the account's extended public key. The press leaves
Explore's eyes as they are while the dialog is over it; **Open the
wallet** leaves Explore, which hides them all as leaving does (F125).

Guide (`docs/guide/explore.md`, Explore): "**Make a wallet** makes the
wallet of this account and shows its first addresses; where it is
already loaded it reads **Open the wallet**."

## Lightning node key

From the Tools tile → ldk-node from a loaded key, or LND with its
aezeed typed (or read by a scanner: its read card's **Use in
Lightning** opens the page with the words in) → **Node id**, with **Show as QR** and **PNG** (F28,
`docs/QR.md` §7;
`lightning-{8 hex}.png` For the stick, "{name} waits for the stick"),
**Made**, the private key masked until **Show** and never a code →
**Save into {vault}** (decision 10's way when none is open), **Out
unprotected…** (the secret sheet).

## QR transfer tab

On the device this is Files and Scan, already: Files' **Show as QR**
sends a file in Files as codes (format, frame rate, part size,
progress, **Done**), and **Scan** receives into Files, routed by
`docs/QR.md` §2.

**Transfer**, in the desktop app only (the online Faraday; never on the
stick or the Pi), is the other end, the computer beside the device:

**Send** → a file from the list of `~/Downloads`, newest first, or one
dropped on the window (X11 only) → animated code, chosen by kind as Files
chooses it (a PSBT as itself; a wallet, key, message or share as text;
anything else in the file envelope) → format, frame rate and part size →
**Done**. Over 262.1 KB: "Larger than 262.1 KB: carry it on a stick".

**Send text** → text typed into the field or put there by **Paste**
(Ctrl+V) → the line under it gives the characters and size, or says why
it is not shown ("Not shown: this is a seed's words", "Not shown: it
holds a private key", "Over 16.4 KB: send it as a file") → **Show as QR**
(on while the text may be sent) → one code, or animated when it does not
fit one, with format, frame rate and part size as Send has them →
**Done**; **Clear** empties the field. A secret is refused by the
device's own reading of it, whole or in any part of the text (F236); bare
hex (a hash, a transaction id) is shown. Leaving Transfer wipes the
text; nothing is saved.

**Receive** → the webcam reads anything, routes nothing → each whole
file saved into `~/Downloads` under its own name (envelope) or
`received-N.psbt` / `.txt`, never over a file ("name (2).ext") → "Saved
{path}"; a file that holds a secret (words, a SeedQR, seed parts or
entries) is saved only past **Save without
encryption?**'s hold (F129), **Cancel** saving nothing, and then "Saved
{path}: this file holds a secret" → the camera stays on for
the next → **Cancel**. Nothing goes into the desktop app's Files; a file
to use in the desktop Faraday is copied in from a stick folder.

## New devices and settings

**A keyboard or pointer arrives** → it does nothing until it types the
code shown, or a person lets the pointer in with input already trusted
→ **Ignore this device** lists it in Settings until unplugged. A
keyboard that types three codes' worth of letters without the code is
ignored the same way, with a toast saying to unplug it to try again.

**A USB touchscreen arrives** (F153) → it does nothing until it taps
the target drawn at a random place on the screen → a tap beside it
shows the tries left, and the third keeps it out until unplugged, with
a toast saying so. Its tap on **Ignore this device** keeps it out. A
touchscreen that is part of the machine (the Pi's panel, a laptop's
own) needs nothing. After three keyboards or touchscreens are kept out
for wrong tries, new ones are ignored until Faraday starts again.

**No camera, or the camera is refused** → the scanner says which, and
offers Type and Read a file.

**Settings**: keyboard layout (US first), display scale, text size
(Standard, Large, Larger, `docs/TEXT-SIZE.md`), idle lock and
power-off times (30 and 60 minutes at first, F58), reduce motion, light
or dark (light on a device with no settings saved, decision 32), the
network (decision 14), ignored devices, **Time** and About (version, build hash). They are kept across a
lock in RAM, and across a power-off in `faraday-settings.txt` on the
boot stick, read at boot before the passphrase (`docs/FLOWS.md` "Files"). Never
for an idle time, the signed-amount memory's seal and the network are
not read from the file.

**Time** (decision 56, `docs/TOTP.md` §1–§3): **Now**, the local date
and time, or **Not known** (a reading before 2020 is not known: the Pi
has no real-time clock and starts at 1970, and a PC running Windows keeps
its hardware clock in local time); **Time zone**, a fixed offset from
UTC stepped by 15 minutes, kept in `faraday-settings.txt`; **Set the
time**, which opens **Set the time** (a Date and a 24-hour Time field,
**Set**, and **Keep this time** once the clock is already known) and
returns to Settings. Setting it stores an offset against the shell's own
clock, kept across a lock, lost at power-off; Faraday never calls
`settimeofday` or `hwclock`. Every date or time shown to a person is
local; what goes into a format (a GPG signature's time, Secure Boot's)
stays UTC.

## Flows in progress (owner, 2026-10-16; F41)

**Why.** The owner was on Places in Create a wallet, opened Settings to
change the theme, and found no way back. The flow was kept (pressing
**Create a wallet** again resumes it), but nothing on screen said so.
The code: `faraday-core/src/in_progress.rs`; the sidebar's `flow_row`,
the small Home's tiles; `Action::Resume`.

1. **A flow in progress has a row in the sidebar, under its tab.** A
   flow is in progress from its first step done until it is finished,
   started over or let go, or its state is wiped (a lock wipes what it
   wipes today). Each has one row, indented under the sidebar entry it
   is opened from: Make a new wallet, Restore a wallet, Back up a
   wallet, Spend, Sign a message and a wallet's backup flow left on its
   own ("Backup of {wallet}") under **Wallets**; Create a vault under
   **Vaults**; the GPG and Secure Boot flows under **Tools**. New
   key (with Add a key) has none: it keeps nothing when its screen is
   left (survey below). The row reads the flow's name and, at its right
   end in the muted tone, its open step ("Make a new wallet · Places");
   a press opens the flow on that step, as it was left. Make a new
   wallet, the vaults it sets up before its keys and the Back up it
   ends in are one row, named Make a new wallet (the name on its own
   screen and on Home: "Create a wallet" is the old name); Restore and
   its vault setup are one row too.
2. **The row is lit while its flow is on screen**, and its tab is not;
   on any other screen the tab is lit as today.
3. **What a row never shows**: a key's words, a passphrase, or any value
   the flow holds. A wallet's or key's name, already in the sidebar's
   lists, may be its name.
4. **Room.** Rows are added after their tab, in the order the flows
   were started; the seeds and wallets groups at the foot give up rows
   as they do today (`sidebar_caps`). A flow that keeps nothing when
   left (a calculator, Explore, Check a signer) has no row.
5. **The narrow window and the small panel** (the compact layout): the
   small Home leads with the box of what is loaded, a line per flow in
   progress, its name and "in progress", opening it, before the keys and
   wallets.
6. The survey of every screen whose state is kept when it is left:
   below.
7. **Speed.** Which flows are in progress is worked out once a frame
   through the drawing memo (`docs/SPEED.md` §3.3) when it costs more
   than lookups; a press decides from the state as it is.
8. Tests.

Considered and not chosen: a "back to the flow" line at the top of
every page (it remembers one flow only, and is a line on every working
screen); Settings as a sheet over the screen (mends this case only).

### Survey (item 6, 2026-10-19)

Every screen whose state is kept when it is left, read from the code:

| Flow | Tab | State | First step done | Finished | Started over, let go | A lock |
|---|---|---|---|---|---|---|
| Make a new wallet, its vault setup, its Back up | Wallets | `create`, the wallet flow of its `owner`, `backup` of the wallet made | a card closed as done | its Back up's checklist all done | **Start over**; **Make a new wallet** once the wallet is made (it resumes only while keys are chosen and none made, or the vaults are being set up) | wiped; the setup's flow is kept as the wallet's |
| Restore a wallet, its vault setup | Wallets | `restore`, the wallet flow of its `owner` | a card done (from a key on Wallets, Kind to Description are done for the person) | Check done: on Done | **Restore** pressed again, unless the vaults are being set up | wiped |
| Back up a wallet, on its own | Wallets | `backup` | a preset, a plan saved before, something placed, or the checklist made | every checklist item done | **Back up** on any wallet's card (the plan comes back from a vault) | wiped; its writes go on in the wallet's flow |
| Spend: the Spend tab, a transaction's steps | Wallets | `spend`, `spend_backup` | a transaction read, or something in on What do you have? | signed and put out | **Start over**; another wallet's spend | the spend is wiped; the tab's own state is kept |
| Sign a message | Wallets | `message` | a card done | the signature made | **Sign a message** pressed again | wiped |
| Create a vault, on its own | Vaults | `vaults.create` | a card of Customise done (the size is done for the person) | the vault made | **Create a vault** pressed again; a flow's detour dropped on Back | wiped |
| GPG, Secure Boot keys | Tools | `backup_flows` | past Where it will be kept | Done | the kind's tile resumes one not made; nothing lets one go | kept |
| A wallet's backup flow, on its own | Wallets | `backup_flows` | past The plan, or the keys made | Done | Back up's plan | kept |
| New key, Add a key | — | `keygen` | | | dropped when its screen is left | |
| BIP-85, Convert a seed, Silent payments, Lightning, Explore, the calculators, Vanity, the word list | — | dropped when left | | | | |
| A transaction taken apart, Check a message, a stick visit, Set the time | — | one page, or set afresh each time | | | | |

What the spec did not foresee:

- **New key keeps nothing when left** (`press`: "Leaving the new-key
  screen drops what it held"), so it has no row. Those drops, and
  BIP-85's, Explore's, the calculators', Vanity's and the others', run
  in `Faraday::press`, the scripted way in; a tap goes through `touch`
  and `act` and skipped them, so on a device these screens kept their
  state when left by a tap. Since 2026-10-19 they run
  after every press however it comes (a tap, a key, the mouse's back
  button, a stick that opens another screen, a scripted press:
  `after_input`), so a screen left by any of them keeps nothing, and
  the mouse's forward button has nothing to reopen.
- **Sign a message** keeps its cards when left: it has a row.
- **Create a vault** opens with its size done for the person, and
  making the vault finishes it: a vault made in one sitting never has a
  row; one with a Customise card done has.
- **Make a new wallet** is finished only when its Back up is: a wallet
  made and not yet backed up keeps the row on Back up until a lock or
  Start over.
- **The Spend tab is kept across a lock**: a route it was given brings
  its row back after a lock, on its open page.
- A row opens the flow's own screen (Make a new wallet, its Back up or
  its vault setup, whichever it was last on), not a detour it opened:
  Add a key from Restore, a vault made for a flow, a key made for a
  slot.
- Rows are ordered by when the flow was begun, noted after each press;
  a flow kept across a lock comes after those noted.
- The wide layout's Home has no tiles for them: the sidebar is there.

### Restart (owner, 2026-10-16; F42)

**Why.** The power sheet offered only Power off. Turning Secure Boot back
on means opening the BIOS at the next start, and a restart is the moment
for it: the maker guides already say to use Restart rather than Shut
down (Dell's: "Shut down can skip the key").

1. **The power sheet offers Restart beside Power off**: **Cancel**,
   **Restart**, **Power off**, Power off still the primary. Restart is
   Power off in everything but its end: the same sheet of what is kept
   and wiped, the same holds (files waiting asks first, decision 5; a
   backup not written holds it, decision 59), the vaults sealed and
   memory wiped as the app ends; then the computer restarts instead of
   powering off. The sheet's title is "Power"; its line on the computer
   reads "The computer powers off, or restarts".
   A restart keeps the memory powered, so before either ends, every file
   waiting in the RAM directory is removed (a seed file copied in from a
   stick, a secret put for the stick), the screen is blanked after the
   goodbye, and init ends the disk process and drops the block cache.
2. **Where.** The PC stick and the Pi. Not Faraday Demo, an app on
   someone's own system, whose sheet stays as it is. The idle power-off
   is unchanged: it never restarts.
3. **The BIOS key.** Where the sheet shows the Secure Boot reminder
   today (a PC stick started with Secure Boot off or unread, not a Mac:
   `secure_boot_reminder`, STARTER §7.5), it shows under the reminder
   line a row **Usual BIOS key** with this computer's key, from its
   maker page (owner, 2026-10-19: "usual", since a maker's models can
   differ and fast startup can skip the key; the guide's code covers
   both), and the guide's QR code as today. The row is a label and its
   value; no other line is added. A computer no maker page matches has
   no row: the reminder and the guide's code, as today.
4. **The keys**, each as the maker's own guide says it
   (`docs/guide/start/maker-*.md`), a field of `Maker` in
   `faraday-guide`, its text in `faraday-strings`:

   | Maker page | BIOS key |
   |---|---|
   | Dell | F2 |
   | HP | Esc, then F10 |
   | Lenovo ThinkPad | F1 |
   | Lenovo IdeaPad, Yoga, Legion | F2 (or Fn+F2) |
   | ASUS | F2, held |
   | Acer | F2 |
   | MSI | Delete |
   | Framework | F2 |
   | Surface | Volume Up, held |

   A maker page that changes its key changes this table in the same
   commit; a test reads both and fails when they differ. The maker is
   matched on the firmware's maker, family and model (SMBIOS); a desktop
   chassis changes no key here (2026-10-19: ASUS's page opens the BIOS
   with F2 on desktops too, and MSI's key is Delete on both), so the
   chassis is not read for it. A maker whose desktops take another key
   adds a desktop key to its row then.
5. **How the machine restarts.** The app ends with an exit status of
   its own for Restart (75 stays the lock's), passed through the shell
   API as a command beside Exit. The image's inittab loop (and the dev
   variant's) keeps the status it ends on and runs `reboot -f` for
   Restart's, `poweroff -f` for any other. The app, not root, still
   does neither itself. The last screen stays up as long as Power off's.
   A restart keeps the memory powered where a power-off lets it fade,
   so Restart never does less wiping than Power off: the app's secrets
   are wiped as it ends (its zeroizing types run as it drops) and the
   kernel zeroes every page freed (`init_on_free=1`), the same path as
   Power off's. Nothing is written to the computer.
6. **Strings** (`faraday/faraday-strings/src/en/`): "Restart",
   "Power", "The computer powers off, or restarts", "Usual BIOS key", and one
   per key in item 4.
7. Tests.

Ruled out (owner, 2026-10-16): **Restart into the BIOS**, the computer
restarting straight into its firmware screen by the UEFI variable
`OsIndications`, as Faraday Starter does on Windows from the person's
own system. The stick writes no firmware variable: README (Stateless,
"no access to its firmware variables"), `EFIVAR_FS` in the x86_64-uefi
board's `kernel.forbidden`, and the Learn pages that cite it. It would
also need a privileged helper for the write. The BIOS key row does the
same job and writes nothing.

### Cancel: leaving a flow for good (owner, 2026-10-19; F51)

A flow in progress has a way out where the person is, not on its
sidebar row (a press on the row only returns to the flow).

1. **Cancel** at the top of every flow that has a sidebar row: it ends the flow, its row goes, and the
   screen it was opened from is shown as it was seen (F77); the screen
   in the table's last column only when that one is not known.
2. **Asked first only when something would be lost** that the person
   would notice: a sheet naming it ("Cancel making this wallet? The plan
   and its places are lost."), **Cancel the flow** and **Keep going**.
   A flow with nothing done past its first card ends at once.
3. **Nothing secret is lost silently.** Keys made or loaded stay in the
   session; a vault with unsealed changes asks as a lock does; files in
   For the stick stay. What ends is the flow's own state: its answers, its
   plan, its board.
4. Create's **Start over** stays: it begins the same flow again; Cancel
   leaves it.
5. Tests.
6. **A key flow opened from Make a new wallet has Cancel too** (owner,
   2026-10-24; F115): New key or Make codex32 shares opened for a slot
   (`Action::KeyGen(Some(slot))`), until its key is locked in. It has no
   row of its own and ends no flow: it returns to Make a new wallet on
   its Keys card, the slot as it was, or on the card before Keys where
   Keys was passed over for this flow (WALLETS §5). It asks first only
   when rolls, characters, words or a passphrase are entered: the
   dialog "Discard this key?", "What is entered for this key so far is
   discarded, and you return to Make a new wallet.", **Keep going** and
   **Discard**. New key opened on its own (Home, Wallets, Tools, Add a
   key) has no Cancel; its back link is as it was.

The code: `faraday-core/src/in_progress.rs` (`flow_cancel`,
`cancel_loses`, `flow_end`, `cancel_sheet`; `Action::CancelFlow`,
`Sheet::CancelFlow`; for a key flow `key_cancel_offered`,
`key_cancel_loses`, `key_cancel_end`, `cancel_key_sheet`,
`Sheet::CancelKey`). What each Cancel ends, and what its sheet says:

| Flow | Asks when | The line | Shown after, when where it was opened is not known |
|---|---|---|---|
| Make a new wallet | its Back up's plan begun; or its vaults' setup begun; or the wallet made, or a card done past the first | "The plan and its places are lost."; "Its answers and the setup of its vaults are lost."; "Its answers are lost." | Wallets, or Home when opened there |
| Restore a wallet | its vaults' setup begun, or a card done past the first | as Make a new wallet's | Wallets, or Home |
| Back up a wallet | always (its row is its plan begun) | "The plan and its places are lost." | the wallet, or the vault's contents it was opened from |
| Spend | a step of the transaction done | "Its steps and any signatures made are lost." | the wallet it was opened on, Home or Wallets |
| Sign a message | a card done past the first | "Its answers are lost." | Wallets |
| Create a vault | a card done past the first other than the size | "Its answers are lost." | Vaults, or Home |
| A key's or a wallet's backup flow, its key not made | past Where it will be kept | "The plan and its places are lost." | Tools, or Wallets for a wallet's |
| A backup flow whose key is made | no Cancel, on its screen or as the flow on screen: it ends only by finishing (owner, 2026-10-19) | | |

The titles: "Cancel making this wallet?", "Cancel restoring this
wallet?", "Cancel this backup?", "Cancel this spend?", "Cancel signing
this message?", "Cancel making this vault?", "Cancel making this key?"
(GPG), "Cancel making these keys?" (Secure Boot). The setup of a
wallet's vaults and a Back up's own flow go with Make a new wallet,
Restore or Back up while their key is not made; one whose key is made
stays, its writes waiting under a row of its own: no Cancel lets go
of F6's and F31's hold. Cancel closes no vault and seals none: an open vault
keeps its changes, and a lock asks about them as it does today.

## Nonce check

**A signed transaction brought in is checked, not signed** (owner,
2026-10-29). A PSBT that carries a signature and that no key loaded here
is left to sign (each key a key origin names on an input not yet final
has signed it, or is not here), and a finished transaction, open the
nonce check (`Screen::NonceCheck`) from every way in, with no choice in
between:

| From | What opens it |
|---|---|
| The camera | the file read: it lands in Files and its check opens; the way back is to where the scan started. The open spend's own PSBT come back stays with the spend |
| Files | the row's one button, **Nonce check**, in place of **Sign** or **Decode** |
| Tools | **Nonce check**: the finished spend's Send it card, as before; else the first such file in Files; else the camera |

A PSBT that a key here is still to sign goes to signing as before. The
spend's own nonce check on its Send it card is compared with the history
and remembered the same way, and the spend cannot be finished while it
refuses.

The screen is a page (Material 3's top app bar with its way back, the
page under it): the file's name; the nonce check as Send it draws it
(each signature against its key and the transaction, every one wrong
refusing it, a nonce used twice refusing it, each signature under a key
loaded here signed again under both nonce rules and the rule named,
"signed elsewhere" for another device's key, "no key origin" for a
signature whose key no origin names, as in a final raw transaction, and
"finalized" for one under a key here read from a final witness); then,
where nothing is wrong, what a person does with a signed transaction:
**Save to a stick** (the finished transaction as `{name}-final.txn`, or
the file as it came; **Remove** while it waits For the stick), **Decode
it**, **Show the raw hex** and **Show as QR** for a finished one, the
PSBT's code for one still wanting signatures ("Not finished: more
signatures are wanted"). A PSBT with every signature it needs is
finished here as Spend finishes one. A wrong signature offers nothing
to carry on ("Not finished: a signature is wrong"), and nor does a final
`scriptSig` or witness that does not match the output it spends, with or without a
signature row ("Not finished: signing data does not match its output", or
"Not finished: final fields do not satisfy the script they spend" where its
shape matches and the script's interpreter refuses it (a final taproot
key-path signature that fails, signed over the annex when the witness has
one, and a script-path signature of a Tap miniscript leaf that fails are
refused so; a leaf that is no miniscript, or of another leaf version, or
is exactly `DUP HASH160 <h> EQUALVERIFY CHECKSIG` spent with a key of an
unknown type (neither empty nor 32 bytes, no element over 520 bytes) and a
non-empty signature, is not run, so Finish does not refuse it, and a
64-byte element on it is data there. A Schnorr signature row of such a
leaf that would read Invalid is rewritten as not checked and counts no
key: a final one as an ECDSA final row of a script not run does, and one
in `tap_script_sigs`
under a leaf the input files with a control block that commits to the
output and states the leaf version of the block, so Collect takes the
file (an ECDSA partial row on a script not run is not so: forged or
unverifiable it stays wrong and Collect refuses it). Under
`OP_CODESEPARATOR` a signature that verifies reads as not run too, the
message worked out being no true one. An element of 65 bytes ending in a
sighash byte other than 00 and 01, and a true signature with 00 appended,
are data to Finish on such a leaf but still refused by Collect, which
cannot tell them from a signature. A true partial row reads Valid and
counts no key; its headline is the count signed and sound ("valid") on a
leaf that drops, and "not checked" (the unchecked count) on a leaf with
`OP_CODESEPARATOR`; a forged one reads "not checked", a key loaded here or
not, and never "a signature is wrong". A row's own text says its key is not
counted because its script was not run, never that it has no origin. A
partial row alone leaves the headline at "not checked", not "a script was
not run". Collect takes no final witness that reads as not run for an input whose
rows finish without it, whichever file brought the witness or the rows
and in whatever order, each input judged alone; keeps an own row over a
file's for the same key and leaf where the file's reads not checked and
the own does not read wrong, an output being read (a separator leaf's
included; on a run leaf a forged row is refused); and keeps an own script
entry that commits to the output over a file's entry of the same control
block that does not. A final witness the output judges true (a run leaf, the
key path) stands over one for a leaf not run, in either order. Where the spend
and a file differ and the device cannot judge between them, Collect refuses
the file in either order and says so without calling the file wrong: two
different final witnesses for leaves not run (or with no output stated to
read them), two different rows for one key and leaf with no output stated on
either side, and two different scripts for one control block with no output
stated. A file that brings a row for a key and leaf the spend lacks, or
holds differently, is refused too when neither the spend nor the file states
the input's output (an honest signer needed the output to sign, so an honest
file states it); a row equal to the spend's is no new row. Where a spend's own
statement of the output is false (a previous transaction of another txid),
that is the reason given, not that no output is stated. A file's wrong
script at a control block the spend lacks is taken with no
output stated, and a later true file repairs it. Two different final
witnesses for leaves not run are refused unless rows that finish the input
beat both, and the same refusal covers a `final_script_sig` on a non-taproot
script not run. The filed leaves and the leaves holding `OP_CODESEPARATOR` of an
input are worked out once per input, a control block check per entry and a
script read per entry; a row then costs a signature check and a read of its
leaf's script, so under a large leaf it costs a pass over that leaf for
each row. A key of an unknown type inside a larger Tap miniscript leaf is
refused, though consensus takes it), one
error line per input; the headline says so when no signature that stands
is wrong and no nonce repeats, an unread input or an unchecked row on
another input notwithstanding), nor a PSBT with an input, final or not,
whose previous transaction is of another txid or lacks the output it
spends, or a final input that states no output ("Not finished: an
input's output cannot be read", no Save to a stick, no Show as QR), nor
one with an input, final or not, whose `witness_utxo` states another
output than its previous transaction of the right txid (the transaction
is the one a node reads: the headline says how many inputs "state an
output their previous transaction does not", the line below says "Not
finished: an input states an output its previous transaction does not",
and a signature made over the stated amount reads wrong against it).
The disagreement is what is said when no signature is wrong and no
nonce repeats, beside another unread input or a failing witness too (the
failing witness's own rows read Invalid and do not count); a wrong
signature on any other row (a signature over the stated amount on the
disagreeing input itself included), or a repeated nonce, is what is
said otherwise. An input not yet final that
merely states no output, and a partly signed PSBT whose inputs are
honest, are offered as they are. A raw transaction
whose inputs state no output they spend is checked against the PSBT in
Files of the same transaction, as Decode reads its amounts; with none,
its headline says how many inputs could not be checked.

A cosigner's file brought into a spend (Collect) is refused, by name,
whatever the spend's own copy holds, when it states something false on
its own (a `witness_utxo` against its previous transaction, a previous
transaction of another txid or without the output) or when its
`witness_utxo` differs from the output of a previous transaction of the
right txid the spend holds, or from the `witness_utxo` the spend states
where neither holds such a previous transaction (a bare one proves nothing:
the spend keeps its value, and the refusal says the two state different
outputs and neither is proven); final or not, and the spend's PSBT stays as
it was, a later clean file still collecting. Of two previous transactions
the one of the right txid is kept, its hash proving it; a `witness_utxo`
alone proves nothing and repairs nothing. Where the spend itself holds a
false statement no file fixes, the reason is the spend's, naming no file.
A repair is all or nothing across inputs: a file repairing one input
while another stays false is refused, blaming no file, and a single file
repairing every false input is the way out.

The check is made once, when it opens, against the keys and wallets
loaded then; Decode's way back makes it again when they have changed.

### Nonce history

**The nonce check remembers, in an open vault, the
signatures it has checked**, and refuses a signature whose key used the
same nonce before on another message, in this session or any before it.
A key that signs two messages with one nonce is given away to anyone who
sees both, so the second is never finished here.

**What is remembered.** One entry per signature that verifies: the x
coordinate of its key, its nonce point (ECDSA's `r`, or BIP-340's `R.x`,
on the key path, the script path, partial fields and final witnesses
alike) and the sighash it verified against. This device's own signatures,
a cosigner's brought in by Collect, and every signature of a file the
Nonce check page opens are remembered alike, but only under a key proved
here, derived at the place the file states for it by a key of a wallet
registered here or by a loaded master; any other key, a fingerprint filed
beside it or not, is neither remembered nor compared. A row not checked (no output
stated, a leaf not run, a preimage) has no message, and is neither
remembered nor compared. The entries of one transaction are kept
together, as `docs/VAULT.md` §7 writes them.

**What reuse is.** The same key and the same `r` over a different
message. The same signature again (another file, a copy of the
transaction) is no fault. Each key is compared with itself only: anyone
can make a valid ECDSA signature over any public `r` under a key worked
out from it, so a file could otherwise claim a key was given away.

**MuSig2.** On the spend, a cosigner's partial signature that verifies
(F220) is remembered under the participant's key, its first public nonce
point as `r`, and SHA-256 of the sighash and the aggregate nonce as the
message: the same round collected twice is the same entry, and that
nonce in any other round is reuse. A public nonce with no partial is not
remembered. The Nonce check page remembers the final signature only.

**When a nonce was used before.** The nonce check's headline is as for a
nonce used twice, and under it one line per key, "{key}: nonce used
before, on another transaction, on inputs {i}"; nothing finishes, and
neither the spend nor the Nonce check page offers **Save to a stick** or
**Show as QR** ("Not finished: a nonce was used before"). A nonce used
twice inside the transaction is refused the same way on the spend, and a
spend refused gives no part file for the next signer either. A refusal,
once found, stays until that spend is closed (or the Nonce check page
leaves that file), even if the vault is closed or the history turned Off.
There is no way past it (F136, F227).

**The line under the headline**, the same on Send it and on the Nonce
check page:

- "Not seen before in this vault", or "Not seen before in {n} open
  vaults";
- "Not checked against this vault: {n} nonce records do not read",
  amber, not a refusal: a record that does not read is kept as written
  (F194), and a read-only vault (a newer format) is compared against
  nothing;
- nothing with no vault open or the setting off. What the history is
  and what it cannot show is Learn's.

**Vaults.** The history is read when a vault is unlocked and written
into every open vault when the vaults are sealed, as signed amounts are
(`docs/WALLETS.md` §3.3); a record read from one vault is never copied
into another. With two vaults open a signature is compared against both.
A vault opened during a spend compares the spend's signatures, and the
Nonce check page's, again, and they are written at the lock. A vault
locked alone writes what is waiting and leaves; the others keep it.
Nothing is kept across a lock outside a vault: **Lock without sealing**
loses what was not yet written, as any change. A vault keeps up to
10,000 entries, the oldest transaction dropped first, and at its ceiling
nonce records are dropped before signed amounts (`docs/VAULT.md` §5).

**The setting.** Settings carries a card after Signed amounts, **Nonce
history**, with **Kept in an open vault** · **Off**, the status line
"{n} signatures remembered" over the open vaults. Off, nothing is remembered, compared or
drawn, and a vault's records are neither read nor added to; at the
vault's ceiling they still go first, as above.
The default is **Kept in an open vault**.

**What it does not show.** It sees only signatures brought in or made
here, while the setting is on and a vault is open, per vault, up to its
cap. A dishonest signer can leak a key without reusing a nonce, so a
clean history does not show a signer is honest. Sibling keys of one xpub
that share a nonce are caught only inside one transaction, by the rule
above. Whoever opens a vault reads each checked input on chain by its
`r`, transactions of wallets not held here included, and a file vault
grows about 200 bytes a signature.

## Check a signer

The flow, its screen and its findings are in `docs/SIGNER-CHECK.md` (§3, the flow; §4, the screen).

## My seed makes my wallet

**Faraday shows what a seed makes, and the person checks it.** A person
who set up a hardware wallet types its words (or scans its SeedQR) and
its passphrase, picks the account their wallet app uses, and Faraday
shows the fingerprints, the account's xpub and its first receive and
change addresses, each whole. The person holds them against what the
wallet app and the hardware wallet show. Nothing has to be typed or
scanned from the wallet app to get a result. At the end the person drops
the seed or keeps it loaded. Its screen
is `docs/DESIGN.md` §5, My seed makes my wallet.

| From | What opens it |
|---|---|
| Check | its first row, **My seed makes my wallet**: the Seed card open |
| Tools | the tile **My seed makes my wallet** in Keys and seeds, after **Explore a key**, its line "Seed words to its xpub and addresses", tags BIP-32, BIP-39, BIP-84: the same. Tools stays the list of every flow; Check is a second way in |

It opens with nothing needed first, no key and no wallet. The screen is a
step flow (`Screen::SeedCheck`) of four cards: **Seed**, **Passphrase**,
**Account** and **Result**.

1. **Seed.** **Type the words** (Add a key's BIP-39 form: the count
   **12 · 15 · 18 · 21 · 24** over its boxes, every word list it reads),
   **Scan a SeedQR** (Standard and Compact; the scanner titled "Scan the
   SeedQR") and, where keys are loaded, **A key loaded here**, a row per
   key by its fingerprint with its "passphrase" mark. Decisions 8 and 9
   hold: with a stick in, Type the words and Scan a SeedQR open the sheet
   that asks for it to be pulled. A SLIP-39 set, codex32 or Seed XOR parts
   are loaded with Add a key first and taken here as a key loaded here.
   The words are whole once their checksum holds, and the card closes on
   "{n} words". A key loaded here skips Passphrase: it is the key as
   loaded.
2. **Passphrase.** **No passphrase**, or **Passphrase** and **Passphrase
   again**, each behind its eye, as New key's Key card asks them, with
   **Use this passphrase** once they match. A SeedQR read asks it on
   decision 52's sheet (**Use without a passphrase**, **Add a
   passphrase**), and this card is closed on the answer. The card closes
   on "Fingerprint 73c5da0a", with " · with a passphrase" where one was
   given. **Change** on Seed or Passphrase takes the key again, and
   works out the Result again if an account is picked.
3. **Account.** The accounts a wallet app makes from one seed, a list
   row each, at account 0 and the network in use (coin type 0 on
   mainnet, 1 on the test networks), each with its path and the start
   its addresses have:
   - **Native SegWit**, BIP-84, `m/84h/0h/0h`, "Addresses start bc1q";
   - **Taproot**, BIP-86, `m/86h/0h/0h`, "Addresses start bc1p";
   - **Nested SegWit**, BIP-49, `m/49h/0h/0h`, "Addresses start 3";
   - **Legacy**, BIP-44, `m/44h/0h/0h`, "Addresses start 1";
   - **Multisig**, BIP-48, `m/48h/0h/0h/2h`, "A cosigner's key";
   - **Another account or path**: Explore's path field and script
     buttons (an account path such as `m/84h/0h/3h`, or `m/48h/0h/0h/1h`
     for a Nested SegWit multisig), with **Use this path**.

   (On the test networks the paths read `1h` for coin type and the
   starts tb1q, tb1p, 2, m or n.) A press on a row picks it and opens
   the Result; the card closes on the row, "Native SegWit ·
   m/84h/0h/0h", and **Change**.
4. **Result.** What the seed makes at that account, worked out on the
   press that picked it (and again on a seed or passphrase taken again).

**How the account is picked** (decided here). Nothing is assumed: the
person picks the account their wallet app uses, and every result names
its path and script. A list rather than all four at once: the result is
then one account's few strings, each whole, to be read against the
wallet app's one screen; four accounts side by side would be four xpubs
and eight addresses to tell apart. Each row's address start lets a person
who does not know the BIP number find the account from the addresses
their wallet app shows. A wrong pick is seen at once (the addresses do
not match), and **Change** on Account, or **Another account** on the
Result, picks again with the key kept.

**The Result** shows, as fact rows:

- **Fingerprint**: the key's, with " · with a passphrase" where one was
  given; and **Without the passphrase**, the same words' fingerprint with
  no passphrase, where one was given. Both are there to be held against
  what the hardware wallet shows with its passphrase and without it, so
  that a passphrase typed wrong (or a wallet set up without one) shows
  as a fingerprint that differs. A key loaded here with a passphrase
  has the row where its words are kept (a BIP-39 key).
- **Path**: the account's, `m/84h/0h/0h` (DESIGN.md §4.6).
- **Script**: in DESIGN.md §4.6's words (Native SegWit, Taproot, Nested
  SegWit, Legacy), "Multisig · Native SegWit" for BIP-48's `2h` and
  "Multisig · Nested SegWit" for its `1h`.
- **Xpub**: the seed's at that path, whole, its origin
  `[73c5da0a/84h/0h/0h]` before it. Where SLIP-132 names a spelling for
  the script (a zpub for BIP-84, a ypub for BIP-49, a Zpub and a Ypub for
  BIP-48), a segmented button over the row shows it or the plain xpub,
  SLIP-132's first, since wallet apps show one or the other; BIP-86 and
  BIP-44 have the xpub alone. It opens its QR sheet.
- **First receive address** and **First change address**: address 0 of
  the account's receive chain and of its change chain; whole, in groups
  of four; the receive address opens its QR sheet, the change address
  nothing. A multisig account has none: its addresses are the whole
  wallet's, made from every cosigner's key, and are checked on the
  wallet's own page once it is loaded.

Every string is shown whole (DESIGN.md §4.5), since a check by eye of
the first and last characters is the check an attacker's address is made
to pass.

**Compare with the wallet app's** (decided here: kept, small). A text
button on the Result, for a person who has a file or a QR code at hand.
It opens one outlined text field (typing and a paste) with **Scan** and
**From Files** beside it, and checks what is given against the account
picked:

- an extended public key in any spelling, with its origin: its public key
  and chain code against the Xpub row's, at the Xpub row's path. A key
  whose origin is at another path is not this account's: "Does not match",
  with the key's path named. A bare key, with no origin, is refused
  ("its key has no fingerprint and derivation path"), as everywhere;
- an address of the network in use: among the first 100 addresses of
  the account's receive and change chains, found where it is ("Receive
  address 7 · m/84h/0h/0h/0/7"); deeper is Explore's Find an address;
- a descriptor, a BIP-388 policy or a wallet file, read by Files' reader
  (`faraday/faraday-intake/src/wallet.rs`, `read_wallet`): matched at
  its own origin paths, as Restore matches a seed to a described wallet
  (`faraday/faraday-core/src/restore.rs`, `restore_refusal`), public key
  and chain code compared, and the key's path named where it is not the one
  picked ("This seed's key is at m/84h/0h/1h"). For a multisig this is the whole wallet's answer, which the
  Result alone cannot give.

It answers with one result line: "Matches" in `OK` ("This seed is key 2
of this wallet · 2 of 3" for a multisig's), or "Does not match" in
`ERR`, with one line under it where one says more: "It matches without
the passphrase" (the same seed with no passphrase makes it; a passphrase
not given is never searched for), or "The fingerprint is this seed's;
the xpub at {path} is not" with the warning glyph (an origin naming this
seed whose key is not, so the file is wrong or was changed). An
extended private key, a seed, or a file holding a seed or a private key, is
refused ("A private key: not taken here") when Compare or Enter is
pressed, or as it is scanned or taken from Files, and the field is
emptied; a value over 16 KiB is refused the same way with its size ("Over
16.4 KB: not taken here"); a value of
another network, or one none of the readers takes, is refused with the
reader's own line. Why kept: a person with the wallet app's file or QR
code gets a match without reading 111 characters, and a multisig's
wallet is checked whole; why small: it is never needed for a result, so
it is one text button, one field and one line, with no search across
accounts.

The Result's actions: **Another account** and **Another seed** (text
buttons; Another seed drops the key and opens Seed), **Compare with the
wallet app's** (a text button), **Keep this key** (outlined) and
**Done** (filled).

**Drop the seed or keep it** (decided here; the owner's call,
2026-11-14). A seed typed or scanned here is held in `faraday-keys` for
the check alone, as the Sweep sheet holds its private key: not among the
session's keys, not in the sidebar, used only for the derivations above
through public reads. At the end the person chooses:

- **Done** drops it, wiped as any key is (`docs/KEYS.md`), and goes back
  where the flow was opened (Check or Tools). So do Another seed,
  Cancel, leaving the screen, the idle lock and a lock: only Keep this
  key keeps it.
- **Keep this key** loads it into the session's keys as Add a key loads
  a typed key (`Keys::add_words_with`, its words and passphrase, the
  label Add a key gives it): it is then the same as a key loaded by Add a
  key (`docs/KEYS.md`), in the sidebar by its fingerprint with its
  "passphrase" mark, a signer for the session's wallets, its words
  behind their reveals, and dropped at a lock or power-off as every
  loaded key is. It leaves with Add a key's toast "Key {fingerprint}
  added" and goes back where the flow was opened. Add a key's refusals
  hold: dimmed, with "Remove the stick first", while a stick is in; a key
  already loaded is not loaded twice; Practice keeps only a sample key.

Why the choice is the person's: the seed is often the hardware wallet's
own, which the person came to check and not to keep, and a key kept
without being asked for is a secret left on the device and a signer in
the session; yet a person setting up a wallet on Faraday often wants the
key next, and typing the words a second time in Add a key is a second
chance to get one wrong. Done is the filled action, so the default ends
with nothing kept. A key picked under **A key loaded here** was loaded
before: the Result has no Keep this key, and Done leaves it as it was.
Practice takes only a sample key here, as Add a key does
(`Keys::is_public`), and refuses any other with Add a key's line.

What the flow keeps is public (the fingerprints, the xpub and addresses
worked out, a value given to compare) until it is left or a lock. It
keeps nothing once its screen is left, so it has no row in Flows in
progress. The derivations take `c: &Compute` and run on the press that
picks the account (or completes a card), and a compared address's search
ticks as Explore's Find an address does
(`faraday/faraday-core/src/explore/addresses.rs`), so drawing only reads.

**What it is built from**, so that nothing is written twice:

- the Seed card's words: Add a key's BIP-39 form
  (`faraday/faraday-core/src/screens/entry.rs`,
  `faraday/faraday-core/src/forms.rs`,
  `faraday/faraday-core/src/word_entry.rs`), and its Scan a SeedQR with
  decision 52's sheet (`faraday/faraday-core/src/qr_view.rs`,
  `ScanPurpose::Seed`);
- the passphrase: New key's Key card's two fields;
- the holding of the key for the check alone: the Sweep sheet's way in
  `faraday-keys` (its own slot beside the session's keys, wiped on Done
  and on a lock); Keep this key: Add a key's load
  (`Keys::add_words_with`, `Faraday::typed_load`);
- the account's xpub and addresses: `faraday-keys`' extended public key
  at a path (`faraday/faraday-keys/src/operations.rs`, `xpub_at`) and
  Explore's addresses at a path;
- the account rows, the path field and the script buttons: Explore's
  (`faraday/faraday-core/src/explore/mod.rs`, `PRESETS`);
- an xpub's spellings: Convert an xpub's (`core/faraday-bip/src/slip132.rs`:
  `decode_xpub`, `key_facts`, `encode_xpub`);
- Compare with the wallet app's: Files' reader (`read_wallet`) and the
  wallet's addresses (`faraday_bip::policy::WalletPolicy`, `address_at`),
  Restore's origin match (`restore_refusal`), Explore's address search
  (`explore/addresses.rs`), and the reader Check an address's scan goes
  through (`faraday/faraday-core/src/scan_read.rs`, `Kind::Address`).

**Tested against** published vectors and an implementation Faraday did
not write (`docs/STANDARDS.md` §0): BIP-84's, BIP-86's and BIP-49's test
accounts (`tools/vectors/addresses/`), whose published account xpub
(and SLIP-132 spelling) and first receive and change addresses are the
Result's rows; BIP-44, which publishes no vectors, against Bitcoin
Core's xpub and addresses for the same account; BIP-48's cosigner xpub
against btclib and btcd; BIP-39's English vectors under `TREZOR` and the
Japanese case (`tools/vectors/bip39/`) for the fingerprint with a
passphrase and without it; BIP-32's test vectors 1 to 5
(`tools/vectors/bip32/`) for a path named by hand; and every derivation
against btclib and btcd, as the cross-check's `bip32` and `addr` areas
run them (`tools/crosscheck/area_bip32.py`,
`tools/crosscheck/area_addr.py`), and a SeedQR against its area
(`tools/crosscheck/area_seedqr.py`). Built: a test for each account row, for
Keep this key (the key loaded as Add a key loads it, refused with a
stick in) and for Done, Cancel and a lock (the key gone).
**Practice** (`docs/MODES.md` §6). Built: the passphrase fields offer
`faraday` (MODES §6.2), and a SeedQR is read through the read sheet. Not
built: the Seed card's word sample and its scanner's sample, and the
compare samples. The design: the Seed card hands Use a sample the
next word sample, as Add a key does, and its scanner that sample's
SeedQR; the passphrase fields offer `faraday` (MODES §6.2). With test
key 1 and **Native SegWit** picked, the Result shows sample Spending's
account xpub (a vpub, or the tpub) and its first receive and change
addresses (MODES §5.3), the same strings sample Spending's wallet shows
in Practice, so the check can be tried as it is meant, by holding two
screens side by side; Taproot shows sample Taproot's. Compare with the
wallet app's hands, in turn, Spending's descriptor, its xpub with its
origin, the same xpub bare, Spending's receive address 0, and a value of
another sample, so that a miss is seen too. Keep this key keeps the
sample key as Add a key does. Its ? opens Learn at Verifying and Tools.

## Check a multisig setup

**Faraday shows what a wallet of several keys is, and the person checks
it.** A person who has a multisig wallet's descriptor, a multisig setup
file or a BIP 129 record (from a coordinator, a hardware wallet's
export, a backup sheet's code) brings it in, and Faraday shows its
quorum, its descriptor checksum and whether the checksum holds, and
each cosigner's fingerprint, path and whole extended public key, the
same rows, in the same order, as the confirmation sheet every wallet of
more than one key comes in through (F254). With a key loaded here, or a
seed given to the check, it says which cosigner that key is. The person
holds the rows against every other signer's device and the coordinator.
Nothing is added to the session unless the person asks, and then only
through that sheet. The sheet is skipped for a wallet whose every key is
a key loaded here at its path, for a wallet of the person's own vault
and for Load samples (`docs/WALLETS.md` §3.5); several held at once get
one sheet each, "{i} of {n}", and Cancel drops only the wallet shown. Built. Its screen is
`docs/DESIGN.md` §5, A multisig setup.

| From | What opens it |
|---|---|
| Check | its row 7, **Check a multisig setup**: the Setup card open |
| Tools | the tile **Check a multisig setup** in Wallets, after **Restore an existing wallet**, its line "Its quorum, checksum and cosigners", tags BIP-380, BIP-388, BIP-129: the same. Tools stays the list of every flow; Check is a second way in |
| Files | a wallet file's row's ⋮, **Check this setup**, From the stick and For the stick alike, on a file Files reads as a wallet of more than one key; not offered on a file of one key or one that does not read as a wallet. That file is the setup, the Setup card closed on its name and shape with **Change**, and the Result (or the refusal line, where the file is refused) shown at once. Back from it lands on Files as it was left |

It opens with nothing needed first, no key and no wallet. The screen is
a step flow (`Screen::MultisigCheck`) of two cards, **Setup** and
**Result**. Restore an existing wallet and the Descriptor checksum tool
stay as they are: Restore still makes a wallet from seeds, and the tool
still takes any descriptor typed; this check is where a setup of several
keys is read and shown whole without making anything.

1. **Setup.** The ways in, a row each:
   - **Scan**: the scanner titled "Scan the wallet's setup", taking a
     descriptor, a BIP-388 policy or a setup file as a static or
     animated code, `ur:crypto-output` included (`docs/QR.md`); a
     code read here goes to this card, named "Scanned", and is not put
     in Files;
   - **From Files**: the files Files reads as a wallet (Files' reader,
     `read_wallet`), a row each by name with the shape it reads as,
     newest first; files of one key are listed dimmed with "One key",
     and files that do not read are not listed;
   - **A wallet loaded here**, where a wallet of several keys is
     loaded: its wallets by name and shape, to be checked again against
     a device (a loaded wallet was confirmed when it came in);
   - **Paste**, on Faraday Demo only, as the scanner's Paste: the text
     goes through the pasted-text classifier (`docs/SCANNING.md` §4.6)
     and, where it is a wallet description, to the same reader.

   A setup read closes the card on its name and shape with **Change**,
   and works out the Result on that press.
2. **Result.** The confirmation sheet's rows, then the page's own rows
   and actions (below).

**What each kind of input does.** Every way in reaches one reader,
Files' `read_wallet` (`faraday/faraday-intake/src/wallet.rs`), so the
check reads exactly what loading would read:

- **A descriptor** (`wsh(sortedmulti(…))`, `sh(wsh(…))`, `sh(…)`,
  `tr(…)` with `sortedmulti_a`, `multi_a` or a tree, `musig(…)`, a
  miniscript) with its checksum or without: read whole and kept as it
  arrived, never written out again from its keys (`docs/WALLETS.md`
  §3.2).
- **A BIP-388 policy**: its template and keys, read as a descriptor.
- **A multisig setup file** (Coldcard's, Sparrow's, Nunchuk's,
  Specter's, Caravan's, a Sparrow or Specter wallet `.json`): read as
  Files reads it; such a file carries no descriptor checksum, so the
  checksum row is worked out here and says so.
- **A BIP 129 record** (its descriptor record, `faraday_bip::bsms`):
  read whole, its first address checked against the wallet it states,
  as Files checks it; a record whose address is not the wallet's is
  refused. A BIP 129 signer's record (one key) is not a setup.
- **A loaded wallet**: its own descriptor, as kept on it.

**What it refuses**, each as one result line in `ERR` in the Setup
card's place, with no Result drawn (nothing of a refused setup is shown
as if it held), the line coming from the reader where it has one:

- **A file or text that does not read as a wallet**: the reader's own
  line ("Not a wallet: {why}", `wallet_not_a_wallet`, naming a JSON file too
  deep or too long, an encrypted Electrum wallet, a SLIP-132 key of
  another script than the file's).
- **A checksum that does not hold**: "The descriptor checksum does not
  hold: given {given}, the descriptor gives {worked out}". A checksum
  that fails says the text was mistyped or changed, so the keys it
  carries are not shown.
- **A key without its origin** (a bare xpub, or one with no fingerprint
  and derivation path): "A key of this wallet has no fingerprint and
  derivation path", Files' line. It stays refused: without its origin
  no key here or seed can be matched to it, and a guessed path is never
  taken.
- **A private key or a seed** (a descriptor holding an xprv or a WIF, a
  file with words): "A private key: not taken here", nothing of it kept
  or drawn, as My seed makes my wallet refuses it; a paste of one is
  refused as the scanner's Paste refuses it (`pasted_secret`) and
  cleared.
- **A wallet of one key**: "One key: not a multisig setup". My seed
  makes my wallet checks a single-key account.
- **A wallet of another network** than the one in use: the reader's
  line, as Files gives it.
- **A wallet whose keys repeat** is not refused, as Restore does not
  refuse it: each repeat has the warning line the Descriptor checksum
  tool gives ("Key 2 is the same as key 1", `docs/WALLETS.md` §7), above
  the cosigner rows.

**The Result.** First the rows of the confirmation sheet (F254), drawn
by the same code as the sheet's body, so that the person learns one
layout:

- the **shape** in the wallet's own words (`Session::shape`): "2 of 3 ·
  native SegWit multisig", "2 of 3 · Taproot multisig", "3 of 3 ·
  MuSig2", "4 keys · Taproot key and leaves", "3 keys · miniscript with
  conditions"; and the name, where the setup gives one;
- per cosigner, in the descriptor's order: **Key 1**, **Key 2** and on,
  its fingerprint, its path from its origin (`m/48h/0h/0h/2h`), and its
  extended public key whole, mono, in groups of four, wrapped and never
  cut, in the spelling the setup gives (an xpub, or the Zpub a setup
  file carries); "Yours" after the path where a key loaded here gives
  that same extended key (public key and chain code) at that path, and
  "Yours · your single-key account" where that path is a single-key
  account's (BIP 44, 49, 84, 86), as the sheet marks it;
- the **Descriptor checksum**, its eight characters, then "holds" in
  `OK` where the setup gave one and it held, or "worked out here, none
  given" for a setup file that carries none;
- for a `tr()` wallet with a script tree, the key path line
  (`docs/WALLETS.md` §3.4): "Key path: none (unspendable)", "Key path:
  {key} alone", or the danger line for a key not of the wallet, in the
  error colour.

Then the page's own rows:

- **Quorum**, where the shape does not state one, as the wallet's
  quorum is worked out everywhere (`Session::quorum`, from
  `Session::spend_ways`): a script that states m of n is that; a MuSig2
  wallet is n of n, every key; a script tree or a miniscript reads "{m}
  of {n} seeds", m the fewest seeds of any way to spend and n the
  distinct seeds, and every seed where the ways cannot be worked out.
  Such a wallet is never read as 1 of n.
- **Ways to spend**, for a script tree or a miniscript: one line per way
  the script can be satisfied (`WalletPolicy::spend_paths`), the keys it
  needs by number and its wait, as the wallet card's Spend paths read
  ("Key 1 and key 2 · any time", "Key 3 · 52 560 blocks after each coin
  arrives · about 365 days"), the key path first for a `tr()` wallet. A
  script with more ways than the list holds reads "More ways to spend
  than are listed here" and lists none. A wallet that states its quorum
  in its script has no such row.
- **First receive address**: address 0 of the wallet's receive chain,
  whole, opening its QR sheet; for a BIP 129 record, with " · as the
  record states". It is the one address every coordinator shows when a
  wallet is set up.

**Which cosigner a key is** (its origin matching). For every key loaded
here, its extended key is worked out at each cosigner's own origin path
whose fingerprint is the key's, and public key and chain code compared,
as Restore matches a seed to a described wallet (`restore_refusal`) and
as the register sheet marks "Yours". A match marks that cosigner's row
"Yours". A fingerprint that is a loaded key's whose extended key at that
path is not marks the row with the warning glyph and "The fingerprint
is {fp}'s; the key at {path} is not": the setup is wrong or was
changed. A key matching no cosigner is not named.

**Check a seed against it** (a text button on the Result), for a seed
not loaded here, such as a hardware wallet's own: the Seed and
Passphrase steps of My seed makes my wallet open under the rows (**Type
the words**, **Scan a SeedQR**; **No passphrase** or the passphrase
typed twice), and the seed is matched as a loaded key is. Its answer is
one result line: "This seed is key 2 of this wallet" in `OK`, that row
marked "This seed" (a seed that is several cosigners reads "This seed is
keys 1, 3 of this wallet", each row marked); "Not a key of this wallet: check the words or the
passphrase" in `ERR`, with "It matches without the passphrase" under it
where the same words with no passphrase make a cosigner (a passphrase
not given is never searched for), or the warning line above where the
fingerprint is a cosigner's and its key is not (under the key line, where
the seed is another cosigner's key). The seed is held in
`faraday-keys` for the check alone, as My seed makes my wallet holds
it, and dropped, wiped as any key is (`docs/KEYS.md`), on **Done**,
**Another setup**, **Check a seed against it** pressed again, the Result
card closed or the Setup card opened, leaving the screen, the idle lock and a lock; it is never loaded from here, so
the Result has no Keep this key (Add a key loads a seed). Decisions 8 and
9 hold: with a stick in, Type the words and Scan a SeedQR open the sheet
that asks for it to be pulled.

**Add this wallet** (decided here: offered, through the sheet). A person
checking a setup they are about to use often wants it loaded next, so
the Result offers **Add this wallet** (filled) while the wallet is not
loaded. It never adds the wallet itself: it opens the confirmation sheet
(F254) over the page, with its rows, its compare line and **Hold to add
this wallet**, and only the hold adds it, as every wallet of more than
one key comes in; Cancel on the sheet leaves the page as it was. Once
added, the page says "Loaded here as {name}" and the filled button is
**Open the wallet**, which opens it in Wallets; a wallet already loaded
(the same descriptor checksum) reads so from the start.
The page leaves out the sheet's compare line: comparing is what the page
is for.

The Result's actions: **Another setup** (a text button, opening Setup),
**Check a seed against it** (a text button), the next step, filled
(**Add this wallet**, then **Open the wallet**), and **Done** (a text
button), which goes back where the flow was opened (Check or Tools).

What the flow keeps is public (the setup read, the rows worked out)
until it is left or a lock, but for a seed given to Check a seed against
it, which is a secret held as above. It keeps nothing once its screen is
left, so it has no row in Flows in progress. The reading, the
derivations for "Yours" and the seed's match take `c: &Compute` and run
on the press that completes a card or gives the seed, so drawing only
reads.

**What it is built from**, so that nothing is written twice:

- the reader: Files' `read_wallet` and the pasted-text classifier
  (`faraday/faraday-intake/src/wallet.rs`,
  `core/faraday-codec/src/classify.rs`), BIP 129's
  `faraday_bip::bsms::DescriptorRecord`;
- the checksum: `faraday_bip::descriptor`'s `verify_checksum` and
  `checksum_facts`, as the Descriptor checksum tool reads it, and its
  repeated-key lines;
- the top rows: the confirmation sheet's body (F254), one piece of code
  for the sheet and the page;
- the quorum and the ways to spend: `Session::shape`, `Session::quorum`
  and `Session::spend_ways` (`faraday/faraday-core/src/wallet.rs`),
  `WalletPolicy::spend_paths`, the key path line of `docs/WALLETS.md`
  §3.4, read from the wallet as read, without adding it to the session;
- "Yours" and the seed's match: Compare's judgement of a wallet's keys
  (`faraday/faraday-core/src/seed_check/compare.rs`, `judge_all` for every
  cosigner and `held_in` for the seed the check holds) and `faraday-keys`'
  `xpub_at`; the seed's steps and its holding: My seed makes my wallet's;
- the first receive address: `WalletPolicy::address_at`.

**Tested against** published vectors and an implementation Faraday did
not write (`docs/STANDARDS.md` §0): BIP 380's checksum vectors, each
valid one read as holding and each changed one refused, and every
checksum shown against Bitcoin Core's `descriptors.py` (the
cross-check's `addr` area); BIP 129's test vectors
(`tools/vectors/bip129/`), its descriptor record read with its first
address; BIP 388's policies; the first receive address of each kind
against Bitcoin Core's `deriveaddresses`; the other wallets' setup
files Files' tests already read (`docs/WALLETS.md` §8). A test for each
refusal (a text that does not read, a checksum that fails, a key without
its origin, a private key, one key), for the quorum of a sorted
multisig, a Taproot multisig, a MuSig2 wallet (n of n), a script tree
and a miniscript (never 1 of n), for "Yours" and for the warning line
of a fingerprint whose key is not, for Check a seed against it (a match,
a miss, "It matches without the passphrase", the seed gone after Done
and a lock), and for Add this wallet reaching the session only after
the sheet's hold.

**Practice** (`docs/MODES.md` §6; when built, its §5.3 and §6.1 list
these). The Setup card's scanner and host strip hand, in turn, as a
scan of each: sample Savings' descriptor (2 of 3 · native SegWit
multisig over test keys 1, 2 and 3, its checksum holding); Savings as
a BIP 129 descriptor record, so the record's first address line shows;
sample MuSig2's descriptor (3 of 3 · MuSig2); sample Taproot tree's
descriptor, so the Quorum, Ways to spend and key path rows show; and
Savings' descriptor with one character of its second key changed and
its checksum kept, so the refusal "The descriptor checksum does not
hold" is seen. With test key 1 loaded (Load samples), Savings' key 1
row reads "Yours". Check a seed against it hands test key 2's words
(bacon's, zebra's and summer's are test keys 1, 2 and 3), so the answer
reads "This seed is key 2 of this wallet", and next a word sample of no
sample wallet, so the miss is seen; the passphrase fields offer
`faraday` (MODES §6.2), which no sample cosigner uses, so the line "It
matches without the passphrase" is seen too. Add this wallet adds the
sample through the sheet's hold, as a real one is added. The samples
are literal text in `faraday-samples` (MODES §5.4: it parses nothing),
and its test checks that each equals what the testkit makes
(`testkit::kits`, `testkit::musig_policy`), that the BIP 129 record
reads to Savings, and that the changed one's checksum fails. Its ?
opens Learn at Verifying and Tools.

**Its strings** (`faraday/faraday-strings/src/en/`, when built; the
confirmation sheet's own are `wallet_confirm_*`, F254, and the reader's,
Restore's and My seed makes my wallet's are reused):

- `check_row_multisig`: "A multisig setup" (now `mscheck_title`); `check_row_multisig_line`:
  "The wallet's descriptor or setup file" (Check's row 7, F255);
- `catalog_check_multisig`: "Check a multisig setup";
  `catalog_check_multisig_line`: "Its quorum, checksum and cosigners";
- `mscheck_title`: "Check a multisig setup";
- `files_check_setup`: "Check this setup" (the file row's ⋮ entry);
- `mscheck_setup`: "Setup"; `mscheck_result`: "Result";
- `mscheck_scan`: "Scan"; `mscheck_scan_title`: "Scan the wallet's
  setup"; `mscheck_from_files`: "From Files"; `mscheck_loaded`: "A
  wallet loaded here"; `mscheck_paste`: "Paste"; `mscheck_scanned`:
  "Scanned";
- `mscheck_one_key`: "One key"; `mscheck_refused_one_key`: "One key:
  not a multisig setup";
- `mscheck_checksum_fails`: "The descriptor checksum does not hold:
  given {}, the descriptor gives {}";
- `mscheck_checksum_holds`: "holds"; `mscheck_checksum_worked_out`:
  "worked out here, none given";
- `mscheck_quorum`: "Quorum"; `mscheck_quorum_seeds`: "{} of {} seeds";
- `mscheck_ways`: "Ways to spend"; `mscheck_ways_too_many`: "More ways
  to spend than are listed here";
- `mscheck_first_receive`: "First receive address";
  `mscheck_as_record_states`: " · as the record states";
- `mscheck_key_n`: "Key {}";
- `mscheck_key_not`: "The fingerprint is {}'s; the key at {} is not";
- `mscheck_check_seed`: "Check a seed against it";
  `mscheck_seed_is_key`: "This seed is key {} of this wallet";
  `mscheck_this_seed`: "This seed";
- `mscheck_another`: "Another setup"; `mscheck_add`: "Add this wallet";
  `mscheck_loaded_as`: "Loaded here as {}"; `mscheck_done`: "Done".
