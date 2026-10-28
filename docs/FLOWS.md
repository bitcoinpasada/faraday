# Flows

**Kind:** Spec
**Status:** Built but for the items under Open; the code is the record where it differs
**Open:** files over 64 MiB on a stick visit (`docs/LARGE-FILES.md`), the Nostr key (`docs/NOSTR.md`), public values and the TOTP secret as codes (`docs/QR.md` §7); backups age, each copy's last check and a recovery path's wait (`docs/BACKUP-FLOW.md` §18, planned)

Every flow, where it starts, and where it leaves the person.

The rule this file checks: **no flow ends on a dead end.** Each one
finishes on a screen that shows what happened and offers the next thing a
person would do. Where a flow cannot go on (no vault open, no file in
Files, a stick still attached), it says what is missing and offers the way
to get it.

`docs/DECISIONS.md` holds the decisions; `docs/VAULT.md` the vault format;
`docs/QR.md` the scanner.

## Decisions this pass made

Moved to `docs/history/FLOWS-LOG.md`. Each decision still in force is
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
SD card", "For the SD card" on the Pi), on Files, the stick visit, the
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
camera closes and the sheet **SeedQR read** shows the key it makes
**Without a passphrase**, with **Use without a passphrase**, **Add a
passphrase** and Cancel. Add a passphrase shows **Passphrase** and
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
card without one. Make a new wallet opens on Kind, open, with its
default ticked (Single key, backup in shares where Make codex32 shares is
offered, else Single key; owner, 2026-10-24); a Tools tile that ticks a
kind opens the card after it. New key opens on its first step, whatever
opened it.
A card that asks one thing (a kind, Yes
or No, a format, a size) closes on the choice and opens the next, and a
press on the option already chosen does too (F103); **Continue** stays
under a card of several values (steppers, ticks, typing) and where the
press is the act.

**Back within a flow** (F104). In Restore, Sign a transaction, the Spend
tab, Sign a message, Create a vault, BIP-85, Silent payments and Vanity,
Back opens the step before the open one with its choices as they were,
and the way back is named after it; Back from the first step leaves the
flow as F77 says. A running step (a vault being made, a search under
way) takes no Back. Once a wallet is made from its seeds, or a key here
has signed, or a MuSig2 round is open here, Back goes back no further
than the step after it. Escape is Back on BIP-85 and Vanity.

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

**Learn** follows Tools (decision 13); the small panel's Home grid
carries it. The sidebar's status block carries the session, Files and
the sticks; its **Files** entry opens Files. **The session strip**
(decision 15) is one line, **Bring in · Open · Work · Write out**, the
current stage in the accent, computed from what is attached, loaded and
waiting; tapping it opens Files. **Bring in** is a stick attached or
an import waiting, and also nothing attached, loaded or waiting at all
(owner, 2026-10-24); **Write out** is files For the stick, or a vault
changed since it was written (decision 26), with no stick attached;
**Work** is a key, wallet or open vault loaded; **Open** is files in the
Inbox with nothing loaded. The mode shows only as an off-mainnet pill that opens the
mode sheet (decisions 14 and 33; F81): no coloured bar along the top on
a test network; the sheet is also Settings' Mode row, the only door on
mainnet.

## Files

**One Files screen holds what came from the stick and what goes to it**
(decision 1). Every file copied in carries the action that uses it: a
PSBT **Sign in Wallets**, `BOOTX64.EFI` **Sign in Secure Boot**, an
entries file **Import into a vault**, a vault file **Unlock**, a received
file **Show as QR**. Both halves are emptied at power-off. Each half
scrolls on its own, with its own scrollbar, under the pointer (decision
55).

**For the stick fills itself** (decision 24). A public file a flow makes
goes For the stick the moment it is made: the signed PSBT and the
finished transaction at Finish, the PSBT with this pass's signatures
while more are needed, the backup checklist's template, sheet or shares
and public files when the checklist is made (made again when the word
count or the shares change), a GPG public key or signature (a revocation
certificate is a secret and goes through the secret sheet, F32), a
signed message, a sealed vault at lock. No button puts a file out: the
row reads "For the stick" with **Remove** in its ⋮, and after a Remove
the flow's button makes it again. The secret sheet stays the one way a
secret goes For the stick.

**A write leaves a receipt** (decision 25). When a stick visit's write is
answered, what it sent leaves For the stick and becomes the receipt,
shown under For the stick as "Written to {label} at {time}" with a row
per file, "verified" or why not. It is kept across a lock until
power-off and replaced by the next write; nothing on it is written
again. A vault written stays in Files as the stick now holds it. With
nothing loaded, Home leads with "Written to {label}" and the file names,
opening Files; a backup checklist item counts a file on the receipt as
done.

### A ⋮ on every file; Rename (decision 61, F54)

Every file row, in Files and on the stick visit, keeps the one action it
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
   PDF** (For the stick), **Rename** (For the stick only), **Details**,
   **Remove**. There is no visible **Remove** button. A row with no main
   action shows only the ⋮. A vault action (Add to vault, Import, Sign
   with key) is not shown, as a button or in the ⋮, while no vault is
   open; the vault's own row keeps its Unlock (F60).
2. **Rename, For the stick.** A popup, "Rename {name}": a field holding
   the name before its extension, the extension shown fixed after it
   (`.ofv` for a vault; every file keeps its own), and **Rename** and
   **Cancel**. Refused, with a line in the popup and Rename disabled: an
   empty name, one starting with a dot, `/`, `\`, `:`, `*`, `?`, `"`,
   `<`, `>`, `|` or a control character, a name over 255 characters with
   its extension, and a name another file For the stick has, ignoring
   case. Spaces at the ends are dropped; the name unchanged leaves Rename
   disabled with no line. The new name is the one the next visit writes
   it under; the visit's **Already on the stick** sheet then works as
   usual. A vault For the stick whose same vault (by identity,
   `docs/VAULT.md` §6) is on the stick under another name is written over
   that file and the file takes the new name (or `name-2.ofv` where
   another file has it); otherwise a vault written over its same vault
   keeps the name the stick has for it. Renaming a vault's file also
   renames the file the open vault seals back into. Renaming a vault's
   file never changes the vault's own name (the open vault's
   **Rename**), nor the reverse.
3. **From the stick has no Rename**: its files are Faraday's copies of
   files on a stick, renamed on the stick itself (item 4).
4. **On the stick (the visit page).** Every file row Faraday lists has a
   ⋮ with **Rename**, **Delete from the stick** and **Details**; the
   computer's own files (decision 54) and the settings file have none.
   **Rename** is the same popup, renaming the file on the stick in place:
   the disk process renames its directory entry, lists the stick again,
   and the visit shows "{old} renamed {new}", or why not. Refused, nothing
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
   file it was written as; a copy renamed on the visit is named anew from
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
writing until the person ticks it on the visit. A secret let out this
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
(§10.1). Not built: **Make the rest** and **Add a share** (§8.1 step 2,
§8.2), a checksum worksheet's final row on Check a string, and the
mark's line beside the fingerprint on Wallets and New key's Key card.

**Add a key is New key, opening on Start** (decision 60, F44). New key's
first card is **Start**: under **Make a new key**, **BIP-39 words**
(ticked) and **Codex32**; under **Add a key you have**, **Type the
words**, **Scan a SeedQR** and **Type codex32 strings**; and under
**Other forms** (closed) **Make SLIP-39 shares**, **Type SLIP-39
shares**, **Type Seed XOR parts**; closed, it shows the choice ("Make a
new key · Codex32") and **Change** until the key is locked in. Codex32
under Make a new key is offered only where a single-key wallet is made:
on its own, and in a single-key Create's slot (`docs/CODEX32.md` §6.9). A key typed is Start · **Words** · Key, one scanned
Start · Key: Words is Add a key's form drawn by its own code (count,
boxes or keyboard, lists; the other forms' fields with **Add this one**
and **Scan a QR code**), whose Continue opens Key once the words are
whole with a checksum that holds or the parts are enough; Key shows the
words behind **Show words**, **Passphrase** and **Passphrase again**
(none for codex32), and **Lock in**, which loads the key as Add a key
does (not marked made here) and fills a Create slot, and leaves the
flow by itself, with no Words or Quiz. Add a key on Wallets and its
Tools tile (one tile for making and for typing a key, owner,
2026-10-24) open with Start closed on Make a new key, and so does New
key from a Create slot (F103): Length is open, and
**Change** on Start reopens it, where a press on a choice, the ticked
one included, moves on by itself. The SLIP-39, codex32 and Seed XOR tiles open with their
choice made and Start closed. The page is titled **Add a key**, and
**New key** for a Create slot and from the Add a key screen's Make a new
key, which opens with Start closed on it. The Add a key screen (Home,
below) stays for a key asked for by name: a slot's fingerprint, a
wallet's missing key, Restore's seed slots, and a seed scanned from Home
or the Spend tab. New key keeps nothing when its screen is left, so it
has no sidebar row (Flows in progress).

**Make a new key** (decisions 34 and 38, F103). Length opens
first, its default count chosen; pressing a count takes it and opens
Randomness, and pressing a way opens the entry card, with no Continue
(a mix keeps its Continue: its sources are ticked first; SLIP-39's
Length keeps its Continue: its steppers are values). New key's BIP-39 cards are Length · Randomness ·
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
(DESIGN §4, Dice roller): **Rolled by this computer** over it, **Roll**
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
stepper k to 31 · None, the secret alone; for k of 4 or more a line that
the booklet publishes no translation sheet for it), **Identifier** (four
characters typed on the codex32 keyboard, or **Suggest**; one used by
the strings typed in Add a key is refused), **Entropy** (Roll dice ·
Type the characters, the latter with the line "Characters from dice or
a worksheet, not chosen"), the entry, titled "Share A · character 7 of
26", **Worksheets**, **Strings** and **Strings made**. The entry shows
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
**Vaults**, which with no vault in Files or open reads **Create a
vault** and opens it; then **Stick visit** while a stick is attached
and nothing is held, and **Backups** once a wallet is known. Create,
Restore, the Spend tab and Create a vault opened from Home have **Home**
as their way back, and from Wallets, **Wallets**.

**Add a key** (make, type or scan a seed) is on Wallets and in Tools →
New key on its Start card (decision 60). The Add a key screen, for a key
asked for by name and a seed scanned from Home: **Scan a SeedQR** first, full width over the words (primary until a word is
typed, when **Add key** is), a form (BIP-39 words, with the count
**12 · 15 · 18 · 21 · 24** above as many boxes, 12 first; SLIP-39
shares, codex32, Seed XOR parts), **Make a new key**, and "From a stick ·
plug it in" (with a stick attached, **Copy files in** → Stick visit; a key
file copied in loads when the stick is pulled). **Scan** floats in the
bottom-right corner of Home on every panel → the scanner (with a stick
in, the sheet asks for it to be pulled first). Home's Scan takes a seed
too: a SeedQR, a CompactSeedQR or a seed's words go to Add a key and are
added as **Scan a SeedQR** adds them, never into Files; a
private key or xprv is named, since Add a key does not take one. The
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
  other, read from the stick, a file over 18 MiB failing its read (F26,
  specified, not built: over 64 MiB, counted and left for a stick
  visit), a PNG as what its QR codes hold; the sheet lists them, "3 of 5
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
  loaded from vault.ofv" (decision 43; Home when no wallet loaded); a
  wrong passphrase → stays, says so; **Cancel** → the vault stays in
  Files, locked → the next vault, or Home;
- **Later**, or a tap beside the sheet → Home, nothing read, nothing
  brought in; Home leads with **Import from {label}** (after Import,
  **Unlock {name}**), which opens the sheet again; with the stick still
  in, the stick visit brings in what it ticks;
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
Unlock", its buttons **Back to the vault** and **Lock**, or **Lock
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
file a lock drops (`docs/history/PLAN.md` §5.3); its Then reads "Lock,
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
Faraday Demo; a Mac as `docs/MAC.md` §3.2. The sheet also offers
**Restart** (below, Restart; F42).

## Sticks and cards

On the Pi every medium is an SD card and every screen says so: "SD card"
for "stick", **SD card visit** for **Stick visit**, **Lock and use SD
card**, "Remove the SD card" (`docs/DECISIONS.md` F4). The flows
below use the PC's words.

**A stick or card inserted while unlocked** → **Lock to use this stick**
(what is kept, then what is wiped from memory and where else it is,
then what happens; decision 28) →
- **Lock and use stick** → lock → **Stick visit**;
- **Not now** → nothing is handed out; a banner says "Remove the stick to
  keep working" until it goes.

**A stick or card inserted while locked, or with no vault open** → **Stick
visit** directly.

**Stick visit** (one screen, both directions):
- **In Faraday** (left): chosen files written, read back, compared:
  those For the stick, then those from it under **From the stick** (below).
  What is written and matched leaves For the stick for the receipt
  (decision 25). When a file to write has a name the stick already
  holds (ignoring case), the sheet **Already on the stick** comes first,
  a row each, with **Replace** (written over the old file: the new bytes
  under a hidden name, read back, then the old file removed and the new
  one renamed to its name), **Keep both** (written beside it as
  `name-2.ext`; the row says "{name} · Keep both: {name-2}") and
  **Cancel** (nothing written, the visit stays). A vault of the same
  length under its own name is listed "{name} · the same vault, updated
  here" and is always replaced; a vault under another vault's name
  "{name} · another vault · kept, this one as {name-2}", whatever is
  chosen. Those two alone bring no sheet, nor does the settings file. A sealed vault replaces the file with its identity, or is written
  under its own name (`name.ofv`, `name-2.ofv` when the stick has one;
  `docs/VAULT.md` §6) where there is none. A failed comparison
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
  the visit.
- **From the stick**: every file copied in, unticked, written from Files,
  where it stays for the next stick: a copy from one stick to another,
  one after the other or both in at once. A file the stick shown holds
  already (by name and size) is listed greyed, tagged **On this stick**
  (**On this SD card**), with a disabled checkbox that cannot be ticked:
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
  (F26, `docs/LARGE-FILES.md`; specified, not built.) A file over 64 MiB
  is listed unticked with its size; ticking it opens the sheet of
  §3 there: **Import the whole file** (with Size, Free after importing
  and what the vault needs; refused where it would leave too little to
  open one) or **Import only what signing needs** (a signing-only entry
  in Files: Sign a file, Delete, nothing else). A
  long read shows its progress and **Stop**. A pending signed file (§7
  there) is a row For the stick, ticked when this stick holds its
  source, else "Needs {source name} on this stick"; it is written with
  progress and **Stop**, and "{source name} changed since it was read.
  Nothing written." where a part differs.
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
  the visit ("{file} deleted, zeros over it first", or why not). The free
  space is zeroed in steps of about 8 MB, each asked for on the tick
  after the last one's answer, read back, the first step also clearing
  deleted directory entries and kept files' last clusters past their
  ends. The whole partition is first made an empty volume with its
  layout and label, then zeroed the same way, once per pass. A sheet
  shows the share done and the time left, with **Stop**; Stop, a failure
  or a pull ends it with a line on the visit. A whole-device wipe is not
  offered: it would need the whole device, outside the stick process's
  boundary. A computer's own files (dot-files and Windows' own,
  `docs/STARTER.md` §4.4) are never offered or deleted one by one, and
  the boot partition is never touched.
- The title's chip says where pulling the stick goes (decision 30):
  "Then: Unlock" when a vault was sealed for it (For the stick, or on the
  last write's receipt), "Then: {flow}" when the stick came in during
  one, else "Then: Home" →
  **Remove the stick** → once gone → **Unlock**, with the vault sealed for
  the visit, or the flow it came in during, or Home, which leads with the
  receipt.

**A stick pulled during a write** → the write is reported failed, the file
stays For the stick, and the visit screen says to put the stick back.

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
lead with "Write {name} to a stick" and the session strip read Write
out.

A locked vault says what it held (decision 23, `docs/VAULT.md` §11): what
it held when last seen open is kept across a lock until power-off; its
row goes by its name and Home's lead is "Unlock {name}", both with the
line "Savings 2 of 3 · key 9a6a2580 · 12 entries · seen 14:02"; with
nothing remembered, "Unlock to see what it holds".

- open vault → **Open** → Vault contents;
- locked vault → **Unlock** → passphrase sheet → Files, with **Load
  from {vault}** over it (F62): **Load ticked** → Wallets on the first
  wallet loaded that can sign here (Files when only keys loaded); **Not
  now** → Files, the vault open, nothing loaded. No sheet when the vault
  holds no wallet or key, or all are loaded already.
- opened from a flow that needs a vault (decision 10): **Unlock** →
  the flow, on the step it was on; the way back returns to it.

**Create a vault** (step cards: Size, then Name and passphrases;
**Customise** puts where it opens, unlock cost and space per passphrase
in Size's place) → **Create vault** → Unlock with the new
vault picked, "Type the passphrase once more to open it", sealed copy
For the stick → **Unlock** → Vault contents, "Nothing in it yet", a
button per add action and **Write it to a stick** (Files) (decision
22). Opened from a flow that needs a vault (decision 10), it shows
"Then: {flow}" by its title, and the unlock goes back to the flow.

**Generate with dice** (a passphrase's card on Name and passphrases): the
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
Name and passphrases; a size pressed moves on to it, as a preset cost
and a space per passphrase do on Customise's cards (F103), Custom's
memory and passes keeping their Continue. The Name field's caption says the name is what it
holds, for anyone who sees the stick. The size rows, the cost card and
the summary panel state one memory figure, the vault's unlock memory
(the Argon2id memory); the panel's row is **Unlock memory**. What this
computer has free is shown only where it falls short. Customise's Where
card has **This computer** ticked by default and untickable, for a vault
that asks less than this computer can give; its row reads the most
unlock memory it gives a vault (the largest custom memory that fits in
its free memory beside the app's 100 MiB, at most 4 GiB) with its free
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
- **Add an entry** → form (title, username, password with **Generate with
  dice**, URL, notes, TOTP secret typed or **Scan**) → saved → the entry
  selected in the list; vault marked unsaved.
- Entries list → **TOTP codes** → the codes screen (below).
- An entry with a TOTP secret → **Show as QR** under **TOTP
  secret** → the QR sheet with the stored `otpauth://` URI, marked
  Secret, no PNG, nothing For the stick; **Done** closes it (F28; not
  built, `docs/QR.md` §7).
- **Import entries** (from a file in Files) → preview: entries found, lines
  skipped with the reason → choose the vault → import → Entries list.
- **Edit** → the same form → back to the item.
- **Hold to delete from vault** → gone; the next item selected.
- A Bitcoin key or wallet → **Open in Wallets** or **Don't load at unlock** / **Load at unlock** (F62: a wallet's, and a key's in no wallet of the vault; a key in one reads "Loads with {wallet}").
- A GPG key → **Export public key** with **Show as QR** and **PNG** on its
  row; **Export the kept revocation** and **Make a revocation** (the
  secret sheet, no code or picture); **Sign a file**, whose list shows
  each file's SHA-256 and a small text file's text, with **Show as QR** and
  **PNG** on each file's row, and **Sign**, whose sheet shows the files
  and the signature time and holds to sign (F63); **Renew** with a term
  chosen; **Hold to mark the key revoked** (F32).
- Secure Boot keys → **Open Secure Boot**.
- A recovery sheet → its descriptor, with **Show as QR** and **PNG** on
  a row under it (F28; not built, `docs/QR.md` §7).

**Change passphrases, cost or space** → **Make a new vault from this one**
→ Create vault, filled in from this vault, with this slot's contents
carried over; the form says that slots whose passphrases are not entered
are not carried (`docs/VAULT.md` §8) → new vault created → the old one
offered for **Remove from this session**, and the stick visit's Delete
files (its hold) for the stick.

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
  **Not checked**. A place that keeps only a sheet reads its file.
- the vault: the vault file holding the wallet and where it stands,
  "vault.ofv · for the stick", "vault.ofv · on STICK, 14:02",
  "vault.ofv · changed since written"; "Not in a vault" when none holds
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
vault.ofv · for the stick" (paper "1 of 2 checked" while one is not), or
"Not backed up"; a press opens Backups on that wallet.

## Backup flow

**Decide where a key will be kept, then make it** (decision 59;
`docs/BACKUP-FLOW.md`, F31). One flow makes and backs up a
GPG key, Secure Boot keys and a Bitcoin wallet's keys (a Nostr key's
part is not built, `docs/NOSTR.md`): where it will be kept (the plan) →
each stick the plan names brought in with nothing secret open, checked,
its vaults read, its place confirmed → existing vaults unlocked, new ones
made → the key made into every vault the plan keeps it in → one lock →
each stick written by place → done, unlock. The plan decides what each
stick holds; only places with a stick are asked for. Two places or more
for GPG and Secure Boot keys, whose signing waits until every place
is written; one or more for a wallet, with a warning when every secret
sits at one place; any for Nostr; no upper bound anywhere. The blank
template is printed from Faraday Starter. The stick visit page is never
shown, and after Back up's checklist nothing waits For the stick.

Tools' **GPG key** and **Secure Boot keys**, Vault
contents' Make actions, a Nostr key's **Save into a vault**, Make a new
wallet and Restore → step cards (pages on a small panel, "Step 2 of
7"; the steps a plan does not need are not drawn):

1. **Where it will be kept**: a row per place (**Add a place**), what
   each keeps (a stick with Vault 1, Vault 2, a new vault; for a
   wallet, the plan's vault part), a row per vault (new or existing),
   the check lines; at least two places for GPG and Secure Boot;
   for a wallet the one-place warning and, on a single key with no
   passphrase, the passphrase line with **Add a passphrase**;
2. **Bring in the sticks** (with anything secret open: **Lock and bring
   in the sticks** first) → "Insert the stick for Vault 1 (Place 1)", or
   **Use {label}, read at {time}** → usable, else why and what to do →
   its vaults: "Which vault is Vault 1?", or **A new vault beside them**
   → **This is the stick for …** (a tap; none after **Use {label}**,
   whose press names the stick), or the hold where two sticks read
   alike; a stick bound to another place refused → pull;
3. **Vaults**, once every stick is read (nothing is unlocked before):
   **Open {vault}** for each one found, then its passphrase and **Unlock
   {vault}**; **Create {vault}** for each new one; both offered where
   both apply, nothing unlocked by itself (F121). Where only Create
   applies (no vault found, none open), Create a vault opens by itself
   once the last stick is bound, after the Pull sheet "Pull the stick to
   create the vault, then your key" while a stick is in. **I do not
   have its passphrase** → a new vault beside it or another stick;
4. **Make the key**: the kind's own form → made into every vault the
   plan keeps it in; a wallet's New key into its vaults, a key kept
   only on paper shown for copying; then, for a wallet, the rest of the
   plan and the checklist;
5. **Lock**: Kept and Wiped from memory → **Lock and write**;
6. **Write the sticks**: "Insert the stick for Vault 1 (Place 1)" with
   what it will hold → a stick unlike the one read: "Not the stick read
   for …"; a vault's second copy: the first stick refused, then **Hold:
   this is a second stick** (**another stick** for a third or later
   copy); a stick named after step 2 (the stick of files, the stick of
   public files, which carries the public files, each key's account key
   and BIP 129 record, and the sheets as PDFs) checked now → writing → "Written
   and checked on {label}", its files and items ticked → pull;
7. **Done**: a row per stick → **Unlock {vault}** → the key's page, or
   the checklist's done card, with nothing For the stick.

A wallet's description goes in shares, and its Coldcard multisig
config is offered, only for a `sortedmulti` (in `wsh`, `sh` or
`sh(wsh)`) or a `tr(NUMS, sortedmulti_a)` wallet, the shapes the shares
rebuild; any other wallet of several keys keeps the whole description
(`docs/WALLETS.md` §5). Shares and the config are rebuilt as Restore
rebuilds them and their first receive and change addresses compared
with the wallet's before any is written, to a stick, the Outbox or a
vault; on a difference none is written.

**Change the plan** under every card: a new vault on an empty stick, or
another copy of a planned vault, adds a stick to step 6; a stick
holding an existing vault takes **Add {vault}** (lock, the stick read,
both vaults unlocked, the key copied across, lock), with a key kept only
on paper offered **Finish its paper copy first**; a place removed before
its write drops its new vault, after it stays on the map.

Back leaves on any card but while writing; the key's page, Home's lead
and a waiting action offer **Continue the backup**. A lock resumes on
the step; a power-off ends the flow, the Power sheet listing "Backup of
{key}: written to {n} of {m} places". A stick visit while a flow is left
lists its files **In the backup**, not tickable.

## Wallets tab

Every flow, entry point and step is in `docs/WALLETS.md`. What it
guarantees for this file's rule:

- **Every way in reaches a wallet card** (`docs/WALLETS.md` §2) that says what
  the wallet is, which keys can sign here, how many signatures the
  transaction has and needs, and the one thing still to do.
- **A `tr()` wallet states who can spend by the key path** (F132,
  `docs/WALLETS.md` §3.4) on its card, on Make a new wallet's Check and,
  for a key not of the wallet, on a sheet that takes a hold before the
  wallet is added and as a caution on every spend from it.
- **A transaction never stops at "can't sign"**: a missing key opens **Scan
  a key**, which takes any of the wallet's missing keys (below); a
  missing signature opens **Collect**; a missing wallet opens **Load the
  wallet**.
- **Signing ends on two results**, the signed PSBT for the wallet that wrote
  the transaction and the finished transaction with its txid, each shown as
  a code and put For the stick the moment the transaction is finished
  (decision 24): Finish's primary is **Insert a stick to write it**
  (opening Files), or **Open Files** with a stick in, and each result's
  card carries **Remove** while it waits.
- **Anything added in a session offers Save to vault**, and locking lists
  what was not saved.
- **Creating a wallet ends in Backup and Save**, never on the descriptor
  alone.
- **Wallets with something loaded is the list and the card alone**
  (decision 12): every job (Sign a transaction, Back up, Show wallet QR,
  Sign a message) is reached from the wallet card, opened from a
  wallet's row, and from Tools. Wallets' ways in are Make a new wallet,
  Restore an existing wallet, **Spend from a backup, step by step**
  (offered with wallets loaded as well; the Spend tab lights **Wallets**
  in the sidebar) and last **Add a key**, "Keys first, wallets later"
  (decision 50).
- **Make a new wallet** (decisions 17, 20 and 37; F53, F55, F98; `docs/CODEX32.md` §14 D5).
  The cards are Kind · Quorum · Use vaults · Keys · Check · Back up;
  the flow opens on Kind, open, its first row ticked. Kind offers
  **Single key, backup in shares (codex32)** ("One key, its backup in 3
  shares, any 2 rebuild it", the counts the scheme's), the default,
  then Single key ("One key, its words on paper") and Multisig ("3 keys,
  any 2 sign", the quorum's), all three native SegWit; the other seven
  are behind **More kinds**. An info
  button follows "(codex32)" in the first row's name; it and the word
  carry a tooltip (DESIGN §4.12, Tooltip; F112): "codex32 is the only
  method that makes a seed and its shares fully by hand, the checksum
  included." A kind pressed closes Kind and opens the next card, the
  ticked one pressed again too. Opened by Move the money, Kind has no
  row ticked and Next waits until one is chosen (owner, 2026-10-24). Use vaults' **Yes** opens
  How many, **No** opens Keys; closed before it is answered it reads
  "Not answered".
- **Single key, backup in shares** (`docs/CODEX32.md` §6.9, §14 D5). On
  chain a single-key native SegWit wallet, one `wpkh` descriptor, one
  xpub. Its Quorum asks the codex32 scheme: **How many to rebuild it**
  (2 to 9, 2 to begin with) and **How many shares** (k to 31, 3), "Any 2
  of the 3 shares rebuild the key.", then the Account row and
  **Continue**; there is no None. Its one slot offers **Make codex32
  shares** only, which opens New key on Make codex32 shares with the
  scheme fixed, past Scheme on the identifier; Lock in fills the slot
  and makes the wallet. Any other key put in the slot is refused with
  its reason. Check has a **Backup** row, "codex32 w0za · 2 of 3". Back
  up opens on codex32's standard way, one string per place on paper
  (`docs/CODEX32.md` §9.5; that way is planned). Restore does not offer
  the kind: the wallet it makes is a Single key one. Every kind whose path has an
  account has an **Account** row, at the foot of Quorum under the
  quorum's line, or at the head of Keys for a kind with no Quorum card:
  "Account" and its number, starting at 0, with **Change**, which opens
  the − n + stepper in its place. The card that carries it says the
  account on its closed line ("2 of 3 · account 1") when it is not 0.
  Keys' **Make the wallet** makes it; Check shows the descriptor as a
  summary row that opens its code with the whole text under it, a
  **Path** row under it with **Change** (which takes the made wallet
  back and opens the Account row's card on its stepper), and the
  section label **The wallet's first addresses**
  above the address rows, so that a multisig's three rows do not read as
  one address per key; its press reads **It matches**, which leaves
  Check open with **Back up wallet** at its foot. There is no Back up
  card (owner, 2026-10-26): **Back up wallet** opens the backup on the
  new wallet (its chip "Then: Wallets", its way back the wallet's card)
  on The standard way. A single key made here is checked as it is made:
  Lock in opens Check with **Back up wallet** at its foot.
- **The Keys card** (decisions 35, 36 and 39). Where it would offer
  one way on only (a single key's empty slot with no key in the
  session, no key or wallet file in the Inbox, offered or refused, and no vault's key, or a backup in
  shares, whose one way is Make codex32 shares), the Continue on the
  card before it opens that key flow for slot 1 in its place; Keys
  stays in the list of steps, not done, and is done with the key once
  it is locked in (owner, 2026-10-24). A multisig, or a slot with more
  than one choice, shows the card. A key loaded here that
  fills one slot, or a cosigner's file whose key fills one slot, is not
  offered for any other slot; clearing the slot offers it again. A code
  scanned while a multisig's slot waits for a cosigner (one marked for
  later, else the first empty) fills it with an account key (an xpub
  file, a BIP 129 key record, a SLIP-132 key, a single-key JSON) or a
  descriptor that names one key, with its fingerprint and path, at the
  kind's derivation (any account) and, for a descriptor, under its
  script type, the path's coin type its xpub's network and that network
  the session's. A key with no origin or at another path is never taken
  for a slot, from a scan or from Files, since its path would be a
  guess; a descriptor of several keys is not split. What cannot be the
  slot's key (no origin, another kind or network, no key for the kind, a
  checksum that does not hold) stays in Files, the toast naming why;
  a key or wallet file in the Inbox that cannot be a slot's key is still
  listed on the card, disabled, with its reason under its name (a key
  with no origin: it needs its fingerprint and derivation path, which
  most wallets export), and a single key's card with such a file is
  shown, not passed over. While a
  slot is empty there is no button at the foot: the first empty slot's
  own **New key** leads; once no slot is empty the foot reads **Make the
  wallet**. A slot
  filled by a key loaded here from BIP-39 words with no passphrase has
  **Add a passphrase** (beside Show xpub QR on a multisig, on the slot's
  own row on a single key): **Passphrase**, **Passphrase again** and
  **Lock in** under the slot; Lock in adds the key the words and that
  passphrase make, labelled as the first with " · passphrase", puts it
  in the slot with a **Locked in** tag, and leaves the first key loaded.
  A key with a passphrase already, a cosigner's xpub, a share, and a key
  from SLIP-39, codex32 or a bare seed have no such button.
- **Sign a transaction reviews the transaction before Check** (decision
  19): the Transaction card comes before Check, carries the txid as one
  of its rows, and Check stays closed once a wallet has been checked this
  power-on.
- **The wallet card's first button** (decisions 43 and 45), primary, is
  **Spend from this wallet**; **Sign the PSBT from the stick** with a
  PSBT for this wallet in Files, which opens its review at once; **Carry
  on the spend · Signatures 1 of 2** while this wallet's spend is under
  way, as Home's lead says too; **Finish the backup first** while a key
  made here waits for its backup. Spend from this wallet opens the Spend
  tab on the wallet, past What do you have? (`docs/history/SCAN-TO-SIGN.md` §4):
  **Load the wallet in Sparrow** (the wallet QR, receive address 0, **It
  is loaded**; done already for a wallet checked this power-on) with **I
  have the PSBT from Sparrow ready: Scan it** over it, then **Write the
  payment in Sparrow** (**Scan the PSBT**, **On a stick**, this wallet's
  PSBTs in Files), then the spend's steps. Who signs, with fewer keys
  here than sign, offers **Scan a key**, its Unlock buttons and **Collect
  a signature**, which opens The other signatures. On a small panel the
  card's spend button sits under the signatures box, and the shortcut is
  its line and **Scan it**.
- **Scan a key; Can spend** (decision 57, `docs/history/SCAN-TO-SIGN.md` §2–§3).
  Where a wallet is short of keys, one **Scan a key** opens the camera at
  once, titled "Scan a key for {wallet}", a line per missing key ("Key 2
  · 5d388376", and " · {place}" where the backup's plan keeps that
  seed), with **Type the words instead** and Cancel. A seed whose key
  with no passphrase is a missing key loads at once, with no passphrase
  question: the camera stays on with "Key 2 added · 1 more" while more
  are needed, else closes with the toast "Key 2 added · Can spend". A key
  already here says "Key 1 · already here"; a seed of no key of the
  wallet closes the camera and opens the passphrase sheet (SeedQR read)
  with "{fp} is not one of {wallet}'s keys", which takes only a
  passphrase giving a missing key and refuses another with the same
  line; anything else is named as Scan a SeedQR names it. **Type the
  words instead** opens "Add a key for {wallet}", which refuses words of
  no missing key with "These words give {fp}, not a key of {wallet}". A
  key loads under its holder's name where the plan keeps one, else none;
  a name that is the fingerprint is never drawn beside it. Scan a key is
  on the wallet card's signatures bar in place of "Add 1 more key or
  collect 1 more signature" (full width under the line on a small
  panel), on the Wallets column as **Scan a key for {wallet}** in place
  of **Add a key** while the wallet shown is short, and on Who signs (the
  Spend tab's and Wallets' Sign step). A wallet with enough keys here to
  sign shows a green **Can spend** on the card's bar (in place of
  "Nothing missing"), on Who signs beside "{here} of {needed} signatures
  here", and at the right of its Wallets row, whose sub-line is then its
  shape alone; the small panel's list and Home's say "Can spend · 2 of
  2". Short, Who signs says "1 of 2 signatures here · 1 more needed" and
  a key not here **Not here**; its Scan a key, Unlock and Collect a
  signature show only while short, or, for a miniscript or tree wallet,
  until the spend is signed.
- **The backup comes before spending** (decision 45, F6). Back up's
  checklist: an item's foot button is its next action (a vault item's
  **Make a vault** or the way to one, then one **Save into {vault}**
  that saves every seed, passphrase and the description the plan puts
  there, then **Continue**; a copy's **Check my copy**, which types the
  words back for a words copy and scans a SeedQR copy; files' **Make
  it**), and the items after one not done do not open. A plan waits
  while a vault's stick has no place ("Vault 1's stick needs a place");
  a plan that keeps no copy of a key made here is not made into a
  checklist ("Seed {fp} is kept nowhere"); **Change the plan** stays
  under every item, and done work the new plan drops stays on the map. A
  key made here signs once its checklist is complete: the wallet card's
  first button is **Finish the backup first**, Who signs lists it
  **Waiting for its backup** with **Finish the backup**, and the card's
  address, Create's Check, the Key card's address and the chart's wallet
  node say "Back up before you receive". The done card, the wallet card
  and the open vault lead with the plan's three checks as a strip above
  the chart.
- **Shares and vaults in a plan** (decisions 40 and 41). A multisig's
  shares take **Keys left off each share**, a slider from 0 to m − 1,
  then the shares' fingerprints and the split audit, with **What is a
  share?** opening Learn on *Shares of a wallet description* (closing
  Learn leaves the plan where it was); the checklist's Shares card has
  the same slider. Each vault of a plan holds the seeds ticked into it,
  vault *v* the *v*th seed by default, a seed in more than one where
  ticked; each vault's stick has a place, by default one that adds no
  second key. The map names each vault ("Vault 1 · 9a6a2580"). The
  checklist has one item per vault, **Vault 1: 9a6a2580**, which makes a
  vault and saves its seeds and the wallet; an open vault holding
  another vault's seed is not offered there (**Lock it and make a new
  vault**). A stick visit with several vault files waiting ticks one.
- **Create's Keys card offers the open vaults' keys** (owner,
  2026-10-19; F45; `docs/history/NEW-WALLET.md` §16.1). Under each slot's
  session keys, each open vault's keys not loaded, under the vault's
  name (a key in another slot is not offered); a press loads that one key as Load into session does
  and fills the slot, the vault staying open. A locked vault whose file
  is read is one **Unlock {vault}** button, coming back to the Keys card
  with the vault open and nothing loaded.
- **New wallet from this one** (owner, 2026-10-19; F45;
  `docs/history/NEW-WALLET.md` §16.2). A wallet's card offers it beside
  Rename (on a small panel, in its list of buttons) for every kind
  Create makes. It
  opens Create filled from the wallet: Kind and Quorum closed with their
  values, the **Account** row (on Quorum, or at the head of Keys, as for
  any new wallet) starting at the lowest account no loaded wallet of the
  same keys and kind uses; each
  slot its key, loaded from an open vault where it is kept there; a
  cosigner's key held only as an xpub stays at the wallet's own account
  and script type, and otherwise its slot is Someone else reading
  "Needs Key n's xpub at account n". Use vaults is asked as for any new
  wallet; the name proposed is "{wallet} · account n" ("{kind} · account
  n" once the kind changed). The wallet it came from is not changed.
- **The backup starts with a plan, then a checklist of only what it
  needs** (owner, 2026-10-09, proposal A; `docs/WALLETS.md` §5). The
  plan opens on The standard way, on the board, with **Add watch-only
  software** and **Customize backup** (F53, `docs/BACKUP-FLOW.md`
  §17.21; the three presets Paper only, Paper and vault, and Paper,
  vault and watch-only software are withdrawn). Before F53 the presets
  filled the plan's questions, each a multi-choice
  list (DESIGN §4.2): where the seeds go, the places (how many, share or
  whole sheet, which keep the vault's stick, their names with a vault
  open), where the wallet description goes, the software and form, each
  passphrase's places. Beside them the map (one box per place, the
  vault, a stick of files, the software, the seeds on their own
  devices, each tagged secret, sealed or public) and the check: "Any one
  place lost: the rest rebuild the wallet", "One place found: can
  spend", "One place found: sees the balance", qualified "with the
  vault's passphrase" where the vault is what decides it. **Make the
  checklist** replaces the questions with only the items the plan needs,
  each done by what it does (a file For the stick or written, a seed or the wallet
  in the vault, a copy matched, the descriptor shown); the envelopes
  alone by **Done**. A place's name lives only in the vault (record
  type 11, `docs/VAULT.md` §7), which the next backup of the wallet
  starts from; no file, sheet, PDF or PNG holds one, and
  the blank template carries empty "Place ____ holds ____" lines
  instead. On a small panel: one question per page, the map as the
  plan's last page and in the envelopes item.
- **A seed's backup: by hand, into a vault, a file only past the secret
  sheet** (owner, 2026-10-09; `docs/WALLETS.md` §5 step 3). Paper stays
  first: a copy item per seed; the vault item has **Save into {vault}**
  (and "with its passphrase" when loaded with one) under each seed; the
  files item **Save as a file…**, which opens the secret sheet for the
  seed. The same on a small panel.
- **The backup says where each seed is kept** (owner, 2026-10-09;
  `docs/WALLETS.md` §5). The map on the side panel marks each line
  with its item's state; the copy page on a small panel lists the wallet (in which open vault, or
  not in one) and every key of the wallet by fingerprint: in which open
  vault, whether its copy by hand was checked, whether its file is in
  For the stick unprotected, or not here (backed up on its own device). With
  vault files and none open, it says the vaults were not checked.
- **Any loaded wallet backs up again, with every file on offer**
  (owner, 2026-10-09; `docs/WALLETS.md` §5). Entries: the wallet's card
  (**Back up**, also on the small panel), Start's **Back up a wallet**,
  the Tools tile, Create's **Back up** card and Restore's **Back up
  this wallet**. With more than one wallet loaded the head is "Back up a
  wallet" with the wallet's name as a chip, which lists the loaded
  wallets; a press on one starts the backup on it. A watch-only wallet
  (no seeds here) is asked nothing about seeds. The checklist's public
  files item is drawn from the
  wallet (descriptor, wallet file, multisig config, BSMS record, Bitcoin
  Core import, each key held here and its BSMS key record), the item
  showing the rows the plan's software picks. Each vault made has its
  own item (or the way to make or unlock one, back to the item), with
  its seeds and the wallet where planned (decision 41).
- **The descriptor and its shares as pictures** (2026-10-09). The
  backup's public files item (**PNG** on the descriptor's row) write `{name}-descriptor.png`: the
  checksummed descriptor as one static code, with the wallet's name, its
  shape (and the network when not mainnet), the keys' fingerprints, the
  descriptor's checksum and "Public: watch only, spends nothing" under
  it. Past one code (version 25 at ECL M) it goes as the BBQr parts the
  QR view makes, `{name}-descriptor-1-of-3.png` and on, each labelled
  with its part. The shares item makes each split share's
  `{name}-share-k-of-n.png` beside its PDF and text: the share's text as
  one code, labelled with the share's number and quorum, the keys it
  holds and leaves off, and "Not a wallet on its own". A scan of a share
  picture comes back as the share, and Restore takes any quorum of them.
- **Every public file a flow makes as a code and a picture**
  (2026-10-09; `docs/QR.md` §4 item 4). Beside a file's row, For the
  stick with **Remove** (decision 24), sit **Show as QR** and **PNG** where a wallet or a person
  reads it from a code: the backup's public files item (descriptor,
  multisig config, BSMS descriptor record, each key held here and its
  BSMS key record), Create's Keys card (**Show xpub QR**, **Xpub PNG for
  the stick**, **Xpub file for the stick** under each key held here),
  Sign a message
  (the message file For the stick as it is signed, **Show as QR**, **PNG
  for the stick**) and
  Silent payments (**Record for the stick**, **Record as QR**, **Record
  as PNG**). The QR sheet carries **PNG for the stick** (**PNG** beside
  the format on a small panel) whenever it shows one code of public
  content; a row's **PNG** is that button without the sheet, and says
  "{name} waits for the stick". Labels under the code: `Key {fp} · {kind}
  · {path}` for a key, `Silent payment address · {fp}` for the silent
  payments address and record (with "Not checked: load key {fp} to check
  this address" while a wallet's key is not loaded; no code where a loaded
  key with its fingerprint gives another scan key or spend key), `Signed by {address}` for a message, the
  wallet's shape and keys for its files. A file past one code says so
  instead of writing parts; a PSBT, a transaction, a file shown from
  Files and a secret offer no picture.
- **Receive addresses as codes** (F28; `docs/QR.md` §7). A receive
  address row opens its code when pressed, a chevron at its end, as
  Create's descriptor row opens the wallet's: the wallet card's first
  receive address (and the small panel's), the first addresses of
  Create's Check, Restore's Check and the chart sheet's wallet, and
  Vanity address's found address. The code is the address as printed,
  shown whole under it; no PNG. A change address, the signing steps'
  **Addresses · Compare** and a wallet whose backup is not done ("Back
  up before you receive") open none.
- **A key in no wallet has a way on**: the card's buttons above with no
  wallet loaded; with wallets loaded, the key's row in the left column
  carries **Make a wallet from this key**. The left column runs top
  down, wallets, keys in no wallet, then its buttons, and scrolls when
  taller than the screen.
- **The wallet's card shows it at a glance** (owner, 2026-10-10;
  `docs/history/NEW-WALLET.md` §6). Its Keys list becomes **At a glance**, a
  structure chart (DESIGN §4.16): the wallet (name, shape, checksum);
  its keys, numbered as before, each with its fingerprint, label and
  where it is ("Can sign here", "Can sign · passphrase", "Cosigner ·
  xpub only", "On its own device"); its backup, a node per spot of the
  plan's map, each listing what it holds ("Key 1 words", "Vault 2
  stick", "Key 2 seed + passphrase", "Description"), its tag, what
  Backups saw of it, and what it gives whoever finds it alone
  ("Cannot see balance", "Sees the balance", "Can spend", "Can spend with the
  vault's passphrase"; a vault "Sealed", with where its stick is). A
  line runs from the wallet to each key and from each key held here to
  every node holding its seed, a colour and dash pattern per key; a
  cosigner's key has none, and its seed is in no node. Above it the
  plan's three checks as a strip (decision 45). The plan is the backup's once its checklist
  is made, else the one recorded this power-on (kept across a lock),
  else an open vault's. No plan: "No backup plan" with **Back up**; a
  plan in a locked vault not remembered: "Plan in {vault}, locked" with
  **Unlock**. Every node, and every line of what a backup node holds,
  opens its sheet (below); while the wallet is short of keys a
  key not here reads **Not here**, and one an open vault holds keeps
  **Load from vault** as a line of its own; the one way to scan any
  missing key is the signatures bar's **Scan a key**. The card's
  body scrolls between its head and its buttons. On a small panel an
  **At a glance** row opens the chart as its own page, one column, no
  lines, each backup node naming the keys it holds ("Key 1 words · Key
  2 in the vault"); Back returns to the card.
- **The chart's sheets** (decision 44). A press on a node of the chart,
  on the card or in an open vault, or on a line of what a backup node
  holds, opens a sheet (DESIGN §4.16): its title, what it is and where
  (Shape and Checksum; Where, Its seed is at; Stick, Found alone, Seen
  here; Kept at), then its actions as menu rows, each leading into the
  flow that does it, and Close (Back on a small panel, where the sheet is
  the page). Opening a sheet changes nothing and puts nothing For the
  stick; nor does opening the backup's checklist from the chart, whose
  items each offer what they make. The wallet: **Open in Wallets** (in a
  vault), **Show wallet QR**, **Public files** (the checklist's item),
  **Print the wallet sheet again** (or the shares), **Rename the wallet**
  (its card's name field), **Change the plan**, **Check addresses** (the
  descriptor row and the first addresses, as Create's Check). A key held
  here, or kept in the open vault: **Show xpub QR**, **Xpub file**,
  **Explore this key**, **Where it is** (its lines bold, the rest dimmed,
  until the next screen or sheet); **Make another backup of this seed**:
  **By hand** (the checklist's copy, with Seed XOR and codex32), **Into
  Vault n**, **As a file**, each where the plan has that item; **Check a
  copy of this seed**: **Type my copy** for a words copy, which opens the
  typed check at the checklist's copy item, and **Scan my copy** for a
  SeedQR copy with a camera, which leaves the chart under the camera. A
  key not here: **Load this key** with its way in, where the quorum needs
  it; **Show its xpub**, from the wallet's descriptor. A place: its
  lines, each opening its own sheet; **Rename**; **What goes in its
  envelope**; **The envelope list** (a PDF For the stick, places by
  number, never by the names the vault keeps). A vault: the vault way or
  **Open the vault**, **Save its seeds into it**, **Copy to another
  stick** (the stick visit), **Write it out again** while it has changed
  since it was written. Files: **Write the seed files again**, **Write
  the public files again**. Software: **Show the descriptor QR again**,
  **Public files for this software**. Lines: words, **Type my copy**;
  SeedQR, **Scan my copy** (**Type my copy** with no camera); **Make
  another copy**; the wallet sheet, **Print it again**, **As a QR
  picture**, **As text**; a share, **Print the shares again**, **Keys
  left off each share**; a vault's stick, the vault way and **Copy the
  vault to another stick**; a seed file, **Write it again**; the
  description, **Show wallet QR**. A flow on Back up comes back to the
  card, or to the vault opened from.
- **The chart changes the plan** (decision 49). A change to where
  something is kept is asked on a sheet first: its title says the change
  ("Move Vault 1 stick to Place 1"), the plan's three check lines read
  before → after, a line made worse in `WARN`, and **Confirm** makes it;
  Close leaves the plan as it was. The changes: **Move** (a line at a
  place, to another place or **A new place**); **Another copy** (words
  or SeedQR by hand, a passphrase, the wallet sheet, a vault's stick, at
  a place not holding it, a passphrase never with its own words); **Add
  here** (a place: what is held here and is not there yet); **Into Vault
  n** and the vault's **Save more into it** (a seed held here, with its
  passphrase where it was loaded with one, the description); **As a
  file**; **I destroyed this copy**, and **Remove from the plan** for a
  marked line or the stick of files ("Destroy what goes off the plan by
  hand"); **Remove the place**, which first asks where each thing in it
  goes (another place, or **Nowhere**) and is pressable once each has
  one. Once confirmed, the flow that makes what was added follows: the
  copy by hand with its check, the vault's item, the seed files, the
  sheet For the stick, the stick visit for a vault's stick ("Vault 1 ·
  copy", its own line, counted by the check), a passphrase shown to copy
  (**Show it**) and then **Check my copy**. From the first change the
  map is the plan: the backup's record and the vault's plan record keep
  it, the chart and the checklist read it, and it loads back the same on
  another power-on. **Change the plan** (the wallet's sheet, the
  checklist's foot) then asks first, "Your changes on the chart are
  replaced". In the vault alone, never in the kept state or on disk
  outside the vault: the date each
  thing of a place was checked (a place's **Mark checked**; a passphrase
  checked), **Lost** and **Exposed** (a line, or **Mark lost**, **Mark
  exposed** on a place: struck through on the chart until taken off the
  plan), a key not held here's **Name its holder** and **Mark checked**
  (the date its holder confirmed it), a place's names and a vault's
  **Rename**; with no vault open these rows read "Open a vault to keep
  this" and lead to one. A check of a copy matched in the checklist with no
  vault open is to be held in RAM only until the next unlock, then
  written into the plan's notes (`docs/BACKUP-FLOW.md` §18.3, planned). A mark says what follows, in order: "Someone can
  now spend" (or "could … with a vault's passphrase") with **Move the
  money** (Make a new wallet, opened on Kind with no kind ticked, the
  person choosing one, owner 2026-10-24; its last card then offers
  **Move the money from {wallet}**, the spend from the old wallet with "Every coin
  to {new wallet}" and its first address on Write the payment); "The
  wallet can no longer be rebuilt from what is left" with **Back up again
  now**; "Someone can see the balance"; "Nothing lost that is not
  elsewhere" with **Make a replacement copy**. **Check it** on a
  passphrase line (or the key's **Its passphrase**): typed into a masked
  field, the key it makes with the seed here compared by fingerprint, the
  typed text wiped at once; "Matches: it makes key {fp}" or "Does not
  match", and a match dates the line in the vault.
- **Backups age** (planned, not built; `docs/BACKUP-FLOW.md` §18).
  Every check of a copy (Check my copy typed or scanned, a codex32
  string's check, a passphrase's Check it, a place's Mark checked, a
  vault's stick read and unlocked) dates that copy in the plan's notes
  in the vault, asking **Which copy was this?** where the seed has that
  form at more than one place; a date is written only from a checked
  clock (`docs/TOTP.md` §1.3). The chart's place nodes, their sheets,
  the card's strip ("To check: Bank box") and Backups show each copy's
  age ("Last checked 14 months ago") and mark it **To check** past the
  interval (12 months proposed); with the clock not checked they show
  dates only. A wallet with a relative timelock shows its **Spend
  paths** on the card, the chart's wallet sheet and the wallet sheet
  PDF, each wait in blocks and days with **Move each coin within**, and
  the PDF a table to fill by hand per deposit.
- **Restore** (decision 48): **Kind** (Single key · native SegWit
  ticked, Multisig · native SegWit, More kinds), with **I have the wallet
  description: Scan it** and **From Files** above it; **Quorum** for a
  multisig (M of N on two sliders, 2 of 3 to start); **Description**
  (Scan it, From a file in Files, the shares, the stick or a vault; for a
  single key **I have the seed** first, for a multisig **I do not have
  it**); **Keys**, a slot per key, whose ways in (Type the words, Scan a
  SeedQR, a vault's seed or a key already loaded, **Its xpub** with no
  description or **Not here** with one) stand over the slot they fill,
  the passphrase asked after the seed (SeedQR read; typed words take it
  on Add a key); **Check** (the descriptor row and the first addresses);
  and **Done**. A description read on Kind or Description fills Kind and
  Quorum and opens Keys. With the description known each seed is matched
  to its keys as it is entered: a match reads "Key 2 of the wallet"; any
  other reads "Not a key of this wallet: check the words or the
  passphrase" and is not loaded. **Make the wallet** once every slot is
  filled. Done, whatever the vault question's answer, offers, all
  secondary, **Spend from this wallet**, **Back up this wallet**,
  **Another wallet from these seeds** (New wallet from this one) and
  **Wallets**. A restored key is not held for its backup. Vaults set up
  before the seeds wait in the wallet's flow: Back up resumes it at the
  plan with them, and until a plan is made the wallet's card and Backups
  read **Continue the backup**.

## Spend tab

Spend left the sidebar and the small Home grid (`docs/history/SIMPLIFY.md`
§6.2, 2026-10-10); it is reached from the Wallets empty state's
**Spend from a backup, step by step**, the Learn sheet's first row
**Spending, step by step**, Home's lead when a spend is under way, and
a wallet card's **Spend from this wallet** (decision 43), which opens it
on that wallet at **Load the wallet in Sparrow**, then **Write the
payment in Sparrow** (the PSBT by camera or stick), with **I have the
PSBT from Sparrow ready: Scan it** over the first page. The tab's own
screen and steps below are unchanged.

The Spend tab for someone spending for the first time
(`docs/SPEND-FROM-BACKUP.md`): the map → **What do you have?**, headed by what
starting from the stick means until something comes in (F103)
(`docs/history/SCAN-TO-SIGN.md` §4, 2026-10-11: a vault in Files with its
Unlock and list, **Scan the QR codes you have**, **Type the words you
have**, **Not sure what you have?**, what came in, and the next
question: with one seed **Anything else in the envelope?** and **That's
everything**; with several and no description **Scan the description**
or **I can't find one**, which shows the seeds' shape and **Make the
wallet**; a description opens the wallet, and a wallet opened moves on,
its row under **Spend from** moving on again when pressed) → check the money in Sparrow
(**Yes, my money shows**; for a wallet worked out from seeds alone,
**Check the money is there, on the online computer** with **No money
shows**, which tries a passphrase, Taproot, nested SegWit and legacy in
turn, §5; **Receive address 0** and the receive rows open their code,
F28) → write the payment in Sparrow → bring the PSBT (camera, or a
stick: the lock cycle, after which the tab opens on the same page) → the
spend's own steps → **Put everything away** (lock, what waits for the
stick written to it, power off). Every page that cannot be reached yet sends the
person to the one that comes first. The intake scanner, **Scan what you
have**, reads one code after another until **Done**: a seed asks how it
loads and the camera comes back, a description opens the wallet, split
sheets rebuild it, an xpub fills a cosigner's box, a SLIP-39 share or a
codex32 string goes to Add a key's form for it, and a PSBT waits for
Bring the transaction here.

**Registering a wallet from a spend** (F135). A spend from a wallet of
several keys that is not registered here is blocked: the Transaction
card's warnings say "input 0 spends from a wallet of several keys that
is not registered here; register the wallet to sign". Where the PSBT
carries the cosigners' extended keys, **Register this wallet** under it
opens a dialog: the shape ("2 of 3, P2WSH, sorted"), a line per
cosigner (fingerprint · path, "· Yours" for a key loaded here), **Name**
(the PSBT's file name, "Savings Unsigned" for `savings-unsigned.psbt`),
**Hold to register** and **Cancel**. The hold adds the wallet and the
spend goes on from the same card, now signable; a wallet whose keys do
not give the spend's addresses, or none of whose keys is a loaded key's
own, stays unregistered and the dialog says which ("These keys do not
give input 0's address", "None of these keys is yours"). Where the spend
cannot tell sorted keys from keys in the order listed, the dialog adds
"Most coordinators sort the keys" over **Sorted** (chosen) and **In
this order**. Where the PSBT does
not carry the keys, the card says "Load this wallet's descriptor to
sign": Files, a code or Restore load it, and the spend goes on. An
input of the person's key in a miniscript or a taproot tree of one key
whose wallet is not registered stops nothing (F141): the warnings say
"Input 1 is left unsigned: no registered wallet covers this taproot
tree. Register its wallet to sign it.", and signing signs the rest.

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
files** (its updates are signed by PK and KEK) and **Sign with db**,
with Files' **Sign in Secure Boot**, read **Back up first** and open the
flow. **Check a signature** does not wait. Keys in a vault unlocked from
a stick sign at once.

**Enrolment files** → policy (Windows-compatible or own keys only) → the
files For the stick → Learn page: enrolling in the firmware's setup screen.

**Sign `BOOTX64.EFI`** → needs the file copied in (none: "Copy
BOOTX64.EFI in from a stick first", with what a stick visit is) → the
release check (F134): the release's `SHA256SUMS` and `SHA256SUMS.asc`
copied in beside it, the signature verified with the release key built
in, the file listed; a refusal named in place of the hold → **Hold to
sign** → signed file For the stick → Learn page: copying it onto the boot
partition on another computer.

**Check a signature** → result against the vault's db certificate.

## APK signing key

Removed (F99): `docs/history/APK-SIGNING.md`.

## Nostr key

(`docs/NOSTR.md`, F27; specified, not built.) BIP-85 application
128002', on the stick and the Pi only: Faraday Demo has no Nostr
key on any network, and Practice has none. Needs a loaded key: with
none, Tools' **Nostr key** tile reads "Load a key", as Child seeds and
passwords does.

The tile opens the BIP-85 screen with Nostr chosen → Key (skipped with
one key loaded) → **Identity and account** (1 and 1, filled in; 0 and
anything above 2147483647 refused; **Find a pair**: the npubs of
identities 1 to 5 at account 1 and accounts 1 to 5 at the identity
typed, a tap picks the pair) → **Value**: the path, the npub with
**Show as QR** and **PNG**, the nsec masked until **Show** and never a
code or a file → **Save into {vault}** (an Entry; no vault open:
decision 10's way into making or unlocking one, and back; or the
backup flow of decision 59, any number of places, none included, with
nothing waiting for it) or **Export
encrypted…** (Password and Again, 12 printable characters or more →
"Encrypting" → `ncryptsec1…` as text, **Show as QR**, **PNG** and **For
the stick**, sealed). There is no **Out unprotected…**: the
ncryptsec is the only way the nsec leaves. Files and labels are named
by the npub's first 13 characters, never by the key's fingerprint or
the two numbers. The BIP-85 tile's Application card lists Nostr
seventh and leads to the same cards.

A saved Nostr entry's page in Vault contents adds **Show as QR** and
**PNG** for its npub, **Export encrypted…**, and, with its key loaded,
a **Path** row saying whether the notes' path gives the entry's key. An
Entry holding an nsec has no **Export for KeePass**. A file holding an
nsec is a secret in Files with no code and no way For the stick; its
**Save into {vault}** makes a Nostr entry.

## Explore

(F28, F76; the values as codes are not built, `docs/QR.md` §7.) From a key's **Explore
this key** or the Tools tile → the path and the script type → the
**Address**, **Public key** and **Extended key** at that path, each a
row that opens its code when pressed (the value as printed, whole under
the code, no PNG) → **Wallets**.

## Lightning node key

From the Tools tile → ldk-node from a loaded key, or LND with its
aezeed typed → **Node id**, with **Show as QR** and **PNG** (F28, not
built, `docs/QR.md` §7;
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
**Done**. Over 256 KiB: "Larger than 256 KiB: carry it on a stick".

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
code shown, or clicks the target drawn → **Ignore this device** lists it
in Settings until unplugged. A keyboard that types three codes' worth of
letters without the code is ignored the same way, with a toast saying
to unplug it to try again. After three keyboards are ignored that way,
new keyboards are ignored without a code until Faraday starts again.

**No camera, or the camera is refused** → the scanner says which, and
offers Type and Read a file.

**Settings**: keyboard layout (US first), display scale, text size
(Standard, Large, Larger, `docs/TEXT-SIZE.md`), idle lock and
power-off times (30 and 60 minutes at first, F58), reduce motion, light
or dark (light on a device with no settings saved, decision 32), the
network (decision 14), ignored devices, **Time** and About (version, build hash). They are kept across a
lock in RAM, and across a power-off in `faraday-settings.txt` on the
boot stick, read at boot before the passphrase (`docs/history/PLAN.md` §5.2). Never
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
   left (a calculator, Explore) has no row.
5. **The narrow window and the small panel** (the compact layout): the
   small Home leads with one tile per flow in progress, its name and
   step, opening it, before its other tiles.
6. The survey of every screen whose state is kept when it is left:
   below.
7. **Speed.** Which flows are in progress is worked out once a frame
   through the drawing memo (`docs/SPEED.md` §3.3) when it costs more
   than lookups; a press decides from the state as it is.
8. Tests: moved to `docs/history/FLOWS-LOG.md`.

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
| Spend: the Spend tab, a transaction's steps | Wallets | `spend`, `family` | a transaction read, or something in on What do you have? | signed and put out | **Start over**; another wallet's spend | the spend is wiped; the tab's own state is kept |
| Sign a message | Wallets | `message` | a card done | the signature made | **Sign a message** pressed again | wiped |
| Create a vault, on its own | Vaults | `vaults.create` | a card of Customise done (the size is done for the person) | the vault made | **Create a vault** pressed again; a flow's detour dropped on Back | wiped |
| GPG, Secure Boot keys | Tools | `backup_flows` | past Where it will be kept | Done | the kind's tile resumes one not made; nothing lets one go | kept |
| A wallet's backup flow, on its own | Wallets | `backup_flows` | past The plan, or the keys made | Done | Back up's plan | kept |
| New key, Add a key | — | `keygen` | | | dropped when its screen is left | |
| BIP-85, Silent payments, Lightning, Explore, the calculators, Vanity, the word list | — | dropped when left | | | | |
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
   (`docs/learn/faraday/boot/maker-*.md`), a field of `Maker` in
   `faraday-learn`, its text in `faraday-strings`:

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
6. **Strings** (`faraday/faraday-strings/src/en.rs`): "Restart",
   "Power", "The computer powers off, or restarts", "Usual BIOS key", and one
   per key in item 4.
7. Tests: moved to `docs/history/FLOWS-LOG.md`.

Ruled out (owner, 2026-10-16): **Restart into the BIOS**, the computer
restarting straight into its firmware screen by the UEFI variable
`OsIndications`, as Faraday Starter does on Windows from the person's
own system. The stick writes no firmware variable: README (Stateless,
"no access to its firmware variables"), docs/history/PLAN.md ("Faraday never writes
EFI variables"), `EFIVAR_FS` in the x86_64-uefi
board's `kernel.forbidden`, and the Learn pages that cite it. It would
also need a privileged helper for the write. The BIOS key row does the
same job and writes nothing.

### Cancel: leaving a flow for good (owner, 2026-10-19; F51)

A flow in progress has a way out where the person is, not on its
sidebar row (a press on the row only returns to the flow).

1. **Cancel** at the top of every flow that has a sidebar row, beside
   **Steps only · Guided**: it ends the flow, its row goes, and the
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
5. Tests: moved to `docs/history/FLOWS-LOG.md`.
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
