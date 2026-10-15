# The backup flow

**Status:** spec v0.3 · 2026-10-14 · decided by the owner
(`docs/DECISIONS.md` F31), reworked the same day to the order "decide
where the key will live, then make it", and §15's questions answered. Specified, not built: batch B1
(§14) comes after APK batch 1 and the GPG fix batch are committed.

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
- **Why this order.** A stick is read only with nothing secret open. A
  key made first would have to be locked into some vault before a stick
  holding the vault it belongs in could be read; a new Bitcoin seed held
  only in memory would be wiped by that lock. So every stick the key
  will be kept on is read before the key exists. Restore, which makes
  nothing new, takes the same steps with its seed typed or scanned at
  step 4.
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
wallet, New key makes each seed straight into the vaults its `vault v`
lines name, with its passphrase where `pass i` puts it in the vault; a
key kept only on paper is made and shown for copying, as today. Restore
takes the seed typed or scanned, into the same vaults. A Nostr key needs
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
Place 2, the descriptor on the stick of files. Create: kind → Where
(Vault 1 new, holding seed 1, at Places 1 and 2) → bring in the two
sticks as in A → Create Vault 1 → New key, made into Vault 1 → Check →
Back up's plan (the rest) → checklist: Vault 1 (done), Public files
(**Make them**), Envelopes → **Write to sticks**: Lock and write → Vault
1 (Place 1) → Vault 1 (Place 2), the second-copy rules → the stick of
files: checked, tap, `savings-descriptor.txt` written → Done → Unlock →
the done card.

**C. A 2-of-3, each key in its own vault.** Vault 1 an existing vault
on a stick at Place 1; Vault 2 and Vault 3 new, on empty sticks at Place
2 and Place 3. Insert the stick for Vault 1 (Place 1): it holds
`main.ofv` and `main-2.ofv`; "Which vault is Vault 1?" → `main.ofv`,
copied in; tap → Vault 2 (Place 2): empty, checked, bound to Vault 2;
tap → Vault 3 (Place 3): empty, reads as Place 2's, so the hold → Unlock
Vault 1; Create Vault 2 and Vault 3, each its own passphrase → New key
three times: Key 1 into Vault 1, Key 2 into Vault 2, Key 3 into Vault 3
→ the rest of the plan and the checklist → Lock and write → the stick
for Vault 1 (Place 1): `main.ofv` replaced, `main-2.ofv` untouched →
Vault 2 (Place 2) → Vault 3 (Place 3) → Done.

**D. A 2-of-3 mix.** Vault 1 on a stick at Place 1, Vault 2 on a stick
at Place 2, Key 3 on paper at Place 3. Where names the two vaults and
their places; Place 3 keeps no stick, so step 2 asks for two sticks
only → Unlock or create Vault 1 and Vault 2 → New key: Key 1 into Vault
1, Key 2 into Vault 2, Key 3 made and shown for copying → the plan's
paper part (Place 3: Key 3's words) → checklist: Templates (printed
from Faraday Starter), Copy of Key
3 (F29's typed check), Vault 1 and Vault 2 (done), Sheets, Envelopes →
Write to sticks: Vault 1 (Place 1), Vault 2 (Place 2), the stick of
public files (the sheets as PDFs) → Done. Key 3 is held
until its copy is checked; Keys 1 and 2 until their sticks are written.

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

- **Make a new wallet** gains, after its kind and shape and before
  Keys, the steps 1 to 3 for the plan's vault part: **Where the keys will
  be kept** (Into vaults or not; the vaults, `vault v`; their stick
  places, `sticks v`; the places, `places`), **Bring in the sticks**,
  **Vaults**. A plan with no vault has none of them. Back up's plan then
  opens with that part closed on its value with **Change** (DESIGN
  §4.14), the paper, SeedQR, sheet, share, descriptor and software
  questions where they are.
- **New key** makes each seed into its vaults (step 4), with F6's flag
  set; **Restore** puts its seed there. `Item::Vault(v)` is then done,
  or needs **Save into {vault}** only for what it still lacks (the
  wallet description where the plan puts it, a plan changed part way);
  `BNewVault` stays for a plan changed part way.
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
  backup is done, and the hold moves to the flow's state until the last
  stick is written.
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
lines do: the plan's map panel, Create's **Where** card for a wallet,
the done card's strip, the wallet card's and the open vault's strips
(NEW-WALLET §14.6), stacked as a fourth row on `small`.

**The passphrase line.** For a single-key wallet whose key has no
passphrase, a fifth line, in the body tone, under the warning, with its
action:

- `plan_passphrase_apart`: "A passphrase kept at another place than the
  seed is recommended."
- `plan_add_passphrase`: **Add a passphrase**.

In Create's **Where** card, before the key is made, **Add a passphrase**
marks the slot so that the Key card asks for one (NEW-WALLET §3.3), and
gives the passphrase its own row in the plan, at a place other than the
seed's. Once the wallet exists, it opens Create on the Keys card with
the key in its slot and §3.6's fields open; the key with a passphrase is
another key and the wallet another wallet, which the Keys card shows by
its fingerprint. The line never blocks Continue, the checklist or a
spend.

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

Restore's seeds route, and Add a key when the plan keeps the seed in
vaults, take steps 1 to 3, then the seed typed or scanned into its
vaults at step 4, then 5 to 7: no extra round. A restored key signs at
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
  passphrase** in Create's Where card makes the Key card ask for one;
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
