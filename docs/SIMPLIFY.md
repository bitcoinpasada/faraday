# Simplifying the everyday flows

**Status:** spec v0.2 · 2026-10-10 · v0.2 records the orchestrator's
rulings from handing out §1: later batches own the Home pieces they
introduce, and "checked" is defined in §2.5 · decided by the owner from a
user-perspective review of the built app (the snapshot tour at 1280×800
and 480×640, `docs/FLOWS.md`, `docs/WALLETS.md`, the Home, Wallets,
Create, Backup, Vault and Sign code). Built 2026-10-10, §1 to §6, one
commit per batch; §3.5 was built with §4, after §4.3. Where the build
and this file differ, `docs/FLOWS.md` and the code
are the record.

The problem it solves: the flows are sound and none ends on a dead end,
but the everyday surfaces are cluttered. Home and Wallets both try to be
a launcher; every flow opens on its first step even when that step has
an obvious default; the expert options sit beside the everyday ones
instead of behind them; and after a lock the person cannot see what
they have or where it is. Tools (`catalog.rs`) already gives experts
every flow with its kind preset, so the everyday surfaces can be quiet.

## 0. Rules for the implementing agent

- `CLAUDE.md` applies in full: every dimension a token, labels not
  prose, tests for user-facing behaviour only, `just` once at the end,
  no `rm`, no commits. Faraday's labels live in the screen code beside
  the screen, as the neighbouring screens do.
- Work the batches in order (§1 to §6). Each batch is one review and
  one completion report. Do not start a batch before the previous one's
  tests pass.
- Where this file names a test, add it to the named file under
  `faraday/faraday-core/tests/`, driving the app through `Action`s as
  `tests/home.rs` does. A test states what a person sees or can do,
  never a geometry or a token.
- Where this file names a doc, update it in the same batch so that the
  doc and the build agree. `docs/FLOWS.md` gets a dated decision line
  per change to a flow; `docs/DESIGN.md` §4 or §5 per change to a
  rendering rule; `docs/WALLETS.md` §5 for Create and Backup;
  `docs/VAULT.md` for anything about a vault's file or state;
  `docs/FAMILY.md` for the Spend tab; `README.md` "Screens" if a
  screenshot no longer matches.
- Render to look: `local/remote-run.sh --fetch out/snapshots/faraday/1280x800
  just faraday-snap 1280x800` and the same for `480x640`. The tour in
  `faraday/shells/snapshot/src/main.rs` drives every flow; where a step
  this file removes or reorders is named there, change the tour too.
- Completion report, one per batch: the items done, each with its
  tests' names and whether they pass; what was left out and why; the
  docs changed; the render names to look at. No restating the spec.
- Later batches own what they introduce. §1 builds Home from the state
  that exists today and leaves a one-line comment where a later piece
  goes; §3.4 adds the summary line to the Unlock lead, §3.5 adds lead
  rule 6, §4.3 adds lead rule 4, and §5 adds the Backups tile and the
  wallet card's line, each as that batch's last item, with the Home
  tests for it in `tests/home.rs`. The session strip's "a vault has
  changed since it was written" clause is likewise §3.5's.
- Old Home tiles that §1.2 does not name (Load a wallet, Bring a PSBT
  in, the import card, the locked-vault card) go; their jobs are the
  lead tile's or another screen's. Tests that encoded the old Home are
  changed to the new rule, never deleted.

## Decisions recorded (owner, 2026-10-10)

1. Create a wallet shows two kinds: **Single key · native SegWit**
   (default) and **Multisig · native SegWit**. The other eight are
   behind **More kinds**.
2. Create a vault offers two sizes as cards: **PCs only** (512 MiB) and
   **PCs and a Raspberry Pi** (64 MiB), both 3 passes. The present
   three sizing cards are behind a small **Customise** button.
3. The boot sheet stays first, before Home. It becomes a front door:
   one line of what was read, the stick pull, then Unlock on the sheet
   itself. Everything is imported by default.
4. A vault's contents, seen while it was open, are remembered across a
   lock in the kept state, as the signed-amount memory is. Accepted
   trade-off: wallet names and fingerprints are in RAM while the vault
   is locked; nothing that spends is.
5. The Outbox becomes a queue that fills itself and a receipt that
   stays; the Inbox brings everything in by default. The two halves are
   named by direction: **From the stick** and **For the stick**.
6. Guided is the default on a device that has never had a setting
   saved; Steps only once the person chooses it. This reverses the
   2026-10-07 default for a first start only.

## 1. Home and the sidebar

### 1.1 Home is the Start section

**What.** On `wide`, remove the three status cards (Session, Files,
Sticks) from Home. The sidebar's status block already carries all
three. Home is the title, then **Start**.

**Where.** `screens.rs` `fn home` (the `cards` array and its loop).
The sidebar status block is unchanged.

**Tests.** `tests/home.rs`: with nothing loaded, Home's hit targets are
Start's tiles, Scan and the sidebar, and no target opens Files or the
visit from a status card.

### 1.2 Start offers jobs, ranked, at most three

**What.** Start is one lead tile, full width, then at most two
secondary tiles, half width each. The lead is the first of these that
applies:

1. a spend under way: **Continue signing {source}** · "{n} of {m}
   signatures"
2. a PSBT in Files (`lead_psbt`): **Sign {name}** · "From Files"
3. an import waiting: **Import from {label}**, or **Remove the stick to
   start the import** when the boot stick is still in (§4.4)
4. a receipt from this power-on's last stick visit and nothing loaded
   (§4.3): **Written to {label}** · the file names · tapping opens Files
5. a locked vault file and nothing loaded: **Unlock {name}**, with the
   remembered summary (§3.4) as its line when there is one; with a
   stick in, the line "Pull the stick to unlock" and no target
6. a vault changed since it was written (§3.5): **Write {name} to a
   stick**
7. otherwise: **Make a wallet** · "Single key or multisig", opening
   Create.

The secondary tiles (owner, 2026-10-10): the first is always
**Wallets** ("{n} wallets", or "Create or restore" when none). The
second is **Stick visit** when a stick is attached and no secret is
held; otherwise **Backups** (§5) once any wallet is known; otherwise
**Vaults** ("{n} open" / "{n} locked" / "None yet"). **Add a key** is
not on Home: it is on Wallets' empty state and in Tools.

**Where.** `screens.rs` `fn home` (the `tiles` vector and its loop);
`compact.rs` `fn prompts` and `fn tiles` for the small panel, where the
lead is the prompt row and the grid drops **Add a key** and **Spend**
(§6.2) and gains **Learn** (§1.4).

**Tests.** `tests/home.rs`: one test per lead rule, each setting up the
state and asserting the lead's label and where its tap lands; a test
that Home never shows more than three Start tiles.

### 1.3 Wallets is the list and the card

**What.** Remove **What do you have?** and **What do you want to do?**
from the Wallets start page (`fn start`, from `section_label(ui, x, y,
"What do you have?")` to the end). With wallets loaded, the page is
**Your wallets** and the wallet card (`fn wallets`); with none, the
three ways in (Create, Load or restore, Add a key) as now. The jobs
those sections offered are reached from the wallet card (Sign a
transaction, Back up, Show wallet QR, Sign a message where the wallet
allows it) and from Tools.

**Where.** `screens.rs` `fn start`; `fn wallets` gains **Sign a
message** beside **Back up** when `message_wallets()` holds the card's
wallet. `compact_screens::wallet_card` the same.

**Tests.** `tests/home.rs` or a new `tests/wallets_page.rs`: with
wallets loaded, the Wallets page's targets are the wallet rows, the
card's buttons and Add a key; the card offers Sign a message for a
single-key wallet with its key here and not otherwise.

### 1.4 Learn in the sidebar and on the small Home

**What.** The sidebar lists **Learn** after Tools, opening the Learn
sheet (`Action::Learn`). The small panel's Home grid has a **Learn**
tile in the same place. The "?" button stays.

**Where.** `screens.rs` sidebar `items` (about line 342); `compact.rs`
`fn tiles`.

**Tests.** `tests/sidebar.rs`, `tests/small_panel.rs`: Learn is
offered and opens the Learn sheet.

### 1.5 The network pill only off mainnet; the chooser in Settings

**What.** Home's network pill (`fn network_pill`, and the small Home's
pill in `compact.rs` `fn home`) is drawn only when the network is not
mainnet, as a badge that opens nothing. Settings gains a **Network**
row of the Setting kind (DESIGN §4.2): Mainnet, Testnet, Signet,
Regtest, applying at once through what `Sheet::Network` applies today.
Every way of opening the sheet goes, the sidebar's test-network badge
included (it becomes a badge that opens nothing), and the sheet is
removed once nothing opens it.

**Where.** `screens.rs` `fn network_pill`, `fn settings`; `compact.rs`
`fn home`; `lib.rs` `Action::NetworkAsk`.

**Tests.** `tests/network.rs`: on mainnet Home has no network target;
Settings changes the network; off mainnet Home shows the badge.

### 1.6 The session strip

**What.** The sidebar's status block (and the small Home's status card)
carries one line of four words, **Bring in · Open · Work · Write out**,
the current stage in the accent. The stage is computed, never stored:

- **Bring in**: a stick is attached, or an import is waiting;
- **Write out**: files are **For the stick** (§4.2), or a vault has
  changed since it was written (§3.5), and no stick is attached;
- **Work**: a key, wallet or open vault is loaded;
- **Open**: none of the above.

Tapping the strip opens Files. It explains the lock cycle without
prose: a stick attached while working shows **Bring in** while the
sheet asks to lock.

**Where.** `screens.rs` sidebar status block; `compact.rs` `fn home`
status card; a `fn session_stage(&self)` on `Faraday` in `lib.rs`.

**Tests.** `tests/sidebar.rs`: the stage for each of the four states.

**Docs.** `docs/DESIGN.md` §4.1 (Status line, Sidebar), `docs/FLOWS.md`
Home.

## 2. Flows

### 2.1 A flow opens on the first step that needs the person

**What.** A step card whose value has a default is closed at entry,
showing its value and a **Change** text button at the card's right (in
place of the chevron) that opens it. The flow opens on the first card
with no default. Opening a closed card does not reset the cards after
it. Applies to Create a wallet (Kind, Quorum), New key (Length,
Randomness), Back up (already so after a preset), Sign a transaction
(§2.5), and Create a vault once §3.1 has given it its defaults: in §2
the vault flow only gains the `default` flag on its cards, and opens
where it opens today. On the small panel the same:
`flow::paged` starts on the first open card.

**Where.** `flow.rs` `Card` gains `default: bool`; `column`,
`column_foot` and `paged` draw a closed-with-default card with
**Change**; each flow's card builder sets it. The flows' `open` state
at entry: `create.rs`, `keygen.rs`, `vaults.rs`, `wallet.rs`.

**Tests.** `tests/create_vault.rs`, a new `tests/create_wallet.rs`,
`tests/keygen.rs`: at entry the open card is the one this file says;
Change opens the card; the cards after it keep their values.

**Docs.** `docs/DESIGN.md` §4.14 (Progress and state) gains the rule.

### 2.2 Create a wallet: two kinds, the rest behind More kinds

**What.** Kind shows two rows, **Single key · native SegWit** (checked
by default) and **Multisig · native SegWit**, and a **More kinds** text
button that expands the other eight rows in today's order. With the
default kind, Kind is closed ("Single key · native SegWit · Change")
and the flow opens on Keys. Choosing Multisig opens Quorum, which has
the default 2 of 3 and is also closed with Change once Multisig is
chosen from a closed Kind; the flow then opens on Keys. Tools' tiles
(`Go::Create(kind)`) open Create with that kind chosen and Kind closed,
whatever the kind.

**Where.** `screens.rs` `fn create_screen` (`CSTEPS`, the Kind card's
rows), `create.rs`.

**Tests.** `tests/create_wallet.rs`: a fresh Create opens on Keys with
the single-key kind; More kinds offers FROST; a Tools tile for MuSig2
opens Create with MuSig2 chosen and Kind closed.

### 2.3 Create ends in the backup plan

**What.** `CSTEPS` becomes Kind · Quorum · Keys · Check · Back up. The
**The wallet** card goes: Keys' Continue builds the wallet (what **Make
the wallet** does today) and opens Check. Check shows the descriptor as
a summary row ("2 of 3 · native SegWit · 9a6a2580, cf0e9805, 048ab54e ·
#qf45pmyh", DESIGN §4.5), tapping which opens the sheet **Show wallet
QR** opens, which must carry the descriptor's full text under the code;
then the first addresses as now. **Secrets into a vault**, **Public
files** and **Paper backup** go: they are items 3, 5 and 7 of the
backup checklist (`docs/WALLETS.md` §5). The **Back up** card's body is
the plan's three presets; choosing one opens Back up a wallet
(`Screen::Backup`) on this wallet with that preset applied, and its
chip reads "Then: Wallets". Back from the backup returns to the wallet
card. The snapshot tour's `create` steps that press the removed cards
change to this path.

**Where.** `screens.rs` `fn create_screen`, `create.rs` (`cstep`),
`backup.rs` entry from Create, `faraday/shells/snapshot/src/main.rs`.

**Tests.** `tests/create_wallet.rs`: after Check, the next card is Back
up with three presets; choosing Paper and vault opens the backup on the
new wallet with that preset; Back returns to the wallet card.
`tests/backup_plan.rs`: the plan's public-files item lists the files
Create's Public files card listed, drawn from the wallet.

**Docs.** `docs/WALLETS.md` §5 (the Create table and the Backup
paragraph), `docs/FLOWS.md` decision 6's "Create a wallet says the same
in its cards" paragraph.

### 2.4 Add a key: BIP-39 words first

**What.** The four form chips (BIP-39 words, SLIP-39 shares, codex32,
Seed XOR parts) are replaced by the words form alone and two text
buttons under the fields: **Other forms** (which shows the chips) and
**Other languages** (as now). Opened from a Tools tile for a form
(`Go::AddKey(form)`), the screen opens on that form with the chips
shown and the title "Add a key · SLIP-39 shares" (the form's name).

**Where.** `screens.rs` `fn entry` (about line 11150 on), `catalog.rs`
unchanged.

**Tests.** `tests/forms.rs`: a fresh Add a key offers the words form
and no share form until Other forms; the SLIP-39 tile opens on shares.

### 2.5 Sign a transaction: Transaction before Check, no Txid step

**What.** `STEPS` order becomes Wallet · Transaction · Check · Signers ·
Sign · Signatures for this transaction · Finish (Path and Nonces where
the kind needs them, in their present places relative to Signers). The
**Transaction id** card goes; its value is a **Txid** row at the foot
of the Transaction card's table, so the small panel, which has no side
panel, still shows it. A wallet counts as **checked** in this power-on
once a person has pressed Continue on its Check card (Sign a
transaction), **It matches** on the Spend tab's "Check the money is
really there" page, or Continue on Create's Check card. `Faraday` keeps
the checked wallets by descriptor checksum, in memory and in the kept
state across a lock (`memory.rs` `kept`, key `checked-wallets`), and a
fresh process with no kept state has none. Check is open at entry when
the wallet is not checked, and closed ("Addresses · Compare") when it
is; opening it and pressing Continue again is allowed and changes
nothing.

**Where.** `screens.rs` `fn spend` (`STEPS`), `wallet.rs` `mod step`,
`memory.rs`, `faraday/shells/snapshot/src/main.rs` `spend_tour`.

**Tests.** `tests/spend_loaded.rs` or `tests/finish.rs`: the card after
Wallet is Transaction and carries the txid; a wallet whose Check card
had Continue pressed has Check closed on the next spend, still after a
lock and the next process, and open again in a fresh process; a wallet
never checked has Check open.

**Docs.** `docs/WALLETS.md` §4, `docs/FLOWS.md` Wallets tab.

### 2.6 The small panel's step title

**What.** `flow::paged` draws no pip row. The title is the card's name
alone; the step count is its own muted "Step N of M" text beside it, so
a step count can never be read as one of the card's own values — on
Restore's Quorum card, beside the wallet's own "2 of 3" (DESIGN §4.1,
Title). The step count moved off the title and beside it, 2026-10-10
(owner).

**Where.** `flow.rs` `fn paged`.

**Tests.** `tests/small_panel.rs`: a flow's page title carries no
count, and the step count is drawn as its own text.

### 2.7 Scan never covers a tile

**What.** On the small Home the grid reserves a bottom inset of the
Scan button's height plus a gap, so the last row scrolls above it and
no tile is under the button.

**Where.** `compact.rs` `fn home`, the `tiles` loop and
`compact_screens::finish`.

**Tests.** `tests/small_panel.rs`: with the full grid, every tile's hit
area is clear of Scan's.

### 2.8 The idle sheet has no clock

**What.** The idle sheet's title is **Locking soon** and its body says
"No input for {n} minutes · locks at {m} · any key or touch keeps it".
No count down anywhere on it (DESIGN principle 10). The sheet still
closes on input and the lock still fires at the set time.

**Where.** `screens.rs` idle sheet (about line 13270).

**Tests.** `tests/idle.rs`: the sheet's text does not change from one
second to the next; the lock fires at the set time.

## 3. Vaults

### 3.1 Create a vault: two sizes, then Customise

**What.** Create a vault's cards become **Size** and **Name and
passphrases**. Size shows two rows: **PCs only** (512 MiB · 3 passes)
and **PCs and a Raspberry Pi** (64 MiB · 3 passes), the second checked
by default, each with its "unlocks in about {t} here" line and "needs
about {m} free". Under them a small **Customise** text button opens the
present three cards (Where will you open it, Unlock cost, Space per
passphrase) in place of Size; their values feed the same `vaults.rs`
state. The side panel's summary is unchanged. Size has a default, so
the flow opens on Name and passphrases (§2.1). Memory that this
computer cannot spare is refused as now.

**Where.** `vault_screens.rs` (the Create vault cards, `VSTEPS` at
line 837), `vaults.rs` (`PRESETS` gains the 512 MiB entry; the
`cost` preset index).

**Tests.** `tests/create_vault.rs`: a fresh Create opens on Name and
passphrases with 64 MiB chosen; PCs only gives 512 MiB; Customise
exposes the three cards and a custom memory survives Continue.

**Docs.** `docs/VAULT.md` §3.1 presets, `docs/FLOWS.md` Vaults.

### 3.2 The name says what the vault holds

**What.** The Name field's caption reads "What it holds, for anyone who
sees the stick · leave empty for vault.ofv" (the rule of `docs/VAULT.md`
§6). No other change.

**Where.** `vault_screens.rs` Name and passphrases card.

### 3.3 After creation: once more to open it; an empty vault's next steps

**What.** After **Create vault**, the Unlock screen opens with the new
vault picked (FLOWS decision 10) and a caption under the title: "Type
the passphrase once more to open it". An open vault with nothing in it
shows "Nothing in it yet" and two buttons, **Put a wallet in it**
(opens Wallets) and **Write it to a stick** (opens Files, where the
vault is already For the stick, §4.2). Vault contents lists only the
categories that hold something, then one **Add…** row that lists every
kind as the category rows' Add actions do today.

**Where.** `vault_screens.rs` Unlock caption, Vault contents.

**Tests.** `tests/vault_way.rs`, `tests/vault_keys.rs`: Create lands on
Unlock with the caption; an empty open vault offers the two buttons; a
vault with one key lists Keys and Add… and no other category.

### 3.4 What a locked vault held is remembered across a lock

**What.** While a vault is open, `Faraday` keeps a `VaultSummary` for
it: file name, vault name, its wallets (name, shape), its keys
(fingerprints), its entry count, its backup map lines (the type 11
records' field 4 lines), and the time it was last open. The summaries
are written into the kept state at lock (`memory.rs` `kept`, key
`vault-summaries`) and read back by the next process; they are gone at
power-off. The Vaults list row of a locked vault with a summary reads
"{name} · locked · Savings 2 of 3 · key 9a6a2580 · 12 entries · seen
14:02" in place of the size and passes; Home's Unlock lead (§1.2 rule
5) uses the same line; the Backups screen (§5) reads from it. A locked
vault with no summary reads "Unlock to see what it holds".

**Where.** `vaults.rs` (`VaultSummary`, built on unlock and on every
save), `memory.rs`, `vault_screens.rs` Vaults list, `screens.rs` Home.

**Tests.** `tests/vault_keys.rs` or a new `tests/vault_summary.rs`:
after unlock, lock and the next process, the Vaults list names the
vault's wallet and key; after power-off (a fresh `Faraday` with no kept
state) it does not.

**Docs.** `docs/VAULT.md` gains a section "What the app remembers of a
locked vault", stating the trade-off (decision 4). `PLAN.md` §5.2 or
wherever the kept state is listed.

### 3.5 A vault's currency

**Built in the §4 batch**, after §4.3 and before §4.5: its "On {label}
· current" state reads the receipt's hashes, which §4.3 makes, and
§4.5's sheets read the state (orchestrator, 2026-10-10).

**What.** Each vault file has one of three states, computed: **Never
written** (sealed into For the stick, no receipt names it); **On
{label} · current** (the last receipt that names it wrote the bytes it
now has); **Changed since written** (open with unsaved changes, or
sealed bytes that differ from the receipt's). The receipt (§4.3) keeps
each written file's hash for this. The state is a line on the Vaults
list row and the Home lead of §1.2 rule 6, and the first rows of the
lock and power-off sheets (§4.5).

**Where.** `vaults.rs` (`fn currency`), `vault_screens.rs`,
`screens.rs` sheets.

**Tests.** `tests/vault_way.rs` or `tests/vault_summary.rs`: the three
states in order across create, write, change.

**Docs.** `docs/FLOWS.md` Vaults overview.

## 4. Files: from the stick and for the stick

### 4.1 Names by direction

**What.** Everywhere a person reads "Inbox" or "Outbox": **From the
stick** and **For the stick** ("From the SD card", "For the SD card" on
the Pi, through `Medium`). The Files screen's two columns, the sidebar
status row ("2 for the stick"), the stick visit's two panes ("Write to
the stick" / "Copy from the stick"), the sheets, every button that said
"To the Outbox" or "Put in the Outbox" (§4.2 changes most of them), the
boot sheet, the lock and power-off sheets, Learn's pages that name them.
Code names (`inbox`, `outbox`, `FileKind`) do not change.

**Where.** `grep -n 'Inbox\|Outbox' faraday/faraday-core/src/*.rs
core/osk-learn/src/en.rs docs/learn/` and every match a person reads.

**Tests.** `tests/inbox.rs`: no screen's labels contain "Inbox" or
"Outbox". (This is a test of what a person sees; it exempts nothing
because nothing shown should carry the words.)

**Docs.** `docs/FLOWS.md` decision 1, `docs/DESIGN.md` where it names
them, `README.md`.

### 4.2 For the stick fills itself

**What.** A public file a flow makes goes to For the stick the moment it
is made: a signed PSBT and a finished transaction at Finish, a
descriptor, wallet file, multisig config, BSMS record or Core import
when the backup checklist is made (its public-files item lists them
with **Remove** per row and is done when they are there), a backup
sheet or template when rendered, a GPG public key, revocation or
signature when made, a signed message, a sealed vault at lock. The
**To the Outbox** and **Put in the Outbox** buttons go; where one stood,
the row reads "For the stick" with **Remove**. Sign's Finish keeps
**Show as QR** and its primary becomes **Insert a stick to write it**
(opening Files), or **Open Files** when a stick is in. The secret sheet
is unchanged and remains the one way a secret reaches For the stick;
the code's refusal of a secret on the plain path stays.

**Where.** `lib.rs` (the one `outbox.push` and the function around it,
which every public file already passes through; `secrets.rs` keeps its
own push for the secret sheet),
`screens.rs` Finish, the backup checklist's public-files item, GPG and
message screens, `vault_screens.rs` lock.

**Tests.** `tests/finish.rs`: finishing a spend puts both files For the
stick with no further press; `tests/backup_checklist.rs`: making the
checklist with Sparrow chosen puts the wallet file For the stick and
the item is done; `tests/secrets.rs` unchanged and still passing.

**Docs.** `docs/FLOWS.md` decision 6 and the Finish line of the Wallets
tab, `docs/WALLETS.md` §5 item 7.

### 4.3 The receipt

**What.** When a stick visit's write completes and reads back, the
visit's result becomes a `Receipt { label, time, files: Vec<(name,
hash, verified: bool)> }` kept on `Faraday`, written into the kept
state at lock (`memory.rs`, key `receipt`) and read back by the next
process; gone at power-off; replaced by the next write. Files shows it
under For the stick as **Written to {label} at {time}**, one row per
file with "verified" or the compare failure; Home leads with it by
§1.2 rule 4; §3.5 reads its hashes. The Written list is not a queue:
nothing on it is written again.

**Where.** `lib.rs` visit write completion, `memory.rs`, `screens.rs`
`fn files` and `fn home`, `compact.rs` prompts.

**Tests.** `tests/visit.rs`: after a write, Files lists the receipt
with the file names; after the lock that follows and the next process,
it is still listed and Home leads with it; a fresh process has none.

### 4.4 The boot sheet is a front door

**What.** The boot import sheet (`boot_import.rs`, `boot_import_screen.rs`)
becomes:

- **With the stick in:** the title "Read from {label}", one line "{n}
  vault · {m} PSBTs · {k} other files" (only the counts that are not
  zero), and the line "Pull the stick to continue". No lists. (The
  reading itself finishes before the sheet shows, as now.)
- **Once pulled, with one vault:** the same title, the vault's name, a
  passphrase field and **Unlock** on the sheet, **Not now** beside it,
  and a small **Choose what to import** text button. Unlock that
  succeeds imports everything (what `ImportAction::Go` does with every
  wallet, key and file chosen) and closes the sheet; Home then leads by
  §1.2. A wrong passphrase stays on the sheet and says so. **Not now**
  closes the sheet with the files waiting, as **Import later** does
  today; Home leads with **Unlock {name}**.
- **Several vaults:** a row per vault, each with **Unlock**, which
  opens the passphrase field under that row; unlocking any one imports
  everything and closes the sheet; the others stay on Vaults, locked.
- **Choose what to import** expands today's ticked lists (wallets,
  keys, files) under the field; **Import** then imports what is ticked
  without unlocking.
- **No vault, some files:** once pulled, everything goes From the stick
  at once and the sheet closes; Home leads with **Sign {psbt}** when
  there is one.
- **Only the settings file:** no sheet.

The Sticks card and the import card on Home go with §1.1; the Home lead
of §1.2 rule 3 reopens the sheet.

**Where.** `boot_import.rs` (`ImportAction` gains `Passphrase(String)`
or reuses the Unlock screen's field logic from `vault_screens.rs`),
`boot_import_screen.rs`, `screens.rs`.

**Tests.** `tests/boot_import.rs`, `tests/boot.rs`: each bullet above
as one test: what the sheet offers with the stick in, after the pull
with one vault, with two, with none and files, with settings only; a
right passphrase imports the wallets and closes; a wrong one stays;
Not now leaves Home leading with Unlock.

**Docs.** `README.md` "Starting a session: the boot import", `PLAN.md`
§5.4, `docs/FLOWS.md` Session.

### 4.5 The lock, write-out and power-off sheets lead with what is kept

**What.** The three sheets (a stick attached while unlocked; Write to a
stick from Backup done; Power off) order their rows: **Kept** first,
one line per vault with its currency (§3.5) and one for the files For
the stick; then **Wiped from memory**, one line per seed and wallet not
in a vault, each annotated from the backup state: "copy checked",
"in {vault}", or "nowhere else" (that line alone in the danger tone);
then **Then**. The lock sheet's title stays.

**Where.** `screens.rs` the three sheets (the "Wiped" and "Kept"
rows at lines 12502, 12509, 13241, 13258 and the "Not in a vault" row
at line 12626), `vaults.rs` `unsaved`,
`Faraday::backup_kept`.

**Tests.** `tests/airgap.rs` or `tests/backup_stick.rs`: with a seed
copied and checked and in a vault, the lock sheet lists it as kept
and nothing as "nowhere else"; with a seed in memory only, that seed
is listed "nowhere else".

### 4.6 Everything comes in by default

**What.** On a stick visit every readable file is ticked for From the
stick, except what today's rules keep unticked: a secret kind (through
the secret sheet), a file over 18 MiB, a file already present by name
and size. **Select all** goes; **Unselect all** takes its place.

**Where.** `screens.rs` stick visit (about line 10400), `lib.rs` visit
state.

**Tests.** `tests/visit.rs`: a fresh visit has the PSBTs and
descriptors ticked and the words file not.

### 4.7 The visit says where it goes next

**What.** The stick visit carries the chip "Then: Unlock" when a vault
was sealed for it, else "Then: Home" (the `Column` chip, as Create a
vault's "Then: {flow}"). After **Remove the stick** it goes there, and
Home leads with the receipt (§4.3).

**Where.** `screens.rs` stick visit header.

**Tests.** `tests/visit.rs`: the chip's text in both cases.

## 5. Backups

### 5.1 The Backups screen

**What.** A new `Screen::Backups`, reached from Home's secondary tile
(§1.2) and from the wallet card's status line (§5.2). It lists every
wallet the app knows about: loaded wallets, wallets in open vaults, and
wallets in remembered summaries (§3.4), once each by descriptor
checksum. Under each, the backup map as the plan draws it (one line per
place, the vault, a stick of files, the software, the seeds on their
own devices, each tagged secret, sealed or public), with each line's
state from `Faraday::backup_kept`, the plan record (type 11) where a
vault holding it is open, the receipt (a public file "on {label},
{time}"), and the vault's currency. A wallet with no plan reads "No
backup plan" with **Back up**. Nothing is marked done that the device
did not see. The screen is a Navigate screen (DESIGN §5): a list from
the top, scrolling. On the small panel, one wallet per page.

**Where.** `lib.rs` (`Screen::Backups`, `Action::Nav`), a new
`backups_screen.rs`, `backup.rs` (`fn backup_kept` extended to read a
plan record and a receipt), sidebar unchanged.

**Tests.** A new `tests/backups.rs`: a wallet backed up with Paper and
vault shows its two paper places checked and the vault line sealed;
after the vault is written, the vault line names the stick; a wallet
with no plan offers Back up; a wallet seen only in a locked vault's
summary is listed from it.

**Docs.** `docs/WALLETS.md` §5 gains "The Backups screen";
`docs/FLOWS.md` a Backups section; `docs/DESIGN.md` §5 adds the screen.

### 5.2 The wallet card's backup line

**What.** Under the shape line of the wallet card, one line: "Backup:
paper ×2 checked · vault.ofv · for the stick" built from the same
sources as §5.1, or "Not backed up" with the **Back up** button already
there. Tapping the line opens Backups on that wallet.

**Where.** `screens.rs` `fn wallets` card, `compact_screens::wallet_card`.

**Tests.** `tests/backups.rs`: the card's line for a backed-up wallet
and for a fresh one.

## 6. Guided and the Spend tab

### 6.1 Guided is the first default

**What.** `guided` defaults to `true` when the kept state and the
stick's settings file carry no `guided` value (a device that has never
had a setting saved); once the switch is pressed either way, the value
is saved with the settings as now. The owner's 2026-10-07 default
(Steps only on the left, where a first start is) becomes: Steps only on
the left; a first start is on Guided.

**Where.** `lib.rs` `guided: false` at construction;
`stick_settings.rs` `apply_setting`.

**Tests.** `tests/settings_file.rs`: a fresh app is Guided; one whose
settings file says `guided=0` is Steps only.

**Docs.** `docs/WALLETS.md` §6, `docs/FAMILY.md` §2.

### 6.2 Spend is the step-by-step face, not a second tab

**What.** **Spend** leaves the sidebar and the small Home grid. The
Spend tab's screen (`Screen::Family`) and code are unchanged and are
reached from: the Wallets empty state, a fourth way in, **Spend from a
backup, step by step** ("A stick, words, or paper"); the Learn sheet's
first row, **Spending, step by step**; and Home's lead when a spend
started in it is under way (§1.2 rule 1 names it). The family page
kept across a lock is unchanged.

**Where.** `screens.rs` sidebar `items`, `fn start` empty state,
`learn_sheet`; `compact.rs` `fn tiles`.

**Tests.** `tests/family.rs`: the Spend tab opens from the Wallets
empty state and from Learn; `tests/sidebar.rs`: the sidebar has no
Spend row.

**Docs.** `docs/FAMILY.md` §2 "Where it sits", `docs/FLOWS.md` Spend
tab.

## 7. Order and gates

Batches: §1, §2, §3 (3.1 to 3.4), §4 (4.1 to 4.3, then 3.5, then 4.4
to 4.7), §5, §6, each ending with its tests, `just`,
the two renders, and the completion report. §4.4 (the boot sheet) and
§5 (Backups) are the two with the most new code; do not fold either
into another batch. After §6, `local/REMAINING.md` gets the owner's
test list for this pass: the desktop app's Home with nothing loaded,
Create a wallet to Backup done on a 2-of-3, the boot sheet with one
vault on the release stick, and the Backups screen after a stick
visit.
