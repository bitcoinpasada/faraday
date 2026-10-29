# The backup flow

**Kind:** Spec
**Status:** Partly built: GPG keys, the Secure Boot keys and the Bitcoin wallet
**Open:** the Nostr key's part (`docs/NOSTR.md`, not scheduled); backups age, when each copy was last checked and a recovery path's wait (§18, planned, not scheduled)

Decided by the owner (`docs/DECISIONS.md` F31; the plan by place F35,
the backup board F37).

The build log is `docs/history/BACKUP-FLOW-LOG.md`.

One guided flow makes and backs up every kind of secret Faraday keeps
in a vault: a GPG key, a set
of Secure Boot keys, a Nostr key (`docs/NOSTR.md`), and the keys of a
Bitcoin wallet with its backup plan (`docs/history/NEW-WALLET.md`). The person
first says where the key will be kept; Faraday reads every stick that
plan uses while nothing secret is open; the vaults are unlocked or made;
the key is made straight into them; one lock seals them; and each stick
is written in turn, by its place. The person stays in the flow from the
first step to the last, and it never shows the stick visit page.

**Why a document of its own.** Four of the five kinds are not wallets,
and the Bitcoin wallet's plan uses the same steps around its own
checklist. The flow is written here once; `docs/history/NEW-WALLET.md` §14.8,
`docs/NOSTR.md` §5 and
`docs/FLOWS.md` decision 59 say where each kind enters it.

## 0. Rules for the implementing agent

Rules still in force:

- A stick is handed only to a clean process (`docs/history/PLAN.md` §5.3 and §5.4).
  Nothing in this flow opens a secret in a process that holds a stick,
  or hands a stick to a process that holds a secret. A change that would
  need either stops and asks the owner.
- The flow is driven by the plan and narrows nothing the plan can say
  (§2). A change that would make the flow refuse a plan the plan screens
  accept stops and asks the owner.
- Every string has the prefix `backup_flow_` (§12), in
  `faraday/faraday-strings/src/en.rs`.

The agent's working rules: moved to `docs/history/BACKUP-FLOW-LOG.md`.

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
  (GPG, Secure Boot) is planned at **two places or more**: the
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
  be made again from seed words: GPG keys, the
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
| `At::Place(p)` holding words, SeedQR, a passphrase, the sheet, shares, codex32 strings | the checklist's paper items (Templates, printed from Faraday Starter; Copy with F29's check; Codex32 strings, §17.23; Passphrase, typed back, NEW-WALLET §14.7 item 5; Sheets) |
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
made into both. For GPG and Secure Boot keys a place can be
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
| GPG key | the primary key's fingerprint | Tools' **GPG key**, Vault contents' **Make a key** | name and email, expiry | the key's page | Sign a file, Renew |
| Secure Boot keys | the owner GUID | Tools' **Secure Boot keys**, **Make keys** | PK, KEK, db | the Secure Boot screen | Enrolment files, Sign or check an image's Sign |
| Nostr key | the npub | the Value card's **Save into a vault** | the Entry (NOSTR §5), with the key loaded | the saved entry's page | nothing |
| A Bitcoin wallet's keys | the descriptor checksum and the fingerprints of keys made here | Make a new wallet, Restore, Back up (§10) | New key's seeds, or the restored seed | the wallet card and the checklist | spending, NEW-WALLET §14.3 |

These tiles no longer ask for an open vault first (decision 10's "Make a
vault first"): step 1 is where the vault is decided. A key, or a public
value of it, is never on a stick before step 6.

**What does not wait**: everything that is not a new signature by the
key: a GPG key's public key and revocation
certificate (made For the stick with the key, decision 24), Secure
Boot's **Check** under **Sign or check an image**. Enrolment files carry signatures by PK and
KEK, so they wait.

## 4. The steps

### 4.1 Where it will be kept

A step card of rows, a place's options as buttons under its row
(DESIGN §4.2, Option buttons):

- one row per place, "Place 1", "Place 2", with what it keeps; **Add a
  place**; a place's buttons choose **A stick with** a vault (Vault 1,
  Vault 2, **A new vault**) or nothing; for a wallet, the paper forms
  stay in Back up's plan (NEW-WALLET §4 and §5);
- one row per vault: "Vault 1 · new" or "Vault 1 · an existing vault",
  with the places that keep its stick; for a wallet, the seeds it holds
  (`vault v`);
- the check lines under them, as the plan's map panel draws them, for
  information.

**Continue** is live whatever the count the kind allows (§1): two or
more places for GPG and Secure Boot, one or more for a wallet, any
for Nostr. A plan with no stick goes straight to step 4. A wallet's
plan shows its check lines and, where they apply, §10.3's two lines.

### 4.2 Bring in the sticks

Only the sticks the plan already names: each `VaultStick(v)` at a place.
Sticks the plan names later (the stick of files, the stick of public
files, a place added after the key is made) are checked at
step 6 instead (§4.6), since they hold no vault the key goes into.

- **A clean process first.** With anything secret open when step 2
  begins, its card's foot is **Lock and bring in the sticks**: the lock
  of `docs/history/PLAN.md` §5.3, the flow's state kept.
- **One place at a time, by the plan**: "Insert the stick for Vault 1
  (Place 1)". Two sticks in: "One stick at a time".
- **Already read this session.** A stick seen this session (the boot
  stick, a visit), whose listing and free space Faraday kept, is offered
  on the card: **Use {label}, read at {time}**, with no insertion, as
  long as its room is known to be enough. That press names the stick:
  once read it is bound to its place with no second tap, unless it reads
  like a stick bound to another place, which asks for the hold below
  (owner, 2026-10-25).
- **Usable**, else why and what to do, before anything is made:
  - not FAT32 as Faraday reads sticks: "Not a FAT32 stick. Format it as
    FAT32, or use another stick";
  - an `FARADAYBOOT` partition: "This is a boot partition. Use another
    stick";
  - read-only: "Cannot be written. Use another stick";
  - too little room: "Not enough room: {needed} needed, {free} free".
    The room is the vault's length, twice where a file of the same vault
    is there to be replaced (VAULT §6 writes beside, then removes), and
    the plan's other files at that place where already known.
- **Which vault.** The stick's `.fvlt` files are listed by name (`vault.fvlt`,
  `vault-2.fvlt`, …), each with its size and, when seen open this
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
    the other is written over at step 6 only after a hold (decided, §14
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
"Pull the stick to unlock"). No vault is unlocked until every stick of
step 2 has been read; then the step offers what fits, each vault on its
own row, in the order the person chooses, and unlocks nothing by itself
(owner, 2026-10-26; F121):

- each existing vault found: **Open {vault}**, then its passphrase and
  **Unlock {vault}**, with Unlock's memory check; a vault kept at several
  places is unlocked once;
- each new vault: **Create {vault}**, Create a vault (decision 21's
  Size, Name and passphrases), its own passphrase, once per vault of the
  plan. Where only Create applies (no vault found on the sticks, none
  open in memory), Create a vault opens for the first new vault by
  itself once the last stick is bound; while a stick is in, the Pull
  sheet comes first, "Pull the stick to create the vault, then your
  key", and Create a vault opens when the stick is out (owner,
  2026-10-25);
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
with no stick skips steps 5 and 6. For a wallet of Make a new wallet,
the idle lock waits until this step (`docs/history/NEW-WALLET.md` §18, F58).

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
- **A stick holding an existing vault the key should go into**: not
  offered. An existing vault is brought in only before the keys (§10.2,
  §17.21 item 5; owner, 2026-10-20). The extra round
  **Add {vault}** that was specified here, and its offer for a key kept
  only on paper, are not built: moved to `docs/history/BACKUP-FLOW-LOG.md`.

**Removing a place:**

- **Before its write**: the key is taken out of that place's vault in
  memory; a vault that was new and held only this flow's keys is
  dropped; the stick leaves step 6.
- **After its write**: the copy stays on the map as what it is until
  **I destroyed this copy** (NEW-WALLET §14.4 item 1, §9.4).
- **The minimums hold**: a GPG or Secure Boot plan keeps two
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
  build of Faraday Demo** (F30) uses its stick folders as sticks; the
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

**A. A GPG key, one vault at two places** (the default).
Where: Vault 1 new, its stick at Place 1 and Place 2 → Insert the stick
for Vault 1 (Place 1): empty, bound, tap → pull → Place 2: empty, reads
as Place 1's did, so **Hold: this is the stick for Vault 1 (Place 2)** →
pull → Create Vault 1, its passphrase → name and email, **Make the key**
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
→ Number of vaults: 3 → insert the stick for Vault 1: it holds `main.fvlt`
and `main-2.fvlt`; "Which vault is Vault 1?" → `main.fvlt`, copied in;
tap → the stick for Vault 2: empty, checked, bound; tap → the stick for
Vault 3: empty, reads as the second did, so the hold → every stick is
read → the step offers Open Vault 1, Create Vault 2 and Create Vault
3: Open Vault 1 and Unlock; Create Vault 2 and Vault 3, each its own
passphrase → New key three times, into memory → Back up's plan: Key 1
into Vault 1, Key 2 into Vault 2, Key 3 into Vault 3, a place each → the
rest of the plan and the checklist, each vault's **Save into** → Lock
and write → the stick for Vault 1 (Place 1): `main.fvlt` replaced,
`main-2.fvlt` untouched → Vault 2 (Place 2) → Vault 3 (Place 3) → Done.

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

`faraday-core/src/backup_flow/mod.rs`, `BackupFlow`;
`BackupFlowArea` (`faraday-core/src/backup_flow/area.rs`) on `Faraday`,
read through `Faraday::backup_flows()`, at most one on screen.

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
(decided, §14 item 4).

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
  the flow (§14 item 2).

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
  the rest or stop there (decided, §14 item 2). The Power
  sheet lists it first, in the danger tone: "Backup of {key}: written to
  {n} of {m} places".
- **A vault changed after a stick was written** (left, unlocked,
  changed, locked): `sealing` takes the new bytes; written sticks stay
  written for every key their sealing held, and show "Changed since
  written" as any copy does.
- **A stick's first copy deleted** (decision 54, a wipe): that stick's
  card opens again, "The stick for Vault 1 (Place 1) no longer has the
  vault".
- **Changing the plan part way**: §4.8 before the lock, §15.8 after it.
  A lock during the round that fills a vault again (§15.8) resumes on
  its Vaults card; a power-off ends it, the key's vaults sealed For the
  stick lost with the rest of `/run/faraday`.

## 10. The Bitcoin wallet

### 10.1 What the code does today

Moved to `docs/history/BACKUP-FLOW-LOG.md`.

### 10.2 The change

(Every bullet below keeps what the plan can say: §2.)

- **Make a new wallet** asks about vaults after its kind and shape and
  before Keys, and its plan is made in Back up after the keys (owner,
  2026-10-15). In order:
  1. **Whether vaults are part of the backup.** A question with Yes and
     No: "Will you use vaults as part of this backup?", and under it,
     in the muted style, "Vaults already on sticks are brought in here,
     not later." (§17.21 item 5: a stick not read this power-on cannot
     be read after the keys without the lock, which wipes them). It is
     not done, and shows no check, until the person has answered it and
     pressed Continue. No goes on to Keys.
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
     and its sticks are named "the stick for Vault 1". Its two cards
     have Make a new wallet's Guided switch: Bring in the sticks says
     why the sticks are read before any key exists (what is checked and
     read on them, that nothing is written until the lock, and that a
     stick is read with a key in memory only after that lock), and
     Vaults says why the vaults are made or unlocked before the keys.
  4. **Keys**, made into memory, as before. No key or wallet description
     is written to any vault by the setup.
  5. **Back up's plan**, after the keys, as it was before F31: which
     seeds go into which vault, how many places, the paper forms, SeedQR,
     files, the passphrase, the wallet description, the software. The
     vaults set up in step 3 are the plan's vaults, numbered in the order
     they were set up; the plan offers at least that many vaults, however
     many seeds there are, and shows each by its name. The checklist's
     vault item saves into the vault set up for it.
  6. **The checklist and Write to sticks**, as batch B1b built them (`docs/history/BACKUP-FLOW-LOG.md`): one
     lock, each stick by place. A plan that keeps a vault at more places
     than the one stick it was set up on writes the extra copies at the
     write step, to new sticks checked when they are inserted, as an
     added copy is (§4.8). A vault the plan adds after the setup is a new
     one, made in the checklist (§17.21 item 4).
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
    sheet or shares as PDFs, to print. It goes to an online computer,
    so it never keeps a secret file (a seed's words, SeedQR or
    passphrase, or a codex32 string, as a file). Where a place that
    keeps a vault's stick holds the wallet, it is that place's first
    stick, in the flow's order, that keeps no secret file; else, where
    the plan's `At::Files` spot holds the wallet and no seed's file,
    that spot's stick; otherwise a stick of its own.
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
  (owner, 2026-10-14), short of a secret file. Where the plan's map puts
  them on a stick that also keeps a vault copy (a chart edit,
  NEW-WALLET §9.7), the flow writes them there, with no warning and
  nothing blocked. A secret file never shares their stick, so a plan
  accepted at the lock can be written: the stick is chosen as above,
  and **Lock and write** refuses a plan that would still put a secret
  file on it, before anything is sealed or made, naming the file and
  the stick (`backup_flow_public_secret`). The backup's Learn page
  states, as a fact, that a stick of public files is read on the online
  computer that runs the software or prints the sheets, and that
  whatever else is on that stick goes there with it (§14 item 6).
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
  templates, {words} words, {copy}", with one clause more per codex32
  length the plan places ("3 from Blank templates, codex32, 48
  characters", `docs/CODEX32.md` §9.3, §17.23), and its foot is **Done**, a press,
  as Envelopes' is. Both presses are kept with the wallet's backup
  record, as are the descriptor shown as a code and a passphrase typed
  back, so Write to sticks' lock and the unlock after it leave them
  done. No template is written to a stick, and
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
secret of the wallet: every seed of the wallet, loaded here or not, in
any form (words, SeedQR, a vault whose stick is there; a seed file, on
the stick of files, is at no place and does not count, nor is a seed
kept on its own device), and every passphrase of those seeds (on
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
| `backup_flow_vault_stick` | the {m} for {} ({}) (vault, place) |
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
| `backup_flow_confirm_place` | This is {} |
| `backup_flow_hold_confirm_place` | Hold: this is {} |
| `backup_flow_bound_place` | This is {} |
| `backup_flow_both` | One {m} for both |
| `backup_flow_pull` | Pull the {m} |
| `backup_flow_no_passphrase` | I do not have its passphrase |
| `backup_flow_new_vault_beside` | A new vault beside it |
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
| `backup_flow_other_files` | This {m} has {}'s files |
| `backup_flow_writing` | Writing {} |
| `backup_flow_written` | Written and checked on {} |
| `backup_flow_written_place` | Written to {} |
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

**Unlock {vault}** and Create a vault's strings are reused.

## 13. Tests

User-facing behaviour, through `Action`s and the test kit's sticks, in
a new `faraday-core/tests/all/backup_flow.rs`; the wallet's in
`tests/all/backup_checklist.rs` and `tests/all/backup_first.rs`.

- **The order.** A GPG and Secure Boot Make starts on **Where it
  will be kept**; Make the key is not offered before its sticks are
  read and its vaults open; with a vault open, step 2 locks first.
- **Step 2.** An empty stick is bound to the plan's new vault with no
  question and offered no copy; a stick holding `vault.fvlt` and
  `vault-2.fvlt` at a place the plan gives an existing vault asks which,
  copies in only that one, and step 6 leaves the other byte for byte; at
  a place given a new vault, **A new vault beside them** is chosen; a
  stick bound to Place 1 inserted for Place 2 is refused; a second
  empty stick of the same label and serial needs the hold; a non-FAT,
  read-only, full or `FARADAYBOOT` stick says why and binds nothing.
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
- **What waits.** A key made in the flow: GPG's Sign a file
  and Renew, Secure Boot's Enrolment files read **Back up first**, and its
  Sign opens the flow, until every place is written, then work after the unlock;
  the same keys unlocked from a stick work at once; a Nostr entry saved
  in the flow exports at once.
- **Every form.** Example C writes three sticks, `main-2.fvlt` untouched;
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
- **Places.** A GPG or Secure Boot plan cannot go below two
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
  one stick to step 6. Removing a place before its
  write drops a new vault holding only this flow's keys; removing one
  after its write keeps it on the map until **I destroyed this copy**.
  A GPG or Secure Boot plan cannot drop below two places.
- **Events.** A stick pulled while writing counts nothing; with no flow
  in the kept state (a power-off) no flow is offered.

## 14. Decided

The owner's answers (2026-10-14).

1. **Places by kind**: two or more for GPG and Secure Boot keys,
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
7. **A key kept only on paper** when a vault is filled again after the
   lock (§15.8): it is typed back in from its copy, **Add the key from
   its copy**. (The choice that stood here belonged to the extra round
   for an existing vault, which is not built:
   `docs/history/BACKUP-FLOW-LOG.md`.)
8. **A stick holding vaults, at a place the plan gives a new vault**:
   **A new vault beside them** first and chosen.

## 15. The plan by place (F35)

(owner, 2026-10-15.) Back up's plan for a Bitcoin wallet is asked place
by place. The questions "The seeds go" and "The wallet description goes",
which asked where each kind of thing goes, are gone. The person says how
many places there are, then fills one card per place with what that
place keeps, then decides the wallet description and where it goes,
then sees the whole map and approves it. The other kinds' plan (§2) is
already by place and does not change. Make a new wallet's vault question
and count, and the vaults set up before the keys (§10.2), do not change:
they are the vaults the place cards offer.

### 15.1 The cards

In this order, as step cards on `wide` and a page each on `small`
(NEW-WALLET §14):

1. *(Replaced by §17.21 item 1: no Start from; the presets only build
   plans.)* **Start from**: moved to `docs/history/BACKUP-FLOW-LOG.md`.
2. *(Replaced by §17: the board.)* **Places**: moved to `docs/history/BACKUP-FLOW-LOG.md`.
3. *(Replaced by §17: the board.)* **Place 1**, **Place 2** …: moved to
   `docs/history/BACKUP-FLOW-LOG.md`. The place's line of fact is `plan::alone`, as §17.1 draws it.
4. *(Replaced by §17.19: shares are added by one button and Faraday
   picks the keys left off.)* **The wallet description**: moved to
   `docs/history/BACKUP-FLOW-LOG.md`. The shares table, with its slider,
   is drawn on the checklist's Shares card (`shares_card`).
5. *(Replaced by §17: the board.)* **Where the description goes**: moved
   to `docs/history/BACKUP-FLOW-LOG.md`. In {vault} with shares keeps the shares there (§15.6);
   the old `At::Files` stick stays read-only (§17.3).
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

### 15.2 The model

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
  A seed's words, SeedQR or passphrase file, or a codex32 string's
  file, put on a stick at a place asks **Save without encryption?**'s
  hold at the drop, once for that file (F129): **Cancel** leaves the
  plan as it was before the drop, and a file held for asks nothing at
  **Lock and write**.

### 15.3 Strings

New, in `faraday/faraday-strings/src/en.rs`: the place card's rows
("Seed words by hand", "SeedQR by hand", "Seed as a file, unprotected",
"Passphrase"), the sheet's title "{} · {}", "{} with its passphrase",
"Also at {}", the description card's title "The wallet description" and
rows "Whole", "In shares"; "Where the description goes", "On paper", "As
a file", "In {}", "Shares here", "Not a place", "Into watch-only
software"; "The plan"; "Files ({})". Strings left unused ("The seeds go",
the old seed and description rows, "Each place keeps") are removed.

### 15.4 Tests

`faraday-core/tests/all/backup_plan.rs` (the wallet's plan, through the
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

### 15.5 Docs

`docs/WALLETS.md` §5 (the questions), `docs/history/NEW-WALLET.md` §5.2 (marked
"Changed by BACKUP-FLOW §17", now §15), `docs/DESIGN.md` §4.2's "Used for".
`docs/learn/04-backups.md` is not part of this batch: leave it, and say in the
report what in it no longer matches.

### 15.6 A share in a vault

(owner, 2026-10-15: "people should be able to store things in vaults or
not in vaults; up to them".) **In {vault}** with shares keeps the shares
ticked at that place in that vault, as §15.1 item 5 says; nothing turns
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
- **The checks** already count a share in a vault (§15.2).

**Tests.** `faraday-vault`: a slot with two type 13 records seals and
opens with every field; a type 13 record without field 1 refuses the
slot. `faraday-core/tests/all/backup_flow_wallet.rs` (or the vault item's
own test file): a 2-of-3 with In shares and Share 2 in Vault 1 at Place
1: Save into Vault 1 puts that share there, the item is done only then,
and the sealed vault holds it after the lock. Restore with that vault
open and Share 1 in the Inbox rebuilds the wallet.

### 15.7 The cards in the guided flow

(owner, 2026-10-15.) The flow's plan card is the same place cards, drawn
by the same code (`plan/cards.rs`), on the flow's own copy of the plan.
Since F2b the place cards are the board (§17, §17.12 item 1), drawn by
`board/screen.rs` on the flow's plan; the keys left off each share are
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

### 15.8 Changes after the lock

(owner, 2026-10-15.) After **Lock and write** no wallet and no key is
loaded, so the plan changes there only in **what goes into a vault and
where a vault's sticks are kept.** That is a seed, its passphrase, the
description, a share or a codex32 string in a vault; a vault's stick added to or taken
from a place; a place added, or the last place taken away while it
keeps only vaults' sticks (a place before another is not taken away:
the later places would be renumbered, sticks already named by place
among them). Each such change sends the vaults it touches back to be
filled, sealed and written (§10.2), in a round of the flow
(`made_plan_changed`, the flow's `adding`):

- A vault whose contents changed (`sealed_marks` differ from what it
  was sealed with) loses its sealing, and its sticks already written
  count as not written. A vault the plan adds is made in the round,
  with Create a vault's form.
- The flow goes back to its Vaults card. The person unlocks that vault
  and, of the vaults that hold the keys, those holding the seeds it
  takes (`flow_needed_vaults`). **Copy the key** ("The new vault takes
  the key from one that holds it") puts into it each seed, passphrase,
  the description, each share and each codex32 string the plan now
  gives it, and takes out what the plan no longer does; every vault of
  the round takes its
  part, or none does. A seed no open vault holds (kept only on paper,
  wiped at the lock) is named, "{seed} is kept only on paper", with
  **Add the key from its copy** to type it back in. A codex32 string a
  vault takes that this power-on does not hold is named, "Share D of
  name is not here", with **Type share D from its copy**: the codex32
  form of Add a key takes that string, checked against the share the
  plan keeps there, and the copy goes on (`docs/CODEX32.md` §9.5).
- Then the flow goes on to its lock and writes those vaults' sticks
  again. A lock during the round resumes on its Vaults card (§9).

A vault already on a stick and not set up before the keys is never
brought in here (§4.8, §17.21 item 5).

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


## 16. Stick pills on every device

(owner, 2026-10-15.) The numbered stick pills that Faraday Demo and
the website demo draw across the top (`stick/pills.rs`, commit bbcfd1e)
are drawn on the PC stick and the Pi too, so a person who brings a stick
back can see it is the one the flow wants.

- **When.** On Faraday Demo and Practice, always, as today. On a
  device (`shell_sticks` false), only while a backup flow on screen uses
  sticks (`sticks_wanted()` is some): the boot stick's pill, then a pill
  per stick of the flow in the flow's slot order.
- **Numbering** (owner, 2026-10-15): the boot stick is Stick 1 and is
  still called **Boot stick**; the backup's sticks are numbered from 2,
  "Stick 2", "Stick 3" … ("Card" on the Pi, by `app.medium`, the boot
  card being Card 1). Everywhere: Faraday Demo, Practice, the device,
  the flow's lines and its "Mark it as" line, and the backup board
  (§17.4, F146). Where only numbers fit,
  the boot stick's pill reads "1". Faraday Demo's backup stick folders follow
  the numbers (`STICK-2` for Stick 2); `BOOTSTICK` stays. A Faraday Demo
  install with a `STICK-1` folder from before renumbers its folders once
  at start, each `STICK-n` to `STICK-(n+1)`, highest first.
- **No press on a device.** A real stick is put in by hand: the pills
  carry no action there. On Faraday Demo and the website's demo a press still plugs or
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

**Tests** (`tests/all/stick_pills.rs`): on a device shell no pills without a
flow, the boot pill and a pill per slot with one; no action on a device;
a bound stick lights its own pill only; a stick matching no slot lights
none and reads its line; the mark line shows after binding and goes on
pull; at 480x640 a five-stick flow fits on one row in numbers alone;
the first backup stick is Stick 2 on every shell.
Render the flow's Bring in the sticks at 480x640 and 1280x800 with the
pills.


## 17. The backup board (F37)

(owner, 2026-10-15.) The plan's place cards and Where the description
goes (§15.1 items 2, 3 and 5) were a set of tick lists the owner found
confusing. They become one board: a card for each place, each card
divided into where a thing can be at that place (on a stick, inside a
vault on a stick, on paper), and beside the cards a list of everything
the backup can keep. The person drags a thing from the list into a
place, or from one place to another. The plan underneath is the map of
§15.2, unchanged in kind: each drop is a map edit (`plan::Edit`).

Scope: Faraday Demo and the PC stick, at any window width (§17.12
item 3 for a narrow window). The Pi is not considered here.

### 17.1 The board

The plan card's body, from Places to Where the description goes, is
replaced by the board, and so is The wallet description's Whole or In
shares (item 4): the shares are in the board's list. Start from (§15.1
item 1), Software's choice of software (item 6) and The plan with Make
the checklist (item 7) stay as cards.

**The places.** A row of place cards, wrapping to more rows when they
do not fit. Each card is a rounded rectangle (DESIGN §4.17) with the
place's name at the top (typed while a vault is open, as §15.7 says;
"Place n" otherwise), an × at its top right that removes it (with what
it holds, `Edit::RemovePlace` with no destination), and under the name
its zones, top to bottom:

- **Stick 2, Stick 3 …**: one zone per stick the place keeps, named by
  its pill's number (§17.4, F146; drawn as a
  box in the place box, with the vault box inside it: see §17.14). A stick
  zone has two parts: the vault on it, when one is ("Vault 1", and what
  the vault holds, §17.3), and its files. A stick with no vault is a
  stick of files. A place starts with no sticks; dropping a file or a
  vault on the card's **+ stick** row, or on the stick part of an
  empty card, makes one, and a press on **+ stick** adds an empty one
  (§17.16).
- **Paper**: what the place keeps on paper.
- Under the zones, the place's line of fact: what someone who finds this
  place alone gets (`plan::alone` for that place, as §15.1 words it).

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
Back up's Map was: see §17.14),
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
  as a stepper (0 to `min(m−1, n−1)`, the slider of §15.1 item 4), and
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
- Under Seeds, **a seed kept as codex32 strings** (a key whose secret is
  a codex32 set) is a group of its own, one chip per string by its share
  index (§17.23).

A chip in the list never runs out: dragging it places a copy. Each list
chip shows how many places keep it ("2 places"), and nothing when none
does.

### 17.2 Moving things

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
  that takes it is edged; a zone that does not take it (§17.3) is not.
  A press that moves past the slop (as scrolling's) is a drag; a press
  that does not is a tap.
- Each drop is one map edit, so the checks (§17.5) and every card's line
  of fact are current on the next frame.

### 17.3 What each zone keeps, and in what form

A thing placed in a zone is one chip there. A drop places the first form
of the zone's row below. While the board takes changes (Customize
backup, §17.6) a placed chip shows its forms as small toggles on the
chip itself ("◉ 9a6a2580 [✓ Words] [✓ SeedQR] ×"; the description's
Text, QR or File, whichever its zone allows): one tap turns a form on or
off, and the last form on stays on. Where the toggles do not fit beside
the name they wrap to a line under it. There is no forms popup (owner,
2026-10-19). On the read-only board the chip reads its forms as text
("9a6a2580 · Words · SeedQR"), with no toggles. Each form is its
own `What` on the map, so turning a form on or off is `Edit::Add` or
`Edit::Drop` of that `What`, and the plan's text, the checks and the
checklist read forms as they read everything else.

| Zone | A seed *i* | Its passphrase | The description, or share *j* |
|---|---|---|---|
| Paper | `Words(i)`, `SeedQr(i)` | `Passphrase(i)` | `Sheet`, or `Share(j)`, as today |
| Stick (files) | `SeedFile(i)` (words, `.txt`), `SeedQrFile(i)` (`.png`) | `PassphraseFile(i)` (`.txt`) | `WalletFile(form)`, `ShareFile(j, form)`: `form::TEXT` (`.txt`), `form::QR` (`.png`); and `SoftwareFile`, the chosen software's own file |
| Vault | `Seed(i)` | `SeedPassphrase(i)`, with its seed | `Wallet`, or `Share(j)` |
| Scanned in (software) | | | `Wallet` at `At::Software`, as today |

A codex32 string's forms per zone are §17.23's.

- New `What`s: `SeedQrFile(i)` and `PassphraseFile(i)` (tag Secret),
  `SoftwareFile` (Public). `WalletFile` and `ShareFile(j)` gain the form
  they are (today the plan's global `form` answers decide it); a plan
  saved before this change reads its `WalletFile` as every form its
  `form` answers had on.
- **A vault keeps records only**: the seed as its key record, its
  passphrase stored with it, the description as its wallet record, a
  share as its share record (§15.6). No file goes into a vault. A
  passphrase dropped into a vault becomes `SeedPassphrase(i)` there when
  the vault holds its seed; a vault that does not hold the seed does not
  take the passphrase (there is no record for a passphrase alone), and
  its zone is not edged for it.
- **A vault at several places** is one vault: what it holds is drawn in
  its zone at every place that keeps its stick, with a link mark (DESIGN
  §4.17), and a drop into it at one place is in it at all of them.

### 17.4 Sticks in the model

- New `At::Stick(p, s)`: stick *s* (from 0) at place *p*. It holds at
  most one `VaultStick(v)` and any of the stick forms of §17.3. Paper
  forms stay at `At::Place(p)`.
- A plan saved before this change reads as it did: at each place, each
  `VaultStick(v)` becomes a stick of its own, in vault order, and the
  place's files, when it has any, a stick of files after them (the "stick
  of files for {place}" of §15.2). The old `At::Files` stick stays
  read-only as §15.1 item 5 left it.
- **The checklist's write step** (§4.6) writes each stick in its place's
  turn: the vault on it, when one is, and its files, in one visit. The
  stick's binding (§4.6's label, serial, listing and the vault files'
  SHA-256) is taken after both are written, so the listing includes the
  files. Survey §4.6's binding before building: it must accept files
  beside a vault on one stick.
- The stick pills (§16) number every stick of every place, and the
  board, its ⋮ menu and its add sheets name each stick by its pill's
  number (F146): the number the backup flow gives its stick where the
  flow has one for every stick of the board, else from 2 in the order
  the flow asks for them (each place's vaults' sticks in vault order,
  then its sticks of files, place by place, then the software's stick
  of files), one numbering across every place. The board's empty sticks
  (§17.16) take the numbers after them. No stick on the board reads
  Stick 1, the boot stick's number.

### 17.5 Checks are warnings

Nothing is refused for being unwise. Every check of the plan (`check`,
`alone`, `all_at_one_place`, `lost_why`, `kept_nowhere`, and the rule
that a passphrase is not kept where its words are) is a line under the
board, current on every drop, and a placed chip that a check names
carries a warning mark (in `WARN`, as a held address is drawn). Examples:
"Seed B is kept nowhere", "Losing Home loses the wallet", "Home keeps
Seed A's words with its passphrase".

**A drop that lets a place see the balance** (owner, 2026-10-19). When a
drop, or a tap-place, makes a place able to see the balance that could
not before (the place header's tag turns from "Cannot see balance" to
"Sees the balance"), a sheet asks: its title "<Place> can see the
wallet's balance", one line naming what completes the description
there ("Share 1 and seed 9a6a2580 together make the whole description.",
"Shares 1 and 2 together make the whole description.", or "The whole
description is kept there."), and **Keep it** (the drop stands) and
**Take it off** (the drop undone). It is shown only on the drop that
changes the place from cannot-see to can-see, and only where the place
cannot spend (which has its own warning); never on a preset's own
placement. It is decided from the plan as it is after the drop; drawing
reads what the drop worked out. A seed kept beside a share gives its
own key, the one the share left off, when the plan works out whether a
place rebuilds the description.

A seed kept as codex32 strings adds its own checks (§17.23).

**Make the checklist** is offered with any warning standing, except
a seed kept nowhere, which holds The plan card as today (§15.1): with
it the checklist would release the hold on a key that has no backup
(F31).

### 17.6 Presets

*Narrowed (owner, 2026-10-20): The standard way is the only preset,
with no Start from card; §17.21. What follows on Customize backup
stands.*

The one way a backup starts (owner, 2026-10-19; narrowed 2026-10-20,
§17.21):

1. **The standard way**. A single key: one place, Paper
   holding the seed's words and the whole description. An m-of-n
   multisig: n places, each Paper holding one seed's words and the
   whole description. No SeedQR (§17.21 item 12). No vaults, no sticks. A seed with a
   passphrase has the passphrase at a place of its own, as every preset
   puts it (§10.3).
2. **Paper only**, 3. **Paper and vault**, 4. **Paper, vault and
   watch-only software**: offered nowhere since §17.21. Paper only is
   now the same plan as The standard way; vaults are added with
   **+ new vault** and the software with **Add watch-only software**. A
   plan saved from one of them reopens as it was, customisable.

**A new plan opens on the board with The standard way placed**, on Back
up and on Create's Back up alike: the person makes no selection. There
is no Start from card (§17.21). A plan already saved opens as it was.

**Customize backup.** Until it is pressed the board shows the placement
as it will be kept, read-only: no × on places, sticks or chips, no
**+ place** or **+ stick**, no drag, and form toggles read as text. The
right-hand column, where the list is, shows two secondary buttons,
**Add watch-only software** and **Customize backup**, instead of the
list; in the narrow column they stand where **Add** does (§17.21 item
3). Continue goes on to The plan as usual. Pressing Customize backup
shows the list (Seeds, Passphrases, Wallet description and shares,
Vaults with **+ new vault**, Watch-only software) and makes the board
editable as §17.1 to §17.5 say. It stays so for that plan: the plan
keeps it (a `custom 1` line in the plan's text; a `preset k` line, no
longer written, is still read, §17.21 item 7), so a plan saved after
customising reopens customisable and one never customised reopens
read-only with the buttons. The plan, the checklist and the
checks under the board are the same in both states. A wallet flow's own
plan (vaults set up before the keys) takes changes from the start: no
preset places those vaults.

### 17.7 A vault's key and wallet, shown and saved

The vault's contents screen (`vault/screen.rs` `detail`) gains, so the
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

### 17.8 Strings

New, in `faraday/faraday-strings/src/en.rs`: "+ stick", "Stick {}",
"Paper", "Scanned in", "Watch-only software", "Seeds", "Passphrases",
"Wallet description", "Whole", "Share {}", "Vaults", "+ vault", "Not
here", "{} places" and "1 place", the form names ("Words", "SeedQR",
"Words file", "SeedQR file", "Passphrase file", "Text file", "QR
picture", "{} file" for the software's), the check lines of §17.5,
"Show SeedQR", "Save to a stick", "Anyone who finds this stick has this
seed". §15's strings the board no longer draws are removed.

### 17.9 Tests

`faraday-core/tests/all/backup_board.rs`, through the app's events as a
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
- Seed B's words `.txt` dropped on Place 1's Stick 2 beside Vault 1:
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

The tests that pressed §15's place cards are changed to make the same
plans on the board, keeping what each checked; none is deleted.

### 17.10 Docs and design

- `docs/DESIGN.md` gains §4.17 **Board**: the place card, its zones,
  the chip (in the list, placed, selected, dragged, with a link mark,
  with a warning mark), the edged drop target. Sizes are tokens in
  `core/faraday-draw/src/tokens.rs` (CLAUDE.md).
- `docs/DECISIONS.md` F37. §15.1 items 2, 3 and 5 are marked "Replaced
  by §17 on `wide`".
- The board redraws on every pointer move while a chip is dragged: it
  draws through the drawing memo of `docs/SPEED.md` §3.3, and its batch
  is timed by SPEED §7. SPEED batch B goes first.

### 17.12 Further rules (owner, 2026-10-15)

1. **Both plan cards.** The board replaces the place cards in Back up
   and in the wallet's guided backup flow (§15.7), drawn by the same
   code (`plan/cards.rs`) on each one's own plan.
2. **Place names** are typed by a press on the card's name, while a
   vault is open, as §15.7 says; otherwise the name is not a field.
3. **A narrow window.** Where the cards and the list do not fit side by
   side (the `compact` layout of a narrow Faraday Demo window or a small
   screen), the place cards stack in one column that scrolls (the
   column's scroll and overlay scrollbar, as every page has), and the
   list is a sheet opened by **Add**, at the column's top. Placing is
   by tap only there: a press on a list chip in the sheet closes the
   sheet with the chip selected, and a press on a zone places it. A
   placed chip's forms open as on `wide`. §15's cards are not drawn on
   Faraday Demo or the PC stick any more.
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
   stick's files are one checklist item with the stick's write (§17.4);
   paper forms are the copy items they are today. A seed's file put on a
   stick asks **Save without encryption?**'s hold when it is placed,
   once for that file (§15.2, F129). A secret's file a plan keeps on a
   stick at a place and no drop held for (from a preset or a plan saved
   before: a seed's words, SeedQR or passphrase, a codex32 string) is
   asked at **Lock and write**, by the same sheet, one file at a time,
   before anything is made; its **Cancel** stops the lock and leaves the
   plan as it is.
7. **Vaults.** The same vault may have two sticks at one place: dropping
   it on a place that has it adds a stick. A vault leaves the plan by the
   × on its list chip, its sticks with it, with a warning line when it
   holds anything. A vault that already exists (made before this backup,
   with contents) shows what it held before in its zones, read-only and
   dimmed; §15.8 says what may be taken out of it.
8. **After the lock** (§15.8): Paper and every stick's file part take no
   drops and no form changes, and a press on them says §15.8's line;
   vault zones and vault sticks change as §15.8 allows. This is not one
   of §17.5's warnings: after the lock no key is loaded, so nothing can
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

### 17.11 Batches

Moved to `docs/history/BACKUP-FLOW-LOG.md`.

### 17.13 Decided while building F1 (2026-10-16)

1. **Old plans** (owner): an old plan whose places kept files gets the
   new checklist, its files written with each stick, one item per stick
   (item 6); an old plan with no files at a place makes exactly the
   checklist it made before. §17.9's old-plan test reads this way.
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
   asked. This supersedes §17.3's "there is no record for a passphrase
   alone".
7. **Read back** (item 10): the storage channel's write already reads a
   text file back; a picture is read back by the disk process's QR
   reading (`faraday-files`, faraday-codec's decoder), not `faraday-scanner`.
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

### 17.14 The board's layout and boxes (owner, 2026-10-16)

Decided after seeing F2a's board; built in F2b. Where this differs from
§17.1, this holds.

1. **Box in a box.** Three nested boxes, so what is on paper, what is on
   a stick and what is sealed in a vault are told apart at a glance:
   - the **place box**, the place card: its name, its stick boxes,
     **Paper** under them (outside every stick box), the line of fact;
   - a **stick box** per stick, inside the place box, headed by its
     pill's number, "Stick 2", "Stick 3" … (§17.4, F146), with its ×;
     everything on that stick is inside it. A
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
   are under the board, with the warning lines (§17.5). The top of the
   board is kept free.
5. Not changed: the checklist stage, after **Make the checklist**, keeps
   its side panel and Map (a choice: the owner has not ruled on it);
   a narrow window keeps the list as a sheet opened by **Add**.

### 17.15 The board in rows, and its colours (owner, 2026-10-16)

Decided from mockups after F2, replacing §17.1's rows of place cards
and §17.14 item 2's colours. Built in F2c.

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

### 17.16 + stick, and a vault kept at no place (owner, 2026-10-16)

Decided from the owner's test of F2c: **+ stick** did nothing when
pressed, which read as broken until a drop on it was tried; and a vault
whose sticks had all been taken off (its places removed) still stood in
The plan with what it held, until its chip was taken off the list.
F40. Built in F2d.

1. **A press on + stick adds a stick.** With nothing held, a press on a
   place's **+ stick** adds an empty stick box there, after its sticks:
   "Stick n", numbered after the plan's sticks (§17.4), with its ×, and
   **+ stick** after it again, so a place can
   be given several empty sticks in a row. A drop on **+ stick** is as
   before (a new stick for what is dropped).
   - An empty stick is the board's, not the plan's: it is kept in the
     board's state (`BoardState`), never on the map, so The plan, the
     checklist, the checks and the plan's text never see it (the map's
     rule that a stick holding nothing goes, §17.4, stands).
   - A drop on an empty stick (a file, or a vault's chip on its stick
     part) makes it the place's next stick of the plan, the same edit as
     a drop on **+ stick**; the empty sticks after it keep their order.
     Empty sticks are drawn after the place's sticks of the plan, so
     their numbers follow on.
   - Its × removes it. Removing its place removes it; the places after
     move down with theirs.
   - The board's empty sticks go when the board is left (its card closed
     or another screen opened): nothing is lost, since they hold nothing.
   - After the lock (§15.8) a press adds one as before; it takes what a
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
     "Seed X is kept nowhere", and holds Make the checklist as §17.5
     says.
   - "Vault n holds nothing" counts only a vault kept at a place, so a
     vault just added by **+ vault** holds nothing back.
   - The map as saved (`Answers::map`, the plan's text) keeps it, so the
     board reads back as it was left.
   - Model: `plan::map` is the plan, the stored map without a vault no
     place keeps a stick of; the board reads and edits the stored map in
     full (a function of its own), and a new vault's number and the
     list's vaults count every vault in it.
   - **A vault that exists already** (set up before the keys, §15.7)
     at no place is dim and out of the plan as any other, and "Vault n's
     stick needs a place" stands under the board, a warning only: it no
     longer holds Make the checklist (§17.5, warnings never refusals).
     Its × takes it out of this plan (asked first when something is
     planned into it, as today): the warning goes, the chip stays on the
     list dim, since the vault exists, and the vault's file is not
     touched; dragged onto a place it is back in the plan, holding
     nothing planned. A vault the board made (**+ vault**, a preset) has
     no warning at no place, and its × removes it from the list as today.
   - After the lock, a vault's stick and the vault change as §15.8
     already allows; nothing here widens it.
4. **Tests** (user-facing behaviour, `tests/all/backup_board.rs`): a press
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

### 17.17 The files box, and the stick and vault glyphs (owner, 2026-10-16)

Decided from the owner's look at F2c: a stick carrying a
vault was nearly all vault box, its own area a thin margin, so where to
drop a thing to keep it on the stick but not in the vault could not be
seen. Built in F2d with §17.16.

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
   target, not the whole stick box. After the lock (§15.8), the files
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
6. **Tests** (`tests/all/backup_board.rs`): a drop on the files box of a
   stick that carries a vault keeps the thing on the stick as a file,
   not in the vault; a stick with no vault takes a drop on its files box
   the same; the files box is there, empty, on a new stick. Snapshots of
   a stick with a vault, with and without files beside it, in the narrow
   column and the wide layout.

### 17.18 A place's name, pressed, is selected (owner, 2026-10-16)

A press on "Place 1" to rename it showed a blinking caret
after the name, which says typing adds to it, though typing replaces it.

1. **The press selects the whole name.** A press on a place's name (while
   a vault is open, §17.12 item 2) focuses the field with the whole name
   selected, drawn with the text field's selection highlight, and no
   caret.
2. **The first key replaces it**: a character replaces the name with
   itself, Backspace or Delete empties it; from then on the field is an
   ordinary text field, the caret at the end of what is typed. An arrow
   key or Home or End lets the selection go and puts the caret there,
   as any text field does.
3. **Tests** (`tests/all/backup_board.rs`): after the press, one key typed
   gives a name of that one character; a snapshot of the name just
   pressed, and after one key.


### 17.19 At most N places; partial descriptions by one button (owner, 2026-10-19)

**At most one place per key.** A plan has at most N places, N the
wallet's number of keys: + place is not offered once there are N, and a
preset never makes more. A single key has one place, except that a
single key whose seed has a BIP-39 passphrase may have a second place,
just for the passphrase (owner, 2026-10-19): its cap is 2, every preset
(the standard way too) makes Place 1 with the seed and Place 2 with the
passphrase alone, and + place is offered until 2. With only one place the
passphrase sits there (in its vault when the preset has one, else beside
the words) and the board warns as before. A multisig's cap stays N: no
extra place for passphrases. An extra backup
somewhere else is a copy made outside the plan (Faraday's copy between
sticks for files, by hand for paper), which the plan does not track.
Fewer places than N are allowed (a place may then hold more than one
seed). With one place, "Any one place lost" is No by the nature of a
single place, and reads so: "No: a single place" in the muted tone, not
as a warning.

The description's shares are not dragged one by one. In Customize
backup, one button adds them to the whole backup, and Faraday chooses
which share goes where.

1. **Add partial descriptions to this backup** (Customize backup, under
   the list's Wallet description): every place gets one share of the
   description; while they are on, a place added later gets its own, and
   the button reads **Remove partial descriptions**, which takes every
   share off. Shares are no longer offered one by one in the list, and
   the "Keys left off each share" stepper goes.
2. **How many keys each share leaves off:** as many as possible, and
   never so many that a group of M places (M the wallet's quorum) cannot
   rebuild the whole description and spend. With one seed per place the
   most is M − 1: 1 for a 2 of 3, 2 for a 3 of 5.
3. **Which share goes where:** Faraday tries every way of handing the
   shares out to the places (which keys each place's share leaves off),
   once, on the press, and keeps one where:
   - every group of places that holds enough seeds to spend can also
     rebuild the description from its shares and the keys of its seeds;
   - where possible, no single place can see the balance from its own
     share and seeds (a share at the same place as the seed of a key it
     leaves off would let that place fill the gap).
   It starts at M − 1 keys left off. With N places and one seed at
   each, M − 1 always works: one share per place, any M places rebuild
   the description, and no place sees the balance alone. With fewer
   places than N (a place holding two seeds), the search may need fewer
   keys left off, down to 1; if even 1 fails, the whole description is
   kept where it is needed and the warnings say so.
4. **The sheet after adding them:** "Each place now holds a part of the
   description. The whole description is still kept at <places>, so the
   balance can be seen there. Take it off any place where the balance
   should stay hidden." with **Take it off every place** and **Keep it**.
   Taking it off every place leaves any M places able to rebuild it.
5. **Warnings** (never refusals, §17.5), under the board, on every
   change:
   - the description cannot be put back together from everything kept
     (naming what is kept and what is missing: "the places hold Share 1
     twice and Share 2; add Share 3 or the whole description");
   - "Any one place lost" names the description when that is what fails;
   - a place that can see the balance from its share and seeds, when the
     search could not avoid it, naming the place and why;
   - a place that holds no seed: "Place 2 holds no seed".
   The sheet when a drop makes a place able to see the balance (Customize
   backup, owner 2026-10-19) stays, for seeds dragged by hand.
6. **Tests:** + place is not offered at N places, and no preset makes
   more; a single key's one place reads "Any one place lost: No: a
   single place" not as a warning; a 2 of 3 with three places: one share
   at each, each leaving off 1 key, no place can see the balance, any 2
   places rebuild the description; a 3 of 5 with five places: each
   share leaves off 2 keys, any 3 places rebuild it, no place sees the
   balance; a 2 of 3 with two places (one holding two seeds): the rules
   hold with fewer keys left off; Take it off every place leaves the
   description only in shares and any M places still rebuild it; Remove
   partial descriptions takes every share off; the result is the same
   on every run.
7. **Extra copies, after the wallet is made** (owner, 2026-10-19). The
   backup's last card (the wallet flow's Done, and Back up's checklist
   once done) carries, in its guided walk-through only (shown in Guided,
   hidden in Steps only), one line: "More copies can be made once the
   wallet is made: a vault copied to another stick, words copied by hand.
   They are not part of the plan." and a Learn link **Extra copies**: a
   new Faraday Learn page (`docs/learn/faraday/extra-copies.md`) saying
   how: a vault's file copied to another stick with Copy between sticks
   (the visit), the words written again by hand from the paper, the
   description saved again; that these copies are not checked by the
   plan; and that each is as sensitive as what it copies.

**As built** (2026-10-19):

- At most N places: the board's **+ place** stops at N (`plan::max_places`)
  and every preset makes one place for a single key and one per seed for a
  multisig. A plan saved with more places keeps them and is not cut down.
  A single key's passphrase, with no second place, goes into the vault
  where the preset has one, else beside its words, which the board warns
  of (§17.5).
- The search (`plan::hand_out`) tries, in a fixed order, each place's
  share from the place's own number on, from `M − 1` keys left off down
  to 1, under a budget of 20 000 hand-outs; it keeps the first where
  every group of places holding `M` seeds, and every group of `M`
  places, rebuilds the description, and no place that cannot spend sees
  the balance. A place that can spend sees the balance anyway and is not
  held to the second rule. Failing the second rule it keeps the first
  hand-out that rebuilds; failing both, one share per place in order
  with a key left off, and the warnings say what is missing. A seed in a
  vault whose stick the place keeps counts as there.
- Take it off every place takes the whole description off each place's
  paper and sticks; a vault and the software keep theirs. It is offered
  only where the hand-out rebuilds.
- The checklist's Shares card still sets the keys left off.

### 17.20 The ⋮ menu: placing without dragging (owner, 2026-10-19)

Dragging stays, and so does tap to select then tap a box; the ⋮ menu is
the visible way to place, move, copy and take off without dragging.

1. **The ⋮ on every chip** in Customize backup: the chip reads its name,
   a thin vertical rule, then ⋮ at its right end (`[ ◉ 9a6a2580 │ ⋮ ]`);
   a placed chip keeps its form toggles before the rule. The ⋮ is its
   own press target; the rest of the chip drags or selects as today.
   Not shown on the read-only board (before Customize backup).
2. **A list chip's ⋮: Add to…**, one menu listing every box that can
   take the item, grouped under each place's name (as the place is
   named): Paper, each stick's files ("Stick 2 · files"), each stick's
   vault ("Stick 2 · Vault 1"), and **New stick** where + stick would
   take it. Only boxes that take the item in some form are listed
   (§17.3). One press places it there with its usual forms, as a drop
   does, and closes the menu; everything a drop does follows (the
   checks, the can-see-the-balance sheet).
3. **A placed chip's ⋮:** **Move to…** (the same list, the item leaving
   its box), **Copy to…** (the same list, the item also kept there, where
   the item allows a copy), and **Take off** (as its ×).
4. **Everywhere.** The narrow column and the Pi's panel use the same ⋮:
   the Add sheet lists the items, each with its ⋮ and Add to…, in place
   of tap a chip then tap a box.
5. **The menu** is a popup under the ⋮ (a sheet on the small panel),
   keyboard reachable: the ⋮ is in the Tab order, Enter or Space opens
   it, arrows move, Enter chooses, Escape closes and returns to the ⋮.
   A press outside closes it.
6. **Tests:** Add to… on a seed lists only boxes that take it, grouped
   by place, and a press places it as a drop would; Move to… moves it;
   Copy to… keeps it in both; Take off takes it off; on the narrow
   column the Add sheet's ⋮ places an item; Escape closes the menu; the
   read-only board shows no ⋮.

**As built** (2026-10-19):

- The menu is a sheet of its own (`Sheet::BoardMenu`), drawn on `wide`
  as a popup under the ⋮ with no backdrop, and on the small panel as
  the full-width sheet with **Cancel** at its foot. What it lists is
  worked out on the press that opens it (`board_menu_boxes`), with the
  board's own `takes`; a box that keeps the item already is not listed
  (a drop there changes nothing), nor, for Move to… and Copy to…, the
  box it is in.
- A vault's chip lists, at each place, the place's first stick with no
  vault (where a drop on the place puts it), its empty sticks, and New
  stick; a place that keeps the vault's stick already offers only its
  empty sticks and New stick (a second stick of it there).
- A chosen box goes through `board_put`: Add to… and Copy to… as a drop
  from the list, Move to… as a drop of the placed chip. Copy to… is on
  every placed chip but a share (Faraday hands shares out, §17.19).
- The watch-only software's chip has a ⋮ too: Add to… lists Scanned in,
  under the software's name.
- In the Add sheet a press on a chip opens its menu, as its ⋮ does; on
  the board itself a tap on a chip still selects it for a tap on a box.
  Escape or Cancel from a menu opened in the Add sheet returns to the
  sheet; a box chosen closes both.
- The ⋮s on screen are the Tab order, in the order drawn (the places,
  then the list); Faraday has no Tab order of its own elsewhere. The
  keyboard's ⋮ stays edged until another is chosen or the board is
  left.
- On a placed chip the × stays, before the rule; the ⋮ is at the
  chip's right end.

### 17.21 One way to start; Customize for the rest (owner, 2026-10-20)

The standard way is the one preset, and it keeps each seed as its
words on paper, without a SeedQR (owner, 2026-10-20): it is now what
Paper only was. The vault presets chose for the person how many vaults
there are and where their sticks go; that, and a SeedQR, are done on
the board in Customize backup.

1. **No Start from.** Back up's plan has no Start from card, no preset
   list and no **Change**: a new plan opens on the board with The
   standard way placed (§17.6), read-only until **Customize backup**.
   Paper only, Paper and vault, and Paper, vault and watch-only software
   are not offered anywhere.
2. **Back up wallet**, at the foot of Make a new wallet's Check once
   the wallet is checked (Make a new wallet has no Back up card since
   the owner's call of 2026-10-26), and Restore's **Back up this
   wallet**, open Back up on the wallet just made, on the board as item
   1 says.
3. **The read-only board** shows two secondary buttons where Customize
   backup is today (the right-hand column; in the narrow column where
   **Add** stands), in this order, with no text above them:
   - **Add watch-only software**: adds the software place (§17.1) with
     the description Scanned in, as Paper, vault and watch-only
     software did. With the software place on the board the button
     reads **Take off watch-only software** and takes it off again. The
     board stays read-only either way. The software place holds no key
     and is not counted in §17.19's places.
   - **Customize backup**, as §17.6 says.
4. **+ vault reads + new vault** in Customize's list: a vault added on
   the board is always a new one, made in the checklist (Make a vault,
   on an empty stick, checked at the write step, §4.8). A vault already
   on a stick is brought in only before the keys (item 5).
5. **The vault question says so.** Under "Will you use vaults as part of
   this backup?" (Make a new wallet and Restore, §10.2), one line in the
   muted style: "Vaults already on sticks are brought in here, not
   later." Reason, not shown: a vault on a stick not read this power-on
   cannot be read after the keys without a lock, and the lock wipes the
   keys and Create's state (the extra round of §4.8 is not
   built).
6. **A planned vault not made yet says so.** A vault box's header band
   reads "Vault 2 · sealed · made in the checklist" while the plan's
   vault has no vault set up for it and none open to save into
   (`backup_vault_target(v)` is `None`); where that does not fit the
   band, "Vault 2 · new". A vault set up before the keys, or the open
   vault the plan saves into, reads "Vault 2 · sealed" as today. The
   header is worked out in the board's memo (`board_view`), which must
   be recomputed when a vault is opened, made or locked (`docs/SPEED.md`
   §3.3).
7. **Plans saved before.** A saved plan whose `preset` line names Paper
   only, Paper and vault, or Paper, vault and watch-only software reopens
   as it was saved and customisable (as if `custom 1`). A plan from The
   standard way keeps its `custom` state. The `preset` line is still
   read, never refused, and is no longer written.
8. **Strings.** Removed when nothing else uses them: the three preset
   names, Start from, Change on it, and Create's preset labels. Added:
   Back up this wallet, Add watch-only software, Take off watch-only
   software, + new vault (replacing + vault), the vault question's line,
   "{} · sealed · made in the checklist" and "{} · new".
9. **Tests** (user-facing behaviour): Create's Back up step offers one
   button and it opens the board read-only with The standard way
   placed; Add watch-only software adds the software place with the
   description Scanned in and the board stays read-only, and its Take
   off removes it; a vault added with + new vault reads "made in the
   checklist" and, once the checklist has made it, "sealed"; a vault set
   up before the keys reads "sealed"; the vault question shows its line
   in Make a new wallet and Restore; a plan text saved from Paper and
   vault reopens unchanged and customisable. Tests that chose a preset
   to reach a plan build the same plan as a person now would: Customize
   backup, + new vault, and the ⋮ menu's Add to… (§17.20), through a
   shared helper; none is deleted.
10. **Docs:** applied; the list moved to `docs/history/BACKUP-FLOW-LOG.md`.
11. **Change the plan opens the board** (owner, 2026-10-20). From the
    checklist, from its done state, and from a wallet flow's cards,
    **Change the plan** goes back to the plan with the board card open
    and scrolled to, as a new plan opens: in Back up (`Action::BPlan`,
    today every card closed) and in a wallet flow (`F::ChangePlan`,
    today The plan card). The board is as it was left: read-only until
    Customize backup, or customisable if it was customised; §15.8's
    limits after the lock stand. Test: from the checklist, Change the
    plan shows the board; Customize backup, then a seed's words toggled
    off and SeedQR on, then Continue and Make the checklist, gives a
    checklist with the SeedQR's item and no words; the same from a
    wallet flow.
12. **Words, not SeedQR, by default** (owner, 2026-10-20). The standard
    way puts each seed's words on paper and no SeedQR, single key and
    multisig alike (`Answers::preset`, today it ticks SeedQR for The
    standard way); everything else it places is unchanged. A SeedQR is
    added in Customize backup, by the seed chip's form toggle. A plan
    saved with a SeedQR keeps it. §17.6 item 1 reads accordingly. Test:
    a new plan's checklist has the words' item and no SeedQR, for a
    single key and a 2-of-3; Customize backup and the SeedQR toggled on
    adds it.
13. **A key with a codex32 set** opens on its own standard way:
    one string per place, on paper, with the description (§17.23).

**As built** (2026-10-20; the block as written is in `docs/history/BACKUP-FLOW-LOG.md`):

- `plan::Preset`'s four variants are plan builders only
  (`Answers::preset`): nothing in the UI offers them. Since item 12
  Paper only builds the same plan as The standard way. A saved plan's
  `preset k` line sets `custom` for any `k` but 0 and is not kept.
- The tests reach a preset's plan by presses through
  `testkit::backup_as(app, preset)`, which the snapshot tool's tours use
  too (`Tour::backup_as`); tests of a plan over two places for a single
  key set the answers directly, since no press reaches them.
- **Add watch-only software** is `BoardPress::Software`, taken while the
  board is read-only; it toggles the software place as the software
  chip does (`Answers::toggle_software`), refused after the lock as that
  chip is.
- Create's Back up card reads "Opens on the standard way" while closed,
  in place of the presets' list. Back up this wallet is the card's
  primary; **Move the money**, where a wallet is made to move money to,
  stays as it was.
- The vault box's band is chosen by the drawing: "{} · sealed · made in
  the checklist" where it fits the band's room, "{} · new" where not.
  Made or not is `backup_vault_target(v)` on Back up's board, and on a
  wallet flow's own board a vault set up before the keys
  (`v < shape.vaults`). `board_view` is worked out once a frame, so a
  vault opened, made or locked shows at the next frame.
- Change the plan sets Back up's open card to the board with the column
  at its top (the board is the first card); a wallet flow's opens its
  board card with the column following it.

### 17.22 How near a place brings a spend (owner, 2026-10-20)

**Why.** A place keeping two seeds of a 2-of-3 read "Cannot see
balance" and gave no warning, because a spend was counted only where
the place could also rebuild the description. The description is
public: every watch-only wallet and every share holds the xpub that is
missing. Whoever holds such a place is one public file away from
spending.

**Rules.**

1. **Seeds alone spend.** A place spends when it keeps the seeds the
   quorum needs, each with its passphrase where it has one; the
   description is not needed. This is the "One place found: can spend"
   line of the check, the "{place} alone can spend the wallet" warning
   and the chart's lines. Rebuilding the wallet when a place is lost
   still needs the description.
2. **How near.** For each place that keeps at least one seed, Faraday
   works out the fewest other places that, with it, keep enough seeds
   to sign. The other places count with their vaults read, so the
   number errs on the side of the warning. Whether this place needs its
   own vaults' passphrases to get there is worked out too, by counting
   it without them.
3. **The spend tag.** The place's header carries a second tag after
   the balance tag:

   | Reach | Tag | Tone |
   |---|---|---|
   | Spends alone | Can spend alone | `WARN` |
   | Spends alone with the vault | Can spend alone with the vault's passphrase | the vault's accent |
   | n more places | Spends with 1 more place / Spends with n more places | `MUTED` |
   | n more, with the vault | Spends with the vault's passphrase and 1 more place / … n more places | the vault's accent |

   A place keeping no seed has no spend tag. The balance tag says only
   the balance: "Cannot see balance", "Sees the balance", "Sees the
   balance with the vault's passphrase". Where the header has no room,
   a tag that cannot show its first letters is left off, with every tag
   after it.
4. **The sheet.** For a wallet of more than one seed, a drop (or a
   tap-place) that brings a place keeping two seeds or more nearer a
   spend opens a sheet with the warning mark. Nearer is fewer places, or
   as many places without the vault's passphrase where it was needed
   before. The title:

   - "Anyone with access to {place} can spend"
   - "Anyone with access to {place} and the vault's passphrase can spend"
   - "Anyone with access to {place} needs only 1 more place to spend"
     (n more places)
   - "Anyone with access to {place} and the vault's passphrase needs
     only 1 more place to spend" (n more places)

   Its line: "{place} keeps k of the m seeds a spend needs." (", counting
   its vaults." when the vault's passphrase is needed). **Keep it** and
   **Take it off** as the balance sheet's. It is asked before the
   balance sheet, which is not shown on the same drop. Never on a
   preset's own placement.
5. A single key's seed shows the spend tag and never the sheet.

**Where.** `plan::reach` and `plan::Reach` (the count, a search over
the places at most the quorum deep), `Alone::balance_tag`; `board.rs`
(`CardView::facts`, `board_sees_ask`, `Sees::spend`); `board/screen.rs`
(`place_head` takes a list of tags, `sees_sheet`'s warning mark).
Strings `board_spend_*` in `faraday-strings`. The tags are worked out
in `board_view`, once a frame through the drawing memo; the sheet's
decision works out each place's reach before and after the drop.

**Tests** (`tests/all/backup_spend_reach.rs`): a 2-of-3 with a second seed
dropped at a place reads "Can spend alone" without the description and
asks with the sheet, Take it off undoing the drop; a 3-of-5 with one
seed per place reads "Spends with 2 more places", and a second seed
dropped reads "Spends with 1 more place" and asks; a second seed put in
the place's vault reads "Spends with the vault's passphrase and 1 more
place"; a first seed at a place asks nothing; a single key asks
nothing.

### 17.23 Codex32 strings (`docs/CODEX32.md` §9; built 2026-10-23)

A seed is kept as codex32 strings (BIP 93) only when its key's secret is
a codex32 set already: the key was made by Make codex32 shares or loaded
from strings, its set's plain facts kept with the key and in its vault
record (`docs/VAULT.md` §7 type 2 field 5). The plan never makes a set
from a seed, and a key of words is never offered codex32 strings: they
would hold its 64-byte seed with the passphrase merged in and give back
neither the words nor the passphrase (owner, 2026-10-26). A plan saved
before, with a split the plan was to make, reads without that form and
its strings. The plan keeps each seed's form (`plan::codex32::C32`, a `codex32` line in the plan's
text: seed, k, length, indices, identifier) and each string is a thing
of the map, named by its share index.

**The list.** Under Seeds, each seed here is a group: its head
"9a6a2580 · codex32 w0za · 2 of 3", one chip per string ("A", "C", "D"),
each reading how many places keep it. A set is fixed: its k and
indices are the set's. A seed with no set has no group and no choice in
Customize backup. Not in a wallet flow's own plan, whose keys are made
after it.

**Each zone, each form.**

| Zone | Forms of string j of seed i | `What` |
|---|---|---|
| Paper | the string written · its Codex32QR drawn | `Codex32Share(i, j)` · `Codex32QrPaper(i, j)` |
| Stick (files) | `.txt` · Codex32QR `.png` | `Codex32ShareFile(i, j)` · `Codex32QrFile(i, j)` |
| Vault | a codex32 share record (`docs/VAULT.md` §7 type 15) | `Codex32Share(i, j)` |

Plan codes `x{i}.{j}`, `xq{i}.{j}`, `xt{i}.{j}`, `xp{i}.{j}`, the index as
its bech32 value. All are tagged Secret (Sealed in a vault).

**What the checks count.** A place, a vault read with its passphrase, or
everything left after a place is lost has a seed once it keeps k distinct
strings of it (or the secret): every check of §17.5 reads strings so. The
board's lines add "Share C of 9a6a2580 is kept nowhere", "Home keeps 2
shares of w0za: the seed", "Losing Home leaves fewer than 2 shares of
w0za", "Vault 1 holds 9a6a2580 and a share of it", "Vault 1 holds
9a6a2580 whole: one place with its passphrase is enough" (a set that
exists, its seed in a vault) and, for a string in a stick's files, "A
share of w0za is a file that is not encrypted; enough such files are the
seed". A place keeping only strings is a place with a seed.

**The standard way** for a key with a set: The standard way's placement
with the seed's words replaced by its strings, one per place, on paper,
the description at each place as the standard way puts it, so three
places for 2 of 3 (`plan::codex32::standard`, from Back up and Create's
Back up alike). A plan saved before the key had a set reads with it
(`plan::codex32::sync`), its strings not placed yet.

**The checklist's item, Codex32 strings of seed i** (after the seed's
copy item). Its body lists each string on paper and its places,
**Checked** or **Not checked**. Its foot is **Check my copy**, which
opens each string to type from its paper; it holds when its
checksum does and its header is the set's ("A · checksum holds, a string
of name", or "Box 4 is not this set's: not a string of name"), and once k
are typed, "These strings rebuild this key" or "These strings rebuild
another key: look at each again". Done when every string on paper is
checked or skipped. The strings go to no file and no stick from here.

**The vault's item** lists, after its seeds, a row for each set whose
strings the plan keeps in that vault, worded as the board's place lines
count them ("1 of 2 shares of 9a6a2580"), marked once the open vault holds
each of them (`board::codex32::vault_lines`, from the plan's kept map).
**A vault holds exactly the plan's strings of the plan's set**: a share
record counts only where its header's threshold and identifier are the
set's, so after **Rotate the set** a string of the old set in the vault
is not the plan's and the item is not done; the item is not done either
while the vault holds a string of the seed the plan does not keep there.
Its save, and the flow's round after the lock (§15.8), take such a
record out and put the plan's string in.

**The set's actions** (`docs/CODEX32.md` §9.7), under the item's list for
a split set that exists, and from the chart's sheet on a string (its rows
open the item): **Add a share** and **Replace C** (k strings of the set
typed in, the secret counting as one; the share at the next unused index,
or the lost one's own, opened to write and type back; the set's facts
gain it); **Rotate the set** (each string marked Held, Lost or Exposed;
with k or more out of reach "2 strings of name may be in other hands:
rotating is not enough; move the coins to a new key", else "Destroy A,
D. …", a press per held string destroyed, then the new set's threshold
and strings and a new identifier typed, the old one refused, made from
this device's randomness; the new strings take the old ones' places and
are to be checked again).

**Fewer presses** (owner, 2026-10-23):
an item that becomes done moves on to the next by itself, as its
**Continue** did: a copy matched, a passphrase compared, a vault saved
into, files or sheets made, the descriptor shown, codex32 strings checked
(after the last string's own Continue). The blank templates, the
envelopes and Write to sticks stay a deliberate press, and an item opened
when already done stays open. The narrow column's Add sheet closes with
**Close**.

**Kept beyond paper** (`docs/CODEX32.md` §2): a string on a stick is
written at **Lock and write** as `codex32-{fp}-{index}.txt` and, at 48
characters, `codex32qr-{fp}-{index}.png`, read back like a SeedQR; a
string in a vault is a share record written by the vault item's save and
the flow's round. Both need the string in this session (made here, or
typed back in the item); one not here is named and stops them. At
unlock the share records of the open vaults are gathered, and `k`
strings of one set load its key. A place's line counts its shares: "1
of 2 shares of 9a6a2580 · alone: nothing", "2 of 2 shares of 9a6a2580:
the seed".

### 17.24 A QR copied by hand: blocks or rows (owner, 2026-10-24; built 2026-10-24)

The SeedQR on a seed's copy item and a codex32 string's Codex32QR on its
item (§17.23) are copied square by square from one grid
(`screens/parts.rs`, `copy_grid`):

- **Blocks** (ticked) · **Rows**. Blocks are counted from the top left
  in fives, as the blank sheets' heavier lines are, so the last block
  of a row or column is narrower where the side is not a multiple of
  five: a 29 × 29 code has five blocks of five and one of four each
  way, a 25 × 25 code five of five, a 21 × 21 (Compact SeedQR) four and
  one of one. Each block is a press of its own, so the pointer over a
  square lights its block, not its row; in Rows each row is one, as
  before. The Codex32QR's blocks are named as its sheet names them,
  rows A to F and columns 1 to 6 ("Block C4"); the SeedQR's by the
  index of their first row and column, as its sheet numbers its lines
  ("Block 5 · 10").
- The block marked is tinted and edged in the accent, and drawn large
  beside the grid (under it on a narrow column) with one square of its
  neighbours faded round it. Its name and **Previous** · **Next** are
  under the key; the arrow keys move the mark a block at a time (Up and
  Down a row at a time in Rows), never past the code's last.
- Grey squares are the ones the blank sheet prints ("Grey · printed on
  the sheet"); black ones are drawn ("Black · draw"). For a SeedQR that
  is the finders, separators, timing and alignment the sheet prints
  (`structural_fixed`); the format areas, which the sheet leaves blank,
  are black. For a Codex32QR it is the finders, timing, alignment square
  and dark module the Codex32QR sheet prints black
  (`codex32_backup::printed`).

**A string's Codex32QR drawn by hand** (`Codex32QrPaper`, §17.23,
`docs/CODEX32.md` §9.5). Where the plan draws a string's Codex32QR at a
place, its item names the sheet ("Sheet · Faraday Starter, Blank
templates, codex32, 48 characters, Codex32QR") and opens on the grid
above; the Templates item counts that sheet ("1 from Blank templates,
codex32, 48 characters, Codex32QR", or "Both" where the string is also
written there). A string made here opens on its grid at once, and one
only drawn is not typed back. A string of a set that exists is typed
from its paper first; once it holds, its grid is shown (owner,
2026-10-24: no reveal of a kept string is added for it). **Scan my
copy** opens the camera on the drawn copy (`ScanPurpose::CheckCodex32`):
read as that string, a box that differs is named on the scanner ("Box 9
differs"), anything else is "Not a codex32 string", and the copy that
holds checks the string and closes the camera. Nothing scanned there is
loaded or filed. Every blank sheet says so in its line (owner,
2026-10-24): "Written by hand. Never photographed or scanned by any
other device once filled in."

Tests: `tests/all/codex32qr_copy.rs` (the sheet named, the grid's
blocks and rows, a copy drawn on the sheet's grid read back and
checked), `check_copy.rs` (the SeedQR's blocks, keys and rows).

## 18. Backups age (planned, not built; F87)

A backup is only as good as its last check: paper fades, a box is
moved, a stick stops reading, a share goes missing. And a wallet with a
timelocked recovery path has a second clock, one Faraday cannot see:
each coin's wait runs from the block it arrived in. This section adds
both clocks to what the plan already shows. It is planned, not
scheduled; `docs/FLOWS.md` (Wallets tab, the chart's sheets) points
here.

### 18.1 What is built today

Read from the tree on 2026-10-23, so that this section adds only what
is missing.

- **Dates already kept.** The plan's notes in the vault
  (`plan::Note`, `faraday/faraday-core/src/plan/mod.rs`) hold
  `checked <at> <what> <date>` per thing at a spot and
  `keychecked <fp> <date>` per key a holder keeps. Three presses write
  them (`faraday/faraday-core/src/chart_edit.rs`): a place's **Mark
  checked** (every thing in the node), a key's **Mark checked** (its
  holder confirmed it), and a passphrase's **Check it** that matched.
  The date is `Faraday::today`, the clock's local date when the clock is
  known, checked or not, and empty when it is not known.
- **Dates already shown.** The chart's sheets show "Last checked" and
  "Last checked here" with the date ("Checked 2026-10-10", or "Checked"
  for an empty date; `faraday/faraday-core/src/glance/sheet.rs`); a
  place's sheet shows the oldest of its things' dates. No age ("14
  months ago") is shown anywhere, and nothing is ever marked due.
- **Checks that keep no date.** The checklist's **Check my copy** (words
  typed back, a SeedQR scanned; and the chart's **Type my copy** and
  **Scan my copy**, which open it) counts a match in the power-on's
  backup record (`Record::checks`, `faraday/faraday-core/src/backups/mod.rs`),
  which Backups reads as **Checked** or **Not checked** per paper place
  and which power-off loses. It writes no vault note: the vault never
  learns that the copy matched, or when, and a check does not know
  which place's copy it was when a seed has copies at two places.
- **Spend paths.** The one place a wallet's waits are shown is the spend
  review's **Path** (`wallet::paths`, `faraday/faraday-core/src/wallet.rs`):
  "52560 blocks after the coins arrived (about 365 days)". The wallet
  card, the chart and the wallet sheet PDF (`wallet_sheet`,
  `faraday/faraday-core/src/pdf.rs`: name, shape, descriptor and its
  code, xpubs, first addresses, three lines to fill) say nothing about
  paths or waits.
- **Recovery wallets.** `core/faraday-bip/src/recovery.rs` builds a
  Liana-shaped wallet from keys and delays (`blocks_for_days`,
  `days_of_blocks`, 144 blocks a day, at most 455 days). Make a new
  wallet does not offer the kind yet (`docs/learn/17-wallet-kinds.md`);
  such a wallet arrives by its descriptor, from Liana or the Miniscript
  compiler. Its create step, **After how long?** (30, 90, 180 or 365
  days, or a number typed from 1 to 455), is `docs/WALLETS.md` §5's
  Recovery row and not in the tree; where it is built it shows each
  choice in days and blocks ("365 days · 52 560 blocks").

### 18.2 Words

- **A copy**: one thing at one place of the plan: a seed's words or
  SeedQR, a codex32 string, a share sheet, the wallet sheet, a vault's
  stick, a passphrase, a key a holder keeps.
- **Checked**: a copy matched here (typed back, scanned, read and
  unlocked) or marked checked by the person, on a date.
- **Age**: today's date less the copy's last checked date.
- **Due**: a copy whose age is the check interval or more, or that was
  never checked with a date.
- **Wait**: a spend path's relative timelock, `older(n)`, in blocks (or
  in 512-second units) from the block each coin arrived in.

### 18.3 Part 1: when each copy was last checked

**What records a check.** Every way a copy is checked writes the same
vault note, `checked <at> <what> <date>`, for that copy alone:

| Check | Copy dated |
|---|---|
| Check my copy, words typed back and matched (the checklist, or the chart's **Type my copy**) | that seed's words at the place checked |
| Check my copy, SeedQR scanned and matched (the checklist, or **Scan my copy**) | that seed's SeedQR at the place checked |
| A codex32 string's Check my copy (`docs/CODEX32.md` §9.4) | that string at its place |
| A passphrase's **Check it** (built) | that passphrase line |
| A place's **Mark checked** (built) | every thing at the place |
| A key's **Mark checked** for its holder (built, `keychecked`) | the key |
| A vault's stick read on a stick visit, and the vault unlocked from it, where the plan keeps one stick of that vault | that stick at its place |

**Which place.** A check opened from the chart knows its place: the line
pressed. A check opened from the checklist knows its seed and form, not
its place; where the plan keeps that form of the seed at more than one
place, the match result asks **Which copy was this?**, one option button
per place by its name (from the vault) or number, and dates the one
pressed. One place: no question. The power-on count Backups shows
(`Record::checks`) keeps working as it does.

**Where it is kept.** In the plan's notes in the vault, as today. With
no vault open, a check matched in the checklist is held in RAM only,
with the power-on's backup record (place by number, the thing by kind,
the date; no names), until the next unlock of the vault that keeps the
plan, which writes it into the plan's notes; power-off before then
loses it. It is never in the kept state and never on disk outside the
vault (Q3, §18.7; `docs/FLOWS.md`, the chart's rule). The chart's
**Mark checked** with no vault open reads "Open a vault to keep this",
as today.

**The age, shown.** A date becomes an age by one function in
`faraday-core`, `glance::age_text(date, today)`: "today", "yesterday",
"{n} days ago" under 60 days, "{n} months ago" under 24 months (whole
months, rounded down), "{n} years ago" after. Where it shows:

- **The chart's place nodes** (the wallet card's At a glance, an open
  vault's wallet item; DESIGN §4.16): a line under the place's name,
  "Last checked 14 months ago" from the oldest of its copies' dates,
  in `WARN` with **To check** at the node's right when any copy there
  is due; "Never checked" in `WARN` when a copy there has no date.
  Vault and files nodes the same.
- **The place's sheet**: per line, "Last checked 2025-08-14 · 14 months
  ago", the line's due mark, and **Mark checked** as today.
- **The wallet card's strip** (the plan's three checks above the
  chart): a fourth line, "Every copy checked in the last 12 months" in
  `OK`, or "To check: Bank box, Home" in `WARN`, its press opening the
  first due place's sheet. The small panel's At a glance page carries
  the same line, and each place row its age.
- **Backups**: a paper place reads "Checked 14 months ago" (and **To
  check** in `WARN` when due) where a date is kept, in place of
  **Checked** or **Not checked**; with no date kept, as today.
- **Home's lead tile** does not change (Q5, §18.7).

**The interval.** **12 months** for every copy, one constant in
`faraday-core` (Q1, Q2): a copy is due when its age is 12 months or
more. For a wallet with a relative wait, the interval for its copies is
the shorter of 12 months and its margin (§18.5; Q9), so each move of the
coins and each check of the copies fall due together.

**With no trusted clock.** A check's date is a kept time, so it follows
`docs/TOTP.md` §1.3: a date is written only from a checked clock
(`clock_checked`: Set the time or Keep this time pressed this
power-on, or the desktop app with its clock known).

1. A check that matches with the clock not checked shows its result as
   today ("Matches"), with **Set the time** beside it, which opens Set
   the time over the page and returns to it; once set, the check is
   dated with that day. Without it, the note is written with an empty
   date, shown "Checked · no date".
2. With the clock not checked, no age is shown and nothing is marked
   due: each place shows its date ("Last checked 2025-08-14"), and the
   strip's fourth line reads "Set the time to see what is due" with
   **Set the time**.
3. A date later than today (a clock set wrong before, or now) shows as
   the date, with no age, and is not due.
4. A copy whose last check has an empty date is due once the clock is
   checked: Faraday cannot say how old it is (Q4). With the clock not
   checked, rule 2 holds: nothing is marked due.

`Faraday::today` changes to read the checked clock only; the three
built presses then follow rule 1.

**Drawing reads.** The ages and due marks are drawing's, from the dates
kept on the chart's state and the clock: `glance::of` (through the
drawing memo, `docs/SPEED.md` §3.3) gains each node's oldest date and
due flag; the date strings are parsed once, where the notes are read.
Today's date is read once per frame. A decision (the strip's press)
computes from the state as it is (`*_now`).

### 18.4 Part 2: a recovery path's wait

**What is stated.** For a wallet with at least one spend path that
carries a relative timelock (`Lock::Blocks` or `Lock::Intervals`,
`core/faraday-bip/src/spend.rs`), whatever made it (Liana, the
Miniscript compiler, Make a new wallet's Recovery kind once built):

- **The wallet card**, under the shape, a **Spend paths** group (one
  fact row per path, DESIGN §4.11): the keys ("Key 1", "Keys 2 and 3, 2
  of 2") and when: "Any time"; "52 560 blocks after each coin arrives ·
  about 365 days"; "After block 900 000" or "After 2027-01-01" for an
  absolute lock. Under the group, for the shortest relative wait, one
  row: **Move each coin within** "about 328 days of arriving" (§18.5's
  margin). On the small panel, a **Spend paths** row opens the group as
  its own page, as At a glance does.
- **Coins last moved** (Q8). The card of a wallet with a relative wait
  offers **I moved every coin**, a dated vault note (`moved <date>`,
  kept with the wallet's plan notes, written from a checked clock as a
  check is, §18.3), and states "Coins last moved 300 days ago · the
  wait ends at about 365", in `WARN` once the age reaches the margin.
  With no vault open the button reads "Open a vault to keep this". The
  date is the person's word: Faraday does not see the chain.
- **The chart's wallet node sheet**: the same rows.
- **The wallet sheet** (PDF; and the blank template's wallet sheet, the
  Starter's, `docs/STARTER.md` §3.8): a **Spend paths** block after the
  xpubs: per path its keys and its wait in blocks and days; then, when
  any path has a relative wait, the line "Coins left unmoved for {days}
  days ({blocks} blocks) can be spent by {keys} alone. Move every coin
  to a new address of this wallet before then." and a table to fill by
  hand per deposit: **Received on** · **Amount** · **Move before**,
  eight empty rows, with "Move before = received on + {margin days}
  days" over it. The sheet file gains `path` and `wait` fields
  (`faraday-sheet 1` stays readable; a sheet without them prints as
  today). The Starter's blank wallet sheet carries the block empty (Q7):
  three path rows with their keys and wait to fill, the line with its
  numbers left blank, and the eight-row table.
- **The split sheets** (shares of the description): one line, "This
  wallet has a recovery path: see its wallet sheet", where it has one.
- **The spend review's Path step** keeps its text and adds the margin
  row nowhere: it states the transaction, not the wallet.

**What Faraday cannot do, and says nowhere on a working screen.**
Faraday is offline: it does not know when a coin arrived or whether it
moved. It shows the wait and the margin; the dates are the person's to
keep, on the sheet. Learn says why (below).

**Learn.** Edited as `docs/learn/`:

- `docs/learn/10-spend-paths.md` gains **Moving coins before the wait
  ends**: each coin's wait starts at its own block; a spend that moves
  every coin to the wallet starts them all again; blocks come about
  every ten minutes on average and faster while mining grows, so a
  wait in blocks can end days early; the wallet card's **Move each coin
  within** keeps a margin; the wallet sheet's table is where deposits
  and their dates go, since this device cannot see them.
- `docs/learn/12-inheritance.md` gains **A wallet that opens to an heir
  after a wait**: what the heir does (waits, then spends with the
  recovery keys alone), what the owner must do meanwhile (move coins
  before each wait ends), and that a wait which ends while the owner is
  alive lets the recovery keys spend.

### 18.5 Days, blocks and the margin

- Blocks to days: `recovery::days_of_blocks` (144 a day, rounded down),
  shown as "about {days} days".
- 512-second units to days: `n × 512 / 86 400`, rounded down, shown
  without "about": the network counts them by median time past.
- **The margin** (Q6): **Move each coin within** is 90 % of
  the shortest relative wait, rounded down to whole days ("about 328
  days" for 52 560 blocks). The wallet sheet's table uses the same
  number.
- An absolute lock (`after`) gets no margin row: moving coins does not
  change it.

### 18.6 Edit list, tests, batches

**Edit list.**

| File | Change |
|---|---|
| `faraday/faraday-core/src/chart_edit.rs` | `today` reads the checked clock; a dated note from the checklist's checks; the pending notes written at unlock |
| `faraday/faraday-core/src/backups/mod.rs` | `copy_matched` takes the place; the pending dated checks on `Record`, written at unlock |
| `faraday/faraday-core/src/screens/backup.rs` | **Which copy was this?**; **Set the time** beside a match with the clock not checked |
| `faraday/faraday-core/src/glance/mod.rs`, `faraday/faraday-core/src/glance/sheet.rs` | `age_text`, each node's oldest date and due flag, the strip's fourth line, the wallet node's spend paths |
| `faraday/faraday-core/src/screens/wallets.rs` | the card's **Spend paths** group, **I moved every coin** and its line; the small panel's row and page |
| `faraday/faraday-core/src/wallet.rs` | the wallet's paths kept on `Wallet` when it is loaded (the policy is read once), with each path's wait and the margin |
| `faraday/faraday-core/src/pdf.rs` | the wallet sheet's **Spend paths** block and table; the split sheets' line |
| `faraday/faraday-strings/src/en.rs` | the strings above |
| `docs/learn/10-spend-paths.md`, `docs/learn/12-inheritance.md` | the two Learn sections (and `faraday/faraday-learn/src/pages/en.rs` from them) |
| `docs/FLOWS.md` | the chart's sheets and Backups bullets: the planned lines become the rule as built, the sentence on a check held until the next unlock among them (its rule, "never in the kept state or on disk outside the vault", amended 2026-10-23) |
| `docs/DESIGN.md` §4.16 | the node's age line and **To check** mark |
| `docs/STARTER.md` §3.8 | the blank wallet sheet's empty **Spend paths** block (Q7) |

**Tests** (user-facing; `faraday-core` unless named):

1. A copy checked on 2025-08-14 reads "14 months ago" on 2026-10-23 and
   is due; one checked on 2026-01-10 is not. `age_text` at each of its
   boundaries (59 and 60 days, 23 and 24 months, a month-end day).
2. Check my copy matched in the checklist, clock checked: the vault's
   plan notes gain the place's dated note; locked and unlocked again,
   and in a new process from the vault file (power-off), the date is
   still there and the chart shows it.
3. A seed with words at two places: the match asks which copy, and only
   the one pressed is dated.
4. The chart's **Type my copy** on a place's line dates that place's
   copy without asking.
5. Clock not checked: a match writes an empty date; **Set the time** on
   its result dates it; no age and no due mark is shown; the strip reads
   "Set the time to see what is due".
6. A date later than today shows as a date and is not due.
7. No vault open: a matched check is held for the power-on, appears in
   no kept state and no file, and is written to the plan's notes when
   the vault opens; after a power-off before that, it is gone.
8. The invariant on the memo: changing a note (Mark checked) changes the
   node's age line on the next frame (`docs/SPEED.md` §3.3's test form).
9. A wallet `wsh(or_d(pk(A),and_v(v:pkh(B),older(52560))))`: the card's
   paths read "Any time" and "52 560 blocks after each coin arrives ·
   about 365 days", and **Move each coin within** "about 328 days"; a
   2-of-2 or single-key wallet has no group; `older` with the time flag
   reads days without "about"; `after` has no margin row.
10. The wallet sheet's text for that wallet carries the paths and the
    table's header; a sheet file without `path` fields prints as before.
11. Budget: a frame drawn again with a chart on screen does no costly
    work (`faraday/faraday-core/src/work/budget/`): the dates and paths are kept.
12. A recovery wallet with a 52 560-block wait: its copies are due at
    328 days, not 12 months; **I moved every coin** pressed on a date
    reads "Coins last moved … ago" and turns `WARN` 328 days later; with
    no vault open the button reads "Open a vault to keep this".

**Batches** (after the structure pass's phase 7 touches `backups` and
`glance`, so as not to fight its moves):

| Batch | What | Model | Estimate |
|---|---|---|---|
| A1 | The dated checks: `today` on the checked clock, the checklist's notes with **Which copy was this?**, the pending notes, tests 2 to 7 | Sonnet | 1 day |
| A2 | The ages: `age_text`, the nodes, sheets, strip, Backups, small panel; tests 1, 8, 11 | Sonnet | 1 day |
| A3 | The paths: kept on `Wallet`, the card's group, the chart sheet, the wallet sheet and split sheets, the Starter's blank sheet's empty block; **I moved every coin** and a recovery wallet's interval; tests 9, 10, 12 | Sonnet | 1.5 days |
| A4 | Learn's two sections, FLOWS and DESIGN as built, this section's plan to `docs/history/BACKUP-FLOW-LOG.md` | Sonnet | 0.5 day |

About 4 days. A1 before A2; A3 after A2 (the interval for a recovery wallet's copies); A4 last.

### 18.7 Questions for the owner

Decided (owner, 2026-10-23), each as recommended:

1. **The interval.** Decided: 12 months for every copy (§18.3).
2. **One interval or per plan.** Decided: one interval, a constant.
3. **Kept state.** Decided: a check matched with no vault open is held
   in RAM only until the next unlock of the vault that keeps the plan,
   and written there; it is never in the kept state or on disk outside
   the vault (§18.3). FLOWS' rule is worded so.
4. **Undated checks.** Decided: a copy whose only check has no date is
   due once the clock is checked (§18.3, rule 4).
5. **Home.** Decided: no Home tile; the card and Backups are enough.
6. **The margin.** Decided: 90 % of the shortest wait (§18.5).
7. **Blank templates.** Decided: the Starter's blank wallet sheet
   carries an empty Spend paths block (§18.4, batch A3).
8. **Coins last moved.** Decided: yes, **I moved every coin**, a dated
   vault note on a recovery wallet's card (§18.4, batch A3).
9. **The two clocks together.** Decided: a recovery wallet's check
   interval is the shorter of 12 months and its margin (§18.3).
