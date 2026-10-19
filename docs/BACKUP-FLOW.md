# The backup flow

**Status:** spec v0.3 · 2026-10-14 · decided by the owner
(`docs/DECISIONS.md` F31), reworked the same day to the order "decide
where the key will live, then make it", and §15's questions answered. Batch B1a (§14, without the Bitcoin
wallet) is built for the APK key, GPG keys and the Secure Boot keys, and
batch B1b for the Bitcoin wallet (§10, §11), each with the choices §16
records; the Nostr key's part is not.

One guided flow makes and backs up every kind of secret Faraday keeps
in a vault: an APK signing key (`docs/APK-SIGNING.md`), a GPG key, a set
of Secure Boot keys, a Nostr key (`docs/NOSTR.md`), and the keys of a
Bitcoin wallet with its backup plan (`docs/NEW-WALLET.md`). The person
first says where the key will be kept; Faraday reads every stick that
plan uses while nothing secret is open; the vaults are unlocked or made;
the key is made straight into them; one lock seals them; and each stick
is written in turn, by its place. The person stays in the flow from the
first step to the last, and it never shows the stick visit page.

**Why a document of its own.** Four of the five kinds are not wallets,
and the Bitcoin wallet's plan uses the same steps around its own
checklist. The flow is written here once; `docs/NEW-WALLET.md` §14.8,
`docs/APK-SIGNING.md` §3 and §4, `docs/NOSTR.md` §5 and
`docs/FLOWS.md` decision 59 say where each kind enters it.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every build and test
  through `local/remote-run.sh`; every dimension a token; labels, not
  prose, on working screens; tests for user-facing behaviour only;
  `just` once at the end of the batch; no `rm`; no commits; no agents.
- Every string in `faraday/faraday-strings/src/en.rs` with the prefix
  `backup_flow_` (§12). OpenSigner's files and strings are not edited.
- A stick is handed only to a clean process (`PLAN.md` §5.3 and §5.4).
  Nothing in this flow opens a secret in a process that holds a stick,
  or hands a stick to a process that holds a secret. A change that would
  need either stops and asks the owner.
- The flow is driven by the plan and narrows nothing the plan can say
  (§2). A change that would make the flow refuse a plan the plan screens
  accept stops and asks the owner.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at.

## 1. What is decided

(owner, 2026-10-14; F31.)

- **One flow for every kind**, in this order:
  1. **Where it will be kept**: the plan, its places, which of them keep
     a stick, and which vault each stick holds.
  2. **Bring in the sticks**: each stick the plan already names is
     inserted, one at a time, with nothing secret open; Faraday checks
     it can be used, reads what it holds, and the person confirms its
     place.
  3. **Unlock and make the vaults**: existing vaults unlocked with their
     passphrases, new ones made with their own.
  4. **Make the key** straight into every vault the plan keeps it in.
  5. **Lock once**: every vault sealed, everything secret wiped; the
     flow's state survives in `/run/faraday`.
  6. **Write each stick by place**: "Insert the stick for Vault 1 (Place
     1)", what will be written, writing, pull it.
  7. **Done**: unlock to use the key.

  A Bitcoin wallet runs the same steps in another order (owner,
  2026-10-15; §10.2): the vaults first, steps 2 and 3 for the vaults the
  person says there will be; then step 4, the keys made into memory; then
  the plan of step 1, in Back up, with those vaults as its vaults; then
  steps 5 to 7.
- **Why this order.** A stick is read only with nothing secret open. A
  key made first would have to be locked into some vault before a stick
  holding the vault it belongs in could be read; a new Bitcoin seed held
  only in memory would be wiped by that lock. So every stick the key
  will be kept on is read before the key exists. For a wallet the sticks
  read before the keys are those of the vaults the person says it will
  use. Restore, which makes nothing new, takes the same steps with its
  seed typed or scanned at step 4.
- **The plan decides what each stick holds**, before any stick is
  inserted. An empty stick holds what the plan puts there and nothing
  else; a vault has one copy per place the plan keeps it at, as many as
  the person adds, and more than one only where the plan says so.
- **Places, by kind.** A key that cannot be made again from seed words
  (APK, GPG, Secure Boot) is planned at **two places or more**: the
  count cannot go below two, and a plan with fewer is not offered. A
  **Bitcoin wallet** may have as many or as few places as the person
  likes, one included; when every secret of the wallet would sit at one
  place, the plan says so in `WARN` and strongly recommends keeping
  them apart (§10.3). A **Nostr key** may have any number, none included,
  with no warning. Places are added or removed at any time, part way
  included (NEW-WALLET §14.4). The plan's check lines report what the
  places give; they do not block.
- **Every form the plan allows**: a vault on a stick, paper (words,
  SeedQR, steel), shares, the wallet sheet, a stick of files, the
  watch-only software, or a mix. The stick steps cover only the places
  whose plan includes a stick. A plan with no stick has nothing to bring
  in and no lock for writing.
- **A vault's second copy** is refused on a stick that holds the first
  copy, so the first is never written over and counted twice, and needs
  the hold "This is a second stick". Faraday cannot prove two sticks are
  different; that is the extent of the check.
- **What waits.** Signing waits for the writes for every key that cannot
  be made again from seed words: the APK signing key, GPG keys, the
  Secure Boot keys. A Nostr key is derived from the seed by BIP-85:
  nothing waits. A Bitcoin key keeps NEW-WALLET §14.3's rule. A key in a
  vault unlocked from a stick signs at once, so existing keys, the
  release's GPG key among them, are not held. A waiting key signs once
  every place its plan gives it is written, which is two or more.

## 2. The plan drives the flow

For a Bitcoin wallet the plan is Back up's (NEW-WALLET §5, §9 and §14),
read from its answers or, once edited on the chart, from its map, and
kept as the type 11 record's lines (`docs/VAULT.md` §7): `seeds`,
`places`, `split`, `wallet`, `software`, `form`, `omit`, `vault v` (the
seeds vault *v* holds), `sticks v` (the places that keep its stick),
`pass i`, and `spot …`. The flow reads `plan::map`'s spots and nothing
else:

| Spot (map) | What the flow does with it |
|---|---|
| `At::Place(p)` holding `VaultStick(v)` | a stick, "the stick for Vault v (Place p)": brought in at step 2, written at step 6 |
| `At::Place(p)` holding words, SeedQR, a passphrase, the sheet, shares | the checklist's paper items (Templates, printed from Faraday Starter; Copy with F29's check; Sheets), as today |
| `At::Vault(v)` | a vault: existing or new (step 1), unlocked or made (step 3), its seeds made into it (step 4) |
| `At::Files` | "the stick of files" (seed files, public files): written at step 6 |
| `At::Software` | the descriptor shown, or its files on a stick: written at step 6 |
| `At::Away` | nothing: the seeds on their own devices |

So the flow allows every map the plan screens and the chart can draw:
places with only paper, places with only a stick, places with both,
vaults at one place or several, a key in one vault or two, a plan with
no vault. Which spots exist is the plan's; the flow only orders the
work.

**The other kinds' plan** is the same model with one key and no paper:
places, each keeping a stick that holds one of the key's vaults. The
default is one new vault, "Vault 1", at Place 1 and Place 2, so two
copies of it. The person may add places, give a place an existing
vault, or give it a vault of its own; a key planned in two vaults is
made into both. For APK, GPG and Secure Boot keys a place can be
removed only while more than two remain: the place count's control
stops at two, and removing a place's stick leaves the place to be given
another. A Nostr key's plan may have any number of places, none
included; with none, the entry is saved into the vaults already open
and no stick is written. It lives in the flow's state (§7), not in a
record: once the flow is done, the vaults themselves hold the key.
GPG's **Paperkey backup** stays on the key's page, outside the plan.

## 3. The kinds

| Kind | Known by (public) | The flow starts at | Step 4 makes | Its page | Signing waits |
|---|---|---|---|---|---|
| APK signing key | the certificate's SHA-256 | Tools' **APK signing key**, Vault contents' **Make an APK key** | Make's form: name, package | the key's page | Sign an APK, and Files' Sign with APK key |
| GPG key | the primary key's fingerprint | Tools' **GPG key**, Vault contents' **Make a key** | name and email, expiry | the key's page | Sign a file, Renew |
| Secure Boot keys | the owner GUID | Tools' **Secure Boot keys**, **Make keys** | PK, KEK, db | the Secure Boot screen | Enrolment files, Sign with db, Files' Sign in Secure Boot |
| Nostr key | the npub | the Value card's **Save into a vault** | the Entry (NOSTR §5), with the key loaded | the saved entry's page | nothing |
| A Bitcoin wallet's keys | the descriptor checksum and the fingerprints of keys made here | Make a new wallet, Restore, Back up (§10) | New key's seeds, or the restored seed | the wallet card and the checklist | spending, NEW-WALLET §14.3 |

These tiles no longer ask for an open vault first (decision 10's "Make a
vault first"): step 1 is where the vault is decided. A key, or a public
value of it, is never on a stick before step 6.

**What does not wait**: everything that is not a new signature by the
key: the APK certificate, a GPG key's public key and revocation
certificate (made For the stick with the key, decision 24), Secure
Boot's **Check a signature**. Enrolment files carry signatures by PK and
KEK, so they wait.

## 4. The steps

### 4.1 Where it will be kept

A Choice-and-rows card (DESIGN §5, Menu rows with Add):

- one row per place, "Place 1", "Place 2", with what it keeps; **Add a
  place**; a place's row opens it: **A stick with** a vault (Vault 1,
  Vault 2, **A new vault**) or nothing; for a wallet, the paper forms
  stay in Back up's plan (NEW-WALLET §4 and §5);
- one row per vault: "Vault 1 · new" or "Vault 1 · an existing vault",
  with the places that keep its stick; for a wallet, the seeds it holds
  (`vault v`);
- the check lines under them, as the plan's map panel draws them, for
  information.

**Continue** is live whatever the count the kind allows (§1): two or
more places for APK, GPG and Secure Boot, one or more for a wallet, any
for Nostr. A plan with no stick goes straight to step 4. A wallet's
plan shows its check lines and, where they apply, §10.3's two lines.

### 4.2 Bring in the sticks

Only the sticks the plan already names: each `VaultStick(v)` at a place.
Sticks the plan names later (the stick of files, the stick of public
files, a place added after the key is made) are checked at
step 6 instead (§4.6), since they hold no vault the key goes into.

- **A clean process first.** With anything secret open when step 2
  begins, its card's foot is **Lock and bring in the sticks**: the lock
  of `PLAN.md` §5.3, the flow's state kept.
- **One place at a time, by the plan**: "Insert the stick for Vault 1
  (Place 1)". Two sticks in: "One stick at a time".
- **Already read this session.** A stick seen this session (the boot
  stick, a visit), whose listing and free space Faraday kept, is offered
  on the card: **Use {label}, read at {time}**, with no insertion, as
  long as its room is known to be enough.
- **Usable**, else why and what to do, before anything is made:
  - not FAT32 as Faraday reads sticks: "Not a FAT32 stick. Format it as
    FAT32, or use another stick";
  - an `OSKBOOT` partition: "This is a boot partition. Use another
    stick";
  - read-only: "Cannot be written. Use another stick";
  - too little room: "Not enough room: {needed} needed, {free} free".
    The room is the vault's length, twice where a file of the same vault
    is there to be replaced (VAULT §6 writes beside, then removes), and
    the plan's other files at that place where already known.
- **Which vault.** The stick's `.ofv` files are listed by name (`vault.ofv`,
  `vault-2.ofv`, …), each with its size and, when seen open this
  session, its summary ("Main · seen 14:02"):
  - a place the plan gives an **existing vault**: "Which vault is Vault
    1?", a row per file; the one chosen is copied in, as Files holds a
    vault from a stick today; the others stay on the stick untouched;
  - a place the plan gives a **new vault**, on a stick holding vaults:
    **A new vault beside them** first and chosen, then a row per file
    (choosing one makes the plan's vault that existing one); the new
    vault is written beside them under a free name;
  - an empty stick at a place the plan gives a new vault: bound to it
    with no question; an empty stick is never offered a copy the plan
    does not put there;
  - a vault the plan keeps at several places: each further place's
    stick takes another copy, beside or over the copy it holds of that
    vault;
  - two files of one vault (one salt) on one stick: "{label} has two
    copies of {vault}: {name} and {name}", and that stick cannot serve
    that vault; another vault, a new one, or another stick;
  - a copy of the chosen vault that differs from one already read this
    session (the same salt, other bytes): "{label}'s copy of {vault}
    differs from {label}'s"; the person picks the copy to unlock, and
    the other is written over at step 6 only after a hold (decided, §15
    item 3).
- **Which place.** Each stick is named by the plan. The person confirms
  it: a tap, **This is the stick for Vault 1 (Place 1)**, when the stick
  reads differently from every stick already bound; a hold, **Hold: this
  is the stick for Vault 2 (Place 2)**, when it reads the same as a
  stick bound to another place (two empty sticks from one image, of the
  same label and serial), since only the person can tell them apart. A
  stick that reads as one already bound to another place, and holds
  something to tell it by, is refused: "This is the stick for Vault 1
  (Place 1)". Two vaults kept at one place may share a stick: asked for
  Vault 2 (Place 1), the stick bound to Vault 1 (Place 1) is accepted
  with a tap, **One stick for both**.
- **What is kept of each stick**: its label, FAT volume serial, listing
  (names and sizes), the SHA-256 and salt of each vault file, its free
  space, and its binding (place, vault, existing file or new).
- **Pull it**, then the next place.

### 4.3 Unlock and make the vaults

With no stick in (a stick inserted opens decision 8's Pull sheet,
"Pull the stick to unlock"):

- each existing vault chosen: **Unlock {vault}**, its passphrase, with
  Unlock's memory check; a vault kept at several places is unlocked once;
- each new vault: Create a vault (decision 21's Size, Name and
  passphrases), its own passphrase, once per vault of the plan;
- **I do not have its passphrase**, under an existing vault: **A new
  vault beside it** (the plan's vault becomes new, written beside that
  file at step 6) or **Use another stick** (back to step 2 for that
  place, which locks first when a vault is already open).

### 4.4 Make the key

The kind's own form (§3), then **Make the key**: made once, and saved
into every vault the plan keeps it in, in one action; the Result shows
what was made and, a row per vault, "In Vault 1", "In Vault 2". For a
wallet, New key makes each seed into memory and no vault is written to
until Back up's plan says which seeds go into which vault (§10.2); a key
kept only on paper is made and shown for copying, as today. Restore
takes the seed typed or scanned, into memory the same way. A Nostr key needs
its seed loaded: the unlock of step 3 loads the keys its vaults hold;
any other is loaded as a key is (Add a key), and the npub is compared
with the one chosen before the lock.

For a wallet, the rest of Make a new wallet, the rest of Back up's plan
(paper, SeedQR, the sheet and shares, the descriptor's form and
software) and the checklist's items follow, unchanged, with the keys in
memory (§10).

### 4.5 Lock once

Only when the plan writes any stick. The **Lock** card: the lock
sheet's sections, **Kept** then **Wiped from memory** (decision 28), and
**Lock and write**. Every open vault is sealed For the stick and
everything secret wiped; the app starts again clean, on step 6. A plan
with no stick skips steps 5 and 6.

### 4.6 Write each stick by place

One card per stick, in place order, then the stick of files, then the
stick of public files:

- **Insert** "Insert the stick for Vault 1 (Place 1)", with a row per
  file it will hold: "{vault file} · {size}", the public files.
- **Checks** on arrival:
  - a stick bound at step 2 must read as it did then (label, serial,
    listing, the vault files' SHA-256): else "Not the stick read for
    Vault 2 (Place 2)", and **Use another stick**, which checks a new
    stick as step 2 does and writes the vault beside what it holds;
  - a vault's second or later copy: refused on a stick that holds an
    earlier copy ("This stick has the first copy"), then **Hold: this is
    a second stick**, or for a third and later copy **Hold: this is
    another stick** (the place named in the record);
  - a stick that holds a file this flow wrote for another stick (by
    SHA-256): "This stick has Place 1's files";
  - a stick the plan named after step 2 (the stick of files, the
    stick of public files, a place added later): step 2's
    usability checks and room now, then the tap **This is {stick}**;
  - two sticks in: "One stick at a time".
- **Writing**: "Writing {file}", DESIGN §4.14's Progress where the write
  goes in parts (`docs/LARGE-FILES.md`).
- **Written**: "Written and checked on {label}" in `OK`, the files and
  the stick's row ticked, and each checklist item whose file it was
  ticked "Written to {stick}"; **Pull the stick**.

The bytes are For the stick's sealed copy. Each file is
`StorageCommand::Write`, read back and compared, as a visit writes it;
a vault replaces the file of its salt (VAULT §6) or goes beside under a
free name; other files follow **Already on the stick**'s rules
(decision 58) with Keep both for a file of another content under the
same name. Each answer adds to the receipt (decision 25), one block per
stick.

### 4.7 Done

A Result: "Written" in `OK`, a row per stick, "Vault 1 (Place 1) ·
{label} · {time}". **Unlock {vault}** opens Unlock with the vault picked
and lands on the key's page, where it signs; for a wallet, the
checklist's done card (NEW-WALLET §14.6), with nothing For the stick.

### 4.8 Changing the plan

(owner, 2026-10-14.) **Change the plan** is under every card, at every
step (NEW-WALLET §14.4). Sticks are asked for only once the plan says
how many places there are and which keep a vault's stick; nothing is
inserted before step 2.

**Adding a place after the key is made:**

- **A new vault on an empty stick**: no extra round. The vault is made
  in memory (Create a vault, its own passphrase) and the key saved into
  it. Its stick is checked (format, room, empty) when it is inserted at
  step 6; one that fails offers **Use another stick**.
- **A copy of a vault already in the plan**: no extra round; one more
  stick at step 6, with the second-copy rules.
- **A stick holding an existing vault the key should go into**: one
  guided extra round, inside the flow, on its own card, **Add {vault}**:
  1. **Lock**: the key's vaults are sealed and kept For the stick, as
     the Outbox is kept; everything secret wiped;
  2. **Insert** that stick, its checks, "Which vault?", the vault copied
     in, the place confirmed (§4.2); pull it;
  3. **Unlock** that vault and one vault holding the key; the key is
     copied across (a wallet's seed by **Save into {vault}**, with its
     passphrase where the plan puts it there; another kind's record as
     it is);
  4. **Lock**, and the flow goes on at step 6.
- **A key kept only on paper** does not survive that lock. Before the
  round the flow offers both, in this order: **Finish its paper copy
  first** (the Copy item and F29's check, then the round, the key
  typed back in from the copy at its unlock), and **Keep it in a vault
  for the round** (a vault made in memory with its own passphrase,
  sealed at the lock, unlocked in the round and dropped after it, never
  written). The first is chosen.

**Removing a place:**

- **Before its write**: the key is taken out of that place's vault in
  memory; a vault that was new and held only this flow's keys is
  dropped; the stick leaves step 6.
- **After its write**: the copy stays on the map as what it is until
  **I destroyed this copy** (NEW-WALLET §14.4 item 1, §9.4).
- **The minimums hold**: an APK, GPG or Secure Boot plan keeps two
  places; the control to remove a place is not drawn at two. A wallet
  may go to one place, with §10.3's warning; a Nostr key to none.

## 5. Screens, progress, leaving

- **Step cards** on `wide`, as Make a new wallet's (DESIGN §4.14;
  decision 16); **pages** on `small`, the head reading "Step 2 of 7" with
  a row per earlier card, "{card} · {value}". Steps 2 and 6 hold one
  sub-card per stick, each closing with its place and label when done.
  The cards a plan skips (2, 3, 5, 6 with no stick) are not drawn and
  not counted.
- **Inside a card**: Insert is the medium's icon and the line centred;
  the vault choice is a Choice; the hold is the Hold layout (the record
  centred, the hold at the bottom with **Use another stick** beside it);
  Writing is the Progress bar; Done is a Result.
- **On the Pi** "stick" is "SD card" by `app.medium` (F4). **A developer
  build of Faraday Desktop** (F30) uses its stick folders as sticks; the
  demo makes none of these keys (F30) and never shows the flow.
- **Never the stick visit**, at any step, 2 and 6 included. While the
  flow is on screen and not left, a stick arriving is the flow's:
  `stick_arrived` neither moves to `Screen::Visit` nor sets the visit's
  defaults; at steps 3 and 4 it brings the Pull sheet; on the Lock card,
  no Lock sheet, since **Lock and write** takes it.
- **Leaving.** Back, Escape or a sidebar item leaves on any card but
  while writing, where pulling the stick is the way to stop. Leaving
  keeps the state. Before step 4 the tile, or Make a new wallet, reads
  **Continue** and resumes; after it, **Continue the backup** on the
  key's page (the **Backup** row, §8), on Home as its lead ("Continue
  the backup of {key}"), and in place of a waiting action. After
  leaving, a stick goes to the stick visit as any stick does, and the
  flow's files For the stick are listed there tagged **In the backup**,
  not tickable.
- **Coming back with a vault open again** (the person unlocked after
  leaving step 6): the flow comes back on **Lock**; written sticks stay
  written.
- **After a lock**, a process with a flow not left opens on its step.
  While a flow waits for a stick there is no idle power-off; a flow left
  is listed with what is waiting.

## 6. Worked examples

**A. The APK key, one vault at two places** (the default).
Where: Vault 1 new, its stick at Place 1 and Place 2 → Insert the stick
for Vault 1 (Place 1): empty, bound, tap → pull → Place 2: empty, reads
as Place 1's did, so **Hold: this is the stick for Vault 1 (Place 2)** →
pull → Create Vault 1, its passphrase → name, package, **Make the key**
→ Lock and write → Insert the stick for Vault 1 (Place 1), writing, pull
→ Place 2: the first stick refused; the other, hold "This is a second
stick", writing, pull → Done → Unlock Vault 1 → the key's page; it
signs.

**B. A single key**: the seed in Vault 1 with its stick at Place 1 and
Place 2, the descriptor on the stick of files. Create: kind → Use
vaults: Yes → Number of vaults: 1 → insert the stick for Vault 1 (empty,
bound, tap) → Create Vault 1 → New key, made into memory → Check →
Back up's plan: the seed in Vault 1, its stick at Place 1 and Place 2,
the description as files → checklist: Vault 1 (**Save into** Vault 1),
Public files (**Make them**), Envelopes → **Write to sticks**: Lock and
write → Vault 1 (Place 1), the stick read for Vault 1 → Vault 1 (Place
2), a new stick checked when it is inserted, with the second-copy rules
→ the stick of files: checked, tap, `savings-descriptor.txt` written →
Done → Unlock → the done card.

**C. A 2-of-3, each key in its own vault.** Vault 1 an existing vault
on one stick; Vault 2 and Vault 3 new, on empty sticks. Use vaults: Yes
→ Number of vaults: 3 → insert the stick for Vault 1: it holds `main.ofv`
and `main-2.ofv`; "Which vault is Vault 1?" → `main.ofv`, copied in;
tap → the stick for Vault 2: empty, checked, bound; tap → the stick for
Vault 3: empty, reads as the second did, so the hold → every stick is
read → Unlock Vault 1; Create Vault 2 and Vault 3, each its own
passphrase → New key three times, into memory → Back up's plan: Key 1
into Vault 1, Key 2 into Vault 2, Key 3 into Vault 3, a place each → the
rest of the plan and the checklist, each vault's **Save into** → Lock
and write → the stick for Vault 1 (Place 1): `main.ofv` replaced,
`main-2.ofv` untouched → Vault 2 (Place 2) → Vault 3 (Place 3) → Done.

**D. A 2-of-3 mix.** Two vaults set up, Vault 1 and Vault 2, on a stick
each; Key 3 on paper. The setup asks for two sticks only → Unlock or
create Vault 1 and Vault 2 → New key three times, into memory → Back
up's plan: Key 1 into Vault 1 and Key 2 into Vault 2, their sticks at
Place 1 and Place 2, Key 3's words at Place 3 → checklist: Templates
(printed from Faraday Starter), Copy of Key 3 (F29's typed check), Vault 1
and Vault 2 (**Save into**), Sheets, Envelopes → Write to sticks: Vault 1
(Place 1), Vault 2 (Place 2), the stick of public files (the sheets as
PDFs) → Done. Key 3 is held until its copy is checked; Keys 1 and 2
until their sticks are written.

**E. Paper only.** No vault, no stick: steps 2, 3, 5 and 6 are not
drawn; the keys are made and copied, the checklist as today.

## 7. The state

`faraday-core/src/backup_flow.rs`, `BackupFlow`;
`Faraday::backup_flows: Vec<BackupFlow>`, at most one on screen.

| Field | Holds |
|---|---|
| `kind`, `subjects` | the kind, and each subject by §3's public value once made |
| `plan` | for a key, its places and vaults (§2); for a wallet, the wallet's checksum, the plan's answers being in Back up's state and, once written, the type 11 record |
| `vaults` | each vault: existing (file name, salt, length) or new (its name), its places |
| `sticks` | each stick: its name (vault and place, or the stick of files, the stick of public files), what step 2 read (label, serial, listing, each vault file's salt and SHA-256, free space), its binding, the files it will hold, and how it is confirmed |
| `sealing` | per vault, the SHA-256 of the sealed bytes the flow writes |
| `written` | per stick written: each file's name and SHA-256, the label, the time |
| `step`, `left`, `back` | where the flow is, whether the person left it, and where the last unlock lands |

**Kept** under the key `backup-flow` in `/run/faraday`, a line per
field, split by tabs, written at every lock and read back by
`restore_kept` (`memory.rs`), as the receipt and the vault summaries
are. **Gone at power-off.** Nothing in it is secret: names, sizes,
salts, hashes, labels, fingerprints, the npub. It says which keys were
made this session and where they are kept, in RAM until power-off, as
the summaries do (VAULT §11). Nothing of it is written to a stick or a
file For the stick.

**What counts as written**: only the flow's own step 6 write, read back
and matched. A write made on the stick visit page does not count
(decided, §15 item 4).

## 8. What waits, and the key's page

- **The key's page** gains the row **Backup**: "Not written", "Written
  to {n} of {m} places", "Written to every place", with **Continue the
  backup** until done. A key unlocked from a stick has no row.
- **While a waiting kind's writes are not done**, its waiting actions
  (§3) read **Back up first** and open the flow on its step.
- **Once every place the plan gives the key is written**, they are
  themselves again, from the unlock **Done** leads to.
- **Nostr** has the row and the flow, and nothing waits.
- The gate reads the flow's state only; the vault carries no mark of
  the flow (§15 item 2).

## 9. What each event does at each step

| Step | A lock | Power-off | Stick pulled | Wrong stick | A copy of the vault not seen |
|---|---|---|---|---|---|
| 1 Where | locks as any lock; the flow resumes on Where | gone; nothing was made | — | — | — |
| 2 Bring in | nothing open to lock | gone; nothing made, nothing written | before the place is confirmed: nothing bound | bound to another place: refused; unusable: why and what to do | differs from a copy read: the person picks; the other replaced only after a hold |
| 3 Vaults | the vaults lock; resumes on Vaults, unlocking again | gone; new vaults never sealed are lost | a stick in brings the Pull sheet | — | — |
| 4 Make | the vaults seal with the key in them, For the stick; resumes on Lock, the key made | gone, the key with it; the Power sheet says "nowhere else" | Pull sheet | — | — |
| 5 Lock | is the step; resumes on step 6 | gone, the sealed vaults with it | — | — | — |
| 6 Insert | nothing open | gone; sticks written hold the key | — | "Not the stick read for …", the first copy, another stick's files: refused | — |
| 6 Hold | — | gone | back to insert, nothing written | — | — |
| 6 Writing | — | gone; a part file VAULT §6 step 4 settles | "Pulled before the copy was checked. Nothing counted", back to insert | — | — |
| 7 Done | — | the copies are on the sticks | — | — | — |

- **After a power-off** nothing of the flow is left, and a key unlocked
  later from a stick signs at once. A power-off between writes leaves
  the copies written so far: the person chose the places, and can write
  the rest or stop there (decided, §15 item 2). The Power
  sheet lists it first, in the danger tone: "Backup of {key}: written to
  {n} of {m} places".
- **A vault changed after a stick was written** (left, unlocked,
  changed, locked): `sealing` takes the new bytes; written sticks stay
  written for every key their sealing held, and show "Changed since
  written" as any copy does.
- **A stick's first copy deleted** (decision 54, a wipe): that stick's
  card opens again, "The stick for Vault 1 (Place 1) no longer has the
  vault".
- **Changing the plan part way**: §4.8. A lock during the extra round
  resumes on its card; a power-off ends it, the key's vaults sealed For
  the stick lost with the rest of `/run/faraday`.

## 10. The Bitcoin wallet

### 10.1 What the code does today

- **The checklist** (`plan::checklist`, `plan::Item`): Templates, a Copy
  per paper seed, a Vault per vault made, SeedFiles, Sheets,
  PublicFiles, ShowDescriptor, Envelopes, each only where the plan
  needs it.
- **Four end in For the stick**: Templates and Sheets (**Make it**,
  `Action::BOut`), SeedFiles (`Action::BFile` through the secret
  sheet), PublicFiles (`Action::BFilesMake`); `backup_item_done` counts
  them done once the file is For the stick or on the last receipt.
- **The vault items write nothing**: `item_action` gives **Make a
  vault** or the way to one (`vault_way`), **Save into {vault}**
  (`Action::BVaultSave`), **Lock it and make a new vault**
  (`Action::BNewVault`, sealing that one vault with `vault_lock_one`),
  then **Continue**; done by `backup_vault_fits`.
- **The writes are left to the stick visit**: the done card's **Write
  to {a stick}** and the chart's stick node (`chart_edit.rs`,
  `glance_sheet.rs`) send `Action::WriteAsk`: with anything open the
  Write out sheet, whose **Lock** sends the next stick to
  `Screen::Visit`; clean, `Screen::Visit` at once.
- **The plan comes after the keys** (decision 20): New key makes the
  seed in memory; a vault item saves it later; F6's hold lifts when
  every item is done, before anything is written.

### 10.2 The change

(Every bullet below keeps what the plan can say: §2.)

- **Make a new wallet** asks about vaults after its kind and shape and
  before Keys, and its plan is made in Back up after the keys (owner,
  2026-10-15). In order:
  1. **Whether vaults are part of the backup.** A question with Yes and
     No: "Will you use vaults as part of this backup?". It is not done,
     and shows no check, until the person has answered it and pressed
     Continue. No goes on to Keys.
  2. **How many vaults.** A count of one to 1024 (the most a saved plan
     reads back); the count refuses more. Not done until its own
     Continue, also after the wallet is made.
  3. **Each vault set up**, one at a time, in this flow and never on the
     stick visit page: **Bring in the sticks** and **Vaults**, as §4.2
     and §4.3 describe them. A stick for each vault is inserted with
     nothing secret open and checked (FAT32, room, writable); a stick
     that holds vaults offers which existing vault this is or **A new
     vault beside them**, an empty one a new vault. Every stick is read
     before any vault is unlocked. Then the existing vaults are unlocked
     with their passphrases and the new ones made with Create a vault's
     own form (name, passphrase twice, cost and memory check). The setup
     asks nothing about places, which key goes into which vault, or paper,
     and its sticks are named "the stick for Vault 1".
  4. **Keys**, made into memory, as before. No key or wallet description
     is written to any vault by the setup.
  5. **Back up's plan**, after the keys, as it was before F31: which
     seeds go into which vault, how many places, the paper forms, SeedQR,
     files, the passphrase, the wallet description, the software. The
     vaults set up in step 3 are the plan's vaults, numbered in the order
     they were set up; the plan offers at least that many vaults, however
     many seeds there are, and shows each by its name. The checklist's
     vault item saves into the vault set up for it.
  6. **The checklist and Write to sticks**, as §16 B1b built them: one
     lock, each stick by place. A plan that keeps a vault at more places
     than the one stick it was set up on writes the extra copies at the
     write step, to new sticks checked when they are inserted, as an
     added copy is (§4.8). A plan that wants a vault that was not set up
     takes the extra round of §4.8.
  The stick read for a vault goes to the first place the plan gives that
  vault's stick; a stick the plan then gives no vault is not written.
  Restore asks the same question and count, sets the vaults up before
  the seeds are entered, and has its plan in Back up after (§11).
- **New key** makes each seed into memory, with F6's flag set;
  **Restore** takes its seed into memory. Neither writes to a vault.
  `Item::Vault(v)` is done when the open vault set up for plan vault `v`
  holds what the plan puts in it, and its **Save into {vault}** puts
  there each seed, its passphrase where the plan puts it, and the wallet
  description where the plan puts it; `BNewVault` stays for a plan
  changed part way.
- **The sticks** of the plan, from its map, one per spot that is a
  stick, holding what the map puts there:
  - a stick per `VaultStick(v)` spot at a place (brought in at step 2),
    with no upper bound: a vault, or any one key's vault, may be kept at
    as many places as the person adds, one copy per place;
  - **the stick of files**, the `At::Files` spot: each seed's file where
    the plan keeps seeds as files;
  - **the stick of public files**: the public files where the plan keeps
    the wallet as files or gives the software its text form, and the
    sheet or shares as PDFs, to print. It goes to an online computer.
    Where the plan's `At::Files` spot holds public files, it is that
    spot's stick; otherwise a stick of its own.
  The last two are named after step 2 and checked at step 6.
- **The public files** are those of `backup_public_names_for`, with the
  account keys added (owner, 2026-10-14; FLOWS decision 6 lists them):
  the descriptor, the multisig config, Sparrow's wallet file, the BSMS
  record, Bitcoin Core's import, as the software and form chosen make
  them, their pictures where the form is QR, and for each key loaded
  here its account key, `xpub-{fp}.txt`, and on a `wsh` or `sh(wsh)`
  multisig at Create's own account its BIP 129 key record,
  `xpub-{fp}-bsms.txt` (as `wallet_key_out` and `wallet_key_bsms` make
  them; the record is signed by the key, so it is made before the
  lock). A cosigner's key not loaded here has neither; its account key
  is in the descriptor.
  A 2-of-3 `wsh` with its three keys made here, Sparrow chosen, the text
  form and the whole sheet at each place writes to the stick of public
  files: `savings-wallet.json`, `xpub-{fp}.txt` and `xpub-{fp}-bsms.txt`
  for each of the three keys, and `savings-backup.pdf`.
- **Which stick the public files share is the person's choice**
  (owner, 2026-10-14). Where the plan's map puts them on a stick that
  also keeps a vault copy (a chart edit, NEW-WALLET §9.7) or a seed's
  file, the flow writes them there, with no warning and nothing
  blocked. The backup's Learn page states, as a fact, that a stick of
  public files is read on the online computer that runs the software or
  prints the sheets, and that whatever else is on that stick goes there
  with it (§15 item 6).
- **A new last item, `Item::Sticks`, Write to sticks**, after Envelopes,
  in any plan with a stick to write: its body a row per stick, "Vault 1
  (Place 1)", "Stick of files · 1 file", each ticked as written, its
  files under it; its foot **Lock and write** (step 5), then step 6 on
  the Back up screen. The items that end in For the stick are done when
  their file is made, as now, and read "Written to {stick}" once it is.
- **F6's hold at the lock**: every other item is done when **Lock and
  write** is offered; the lock clears the held flag (VAULT §7, type 2
  bit 1) in the vaults it seals, so the copies on the sticks say the
  backup is done, and the hold moves to the flow's state. A held key is released when every stick of the plan is written and each of the seed's own planned sticks is among them, non-empty; a key the plan keeps only on paper is released when the other items are done.
- **The done card** loses **Write to {a stick}**: nothing the plan made
  is left For the stick. A written file has left For the stick, so the
  Outbox never holds it twice; one taken by two sticks leaves once both
  are written. The chart's stick node keeps `WriteAsk`, outside the
  checklist.
- **The blank template** is printed from Faraday Starter (owner,
  2026-10-14; `docs/STARTER.md` §3.8), on the online computer, before
  the copies by hand. The checklist's **Templates** item no longer makes
  a file: its body reads "Print {n} from Faraday Starter: Blank
  templates, {words} words, {copy}" and its foot is **Done**, a press,
  as Envelopes' is. No template is written to a stick, and
  `blank-template-{words}-words.pdf` is no longer made here.

### 10.3 One place, and the passphrase

(owner, 2026-10-14.) One backup place is allowed for a wallet, and
nothing here forces a choice; two lines of fact say what the plan
gives.

**What NEW-WALLET's plan does today.** `Answers::defaults` gives a
single key two places and puts a seed's passphrase (`pass i`) at the
first place that keeps none of its words; `set_places` never lets a
passphrase share a place with its words on paper. A vault's stick is
not counted: with the Paper and vault preset on a single key, the words
are at Place 1, Vault 1's stick at Place 2, and the passphrase on paper
also at Place 2, beside the vault that holds the seed. "One place found:
can spend" reports a place that alone gives a spend; nothing names the
case of every secret at one place, and a single key with no passphrase
gets no word about it. §3.6 offers **Add a passphrase** on Create's
Keys card, for a key loaded from words without one.

**The one-place warning.** A new test, `plan::all_at_one_place(shape,
answers) -> Option<usize>`, gives the place, if any, that holds every
secret of the wallet: every seed loaded here, in any form (words,
SeedQR, a vault whose stick is there; a seed file, on the stick of
files, is at no place and does not count), and every passphrase of those seeds (on
paper there, or in a vault whose stick is there). It fires on a
one-place plan, on every seed and passphrase at Place 1 beside other
places, and whenever a seed and its passphrase are at the same place.
A single key with no passphrase fires it at every place that keeps the
seed. When it fires, a fourth line, in `WARN`, under the three check
lines:

- `plan_all_at_one_place`: "Every secret of this wallet is at {}.
  Separate places are strongly recommended."

It is drawn by `glance::check_lines`, so it shows wherever the three
lines do: the plan's map panel, the done card's strip, the wallet card's and the open vault's strips
(NEW-WALLET §14.6), stacked as a fourth row on `small`.

**The passphrase line.** For a single-key wallet whose key has no
passphrase, a fifth line, in the body tone, under the warning, with its
action:

- `plan_passphrase_apart`: "A passphrase kept at another place than the
  seed is recommended."
- `plan_add_passphrase`: **Add a passphrase**.

**Add a passphrase** opens Create on the Keys card with the key in its
slot and §3.6's fields open; the key with a passphrase is another key
and the wallet another wallet, which the Keys card shows by its
fingerprint. The line never blocks Continue, the checklist or a spend.

**The default with a passphrase.** `Answers::defaults` and every preset
put a seed's passphrase at the first place that keeps no copy of that
seed in any form, its vault's stick included; with none, a place is
added. So Paper and vault on a single key
with a passphrase gives: words at Place 1, Vault 1's stick at Place 2,
the passphrase at Place 3. The form at each place stays the person's:
vault, paper, steel or other.

**No cap on places** (owner, 2026-10-14). `Shape::max_places` (today
the larger of the seed count and three) goes, and `set_places` keeps
only its lower bound of one: a wallet, or any one key of it, may be
kept at as many places as the person adds, a second place for one key
of a 2-of-3 included. One copy per place, with no upper bound.

**Nothing narrows the plan.** The person can still choose one place and
no passphrase, or a passphrase beside its seed, and sees the lines; and
public files on any stick, with no line.

## 11. Restore

Restore asks the same question as Make a new wallet (§10.2): will vaults
be part of this backup, and how many. With vaults, the vaults are set up
(steps 2 and 3) before the seeds are typed or scanned, which then go
into memory. Back up's plan, which says which seed goes into which vault,
comes after, as it does for Make a new wallet. A restored key signs at
once (F6); the flow is offered and nothing waits for it.

## 12. Strings

In `faraday/faraday-strings/src/en.rs`, prefix `backup_flow_`. `{m}` is
the medium (`app.medium`), as `write_out_*` take it; a stick's name is
`{stick}`.

| Name | Text |
|---|---|
| `backup_flow_card_where` | Where it will be kept |
| `backup_flow_card_sticks` | Bring in the {m}s |
| `backup_flow_card_vaults` | Vaults |
| `backup_flow_card_make` | Make the key |
| `backup_flow_card_lock` | Lock |
| `backup_flow_card_write` | Write the {m}s |
| `backup_flow_card_done` | Done |
| `backup_flow_add_place` | Add a place |
| `backup_flow_place_keeps` | A {m} with |
| `backup_flow_new_vault` | A new vault |
| `backup_flow_existing_vault` | An existing vault |
| `backup_flow_vault_new` | {} · new |
| `backup_flow_stick_for` | the {m} for {} ({}) (vault, place) |
| `backup_flow_files_stick` | the {m} of files |
| `backup_flow_public_stick` | the {m} of public files |
| `backup_flow_lock_bring` | Lock and bring in the {m}s |
| `backup_flow_insert` | Insert {} |
| `backup_flow_use_read` | Use {}, read at {} |
| `backup_flow_one_at_a_time` | One {m} at a time |
| `backup_flow_not_fat` | Not a FAT32 {m}. Format it as FAT32, or use another {m} |
| `backup_flow_boot_part` | This is a boot partition. Use another {m} |
| `backup_flow_read_only` | Cannot be written. Use another {m} |
| `backup_flow_no_room` | Not enough room: {} needed, {} free |
| `backup_flow_which_vault` | Which vault is {}? |
| `backup_flow_beside` | A new vault beside them |
| `backup_flow_two_copies` | {} has two copies of {}: {} and {} |
| `backup_flow_copies_differ` | {}'s copy of {} differs from {}'s |
| `backup_flow_this_is` | This is {} |
| `backup_flow_hold_this_is` | Hold: this is {} |
| `backup_flow_bound_to` | This is {} |
| `backup_flow_both` | One {m} for both |
| `backup_flow_pull` | Pull the {m} |
| `backup_flow_no_passphrase` | I do not have its passphrase |
| `backup_flow_beside_it` | A new vault beside it |
| `backup_flow_use_another` | Use another {m} |
| `backup_flow_in_vault` | In {} |
| `backup_flow_lock_and_write` | Lock and write |
| `backup_flow_add_vault` | Add {} |
| `backup_flow_finish_paper` | Finish its paper copy first |
| `backup_flow_vault_for_round` | Keep it in a vault for the round |
| `backup_flow_not_read` | Not the {m} read for {} |
| `backup_flow_has_first` | This {m} has the first copy |
| `backup_flow_hold_second` | Hold: this is a second {m} |
| `backup_flow_hold_another` | Hold: this is another {m} |
| `backup_flow_has_files_of` | This {m} has {}'s files |
| `backup_flow_writing` | Writing {} |
| `backup_flow_written` | Written and checked on {} |
| `backup_flow_written_to` | Written to {} |
| `backup_flow_pulled` | Pulled before the copy was checked. Nothing counted |
| `backup_flow_not_checked` | The copy did not check on {}. Nothing counted |
| `backup_flow_first_gone` | {} no longer has the vault |
| `backup_flow_done` | Written |
| `backup_flow_row` | Backup |
| `backup_flow_row_none` | Not written |
| `backup_flow_row_some` | Written to {} of {} places |
| `backup_flow_row_all` | Written to every place |
| `backup_flow_continue` | Continue the backup |
| `backup_flow_continue_make` | Continue |
| `backup_flow_first` | Back up first |
| `backup_flow_home_lead` | Continue the backup of {} |
| `backup_flow_power` | Backup of {}: written to {} of {} places |
| `backup_flow_in_the_backup` | In the backup |
| `backup_flow_item_sticks` | Write to {m}s |

The plan's lines of §10.3 are the plan's own strings: `plan_all_at_one_place`
"Every secret of this wallet is at {}. Separate places are strongly
recommended.", `plan_passphrase_apart` "A passphrase kept at another
place than the seed is recommended.", `plan_add_passphrase` "Add a
passphrase", and `item_templates_starter` "Print {} from Faraday
Starter: Blank templates, {} words, {}" for the Templates item.

**Unlock {vault}** and Create a vault's strings are reused. APK batch
1's `apk_back_up_first`, `apk_backup_line`, `apk_backup_line_once`,
`apk_same_stick`, `apk_back_to_first`, `apk_signs_from_now` go (§14).

## 13. Tests

User-facing behaviour, through `Action`s and the test kit's sticks, in
a new `faraday-core/tests/backup_flow.rs`; the wallet's in
`tests/backup_checklist.rs` and `tests/backup_first.rs`.

- **The order.** An APK, GPG and Secure Boot Make starts on **Where it
  will be kept**; Make the key is not offered before its sticks are
  read and its vaults open; with a vault open, step 2 locks first.
- **Step 2.** An empty stick is bound to the plan's new vault with no
  question and offered no copy; a stick holding `vault.ofv` and
  `vault-2.ofv` at a place the plan gives an existing vault asks which,
  copies in only that one, and step 6 leaves the other byte for byte; at
  a place given a new vault, **A new vault beside them** is chosen; a
  stick bound to Place 1 inserted for Place 2 is refused; a second
  empty stick of the same label and serial needs the hold; a non-FAT,
  read-only, full or `OSKBOOT` stick says why and binds nothing.
- **Steps 3 and 4.** New vaults are made with their own passphrases; the
  key is in every vault the plan keeps it in after one **Make the key**;
  **I do not have its passphrase** offers a new vault beside it.
- **Two writes end it.** Example A completes: both sticks hold the same
  sealed bytes; the key's page reads "Written to every place".
- **The second copy.** The first stick put back for Place 2 is refused,
  nothing written; a stick unlike the one read for Place 2 is refused,
  "Not the stick read for …"; nothing is written before the hold.
- **Never the stick visit.** At every step of examples A to D, sticks
  inserted and pulled, the screen is never `Screen::Visit`, and no Lock
  or Write out sheet comes up.
- **A lock resumes at the right step.** An idle lock on Where, on
  Vaults, on Make with the key made, and the kept state read back after
  the first stick of step 6, each reopen on the right card; the last on
  the next stick not written.
- **What waits.** A key made in the flow: Sign an APK, GPG's Sign a file
  and Renew, Secure Boot's Enrolment files and Sign with db read **Back
  up first** until every place is written, then work after the unlock;
  the same keys unlocked from a stick work at once; a Nostr entry saved
  in the flow exports at once.
- **Every form.** Example C writes three sticks, `main-2.ofv` untouched;
  example D asks for two sticks only, checks Key 3's copy by typed
  words, and writes Vault 1, Vault 2 and the stick of public files;
  example E draws no stick step and no lock.
- **Owner's answers.** A vault kept at four places asks for four sticks,
  the third and fourth with **Hold: this is another stick**; a 2-of-3
  with a second place for Key 1's vault writes that copy too; the plan
  accepts a fifth place on a single key. The 2-of-3 `wsh` of §10.2
  writes `savings-wallet.json`, three `xpub-{fp}.txt`, three
  `xpub-{fp}-bsms.txt` and `savings-backup.pdf` to the stick of public
  files, and no vault copy goes on it. A map with public files on a
  vault's stick writes them there, with no warning line. The Templates item writes no file
  and is done by its press. A power-off after Place 1's write, then an
  unlock from Place 1's stick: the key signs at once and no flow is
  offered.
- **Places.** An APK, GPG or Secure Boot plan cannot go below two
  places; a Nostr plan of no place saves into the open vault and writes
  no stick; a wallet plan of one place is accepted, with the warning
  "Every secret of this wallet is at Place 1" under its check lines in
  the map panel and on the done card; a two-place single key with seed
  and passphrase at Place 1 shows it, and with them apart does not; a
  single key with no passphrase shows the passphrase line, whose **Add a
  passphrase** opens Create on Keys with the passphrase fields open;
  Paper and vault with a passphrase puts the passphrase at a third
  place by default; none of the lines blocks Continue or the checklist.
- **The checklist writes inside the flow.** In example B the plan's
  last item is **Write to sticks**; each stick's row, each file and the
  Public files item tick only when their stick's write is checked; after
  Done nothing is For the stick; a stick visit while the flow is left
  lists its files **In the backup**, not tickable; a plan with seed files
  and Sparrow writes the software's files on a stick of their own.
- **Changing the plan** (§4.8). After the key is made: adding a place
  with a new vault on an empty stick asks for no lock before step 6, and
  the stick is checked there; adding a copy of a vault in the plan adds
  one stick to step 6; adding a stick holding an existing vault runs
  **Add {vault}**: lock, the stick read and its vault picked, both
  vaults unlocked, the key copied across, lock, step 6, with the screen
  never `Screen::Visit`; a key kept only on paper is offered **Finish
  its paper copy first** before that round. Removing a place before its
  write drops a new vault holding only this flow's keys; removing one
  after its write keeps it on the map until **I destroyed this copy**.
  An APK, GPG or Secure Boot plan cannot drop below two places.
- **Events.** A stick pulled while writing counts nothing; with no flow
  in the kept state (a power-off) no flow is offered.

## 14. Batch B1

After APK batch 1 (`docs/APK-SIGNING.md` §11) and the GPG fix batch
are committed. Renders at `1280x800` and `480x640`: Where, a stick
being brought in with the vault choice, the hold, Vaults, Lock, a stick
written, Done, the checklist's **Write to sticks**, and Make a new
wallet's new cards. It touches `faraday-core`, so the device app's
builds follow it.

**Adds.** `faraday-core/src/backup_flow.rs` (§7, the steps, the
checks); the cards (§4, §5) in one screen function used by the kinds'
Make screens and by Make a new wallet and Back up; the hooks in
`stick_arrived` and on a pull; the gate (§8) for the APK key, GPG and
Secure Boot, which no earlier batch builds; Home's lead; the Power
sheet's line; for the wallet, §10.2: Create's new cards, New key into
vaults, `Item::Sticks` in `plan::checklist`, its foot in `item_action`
and done test in `backup_item_done`, the "Written to" lines, the visit's
**In the backup** tag; §12's strings; §13's tests.

**Replaces, in APK batch 1** (`faraday-core/src/apk.rs`, `memory.rs`,
`vaults.rs`, `visit_taken.rs`, `lib.rs`, `stick_wipe.rs`):

- the placeholder `V::ApkWrite`, whose **Back up first** and **Write
  the vault to two sticks** lock the vault and open the stick visit with
  the vault ticked;
- Make an APK key's form with its vault buttons: Make is step 4, the
  vault decided at steps 1 to 3;
- `ApkState::{waits, visit_for, back, sealed}`, `Wait`, `First`,
  `waits_encode`, `waits_decode`, the kept keys `apk-waits`,
  `apk-visit`, `apk-back`; `apk_waiting`, `apk_second_refusal`,
  `apk_inbox_ticks`, `apk_written`, `apk_sealed`, `apk_first_gone`,
  `apk_return`, and their hooks;
- the strings of §12's last paragraph; `tests/apk.rs`'s two-stick
  tests, which move here.
- `vault_is_this_sessions` and `apk_vault_not_replaced` stay: they guard
  the ordinary visit's copy of a vault from Files.

**Replaces, in the GPG fix batch:** Make a key's vault step and its
place in Vault contents (now step 4 of the flow); **Sign a file** and
**Renew** gain the gate. **Secure Boot:** Make keys becomes step 4; the
gate on Enrolment files and Sign with db. **Nostr** (N1, not built):
whichever of N1 and B1 is second makes Save into a vault the flow.
**Decision 10** no longer applies to the GPG, Secure Boot and APK tiles.

**Replaces, in the wallet:** the done card's **Write to {a stick}**;
Back up's vault part, which moves before Keys; F6's hold moves at the
lock; the passphrase's default place in `Answers::defaults` and
`Answers::preset` (§10.3); `glance::check_lines` gains §10.3's lines;
`Shape::max_places` and `set_places`' upper clamp go; the account keys
join `backup_public_names_for` and `backup_files_make`; the Templates
item's **Make it** (`Action::BOut(0)`, `blank-template-{words}-words.pdf`)
becomes the Starter line and **Done**.

**Batch T, Faraday Starter's blank templates** (`docs/STARTER.md`
§3.8, §11), its own small batch, built before B1 so the Templates line
points at a screen that exists: it touches only the Starter
(`faraday-starter`, `shells/starter`) and reuses `faraday-core`'s
`backup::sheet_blank` and `pdf::sheet` as they are. Its build is the
Starter's.

## 15. Decided

The owner's answers (2026-10-14).

1. **Places by kind**: two or more for APK, GPG and Secure Boot keys,
   with no single-copy hold; one or more for a wallet, with §10.3's
   warning; any number, none included, for Nostr.
2. **A power-off between writes**: no mark in the vault and no refresh.
   The flow's state is in `/run/faraday` only and is gone at power-off;
   a key in a vault unlocked from a stick signs at once. A power-off
   between writes leaves the copies already written: the person chose
   the places, and can write the rest or stop there.
3. **A copy of the same vault not seen this session, and different**:
   replaced only after a hold.
4. **Writes made on the stick visit page** do not count toward the flow.
5. **Places and copies have no cap**: any number of places, a vault or
   any one key kept at as many as the person adds, one copy per place.
   The only lower bound is two places for a key that cannot be made
   again from seed words.
6. **Account keys** are among the public files (§10.2). **The blank
   template** is printed from Faraday Starter (`docs/STARTER.md` §3.8);
   no template is written to a stick. Public files may share a stick
   with a vault copy, as the person chooses, with no warning. The
   backup's Learn page says, as a fact, that a stick of public files is
   read on an online computer with whatever else it holds: recommended,
   as the one place the person learns what that choice means, since no
   working screen explains it.
7. **A key kept only on paper before an extra round** (§4.8): both
   offered, **Finish its paper copy first** chosen.
8. **A stick holding vaults, at a place the plan gives a new vault**:
   **A new vault beside them** first and chosen.

## 16. Built (batch B1a)

What B1a built, and where the spec was silent or the build followed the
closest existing pattern (`gpg.rs`, the stick visit, the lock, the kept
state, NEW-WALLET's checklist). The wallet's part is batch B1b.

**Where it is.** `faraday-core/src/backup_flow.rs` (the state of §7, the
plan, the kept state, the gate of §8), `backup_flow_steps.rs` (the steps,
`FlowAction`, the checks) and `backup_flow_screen.rs` (the cards).
`Screen::BackupFlow` is the flow's one screen and `Action::Flow` its
presses. Step 3's Create a vault is `vault_screens::create`, drawn in the
flow's place with `Screen::BackupFlow` kept, so no other screen shows.

1. **The plan source.** A flow is built from a `Plan`: places, each with
   the vaults whose stick it keeps (none: no stick there), and the vaults
   (`VaultPlan`: planned existing or new, then its file, salt and length
   once known). `BackupFlow::new(kind)` fills it with the key's own plan,
   one new vault at two places; `BackupFlow::with_plan(kind, plan)` takes
   any other, which is how B1b gives a wallet's. Only places that keep a
   stick take part in steps 2, 5 and 6. Sticks named after step 2 (the
   stick of files, the stick of public files) are `Slot`s with
   `Target::Files` or `Target::Public` and a list of Outbox names; step 6
   checks them as step 2 does, takes the tap, and writes them by the same
   routine. Nothing makes one yet.
2. **One stick per vault, or one for both.** A place may keep several
   vaults. By default each is a stick of its own (`Target::PlaceVault`),
   asked for in turn; **One {stick} for both** on the place's card puts
   them on one stick (`Plan.together`, `Target::Place`), and **Also {vault}**, which adds
   {vault} here** adds a vault to a place. Nothing forces either. A key's
   own plan gives each place one vault; a place with "nothing" is not
   offered, since it would not count toward the two a key needs.
3. **The Tools tile.** With no vault open, or a flow waiting before its
   key is made, the tile opens the flow (**Continue** on the tile while
   one waits). With a vault open it opens that vault's category, where
   the keys and their actions are, and **Make a key** there starts the
   flow; a person with a vault open reaches an existing key without
   going through Where. Decision 10 no longer applies to these three
   tiles.
4. **Usable sticks (§4.2).** An `OSKBOOT` label is refused. The disk
   process lists every USB and SD device it is handed (a partition with
   no FAT volume, a disk with no partition, an empty reader slot not) and
   marks each with a `Problem` on the wire (`faraday-files::proto::Stick`):
   `NotFat`, or `ReadOnly` when the kernel's `ro` flag of the partition
   or its disk is set. The stick shell sends `StorageEvent::StickProblem`
   for each, and the flow shows it at step 2 with what to do. The privilege
   split is as `PLAN.md` has it: only the disk process opens a node, and
   the app only sees the answer. Room is asked with `StickSpace`; a new
   vault's length is counted at the default slot's file (a larger slot
   chosen at step 3 is checked by the write).
5. **Already read (§4.2).** **Use {label}, read at {time}** is offered for
   the stick the machine started from, at step 2 and at step 6; the reading
   of it ends at the bind, so nothing waits for it to be pulled, and at
   step 6 it is checked like any stick. Keys are still made with no stick
   in, the boot stick too. Sticks read earlier in the flow are not offered
   again.
6. **Which vault.** A vault the plan already knows (its salt is known from
   an earlier place) is never asked again: its copy on the next stick is
   written over, or the vault goes beside. Choosing a file at a place
   whose vault was planned new makes it that existing vault; the choice
   takes effect when the place is confirmed. Two files of one vault, and
   a copy that differs from the one first read (**Use {label}'s copy**,
   either), are as §4.2 says. A different vault already under that name
   in Files keeps its name; the copy comes in under a free one.
7. **Refusals at step 6.** A stick that holds a file the flow wrote for
   another stick is refused before it is compared with what step 2 read
   ("This stick has the first copy" for a copy of the same vault,
   "This stick has Place 1's files" otherwise); only vault files are read
   for that, since a stick of files holds no vault in B1a. "Not the stick
   read for Vault 1 (Place 2)" names the vault and place alone.
8. **The hold at step 6** is asked for every later copy of a vault
   (**a second stick**, **another stick** from the third), whether or not
   the stick looks like the first; the first stick put back is refused
   outright, since no hold would make that right.
9. **Make (§4.4).** The key is made once and saved into every vault of the
   plan in one action; if any vault cannot take it, none does. A GPG key
   is made at once (its toast says where the revocation is kept); the
   others on the tick after "Making the key" is drawn. The result shows
   the kind's own rows, its public file with **Show as QR** and **PNG**,
   and a row per vault. Make is refused until every vault of the plan is
   open, the clock is asked for first, and Escape does not leave the
   form.
10. **Step 3** unlocks the existing vaults first (the first one still
    locked takes the typing), then makes each new one. A vault made in
    the flow is unlocked at once with the passphrase just typed twice,
    where decision 10 types it once more.
11. **The lock and where a lock resumes (§9).** A key that is made when
    a lock comes resumes on step 6, since the lock was the step; before
    it, on Vaults. The state is kept under `backup-flow` with the flows'
    plans, sealings and written copies, and no secret. What was on screen
    when it was kept opens on return.
12. **The extra round (§4.8)** shares the steps: the flow is `adding`, step
    2 reads only a stick that holds an existing vault, step 3 asks for
    the added vault and one that holds the key, step 4 is **Copy the
    key**, then the lock and step 6. A new vault on an empty stick needs
    no stick read and no lock while the key's vault is open. The paper
    case is the wallet's (B1b).
13. **Removing a place** before its write drops a new vault that held only
    this flow's key and takes the key out of an existing one; after its
    write the control reads **I destroyed this copy** and the place stays
    until it is pressed. The minimums hold.
14. **The gate (§8).** A key made in a flow waits until every one of its
    sticks is written; the key's page has the row **Backup**, its waiting
    actions read **Back up first** and open the flow on its step, and
    Files' row for an APK a waiting key fits does the same. Sign a file,
    Renew, Enrolment files and Sign with db, and Sign an APK, wait; Check a
    signature does not. A key no flow made here signs at once.
15. **A written copy is deleted or the stick wiped** opens that stick's
    card again. Each written stick keeps its label, volume serial and the
    SHA-256 of its files in the state that survives a lock, and a wipe or
    delete is matched on those, never on "all". Two sticks that read
    exactly alike (one label, one serial, the same files) cannot be told
    apart: both open again.
16. **The stick visit** lists the sealed vaults a flow will write as
    **In the backup**, not tickable, and Back is not offered while a write
    is under way. An idle power-off waits while the flow is on screen.
17. **The receipt** is the last stick's, as the visit's is.
18. **The key's public files** (GPG `.asc`, APK certificate `.pem`, the
    Secure Boot enrolment files) are made with the key and written in the
    flow at step 6 to the stick of the place the plan names (**Public
    files here**, Place 1 by default; any place may), listed on that
    stick's card and in Done. They may share a stick with a vault. A
    Secure Boot key's enrolment files wait for the backup like its signing
    does (§3), so they are written in the last write of step 6, to
    whichever stick completes the backup, never the first when there are
    two places. After Done nothing the flow made waits For the stick: the
    sealed copies and the public files move to Files, and a copy deleted
    later is written again from there.
19. **A stick that returns mid-write.** A stick pulled while it was being
    written counts nothing. When it comes back, differences that are only
    this flow's own sealed bytes or its part file are accepted, a write
    that landed whose answer was lost is recognised and counted, and a
    file the stick already holds is written beside under a free name.
    A stick named at step 6 that holds another copy of a flow vault is
    not asked which copy to unlock (the key is made); that copy is
    declined, and it is written over only after the hold. The flow's writes
    are not an ordinary visit's: its receipt is marked, so a later visit
    never writes the flow's vault over another copy. A whole wipe counts
    as soon as the partition is emptied, whether or not it is then
    stopped; one that fails or is cut off before its first answer reopens
    them too. A partition the grant has not handed out yet waits and shows
    no problem; a stick whose room cannot be asked for says it cannot be
    read, with Use another stick; each listing clears and re-tells what is
    wrong with a stick. The visit lists a waiting Secure Boot key's
    enrolment files as In the backup, not tickable.
    Limit: a later copy's stick that reads exactly like the first stick
    is refused as holding the first copy.
20. **A copy that differs** from one already read, or from the one in
    Files, is asked about at every bind, the first place included. The
    copy not unlocked is marked declined on its slot, and writing that
    stick needs the hold (a second stick's, whatever it looks like).
21. **The kept state** has a version line (`version\t2`); it is validated
    after it is read (indices in range, slots as the plan makes them, from
    Lock on the key made, from Write on a sealed copy per vault). A flow
    that does not read is dropped with the toast **A kept backup did not
    read and was dropped**. A lock from the Set the time screen reached
    from Make, or before **Copy the key** in the added-vault round,
    resumes the flow (on Vaults when the key is not in every vault). A
    flow whose key is made is never dropped: what is wrong with its steps
    (a plan changed at step 6 and locked with no Continue) is repaired by
    sending it back to Vaults as an extra round. The keys that wait are
    also kept apart (`backup-gate`), so a kept state that cannot be read
    at all leaves each of them waiting, with a flow at Where to finish it
    (such a stub knows no vault, so it stays waiting until power-off; it
    fails closed). The gate is the first thing a save writes, whole (a
    temporary file, then a rename), the flows are restored before it, and
    a lock that cannot keep a waiting key's state does not go ahead.
22. **Who writes.** Write and Confirm act only on a clean process at step
    6; a reading ends when the step changes. A stick is written only when
    every vault of its slot has its sealing and its Outbox copy, and no
    file of the slot is a secret. The plan cannot be changed while a write
    is under way. A sealing is recorded only if the key's record is in the
    vault. Removing a place takes a new vault out of the plan and the
    session only if it holds nothing but this flow's key, and never the
    last vault that holds the key. The signing functions (`gpg_code` for a
    signature, `gpg_sign_ticked`, `gpg_renew`, `sb_enrol`, `sb_sign`) refuse
    a waiting key themselves, not only the screens that offer them.
23. **Not built in B1a:** the progress bar for a write (a vault is small),
    "Changed since written" for a vault changed after a write (the flow's
    writes stay counted), a Nostr key's part, and everything of §10 and
    §11 (B1b).

**B1b needs from this code.** `BackupFlow::with_plan(Kind::Wallet, plan)`
with a `Plan` from `plan::map`: its stick places with `vault v`'s indices,
`VaultPlan::existing` from the plan's choice; a `Kind::Wallet` with
`category`, `record_kind` and `subject_of` for a wallet (the descriptor's
checksum), and a result in `make_body` of its own; `Step::Make` replaced by
New key and the checklist, entered through `flow_enter`/`flow_settle`
(which advance Sticks, Vaults and Write); the Files and Public `Slot`s with
their Outbox names for the stick of files and the stick of public files;
and `Kind::min_places` returning one. The `Step`'s Done transition already
moves the sealed vaults into Files. The gate for a wallet is
`BackupFlow::waiting` read by `backup_waits`.

## 16. Built (batch B1b)

The wallet's part: §10 and §11, on the code B1a built. Where the spec was
silent, or two readings were open, the build followed the closest B1a
pattern; each choice is here.

**Where it is.** `faraday-core/src/backup_flow_wallet.rs` (what only a
wallet needs: its flow, the hold, the files, the round),
`backup_flow_wallet_screen.rs` (its Where and Make cards and Make a new
wallet's three), the shared `backup_flow.rs`, `backup_flow_steps.rs` and
`backup_flow_screen.rs` with a `Kind::Wallet` arm where a wallet differs,
`plan.rs` (the checklist's `Item::Sticks`, `all_at_one_place`, the
passphrase's default place, no cap), `glance.rs` (the two lines under the
checks) and `lib.rs`/`screens.rs` for the checklist and the cards.

1. **The plan is `plan_of(shape, answers)`**, the map's: a place for each
   place of the map keeping the vaults whose stick it holds; vaults
   numbered as the answers number them (a number no map uses is an empty
   `VaultPlan` that no place names, so `used_vaults()` never reaches it);
   the stick of files where the map has an `At::Files` spot with seed
   files, or holding the wallet; the stick of public files where nothing
   else carries them. The public files and sheets go (`PublicOn`): to the
   stick of a place that keeps a vault copy and also holds the wallet (a
   chart edit), else to the stick of files when it holds the wallet,
   else to a stick of their own. A plan whose checklist has no
   Sheets, public files, seed files or vault stick has no slot, so no
   stick step and no lock. The default Paper preset has a sheet, so it
   has the stick of public files and the lock; a plan with no sheet
   (the sheet row unticked), no vault and no files has none.
2. **The wallet's state in the flow** is `WalletFlow`: its shape (the
   quorum and the seeds by name), its answers (`Answers::to_text`, no
   secret), its descriptor checksum and name, the fingerprints it holds
   back, and the seeds each vault was sealed with. All of it is in the
   kept state (`version 3`) and the held fingerprints in `backup-gate`
   too, so a flow that does not read still holds its keys.
3. **The vaults are set up on the flow's own screen** for a wallet, and
   Make a new wallet hosts the setup (reworked 2026-10-15, §10.2): its
   card list gains **Use vaults** (the question, Yes and No, after Kind
   and Quorum), **Number of vaults** (a count of one or more) and, once the
   setup has begun, **Bring in the sticks** and **Vaults** as closed
   cards with **Change**. The question and the count are never done
   until the person has answered and pressed their own Continue; with No,
   Continue goes to Keys and no flow exists. The setup opens on **Bring
   in the sticks**, a stick for each vault named "the stick for Vault 1"
   (a page each on a small panel); when the vaults are open it returns to
   Keys. Restore has the same two cards before Seeds and returns to
   Seeds; for a wallet read from its description, the wallet is tied to
   the setup when the setup ends.
4. **Before the keys exist** the flow's plan is the setup's: Vault *v* at
   Place *v*, each new, whatever Back up's plan will be; its shape is a
   placeholder, a seed to be made here for each key ("Key 1"). When the
   wallet is made the shape is the real one, with the count of vaults set
   up (`Shape::vaults`, kept in the flow's state), and the answers are the
   wallet's defaults fitted to it (`Answers::refit`; a vault set up stays
   whether or not a seed is ticked into it, and the plan offers at least
   that many vaults, however many seeds there are). The sticks read in
   the setup are kept by vault (`BackupFlow::parked`, not kept across a
   lock) and go to the first place Back up's plan gives each vault's
   stick.
5. **No key goes into a vault until the checklist's vault item saves it.**
   The hook that saved each key into its vaults
   (`wallet_flow_save_keys`), the mark that asked for a passphrase before
   the key existed (`CreateState::pass_marked`), Back up's adoption of the
   flow's answers and the preset's keeping of the vault part are gone.
   Back up opens at its presets; the vault item's **Save into** puts in
   the vault set up for its number each seed, its passphrase where the
   plan puts it, and the wallet description where it goes there.
6. **The checklist's last item**, `Item::Sticks`, shows the lock's Kept and
   Wiped from memory and a row for each stick by place (the stick of files
   and of public files by name), each with its files, ticked as written.
   Its foot **Lock and write** is the lock: one press (the Lock card's
   sections are in the item, so the flow's Lock card is passed). It acts
   only with every other item done and every vault of the plan open and
   holding what the plan puts in it. The vault part of the plan is saved
   (`plan_save`) first, each vault is resealed (`changes` raised, so a
   copy for every stick is sealed), the vault's open index is the plan
   vault the flow knows, else the one that fits; the flow takes the
   files the checklist made. An item that ends For the stick is done when
   its file is made, as before, or written by the flow; its closed line
   says **Written to {label}** once it was. The item is done when every
   stick is written: the Back up screen, opened again from the vault after
   Done, shows it.
7. **F6's hold.** The held keys the plan keeps in a vault move to the
   flow's state at the press (`WalletFlow::held`), and the marks come off
   the vault records and the plan's `held` lines before the seal, so the
   sealed copies say the backup is done; a key the plan keeps only on
   paper is released when the other items are done, as its copy was
   checked; every other held key is released by the one rule: every stick
   of the plan is written, and the seed's own planned sticks are among them,
   non-empty. `held_keys` is the stored hold plus the flow's; a record
   saved while a flow holds a key says nothing of it.
8. **Seed files.** A seed's file let out unprotected on the secret sheet
   is written to the stick of files and to no other, as the one secret the
   flow writes; every other file of a stick still must not be one.
9. **The account keys** (`xpub-{fp}.txt`, and `xpub-{fp}-bsms.txt` on a
   `wsh` or `sh(wsh)` multisig at Create's own account) are among the
   public files: named by `backup_public_names_for` (without signing),
   made by `backup_files_make` with the other public files when the
   checklist is made, written to the stick that carries them. A key not
   held here has none; a single key has none.
10. **Templates.** The item's body says "Print {n} from Faraday Starter:
    Blank templates, {words} words, {Words|SeedQR|Both}" with the 24 or
    12 chosen, its foot is **Done** (`BackupState::printed`), and no
    template is made: `blank-template-{words}-words.pdf` and
    `public_made`'s arm for it are gone.
11. **§10.3.** `plan::all_at_one_place` (the three cases of §10.3) and
    `Check::{one_place, passphrase_apart}`; `glance::check_lines` draws the
    two lines under the three boxes wherever they are drawn, with the
    place's name from the chart's nodes or the vault. **Add a passphrase**
    after the wallet exists opens Create anew on Keys with the single
    key in its slot and the fields open: a new key and wallet. The
    passphrase's place is the first that keeps no copy of its seed in
    any form; where none, a place is added that keeps no paper
    (`Answers::bare`, a new `bare` line in the plan's text), since the
    spread would otherwise give every new place another copy of the
    words. `Answers::preset` places the passphrase after the vault's stick.
    A single key with Paper and vault and a passphrase: words at Places 1
    and 2, the vault's stick at Place 2, the passphrase at Place 3.
12. **No cap.** `Shape::max_places` is gone and `set_places` keeps its
    lower bound of one; so does `Answers::from_text`.
13. **Restore** takes the same question, count and setup before its
    seeds, its seeds into memory, then Back up's plan and the checklist. A restored key is no key made here, so nothing is
    held back and it signs at once; the flow is offered by the plan and
    the key's page shows the written count as any flow does.
14. **Change the plan** for a wallet edits the answers (`FlowAction::Answer`
    takes Back up's question and row), at every step; a stick already
    written is not taken off the plan from here. After the lock a vault
    that now holds a seed it was not sealed with is as one added: its
    written sticks are cleared, the round unlocks it and the vaults that
    hold the seed, copies the records across, locks and writes again. The
    round reads the key records of the open vaults; a seed kept only on
    paper is typed back (**Add the key from its copy**) and then saved.
15. **The done card** loses **Write to {a stick}** and the line of files
    waiting; **Open the wallet** is its first button. Done's **Unlock
    {vault}** opens the wallet's checklist, done. A plan with no vault has
    **Home** in its place.

16. **Review fixes.** (a) A flow that is Done is never looked up for a new
    backup: a new lock starts a fresh flow for the wallet, so a second
    backup in the same power-on writes again. (b) Each wallet's flow is tied
    to its Make a new wallet or Restore by an id (`owner`, not kept across a
    lock). (The press hook that saved keys into the flow's vaults is gone;
    see the reworked note below.) (c) F6
    holds every seed the plan keeps on any stick, in a vault or as a seed
    file, and moves it to the flow at the lock. (d) `backup-flow` is written
    like `backup-gate`, by a temporary file and a rename; a flow with no
    wallet part is not read, and the gate's held fingerprints are merged
    into a flow that is. (e) `refit` returns which rows it kept and the
    plan's vaults follow their rows, so a vault keeps its salt when a slot
    turns out to be a cosigner's key. (f) After the lock a seed is matched by
    its words (its base fingerprint), the passphrase is put in the vault only
    where the plan puts it there, and the round copies only this wallet's
    `WALLET` and `PLAN` records. (g) A vault preset chosen after the keys
    exist is the adding-after-the-keys case of §4.8: a new vault is made at
    once with the key in it and its stick is checked at step 6; a vault that
    exists takes the extra round. (h) Make a new wallet's Bring in
    and Vaults cards are done once the wallet is made (the question and the
    count are done only by their own Continue), and
    Check's Continue opens Back up. (i) A seed has one record when a passphrase is
    added later. (No record is saved before the lock now, so none is
    removed.) (j) A plan changed after the
    lock to add the wallet description or a passphrase to a vault that
    exists is a round. (k) `refit` recommends and never forces: a passphrase
    is placed apart only when a preset is chosen or a passphrase is added;
    attaching, adopting and resuming move no placed passphrase and add no
    place. (l) A seed file is matched as `{stem}-{fp}-...` for this wallet's
    seeds exactly, in the slot's files and again at write time.
17. **Public files beside a plain seed file.** When any seed file is
    planned, the public files get their own stick by default (`PublicOn::Own`)
    so a seed file never shares a stick with the files that are handed
    around. Merging them stays a deliberate choice. This is a default the
    owner may change.
18. **Back** on a card of a wallet's flow before its keys exist returns to
    Make a new wallet or Restore. Taking a written stick off the plan says
    so in a toast (**A stick already written stays on the plan**). The seed
    files and shares items of the checklist read **Written to {stick}** once
    written.

19. **Second review.** A key held back is released by one rule: every
    stick of the plan is written, and the seed's own planned sticks are among
    them; the second is a check against a plan that shrank. After the lock
    an answer that leaves a held seed or its passphrase on no stick, or a
    vault with no stick, is refused with a toast. A plan change after the
    lock marks the vaults that must hold more as stale at once, without
    Continue. A
    seed file goes only to a stick of files that does not carry the public
    files, and only for a seed the plan keeps as a file now. Every `refit`
    remaps the flow's vaults, the lock keeps the flow's own vault while it
    fits, every Make a new wallet and Restore has its own id (never 0, and a
    flow read back after a lock gets one nothing matches), a Done flow that
    waits hands its hold to the flow that replaces it, and Add a passphrase
    acts on the wallet shown.

20. **Review of the reworked order (2026-10-15).** (a) A plan vault with no
    vault set up for it, while another is open, offers **Make a vault**
    (a new vault on an empty stick, written at the write step as an added
    copy is) and, when a vault on a stick read this power-on is locked,
    the way to unlock it. A vault on a stick not read yet is the extra
    round of §4.8, which is not built before the keys (see Open).
    (b) **Lock it and make a new vault** locks the vault the plan vault
    would be saved into, never the selected one; the vault set up for a
    plan vault is not locked from there, and the press says so.
    (c) **Lock and bring the sticks** asks as any lock does, and the Make a
    new wallet or Restore that asked for the setup is kept across it
    (kind, quorum, answer and count, nothing secret) and returns past
    its questions. The question and the count are fixed once a vault of
    the setup is open, and Keys (Seeds on Restore) stay closed until the
    setup is done. (d) The question and the count are done only by their
    own Continue, also once the wallet is made. (e) The count is one to
    1024, what a saved plan reads back.

**Reworked (owner, 2026-10-15).** Items 3 to 5 and 13 describe the order
as reworked. The first build asked where the keys would be kept (places,
vaults, which seed goes into which vault, the stick places) before any
key existed, in a card of seed forms and then four checkboxes; that
card, its actions, the plan editing before the keys and the hook that
saved each key into its vaults are gone. The tests that carried that
order set up the vaults first and decide the rest in Back up.

**Open (for the owner).**

- **Keep it in a vault for the round** is not built, because a lock wipes
  Create and the checklist. What is built is the round after the lock, with
  the key typed back from its checked copy, which is the default choice.
- **A key kept only on paper in a multisig whose description reaches only a
  stick** is released before the writes (§16 item 7 of B1a). Is that the
  wanted order, or does it wait until the stick that holds the description is
  written?
- **L4 and L5 of the security review** fail closed: nothing secret is lost
  or sent. They are left as they are.

**Not built in B1b, and why.**

- **The two choices for a key kept only on paper before an extra round**
  (§4.8, §15 item 7): **Finish its paper copy first** and **Keep it in a
  vault for the round**. A wallet's keys live in Create and the checklist,
  whose state a lock wipes, and the flow's round that starts before the
  final lock has nowhere to return to once the lock has wiped them: the
  decision needed is whether such a round returns to the checklist (and
  which of its items it keeps) or to the writes. What is built is the round
  after the lock, where the key is typed back from the copy the checklist
  checked. This needs the owner.
- **The Nostr key's part** (N1), as before.
- **A progress bar for a write** and **"Changed since written"**, as in B1a.

## 17. The plan by place (F35)

(owner, 2026-10-15.) Back up's plan for a Bitcoin wallet is asked place
by place. The questions "The seeds go" and "The wallet description goes",
which asked where each kind of thing goes, are gone. The person says how
many places there are, then fills one card per place with what that
place keeps, then decides the wallet description and where it goes,
then sees the whole map and approves it. The other kinds' plan (§2) is
already by place and does not change. Make a new wallet's vault question
and count, and the vaults set up before the keys (§10.2), do not change:
they are the vaults the place cards offer.

### 17.1 The cards

In this order, as step cards on `wide` and a page each on `small`
(NEW-WALLET §14):

1. **Start from**: the three presets, unchanged. Each fills every card
   below; every answer stays changeable.
2. *(Replaced by §19 in Back up, batch F2a, and in the wallet's guided
   flow at any width, batch F2b.)* **Places**: "Number of places", the stepper (one or more, no upper
   bound, §10.3), and with a vault open each place's name, as today. The
   share question and the vault-stick sections leave this card.
3. *(Replaced by §19 in Back up, batch F2a, and in the wallet's guided
   flow at any width, batch F2b.)* **Place 1**,
   **Place 2** … one card per place, titled with the
   place's name where it has one. A multi-choice list (DESIGN §4.2) of
   what the place keeps, a row each:
   - **Seed words by hand**
   - **SeedQR by hand**
   - **Vault 1**, **Vault 2** …, a row per vault of the plan
     (`Shape::vault_rows`), named as `backup_vault_title` names it today
   - **Seed as a file, unprotected**
   - **Passphrase**, drawn only when a seed here has one.
   Ticking a row opens a sheet (a popup; full screen on `small`) titled
   "{row} · {place}", whose body is a multi-choice of the seeds loaded
   here, by fingerprint, and **Done**:
   - for words, SeedQR and file: which seeds this place keeps in that
     form; for words and SeedQR, a seed whose passphrase this place keeps
     is dimmed and cannot be ticked, so nothing already planned is moved;
   - for a vault: which seeds the vault holds, and under them, for each
     ticked seed that has a passphrase, "{fingerprint} with its
     passphrase". A vault is one vault wherever its stick is kept: the
     sheet edits the vault itself, and where another place also keeps
     its stick it reads "Also at {place}" above Done;
   - for the passphrase: which seeds' passphrases this place keeps; a
     seed whose words or SeedQR this place keeps is dimmed and cannot be
     ticked (the rule `set_places` keeps today).
   **Done** with no seed ticked leaves the row unticked, except a vault's
   row: a vault may hold only the description or shares (the person's
   choice, §17.6), so its stick stays at the place and the row reads that
   it holds nothing yet; a vault still holding nothing holds The plan
   card, with a line naming it, in place of Make the checklist. A ticked row
   shows what it holds under its label ("9a6a2580, 8c7e2500"); a press on
   the label opens its sheet again; a press on its box unticks it and
   clears what it held at this place (for a vault, its stick at this
   place; the vault keeps its seeds while another place keeps its stick).
   Under the list, the place's own line of fact: what someone who finds
   this place alone gets (`plan::alone` for `At::Place(p)`), as the map
   panel words it. Then **Continue**.
4. **The wallet description**, for a wallet whose description splits
   (`Shape::splits`); not drawn otherwise. Two rows, one ticked:
   **Whole** and **In shares**, with **What is a share?** beside In
   shares (as today beside Its own share). With In shares, the slider
   **Keys left off each share** (0 to `min(m−1, n−1)`) and the shares
   table with the split's audit lines (`shares_card`, unchanged).
5. *(Replaced by §19 in Back up, batch F2a, and in the wallet's guided
   flow at any width, batch F2b.)* **Where the
   description goes**: a section per place, its name as the
   label, each a multi-choice of forms: **On paper**, **As a file**, and
   **In {vault}** for each vault whose stick that place keeps. With
   shares, a second list under each place's forms, **Shares here**: Share
   1 … Share n; the forms ticked hold every share ticked there. Then a
   last section, **Not a place**: **Into watch-only software**, and for
   a plan saved before this change, what its stick of files
   (`At::Files`) holds, a row each, which can be unticked.
6. **Software**: as today (the software and the form), asked when the
   description goes into watch-only software or as a file at any place.
7. **The plan**: the map, every spot, with the check lines and §10.3's
   lines under it (`map_rows` and `glance::check_lines`, as the map panel
   draws them), then **Make the checklist**. This card is the approval:
   the checklist is made only from here. On `wide` the map panel beside
   the cards stays.

The holds that stand in place of the way on (a vault's stick at no
place, a key made here kept nowhere, NEW-WALLET §14.2 and §14.4) are
drawn on The plan card, where Make the checklist would be.

### 17.2 The model

- **The map is the plan from the first answer.** The presets and
  `Answers::defaults` still give their answers; the first answer on any
  card from Places on makes `plan::map(shape, answers)` the plan
  (`Answers::set_map`), and every answer after it is a map edit
  (`plan::edited`, `with_added`, `without`). The rule fields (`seeds`,
  `wallet`, `vaults`, `sticks`, `pass`) are read only for a preset and
  for a plan saved before this change. `split` and `omit` stay: they
  decide the shares.
- **Places count.** Raising the count adds an empty place; lowering it
  removes the last place and what it held (`Edit::RemovePlace` with no
  destination). A seed left kept nowhere holds The plan card, as today.
- **Files at a place.** A file is kept at a place, on a stick there:
  `What::SeedFile(i)` may be held by `At::Place(p)` (tag Secret), and two
  new things, `What::WalletFile` (the description's public files) and
  `What::ShareFile(j)` (share *j* as files), are held by a place (tag
  Public). `At::Files` stays for a plan saved before this change and for
  the stick of public files that goes to the online computer
  (§10.2), which the software's files use.
- **Shares in a vault.** `At::Vault(v)` may hold `What::Share(j)` (tag
  Sealed) as it holds `What::Wallet`.
- **The check** (`check`, `alone`, `all_at_one_place`, `lost_why`,
  `kept_nowhere`) counts a seed file at a place as that seed at that
  place, and a file or a share in a vault as that part of the
  description, read as the vault is. `seed_on_stick` counts a seed file
  at a place, so F6's hold lasts until that place's stick is written.
- **The plan's text.** `spot` lines write the new things (`what_code`,
  `what_read`); a plan saved before this change, with no `spot` lines,
  reads as it did and is shown on the cards as its map.
- **The checklist** comes from the map as today. A place that keeps any
  file has **the stick of files for {place}**, written at step 6 in its
  place's turn (§4.6), named "Files ({place})"; its files are the seed
  files and the public files that place keeps. A stick of files at a
  place may hold both; nothing warns (§10.2's rule for public files).

### 17.3 Strings

New, in `faraday/faraday-strings/src/en.rs`: the place card's rows
("Seed words by hand", "SeedQR by hand", "Seed as a file, unprotected",
"Passphrase"), the sheet's title "{} · {}", "{} with its passphrase",
"Also at {}", the description card's title "The wallet description" and
rows "Whole", "In shares"; "Where the description goes", "On paper", "As
a file", "In {}", "Shares here", "Not a place", "Into watch-only
software"; "The plan"; "Files ({})". Strings left unused ("The seeds go",
the old seed and description rows, "Each place keeps") are removed.

### 17.4 Tests

`faraday-core/tests/backup_plan.rs` (the wallet's plan, through the
app's actions as a person presses them):

- A 2-of-3 with its three keys made here, three places: three place
  cards. At Place 1, ticking Vault 1 opens its sheet; ticking the first
  and second seeds and Done puts both in Vault 1, its stick at Place 1.
  At Place 2, Vault 2 with the third seed. The map holds exactly that.
- At Place 3, Seed words by hand with the first seed: the map's Place 3
  holds its words; the line under the card says what finding Place 3
  alone gives.
- Done with no seed ticked leaves the row unticked and the map
  unchanged.
- Vault 1 ticked at Place 1 and Place 3: one vault, two sticks; its
  sheet at Place 3 reads "Also at Place 1", and a seed ticked there is
  in the vault at both.
- A seed with a passphrase: its row is dimmed in Place 1's Passphrase
  sheet while Place 1 keeps its words.
- In shares with one key left off: Where the description goes offers
  Shares here; Share 1 on paper at Place 1 is on the map there.
- Seed as a file at Place 2: the write step has the stick of files for
  Place 2, and the seed is held (F6) until it is written.
- Lowering the places from three to two removes Place 3 and what it
  held; a seed kept only there holds The plan card.
- Make the checklist is only on The plan card; the checklist matches
  the map.
- Each preset still fills the place cards (Paper and vault on a single
  key: words at Place 1, Vault 1's stick at Place 2).
- A plan text saved before this change (no `spot` lines) loads and shows
  on the place cards as its map.

The tests that pressed the old questions are changed to press the place
cards, keeping what each checked; none is deleted.

### 17.5 Docs

`docs/WALLETS.md` §5 (the questions), `docs/NEW-WALLET.md` §5.2 (marked
"Changed by BACKUP-FLOW §17"), `docs/DESIGN.md` §4.2's "Used for".
`docs/learn/07-backups.md` is upstream's: leave it, and say in the
report what in it no longer matches.

### 17.6 A share in a vault

(owner, 2026-10-15: "people should be able to store things in vaults or
not in vaults; up to them".) **In {vault}** with shares keeps the shares
ticked at that place in that vault, as §17.1 item 5 says; nothing turns
it into the whole description.

- **The record.** A new vault record, type 13, **Wallet share**
  (`docs/VAULT.md` §7): 1 the share, as the share's `.txt` file holds it
  (`backup::split_share`), required; 2 the wallet's name, as type 3's
  field 2; 3 which share, `u8`, from 1; 4 how many shares, `u8`; 5 the
  wallet, as type 11's field 1 holds it. Several per slot, one per share
  and wallet; a share is matched by field 5 and field 3, never by name. `faraday-vault` reads and writes it;
  `tools/vault/open.py` names it and its fields. A build before this one
  refuses a slot holding it (§7: an unknown type refuses the slot);
  VAULT.md says so, as F32 did for type 7's new fields.
- **Saving.** The checklist's vault item (**Save into {vault}**) saves
  each share the map puts in that vault as a type 13 record, beside the
  seeds and the whole description it saves today, and is done only when
  the open vault holds every one. Saved twice, a share is kept once (same
  wallet, same share number).
- **Reading.** The open vault's contents list a share as "{wallet} ·
  Share 2 of 3". Restore's shares (`restore_share_texts`) take the
  shares in every open vault beside the ones in the Inbox, so shares
  from vaults and from sticks or paper merge as today
  (`restore::merge`).
- **The checks** already count a share in a vault (§17.2).

**Tests.** `faraday-vault`: a slot with two type 13 records seals and
opens with every field; a type 13 record without field 1 refuses the
slot. `faraday-core/tests/backup_flow_wallet.rs` (or the vault item's
own test file): a 2-of-3 with In shares and Share 2 in Vault 1 at Place
1: Save into Vault 1 puts that share there, the item is done only then,
and the sealed vault holds it after the lock. Restore with that vault
open and Share 1 in the Inbox rebuilds the wallet.

### 17.7 The cards in the guided flow

(owner, 2026-10-15.) The flow's plan card is the same place cards, drawn
by the same code (`plan_cards.rs`), on the flow's own copy of the plan.
Since F2b the place cards are the board (§19, §19.12 item 1), drawn by
`board_screen.rs` on the flow's plan; the keys left off each share are
the board list's stepper.

- **Keys left off each share** is drawn in the flow too, with its table
  and audit lines, from the flow's shape: `split_plan(n, m, omit)` and
  the keys named by fingerprint. It sets the flow's own `omit`. The
  shares' files are made at the write step, as today.
- **Place names** are typed on the Places card while a vault of the flow
  is open (before the lock), and kept in the plan record each vault
  holds (VAULT §7 type 11 field 3). After the lock no vault is open:
  the name fields are not drawn and places read "Place n". A name is
  never kept in the flow's state.
- **One way on.** In the flow, a card's Continue opens the next card,
  and The plan card's is the flow's Continue; nothing else draws one.

### 17.8 Changes after the lock

(owner, 2026-10-15.) After **Lock and write** no wallet and no key is
loaded, so the plan changes there only in what the vault round (§4.8)
handles: **what goes into a vault and where a vault's sticks are kept.**
That is a seed, its passphrase, the description or a share in a vault;
a vault's stick added to or taken from a place; a place added, or the
last place taken away while it keeps only vaults' sticks (a place before
another is not taken away: the later places would be renumbered, sticks
already named by place among them). Each such change sends the
vaults it touches back to be filled, sealed and written (§10.2).

**Files and paper stay as they were made.** After the lock, a change to
words or a SeedQR by hand, a passphrase on paper, a seed's file, the
sheet or a share on paper, the description's or a share's files, the
software or the form, or Whole / In shares and the keys left off each
share while any of the description is on paper or in a file, is refused
with "Files and paper stay as made after the lock; finish the writes,
then Back up again to change them", by every route: the flow's cards,
Back up's cards, presets and the wallet's chart, with a vault open or
not. To change them the person finishes this backup's writes and runs
**Back up** again on the wallet, a backup of its own (owner,
2026-10-15). While a backup of the wallet is past its lock and not done,
Back up's checklist and its Lock and write never replace that backup's
plan.

**Taking something out of a vault** is allowed only for what this backup
itself put into that vault. Refused, with the plan left as it was:

- taking out what the vault held before this backup opened it ("Vault
  {n} held this before; take it out from the vault's contents");
- taking a key, or its passphrase, out of a vault that holds any record
  of another wallet ("Another wallet in Vault {n} uses this key; take it
  out from the vault's contents"): a hidden wallet on the same words with
  a passphrase kept apart cannot be told from the records;
- any change while a stick is being written.

A key made here is released only when the sticks actually written hold
it: a vault sealed with it, or a stick of files whose written files
include its file. No wallet is loaded after the lock other than by
unlocking a vault.


## 18. Stick pills on every device

(owner, 2026-10-15.) The numbered stick pills that Faraday Desktop and
the website demo draw across the top (`stick_pills.rs`, commit bbcfd1e)
are drawn on the PC stick and the Pi too, so a person who brings a stick
back can see it is the one the flow wants.

- **When.** On Faraday Desktop and the demo, always, as today. On a
  device (`shell_sticks` false), only while a backup flow on screen uses
  sticks (`sticks_wanted()` is some): the boot stick's pill, then a pill
  per stick of the flow in the flow's slot order.
- **Numbering** (owner, 2026-10-15): the boot stick is Stick 1 and is
  still called **Boot stick**; the backup's sticks are numbered from 2,
  "Stick 2", "Stick 3" … ("Card" on the Pi, by `app.medium`, the boot
  card being Card 1). Everywhere: Faraday Desktop, the demo, the device,
  the flow's lines and its "Mark it as" line. Where only numbers fit,
  the boot stick's pill reads "1". Desktop's backup stick folders follow
  the numbers (`STICK-2` for Stick 2); `BOOTSTICK` stays. A Desktop
  install with a `STICK-1` folder from before renumbers its folders once
  at start, each `STICK-n` to `STICK-(n+1)`, highest first.
- **No press on a device.** A real stick is put in by hand: the pills
  carry no action there. On Desktop and the demo a press still plugs or
  pulls, as today.
- **States.** Out: dim, as today. Wanted now: the flow's `now`, edged in
  its colour, as today's `asked`. In: lit, and on a device only when the
  stick in is recognised as that slot's: it reads as its `Bound` (label,
  serial, listing, the vault files' SHA-256, §4.6's check). A stick in
  that matches no slot lights no pill.
- **The boot stick in a place.** A slot the boot stick holds is Stick 1
  (Boot stick); the other slots are numbered from 2 in slot order,
  skipping it. The boot stick gets no "Mark it as" line.
- **One stick wanted.** Which stick is wanted now is decided once, and
  the edged pill, the card's Insert line and the flow use that one.
- **Mark it.** When a stick is first bound to its slot (§4.2, and a stick
  first checked at step 6, §4.6), its card says "Mark it as Stick {n}"
  ("Mark it as Card {n}" on the Pi), the number of its pill, under the
  stick's row, until it is pulled.
- **A stick from elsewhere.** A stick in that is not the one wanted and
  matches another slot reads "This is Stick {m}", and one not yet settled
that reads as another slot's stick (a blank stick read alike) reads
"This reads as Stick {m}" above the hold; one that matches none,
  "Not one of this backup's sticks". §4.6's own refusal stays as it is.
- **Size.** The pills grow: height 28, text 13, the dot radius 4, and the
  padding to match; on the Pi's small panel (`small`) height 24 and text
  12. Where the row does not fit, the short form ("Stick 2"), then the
  number alone ("1"), before wrapping to a second row.

**Strings** (`faraday-strings`, `backup_flow_` prefix): "Mark it as {}
{}", "This is {} {}", "Not one of this backup's sticks".

**Tests** (`tests/stick_pills.rs`): on a device shell no pills without a
flow, the boot pill and a pill per slot with one; no action on a device;
a bound stick lights its own pill only; a stick matching no slot lights
none and reads its line; the mark line shows after binding and goes on
pull; at 480x640 a five-stick flow fits on one row in numbers alone;
the first backup stick is Stick 2 on every shell.
Render the flow's Bring in the sticks at 480x640 and 1280x800 with the
pills.


## 19. The backup board (F37)

(owner, 2026-10-15.) The plan's place cards and Where the description
goes (§17.1 items 2, 3 and 5) were a set of tick lists the owner found
confusing. They become one board: a card for each place, each card
divided into where a thing can be at that place (on a stick, inside a
vault on a stick, on paper), and beside the cards a list of everything
the backup can keep. The person drags a thing from the list into a
place, or from one place to another. The plan underneath is the map of
§17.2, unchanged in kind: each drop is a map edit (`plan::Edit`).

Scope: Faraday Desktop and the PC stick, at any window width (§19.12
item 3 for a narrow window). The Pi is not considered here.

### 19.1 The board

The plan card's body, from Places to Where the description goes, is
replaced by the board, and so is The wallet description's Whole or In
shares (item 4): the shares are in the board's list. Start from (§17.1
item 1), Software's choice of software (item 6) and The plan with Make
the checklist (item 7) stay as cards.

**The places.** A row of place cards, wrapping to more rows when they
do not fit. Each card is a rounded rectangle (DESIGN §4.17) with the
place's name at the top (typed while a vault is open, as §17.7 says;
"Place n" otherwise), an × at its top right that removes it (with what
it holds, `Edit::RemovePlace` with no destination), and under the name
its zones, top to bottom:

- **Stick 1, Stick 2 …**: one zone per stick the place keeps (drawn as a
  box in the place box, with the vault box inside it: see §19.14). A stick
  zone has two parts: the vault on it, when one is ("Vault 1", and what
  the vault holds, §19.3), and its files. A stick with no vault is a
  stick of files. A place starts with no sticks; dropping a file or a
  vault on the card's **+ stick** row, or on the stick part of an
  empty card, makes one, and a press on **+ stick** adds an empty one
  (§19.16).
- **Paper**: what the place keeps on paper.
- Under the zones, the place's line of fact: what someone who finds this
  place alone gets (`plan::alone` for that place, as §17.1 words it).

After the last card, a **+** card adds an empty place. There is no
count to set first: the board starts with one empty place.

**The software place.** The list's **Watch-only software** entry is a
place too, kind software, at most one: its card is titled with the
software chosen (or "Watch-only software"), has stick zones like any
place, and in place of Paper a zone **Scanned in**: the description
shown as a QR code for the software to scan, nothing kept. It counts in
the checks as a place that keeps what it holds (the description, never
a seed).

**The list.** On the right of the board (in the right-hand column where
Back up's Map was: see §19.14),
the things the backup can keep,
grouped, each a chip (DESIGN §4.17):

- **Seeds**: one chip per seed of the wallet, named as today (its label
  or fingerprint). A seed whose key is not here (a cosigner's) is drawn
  dim, reads "Not here" and cannot be dragged.
- **Passphrases**: one chip per seed with a passphrase.
- **Wallet description**: **Whole**, and for a wallet whose
  description splits (`Shape::splits`, a multisig) the partial
  descriptions, **Share 1 … Share n**, always, with no Whole or In
  shares to choose first. Above the shares, **Keys left off each share**
  as a stepper (0 to `min(m−1, n−1)`, the slider of §17.1 item 4), and
  under each share chip the keys it carries, by fingerprint. A share is
  made from the wallet when it is shown or written (`backup::split_plan`,
  `split_share`); nothing is made before. Changing the keys left off
  changes what each share carries and leaves every placed share where it
  is; the checks follow at once. Whole and shares may both be placed;
  the checks count the description kept where the whole is, or where
  enough shares are to rebuild it, as `check` does today.
- **Vaults**: one chip per vault of the plan, and **+ vault**, which
  adds one (the vault rows of §10.2: a vault the plan makes).
- **Watch-only software**, while the board has no software place.

A chip in the list never runs out: dragging it places a copy. Each list
chip shows how many places keep it ("2 places"), and nothing when none
does.

### 19.2 Moving things

- **Drag** a chip from the list onto a zone: the thing is kept there
  (`Edit::Add`). From one zone to another: moved (`Edit::Move`). From a
  zone back onto the list, or by the × on a placed chip: taken off
  (`Edit::Drop`). A vault chip dragged onto a place card puts the
  vault's stick there: on the card's first stick with no vault, or on a
  new stick when every stick there carries one.
- **Tap to place**, for a trackpad, a touchscreen or anyone who does not
  drag: a press on a list chip selects it (edged); the zones that take
  it are edged too; a press on one places it; a press anywhere else, or
  on the chip again, lets it go. The same for a placed chip: selected,
  then a press on another zone moves it.
- While a chip is dragged it follows the pointer, and the zone under it
  that takes it is edged; a zone that does not take it (§19.3) is not.
  A press that moves past the slop (as scrolling's) is a drag; a press
  that does not is a tap.
- Each drop is one map edit, so the checks (§19.5) and every card's line
  of fact are current on the next frame.

### 19.3 What each zone keeps, and in what form

A thing placed in a zone is one chip there; its forms are the marks on
it ("Seed A · Words · SeedQR"). A drop places the first form of the
zone's row below; a press on the placed chip opens a small popup of its
forms as checkboxes (DESIGN §4.2), at least one ticked. Each form is its
own `What` on the map, so turning a form on or off is `Edit::Add` or
`Edit::Drop` of that `What`, and the plan's text, the checks and the
checklist read forms as they read everything else.

| Zone | A seed *i* | Its passphrase | The description, or share *j* |
|---|---|---|---|
| Paper | `Words(i)`, `SeedQr(i)` | `Passphrase(i)` | `Sheet`, or `Share(j)`, as today |
| Stick (files) | `SeedFile(i)` (words, `.txt`), `SeedQrFile(i)` (`.png`) | `PassphraseFile(i)` (`.txt`) | `WalletFile(form)`, `ShareFile(j, form)`: `form::TEXT` (`.txt`), `form::QR` (`.png`); and `SoftwareFile`, the chosen software's own file |
| Vault | `Seed(i)` | `SeedPassphrase(i)`, with its seed | `Wallet`, or `Share(j)` |
| Scanned in (software) | | | `Wallet` at `At::Software`, as today |

- New `What`s: `SeedQrFile(i)` and `PassphraseFile(i)` (tag Secret),
  `SoftwareFile` (Public). `WalletFile` and `ShareFile(j)` gain the form
  they are (today the plan's global `form` answers decide it); a plan
  saved before this change reads its `WalletFile` as every form its
  `form` answers had on.
- **A vault keeps records only**: the seed as its key record, its
  passphrase stored with it, the description as its wallet record, a
  share as its share record (§17.6). No file goes into a vault. A
  passphrase dropped into a vault becomes `SeedPassphrase(i)` there when
  the vault holds its seed; a vault that does not hold the seed does not
  take the passphrase (there is no record for a passphrase alone), and
  its zone is not edged for it.
- **A vault at several places** is one vault: what it holds is drawn in
  its zone at every place that keeps its stick, with a link mark (DESIGN
  §4.17), and a drop into it at one place is in it at all of them.

### 19.4 Sticks in the model

- New `At::Stick(p, s)`: stick *s* (from 0) at place *p*. It holds at
  most one `VaultStick(v)` and any of the stick forms of §19.3. Paper
  forms stay at `At::Place(p)`.
- A plan saved before this change reads as it did: at each place, each
  `VaultStick(v)` becomes a stick of its own, in vault order, and the
  place's files, when it has any, a stick of files after them (the "stick
  of files for {place}" of §17.2). The old `At::Files` stick stays
  read-only as §17.1 item 5 left it.
- **The checklist's write step** (§4.6) writes each stick in its place's
  turn: the vault on it, when one is, and its files, in one visit. The
  stick's binding (§4.6's label, serial, listing and the vault files'
  SHA-256) is taken after both are written, so the listing includes the
  files. Survey §4.6's binding before building: it must accept files
  beside a vault on one stick.
- The stick pills (§18) number every stick of every place.

### 19.5 Checks are warnings

Nothing is refused for being unwise. Every check of the plan (`check`,
`alone`, `all_at_one_place`, `lost_why`, `kept_nowhere`, and the rule
that a passphrase is not kept where its words are) is a line under the
board, current on every drop, and a placed chip that a check names
carries a warning mark (in `WARN`, as a held address is drawn). Examples:
"Seed B is kept nowhere", "Losing Home loses the wallet", "Home keeps
Seed A's words with its passphrase".

**Make the checklist** is offered with any warning standing, except
a seed kept nowhere, which holds The plan card as today (§17.1): with
it the checklist would release the hold on a key that has no backup
(F31).

### 19.6 Presets

The board starts empty: one empty place, nothing kept. **Start from**
stays above the board, and a preset fills the board as it fills the
map today; the person changes it by dragging. Presets that make sense
of the board (sticks, forms) are a later question for the owner.

### 19.7 A vault's key and wallet, shown and saved

The vault's contents screen (`vault_screens.rs` `detail`) gains, so the
PNG and text forms need not be kept in a vault:

- On a key record: **Show SeedQR**, behind the same hold-to-reveal as
  its words, the code made from the record at that moment; and **Save to
  a stick**, a popup with Words (`.txt`) and SeedQR (`.png`), one or
  both, then the stick, as Files' save does. The popup says "Anyone who
  finds this stick has this seed", as a seed file on the board does.
- On a wallet record: **Show as QR**, the QR view the Wallets page uses
  (several codes for a long descriptor); and **Save to a stick**:
  Descriptor (`.txt`), QR (`.png`), and the software files Back up
  makes.

Nothing new is stored in the vault.

### 19.8 Strings

New, in `faraday/faraday-strings/src/en.rs`: "+ stick", "Stick {}",
"Paper", "Scanned in", "Watch-only software", "Seeds", "Passphrases",
"Wallet description", "Whole", "Share {}", "Vaults", "+ vault", "Not
here", "{} places" and "1 place", the form names ("Words", "SeedQR",
"Words file", "SeedQR file", "Passphrase file", "Text file", "QR
picture", "{} file" for the software's), the check lines of §19.5,
"Show SeedQR", "Save to a stick", "Anyone who finds this stick has this
seed". §17's strings the board no longer draws are removed.

### 19.9 Tests

`faraday-core/tests/backup_board.rs`, through the app's events as a
person makes them (a press, moves past the slop, a lift), on a 2-of-3
with three keys made here unless said:

- The board starts with one empty place and nothing kept; **+** adds a
  place; × removes it and what it kept.
- Dragging Seed A onto Place 1's Paper keeps its words there; its
  popup's SeedQR adds the SeedQR; the chip reads both forms; the map
  holds `Words(0)` and `SeedQr(0)` at Place 1.
- Tap to place gives the same map as the drag.
- Dragging Vault 1 onto Place 1 and onto Place 3 makes a stick at each;
  Seed A dropped into Vault 1 at Place 1 is drawn in Vault 1 at Place 3
  too, and the map holds it once, in the vault.
- A second vault dropped on Place 1 goes on a second stick there.
- Seed B's words `.txt` dropped on Place 1's Stick 1 beside Vault 1:
  the write step writes the vault and the file on that one stick, and
  the stick is recognised afterwards.
- A passphrase dropped on paper that keeps its words is kept, with the
  warning line and mark; Make the checklist is still offered.
- A seed kept nowhere holds The plan card.
- A cosigner's seed is dim and does not move.
- Shares: with no choice made, the list offers Share 1 to Share 3 of
  the 2-of-3; Share 1 on Home's Paper and Share 2 in Vault 1 are on the
  map there; changing the keys left off from 1 to 0 changes the keys
  each share chip lists and leaves both where they are; the checks say
  the description is kept when two shares together rebuild it.
- The software place: its card, the description Scanned in, and a
  stick there with the software's file; losing every other place, the
  checks count the description as kept.
- A plan text saved before this change loads, its vault sticks and its
  stick of files on sticks of their own, and the checklist it makes is
  the one it made before.
- Vault contents: Show SeedQR reveals only while held; Save to a stick
  writes the chosen files and says the seed line first; a wallet's Save
  writes its descriptor files.

The tests that pressed §17's place cards are changed to make the same
plans on the board, keeping what each checked; none is deleted.

### 19.10 Docs and design

- `docs/DESIGN.md` gains §4.17 **Board**: the place card, its zones,
  the chip (in the list, placed, selected, dragged, with a link mark,
  with a warning mark), the edged drop target. Sizes are tokens in
  `core/osk-ui/src/tokens.rs` (CLAUDE.md).
- `docs/DECISIONS.md` F37. §17.1 items 2, 3 and 5 are marked "Replaced
  by §19 on `wide`".
- The board redraws on every pointer move while a chip is dragged: it
  draws through the drawing memo of `docs/SPEED.md` §3.3, and its batch
  is timed by SPEED §7. SPEED batch B goes first.

### 19.12 Further rules (owner, 2026-10-15)

1. **Both plan cards.** The board replaces the place cards in Back up
   and in the wallet's guided backup flow (§17.7), drawn by the same
   code (`plan_cards.rs`) on each one's own plan.
2. **Place names** are typed by a press on the card's name, while a
   vault is open, as §17.7 says; otherwise the name is not a field.
3. **A narrow window.** Where the cards and the list do not fit side by
   side (the `compact` layout of a narrow Desktop window or a small
   screen), the place cards stack in one column that scrolls (the
   column's scroll and overlay scrollbar, as every page has), and the
   list is a sheet opened by **Add**, at the column's top. Placing is
   by tap only there: a press on a list chip in the sheet closes the
   sheet with the chip selected, and a press on a zone places it. A
   placed chip's forms open as on `wide`. §17's cards are not drawn on
   Desktop or the PC stick any more.
4. **A passphrase in a vault on its own.** A new vault record, type 14
   **Passphrase** (`docs/VAULT.md` §7): the passphrase of the key whose
   fingerprint it names, without the key's words. A passphrase dropped
   into a vault that does not hold its seed is kept there as this
   record (new `What::PassphraseSealed(i)`, tag Sealed); a vault that
   holds its seed keeps it in the key record as today. Loading a key
   whose passphrase is in another open vault offers it, as a stored
   passphrase is offered today.
5. **Read back.** Every form the board can keep is read back by Restore
   (the Inbox and Restore by kind): a words file, a SeedQR picture, a
   passphrase file, the description's text and QR picture, a share's
   text and QR picture, the software's file. Tests: for each form, a
   wallet backed up with that form at one place is restored from that
   place's contents alone (with the passphrase and the other shares from
   where the plan put them).
6. **File names**, as Back up names files today: `{wallet}-descriptor.txt`
   and `.png`, `{wallet}-share-{j}.txt` and `.png`, the software's own
   name for its file; for a seed, `seed-{fingerprint}.txt`,
   `seedqr-{fingerprint}.png` and `passphrase-{fingerprint}.txt`. Each
   stick's files are one checklist item with the stick's write (§19.4);
   paper forms are the copy items they are today.
7. **Vaults.** The same vault may have two sticks at one place: dropping
   it on a place that has it adds a stick. A vault leaves the plan by the
   × on its list chip, its sticks with it, with a warning line when it
   holds anything. A vault that already exists (made before this backup,
   with contents) shows what it held before in its zones, read-only and
   dimmed; §17.8 says what may be taken out of it.
8. **After the lock** (§17.8): Paper and every stick's file part take no
   drops and no form changes, and a press on them says §17.8's line;
   vault zones and vault sticks change as §17.8 allows. This is not one
   of §19.5's warnings: after the lock no key is loaded, so nothing can
   be written to paper or a file.
9. **Keyboard.** Placing by keyboard alone is not specified here; a
   later batch.
10. **Read back after the write** (owner, 2026-10-15). At the write step,
   each file written to a stick is read back from the stick at once and
   checked against what was meant: a text file's bytes; a SeedQR or QR
   picture decoded (`faraday-scanner`) to the seed's words or the
   description's text. A file that does not read back is written again
   once; if it still does not, the stick's item is not done and the card
   says "{file} did not read back from {stick}; try another stick". This
   is beside §4.6's binding, which checks the vault files and the
   listing.

### 19.11 Batches

- **F1, the model**: `At::Stick`, the new `What`s and forms, reading old
  plans, the write step's sticks and file names, the passphrase record
  Restore reading every form, and the read back after the write (§19.3,
  §19.4, §19.12 items 4–7, 10).
  Tests on the map, the checklist and the round trips.
- **F2, the board**: the cards, the list, drag and tap to place, forms
  popup, warnings, both plan cards, place names, the narrow column, the
  lock (§19.1, §19.2, §19.5, §19.6, §19.12 items 1–3, 8), DESIGN §4.17.
- **F3, the vault's show and save** (§19.7). Independent of F1 and F2.
- **F2d, + stick and a vault at no place** (§19.16), from the owner's
  test of F2c. Before F3.

F1 built 2026-10-16. F2d and F3 built 2026-10-19. F3's choices where
the spec says nothing: the SeedQR row is shown for English words only;
the save popup's warning is a line, and Save is the consent; a wallet
record's popup lists every software file (the record says no software),
the descriptor alone ticked.

**F2a built** (2026-10-16): the board in Back up on `wide` (§19.1–§19.3,
§19.5, §19.6, §19.12 items 2, 7, 8), DESIGN §4.17. Left for F2b: the
board in the wallet's guided flow (§19.12 item 1), the narrow column with
**Add** (item 3), and §17's strings no longer drawn. Choices made where
the spec says nothing:

- Back up opened on a wallet with no plan starts on Start from over an
  empty board (one place, nothing kept, `Answers::empty`); a preset fills
  it as before.
- The board is one card, **Places**, between Start from and the
  software; its Continue opens the next card.
- A press on a placed chip both selects it (a press on another zone moves
  it) and, where its zone has more than one form for it, opens its forms.
  A chip with one form there has no popup. A press on any other control
  lets the chip go.
- The software place's stick is the stick of public files (`At::Files`):
  it takes the whole description only, in the forms the Software card
  answers. Its × takes the software and that stick off together.
- A vault's last stick taken off a place leaves the vault on the plan,
  holding what it holds, and the plan held on "Vault n's stick needs a
  place"; a vault leaves the plan only by the × on its list chip.
- **+ vault** adds a vault holding nothing and kept at no place; the list
  shows the plan's vaults and the vaults set up before the keys, not one
  per seed.
- The description counts as in shares (`split`) while any share is kept
  anywhere; with Whole and shares both on paper, the checklist's Sheets
  item prints the shares (it makes one or the other).
- After the lock only the last place has a ×; the **+** card stays (a
  place may be added, §17.8).
- The warning mark goes on a passphrase and its seed's words kept at one
  place, and on a seed whose every copy is at one place when losing that
  place loses the wallet. The checks that stand are lines under the
  board: a seed or passphrase kept nowhere, `lost_why`, every secret at
  one place, a place that alone can spend (with two places or more), a
  vault with no stick, a vault holding nothing.
- The holds of NEW-WALLET §14.2 and §14.4 (a vault with no stick, a vault
  holding nothing) still hold The plan beside a seed kept nowhere; the
  owner kept them (2026-10-16), so §19.5's "only a seed kept nowhere"
  reads "a seed kept nowhere, or a vault with no stick or holding
  nothing".
- Timed per SPEED §7 (SPEED §6): the board's frames, a drag's move and
  the forms popup 5–7 ms at 1366×768 and 14–15 ms at 2560×1600 scale
  2; Back up's checklist as before.

**F2b built** (2026-10-16): the board in the wallet's guided flow's plan
card, on the flow's own plan, before and after the lock (§19.12 items 1
and 8); the narrow column with **Add** (item 3); §17's cards, their
`Action`s and handlers and their strings gone; §19.14's boxes, colours,
list column and checks. Choices made where the spec says nothing:

- Whose plan a board press edits follows the screen: the flow's on the
  flow's screen, Back up's (or, past the lock, its flow's) on Back up.
  The flow keeps its own board state (the chip held) with its screen
  state; nothing of it is kept across a lock.
- In the guided flow the board is the plan card's first card, then the
  software where it is asked, then The plan; the flow has no Start from,
  as before. After the lock, the board's paper and files act as in Back
  up after the lock (§19.12 item 8); every change still goes through the
  flow's own refusals (§17.8).
- Place names on the flow's board are typed only where §17.7 typed them
  (before the lock, a vault open, Back up open on the wallet); they are
  Back up's names, and a place removed there takes its name with it.
- **Add** is a button at the top of the board card's body in the narrow
  column; the sheet is the list as on `wide` (with the keys left off and
  **+ vault**), a head reading Add and a way out. A press on the
  software's chip there places the software place at once, as on
  `wide`. A placed chip's × and its forms work as on `wide`.
- The narrow column takes no drag at all: a press on a chip is a tap,
  and the page scrolls under the finger.
- A stick box's × takes the stick off with all it carries (its vault's
  stick and its files); after the lock only a stick that carries a vault
  and no file shows it.
- A chip is the card's surface edged in the border on every level, so it
  stands on paper, a stick box, a vault box and the list alike; Paper and
  the software's zones are the place box's surface edged in the border;
  a stick box is the inner fill with a heavier border edge; a vault box
  is the accent at low opacity with the accent's edge (DESIGN §4.17).
  These are the palette's own colours, so every theme draws them.
- The list column on the guided flow's screen is the side panel's width
  (`side_panel_w`), as on Back up; the flow's column narrows by it while
  the board is shown on `wide`.
- The Pi draws what the PC stick draws at its size: the narrow column.
  Nothing of §17's cards is kept for it (§19 left the Pi out; the Pi's
  screens are a later question for the owner).
- §17's model functions in `plan.rs` (`set_row`, `row_seeds`, the forms
  of Where the description goes) stay: the presets, old plans and the
  tests read plans through them.
- Timed per SPEED §7: at 1366×768 the board's frames with the list
  column, a drag's move and the forms popup 5–6.5 ms, at 2560×1600
  scale 2 14–16 ms; the narrow column at 800×600 (`compact`) 2 ms a
  frame, a scroll step and its frame 2 ms, the Add sheet 1.7 ms. The
  guided flow's board is not reached by the harness (it needs sticks
  served and a restart past the lock); it is the same view and drawing.

**F2c built** (2026-10-16): the board in rows and its colours (§19.15),
and one page colour across the app (`docs/DECISIONS.md` F39); DESIGN
§4.17 and §5. Choices made where the spec says nothing:

- A place's sticks and Paper share the row's width equally, **+ stick**
  `BOARD_ADD_STICK` (72 dp) between them. "A chip's width" is
  `BOARD_BOX_MIN`, 160 dp: a fingerprint's chip with its marks. Below it
  Paper keeps that width and the sticks wrap, as many to a line as fit;
  the last line of sticks takes what the row has over them.
- The software place is drawn as a place: its band (no line of fact),
  its one stick of public files as a stick box "Stick 1" with no × of
  its own and no **+ stick** (it keeps one stick), and Scanned in in
  Paper's colours.
- The glyphs are the closed icon set's: house (place), drive (stick, as
  a stick is drawn elsewhere), vault, and file for Paper (the set has no
  pencil).
- The line of fact is a tag on `SURFACE`, outlined and written in the
  chart's tone for it. In Bitcoin Orange Light that tone for a sealed
  place is the theme's own accent, 2.3:1 on its surface, as the chart
  already draws it; the board's own colours all pass.
- **+ place** is a dashed row in `MUTED`, label and edge, `BOARD_HEAD`
  high, as the mockup's grey.
- On a vault's band everything is in `VAULT_ON`: glyph, name, link and
  warning marks, ×, and the ring of the vault held. The lock mark on a
  sealed chip is `VAULT_EDGE`.
- A chip's edge is the mockup's (#AEB9C3 in the light theme, 1.6:1
  against the stick): a chip is not one of the table's boxes, and its
  surface, edge and shadow set it apart. Its faint shadow is a fill
  `BOARD_CHIP_DROP` below it, not a blur, which would cost a frame a
  blur per chip.
- On Back up the board takes its step's whole width, under the step's
  number as well as its title; in the guided flow it stays inside the
  plan card's own card, as F2b drew it.
- The theme tables key a colour by its dark value, so `VAULT_EDGE` and
  `VAULT_ON` are one step from `ACCENT` and `ON_ACCENT` in the dark
  theme (#83D8EE, #0D121A).
- F39 across the app: the page (wide, narrow and the held frame), a
  sheet and the gap a stretched scroll shows are `SURFACE`; a step is a
  section with a `LINE` rule above it and one under the last, on `wide`
  and in a small panel's list of steps; a card or tile on the page
  (`Ui::card`, Home's and Tools' tiles, the narrow menu, menu rows, the
  codes' rows, the vault pickers, the pill, the guided flow's plan card
  heads) keeps its fill and is edged in `BORDER` instead of `LINE`,
  since the fill no longer sets it apart. Wells drawn in `BG` (fields,
  the chart's nodes) are unchanged. The Starter, a program of its own
  with its own drawing, is not changed.
- Timed per SPEED §7: at 1366×768 the board's frames, a chip selected,
  a drag's move and the forms popup 5.3–6.6 ms (median), at 2560×1600
  scale 2 12.7–14 ms; the narrow column at 800×600 2.3 ms a frame and
  a scroll step, the Add sheet 1.9 ms. The checklist's frames 3.5–8.6 ms
  at 1366×768 (Vault 2's card 13 ms) and 8–14 ms at 2560×1600 (Vault
  2's 21 ms), Create's Keys 3–4.6 ms and 11 ms.

### 19.13 Decided while building F1 (2026-10-16)

1. **Old plans** (owner): an old plan whose places kept files gets the
   new checklist, its files written with each stick, one item per stick
   (item 6); an old plan with no files at a place makes exactly the
   checklist it made before. §19.9's old-plan test reads this way.
2. **Stick files** are made into the Outbox at Lock and write, while
   the keys are loaded. A seed's words file, SeedQR picture or
   passphrase file is made only past the seed-file warning (the secret
   sheet, one per seed), on the board's plan as on the place cards
   still drawn where the board is not.
3. **Every stick is its own write.** The flow's targets are keyed by
   stick: a vault's second stick at a place, and a place's second stick
   of files, are each one write, one checklist item and one pill
   (`Target::Another`). Sticks of files at one place share their label
   and are told apart by the pill's number. A stick of files put back
   is recognised by its written files, as a vault stick is by its vault
   files.
4. **File names**: a share's files keep Back up's names,
   `{wallet}-share-{j}-of-{n}.txt` and `.png` (item 6's "as Back up
   names files today"). The old `At::Files` stick keeps its old names
   and its own item.
5. **Plan text codes**: a stick is `stick{p}.{s}`; `qrfile{i}`,
   `passfile{i}`, `vpass{i}`, `wallettxt`, `walletqr`, `sharetxt{j}`,
   `shareqr{j}`, `softwarefile`. An old `walletfile` or `sharefile{j}`
   reads as every form the plan's form answers had on; read alone, as
   the QR form.
6. **The passphrase record** (type 14, `docs/VAULT.md` §7) names the
   key's fingerprint with its passphrase, the one wallets and file
   names use. A key loaded from a vault's key record without a stored
   passphrase takes one from a type 14 record in any open vault whose
   fingerprint matches, as a stored passphrase is taken; nothing is
   asked. This supersedes §19.3's "there is no record for a passphrase
   alone".
7. **Read back** (item 10): the storage channel's write already reads a
   text file back; a picture is read back by the disk process's QR
   reading (`faraday-files`, osk-codec's decoder), not `faraday-scanner`.
   A public picture is checked against the SHA-256 of what its code
   must hold. A SeedQR is checked byte for byte against the Outbox's
   picture and, while its key is loaded, against the key's words; no
   value derived from a seed or a passphrase is kept. A text file
   written again goes beside the failed one, never over it.
8. **Nothing secret is kept outside a vault.** The flow's kept state and
   the visit's receipt record a written file's SHA-256 only when the
   file is not a secret; a secret's is all zeros.
9. **Restore** pairs a words file with `passphrase-{fp}.txt` when the
   two give `fp`, and lists them as one seed.

### 19.14 The board's layout and boxes (owner, 2026-10-16)

Decided after seeing F2a's board; built in F2b. Where this differs from
§19.1, this holds.

1. **Box in a box.** Three nested boxes, so what is on paper, what is on
   a stick and what is sealed in a vault are told apart at a glance:
   - the **place box**, the place card: its name, its stick boxes,
     **Paper** under them (outside every stick box), the line of fact;
   - a **stick box** per stick, inside the place box, headed "Stick 1",
     "Stick 2" … with its ×; everything on that stick is inside it. A
     file kept on the stick (a seed's words file or SeedQR picture, the
     description's files, a passphrase file) sits directly in the stick
     box, outside any vault box;
   - a **vault box**, inside the stick box when the stick carries a
     vault, headed "Vault n" with its ×; only what is sealed in the vault
     is inside it, and each chip there carries a lock mark.
   A drop on the stick box outside the vault box keeps the thing on the
   stick as a file; a drop on the vault box seals it in the vault.
2. **Contrast and colour.** Each level has its own fill and edge, a
   clear step from the box around it: the place box on the card
   surface; the stick box a soft neutral or cool tint one step off it,
   with a stronger edge than the card's; the vault box a gentle tint of
   the accent, edged in the accent. The colours are Faraday's own
   palette (named colours, light and dark), each step visible in both
   appearances, text contrast as DESIGN asks. DESIGN §4.17 names them.
3. **The list where the Map was.** While the board is shown (Back up's
   plan, and the guided flow's plan card, on `wide`), the side panel's
   Map is not drawn: the list of things to place fills that right-hand
   column, full height, scrolling on its own, and the place cards take
   the whole main column. The Map's per-place lists go, since the cards
   show the same.
4. **The summary checks at the bottom.** The Map's three checks ("Any
   one place lost: the rest rebuild the wallet", "One place found: can
   spend", "One place found: sees the balance", each with its yes or no)
   are under the board, with the warning lines (§19.5). The top of the
   board is kept for the access layers' row (§20.7).
5. Not changed: the checklist stage, after **Make the checklist**, keeps
   its side panel and Map (a choice: the owner has not ruled on it);
   a narrow window keeps the list as a sheet opened by **Add**.

## 20. Access layers (F38)

(owner, 2026-10-16. Specified, not built; after §19's F3.)

A backup may serve more than one access layer. Each layer reaches the
wallet by its own route, and the same things at the same places serve
every layer, depending on what each layer knows. The person says, for
each layer, what it is able to spend with; Faraday works out what goes
where so that each layer can, and so that a stranger who finds places
cannot.

### 20.1 Access layers

An **access layer** ("Access layer 1", "Access layer 2" …) has one or
more **routes** (§20.9), each a list of what it is **able to spend
with**, and a threshold: "any k of" the places in it. An entry in the
list is one of:

- a **place** of the plan ("Place 1", "Place 2" …): everything kept
  there;
- a **vault's password** known to the layer;
- a **passphrase** known to the layer.

Examples: "Access layer 1, route 1: Place 1, Vault 1's password";
"Access layer 1, route 2: any 2 of Places 1, 2 and 3, on paper";
"Access layer 2: any 3 of Places 1 to 5, on paper".

The board starts with one access layer; **+ layer** adds one and × on a
layer removes it. Presets give one, two or three layers. There are no
layers that must not spend; the stranger of §20.2 is the one rule
against.

**Renaming.** A layer is renamed as a place is (§17.7, §19.12 item 2):
by a press on its name, while a vault of the flow is open, and the name
is kept only in the plan record each vault holds, beside the places'
names. A name says who and where, so it is never kept outside a vault:
not in the plan text on a stick, the flow's kept state or a file. With
no vault open, layers read "Access layer n" and places "Place n"; the
instructions of §20.5 use the names when they are known.

A route is "met" when, with only its list (any k of its places, and
what the layer knows), the wallet can be rebuilt and spent: at least `m` of its seeds (each with its passphrase where it has
one) and the description, whole or from enough shares (`check`, as the
board counts today). A vault counts only for a layer that knows its
password; a written vault password (§20.4) counts for any layer that
has the place it is kept at. A layer is met when every one of its
routes is.

### 20.2 The stranger

A **stranger** knows nothing and finds places. The plan's **stranger
margin** is the largest `k` such that a stranger who finds any `k`
places cannot spend. Faraday makes it as large as the wallet and the
layers allow: with more seeds, a larger quorum or more places it can
grow (a 3-of-5 leaves more room than a 2-of-3). A margin of at least 1
(no one place alone spends) is required unless a layer's list is one
place with no password, which makes that place alone spend; the board
then warns.

Second, as far as it does not lower the margin: the number of places a
stranger must find to see the balance (the description) is made as
large as it can be.

### 20.3 Working it out

**Access layers** is a card after **Start from** and before the board
(§19). Its **Work it out** runs the search and fills the board:

1. Over the plan's places, seeds, the description and its shares, and
   the vaults, Faraday chooses where each is kept and in what form
   (paper, a file on a stick, sealed in a vault) so that every layer is
   met.
2. Among the placements that meet every layer it keeps those with the
   largest stranger margin, then the largest margin for the balance,
   then the fewest things kept (copies, sticks, vaults, written
   passwords), then the fewest secrets kept unsealed.
3. The board shows the result. The person may change anything on it;
   every change checks every layer again and the margin, as lines
   under the board (in the All view, §20.7): one per layer ("Access layer 3: Place 1,
   Place 3, Place 4 · can spend") and "A stranger who finds any 2 places
   cannot spend". A layer no longer met, or a margin lowered, is a
   warning line, never a refusal (§19.5).

The search runs on the press, never while drawing. It is exhaustive over
the plan's places and seeds, which are few; where it would take longer
than the spec's bound (to be measured on the slow laptop, SPEED §7) it
shows the best placement found and says it is the best found.

### 20.4 Conflicts

When no placement meets every layer, or meets them only with a margin
of 0, Faraday says which layers conflict and why, in one line each
("Access layer 2 with Place 1 alone and a stranger at Place 1 cannot
both hold: Place 1 alone would spend"), and offers what would resolve
it, each a press:

- add a place to a layer's list, or a new place;
- give a layer a vault's password, or write a vault's password on paper
  at a place (a new thing on the board, **Vault n's password**, kept as
  words on paper only, Secret);
- add a vault;
- for a wallet still being made, a larger quorum or more seeds (only
  where the wallet's shape is not yet fixed).

### 20.5 Instructions, by hand

The map of which layer reaches what is sensitive: it is not printed and
not written to any file. For each layer Faraday shows, on screen only, a
template to copy out by hand: the places to go to, in order, and what
to take from each; the secrets needed, by name only ("Vault 1's
password"), never the secret; what to open and restore with. Leaving
the screen leaves nothing.

### 20.6 What it adds to the plan

The layers are part of the plan: kept with it (plan text, the vault's
plan record) and read back with it. A plan saved before this has no
layers and reads as one layer, Access layer 1, able to spend with
everything.

### 20.7 The layer view

At the top of the board a row of pills chooses whose view the board
shows, one at a time: **All**, **Access layer 1**, **Access layer 2** …,
**Stranger**. All is the default. The view changes only what is drawn,
never the plan. The checks under the board (§19.14 item 4) are the
chosen view's.

| Chosen | The board | The checks under it |
|---|---|---|
| **All** | As it is, and on each place card small numbered marks for the layers whose list holds it ("1 2") | The plan's checks and warning lines, as §19.5 and §20.3 |
| **Access layer n** | Its places drawn as they are, the other places dimmed; a vault whose password it knows drawn open, its contents readable; a vault it cannot open drawn shut, its contents hidden; the passphrases it knows marked | **Can spend**: yes or no. **Places it needs**: the fewest of its places that spend ("2 of its 3"). **Places it can lose**: how many of its places may be lost and it still spends. **Sees the balance**: yes or no |
| **Stranger** | A stepper, how many places the stranger found (from 1); every set of that many places is tried, and the board shows the set that gives the most | **Cannot spend with any k places**: the stranger margin (§20.2). **Cannot see the balance with any k places**: its margin. Each a yes or no for the stepper's count |

Several layers are not shown at once: each layer's checks answer for
that layer alone, and All's numbered marks show which layers reach each
place. The view is worked out once per frame through the drawing memo
(SPEED §3.3); a change of view is a press.

### 20.8 The search's bound and the batches (owner, 2026-10-19)

**The bound.** **Work it out** takes at most one second on the owner's
slow laptop. The search is not exhaustive in the literal sense: a 3-of-5
with three layers has far more placements than can be tried. It is a
branch and bound over the things to place (seeds, the description or
its shares, vaults, written passwords), each assigned places and forms
in turn, a branch dropped as soon as it cannot beat the best found on
§20.3's order (a layer it cannot meet any more, or a margin already
below the best). Places are interchangeable until named, so a placement
and its renumberings are tried once.

- The search counts its steps and stops at a **step budget**, not a
  clock, so it gives the same answer on every machine and in tests. The
  budget is set in L2 so the worst preset case (3 of 5, three layers,
  each with a paper route, two vaults) takes at most 250 ms on the
  build machine's release build; the owner confirms one second or less
  on the slow laptop with a test build's slow-event line (SPEED §3.11).
- Stopped by the budget, the board shows the best placement found and
  one line under it: "Best found; not every placement was tried".
- The search runs on the press of Work it out (and Looks right's
  questions, §20.10), never while drawing (SPEED §3.3).

**Batches.** Each a commit, `just` green, the board timed per SPEED §7
where drawing changes.

- **L1, the model** (§20.1, §20.2, §20.6, §20.9): layers and routes in
  the plan (`Answers`), the plan text and the vault's plan record, a
  plan saved before reading as one layer able to spend with everything;
  a route "met" and the stranger margin and balance margin as plain
  functions of the plan. Tests on those functions against hand-worked
  cases (a 2-of-3 with the Paper and vault preset; a 3-of-5 with one
  place a stranger can spend from), and the round trip.

  **L1 built** (2026-10-19): `faraday-core/src/access.rs` (`Route`,
  `Layer`, `Reach` with `route_met`, `layer_met`, `stranger_margin`,
  `balance_margin`, and for §20.12 a route's `paper`, `paper_met`,
  `every_layer_on_paper` and `kept`, which counts a copy at a place
  once whatever the layers; `Answers::access_layers`, `everything`),
  `Answers`
  `layers` written as `layers n` and `route` lines in the plan's text,
  `What::VaultPassword(v)` (`vpw0`, paper, Secret), a layer's name in
  the plan record's field 6 (`docs/VAULT.md` §7) and kept when the plan
  is saved again; tests `faraday-core/tests/access_layers.rs`. A route
  counts only its places and what it knows: seeds on their own devices
  and the watch-only software count for no layer and no stranger. A
  paper route counts only paper at its places (a written password
  among it) and what the layer remembers; with no stick on paper, no
  vault opens on it.
- **L2, Work it out** (§20.3, §20.4, this section's bound): the
  Access layers card (layers, routes, any k of n, passwords
  remembered or written, + layer and ×), the search, the conflicts and
  what would resolve each, the warning lines under the board. Tests:
  every preset's result meets every layer with the margin hand-worked
  for it; a conflict names its layers and its first resolution, pressed,
  removes it; the budget's result is the same on every run.

  **L2 built** (2026-10-19): `faraday-core/src/work_out.rs` (the
  search), `layers.rs` (the card's presses, Work it out, the conflicts),
  `layers_screen.rs` (the card), the lines under the board in
  `board.rs`; strings in `faraday-strings`; tests
  `tests/access_layers.rs` and `tests/backup_board.rs`. Its choices:
  - **The card** is `qstep::LAYERS`, after Start from and before the
    board, in Back up and in the flow's plan card alike. A preset still
    opens the board; the card is opened by a press. A plan that keeps
    no layers shows, and Work it out works to, the one a fresh plan
    starts with (`access::fresh`: a paper route of every place, any `m`
    of them, remembering the passphrases of the seeds loaded here); the
    first edit or Work it out keeps it in the plan. L1's reading of an
    old plan as one layer able to spend with everything stays for the
    model (`Answers::access_layers`); the card and the lines show the
    fresh layer instead. A route's vault password cycles Not known,
    Remembered, Written down (`Route::written`, ` written <vaults>` in
    the plan's text); a written one is kept on paper at one of the
    route's places, wherever the search finds best. Password rows are
    drawn off paper only.
  - **The search** places paper only at a place (a file on a stick
    spends as paper does for every route but a paper route and costs a
    stick); it does not place passphrases (a route counts those its
    layer remembers, and a seed with a passphrase no route remembers is
    not placed). A vault whose password nobody writes down holds every
    seed placed and the description (what a vault holds is not counted
    as kept); one whose password is written down has what it holds
    searched too. A vault the plan made that no route needs goes; one
    set up before the keys stays, holding nothing. Places the routes
    treat alike, and seeds they treat alike, are interchangeable (double
    lex). Bounds: both margins only fall as things are added; what is
    kept only grows, plus the most any one set of a paper route's places
    lacks, plus the sticks and vaults a route through a vault must have
    at sets no larger than the margin. Things are tried at one place
    first, then none, then more. Routes that cannot be met at all (any
    0, more places than it has, fewer usable seeds than the quorum) are
    left out of the search and are conflicts.
  - **The budget** is 400,000 steps (a step: one choice of places for
    one thing). The worst preset case (a 3 of 5; Access layer 1 through
    Vault 1 at Place 1 or any 3 of the five on paper; Access layer 2
    through Vault 2 at Place 2 or 3, or any 3 of Places 2 to 5 on paper;
    Access layer 3 any 3 of the five on paper) ends, every placement
    tried, in 320,119 steps, 163 to 168 ms on the build machine's
    release build; a case that runs the whole 400,000 steps took 196 to
    205 ms there. A 3 of 5 with its fresh layer takes 15,070 steps,
    about 7 ms; a 2 of 3, 114. The other shapes tried for a margin of 0
    add at most three eighths of the budget.
  - **Conflicts** read from the layers alone: a layer with no paper
    route (resolved by a paper route of its places); a route that can
    sign with fewer seeds than the quorum (by a passphrase remembered,
    where that would do; otherwise no press); a route any one of whose
    places meets it with nothing the stranger lacks (resolved by a place
    added to the route, a new one when it has them all, and two of its
    places needed; off paper by a vault's password known, or a vault
    added for it; by a passphrase remembered). A written password never
    resolves a route of one place: the stranger there reads it. Where
    Work it out finds a margin of 0 that none of these explains, it
    works the routes out for the other shapes of §20.12 item 2 (single
    key, 2 of 3, 3 of 5, an eighth of the budget each) and names the
    first that keeps every place from spending alone ("These routes
    need a 2 of 3; this wallet is a single key"), or says none does.
    With nothing in conflict the board opens on the result; otherwise
    the card stays, with the conflicts.
  - **The lines under the board** (the drawing memo): per layer
    "Access layer n: its places · can spend", or a warning "· cannot
    spend"; a warning when it has no paper route or cannot spend from
    it; the stranger's line, a warning at a margin of 0 or below what
    Work it out found for the same layers; "Best found; not every
    placement was tried" while the board is as a Work it out that ran
    out of steps left it. Layers are named "Access layer n" (names
    typed come with L3).
  - Removing a place takes it out of every route and moves the places
    after it down (`access::without_place`), on the board and the
    wallet chart.
- **L3, the layer view** (§20.7, §19.18 item 3): the pills, the
  board drawn per view through the drawing memo, each view's checks,
  the Stranger stepper, the layers' marks in All; a layer's name typed
  as a place's is. Snapshots of each view.
- **L4, instructions by hand** (§20.5): each layer's template on screen,
  nothing written. A test that leaving the screen leaves no file and
  nothing in the plan text.
- **L5, the questions in Back up** (§20.10): the cards, the result in
  words, Looks right and Change it on the board, a wallet's shape
  respected.
- **L6, Create starts with the questions** (§20.11): Plan the backup
  first, the suggestion and its alternatives, I know what I want, The
  standard way in Create and Back up.

DESIGN rows (§4.17 and a row for the Access layers card and the view
pills) come with L2 and L3.

### 20.9 Routes, thresholds and the paper route (owner, 2026-10-16)

- **Routes.** A layer may reach the wallet more than one way: an
  everyday route (a vault opened with its password, say) and a paper
  route for when a stick goes bad. Each route is checked on its own;
  the layer's checks (§20.7) are per route ("Access layer 1, route 2,
  paper: can spend").
- **Thresholds.** A route is "any k of" its places: a 3-of-5 wallet's
  layer may have all five places and spend from any three, as multisig
  allows. A route that needs every place is k equal to its count. The
  default comes from the wallet: an m-of-n wallet suggests "any m of n
  places", a seed at each. **Places it can lose** (§20.7) is the most
  places the layer can lose and still spend by some route.
- **The paper route** (required since 2026-10-19, §20.12). Sticks go
  bad, so Faraday offers every layer a route made of paper alone (a remembered passphrase may be part of
  it; no stick and no vault). It is optional: a layer without one is a
  warning line, "Access layer n has no paper route", never a refusal.
- **A written vault password.** A route through a vault asks whether
  its password is remembered or written down; written down, "Vault n's
  password" is kept on paper at one of the route's places (§20.4).
- **No balance-only layers.** A layer is for spending; there is no layer
  that only sees the balance.
- **No passphrase is required.** A route of paper alone with nothing
  remembered is open to anyone who finds k of its places; that is what
  keeping places safe means, and the stranger margin (§20.2) shows it.
  Faraday does not insist on a passphrase.

### 20.10 The questions (owner, 2026-10-16)

Back up opens on the questions, one card each, a likely answer already
chosen; **Start from**'s presets and the board stay for those who know
what they want. Explanations are in each card's guided walk-through,
not on the card (CLAUDE.md).

1. **How many access layers?** One, two or three.
2. For each layer:
   1. **How do they usually get in?** On paper only, or with a vault.
   2. With a vault: **Will they remember its password, or should it be
      written down?**
   3. **How many places will they have, and how many must they
      reach?** "Any k of n", from the wallet's quorum by default.
   4. With a vault: **A paper route too?** Yes by default; its "any k
      of n" suggested by Faraday.
   5. **Will they remember a passphrase?** Yes or no.
3. Once: **Paper, sticks, or both?** A preference Faraday follows where
   it can.

Faraday then works the board out (§20.3) and shows the result in words
first: a line per layer and route ("Access layer 1: usually Place 1
with the password; on paper, any 2 of Places 1, 2 and 3"), the stranger
margin, and what it takes ("3 places, 2 sticks, 1 vault, 4 paper
copies"). **Looks right** goes on to the places' names and The plan;
**Change it on the board** opens the board. Conflicts are §20.4's lines,
each with what would resolve it.

### 20.11 The questions choose the wallet; the standard way (owner, 2026-10-16)

**Create a wallet starts with the questions.** Its first card offers:

- **Plan the backup first** (the default): the questions of §20.10.
  From the answers Faraday suggests the wallet, in words with the
  reason ("3 of 5: Access layer 1 can lose any 2 of its 5 places"),
  and one or two others with what they trade ("2 of 3: fewer seeds,
  can lose 1 place"). The person chooses; the keys are made; the board
  opens filled from the answers, nothing asked twice.
- **I know what I want**: the kinds and quorums as Create offers them
  today (NEW-WALLET §2), no questions.

For now Faraday suggests only single-key wallets and standard multisig
quorums (2 of 3, 3 of 5, and so on); the other kinds Create makes
(MuSig2, Taproot trees …) are chosen by **I know what I want** only.

In **Back up**, for a wallet already made, the questions work within its
shape; answers that need another quorum say so ("These routes need a 3
of 5; this wallet is a 2 of 3").

**The standard way.** Wherever the questions or the board are offered
(Create's backup, Back up), **The standard way** skips them: the
person sees each seed's words and its SeedQR to write down, and the
description to show, save to a stick or print, in whatever forms they
pick, with no places, layers or board. It is a plain checklist of what
they chose. F31 still holds: a key made here is released when its
seed's copy is checked as F29 checks a copy (the words, or their
numbers, typed back), as every checklist releases it; the standard way
asks nothing more than that.


### 20.12 Layers first, then the wallet; every layer has a paper route (owner, 2026-10-19)

Revises §20.9's paper route and §20.11's suggestion.

1. **Layers and the backup's parts first.** Plan the backup first
   (§20.11) asks for the layers and what the backup has to work with
   (places, vaults, what each layer reaches, knows and remembers,
   §20.10) before any wallet is chosen. Faraday then suggests the
   wallet's shape from them.
2. **The shapes tried.** Single key; single key with a passphrase;
   2 of 3; 3 of 5. For each, Faraday works out the best placement
   (§20.3) and suggests the shape that meets every layer, each with its
   paper route, with the largest stranger margin (§20.2), then the
   fewest things kept; one or two others are shown with what they trade
   ("2 of 3: fewer seeds; a stranger needs 2 places"). The person
   chooses. One second covers all the shapes together (§20.8).
3. **Every layer has a paper route, required.** Replaces §20.9's
   optional paper route and its warning: a placement is not offered
   unless every layer can spend from paper alone (with what it
   remembers: a passphrase, a vault's password written on paper is
   paper). Where no shape allows it, that is a conflict (§20.4), with
   what would resolve it (a passphrase remembered, another place, a
   larger quorum).
4. **Layers may share paper.** The same paper copy at a place serves
   every layer whose route holds that place: a seed written once at
   Place 2 counts for Access layer 1's and Access layer 2's paper
   routes alike. Layers do not each need copies of their own; the
   search counts a copy once (§20.3 item 2, the fewest things kept).
5. **The stranger and the paper routes.** A layer whose paper route is
   any k of its places, with nothing remembered, lets a stranger who
   finds those k places spend: the stranger margin is at most k − 1
   there. A passphrase the layer remembers lifts that limit; the
   suggestion shows it ("Single key with a passphrase: a stranger who
   finds the paper cannot spend; Access layer 1 must remember the
   passphrase").
6. **Back up for a wallet already made** keeps its shape: the
   placement is worked out within it, and a layer the shape cannot meet
   is a conflict naming the shape it needs ("These routes need a 3 of
   5; this wallet is a 2 of 3").

Batches: items 3 and 4 are L1's model and L2's search; items 1, 2 and
5 are L5 and L6 (§20.8).

### 20.13 A route says what kinds of backup it holds (owner, 2026-10-19)

A route (§20.1, §20.9) says how many locations give full access, and
what kinds of backup each of those locations holds. It names kinds, not
particular seeds: in Create a wallet there are no seeds yet. Faraday
suggests the wallet's shape from the routes (§20.12): how many keys,
and how many of them spend. It never makes or chooses a key: the
person fills each key on Create's Keys step as today (a key already
loaded, a vault's key, New key with dice, coins or any other source),
and Faraday walks them through it. Once the keys exist, Work it out
decides which key's seed and which part of the description go to each
location, and the board shows it to be changed by hand.

1. **What a route says.** Under each route on the Access layers card:
   - **Full access from**: any k of its n locations (k may be n, or 1).
   - **At each location** (every location of the route alike):
     - **Seeds**: how many (0, 1, 2 …), and in what form: words on
       paper, or a file on a stick;
     - **Wallet description**: none, part, or whole;
     - **Vault**: none, or one, its password remembered or written
       down (not on a paper route, where seeds and the description are
       on paper and a vault is none).
   - **They remember**: a passphrase, a vault's password (as §20.1).
   Left unset, a route's kinds are Faraday's to choose, as in L2.
2. **Different kinds for one layer** are different routes: a layer that
   usually opens a vault at one location and also has a paper route over
   three has two routes, each with its own kinds. Every location of a
   route holds the same kinds (owner, 2026-10-19).
3. **What the kinds mean.** Any k locations of the route together must
   give full access: k × seeds per location at least the wallet's
   quorum m, as distinct seeds across every k of them; the description
   whole at each location, or a part at each, any k parts enough; a
   vault holds what the route's locations need in it, and the route
   knows or finds its password.
4. **Faraday suggests the wallet** (Create, Plan the backup first): the
   shapes of §20.12 item 2 that every route's kinds allow, ranked as
   there; the person chooses one, then fills its keys as Create's Keys
   step does today (NEW-WALLET §2, §15, §16.1); Faraday adds no key of
   its own. Example: Layer 1, 1 location, a vault, password remembered;
   Layer 2, paper, any 2 of 3 locations, 1 paper seed and part of the
   description each: a 2 of 3, the description in 3 parts any 2
   enough. The person makes or picks the three keys; then Work it out
   puts 1 seed and a part at each location for Layer 2 and 2 seeds and
   the whole description in Layer 1's vault; a stranger at one location
   has 1 seed and a part.
5. **In Back up** the wallet exists: its shape is fixed, the kinds
   constrain Work it out's placement, and a route the shape cannot meet
   names the shape it needs (§20.12 item 6).
6. **Conflicts** (§20.4): a route whose kinds cannot give full access
   (2 locations of 1 seed each for a 3 of 5), or that would let a
   stranger spend from fewer locations than another route's own k,
   names the route and why, with its resolutions as presses: more seeds
   per location, more locations, the description whole, a passphrase
   remembered.
7. **Kept with the plan** as the route's other answers (plan text,
   vault plan record), read back; a route without kinds reads as
   unset.
8. **Tests** (`tests/access_layers.rs`): §20.13 item 4's example gives
   a 2 of 3 with those placements; a route of 1 location, 1 paper seed
   and the whole description with a passphrase remembered gives single
   key with a passphrase; 2 locations of 1 seed each against a 3 of 5
   in Back up is a conflict naming the route; unset kinds give L2's
   results unchanged; the kinds round-trip.

Built as L2b (the kinds on the card, in the model and constraining Work
it out in Back up) and L6 (picking the wallet in Create, §20.8).

### 19.15 The board in rows, and its colours (owner, 2026-10-16)

Decided from mockups after F2, replacing §19.1's rows of place cards
and §19.14 item 2's colours. Built in F2c.

1. **One place per row.** The places stand one under another, each the
   whole width of the main column, `BOARD_PLACE_GAP` (20 dp at scale 1)
   apart. A place's row: its header band (the place glyph, its name,
   its line of fact as an outlined tag, its ×), then left to right its
   stick boxes, a **+ stick** box, and Paper (or Scanned in at the
   software place). The stick boxes share the width; when they do not
   fit at a chip's width each, they wrap onto a second line inside the
   place. A row is as tall as its tallest box. **+ place** is a row of
   its own after the last place; the software place is last. In the
   narrow column the parts of a place stack instead.
2. **One hue per kind of box**, pale fills and strong edges, colour
   carried by edges and header bands, never by colour alone:

   | Box | Fill | Edge | Header | Glyph |
   |---|---|---|---|---|
   | Place | a pale green | green, with a thick left stripe | a green band, the name in dark green | ⌂ |
   | Stick | a cool slate grey | slate | "Stick n" in slate | ▭ |
   | **+ stick** | the stick's fill | the stick's edge, dashed | "+ stick" | |
   | Vault | a pale accent tint | the accent, 2 dp | a solid accent band, "Vault n · sealed" on it | ▣ |
   | Paper | a warm cream | brown | "Paper" in brown | ✎ |
   | Chip | the surface, a faint shadow | a stronger neutral | | |

   Every box's edge is at least 3:1 against the fill around it and all
   text at least 4.5:1 on its fill, in every theme. The light theme's
   values are the mockup's (place #EDF6EF / band #DCEEE1 / edge #5E9A73
   / text #1E5B35; stick #E3E8ED / #7D8B98 / #36424D; vault #D3EAF3 /
   #0B7A9E, band #0B7A9E with white; paper #FBF5E6 / #977A36 / #6A501A);
   each other theme has its own, by the same rules, named colours in
   the palette and DESIGN §4.17.

### 19.16 + stick, and a vault kept at no place (owner, 2026-10-16)

Decided from the owner's test of F2c: **+ stick** did nothing when
pressed, which read as broken until a drop on it was tried; and a vault
whose sticks had all been taken off (its places removed) still stood in
The plan with what it held, until its chip was taken off the list.
F40. Built in F2d.

1. **A press on + stick adds a stick.** With nothing held, a press on a
   place's **+ stick** adds an empty stick box there, after its sticks:
   "Stick n" with its ×, and **+ stick** after it again, so a place can
   be given several empty sticks in a row. A drop on **+ stick** is as
   before (a new stick for what is dropped).
   - An empty stick is the board's, not the plan's: it is kept in the
     board's state (`BoardState`), never on the map, so The plan, the
     checklist, the checks and the plan's text never see it (the map's
     rule that a stick holding nothing goes, §19.4, stands).
   - A drop on an empty stick (a file, or a vault's chip on its stick
     part) makes it the place's next stick of the plan, the same edit as
     a drop on **+ stick**; the empty sticks after it keep their order.
     Empty sticks are drawn after the place's sticks of the plan, so
     their numbers follow on.
   - Its × removes it. Removing its place removes it; the places after
     move down with theirs.
   - The board's empty sticks go when the board is left (its card closed
     or another screen opened): nothing is lost, since they hold nothing.
   - After the lock (§17.8) a press adds one as before; it takes what a
     stick takes after the lock, a vault's stick.
   - In the narrow column the same: a press on **+ stick** adds one.
2. **+ stick shows it takes what is held.** While a chip that **+ stick**
   takes is held, by a tap or a drag, its dashed edge is drawn solid in
   the stick's edge colour, at every place, so the person sees where a
   new stick can be made before the pointer reaches one. The accent edge
   round the zone that a drop or tap would place on is as before. No
   line of explanation is added (a working screen carries none).
3. **A vault kept at no place is not in the plan.** A vault no place
   keeps a stick of stays on the board's list, its chip drawn dim (and
   still picked up, unlike a seed that is not here), with what it holds,
   and a drag of its chip onto a place puts it back with all of it.
   Everything but the board reads the plan without it:
   - The plan step (`map_rows`) does not list it; the checklist makes
     no item for it; the flows do not set it up or write it.
   - The checks do not count what it holds: a seed kept only there is
     "Seed X is kept nowhere", and holds Make the checklist as §19.5
     says.
   - "Vault n holds nothing" counts only a vault kept at a place, so a
     vault just added by **+ vault** holds nothing back.
   - The map as saved (`Answers::map`, the plan's text) keeps it, so the
     board reads back as it was left.
   - Model: `plan::map` is the plan, the stored map without a vault no
     place keeps a stick of; the board reads and edits the stored map in
     full (a function of its own), and a new vault's number and the
     list's vaults count every vault in it.
   - **A vault that exists already** (set up before the keys, §17.7)
     at no place is dim and out of the plan as any other, and "Vault n's
     stick needs a place" stands under the board, a warning only: it no
     longer holds Make the checklist (§19.5, warnings never refusals).
     Its × takes it out of this plan (asked first when something is
     planned into it, as today): the warning goes, the chip stays on the
     list dim, since the vault exists, and the vault's file is not
     touched; dragged onto a place it is back in the plan, holding
     nothing planned. A vault the board made (**+ vault**, a preset) has
     no warning at no place, and its × removes it from the list as today.
   - After the lock, a vault's stick and the vault change as §17.8
     already allows; nothing here widens it.
4. **Tests** (user-facing behaviour, `tests/backup_board.rs`): a press
   on **+ stick** with nothing held shows a new empty stick and a
   further **+ stick**, and The plan and the checklist are unchanged;
   a file dropped on the empty stick is kept on a stick of the plan at
   that place; the empty stick's × removes it; a place removed takes
   its empty sticks; with places 2 and 3 removed, their vaults are not
   in The plan or the checklist, are still on the list with what they
   hold, drawn dim, and a seed kept only in one of them is kept
   nowhere; a vault dragged back onto a place returns with what it held;
   a vault set up before the keys, at no place, reads "needs a place"
   and Make the checklist is still offered; its × takes the warning
   away and leaves its chip dim on the list, and dragged onto a place
   it is in The plan again.

### 19.17 The files box, and the stick and vault glyphs (owner, 2026-10-16)

Decided from the owner's look at F2c (tracker #146): a stick carrying a
vault was nearly all vault box, its own area a thin margin, so where to
drop a thing to keep it on the stick but not in the vault could not be
seen. Built in F2d with §19.16.

1. **A files box in every stick box.** Inside the stick box, under its
   head and above the vault box, a box of its own for the files kept on
   the stick outside any vault: header "Files · not sealed", in the
   files colours (DESIGN §4.17, a pale blue, light and dark per theme).
   Under its header the stick's files, chips laid as a zone's; empty, it
   keeps one chip row's height. Every stick box has it, with a vault or
   without, so the target is the same on every stick. It has no ×:
   taking the stick off removes it with all it carries.
2. **Drops.** A press or a drop on the files box keeps the thing on the
   stick as a file; on the vault box, it seals it in the vault, as
   before. The rest of the stick box (its head and margins) still takes
   a drop as a file, as today, so nothing that worked stops working;
   while a chip a file can be is held, the files box is the edged
   target, not the whole stick box. After the lock (§17.8), the files
   box takes what a stick takes then.
3. **The software place's stick** (public files) is the same: its files
   in a files box, no vault box.
4. **The stick glyph.** A new USB-stick glyph (a body with its connector
   at one end), Faraday's own drawing, replaces `Icon::Drive` everywhere
   a stick is meant across Faraday and the Starter: the stick box's head,
   the medium's glyph (`medium.rs`), the Starter's drive rows. Where the
   medium is an SD card (the Pi, F4) the same places draw `Icon::SdCard`
   and say "SD card n" where they say "Stick n" (`Medium` already words
   it). A drive that is not a stick (the Starter's internal disk, if it
   has one) keeps `Icon::Drive`.
5. **The vault glyph on the board.** The vault box's header draws the
   closed lock (`Icon::Lock`) in `VAULT_ON`, as the chips sealed in it
   carry, in place of `Icon::Vault`; the vault's chip in the list
   (`Thing::Vault`) takes the closed lock too.
6. **Tests** (`tests/backup_board.rs`): a drop on the files box of a
   stick that carries a vault keeps the thing on the stick as a file,
   not in the vault; a stick with no vault takes a drop on its files box
   the same; the files box is there, empty, on a new stick. Snapshots of
   a stick with a vault, with and without files beside it, in the narrow
   column and the wide layout.

### 19.18 A place's name, pressed, is selected (owner, 2026-10-16)

Tracker #147: a press on "Place 1" to rename it showed a blinking caret
after the name, which says typing adds to it, though typing replaces it.

1. **The press selects the whole name.** A press on a place's name (while
   a vault is open, §19.12 item 2) focuses the field with the whole name
   selected, drawn with the text field's selection highlight, and no
   caret.
2. **The first key replaces it**: a character replaces the name with
   itself, Backspace or Delete empties it; from then on the field is an
   ordinary text field, the caret at the end of what is typed. An arrow
   key or Home or End lets the selection go and puts the caret there,
   as any text field does.
3. **Access layers' names** (§20.1) are pressed and typed the same way.
4. **Tests** (`tests/backup_board.rs`): after the press, one key typed
   gives a name of that one character; a snapshot of the name just
   pressed, and after one key.
