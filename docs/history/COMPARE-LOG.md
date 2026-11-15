# Compare two PSBTs: how it was built as a tool of its own

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/COMPARE.md` on 2026-11-15, when Compare two
PSBTs was folded into Check a signer (F270, `docs/SIGNER-CHECK.md` §11).
Each keeps its original number; the source keeps its heading and a line
naming where it went.

## Header (from docs/COMPARE.md)

**Kind:** Reference
**Status:** Built (F233)

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

## 2.1 Where it started (from docs/COMPARE.md)

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

## 2.2 The cards and where the transactions came from (from docs/COMPARE.md)

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

## 2.5 The differences as first worded (from docs/COMPARE.md)

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

## 2.7 The screen (from docs/COMPARE.md)

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

## 6.2 Compare's own tests (from docs/COMPARE.md)

- faraday-core, tests/all/compare.rs (new): the tile opens the flow;
  Files' ⋮ **Compare** on the replacement opens it with the original as
  First and the result shown; Swap; Change; a scanned PSBT and a
  scanned transaction reach their card and not Files or Spend; loading
  "abandon" after a comparison turns U2 into Only the fee is raised;
  **Check a signer** shows only for one unsigned transaction and opens
  it checked; Done returns where the flow was opened.

## 7. Decisions, as written (from docs/COMPARE.md)

F233 records the design's calls that bind later changes: Compare lists
and does not judge; the fee bump is Faraday's own rule, failing closed,
with I1 to I5, X1, P1, P2 and F1 to F3 as §3.2 has them; the result is
a statement and never a gate; it shows only where §3.1 applies and, on
the review, only where Files holds another transaction spending the same
inputs. A change to any of these rules is a decision. It is on the
checklist in `docs/KEYS.md`.

No difference from Material: both screens are DESIGN §4's components in
DESIGN §5's step flow and Spend's review.
