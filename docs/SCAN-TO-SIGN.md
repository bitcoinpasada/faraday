# Scan to sign, and the Spend tab's intake

**Status:** built 2026-10-11: batch A `ceb06e4`, batch B in the commit after it; spec v0.1 · decided by the owner
(`docs/DECISIONS.md` F8, HANDOFF item 67). Where the build and this file
differ once built, `docs/FLOWS.md`, `docs/FAMILY.md` and the code are the
record.

The owner tried the demo's backup start (Seed A loaded from the vault,
Seed B on paper) and found that bringing in the second signer took three
presses through a page about typing, behind a link on a key box chosen
by fingerprint, when the person does not know which key their paper is.
The Spend tab, meant for a first-time spender, asks them to classify
their envelope before anything is read. This spec makes scanning the
first way in everywhere a key is missing, lets Faraday work out which key
a paper is, marks a wallet that can spend, and turns the Spend tab's
first question into "bring in what you have". It also adds batch signing
to GPG's Sign a file.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every build and test
  through `local/remote-run.sh`; every dimension a token; labels, not
  prose, on working screens (the Spend tab's lead and More text are the
  tab's own, `docs/FAMILY.md` §1 rule 6); tests for user-facing
  behaviour only; `just` once at the end of a batch; no `rm`; no
  commits; no agents.
- Two batches, in order. **Batch A:** §1, §2, §3, §6. **Batch B:** §4,
  §5. Each is one review and one completion report. Do not start batch
  B before batch A is reviewed.
- Tests drive the app through `Action`s, `Command`s and scanner payloads
  as the neighbouring tests do (`tests/scan.rs`, `tests/glance.rs`,
  `tests/family.rs`, `tests/spend_from_vault.rs`, `tests/gpg.rs`), on the
  test kit's seeds (`testkit::TEST_SEEDS`) and its 2-of-3 wallet. A test
  states what a person sees or can do; never a geometry, a token or a
  colour.
- An existing test that asserts old wording or the old routes is
  changed to the rule it now encodes, never deleted to pass.
- Docs in the same batch as the code they describe (§7).
- To look: `local/remote-run.sh --fetch out/snapshots/faraday/1280x800
  just faraday-snap 1280x800`, and `240x320` for the small panel. The
  snapshot tour (`faraday/shells/snapshot/src/main.rs`) follows the new
  wording and routes; §7 names the shots it gains.
- Completion report: the items done, each with its tests and whether
  they pass; what was left out and why; docs changed; render names to
  look at. No restating the spec.

## Decisions (owner, 2026-10-11)

1. **One way to scan any missing key.** Where a wallet is short of keys,
   one **Scan a key** opens the camera at once and takes any of the
   wallet's missing keys; Faraday works out which. The per-key "Add its
   key" goes.
2. **Can spend.** A wallet with enough keys here to sign shows a green
   **Can spend** pill, on the wallet card and on the Spend tab's Who
   signs, and its list row says so.
3. **Names on Home:** "Make a new wallet" and "Restore an existing
   wallet".
4. **The Spend tab asks for data, not a classification.** "Scan the QR
   codes you have" and "Type the words you have"; Faraday works out the
   wallet from what comes in and asks only the next question. A vault
   is offered only when one is found.
5. **One seed and "That's everything"** opens native SegWit without
   asking. The Check page says plainly to look on the online computer,
   and **No money shows** walks through a passphrase, Taproot, nested
   SegWit and legacy; Continue only through **Yes, my money shows**.
6. **Seeds without a description** build standard multisig, native
   SegWit by default, only when every key is in hand as a seed or an
   xpub.
7. **Restore on the Wallets tab** stays as it is; it may take this front
   end later (REMAINING).
8. **GPG Sign a file** signs several files at once from a checkbox list.

Decided in this spec (architect, inside the owner's eight):

9. A scanned seed whose key with no passphrase is one of the wallet's
   missing keys loads at once, with no passphrase question: the match
   proves it has none. A seed that matches nothing opens the passphrase
   sheet (`seed_read.rs`) with the reason, so a seed with a passphrase
   is still found. Everywhere else the passphrase sheet works as
   decided on 2026-10-11.
10. A missing key's chart node keeps its own sheet and loses its "Add
    its key" line; the one Scan a key is in the signatures bar. A node
    whose key is in an open vault keeps **Load from vault**, which says
    which key it is.
11. On Who signs (the Spend tab's and Wallets' Sign step), the scan and
    unlock buttons show only while the wallet is short of signers;
    for a wallet whose spending paths differ (miniscript and tree
    kinds, `guide.rs`'s `Kind::Miniscript | Kind::Tree`) they stay until
    the spend is signed, since a count of keys does not settle which
    path signs.

## 1. Names on Home and Wallets (batch A)

Every way in that names these two flows reads the same:

| Was | Now |
|---|---|
| Make a wallet (Home lead and tile) | Make a new wallet |
| Create a wallet (Wallets empty state, its small-panel row; Create's heading and the back labels that name it, `keygen_screen.rs`, `compact.rs`, `vaults.rs`) | Make a new wallet |
| Load or restore a wallet (Home tile, Wallets empty state and loaded card, Restore's heading, `compact.rs`, `vaults.rs`, `catalog.rs`'s entry) | Restore an existing wallet |

The sub-lines stay ("Single key or multisig", "From its backups").
"Make a wallet from this key" (`screens.rs` `FROM_KEY`) is a different
action and stays. `docs/learn/` and `docs/FLOWS.md` follow where they
name the tiles.

Tests: `tests/home.rs` asserts the two new names on Home and on the
empty Wallets tab; the tests that pressed the old names press the new.

## 2. Scan a key (batch A)

### 2.1 The wallet's key scanner

A new `ScanPurpose::WalletKey(usize)` (the wallet's place in the
session) and `Action::ScanKey(usize)`. Like `ScanSeed` it needs
`may_load_keys()`, and a stick attached goes through `Sheet::Pull` as
the other key-loading actions do (`Faraday::pull_what`, item 65.2:
"Pull the stick to add a key").

The camera sheet for it:

- Title **Scan a key for {wallet}**.
- One line per key it will take: `Key {n} · {fingerprint}`, and where
  the wallet's backup plan names where that seed is kept, ` · {place}`
  (the place names from the plan the chart draws, `glance.rs`).
- Buttons: **Type the words instead**, **Cancel**.

What a read does:

| Read | Result |
|---|---|
| A SeedQR or CompactSeedQR (or a code of BIP-39 words) whose key with no passphrase is a missing slot | The key loads into that slot at once, no passphrase sheet. If the wallet still needs keys, the camera stays open with the note `Key {n} added · {k} more`; once it has enough, the camera closes and a toast says `Key {n} added · Can spend`. |
| The same, for a key already here | Note `Key {n} · already here`; camera stays. |
| A seed whose key matches no slot | The camera closes and the passphrase sheet opens on it, with the line `{fp} is not one of {wallet}'s keys`. With a passphrase that gives a missing key it loads into that slot; one that gives no key of the wallet is refused there with the same line and the sheet stays (as `restore_refusal` does for Restore). |
| Anything else | Named as `what_instead_of_a_seed` names it; camera stays. |

The key's label: the backup plan's name for that seed when the plan has
one (Seed B), else none, as a key typed with no label. A label equal to
its fingerprint is never drawn beside the fingerprint (see §3.3).

**Type the words instead** closes the camera and opens Add a key for the
wallet: a new `EntryState::wallet: Option<usize>`. Its heading is
**Add a key for {wallet}**; Add key takes words (and passphrase) whose
key is any missing slot of that wallet, and refuses others with
`These words give {fp}, not a key of {wallet}` (a new `Refusal` case
beside `WrongKey`). `Action::Entry(Some(fp))` for one slot stays for the
callers that keep it (Restore's slots).

### 2.2 The wallet card

The signatures bar (`screens.rs` wallet card, `compact_screens.rs`'s
card):

- Short of signers: the pips and `{here} of {m} signatures possible
  here` as now; on the right, in place of "Add 1 more key or collect 1
  more signature", a primary **Scan a key** button (`Action::ScanKey`).
  Disabled as the other key buttons are when keys may not load. On the
  small panel the button goes full width under the line.
- Enough: the same line, and on the right a green **Can spend** pill
  (`ui.chip` with `OK`), in place of "Nothing missing".
- `missing_keys_line` ("Not in vault.ofv · perhaps another vault…")
  stays as it is: it says where else to look and carries Unlock.

The Spend from this wallet button and the rest of the card are
unchanged.

### 2.3 The chart's key nodes

In `glance.rs`, a key that is not here and not in an open vault, while
the wallet is short: state line **Not here** (muted), and no way-in
line (no "Add its key"). The node's own press still opens its sheet. A
key in an open vault keeps **Load from vault**. Once the wallet can
spend, the nodes are as now ("Cosigner · xpub only").

The same rows in Wallets' Sign step 4 (`screens.rs`, the `4 =>` arm
with "Add its key") and the Spend tab's Who signs follow §3.

### 2.4 The Wallets list

- The column's **Add a key** button: with the selected wallet short of
  signers, it reads **Scan a key for {wallet}** and opens §2.1;
  otherwise **Add a key** as now.
- A wallet's row: when it can spend, a green **Can spend** at the right
  of its name line, and its sub-line is the shape alone (`2 of 3 ·
  native SegWit multisig`); short, the sub-line is `{shape} · {here} of
  {m} here` as now; watch only as now. The small panel's list and the
  other places that say `Ready to sign · {here} of {m}` say `Can spend
  · {here} of {m}`.

### 2.5 Add a key leads with Scan

On Add a key (`Screen::Entry`), above the words: a full-width **Scan a
SeedQR** button, primary while no word is typed and secondary once one
is (Add key is then the primary). The foot's own "Scan a SeedQR" goes.
Other forms, Make a new key and the rest are unchanged. Small panel
alike, above its word keyboard's field.

Tests (`tests/scan_a_key.rs`, new): on the 2-of-3 wallet with Seed A
loaded,
- the card offers Scan a key and no "Add its key" anywhere; pressing it
  turns the camera on;
- Seed B's SeedQR loads Key 2 with no passphrase sheet, the camera turns
  off, and the card shows Can spend and the list row Can spend;
- Seed A's SeedQR says `Key 1 · already here` and the camera stays on;
- a seed of no key of the wallet opens the passphrase sheet with the
  `not one of` line, and its plain choice is refused there;
- on a wallet needing two more keys, the first match leaves the camera on
  with `… · 1 more`;
- Type the words instead, then Seed C's words, loads Key 3; another
  seed's words are refused with the new line;
- with a stick attached, Scan a key opens the Pull sheet;
- the Wallets column button reads `Scan a key for {wallet}` while short.

## 3. Who signs (batch A)

The Spend tab's signers card (`family_screen.rs` `signers`) and Wallets'
Sign step 4 (`screens.rs`):

### 3.1 The count

- Enough: **{here} of {needed} signatures here** and a green **Can
  spend** pill beside it, in place of "Needs 2 · 2 here".
- Short: **{here} of {needed} signatures here · {k} more needed** in
  `WARN`.

### 3.2 The rows and buttons

- A key here: **Signs here**, as now.
- A key not here: **Not here** (muted chip), no button; in an open vault,
  **Load from vault** as now.
- Under the rows, only while short (decision 11 for its exception): a
  primary **Scan a key** (`Action::ScanKey`, the wallet), the **Unlock
  {vault}** buttons, and **Collect a signature** where the step exists.
  "Scan a SeedQR", "Type its words", "Add its key" and the "Add a key
  here" heading go.
- Enough: none of those buttons; Continue as now.

### 3.3 Labels

A key's label equal to its fingerprint (a key scanned with no name) is
not drawn beside it, on these rows as on the chart.

### 3.4 The guide text

`guide.rs`'s Who signs text names the chip "Signs here", which stays;
it changes only if the wording drifts.

Tests (`tests/spend_from_vault.rs`, `tests/family.rs`): at quorum the
card shows `2 of 2 signatures here` and Can spend and no Scan a key,
Scan a SeedQR or Collect a signature; short, it shows `1 of 2
signatures here · 1 more needed`, Scan a key, and no Type its words;
Scan a key there with Seed B's SeedQR reaches Can spend; a key scanned
with no name shows its fingerprint once.

## 4. The Spend tab's intake (batch B)

### 4.1 The page

Pages 3 and 4 (`page::HOLDING`, `page::OPEN`) become one card, **What do
you have?**, at `page::HOLDING`. `page::OPEN` is no longer in the column
(`family_pages`); its constant stays so a kept `open=3` from before
maps to `HOLDING` on restore. The card is done when a wallet is chosen
(`family_ready`), and then the column goes on to Check as now. The
route (`Route`) is no longer chosen: it is set from what came in
(Vault when a vault was unlocked here, Paper when a description came
in, Words when only seeds did), so the pages after it, the kept state
and `SpendFrom` work as before. `F::Holding` goes.

**Lead** (the tab's text, `family_text.rs`): "Scan or type everything
in the envelope, one thing at a time, in any order. Faraday works out
the wallet from it."

The card, top to bottom:

1. **A vault, when one is found.** A locked vault in Files (the stick
   this computer started from brings its own in): the vault block as
   page 4's vault route draws it now (pick, Remove the stick,
   passphrase, Unlock, then the vault's list and Load), headed
   **Unlock {vault}**. Then the label **Or bring in what is on paper**
   before item 2. With no vault in Files, nothing of the vault shows.
2. **Two buttons:** primary **Scan the QR codes you have** (the intake
   scanner, §4.2), secondary **Type the words you have** (Add a key,
   returning here; `F::TypeWords`).
3. A link, **Not sure what you have?**, which shows and hides the list
   of what each thing looks like (`family_text::looks`, as `F::Unsure`
   does now).
4. **What came in:** a row each, newest last: `Seed · {fp}` (and its
   label), `Wallet description · {name} · {shape}`, `Part {k} of {n}`
   for split sheets (from `restore::merge`), `Key · {fp} · xpub` for a
   cosigner's public key, `Transaction · kept for later` for a PSBT.
5. **The next question**, by what came in (§4.3).
6. `family.error` and **Start over** as now.

### 4.2 The intake scanner

`ScanPurpose::Intake`, opened by **Scan the QR codes you have** and by
the question buttons that say Scan. Its sheet is titled **Scan what you
have**, with the hint line the state calls for (§4.3), and buttons
**Done** and, when it applies, **Type the words instead**. It reads
anything, one code after another, and stays open until Done:

| Read | Result |
|---|---|
| A SeedQR, CompactSeedQR or a code of BIP-39 words | The passphrase sheet opens on it (as everywhere, 2026-10-11), except when a description is in and the key with no passphrase is one of its missing slots: then it loads at once (decision 9). Once the sheet loads it, the camera comes back on for the next code. Note `Seed {fp} read`. |
| A seed that matches none of a description's keys | The passphrase sheet with `{fp} is not one of {wallet}'s keys`, and **Not this one**, which drops the words and returns to the camera. |
| A wallet description (any form `read_wallet` takes, whole or in UR/BBQr parts) | Into Files as now; the wallet opens (`family_use_wallet`). Note `Wallet description read`. |
| A split sheet's part | Into Files; note `Part {k} of {n} read`, or the wallet rebuilds when the parts add up (`family_rebuild`). |
| An xpub, zpub or similar public key | Into Files as a key; listed as `Key · {fp} · xpub`; it fills a cosigner box of §4.3's shape when one is open. Note `A public key: it shows the wallet but cannot sign`. |
| A SLIP-39 or codex32 share | Handed to Add a key's form for it as the `KeyPart` scan does today, returning here. |
| A PSBT | Into Files; note `A transaction to sign · kept for later`. The Bring page finds it there (`lead_psbt`); it does not start the spend before the wallet is open. |
| Anything else | Named, as the Files scanner names it. |

### 4.3 The next question

Worked out from app state each frame (not stored), except two answers
kept in `FamilyState`: `everything: bool` (That's everything) and
`no_description: bool` (I can't find one). Both clear on Start over.

| What came in | The question |
|---|---|
| Nothing | none |
| A description (whole or rebuilt) | none: the wallet opens and the column goes on to Check. Seeds that came in already are taken into its slots; one that matches no slot shows `Seed {fp} is not one of {wallet}'s keys` with **Add its passphrase** (the passphrase sheet on that seed). The other seeds are asked for on Who signs, after the transaction is read (`docs/FAMILY.md` rule 4). |
| One seed, nothing else | **Anything else in the envelope?** Hint: "Another list of words, another QR code, a line starting wsh( or tr(, or a word kept apart from the list." Buttons: **Scan more**, **Type more words**, **That's everything**. That's everything opens the single-key wallet as native SegWit (`family_words_kind`) and §5 applies. |
| Two seeds or more, no description, not `no_description` | **These seeds are one wallet with several keys.** Hint: "Its description says how they fit together: a QR code labelled wallet, descriptor, setup or multisig; a long line starting wsh( or tr(; or sheets with lines like 73C5DA0A: xpub…". Buttons: **Scan the description** (the intake scanner, that hint on its sheet), **I can't find one**. |
| Two seeds or more, `no_description` | The seeds' shape as now (`seeds_screen::shape`): M of N on the sliders, a box per missing key, the kind (native SegWit multisig first), the path. While N exceeds the seeds here, the line `Without the description Faraday needs all {n} keys: their seeds or xpubs`, and the boxes take scans (§4.2) or typing. **Make the wallet** builds it once every key is in, as now; §5's guess applies to its kind. |
| Only public keys, parts not adding up, or shares part-way | The rows say what is in; the two buttons stay. |

The single-key wallet no longer opens on the first seed by itself
(`family_key_added`): it waits for That's everything, or for a PSBT that
names one of its kinds (`family_use_psbt` keeps trying the kinds).

### 4.4 What stays

The vault route's list and Load, the stick visit, Rebuild, the seeds
shape, Restore's pieces and the passphrase sheet are reused as they
are. A wallet card's **Spend from this wallet** skips the intake as now
(`docs/FAMILY.md` §7). The Loaded overview (`family_overview`) is
unchanged.

Tests (`tests/family.rs`, rewritten where it pressed the answers):
- a vault in Files: the card offers Unlock {vault}; the test stick's
  vault and passphrase open the wallet and the column goes on to Check
  (the old vault route test, through the new card);
- no vault in Files: no Unlock on the card;
- one seed by SeedQR: the card asks Anything else in the envelope?; That's
  everything opens native SegWit with the test kit's first address;
- two seeds, then the description by scan: the wallet opens with both
  keys here and goes on to Check;
- the description first, then a seed of another wallet: the
  `is not one of` line;
- two seeds and I can't find one: the shape shows, and with all three
  seeds Make the wallet gives the test kit's 2-of-3 first address;
- split sheets scanned in any order rebuild the wallet;
- a PSBT scanned on the card is kept and used on Bring, not before;
- a lock on the card comes back on it with what came in still listed
  (keys come back by the vault or not at all, as today).

## 5. Check the money is there, when Faraday guessed (batch B)

When the wallet was guessed (the words route's single-key wallet, or
seeds built without a description), `page::CHECK` changes:

- Title **Check the money is there, on the online computer**.
- The line `Trying: {kind}, {no passphrase | with a passphrase} · {k} of
  {total}`.
- The wallet QR as now (Show the wallet as a QR code), with the line
  "Open this in the wallet on the online computer".
- The addresses as now.
- Two buttons: primary **Yes, my money shows** (the only way on; it
  marks the wallet checked as It matches does), and **No money shows**.

**No money shows** goes to the next way, in this order, each replacing
the wallet (`family_words_kind`, or the seeds' kind for multisig) and
redrawing the QR with "Open this one on the online computer too":

1. With a passphrase (single key only): the question **Is there a word
   or sentence kept apart from the list?** with **Yes, type it** (the
   passphrase sheet on that seed) and **No**, which skips to 2.
2. Taproot (single key) or Taproot multisig (several keys).
3. Nested SegWit.
4. Legacy.

The ways tried stay listed above, each `{kind} · no money` (struck, as
the chart strikes a lost place). After the last: **None of these show
money.** with "The words may belong to a wallet with several keys, or
need a different passphrase." and buttons **Scan more** and **Type a
passphrase**, which reopen the intake card. A wallet that came from a
description or a vault was not guessed: its Check page is as now, with
**Yes, my money shows** in place of It matches.

Tests (`tests/family.rs`): on the single-key route, No money shows
steps through the passphrase question, Taproot, nested SegWit and
legacy, each with its own first address; Continue is reached only by
Yes, my money shows; after legacy the None of these line shows; a
described wallet's Check has no No money shows.

## 6. GPG: sign several files (batch A)

Sign a file (`vault_screens.rs`, `app.vaults.signing`) becomes a checkbox
list (DESIGN §4 Multi choice, `Ui::multi_list` or `Ui::checkbox` rows):

- **Select all** first; then a row per Inbox file: checkbox, name, size.
  Files ending `.asc` are not listed.
- One file only: it starts ticked; several: none.
- Each row keeps **Show as QR** and **PNG** for that one file (a QR holds
  one signature); on the small panel they go on the row's second line
  as now.
- The foot: primary **Sign {n} files** (`Sign 1 file`; **Sign**,
  disabled, with none ticked), then **Back to the key**.
- Sign writes one `{name}.asc` per ticked file to the Outbox in one go,
  one toast `{n} signatures wait {for the stick}` (`medium.for_the()`),
  clears the ticks and stays on the page. A file with its `.asc` in the
  Outbox shows a green **Signed** marker on its row.

State: `vaults.sign_ticked: Vec<bool>` (or a set of Inbox indices,
cleared when the Inbox changes), `VaultAction::GpgTick(usize)`,
`GpgTickAll`, `GpgSignTicked`. `gpg_sign(k)` stays for the QR and PNG
paths.

Tests (`tests/gpg.rs`): two files ticked and Sign 2 files put two `.asc`
in the Outbox, each verifying against its own file with the key's
certificate (as the existing sign test verifies); an `.asc` in the
Inbox is not listed; with one file it is ticked and Sign 1 file signs
it; the page stays open and the signed rows show Signed.

## 7. Docs and tour

- Batch A: `docs/FLOWS.md` (Home's tiles, the wallet card, Add a key,
  Who signs, Sign a file), `docs/NEW-WALLET.md` §6 and §11 (the bar and
  the nodes; mark the changed lines "changed by SCAN-TO-SIGN §2"),
  `docs/DESIGN.md` only if a component's description changes (the
  pill is an existing chip), `docs/learn/` where it names the tiles.
  Tour: the wallet card short (Scan a key) and after a scan (Can spend),
  the key scanner sheet, Sign a file with two ticked; at 1280x800 and
  240x320.
- Batch B: `docs/FAMILY.md` §3 (the table), §4 pages 3 and 4 (replaced
  by the intake), §4 page 5 (the guessed Check), §6; `docs/FLOWS.md`'s
  Spend tab. Tour: the intake empty, after one seed (Anything else),
  after two (the description question), the guessed Check after No
  money shows; at 1280x800 and 240x320.
