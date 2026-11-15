# Check a signer: how Compare two PSBTs was folded into it

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/SIGNER-CHECK.md` on 2026-11-15, once the one
flow for two PSBTs or finished transactions (its §11) was built (F270).
What it describes as it is built is `docs/SIGNER-CHECK.md` §11; what it
says Compare two PSBTs was is `docs/history/COMPARE-LOG.md`.

## Header (from docs/SIGNER-CHECK.md)

**Kind:** Spec
**Open:** the one flow for two PSBTs (§11), Compare two PSBTs folded into Check a signer, is not built: Compare is still its own tile and screen (`docs/COMPARE.md`)
**Status:** Built but for §11; the check is `faraday_psbt::signer` and the flow is the decode area's `check.rs` and `check_screen.rs`; in Practice both scanners offer a sample pair (`docs/MODES.md` §5.3)

## 8.1 The flow's files, as first planned (from docs/SIGNER-CHECK.md)

- **The flow**, in the decode area: Decode and Check a signer both read
  a transaction from Files or a scan to show what it is. The area's
  module becomes a folder, faraday/faraday-core/src/decode/ with
  mod.rs (today's `decode.rs`, the area struct gaining a private
  `check` field), check.rs (the flow's state and logic) and
  check_screen.rs (its cards); `screens/decode.rs` moves to
  decode/screen.rs. No field is added to `Faraday` and no file gains an
  `impl Faraday` (`tools/lint-layout.counts`); the decode line in that
  list is renamed with the file. `lib.rs`'s code map names the files
  and the new `Screen::SignerCheck`.

## 11. The one flow for two PSBTs, as designed (from docs/SIGNER-CHECK.md)


Compare two PSBTs (`docs/COMPARE.md` §2) is folded into Check a signer:
one tile, one screen, one guide page, and one Result that holds the
comparison's differences, the fee bump result where it applies and, where
both files are PSBTs, the signer's report. Nothing in the check
(`core/faraday-psbt/src/signer.rs`), the comparison
(`core/faraday-psbt/src/diff.rs`) or the fee bump
(`core/faraday-psbt/src/bump.rs`) changes: the fee bump's rules and
wording are COMPARE §3, and Sign a transaction's review keeps its fee bump
section (COMPARE §4), its **Compare** opening this flow. This section
replaces COMPARE §2 and changes §3 and §4 above where it says so; until it
is built, those describe the app.

### 11.1 The cards and what each takes

The two cards keep their names, **Sent** and **Returned**. Compare's
First is Sent and its Second is Returned; between an original and its
replacement (COMPARE §3.1), Sent is the original and Returned the
replacement.

Each card takes what Compare's cards take today: a PSBT, version 0 or 2,
raw or as base64 text, or a finished transaction, raw or as hex, from
Files or a scan. Anything else is refused on the card and in the scanner
with "Not a PSBT or a transaction", as `wallet::read_psbt` and
`faraday_psbt::transaction::read_raw` refuse it.

One side holds the union of the two flows' sides today
(`decode::check::Side`, the one `Side`): the bytes, the version 0 form, the
PSBT's own version (`None` for a finished transaction), the outpoints it
spends (sorted), its unsigned transaction's txid, its name, its origin
(`check::Origin`, already shared) and `differs`. The bytes of a PSBT are
kept as Check a signer keeps them (`check::raw_bytes`: raw, base64 decoded
where the file is text), which `signer::check` reads; the bytes of a
finished transaction as Files holds them (raw or hex), which `read_raw` and
`raw_as_psbt` read, and never through `raw_bytes`, since hex text can also
decode as base64. `differs` compares with `check::same_bytes` for both.

**A finished transaction is not a returned PSBT.** The signer check runs
only where both sides are PSBTs. It cannot be run on a finished
transaction: it reads both files' own bytes for the PSBT's version, the
pairs only version 2 has and BIP-370's modifiable flags (`file_fields`,
§6.7), and a finished transaction's bytes are no PSBT, which the check
calls an unnamed field and never clean; and the PSBT form of a finished
transaction (`raw_as_psbt`) holds only the final scriptSigs and witnesses,
so every key origin, UTXO and script the Sent PSBT carries would read as
removed, and no signature in it is held to a key the sent PSBT names
(§6.2). With a finished transaction on either card the Result says "The
signer check needs two PSBTs" and shows the fee bump and the differences
only.

Nothing refused today is accepted: today's cards refuse a finished
transaction ("Not a PSBT"); the merged card takes it, as Compare's did,
and the check still never judges it, so no pair reaches a verdict the
check would not give it today. Every finding, verdict and refusal of §5
and §6 stands as it is.

### 11.2 Names and ways in

- **Screen and presses.** `Screen::SignerCheck` is the one screen;
  `Screen::Compare` goes. `Action::SignerCheck(SignerPress)` is the one
  action; `Action::Compare` and `ComparePress` go. `SignerPress` gains
  `FromReview(bool, usize)` (Compare's, as it is); `ComparePress::CheckSigner`
  goes, since the report is on the same card. `decode::check::Slot` (Sent,
  Returned) is the one slot; `compare::Slot` goes.
- **Scanners.** `ScanPurpose::CheckSent` and `ScanPurpose::CheckReturned`,
  titled "Scan the PSBT sent" and "Scan the PSBT returned", take what
  Compare's did: `crypto-psbt`, animated or not, base64 text and a finished
  transaction's hex. Practice's sample pair stays (`docs/MODES.md` §5.3).
  `ScanPurpose::CompareFirst` and `CompareSecond` go.
- **Tools.** One tile, `Go::SignerCheck`, in the Transactions group after
  **Nonce check**: "Check a signer", its line "Sent and returned PSBTs
  compared", its standards BIP-174, BIP-373 and BIP-125. `Go::Compare` and
  its tile go, and so does its row in `docs/WALLETS.md` §7. A press reopens
  a kept flow as it was; otherwise it opens on §3's default pair.
- **Files.** One entry in the ⋮ of a PSBT's row and of a finished
  transaction's row, From the stick and For the stick: **Check a signer**,
  after **Show as QR** (`docs/FLOWS.md`, A ⋮ on every file, item 1);
  **Compare** goes. It opens the flow with that file as Returned. Sent is
  then, the first of these that holds: the one other PSBT of its unsigned
  transaction with fewer signatures (§3, From Files); the one file of
  another unsigned transaction that spends an input of it, counted as
  COMPARE §4.1 counts (the original it replaces); else the Sent card is
  open.
- **Check** (DESIGN §5, Check, row 4 "What it signed"): the row opens the
  flow as the tile does, with **Check** as its way back ("Back to Check",
  F146).
- **Sign a transaction's review.** A fee bump result's **Compare** (its
  label kept) presses `SignerPress::FromReview`: the other transaction's
  file is Sent and the PSBT under review is Returned, named by its source
  as the review names it, and the flow runs.

### 11.3 The lists

Each card lists every PSBT and every finished transaction in Files, From
the stick then For the stick. A PSBT's row reads "{n} signatures · {txid
short form}" (from its `PsbtSeen`), a finished transaction's "Finished ·
{txid short form}" (its id kept on the state, as Compare keeps it). Empty:
"No PSBT or transaction in Files" and **Scan**. The card chosen second
lists first the files that spend an input of the other card's file, those
of its own unsigned transaction first, then "Other transactions" and the
rest; the file on the other card is not listed. §3's default pair reads
PSBTs only, as today.

### 11.4 When it runs

On the press that completes the second card, on **Swap**, and when a key
is loaded or dropped or a wallet registered while a result is kept (the
signer's nonce lines and which outputs are the person's depend on them),
with the token the glue mints. It always makes the comparison
(`faraday_psbt::diff::list`) and, where COMPARE §3.1 applies, the fee bump
(`faraday_psbt::bump::judge`), over each side's inspection, a finished
transaction's amounts filled as COMPARE §2.2 says; where both sides are
PSBTs, also the check and its nonce lines (§8.1). One sync,
`Faraday::signer_check_sync`, takes in `compare_sync`: it makes the result
again when the keys or wallets differ from those it was made with
(`InspectInputs`), works out `differs` and the finished transactions' ids
the lists read, and the two stale paths are one. The invariant of §8.2
holds for the whole result: it is the result for the two sides kept,
against the keys and wallets loaded. Drawing reads it; one budget test
holds the flow (`faraday/faraday-core/src/work/budget/signer.rs`).

### 11.5 The Result card

Top down; checked against Material 3's cards, lists and dividers pages:

1. **Fee bump**, only where COMPARE §3.1 applies (the txids differ and an
   outpoint is spent by both): the section label, the result line in its
   tone, its lines and its facts, as Compare shows them today
   (`BumpShown`). Then a full-width divider (Material 3 Dividers).
2. **The verdict**: where both are PSBTs, §5's Result line, unchanged;
   else "The signer check needs two PSBTs", in body text,
   on-surface-variant, with no icon and no tone.
3. **Facts**, label over value: where the check ran and the two are one
   unsigned transaction, §4's facts as they are; otherwise Compare's:
   "Transaction" (both txids, labelled "Sent" and "Returned", or the one
   txid whole), "Fee" for each, labelled the same, or "Not known", and
   "Inputs" and "Outputs" counted in both and on one side only.
4. **Findings** and **Stated**, as §4, where the check ran (Findings not
   where the two share no input, as today).
5. **Differences**, the section label, then a line per difference in body
   text, no icon and no tone, in COMPARE §2.7's order; "No differences"
   where there are none.
6. The actions: **Show as QR** (secondary, only where Returned is a PSBT),
   **Swap** (secondary), **Load {fp}** (secondary, one per key: the
   check's keys not loaded, then the fee bump's, COMPARE §3.4's U2 and
   U3, not already named), **Choose again** (ghost, opens Returned),
   **Done** (primary, back to where the flow was opened, F77).

The fee bump comes first because two files cannot say whether the second
is a replacement or a signer's return that changed the transaction: both
readings are shown, and the one that names the files original and
replacement is read first by a person who brought a replacement. The
verdict below it reads the same pair as sent and returned, and is not
clean (§6.1's findings at Blocked). Neither changes the other: the fee
bump changes no finding's level, and the findings do not change the fee
bump's result.

| The two files | Fee bump | Verdict | Facts | Differences |
|---|---|---|---|---|
| One unsigned transaction, both PSBTs, Returned signed | Not shown | §5's, such as "Only signatures were added" | §4's | The signature counts and finals (C3, C6 to C9) |
| The same bytes twice | Not shown | "1 finding", "No signature was added" (§6.2) | §4's | "No differences" |
| One unsigned transaction, a finished transaction on a card | Not shown | "The signer check needs two PSBTs" | Compare's | As Compare lists them |
| Different transactions spending an outpoint in common | COMPARE §3.4's result | §5's "{n} findings", or the line above where a card holds a finished transaction | Compare's | As Compare lists them |
| No input in common | Not shown | "Not the same transaction", or the line above | Compare's | Each input and output on one side only |

On the small panel each card is a page with the bar's tools; the Result
page scrolls.

### 11.6 Wording

The differences (COMPARE §2.5) take this flow's two names: "first"
becomes "sent", "second" becomes "returned", and "{x} only" becomes "{x}
in the sent only" or "in the returned only". The strings keep their places
in `faraday/faraday-strings/src/en.rs`; a key naming first or second is
renamed to sent or returned.

| Id | Wording |
|---|---|
| C1 | "Version: {a} sent, {b} returned" |
| C2 | "Locktime: {a} sent, {b} returned" |
| C3 | "PSBT version: {a} sent, {b} returned" |
| C4 | "Input {op}: in the sent only" / "… in the returned only" |
| C5 | "Input {op}: sequence {a} sent, {b} returned" |
| C6 | "Input {op}: spends {a} sent, {b} returned" |
| C7 | "Input {op}: amount stated in the sent only" / "… in the returned only" |
| C8 | "Input {op}: {a} signatures sent, {b} returned" |
| C9 | "Input {op}: final in the sent only" / "… in the returned only" |
| C10 | "Output to {addr}: {amount}, in the sent only" / "… in the returned only" |
| C11 | "Output to {addr}: {a} sent, {b} returned" |
| C12 | "Input {op}: spent twice in the sent" / "… in the returned" |
| C13 | "Output to {addr}: yours in the sent only" / "… in the returned only" |

The facts' "{n} first only" and "{n} second only" become "{n} in the sent
only" and "{n} in the returned only"; the facts' labels "First" and
"Second" become the cards' "Sent" and "Returned". The fee bump keeps
"original" and "replacement" (COMPARE §3.4). One string is new: "The
signer check needs two PSBTs". The strings that only Compare's tile, its
Files entry, its scanners and its Result's **Check a signer** read go.

### 11.7 Where it lives

The flow stays in the decode area, and Compare's folder is folded into it,
all in faraday/faraday-core/src/decode/: check.rs holds the one state,
side, slot and press; check_screen.rs the cards and the one Result;
check_words.rs the check's words; Compare's others.rs (the files sharing
an input) becomes check_others.rs, and its words.rs (the differences' and
the fee bump's words) diff_words.rs. The presses stay in the area's
mod.rs. No field is added to `Faraday`; the area's `compare` field goes.

### 11.8 The manifest and the guide

In `docs/crypto/actions.toml` both actions keep their ids, names, labels,
classes and own lines: `sign-check` ("Check a PSBT's signatures") drops
"Compare" from its screens, and `fee-bump` ("Compare PSBTs and the fee
bump": the comparison is still made, and the name says what is done, not
which tile) names "SignerCheck" for "Compare"; their functions follow the
code. The rows `check-signer` and `fee-bump` keep their ids.

The guide has one page, `docs/guide/signer-check.md`: Compare's fee bump
and differences sections join it under its key, and its Actions section
loses Compare's **Check a signer**.
