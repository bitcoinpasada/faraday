# Check a signer

**Kind:** Spec
**Status:** Planned, not built; to be built after the structure pass (`docs/STRUCTURE-PASS.md` §7)
**Open:** all of it, batches S1 to S5 (§10.2); the owner's choices in §10.4 are decided (2026-10-23)

A Tools flow. The person picks the PSBT they sent to a signing device
and the PSBT that came back, and Faraday states whether the device did
only what it was asked: added signatures, or finalized an input from
signatures that verify. Anything else is a finding. The decision is
F84 (`docs/DECISIONS.md`); nonces chosen by a signer that holds no
seed here are `docs/ANTI-EXFIL.md`, which this flow comes before.

## 1. The rule

The verdict fails closed.

1. **Clean** means: the unsigned transaction is the same, and every
   difference between the two PSBTs is one of these:
   - a signature added under a key the sent PSBT names for that input
     (a key origin on it, §6.2), that verifies over this transaction,
     under the sighash the input asks for;
   - an input finalized, where Faraday's own finalizer, given the sent
     PSBT and the signatures the final fields carry, writes the same
     final fields byte for byte, and those signatures pass the line
     above (§6.3);
   - the fields a finalizer clears, cleared on an input it finalized;
   - a MuSig2 public nonce or partial signature (BIP-373) added for a
     participant the sent PSBT names (§6.4).
2. **Anything else is a finding**: a field added, changed or removed, an
   unknown or proprietary pair that appeared or changed, a signature
   removed or changed, bytes in the file its fields do not account for.
3. **A signature not checked is never clean.** Where a signature cannot
   be checked (no previous output, a script this crate does not take
   apart, a script that names no key), that is a finding.
4. **A signature under a key the sent PSBT did not name is a finding**,
   whether or not it verifies.
5. A nonce that cannot be recomputed here, because the key's seed is
   not loaded, is not a finding; the screen states how many were
   recomputed (§4, the "Nonces recomputed" fact). A clean verdict does
   not require the seeds loaded (§10.4 item 2).

## 2. What is built today

Every check is in `core/faraday-psbt`; nothing in faraday-core calls
the comparison.

- `faraday_psbt::verify::compare` (`core/faraday-psbt/src/verify.rs`):
  the unsigned transaction's fields, the signing state (which keys
  signed which input, what is finalized), and the other pairs compared
  on what they decode to. Gaps the check needs filled (batch S1):
  - unknown and proprietary pairs are compared as one string per map,
    so a finding cannot name the pair; global and output proprietary
    pairs are not compared at all;
  - a signature is compared by its key only, so the same key with other
    signature bytes is no difference; `tap_key_sig` likewise;
  - final `scriptSig` and witness are not compared when both are final;
  - not compared: an output's taproot key origins and tap tree, an
    input's hash preimages (`ripemd160`, `sha256`, `hash160`,
    `hash256`), the PSBT's global version;
  - BIP-373's MuSig2 pairs live in `unknown` (`core/faraday-psbt/src/musig.rs`)
    and are compared as unknown.
- `verify_signatures`, `repeated_nonces`, `deterministic` in the same
  file; `deterministic` reaches a seed only through faraday-keys'
  `recompute_signatures` (`faraday/faraday-keys/src/session.rs`), the
  Spend tab's nonce check's path (`Session::nonce_check`,
  `faraday/faraday-core/src/wallet.rs`).
- The inspection (`core/faraday-psbt/src/inspect.rs`): change verified
  against the loaded wallets, and the signature cards
  (`core/faraday-psbt/src/inspect/sighash.rs`): invalid, unchecked, and
  a sighash other than the one the input asks for.
- Finalization (`core/faraday-psbt/src/finalize.rs`): single key, plain
  multisig, and miniscript and taproot trees through `miniscript`.

## 3. The flow

**Where it starts.** A tile in Tools' Transactions group, after **Nonce
check**: "Check a signer", its line "Sent and returned PSBTs compared",
its standards BIP-174 and BIP-373. It opens with nothing needed first:
no key, no wallet. It has no sidebar row and no Cancel: it keeps
nothing a person would lose (`docs/FLOWS.md`, Flows in progress, item
4). Its state, public, is kept until a lock or power-off, as Decode's
is.

**From Files.** A PSBT row's ⋮ in Files, From the stick and For the
stick alike, carries **Check a signer** (`docs/FLOWS.md`, A ⋮ on every
file, item 1: after **Show as QR**). It opens the flow with that file as
Returned, its card closed on it with **Change**. Where exactly one other
PSBT in Files shares its unsigned transaction and carries fewer
signatures, that one is Sent and the check has run; otherwise the Sent
card is open, the file's transaction first.

**Steps.** A step flow (DESIGN §5), three cards:

1. **Sent**: the PSBT that went to the device.
2. **Returned**: the PSBT that came back.
3. **Result**: the verdict and the findings.

**Where the PSBTs come from.**

- **Files**: every PSBT in Files, From the stick and For the stick
  alike (a PSBT this device signed and wrote out is a fair Sent). A row
  per file: its name, and on its line "{n} signatures · {txid short
  form}" (DESIGN §4.5). What the line reads comes from the file's
  `PsbtSeen`, kept on the Inbox item since it came in: a lookup.
- **Scan**: a secondary button on each of the two cards opens the
  scanner titled "Scan the PSBT sent" or "Scan the PSBT returned"
  (DESIGN §4.9; `crypto-psbt`, animated or not, and base64 text). A PSBT
  read there goes to that card, named "Scanned", and nowhere else: it
  is not put in Files and does not open Spend. Anything else read is
  refused with the scanner's own line, "Not a PSBT".
- Bytes that do not parse as a PSBT, version 0 or version 2, are
  refused on the card as Files refuses them (`wallet::read_psbt`).

**Which is which.** The person says. One default: where exactly two
PSBTs in Files share an unsigned transaction and one carries fewer
signatures, that one is Sent and the other Returned, both cards closed
on their choice with **Change** (DESIGN §4.14, Default), and the check
has run. Otherwise both cards are open, Sent first.

The Returned card lists first the PSBTs of the Sent one's transaction,
then under a section label "Other transactions" the rest. The file
chosen as Sent is not listed. Two files with the same bytes are listed
both; picking them gives "No signature was added" (§6.2, S9).

**Swap** on the Result card swaps the two and checks again.

**Different transactions.** The check compares the unsigned
transactions first.

- No input in common: the verdict is **Not the same transaction**, with
  the two txids as facts and no other finding. **Choose again** opens
  the Returned card.
- Some input in common and any field of the unsigned transaction
  different: each difference is a finding at Blocked (§6.1), and the
  rest of the check still runs, so the person sees what else changed.

**When it runs.** On the press that completes the second card (a
choice or a scan), or on Swap, the glue mints the token and the check
runs once; its report is kept on the state (§8.2). It runs again when a
key is loaded or dropped while a report is kept, since the nonce lines
depend on it.

**Where it leaves the person.** The Result card's actions: **Show as
QR** (the Returned PSBT, public, the QR sheet), **Swap**, **Choose
again** (ghost), **Done** (back to where the flow was opened, F77).
Where the seed of a key that signed is not loaded: **Load {fp}**,
secondary, which opens Add a key and comes back to the result.

## 4. The screen

`Screen::SignerCheck`, a step flow (DESIGN §5), built from DESIGN §4's
components only. Labels, values and actions; no explanation on the
working screen. The ? opens the Learn section (§10.3).

| Card | Open | Closed |
|---|---|---|
| Sent | Option rows (§4.2), one per PSBT: the file's name; "{n} signatures · {txid}" as its line. Under them **Scan**, secondary. Empty: "No PSBT in Files" (§4.14, Empty state) and **Scan**. | The file's name, or "Scanned"; **Change**. |
| Returned | The same rows, the Sent one's transaction first, then "Other transactions" (§4.12, Section label) and the rest; **Scan**. | As Sent. |
| Result | Below. | — |

The Result card, top down:

1. **The verdict**, a Result line (§4.11) in its tone: §5.
2. **Facts** (§4.11, Fact), label over value:
   - "Transaction": the txid, whole (§4.5) when the same, and both when
     not, labelled "Sent" and "Returned";
   - "Signatures added": a count;
   - "Inputs finalized": a count, only where one was;
   - "Nonces recomputed": "{n} of {m}", or "None: {fp} not loaded",
     one line per key not loaded;
   - "Sends", "Fee", "Change": the inspection's values for the Returned
     PSBT against the loaded wallets (§4.7), "Change": "verified" or
     "not verified".
3. **Findings**, a section label, then a Warning line (§4.11) per
   finding, ordered Blocked, Danger, Caution, then by where (the
   transaction, the inputs in order, the outputs in order). Each line
   leads with its level's word in its tone, as the Spend review's
   warnings do. "No findings" where there are none.
4. **Stated**, a section label, then the Info lines (§6.8), where there
   are any.
5. The actions (§3).

On the small panel each card is a page with the bar's tools (§4.1);
the Result page scrolls.

## 5. The verdict

| Verdict | Tone | When |
|---|---|---|
| "Only signatures were added" | `OK` | No finding, no input finalized |
| "Only signatures were added; {n} inputs finalized from them" | `OK` | No finding, an input finalized ("1 input finalized") |
| "Only nonces and signatures were added" | `OK` | No finding, a MuSig2 nonce among what was added |
| "{n} findings" | `ERR` | A finding at Danger or Blocked ("1 finding") |
| "{n} findings" | `WARN` | Findings at Caution only |
| "Not the same transaction" | `ERR` | No input in common |

A clean verdict is never shown with a finding under it, and a finding
is never Info.

## 6. The findings

Every finding names where it is: "Transaction", "Input {i}" or
"Output {i}", counted from 0 as the Spend review counts. A key is named
as the review names one (`SignaturePresent::name`: the fingerprint
where an origin gives one, else the first eight hex characters).
Amounts are in BTC (§4.7), scripts as their address in short form
(§4.5) or "script {n} bytes" where they have none.

Levels, in the check: **Blocked**, the transaction is not the one sent,
or a signature gives the key away or commits to no output; **Danger**,
something added or changed beyond signatures; **Caution**, something
removed, or not checked.

### 6.1 The unsigned transaction

| Id | Level | Wording |
|---|---|---|
| T1 | Blocked | "Transaction: version {a} sent, {b} returned" |
| T2 | Blocked | "Transaction: locktime {a} sent, {b} returned" |
| T3 | Blocked | "Transaction: {a} inputs sent, {b} returned" |
| T4 | Blocked | "Transaction: {a} outputs sent, {b} returned" |
| T5 | Blocked | "Input {i}: spends {b}, not {a}" |
| T6 | Blocked | "Input {i}: sequence {a} sent, {b} returned" |
| T7 | Blocked | "Output {i}: {b} returned, {a} sent" |
| T8 | Blocked | "Output {i}: pays {b}, not {a}" |

### 6.2 Signatures

"Added" is a signature on the Returned PSBT that is not on the Sent one,
in a partial-signature field, `tap_key_sig`, `tap_script_sigs` or read
out of a final field the device wrote (`verify_signatures`). A key the
sent PSBT **names** for an input is one in that input's
`bip32_derivation` or `tap_key_origins`; for a taproot key path, the
output key of the `tap_internal_key` it names with an origin.

| Id | Level | Wording | When |
|---|---|---|---|
| S1 | Blocked | "Input {i}: the signature under {key} is not a signature of this transaction" | Added, `Verdict::Invalid` |
| S2 | Caution | "Input {i}: the signature under {key} was not checked: {reason}" | Added, `Verdict::Unchecked` |
| S3 | Danger | "Input {i}: signature under {key}, a key the sent PSBT does not name" | Added under a key not named |
| S4 | Danger | "Input {i}: the signature under {key} signs with {kind}, not the {asked} asked for" | The sighash card's rule (`inspect/sighash.rs`) |
| S4n | Blocked | the same, with ": outputs are not committed to" | as S4, a `NONE` sighash the input does not ask for |
| S4m | Caution | "Input {i}: the signature under {key} signs with {kind}, which commits to more than the {asked} asked for" | as S4, a sighash committing to everything asked and more: every input over `ANYONECANPAY`, `ALL` over `SINGLE` over `NONE` (a `SINGLE` with no output at its index commits to none) |
| S5 | Blocked | "{key}: one nonce on inputs {list}" | `repeated_nonces` over the Returned PSBT, sent signatures included |
| S6 | Danger | "Input {i}: the signature under {fp} was not made by RFC 6979 or BIP-340" | The key's seed is loaded and `deterministic` finds no rule |
| S7 | Caution | "Input {i}: the signature under {key} was removed" | On Sent, not on Returned (in no field and not in a final field) |
| S8 | Danger | "Input {i}: the signature under {key} changed" | On both, other bytes |
| S9 | Caution | "No signature was added" | No signature or MuSig2 nonce added on any input, and no input finalized |

### 6.3 Finalization

An input the device finalized (final fields on Returned, none on Sent):

- Its signatures are read out of the final fields and held to §6.2.
- Faraday finalizes a copy of the Sent PSBT's input with exactly those
  signatures as partial signatures (`finalize.rs`); the result's
  `final_script_sig` and `final_script_witness` must equal the
  device's byte for byte. That covers the scripts, a control block, a
  multisig's dummy item, and any extra item a device could add to the
  witness.
- The fields Faraday's own finalizer clears (BIP-174's list) are
  expected to be gone from that input; their absence is no finding.

| Id | Level | Wording | When |
|---|---|---|---|
| F1 | Danger | "Input {i}: finalized; the final fields differ from what its signatures give" | Faraday's final fields differ |
| F2 | Caution | "Input {i}: finalized; Faraday cannot finalize this input to compare" | Faraday's finalizer refuses it |
| F3 | Danger | "Input {i}: the final fields changed" | Final on both, bytes differ |
| F4 | Caution | "Input {i}: the final fields were removed" | Final on Sent, not on Returned |

### 6.4 MuSig2

BIP-373's pairs (`musig.rs`) are known to the check, not unknown.

- A public nonce added for a participant the input's participants field
  on the Sent PSBT names, where Sent carries none from it: no finding,
  and an Info line (§6.8). A nonce is random by design and cannot be
  checked.
- A partial signature added for such a participant: checked with
  `partial_sig_verify` (`core/faraday-bip/src/musig.rs`) against the
  nonces and participants on the Returned PSBT. One that fails is S1's
  wording, Blocked; one that cannot be checked (a nonce missing) is S2.
- The aggregate as `tap_key_sig` is a Schnorr signature and §6.2
  holds.
- Anything else in those pairs (a participants field changed, a second
  nonce from one participant, a nonce changed) is a field finding
  (§6.5).

### 6.5 Other fields

Every pair the comparison decodes, other than signatures, finalization
and BIP-373's, at every level: global, each input, each output.

| Id | Level | Wording |
|---|---|---|
| M1 | Danger | "{where}: {field} added" |
| M2 | Danger | "{where}: {field} changed" |
| M3 | Caution | "{where}: {field} removed" |

`{field}` is `Field`'s name (`verify.rs`): "key origins", "spent
output", "previous transaction", "witness script", "extended public
keys" and the rest, with the new ones S1 adds: "taproot tree",
"preimages", "PSBT version".

### 6.6 Unknown and proprietary pairs

A device can carry data out in a pair the coordinator copies on without
reading. Each pair that appeared or changed is a finding of its own,
named by its key type and sizes; nothing of its content is shown.

| Id | Level | Wording |
|---|---|---|
| P1 | Danger | "{where}: unknown field added · type {0xNN} · key {k} bytes · value {v} bytes" |
| P2 | Danger | "{where}: unknown field changed · type {0xNN} · value {a} → {b} bytes" |
| P3 | Danger | "{where}: proprietary field added · prefix {p} bytes · subtype {s} · value {v} bytes" |
| P4 | Danger | "{where}: proprietary field changed · prefix {p} bytes · subtype {s} · value {a} → {b} bytes" |
| P5 | Caution | "{where}: unknown field removed · type {0xNN}" |
| P6 | Caution | "{where}: proprietary field removed · prefix {p} bytes · subtype {s}" |

A proprietary prefix is the device's own text and is not shown: its
length is. Faraday's own `osk` records (blocked on input, F127)
never leave in a PSBT, so one on a Returned PSBT is P3 like any other.
Anti-exfil's pairs, when built (`docs/ANTI-EXFIL.md` §7), become known
pairs with checks of their own.

### 6.7 The file's bytes

The Returned file, base64 decoded where it is text, is longer than its
fields written out again: bytes after the last map, or a length written
in more bytes than it needs. A map written in another order is the same
length, and is no finding.

| Id | Level | Wording |
|---|---|---|
| E1 | Danger | "Returned file: {n} bytes its fields do not account for" |

Where the parser refuses such a file, it is refused on the card (§3),
which is no clean verdict either.

### 6.8 Stated, not findings (Info)

| Wording | When |
|---|---|
| "Input {i}: public nonce added for {key}" | §6.4 |
| "Input {i}: finalized" | An input finalized with no finding on it |

## 7. Unknown fields in a single PSBT's review

The Spend review and every other inspection of one PSBT state the pairs
this crate does not read: one warning, Info, with the count across the
whole PSBT.

- `WarningKind::UnknownFields { count }`, `Level::Info`, worded "{n}
  unknown or proprietary fields: not read" ("1 unknown or proprietary
  field: not read").
- Counted: every pair in a global, input or output `unknown` or
  `proprietary` map, less BIP-373's MuSig2 pairs (`musig.rs`) and
  Faraday's `osk` records (`threshold.rs`), which this crate reads.
- Info does not gate signing, and the Nonce check's headline is
  unchanged.

## 8. Code

### 8.1 Where it lives

- **The check**, in core/faraday-psbt/src/signer.rs (new),
  `faraday_psbt::signer`: `check(c, sent, returned, sent_bytes,
  returned_bytes, ctx) -> Report`, pure, with no key. `Report` holds the
  verdict, the facts, the findings as typed values (`Finding { level,
  at, kind }`, one `kind` per id in §6) and the Info lines. It calls
  `compare`, `verify_signatures`, `repeated_nonces`, the sighash rule,
  `finalize` on a copy, `partial_sig_verify`, and the inspection for
  the facts. Its first doc line says what it costs (CLAUDE.md).
- **The nonce lines**, in faraday-core's glue: `recompute_signatures`
  through the session's keys (no new reveal, `docs/KEYS.md`), turned
  into S6 and the "Nonces recomputed" fact.
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
- **Words**: every finding's and fact's wording in
  `faraday/faraday-strings/src/en.rs`; faraday-core fills them from the
  typed findings. The core crate returns no text for the check.

### 8.2 Costs and drawing

- The check costs a parse of both files, a signature verification per
  signature, a finalization per finalized input, and an inspection: it
  runs once, on a press, with the token the glue mints, and `Report` is
  kept on the flow's state beside the two PSBTs it describes.
- The invariant, written on the field: the report is the check of the
  two PSBTs kept, against the keys loaded. It is replaced whenever
  either PSBT changes (a pick, a scan, Swap) and whenever a key is
  loaded or dropped. Tested by loading a key after a check (§9.2).
- Drawing reads the report and the Inbox items' `PsbtSeen`; it works
  nothing out. A budget test holds it (`faraday/faraday-core/src/work/budget/`).

## 9. Tests

User-facing behaviour, against fixtures, and against another
implementation where one exists.

### 9.1 Fixtures

In `tools/vectors/psbt/`, listed in its README. Existing pairs used as
they are:

| Sent | Returned | Expect |
|---|---|---|
| `wallet-musig-core-first.psbt` | `wallet-musig-core-first-signed.psbt` | clean: a nonce and a partial signature added (§6.4) |
| `wallet-threshold-first.psbt` | `wallet-threshold-signed.psbt` | clean, input finalized |
| `wallet-tapmulti-first.psbt` | `wallet-tapmulti-signed.psbt` | clean, two script-path signatures |
| `demo-regtest.psbt` | `wallet-2of3.psbt` | Not the same transaction |

New, named signer-*.psbt, each a Sent and a Returned or a Returned
against an existing Sent. Two come from Bitcoin Core 31.1
(`walletprocesspsbt`, as the tapmulti and MuSig2 fixtures did); the
rest are written by a fixture writer in the test suite from those, one
change each, and committed:

| Fixture | From | Expect |
|---|---|---|
| core-signed | `wallet-2of3.psbt` signed by Core holding the "zoo" key | clean |
| core-final | `demo-regtest.psbt` signed and finalized by Core | clean, 1 input finalized |
| output-changed | core-signed, output 0 one sat less | T7, verdict "{n} findings" |
| invalid | core-signed, a signature byte flipped | S1 |
| foreign-key | core-signed plus a valid signature under a key no origin names | S3 |
| sighash-none | `demo-regtest.psbt` signed with `NONE` | S4n |
| sighash-single | the same with `SINGLE` | S4 |
| nonce-reuse | two inputs signed under one key with one nonce | S5 |
| random-nonce | signed by the "abandon" key with a random nonce | S6 with "abandon" loaded; clean with "Nonces recomputed: None" without |
| sig-removed | `wallet-tapmulti-signed.psbt` less one signature, against itself as Sent | S7 |
| sig-changed | core-signed, the signature remade with another valid nonce | S8 |
| final-extra | core-final with an extra witness item | F1 |
| origin-removed | core-signed less an input's key origins | M3 |
| xpub-added | core-signed plus a global xpub | M1 |
| unknown-added | core-signed plus a global unknown pair | P1 |
| unknown-sent, unknown-changed | an unknown pair on Sent, its value two bytes longer on Returned | P2 |
| proprietary-added | core-signed plus an input proprietary pair | P3 |
| osk-added | core-signed plus an `osk` record | P3 |
| trailing | core-signed with eight bytes after the last map | E1, or refused at read |

### 9.2 The tests

- core/faraday-psbt/tests/all/signer.rs (new): every row of §9.1 gives
  its verdict and exactly its findings, at their levels; Swap of a clean
  pair gives S7 per signature and no clean verdict; a pair written with
  its maps in another order is clean; the two Core fixtures are clean.
- `core/faraday-psbt/tests/all/inspect.rs`: proprietary-added and
  unknown-added give one Info `UnknownFields` with count 1;
  `demo-regtest.psbt` gives none; the MuSig2 and threshold fixtures give
  none.
- faraday-core, tests/all: the tile opens the flow; with two PSBTs of
  one transaction in Files the cards open closed on the default and the
  result reads "Only signatures were added"; Swap; Change; a scanned
  PSBT reaches its card and not Files or Spend; a Returned with a P3
  shows the line with its sizes; loading "abandon" after a check of
  random-nonce turns its result to S6; Done returns where the flow was
  opened.
- Budget: drawing the Result card again, a pointer move and a scroll
  do no costly work; the press that completes the second card counts
  one check.

## 10. Plan

### 10.1 Rules for the implementing agent

CLAUDE.md holds. In particular: a finding's wording is in `en.rs` only;
the check fails closed, so a case this spec does not name is a finding
at Danger, never clean, and is raised with the owner; the check touches
no key material outside `recompute_signatures`; the structure ratchets
only fall.

### 10.2 Batches

| Batch | What | Model |
|---|---|---|
| S1 | `compare`'s gaps (§2): pair-by-pair diff of unknown and proprietary pairs with key type and sizes at every level; signature bytes and final fields compared; the fields not compared added; BIP-373 pairs read as known. Its tests in `core/faraday-psbt/tests/all/verify.rs` | Opus |
| S2 | `faraday_psbt::signer`: §1, §5, §6, the byte count of §6.7; the fixtures of §9.1 and the README rows; §9.2's core tests | Opus |
| S3 | §7: `UnknownFields` in the inspection, its string, its test | Sonnet |
| S4 | The flow: the decode area's folder, `Screen::SignerCheck`, the tile, Files' ⋮ entry (§3), the cards, the strings, the nonce lines, the Learn section, faraday-core's and the budget tests; then the docs of §10.3, and this file's plan moved to docs/history/SIGNER-CHECK-LOG.md (new) and its Kind to Reference | Opus for the flow, Sonnet for the docs |
| S5 | Practice: a sample pair, one clean and one with a proprietary pair (§10.4 item 4) | Sonnet |

### 10.3 Edit list

| File | Change |
|---|---|
| `core/faraday-psbt/src/verify.rs` | S1 |
| core/faraday-psbt/src/signer.rs (new), `core/faraday-psbt/src/lib.rs` | S2 |
| `core/faraday-psbt/src/inspect.rs` | S3: `WarningKind::UnknownFields` |
| `core/faraday-psbt/README.md` | S2: the module's line |
| `tools/vectors/psbt/README.md` | S2: the fixtures |
| `faraday/faraday-core/src/decode.rs`, `faraday/faraday-core/src/screens/decode.rs` | S4: into the decode/ folder (§8.1) |
| `faraday/faraday-core/src/lib.rs` | S4: `Screen::SignerCheck`, the code map |
| `faraday/faraday-core/src/catalog.rs` | S4: the tile, `Go::SignerCheck` |
| `faraday/faraday-core/src/file_menu.rs`, `faraday/faraday-core/src/screens/files.rs` | S4: **Check a signer** on a PSBT row's ⋮ (§3) |
| `faraday/faraday-strings/src/en.rs` | S3, S4 |
| `tools/lint-layout.counts` | S4: the decode line renamed |
| `docs/learn/06-verifying.md` | S4: a section on checking a signer, which the ? opens |
| `docs/FLOWS.md` | S4: a section "Check a signer" (the text is in the report that brought this spec); A ⋮ on every file, item 1, gains **Check a signer** after **Show as QR** |
| `docs/DESIGN.md` | S4: §5's table, a row for `SignerCheck`, and the count of `Screen`'s variants |
| `docs/README.md` | S4: this file under Spec now, under Reference when built; a By-question row |
| `docs/ANTI-EXFIL.md` | S4: the sentence on the checks that need no new protocol names this file |

### 10.4 Questions for the owner

Decided (owner, 2026-10-23), each as recommended:

1. **Levels.** Decided: a field removed (M3), a signature removed (S7)
   and a signature not checked (S2) stay Caution; added or changed is
   Danger. All fail the verdict.
2. **Nonces not recomputed.** Decided: a clean verdict does not require
   every added signature's nonce recomputed here; the "Nonces
   recomputed" fact states the count (§1 item 5, §4).
3. **MuSig2 nonces.** Decided: a nonce added is Info (§6.4, §6.8).
4. **Practice.** Decided: yes, a sample pair in Practice, one clean and one
   with a proprietary pair; batch S5 is in the plan (§10.2).
5. **A finished transaction as Returned.** Decided: not now. A
   Returned file that is a transaction, not a PSBT, is refused on the
   card (§3).
6. **Files' ⋮.** Decided: yes, **Check a signer** on a PSBT row's ⋮,
   opening the flow with that file as Returned (§3, batch S4, §10.3).
7. **The decode area as a folder.** Decided: the flow is in the decode
   area, made a folder (§8.1); no field is added to `Faraday`.
