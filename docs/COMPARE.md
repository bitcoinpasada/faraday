# Compare two PSBTs, and the fee bump

**Kind:** Spec
**Status:** Specified (F233), not built
**Open:** none of it is built: the Compare two PSBTs tool in Tools, the fee bump result in it and on Sign a transaction's review, the two reading functions in `core/faraday-psbt`, their tests and fixtures, Practice's sample pair, the Learn section and the guide's pages

Two features, one check. **Compare two PSBTs** is a read-only tool that
lists how two PSBTs differ, without judging them. The **fee bump**
result says whether a replacement only raises the fee of an original:
the same payments to the same addresses, the extra fee taken from the
person's own money. It is shown in Compare when the two are an original
and its replacement, and on Sign a transaction's review when Files
holds another transaction spending the same inputs; nowhere else. The
decision is F233 (`docs/DECISIONS.md`).

Check a signer (`docs/SIGNER-CHECK.md`) stays the tool for a PSBT sent
to a signing device and the one that came back: one transaction, every
field. Compare is for any two, two transactions included, and reads
only what moves money and what signs it (§2.3). Decode already states
whether an input signals replace-by-fee; the fee bump does not repeat
it as a rule (§3.5).

## 1. Who needs it

A wallet on the online side builds a replacement when a transaction is
stuck: Bitcoin Core's `psbtbumpfee`, Sparrow's and Electrum's "Increase
fee". The replacement comes to the device as a new PSBT. A wallet that
is wrong or hostile can change a payment's address or amount in it,
add a payment, or drop an input the original spent, and the review of
one PSBT alone cannot tell that from the original's intent. Only a
comparison with the person's own copy of the original can.

The original must be the person's copy, not the wallet's: a wallet
that alters the replacement can alter its copy of the original too.
Files empties at power-off (`docs/FLOWS.md`, Files), so the original
comes back from a stick (the PSBT or the finished transaction this
device wrote out, kept by the person) or by the camera. Faraday cannot
tell whose copy a file is; every fee bump result names the file it was
compared against, so the person sees which copy it rests on.

## 2. Compare two PSBTs

### 2.1 Where it starts

A tile in Tools' Transactions group, after **Check a signer**
(`faraday/faraday-core/src/catalog.rs`, a new `Go::Compare` beside
`Go::SignerCheck`; `docs/WALLETS.md` §7's table gains it): "Compare two
PSBTs", its line "Two PSBTs side by side", its standards BIP-174 and
BIP-125. It opens with nothing needed first. Like Check a signer it has
no sidebar row and no Cancel, and its state, public, is kept until a
lock or power-off: the tile reopens a kept comparison as it was, with
the way back set anew.

**From Files.** A PSBT's row and a finished transaction's row, From the
stick and For the stick alike, carry **Compare** in their ⋮, after
**Check a signer** where the row has it (`docs/FLOWS.md`, A ⋮ on every
file, item 1). It opens the flow with that file as Second. Where
exactly one other transaction in Files spends an input of it (§4.1's
rule), that one is First and the comparison has run; otherwise the
First card is open, the files sharing an input listed first.

**From Sign a transaction's review.** A fee bump result there (§4) has
**Compare**, which opens the flow with that file as First and the PSBT
under review as Second, named by its source as the review names it,
and the comparison run.

### 2.2 The cards and where the transactions come from

A step flow (DESIGN §5), three cards, as Check a signer's (SIGNER-CHECK
§3, §4), with its state types and its card drawing taken as the
pattern:

1. **First**: a PSBT, or a finished transaction.
2. **Second**: the same.
3. **Result**: the facts and the differences (§2.7).

Each card takes its transaction from:

- **Files**: every PSBT and every finished transaction in Files, From
  the stick and For the stick. A row per file: its name, and on its
  line "{n} signatures · {txid short form}" for a PSBT (the line
  Check a signer reads from the file's `PsbtSeen`), "Finished · {txid
  short form}" for a finished transaction. The Second card lists first
  the files that spend an input the First one spends, then, under the
  section label "Other transactions", the rest. The file chosen on the
  other card is not listed.
- **Scan**: **Scan** on each card opens the scanner titled "Scan the
  first PSBT" or "Scan the second PSBT" (DESIGN §4.9; `crypto-psbt`,
  animated or not, base64 text, and a finished transaction's hex). What
  is read goes to that card, named "Scanned", and nowhere else: not to
  Files and not to Spend. Anything else read is refused with the
  scanner's line "Not a PSBT or a transaction".
- Bytes that are neither a PSBT, version 0 or 2, nor a finished
  transaction are refused on the card as Files refuses them
  (`wallet::read_psbt`, `faraday_psbt::transaction::read_raw`).

A finished transaction is read as Decode reads one: as a PSBT whose
inputs are final with its own scriptSigs and witnesses
(`raw_as_psbt` in `faraday/faraday-core/src/decode/mod.rs`), the amounts
it spends taken from the PSBTs of the same unsigned transaction in Files
(`Faraday::amounts_in`), and, for an input still without one, from the
other card's PSBT for the same outpoint (§3.2, rule I4).

A file taken from Files is kept as it was chosen; where Files no longer
holds it, the closed card reads its name followed by "changed or no
longer in Files", as Check a signer's does. **Swap** on the Result card
exchanges the two and compares again; **Choose again** opens the Second
card.

**When it runs.** On the press that completes the second card (a pick
or a scan), on Swap, and when a key is loaded or dropped or a wallet
registered while a result is kept (which outputs and inputs are the
person's own depends on them, §3.2): the glue mints the token, both
are inspected against the loaded keys and wallets, and the result is
kept on the state.

### 2.3 What it reads

From each side, through the inspection the review already makes
(`faraday_psbt::inspect`, against the loaded keys and registered
wallets):

- the unsigned transaction's id, version and locktime (as the review
  words a locktime: none, a block height, a time);
- the PSBT's own version, 0 or 2, read from the file's bytes as Check
  a signer reads it; a finished transaction has none;
- each input: the outpoint it spends, its sequence, the amount it spends
  where the PSBT states one, the signatures on it (partial, taproot and
  in final fields, as `verify::count_signatures` counts them), whether
  it is final, and whether it is the person's (`InputInfo::is_ours`);
- each output: its script and amount, and whether it is verified the
  person's own (`OutputInfo::is_ours`: change, or a receive address,
  of a loaded key or a registered wallet), claims a key origin under a
  key not loaded, or neither;
- the fee and the fee rate where every input's amount is known, the
  rate over the inspection's virtual size (an estimate before signing,
  exact for a final input), and the total in and out.

### 2.4 How inputs and outputs are matched

- **Inputs, by outpoint.** An outpoint spent by both is one input in
  both, wherever it stands in each. One spent by one side only is
  listed on that side. A side that spends one outpoint twice (a
  transaction that can never be valid) is a difference of its own (C12).
- **Outputs, by script.** For each script, the outputs carrying it on
  each side are taken in index order. First each output of First is
  paired with the first unpaired output of Second with the same script
  and the same amount; then the outputs left on each side with that
  script are paired in index order, as one output whose amount changed;
  the rest are on one side only. So two payments to one address, their
  amounts swapped between positions, are no difference, and a script
  that appears three times on one side and twice on the other leaves
  one output on one side only.
- **Position is not a difference.** An input or output that stands at
  another index is shown with both indexes on its row ("Output 0, 1 in
  the second") and not counted: wallets order outputs at random or by
  BIP-69, and a replacement may reorder them.

### 2.5 The differences

Each difference is one line, a plain statement. Where a line names an
input it names it by its outpoint in short form ("Input 3f2a 91c0 … 8e1d:1",
DESIGN §4.5), and where it names an output, by its address in short form
or "script {n} bytes" where it has none. Amounts are in BTC (DESIGN
§4.7).

| Id | Wording | When |
|---|---|---|
| C1 | "Version: {a} first, {b} second" | The transaction's version differs |
| C2 | "Locktime: {a} first, {b} second" | The locktime differs |
| C3 | "PSBT version: {a} first, {b} second" | Both are PSBTs, of versions 0 and 2 |
| C4 | "Input {op}: first only" / "Input {op}: second only" | The outpoint is spent on one side only |
| C5 | "Input {op}: sequence {a} first, {b} second" | A shared input's sequence differs; each as Decode words it (`decode_locktime_no_rbf` and its siblings in `faraday/faraday-strings/src/en.rs`) |
| C6 | "Input {op}: spends {a} first, {b} second" | Both state the amount a shared input spends, and they differ |
| C7 | "Input {op}: amount stated first only" / "… second only" | One side states the amount, the other none |
| C8 | "Input {op}: {a} signatures first, {b} second" | A shared input's signature count differs |
| C9 | "Input {op}: final first only" / "… second only" | Final on one side only |
| C10 | "Output to {addr}: {amount}, first only" / "… second only" | An output left unpaired (§2.4) |
| C11 | "Output to {addr}: {a} first, {b} second" | A pair whose amounts differ |
| C12 | "Input {op}: spent twice in the first" / "… in the second" | One side spends one outpoint twice |
| C13 | "Output to {addr}: yours in the first only" / "… in the second only" | A paired output is verified the person's own on one side and not on the other |

### 2.6 What it does not show

Compare reads what moves money and what signs it; everything else in a
PSBT is Check a signer's (SIGNER-CHECK §6.5 to §6.9):

- key origins, scripts (redeem, witness, taproot trees and leaves),
  hash preimages, global extended keys, MuSig2 fields, unknown and
  proprietary pairs: not compared. Where the two are one unsigned
  transaction, the Result card offers **Check a signer** instead (§2.7);
- which key made which signature, and whether a signature verifies:
  the counts only. A signature's validity is the review's and Check a
  signer's, not a difference between two files;
- the fee and fee rate are facts, not differences: they follow from
  the lines above.

### 2.7 The screen

`Screen::Compare`, a step flow (DESIGN §5) of DESIGN §4's components
only; labels, values and actions, no explanation. The ? opens the Learn
section (§6.2).

| Card | Open | Closed |
|---|---|---|
| First | Option rows (§4.2), one per file, its line as §2.2 says. Under them **Scan**, secondary. Empty: "No PSBT or transaction in Files" (§4.14, Empty state) and **Scan**. | The file's name, or "Scanned"; **Change**. |
| Second | The same rows, the files sharing an input with First first, then "Other transactions" (§4.12, Section label) and the rest; **Scan**. | As First. |
| Result | Below. | — |

The Result card, top down:

1. **Facts** (§4.11, Fact), label over value:
   - "Transaction": the txid, whole (§4.5), when the two are one
     unsigned transaction; else both, labelled "First" and "Second";
   - "Fee": "{fee} sat · {rate} sat/vB" (§4.7) for each, labelled
     "First" and "Second", or "Not known" where an input's amount is
     not;
   - "Inputs": "{n} in both", then "{n} first only", "{n} second only"
     where there are any; "Outputs" the same, counted by §2.4's pairs.
2. **Fee bump**, a section label, then the result of §3, only where
   §3.1 says it applies.
3. **Differences**, a section label, then a line per difference (§2.5)
   in body text, no icon and no tone: the transaction's (C1 to C3), the
   inputs' in the First's order and then the second's own, the outputs'
   the same. "No differences" where there are none.
4. The actions: **Swap**, **Choose again** (ghost), **Check a signer**
   (secondary, only where the two are one unsigned transaction: it
   opens Check a signer with First as Sent and Second as Returned, the
   check run), **Load {fp}** (secondary, per key §3.4's U2 or U3
   names: it opens Add a key and comes back to the result), **Done**
   (back to where the flow was opened, F77).

On the small panel each card is a page with the bar's tools (§4.1); the
Result page scrolls.

## 3. The fee bump

### 3.1 When it applies

Between an **original** and a **replacement**: in Compare, First is the
original and Second the replacement (Swap exchanges them); on the
review, the file in Files is the original and the PSBT under review the
replacement. It applies when the two are different unsigned
transactions (their txids differ) and spend at least one outpoint in
common: two transactions spending one coin cannot both be mined, so the
replacement replaces the original. Otherwise no fee bump result is
shown.

### 3.2 The rules

The result is **Only the fee is raised** when every rule below holds.
A rule that fails gives a line (§3.4). The rules read the two
inspections and the two PSBTs and nothing else; nothing is taken from
the network, which the device cannot see.

**Inputs.**

- **I1.** Every outpoint the original spends, the replacement spends.
  A replacement that leaves one out frees that coin and is another
  transaction, not a fee bump.
- **I2.** An input the replacement adds is the person's own
  (`InputInfo::is_ours` against the loaded keys and registered
  wallets): the extra fee may come from the person's coins, never from
  someone else's.
- **I3.** A shared input keeps its sequence, or neither sequence
  enables a relative lock (BIP-68: the transaction's version 2 or
  higher and the sequence's bit 31 clear). A change between
  0xfffffffd and 0xfffffffe or 0xffffffff, which turns replace-by-fee
  signalling on or off, is no line.
- **I4.** A shared input's amount is the same where both state it. An
  input of the original that states none takes the replacement's for
  the same outpoint; this is how a finished transaction, which states
  no amounts, gets its fee. A lie in a shared amount is the same lie on
  both sides, so it cannot make a change of payments look like a fee
  bump: the payments are compared by themselves (P1) and the fee's rise
  is the difference of two sums that both carry it.
- **I5.** Neither spends one outpoint twice.

**The transaction.**

- **X1.** The same version and the same locktime. A replacement that
  moves the locktime changes when it can be mined, which is not the
  fee. Bitcoin Core's `psbtbumpfee` keeps the original's locktime; the
  Core fixture (§6.1) holds Faraday to that, and if Core is found to
  move it, that is the owner's call before this rule changes.

**Outputs.** An output is **yours** where the inspection verifies it the
person's own (`OutputInfo::is_ours`: on the change chain or the receive
chain of a loaded key, or a registered wallet's own script at the index
its key origins name), or where its script is the script of an output
verified yours on the other side. Every other output is a **payment**,
an output whose key origin names a loaded key and does not verify
(`OutputKind::UnverifiedChange` under a loaded key: a spoofed change
output, `WarningKind::ChangeSpoof`) included.

- **P1.** The payments of the replacement are the payments of the
  original: the same scripts with the same amounts, as many of each,
  paired by §2.4's rule. An amount higher is a change of payment as an
  amount lower is: the result says the fee alone is raised, and paying
  a recipient more is not that. A payment cut to pay the fee (a "send
  everything" spend whose only output pays the fee) is a change of
  payment, and so said.
- **P2.** The person's own outputs may fall, rise, go or be added: the
  change the fee is taken from, a new change output where an input was
  added, change dropped below dust.

**The fee.**

- **F1.** Both fees are known: every input of each states its amount,
  after I4.
- **F2.** The replacement's fee is higher than the original's.
- **F3.** The replacement's fee rate is higher than the original's,
  each worked out as its fee times 1,000 over its virtual size, in
  whole sat per 1,000 vB, the size the inspection estimates
  (`Inspection::vsize_estimate`, exact for a final input).

With P1 and F2, the arithmetic leaves one place the extra fee can come
from: the person's own outputs and the inputs I2 lets the replacement
add. That is the "taken from change" of the claim, and no rule of its
own is needed for it.

### 3.3 Not a fee bump, or cannot tell

A rule fails in one of two ways.

- **More than the fee changed.** I1, I2 (an input added under no key
  origin, or one naming a loaded key that does not verify), I3, I4 (two
  amounts stated that differ), I5, X1, P1, F2 or F3 fails on what the
  two files state. The result is in `ERR`.
- **Cannot tell.** No rule fails on what the files state, but one
  cannot be worked out:
  - an input whose amount neither side states (F1);
  - an output, on either side, whose key origin names a key not loaded,
    so whether it is yours is not known (P1, P2): loading that key
    tells;
  - an input the replacement adds whose key origin names a key not
    loaded (I2);
  - the original carries no key origin on any output (a finished
    transaction, or a PSBT whose wallet wrote none), and one of its
    outputs is paired with nothing in the replacement: whether it was
    the person's change, dropped, or a payment, dropped, is not stated
    anywhere (P1).
  The result is in `WARN`.

Where both kinds are found, the result is More than the fee changed,
its lines first and the cannot-tell lines after them. A result of Only
the fee is raised is never shown with a line under it.

### 3.4 The results and their wording

The result line (DESIGN §4.11, Result) in its tone, then its lines
(DESIGN §4.11, Warning, in the result's tone) in the order of §3.2's
rules, then the facts.

| Result | Tone | Wording in Compare | Wording on the review |
|---|---|---|---|
| Only the fee is raised | `OK` | "Only the fee is raised" | "Only the fee is raised from {name}" |
| More than the fee changed | `ERR` | "More than the fee changed" | "More than the fee changed from {name}" |
| Cannot tell | `WARN` | "Cannot tell whether only the fee is raised" | "Cannot tell whether only the fee is raised from {name}" |

`{name}` is the original's file name. The lines, "original" and
"replacement" in both places:

| Id | Rule | Wording |
|---|---|---|
| B1 | I1 | "Input {op}: in the original, not in the replacement" |
| B2 | I2 | "Input {op}: added, and not yours" |
| B3 | I3 | "Input {op}: sequence {a} in the original, {b} in the replacement" |
| B4 | I4 | "Input {op}: spends {a} in the original, {b} in the replacement" |
| B5 | I5 | "Input {op}: spent twice in the original" / "… in the replacement" |
| B6 | X1 | "Version: {a} in the original, {b} in the replacement" |
| B7 | X1 | "Locktime: {a} in the original, {b} in the replacement" |
| B8 | P1 | "Pays {b} to {addr}, not {a}" |
| B9 | P1 | "No longer pays {a} to {addr}" |
| B10 | P1 | "Pays {b} to {addr}, which the original does not" |
| B11 | F2 | "Fee: {a} sat in the original, {b} sat in the replacement" |
| B12 | F3 | "Fee rate: {a} sat/vB in the original, {b} sat/vB in the replacement" |
| U1 | F1 | "Input {op}: amount not stated" |
| U2 | P1, P2 | "Output to {addr}: change of {fp}, which is not loaded" |
| U3 | I2 | "Input {op}: added, of {fp}, which is not loaded" |
| U4 | P1 | "Output to {addr}: in the original only, and not stated to be yours" |

The facts under the lines (DESIGN §4.11, Fact):

- "Fee": "{a} sat · {r} sat/vB", then "{b} sat · {s} sat/vB", labelled
  "Original" and "Replacement"; "Not known" for a fee F1 cannot give;
- "Inputs added": a count, only where the replacement adds any;
- "Replace-by-fee": "Signalled by the original" or "Not signalled by
  the original" (§3.5).

On the review, under them, **Compare** (text button) opens Compare on
the pair (§2.1).

### 3.5 Against BIP-125, and where the rule is Faraday's own

BIP-125 sets what a node relays as a replacement; it says nothing of
what the replacement pays. Faraday checks what it can know offline and
states the rest:

- **Rule 1, the original signals.** Not a rule here: Bitcoin Core 28
  and later replace without the signal by default (full replace-by-fee),
  and the signal moves no money. It is stated (§3.4, "Replace-by-fee").
- **Rule 2, no new unconfirmed inputs.** Not checked: whether an input
  is confirmed is the chain's, which the device cannot see.
- **Rule 3, the fee is at least the original's.** Faraday is stricter:
  higher (F2).
- **Rule 4, the rise pays for the replacement's own relay** at the
  node's incremental rate. Not checked: the rate is each node's policy.
  The two fees and the size are stated, and a wallet that bumps works
  it out.
- **Rule 5, at most 100 transactions replaced.** Not applicable: one
  original.
- **The fee rate rises.** Bitcoin Core's own replacement policy
  (`doc/policy/mempool-replacements.md`, its rule 6) wants the
  replacement's rate above the original's; Faraday requires it (F3).

Faraday's own, under STANDARDS §0 ("Faraday's own, conservative"): I1
to I5, X1, P1 and P2, which no standard defines. Each fails closed: a
case it does not know is Cannot tell, never Only the fee is raised.
The result reads amounts and scripts and makes no cryptography of its
own: it is not one of the cryptographic actions that carry F177's
label, "own specification, not yet reviewed outside Faraday". It is a
review rule like the change check, and gets a row in STANDARDS §4 (the
`docs/crypto/actions.toml` row beside `check-signer`, its
`standard_note` "Faraday's review rules", `own` naming F233), checked
against what Bitcoin Core's `psbtbumpfee` makes (§6.1).

### 3.6 A statement, not a gate

The fee bump result does not stop signing, on the review or anywhere.
A person may mean a replacement that does more than raise the fee (a
payment added to a stuck transaction, a payment cancelled by sending it
back to themselves), and the review's own checks of the replacement as
one PSBT (its warnings, its holds, F136) still stand in full. The
result says what the replacement changes from the original, as Decode
says what a transaction does.

## 4. On Sign a transaction's review

### 4.1 Which files count

While a PSBT is open on Sign a transaction (Spend's review, opened by a
scan, a file's **Sign**, or the Spend tab), a file in Files is "another
transaction spending the same inputs" when:

- it is a PSBT or a finished transaction, From the stick or For the
  stick;
- its unsigned transaction's id is not the open PSBT's (a file of the
  same unsigned transaction is a copy of it, not another);
- it spends at least one outpoint the open PSBT spends.

Files of one unsigned transaction count as one: the first PSBT among
them in Files' order (From the stick, then For the stick, each in its
list's order), else the first finished transaction, is the one compared
and named. With none, the review shows nothing of this section, and
nothing is worked out for it. With several other transactions (a
transaction bumped twice), one result each, in Files' order.

What each file spends is read once when it comes in, as its `PsbtSeen`
is (`docs/SPEED.md` §3.3): `Item` gains its outpoints, sorted, for a
PSBT and a finished transaction, through `wallet::classify_read`, which
parses the file already. Finding the files that count is then a lookup
over Files, with no file read again.

### 4.2 What the review shows

On the Transaction step (`faraday/faraday-core/src/screens/spend.rs`,
`step_body`), after the fee and before the warnings: the section label
"Fee bump", then for each other transaction its result line (§3.4, the
review's wording), its lines, its facts and **Compare**. The step's
summary line is unchanged. Nothing else on the review changes, and the
fee bump changes no other warning's level.

### 4.3 When it is worked out

Beside the spend's inspection, kept on `SpendState` with it: the
results, already in words, and what they were made from (each other
file's name and txid). `Faraday::refresh_spend` makes them again when
it inspects again (the PSBT, the network, the keys or the wallets
changed: `SpendState::inspected_for`) or when the set of files that
count (§4.1) is not the one they were made from: a file brought in or
removed while the review is open. Making them costs an inspection of
each other transaction's file (and, for a finished transaction, what
Decode's `amounts_in` costs), with the token the press's glue holds.
Drawing reads the kept lines and works nothing out.

## 5. Code

### 5.1 In core/faraday-psbt (added; what it accepts is unchanged)

Two modules, both pure, both reading inspections made by the caller, so
neither costs more than walking the two transactions:

- **core/faraday-psbt/src/diff.rs (new), `faraday_psbt::diff`.**
  `diff::list(first: Side<'_>, second: Side<'_>) -> Differences`, where
  `Side { psbt: &Psbt, inspection: &Inspection, psbt_version: Option<u8> }`
  (`None` for a finished transaction). `Differences` holds the two
  txids, the facts of §2.7 as numbers, the inputs and outputs paired by
  §2.4 (each pair with both indexes), and the differences as typed
  values, one variant per id of §2.5. It returns no text.
- **core/faraday-psbt/src/bump.rs (new), `faraday_psbt::bump`.**
  `bump::judge(original: Side<'_>, replacement: Side<'_>, loaded:
  &[Fingerprint]) -> Option<Bump>`: `None` where §3.1 says it does not
  apply; otherwise `Bump { result, lines, fees, added_inputs, signalled
  }`, `result` one of `OnlyFee`, `MoreChanged`, `CannotTell`, and the
  lines typed, one variant per id of §3.4, in the rules' order, the
  fingerprints U2 and U3 name carried for **Load {fp}**. It uses
  `diff`'s pairing, so the two never pair outputs differently.
  `bump::fill_amounts(original: &mut Psbt, from: &Psbt)` sets the
  `witness_utxo` of each original input that states no amount from the
  input of `from` spending the same outpoint (I4); the caller runs it
  before inspecting the original.
- `lib.rs` declares both modules. No existing function changes; the
  inspection already gives every value the rules read.

### 5.2 In faraday-core

- **faraday/faraday-core/src/compare/ (new), the area's folder**
  (`docs/STRUCTURE-PASS.md` §7.1): `mod.rs`, the flow's state
  (`CompareState`, the two sides as Check a signer's `Side` keeps them,
  the result, its words, what it was made with), its presses
  (`ComparePress`, as `SignerPress`) and the logic that needs no app;
  `screen.rs`, the cards; `words.rs`, every difference's, line's and
  fact's text from the typed values, in exhaustive matches, the fee
  bump's included, which the review uses too; and `others.rs`, §4.1's
  rule over Files and the making of the review's results.
- **Its state lives in the decode area,** as Check a signer's does: the
  decode area's struct gains a private `compare` field with its
  accessors, and the presses that need the app (`compare_act`, the
  sync on a key loaded or dropped) are in
  `faraday/faraday-core/src/decode/mod.rs`'s `impl Faraday`. No field
  is added to `Faraday` and no file gains an `impl Faraday`
  (`tools/lint-layout.counts`). `raw_as_psbt` and `amounts_in` there
  become `pub(crate)` for the compare area.
- **lib.rs's glue:** `mod compare;`, `Screen::Compare` and its row in
  the code map's table (`compare/screen.rs`), the `Action::Compare`
  arm sent to `decode_act` beside Check a signer's. lib.rs is over
  5,000 lines and its count only falls (F196), so the same change moves
  `Item` and its `impl` (with the new outpoints field) out of lib.rs
  into faraday/faraday-core/src/item.rs, re-exported as `crate::Item`,
  and lowers lib.rs's line in `tools/lint-layout.counts`.
- **The review:** `SpendState` (`faraday/faraday-core/src/wallet.rs`)
  gains the kept results and what they were made from; `refresh_spend`
  calls `compare::others` once (§4.3); `screens/spend.rs` draws them on
  the Transaction step.
- **Files' ⋮:** `faraday/faraday-core/src/file_menu.rs` gains
  **Compare** on a PSBT's and a finished transaction's row.
- **Tools:** `catalog.rs` gains the tile (§2.1).
- **Practice:** both cards' scanners offer a sample pair, the Core
  original and its replacement (§6.1), as Check a signer's offer
  theirs (`docs/MODES.md` §5.3, `faraday/faraday-core/src/samples/`).
- **Words:** every string in `faraday/faraday-strings/src/en.rs`.

### 5.3 Costs and drawing

- Compare costs a parse of each file and an inspection of each (both
  `inspect`, which says what it costs), and, for a finished transaction,
  what `amounts_in` costs. It runs once on a press, with the token the
  glue mints, and its result is kept with the two files it describes.
- The invariant, written on the field as Check a signer's is: the
  result is the comparison of the two files kept, against the keys and
  wallets loaded; it is replaced when either file changes and when a
  key or wallet is loaded or dropped.
- The review's results follow §4.3.
- Drawing reads the kept words and the Inbox items' `PsbtSeen`; it
  works nothing out. Budget tests hold it
  (`faraday/faraday-core/src/work/budget/`).

## 6. Tests

User-facing behaviour, against fixtures, the replacement Bitcoin Core
makes among them.

### 6.1 Fixtures

In `tools/vectors/psbt/`, listed in its README. The repository holds
no fee bump pair from a real wallet today; three come from Bitcoin
Core, the pinned release `tools/scripts/signer-core.py` already runs on
regtest (`tools/crosscheck/core_node.py`), by a new script,
tools/scripts/bump-core.py: a regtest wallet holding the "abandon …
about" key at BIP-84 is funded by mining, sends one payment with change
(`walletcreatefundedpsbt`, replace-by-fee on), signs, finalizes and
broadcasts it, and `psbtbumpfee` makes its replacement.

| Fixture | What it is | Expect |
|---|---|---|
| bump-core-original.psbt | The payment as `walletcreatefundedpsbt` made it, unsigned, with its change output's key origin | — |
| bump-core-original.tx | The same, signed and finished, as hex | — |
| bump-core-replacement.psbt | `psbtbumpfee`'s replacement of it | against either original, with "abandon" loaded: Only the fee is raised |
| bump-core-added.psbt | `psbtbumpfee` at a rate the change cannot pay, so Core adds an input of the wallet's | Only the fee is raised; "Inputs added: 1" |

The rest are written by a fixture writer in the test suite from those,
one change each, and committed as bump-*.psbt:

| Fixture | From the replacement, | Expect |
|---|---|---|
| bump-pay-less | the payment one sat less | More: B8 |
| bump-pay-more | the payment one sat more, the change one less | More: B8 |
| bump-pay-other | the payment's script another address's | More: B9 and B10 |
| bump-pay-added | a second payment, taken from change | More: B10 |
| bump-input-dropped | (from the added-input pair) the original's input left out | More: B1 |
| bump-input-foreign | an input added under no key origin | More: B2 |
| bump-locktime | the locktime one block later | More: B7 |
| bump-sequence-lock | a shared input's sequence a relative lock of 10 blocks | More: B3 |
| bump-fee-same | the change as in the original, the fee unchanged | More: B11 and B12 |
| bump-rate-lower | an input of the wallet's added for one sat more fee, so the rate falls | More: B12 |
| bump-change-unloaded | (no file: the replacement with "abandon" not loaded) | Cannot tell: U2 |
| bump-no-amount | the original with an input's UTXO removed and the replacement's too | Cannot tell: U1 |

Cases built in the test, each asserting its exact result and lines in
order: the original as the finished transaction with the replacement
dropping change (U4) and keeping it on the same script (Only the fee);
two payments to one address with their amounts swapped between
positions (Only the fee); outputs reordered (Only the fee, and no
difference in Compare); a shared input's amounts stated differently
(B4); one outpoint spent twice (B5); a spoofed change output in the
replacement under the loaded key (More: B10); the same unsigned
transaction on both sides (no result); no outpoint in common (no
result); the pair swapped (B11, B12).

### 6.2 The tests

- core/faraday-psbt/tests/all/bump.rs (new): every row of §6.1 gives
  exactly its result and lines; the Core pairs give Only the fee is
  raised with "abandon" loaded, against the PSBT original and against
  the finished one.
- core/faraday-psbt/tests/all/diff.rs (new): each difference of §2.5
  from a pair built for it, and none for a pair with its maps in another
  order or its outputs reordered; the Core pair lists C10 or C11 for the
  change and nothing for the payment.
- faraday-core, tests/all/compare.rs (new): the tile opens the flow;
  Files' ⋮ **Compare** on the replacement opens it with the original as
  First and the result shown; Swap; Change; a scanned PSBT and a
  scanned transaction reach their card and not Files or Spend; loading
  "abandon" after a comparison turns U2 into Only the fee is raised;
  **Check a signer** shows only for one unsigned transaction and opens
  it checked; Done returns where the flow was opened.
- faraday-core, the review: the Core replacement opened on Sign a
  transaction with the original in Files shows "Only the fee is raised
  from bump-core-original.psbt"; with no other transaction in Files, no
  "Fee bump" label; a file of the same unsigned transaction does not
  count; the original brought into Files while the review is open adds
  the result; bump-pay-other shows its lines and signing still goes on;
  **Compare** opens Compare on the pair.
- Budget: drawing Compare's Result and the review's Transaction step
  again, a pointer move and a scroll do no costly work; the press that
  completes the second card counts its inspections; a press on the
  review that changes nothing about the spend or Files makes no
  inspection of another file.
- Learn: docs/learn/06-verifying.md gains a section on comparing two
  PSBTs and a fee bump, the page the ? opens; the guide gains
  docs/guide/screens/compare.md and the review's line on
  `docs/guide/screens/sign.md`.

## 7. Decisions

F233 records the design's calls that bind later changes: Compare lists
and does not judge; the fee bump is Faraday's own rule, failing closed,
with I1 to I5, X1, P1, P2 and F1 to F3 as §3.2 has them; the result is
a statement and never a gate; it shows only where §3.1 applies and, on
the review, only where Files holds another transaction spending the same
inputs. A change to any of these rules is a decision. Its rule in
`tools/impact.toml` is `keys.fee-bump-rules`, a review rule; it names
this document today, and the change that adds core/faraday-psbt/src/bump.rs,
diff.rs and the compare folder adds them to its `when`.

No difference from Material: both screens are DESIGN §4's components in
DESIGN §5's step flow and Spend's review.

## 8. What comes first, and which files change

The two modules in `core/faraday-psbt` come first: everything else reads
their types. They change core/faraday-psbt/src/diff.rs and bump.rs (new),
`core/faraday-psbt/src/lib.rs`, and the tests diff.rs and bump.rs under
`core/faraday-psbt/tests/all/` with `core/faraday-psbt/tests/all/main.rs`.
The Core fixtures are separate and can be made beside them: the script
tools/scripts/bump-core.py (new), the bump-core-* files and
`tools/vectors/psbt/README.md`; the written fixtures and the Core pair's
test follow both.

Then the shared part of faraday-core, before the two screens: `Item`
moved to faraday/faraday-core/src/item.rs with its outpoints
(`faraday/faraday-core/src/lib.rs`, `faraday/faraday-core/src/wallet.rs`'s
`classify_read`, `tools/lint-layout.counts`), the compare folder's
`mod.rs` shell, `words.rs` and `others.rs`, `raw_as_psbt` and
`amounts_in` made `pub(crate)` in `faraday/faraday-core/src/decode/mod.rs`,
and the fee bump's strings in `faraday/faraday-strings/src/en.rs`.

After it, two pieces in parallel. The Compare flow: compare/mod.rs's
flow and compare/screen.rs, `faraday/faraday-core/src/decode/mod.rs`
(the field, accessors and glue), `faraday/faraday-core/src/lib.rs`
(the screen, the dispatch arm, the code map), `faraday/faraday-core/src/action.rs`,
`faraday/faraday-core/src/catalog.rs`, `faraday/faraday-core/src/file_menu.rs`,
the samples, Compare's strings in `en.rs`, and its faraday-core tests. The
review: `faraday/faraday-core/src/wallet.rs` (`SpendState`,
`refresh_spend`), `faraday/faraday-core/src/screens/spend.rs`, the
review's strings in `en.rs`, its tests and the budget test. Both add
to `en.rs`, each in its own group, and the review's **Compare** button
needs the flow's press, so the review lands second or adds that one line
after the flow lands. Neither touches the other's files otherwise.

Last, the documents: the Learn section (`docs/learn/06-verifying.md`
and through `tools/learn/sync.py` the Learn pages' `en.rs`), the guide's
pages, `docs/FLOWS.md`'s Files ⋮ list and Spend's review, `docs/WALLETS.md`
§7's Transactions row, the STANDARDS row (`docs/crypto/actions.toml`),
and this document made a Reference, what is only about building it moved
to docs/history/COMPARE-LOG.md.
