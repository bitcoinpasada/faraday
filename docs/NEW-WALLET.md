# Making a wallet, second pass

**Status:** spec v0.1 · 2026-10-10 · decided by the owner from a walk
through Create a wallet, New key and Back up a wallet on the built app
(single key, then a 2-of-3). Where the build and this file differ once
built, `docs/FLOWS.md` and the code are the record.

The problem it solves: a new single-key wallet is made in three flows
that each repeat part of the others. New key asks for the words to be
written down and quizzed before the wallet exists, while Back up asks
for the same again an hour later; no step offers a BIP-39 passphrase;
nothing shows the person that the words they rolled are the words the
key was made from. The multisig Keys card offers a key already used in
another slot. The backup plan puts every seed into one vault, and leads
with shares a person has not heard of.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every build and test
  through `local/remote-run.sh`; every dimension a token; labels, not
  prose, on working screens; tests for user-facing behaviour only;
  `just` once at the end of a batch; no `rm`; no commits; no agents.
- Work the batches in order. Each batch is one review and one
  completion report. Do not start a batch before the previous one is
  reviewed.
- Tests go in the named files under `faraday/faraday-core/tests/`,
  driving the app through `Action`s as the neighbouring tests do. A test
  states what a person sees or can do. Tests that encoded a rule this
  file changes are changed to the new rule, never deleted.
- Docs in the same batch: `docs/FLOWS.md` a dated decision line per
  changed flow; `docs/WALLETS.md` §5 for Create and Back up;
  `docs/DESIGN.md` §4.14 for the opening rule; `docs/VAULT.md` for §5's
  vaults.
- The snapshot tour (`faraday/shells/snapshot/src/main.rs`) drives these
  flows; where a step this file removes, adds or reorders is named
  there, change the tour too. To look: `local/remote-run.sh --fetch
  out/snapshots/faraday/1280x800 just faraday-snap 1280x800` (and
  `480x640`).
- Completion report: the items done, each with its tests and whether
  they pass; what was left out and why; docs changed; render names to
  look at. No restating the spec.

## Decisions recorded (owner, 2026-10-10)

1. The light theme is the default on a first start.
2. A test network is marked by the sidebar pill alone; the coloured bar
   along the top goes.
3. Create a wallet and New key open on their first step. (Reverses
   SIMPLIFY §2.1 for these two flows only; Sign, Back up and Create a
   vault keep it.)
4. The passphrase belongs to making a wallet, not to making a key. New
   key opened from Create offers it (optional); New key on its own
   (Tools, Seeds) makes the key alone and asks for no passphrase. A key
   made or loaded without one can take a passphrase later, when it is
   chosen for a slot in Create.
5. A key's words, and its passphrase when it has one, are locked in
   right after the entry, with the key's fingerprint and what it makes
   shown at once, so the person sees the words they rolled are the key
   that was added. The randomness check is shown on the entry card as
   soon as the last entry is in, before anything is locked in; Back up
   comes last.
6. Inside Create, New key neither asks for the words to be written down
   nor quizzes them: Back up shows them and checks the copy.
7. A key chosen for one slot is offered in no other slot.
8. The backup plan leads with the whole wallet sheet; shares are second,
   with a Learn page and a slider for the keys left off each.
9. The backup plan puts one seed in each vault by default, each vault
   on its own stick; a person may tick more seeds into a vault.
10. A loaded wallet's card shows the wallet **at a glance**: one chart of
    the wallet, its keys and its backup, where each thing is and what
    each place gives whoever finds it.
11. The wallet's map travels in the vault (the type 11 Backup plan
    record) and its chart loads with the wallet on unlock. Seen from an
    open vault, the chart runs the other way: the places and vaults on
    top, flowing down through the keys into the wallet at the bottom,
    the way a recovery goes.
12. Every node and line of the chart can be pressed to look after the
    backup: rename, check, print again, copy, move, mark lost or
    exposed (§9). An edit there makes the map the plan.

## 1. Appearance

### 1.1 Light by default

**What.** A device with no settings saved starts in `Theme::Light`.
**Where.** `faraday/faraday-core/src/ui.rs` `enum Theme` (the
`#[default]` and the doc comments that name the first-start theme).
**Tests.** `tests/appearance.rs`: a first start is in the light theme
(change the test that says otherwise).

### 1.2 No bar for a test network

**What.** The 4-point `WARN` fill along the top on a test network goes,
in both places it is drawn (`screens.rs`, the main area after
`notices(...)` and the sidebar above the pill), and on the small panel
if `compact_screens.rs` draws one. The pill stays as it is.
**Tests.** None (a drawing detail). **Docs.** `docs/DESIGN.md`, wherever
it describes the bar.

## 2. Create a wallet: first step, slots, labels

### 2.1 Create and New key open on step 1

**What.** Create a wallet opens with **Kind** open, whatever opened it
(Wallets, Home, a Tools tile with its kind already ticked). Kind's
Continue closes it as done and opens the next card. New key opens with
**Length** open; Length gains a **Continue** that takes the count shown
and opens Randomness (pressing a count does so already); Randomness's
Continue opens the entry card as now. Cards with a default and not open
still show their value and **Change** (§4.14).
**Where.** `create.rs`/`lib.rs` where Create's `open` is set at entry;
`KeyGen::new` (`keygen.rs`), whose comment cites the rule being
reversed; the Length body in `keygen_screen.rs`.
**Tests.** `tests/create_wallet.rs`, `tests/keygen.rs`: a fresh Create
opens on Kind; a Tools tile for MuSig2 opens on Kind with MuSig2 ticked;
a fresh New key opens on Length, and two Continues reach the entry card.

### 2.2 A key in one slot is offered in no other

**What.** On the Keys card, a key loaded here that fills one slot is
not offered in any other slot; a cosigner's file whose key fills one
slot is not offered in another. Clearing the slot offers it again.
**Where.** `screens.rs` `create_body`, `cstep::KEYS` (the `row` built
per slot).
**Tests.** `tests/create_wallet.rs`: on a 2-of-3, after a key fills Key
1, Key 2 does not offer it; after Clear on Key 1, it does.

### 2.3 The Keys card's foot button

**What.** While a slot is empty, the button at the foot of Keys reads
**New key for Key N** (Share N on a threshold wallet), N the first empty
slot, and opens New key for that slot. Once no slot is empty it reads
**Continue**, as now.
**Where.** `create_body`, the Continue at the end of `cstep::KEYS`.
**Tests.** `tests/create_wallet.rs`: on a fresh 2-of-3 the foot button
opens New key for Key 1.

### 2.4 Check names whose addresses they are

**What.** On Create's Check, above the address rows, a section label
**The wallet's first addresses**, so that three rows on a 2-of-3 do not
read as one per key.
**Where.** `create_body`, `cstep::CHECK`.

## 3. New key: the check on the entry card, lock in, then back up

### 3.1 The steps

`kstep` gains **KEY** after ENTER. Which cards a flow shows:

| New key | Cards |
|---|---|
| BIP-39, for a Create slot | Length · Randomness · *entry* · Key |
| BIP-39, on its own (Tools, Seeds) | Length · Randomness · *entry* · Key · Words · Quiz |
| SLIP-39 shares | as today: Length · Randomness · *entry* · Check · Words · Quiz |

*Entry* is the card named for the source today (Rolls, Flips, …). For
BIP-39 the CHECK card goes: its content moves onto the entry card
(§3.2), and no separate step is shown. SLIP-39 keeps its Check card.
The card list is computed per flow as `crate::create_steps` does for
Create, and `flow::paged` (small panel) uses the same list.

### 3.2 The entry card

- When the last entry is in, the card shows the randomness check there,
  under the entries: the counts the Check card shows today (rolls,
  faces, chi-square, longest run, or the source's own) and its verdict.
  No caution: "Fair: no cautions" in `OK`. A caution: each caution line
  in `WARN`, and **Roll again** (or the source's word for it) beside the
  foot button, which clears the entries. A caution is not a refusal; the
  foot button still works. A source with no counts (this device's
  generator, the camera) shows no check.
- Its foot button reads **Continue** when the words are on screen as
  they come in (`KeyGen::reveals()`), else **Make the words**. Either
  way it opens Key; it is not shown until the last entry is in.
- When the words are on screen and the flow is for a Create slot, one
  line under the words so far: "No need to write the words down yet:
  Back up shows them again."

### 3.3 The Key card

Before lock-in:

- The words, in the grid the Words card draws today; shown at once when
  `reveals()`, else hidden behind **Show words** as now.
- For a Create slot only: **Passphrase** and **Passphrase again**,
  masked text fields, the same kind Add a key uses for its passphrase;
  empty means none, the placeholder reads "Not set". Unequal fields: "The
  two passphrases differ", and no lock-in. New key on its own has no
  passphrase fields: it makes the key alone (decision 4).
- **Lock in** (primary; Enter presses it). It adds the key: through
  `Session::add_words_with` with the passphrase (the handout path in
  `keygen_add` stays), under the self-test gate `keygen_add` has. For a
  Create slot it fills the slot. For a single-key Create it also makes
  the wallet, as Keys' Continue does today (`create_make`), and marks
  Create's Keys and Check done.

After lock-in, the same card shows:

- The words and, for a Create slot, the passphrase line: **Passphrase**
  with a pill (`Ui::pill`) reading **Set** or **Not set**, never a bare
  "none", which reads as a passphrase (owner, 2026-10-10). Read-only,
  with a **Locked in** tag (lock icon, `OK`).
- **Fingerprint**, then by context:
  - single-key Create: the wallet's descriptor row (`descriptor_row`)
    and **First address** with the wallet's receive 0/0;
  - a multisig, MuSig2 or threshold slot: **Key**, the kind's key
    text (`NewKind::key_text`, `[fp/path]xpub`), and no address;
  - on its own: **Key**, the native SegWit key text, no address.
- **Continue**, which ends the flow (§3.4).

Summary when closed: "cf0e9805 · passphrase not set · locked in" (or
"· passphrase set ·"; on its own "cf0e9805 · locked in"); "Not locked in"
before. After lock-in Length,
Randomness and the entry card open to be read but change nothing: no
Change, no inputs, as Create's cards are once `built`.

### 3.4 Leaving New key

After lock-in nothing about the key changes; to make a different key,
leave the flow and make another. Key's **Continue**:

- for a Create slot: leaves New key for Create. A single-key Create
  opens on **Back up** (Keys and Check done); a multisig opens on Keys.
- on its own: opens Words, then Quiz, as today, the key already added;
  the Quiz's last button reads **Done** and leaves the flow.

The toast on leaving stays: "Key … added; back up its words before you
rely on it".

### 3.5 Back up gets the passphrase

Nothing new to build: a key added with a passphrase is a seed with
`passphrase` in the plan's `Shape`, so the plan asks where the
passphrase goes and the checklist has it written. Verify by test.

### 3.6 A passphrase for a key chosen in Create

A key made on its own, or loaded from words without a passphrase, can
take one when it is chosen for a slot. On the Keys card, a slot filled
by a key loaded here from BIP-39 words with no passphrase shows an
**Add a passphrase** button beside Show xpub QR (on a single-key wallet
too, where there is no xpub row: it goes on the slot's own row). It
opens, in the card under that slot, **Passphrase** and **Passphrase
again** and **Lock in**: the same fields and rules as §3.3. Lock in adds
the key the words and that passphrase make (a new fingerprint, through
`add_words_with`, labelled as the first key with " · passphrase"),
puts it in the slot in place of the first, and shows the new
fingerprint with a **Locked in** tag. The key without the passphrase
stays loaded and is offered again elsewhere. A slot whose key already
has a passphrase, a cosigner's xpub, or a share has no such button. A
key loaded from SLIP-39, codex32 or a bare seed has none either: its
passphrase, if any, was given when it was loaded.

**Where (all of §3).** `keygen.rs` (`kstep`, `KeyGen`, `keygen_act`,
`keygen_add`, `keygen_key`), `keygen_screen.rs`, `lib.rs` (`create_make`
and Create's re-entry), the session's way of removing a key (find the
one Seeds' Forget uses).

**Tests.** `tests/keygen.rs`:
- a Create-slot New key shows no Words or Quiz card;
- Lock in with a passphrase adds the key whose fingerprint is that of
  the words with that passphrase (compare with `add_words_with`);
- unequal passphrases lock nothing in;
- after Lock in, pressing a count on Length changes nothing;
- with the last roll in, the entry card shows the check's verdict, and
  a Create-slot flow has no separate Check card;
- Roll again on a caution clears the entries and adds no key;
- a single-key Create: after Key's Continue, Create is open on Back up, and
  the wallet's first address is the one Key showed;
- a multisig slot's Key card shows no address;
- New key on its own shows no passphrase fields and adds the key
  without one.

`tests/create_wallet.rs`: a key made on its own, chosen for a
single-key slot, takes a passphrase there; the slot then holds the
fingerprint of the words with that passphrase, and the wallet made is
that key's; a slot whose key has a passphrase offers no Add a
passphrase.

`tests/backup_plan.rs`: a single-key Create with a passphrase gives a
plan that asks where the passphrase goes.

**Docs.** `docs/FLOWS.md` (New key and Create), `docs/WALLETS.md` §5.

## 4. Back up: the whole sheet first; shares with Learn and a slider

### 4.1 Places

- "Each place keeps" lists **The whole wallet sheet** first, ticked by
  default (`Answers::defaults` sets `split: false`), then **Its own
  share**. Keep `Question::Split`'s row numbers matching the rows drawn.
- Beside **Its own share**, a text button **What is a share?** opens
  Learn on §4.2's page; Back from Learn returns to the plan on Places.
- With Its own share ticked, under the list: **Keys left off each
  share**, a slider (`Ui::slider`, as `seeds_screen.rs` uses it) from 0
  to the most `split_plan` allows (`min(m−1, n−1)`), the value shown;
  then the table of shares and their keys' fingerprints and the audit's
  lines, as the checklist's Shares card draws them today. A 3-of-5 can
  leave 0, 1 or 2 keys off each share.
- The checklist's Shares card uses the same slider in place of its
  0/1/… buttons. One function draws slider, table and lines for both.

### 4.2 The Learn page

`docs/learn/faraday/shares.md`, wired as `upgrade-a-stick.md` is in
`learn.rs`, and listed among the backup's Learn pages. Plain statement,
no mannered prose, Learn's usual length. It covers: what a wallet
description is and that working out any address needs every key in
it; a share is the description with some keys left off; what leaving
k keys off each of n shares gives (any m shares hold every key; one
share alone cannot see the balance), with the 2-of-3 and 3-of-5 cases;
that shares are not secret sharing (the keys are public; shares decide
who can see the balance, not who can spend); how Faraday's Restore
rebuilds the whole description from shares; that other software
(Sparrow, a coordinator) needs the whole description, so the person
rebuilding uses Faraday or rebuilds it first.

**Tests.** `tests/backup_plan.rs`: a fresh multisig plan keeps the whole
sheet in each place; ticking Its own share on a 3-of-5 offers 0 to 2
keys left off; What is a share? opens the page. `tests/learn.rs` if it
lists every page.

## 5. Back up: one seed per vault

### 5.1 The model

`Answers` gains the vaults: for vault v, which seeds it holds
(`vaults: Vec<Vec<bool>>`), and where its stick is kept (`sticks`
becomes per vault, a flag per place). Defaults when seeds go into a
vault: one vault per seed loaded here, vault v holding seed v only. Vault
v's stick goes to the first place that keeps no other seed's words and
no other vault's stick of this wallet; failing that, to the place that
keeps seed v's own words (a copy of a key already there, never a second
key). So no place holds two different keys, even behind vault
passphrases, where the places allow it. On a single key with two
places: words at Place 1, the vault's stick at Place 2. On a 2-of-3
with two seeds here: Vault 1's stick at Place 3, Vault 2's at Place 2.
A vault with no seed ticked is not made,
except that when the wallet description goes into a vault and no seed
does, one vault holds the description. The wallet description, when it
goes into the vault, goes into every vault made. A passphrase "in the
vault with its seed" goes into whichever vault holds that seed.

`At::Vault` and `What::VaultStick` carry the vault's number; `map` gives
one spot per vault made; `check` treats each vault as readable from a
place keeping its stick, with that vault's passphrase. The check lines
keep their wording.

`to_text`/`from_text` write a `vault v bits` line per vault and a
`sticks v bits` line per vault. An old plan, with one `sticks` line and
no `vault` lines, reads as one vault holding every seed here with its
stick at the places ticked, so plans saved before this pass still load.

### 5.2 The questions

- **The seeds go**, with **Into vaults** ticked: under the list, a
  section per vault, **Vault 1**, **Vault 2** …, each a multi-choice
  list of the seeds loaded here; a seed may be ticked in more than one.
- **Places**: the section "Stick with the vault" becomes one per vault
  made, "Vault 1's stick", a multi-choice of the places.
- The map panel names each vault ("Vault 1 · 9a6a2580").
- The preset rows' names stay; the row "Into the vault" reads "Into
  vaults".

### 5.3 The checklist

- The vault item becomes one item per vault made: **Vault 1:
  9a6a2580**. Its body: make a new vault (`vault_way`, back to this
  item), then save that vault's seeds into it (with the passphrase where
  planned) and the wallet description where planned. An open vault that
  already holds a seed another vault item has is not offered for this
  item ("Lock it and make a new vault"). Done when the open vault holds
  every seed of its item. The separate "wallet into the vault" item
  folds into these.
- Envelopes name which vault's stick goes in each place.
- A stick visit with more than one vault file waiting For the stick
  ticks one vault file by default, the first not yet written; the rest
  stay for the next stick. The receipt says which vault went where.

**Where (all of §5).** `plan.rs`, `screens.rs` (`plan_body`,
`backup_body`, `seed_copies`, the map panel), `lib.rs` (`bstep`,
`plan::Item`), the For the stick defaults (SIMPLIFY §4.2's code).

**Tests.** `tests/backup_plan.rs`: a 2-of-3 with two seeds here and
Paper and vault makes two vaults, one seed each, each stick at a place
without that seed's words; ticking seed 2 into vault 1 too keeps both;
an old plan text with one `sticks` line still loads.
`tests/backup_checklist.rs`: two vault items; the second does not
accept the vault the first filled. `tests/backup_stick.rs`: with two
vault files waiting, a visit ticks one.

**Docs.** `docs/WALLETS.md` §5, `docs/VAULT.md` (a vault per seed by
default), `docs/learn/07-backups.md` only if it says "the vault" as one
— it is upstream's, so leave it and note it in the report instead.

## 6. The wallet at a glance

### 6.1 What it shows

On the Wallets card (`Screen::Start`'s right pane, the card that opens
when a loaded wallet is pressed), the **Keys** section is replaced by
**At a glance**, a chart in three rows joined by lines:

1. **The wallet**, one node: its name, shape ("2 of 3 · native
   SegWit"), the descriptor's checksum.
2. **Its keys**, a node each, numbered as the Keys list numbers them
   today: fingerprint, label, and where it is: "Can sign here", "On its
   own device", "Cosigner · xpub only", "Waiting for the cosigner's
   xpub"; "· passphrase" when it was loaded with one. A line from the
   wallet to each key.
3. **Its backup**, a node per spot of the plan's map (`plan::map`):
   Place 1, Place 2 …, each vault ("Vault 1 · stick at Place 2"),
   unprotected files, watch-only software, on its own device. Each node
   lists what it holds, as the map panel does, with the tag (secret,
   sealed, public) and the checked state the Backups screen shows, and
   one line of what it gives whoever finds it alone: "Nothing",
   "Sees the balance", "Can spend", "Can spend with the vault's
   passphrase"; a vault's node reads "Sealed" and names the place its
   stick is kept. A line from each key to every node that holds its
   seed (words, SeedQR, file, a vault). Each key held here has its own
   line colour, from a categorical ramp of tokens, and its own dash
   pattern, so the lines are told apart without colour too; a cosigner's
   key has no lines. The wallet's description (sheet, share, in a vault
   or software) is a text line in the nodes that hold it, with no line
   drawn to it.

Under the chart, the plan's three check lines (`plan::CHECK_LINES` with
`plan::check`), as the backup's map panel shows them.

No plan known for the wallet: the third row is one node, "No backup
plan", with **Back up**. A plan kept in a locked vault and not
remembered (SIMPLIFY §3.4): "Plan in <vault>, locked", with **Unlock**.
A wallet whose keys are all elsewhere (watch-only) shows its key row
and backup row the same way.

The card's other parts stay: the signatures bar, the first receive
address, the buttons.

### 6.2 How it is drawn

- A new component, **Structure chart**, added to `docs/DESIGN.md` §4
  (after §4.15 Keys): nodes are the summary-row box (radius, padding
  and type of §4.5), lines are 1-point strokes from the bottom edge of a
  node to the top edge of the next row's, orthogonal (down, across,
  down) so they never cross a node. Every dimension a token in
  `tokens.rs`; add the ones it needs.
- The per-node "found alone" line is computed the way `plan::check`
  computes "One place found" for each place; add a function in
  `plan.rs` that returns it per spot, and have `check` use it, so the
  two never disagree.
- Rows wider than the card wrap onto a second line of nodes; the lines
  follow.
- Pressing a backup node opens Back up a wallet on this wallet at its
  checklist; pressing a key node opens that key in Seeds (where Seeds
  shows a key today).
- Small panel (`ui.compact`): the card shows an **At a glance** row that
  opens the chart as its own page (Back returns to the card). There the
  rows stack in one column and no lines are drawn; each backup node
  names the keys it holds ("Key 1 words · Key 2 in the vault") instead.

**Where.** `screens.rs` (the Wallets card; `backup_line` and the map
panel's code to reuse), `plan.rs` (the per-spot finding), a new
`faraday-core/src/glance.rs` for the chart's layout, `core/osk-ui/src/
tokens.rs`, `compact_screens.rs` for the small panel's page.

**Tests.** A new `tests/glance.rs`: a single-key wallet with the Paper
and vault preset shows Place 1, Place 2 and Vault 1, with the seed's
words at one place and the vault's stick at the other; a 2-of-3 with one
cosigner xpub shows that key as "Cosigner · xpub only" and no backup
node holds its seed; a place holding the whole sheet says "Sees the
balance"; with no plan, "No backup plan" and Back up opens the backup;
pressing a place opens the checklist.

**Docs.** `docs/DESIGN.md` §4 (the component) and §5 (the Wallets
screen), `docs/FLOWS.md` (Wallets tab), `README.md` "Screens" if its
Wallets screenshot changes.

## 7. The map from a vault, the other way up

### 7.1 It loads with the wallet

A vault holds each wallet's plan as a type 11 record (`docs/VAULT.md`):
its answers, its place names and its map lines. On unlock, a wallet the
vault holds is loaded with its plan, and its card's chart (§6) draws
from that record at once, without opening Back up. (Built with §6.)

### 7.2 The open vault shows it from the backups up

When a vault is open, its view (where SIMPLIFY §3.3 opens a vault just
unlocked) lists each wallet it holds with that wallet's chart reversed:
the backup row on top (places, vaults, files, software), the keys under
it, the wallet at the bottom, the lines running down from where each
seed is kept into its key and from the keys into the wallet. Each
backup node keeps its "found alone" line; the check lines go above the
chart, as the question a recovery starts from. Pressing the wallet node
opens its card in Wallets; pressing a place opens its Back up checklist,
as in §6. On the small panel, the stacked page of §6.2 in the reverse
order. A wallet with no plan in this vault shows its wallet and key
nodes only, with "No backup plan in this vault".

**Where.** `vault_screens.rs` (the open vault's view), the chart's
layout from §6 with its reverse direction, `compact_screens.rs`.

**Tests.** `tests/glance.rs`: an open vault holding a wallet with a plan
shows the chart with the places above the keys and the keys above the
wallet; pressing the wallet node opens its card.

**Docs.** `docs/DESIGN.md` §4 (the chart's two directions), §5 (the
vault view), `docs/VAULT.md` (what the view shows), `docs/FLOWS.md`.

## 9. Acting on the chart

The chart of §6 and §7 is also where the backup is looked after: any
node, and any line inside a node, can be pressed.

### 9.1 What a press does

A press opens a sheet for that node or line: what it is, where it is,
when it was last checked here, and its actions. Rules for every action:

- An action that changes where something is kept shows the check's
  three lines before and after ("One place found: can spend · No → Only
  with the vault's passphrase") and asks to confirm; a change that makes
  a line worse is shown in `WARN`.
- A secret copy is only ever made through the flows that exist: by hand
  with the copy check, into a vault, as a file after its warning. Files
  go For the stick, as everywhere.
- A secret action needs the seed loaded here. When it is not: "Load
  this key" with the way to it (unlock the vault that holds it, or add
  its words).
- Names, dates and marks (§9.5) are kept only in a vault. With none
  open, those actions read "Open a vault to keep this".
- On the small panel the sheet is a page; Back returns to the chart.

### 9.2 The wallet node

Show wallet QR (the descriptor, full text under it) · Public files for
the software (For the stick) · Print the wallet sheet again · Rename
the wallet · Check addresses (Create's Check) · Change the plan (Back
up's plan questions).

### 9.3 Key nodes

**A key held here:** Show xpub QR, Xpub file · **Make another backup of
this seed**: choose the form (words by hand, SeedQR by hand, into a
vault, Seed XOR parts, codex32 shares, a file), then the place (one of
the plan's, or a new one), then the form's own flow; the plan gains the
copy · **Check a copy of this seed** (Scan my copy, or the words typed
and compared; nothing kept but the result) · **Where it is** (its lines
on the chart drawn bold, the rest dimmed).

**A key not held here** (a cosigner's, or on its own device): Show its
xpub · **Name its holder** ("Alice's Coldcard"; kept in the vault like a
place's name) · **Mark checked**, the date its holder confirmed their
backup. Nothing secret is offered.

**A FROST share:** as a key held here, less the xpub.

### 9.4 Backup nodes and the lines inside them

**A place:** Rename · Mark checked (each thing in it, today) · What goes
in its envelope (the envelope list, For the stick as a PDF) · **Add
here**: a copy of a seed held here, its passphrase, the wallet sheet, a
share, a vault's stick · **Mark lost**, **Mark exposed** (§9.6) ·
Remove the place: asks where each thing in it goes first.

Inside a place, per line:

| Line | Actions |
|---|---|
| A seed's words or SeedQR | Check this copy · Make another copy (elsewhere) · Move to another place · I destroyed this copy · Lost · Exposed |
| A passphrase | Check it: typed, the key it makes compared with the loaded one by fingerprint, nothing kept · Move · Another copy · Lost · Exposed |
| The wallet sheet | Print again (PDF), as a QR picture, as text · Move · Another copy elsewhere |
| A share | Print again (sheet PDF, text file, QR picture) · Move · Keys left off each share (the slider; changes every share, so it is a plan change) |
| A vault's stick | Open that vault (unlock) · Copy the vault to another stick, at another place (a new line, "Vault 1 · copy") · Move the stick · Write it out again, when the vault has changed since it was written (SIMPLIFY §3.5) |

**A vault node:** Unlock or open · Rename · Save more into it (another
seed held here, a passphrase, the description), each a plan change ·
Its sticks, and Copy to another stick, as above.

**Watch-only software:** Show the descriptor QR again · Public files
for that software.

**Unprotected files:** what they are and where they were written ·
Write again · Remove from the plan, which asks to destroy the stick's
copies by hand and shows the check after.

### 9.5 What is kept, and where

The type 11 record (`docs/VAULT.md`) gains, per thing on the map: the
date it was last checked here, its holder's name for a key not held
here, and a mark (lost, exposed, destroyed). Never a secret: a
passphrase is checked by the fingerprint it makes, and only "checked
on" is kept.

### 9.6 Lost and exposed

Marking a place or a line lost or exposed runs the check with that
place gone, or found by someone, and says what follows, in order of
what to do:

1. Someone can now spend, or could with a vault's passphrase they
   might guess: **Move the money to a new wallet**, which opens Create a
   wallet and then Spend's sweep of every coin to it. In `ERR`.
2. The wallet can no longer be rebuilt from what is left: **Back up
   again now**, while the seeds held here can still sign. In `ERR`.
3. Someone can see the balance: a line saying so; nothing to undo.
4. Nothing lost that is not elsewhere: **Make a replacement copy**,
   opening the same form for the same seed.

A marked thing stays on the chart, struck through, until the person
removes it from the plan.

### 9.7 The plan once it is edited here

The plan's answers spread things over the places by rule (`spread`). An
edit on the chart (a move, a copy, an added or removed thing) is not a
rule, so from the first such edit the map itself is the plan: the type
11 record's map lines (field 4) are read back as the spots, and
`check` runs on them as it runs on a drawn map today. The answers stay
for the plan questions; Change the plan from them, or a preset, asks
first: "Your changes on the chart are replaced".

### 9.8 The owner's answers (2026-10-10)

1. Edits on the chart make the map the plan (§9.7): yes.
2. Lost and exposed, with the move-the-money way out (§9.6), in this
   pass: yes.
3. Holder names and checked dates for keys not held here (§9.3, §9.5):
   yes, in the vault only.
4. Copies of a vault on more sticks, each its own line (§9.4): yes.

## 10. Order

1. §1 and §2 together (mechanical).
2. §3.
3. §4.
4. §5.
5. §6 (with §7.1).
6. The passphrase wording (§3.3, 2026-10-10): the locked Key card's
   pill, the summary, and the empty field's "Not set" in
   `screens.rs` `pass_field`, which Create's Add a passphrase shares.
   And a test: a key locked in with a known passphrase, then a lock;
   no byte of the kept state (`Faraday::kept`) nor any file written to
   the stick holds the passphrase.
7. §7.2.
8. §9, in two batches: the sheets and the
   actions that exist as flows today (§9.1–§9.4); then the edited map,
   the marks and lost/exposed (§9.5–§9.7).

Then the owner's walk: a single key from Wallets to Backup done, with
dice by hand and a passphrase; a 2-of-3 with two keys made here and one
cosigner later; the 3-of-5 slider; each wallet's card at a glance.
