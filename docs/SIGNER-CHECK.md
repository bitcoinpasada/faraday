# Check a signer

**Kind:** Reference
**Status:** Built; the check is `faraday_psbt::signer` and the flow is the decode area's `check.rs` and `check_screen.rs`; in Practice both scanners offer a sample pair (`docs/MODES.md` §5.3)

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
     under the sighash the input asks for, and for a script path over a
     leaf the sent PSBT holds;
   - an input finalized, where Faraday's own finalizer, given the sent
     PSBT and the signatures the final fields carry, writes the same
     final fields byte for byte, and those signatures pass the line
     above (§6.3);
   - the fields a finalizer clears, cleared on an input it finalized;
   - a MuSig2 public nonce or partial signature (BIP-373) added for a
     participant the sent PSBT names (§6.4);
   - a field added whose whole content is checked against the unchanged
     transaction or the sent PSBT, stated and not a finding (§6.9, F199).
2. **Anything else is a finding**: a field added (other than one §6.9
   states), changed or removed, an
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
   not require the seeds loaded (`docs/history/SIGNER-CHECK-LOG.md` §10.4 item 2).

## 2. What is built today

Every check is in `core/faraday-psbt`; faraday-core's flow (§8.1) calls
`signer::check`.

- `faraday_psbt::verify::compare` (`core/faraday-psbt/src/verify.rs`):
  the unsigned transaction's fields, the signing state (which keys
  signed which input, what is finalized), and the other pairs compared
  on what they decode to, including an input's hash preimages, an
  output's taproot tree and taproot key origins; every field of `bitcoin`'s
  `Psbt`, `Input` and `Output` is named in `metadata_diff`'s patterns, so a
  field a later `bitcoin` adds fails to compile there until it is read. The
  check reads unknown and proprietary pairs (pair by pair, by key type and
  sizes), signature bytes, final fields and BIP-373's MuSig2 pairs itself,
  in `signer.rs`. It reads both files as the person has them (`sent_bytes`,
  `returned_bytes`), which the version 0 forms do not show: the PSBT's
  version, the pairs only version 2 has, BIP-370's modifiable flags, and
  the returned file's length (§6.5, §6.7). Scripts and transactions are
  compared on their serialized bytes, never on `Debug`.
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
is: the tile reopens a kept flow as it was, its report included, with the
way back set anew. Files' ⋮ **Check a signer** replaces what is kept.

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

**Which is which.** The person says. One default: where Files holds one
unsigned transaction in exactly two PSBTs, their signature counts
differing, the one with fewer is Sent and the other Returned, both cards
closed on their choice with **Change** (DESIGN §4.14, Default), and the
check has run. Files of other transactions, one each, do not stop it; a
second transaction with two files, or three files of one transaction,
leave no default. Otherwise both cards are open, Sent first.

The Returned card lists first the PSBTs of the Sent one's transaction,
then under a section label "Other transactions" the rest. The file
chosen as Sent is not listed. Two files with the same bytes are listed
both; picking them gives "No signature was added" (§6.2, S9).

A signature count is every signature the file carries: partial,
taproot key and script, and those in final fields.

A file taken from Files is kept as it was chosen. Where Files now holds
another file of that name, or none, the copy and its report stay and the
closed card reads the name followed by "changed or no longer in Files"
(the same name and bytes in either box is the file);
**Change** picks again.

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
working screen. The ? opens the Learn section (`docs/history/SIGNER-CHECK-LOG.md` §10.3).

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
   - "Nonces recomputed", only where a signature was added under an
     ordinary key: "{n} of {m}" where a loaded key recomputed any, with a
     line "{fp} not loaded" under it per key that signed and is not
     loaded (a mix of keys loaded and not); where none was recomputed,
     "None: {fp} not loaded", one line per key not loaded;
   - "Sends", "Fee", "Change": the inspection's values for the Returned
     PSBT against the loaded wallets (§4.7); "Change" reads "verified"
     where at least one output is verified change on the change chain and
     no output claims a key origin it could not verify
     (`OutputKind::UnverifiedChange`), else "not verified".
3. **Findings**, a section label, then a Warning line (§4.11) per
   finding, ordered Blocked, Danger, Caution; within a level by where
   (the transaction, the inputs in order, the outputs in order), and
   within a place in the order the check found them (a stable sort). Each line
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
| "Only signatures were added" | `OK` | No finding, no input finalized, no MuSig2 nonce added |
| "Only signatures were added; {n} inputs finalized from them" | `OK` | No finding, an input finalized ("1 input finalized"), no MuSig2 nonce added |
| "Only nonces and signatures were added" | `OK` | No finding, a MuSig2 nonce among what was added, an input finalized or not (the nonce line wins) |
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
output key of the `tap_internal_key` it names with an origin. One key is
one point: a public key in its compressed and in its uncompressed form
is the same key wherever two signatures, or a signature and Sent's, are
compared (a repeated nonce, a changed signature). An added signature
filed under a form of a key other than one Sent's scripts hold (its
P2PKH hash, or its bytes in a script) is S3, the closest row; where no
script of Sent holds the key, only the compressed form is held.

| Id | Level | Wording | When |
|---|---|---|---|
| S1 | Blocked | "Input {i}: the signature under {key} is not a signature of this transaction" | Added, `Verdict::Invalid` |
| S2 | Caution | "Input {i}: the signature under {key} was not checked: {reason}" | Added, `Verdict::Unchecked` |
| S3 | Danger | "Input {i}: signature under {key}, a key the sent PSBT does not name" | Added under a key not named |
| S4 | Danger | "Input {i}: the signature under {key} signs with {kind}, not the {asked} asked for" | The sighash card's rule (`inspect/sighash.rs`) |
| S3l | Danger | "Input {i}: the signature under {key} is over a leaf the sent PSBT does not hold" | A `tap_script_sigs` signature whose leaf hash is in no `tap_scripts` of Sent and in no leaf list of that key's origin on Sent |
| S4n | Blocked | the same, with ": outputs are not committed to" | as S4, a `NONE` sighash the input does not ask for, and any `SINGLE` signature on an input with no output at its index (it commits to no output, whatever is asked) |
| S4m | Caution | "Input {i}: the signature under {key} signs with {kind}, which commits to more than the {asked} asked for" | as S4, a sighash committing to everything asked and more: every input over `ANYONECANPAY`, `ALL` over `SINGLE` over `NONE` (a `SINGLE` with no output at its index commits to none) |
| S5 | Blocked | "{key}: one nonce on inputs {list}" | `repeated_nonces` over Sent's and Returned's signatures together: one `r` or `R` in two signatures under any keys, compared as its 32-byte x coordinate for ECDSA and Schnorr alike (sibling keys of one xpub differ by a number the xpub gives, and one point is an ECDSA and an x-only key), or a point of a MuSig2 public nonce in two places, compared by its x coordinate (§6.4). An honest signer never does either. `{key}` names every key it was found under, separated by commas; `{list}` lists an input once. A signature changed to one with the same nonce is S5 as well as S8 |
| S6 | Danger | "Input {i}: the signature under {fp} could not be reproduced. Either the device signed with fresh randomness, or it chose the nonce" | The key's seed is loaded and `deterministic` finds no rule (RFC 6979, or BIP-340 with zero auxiliary randomness) for that signature's own bytes. A signature made with fresh auxiliary randomness (BIP-340 recommends it) is not told from one with a chosen nonce, which is what the check is for, so it is S6 as well, and the line says it is one or the other |
| S6u | Danger | "Input {i}: the signature under {fp} could not be recomputed" | An added signature whose key's master is among those given, and no result for its bytes (no signature hash could be computed, or another signature took its place in the view) |
| S7 | Caution | "Input {i}: the signature under {key} was removed" | On Sent, not on Returned (in no field and not in a final field) |
| S8 | Danger | "Input {i}: the signature under {key} changed" | On both, other bytes, unless both are final (F3 says so); and two signatures on Returned under one key and leaf with other bytes, in whichever fields |
| S9 | Caution | "No signature was added" | No signature or MuSig2 nonce added on any input, and no input finalized |

### 6.3 Finalization

An input the device finalized (final fields on Returned, none on Sent):

- Its signatures are read out of the final fields and held to §6.2.
- Faraday finalizes a copy of the Sent PSBT's input with exactly those
  signatures as its partial, `tap_key_sig` and `tap_script_sigs` (the
  copy's own are cleared first) (`finalize.rs`); the result's
  `final_script_sig` and `final_script_witness` must equal the
  device's byte for byte. Where the finalizer runs, that covers the
  scripts, a control block, a multisig's dummy item, and any extra item
  a device could add to the witness. Where it refuses (F2) nothing is
  compared and nothing accounts for what the final fields carry.
- The fields Faraday's own finalizer clears (BIP-174's list) are
  expected to be gone from that input; their absence is no finding.

| Id | Level | Wording | When |
|---|---|---|---|
| F1 | Danger | "Input {i}: finalized; the final fields differ from what its signatures give" | Faraday's final fields differ, or no signature can be read out of them and Faraday's finalizer refuses the input |
| F2 | Danger | "Input {i}: finalized; Faraday cannot finalize this input to compare" | Signatures were read out of the final fields and Faraday's finalizer refuses it (fewer signatures than the threshold, say): the other items of the final fields are unchecked |
| F3 | Danger | "Input {i}: the final fields changed" | Final on both, bytes differ |
| F4 | Caution | "Input {i}: the final fields were removed" | Final on Sent, not on Returned |

### 6.4 MuSig2

BIP-373's pairs (`musig.rs`) are known to the check, not unknown.

- A pair's key is read whole: participant (33 bytes), aggregate (33)
  and, optionally, a leaf hash (32). The aggregate must be one of the
  three forms the crate reads, anchored to Sent: the root aggregate Sent's
  participants field for that input is keyed by, the aggregate derived at
  the sub-path (Sent's internal key), or the taproot output key of the
  output Sent spends (which Core keys a key-path pair by, BIP-373), with
  the participant under it. A key with a leaf hash (98 bytes; BIP-373
  allows the form) is not read: script-path MuSig2 is not loaded, so such
  a nonce or partial signature is a field finding (M1, M2), whichever
  leaf it names. With the
  wallet registered the crate's own reader decides, and it checks the
  output key's parity and the derived aggregate by its x coordinate;
  without it, the output key is the script's x coordinate alone. Any other length or value makes the pair a field finding
  (§6.5), never clean, for a nonce and a partial signature alike.
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
- On an input that is final on Returned, the removal of Sent's participant,
  nonce and partial signature pairs is no finding: BIP-174's Input
  Finalizer clears every field but the UTXO and unknown fields, and to a
  reader of BIP-373 these types are known. Pairs kept there are compared
  as above (a changed one is M2); an input not final keeps M3.
- A point of a public nonce that is in two places, compared by its
  x coordinate, is S5 (§6.2),
  over Sent's and Returned's nonce pairs together: another input, or
  another participant or aggregate on one input. The same pair on Sent and
  Returned at one place is one place. Three partial signatures over one
  nonce solve for the key (BIP-327).

### 6.5 Other fields

Every pair the comparison decodes, other than signatures, finalization
and BIP-373's, at every level: global, each input, each output.

| Id | Level | Wording |
|---|---|---|
| M1 | Danger | "{where}: {field} added" |
| M2 | Danger | "{where}: {field} changed" |
| M3 | Caution | "{where}: {field} removed" |

An empty script is a script: adding one is M1, and replacing a script
with an empty one is M2.

M1 is not raised for a field §6.9 states: that one is an Info line.

`{field}` is `Field`'s name (`verify.rs`): "key origins", "spent
output", "previous transaction", "witness script", "extended public
keys", "taproot tree", "preimages" (the four hash kinds together) and the
rest, and for the files' own bytes "PSBT version", "version 2 fields" and
"modifiable flags". An added preimage, tap tree or taproot key origins is
never stated at Info (§6.9 does not name them).

**The version** (at the transaction). Version 0 on one side and 2 on the
other is M2. Both 0: an explicit `PSBT_GLOBAL_VERSION` of 0 on Returned
only is M1 and on Sent only is M3 (`bitcoin` does not keep it, so the
files' bytes say it); Bitcoin Core writes none.

**Version 2 only pairs** (both files version 2). Every pair of a type
`PsbtV2::to_v0` leaves out (BIP-370's and BIP-375's, `GLOBAL_V2_ONLY`,
`INPUT_V2_ONLY`, `OUTPUT_V2_ONLY` in `v2.rs`) other than the modifiable
flags is compared byte for byte, by place, on the two files' maps: added is
M1, changed M2, removed M3, one finding of each at a place ("version 2
fields"). A required locktime can change with no change to the
transaction worked out from it. BIP-375's silent payment pairs are in
those lists (global `0x07`, `0x08`, input `0x1d`, `0x1e`, output `0x09`,
`0x0a`), so none goes unread; on a version 0 file they are unknown pairs
(P1 to P6).

**The modifiable flags** (`PSBT_GLOBAL_TX_MODIFIABLE`, "modifiable flags").
BIP-370 ("Signer") requires a signer to update them, so a byte comparison
would condemn every conforming signer. The result is worked out from
Sent's byte: each signature added (partial, taproot, MuSig2 partial, or in
final fields) that does not use `SIGHASH_ANYONECANPAY` clears bit 0
(inputs modifiable), each that does not use `SIGHASH_NONE` clears bit 1
(outputs modifiable), each that uses `SIGHASH_SINGLE` sets bit 2; other
bits never change, and a taproot signature with no sighash byte
(`SIGHASH_DEFAULT`) commits like `ALL`. With the field on Sent, Returned's
byte equal to the result is nothing; any other value is M2 and the field
gone is M3, including a byte left as it was where the result differs.
That is strict for a signer that does not update the field, on purpose: a
case the rule does not name stays a finding. With no field on Sent,
staying absent is nothing, and a field on Returned is nothing only where
a signature was added and it holds exactly the result worked out from 0,
`00` included (BIP-370's signer "must update" the field, and a signer with
nothing to clear writes it as it is); anything else is M1, and so is the
field added with no signature added. A returned file with no signature added
holds Sent's byte, which the rule gives by itself.

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

The comparison is against the fields written out again in `bitcoin`'s
canonical form, not the maps as read (a key written as 65 uncompressed
bytes decodes to the same field and is longer than its canonical 33): the
length of that form, and of an explicit `PSBT_GLOBAL_VERSION` of 0 where
the file has one, is what the file's own length is held to. For a version
2 file the length is the version 0 form's, less the
`PSBT_GLOBAL_UNSIGNED_TX` pair `to_v0` wrote, plus every pair only version
2 has, written out again (§6.5 compares them all, so they are accounted
for); bytes past the last map, or a pair written longer than it needs,
are E1. A file whose maps cannot be read at all is `Kind::Unnamed` at
Danger, never clean, worded "{where}: {field} differs in a way the check
does not name" (`signer_unnamed` in `en.rs`).

Where the parser refuses such a file, it is refused on the card (§3),
which is no clean verdict either.

### 6.8 Stated, not findings (Info)

| Wording | When |
|---|---|
| "Input {i}: public nonce added for {key}" | §6.4 |
| "Input {i}: finalized" | An input finalized with no finding on it |
| "{where}: {field} added" (Info) | A field added that §6.9 checks in full |

### 6.9 Fields a signer adds that are checked in full (F199)

Major signers add fields beside their signatures: Bitcoin Core adds
scripts, UTXOs and tap fields for what it can solve, and Passport adds
the sighash type. An added field is stated, at Info and not a finding,
when its content is fixed by what was sent, each anchored only to the
unchanged transaction or to the sent PSBT, never to the returned PSBT's
own copy of anything. Each applies to a field absent on Sent and present
on Returned, under a key of exactly one byte (the type byte; the parser
refuses a longer key for these fields):

| Field | Stated when |
|---|---|
| Witness script | Its SHA256 is the P2WSH program of the output's script, or of the spent output's script; for P2SH-P2WSH, the HASH160 of `0 <SHA256(script)>` is the P2SH script, anchored directly and not through a redeem script Returned supplies |
| Redeem script | Its HASH160 is the P2SH script of the output or of the spent output |
| Witness UTXO | Equal to the output at this input's index in the sent non-witness UTXO, whose txid is this input's previous txid |
| Non-witness UTXO | Its txid is this input's previous txid and every input of it has an empty witness, read as parsed: a transaction with witness data has the same txid |
| Input internal key and merkle root | Together they recompute the spent output's key (no root: no script tree); a value Sent held may be used |
| Sighash type | Equal to the sighash byte of every signature on the input, and at least one: with none, it is a finding |

The spent output's script comes only from the sent PSBT's UTXO fields,
a non-witness UTXO with its txid checked; with none for the input there
is no anchor and the added field is M1. Tap trees, leaf scripts, key
origins, extended public keys, preimages, an output's internal key,
proprietary and unknown fields are findings. A removal is M3.

The wording of the Info line never says "changed" or "mine": whether a
signer adds such a field is itself a signal (Core adds a script only to
an output it can solve). What is left is a presence bit and the order of
a map, small beside a signature's nonce.

Sources (read 2026-11-03, nothing run): Bitcoin Core 31.1 `src/psbt.cpp`
(adds scripts, derivations, tap fields and UTXOs; drops non-witness UTXOs
when every input is taproot); Passport 2.3.11 `psbt.py` (adds the sighash
type); Coldcard `shared/psbt.py` (adds a redeem script for imported-key
inputs). Seed Signer, Specter DIY, Trezor, BitBox02 and Ledger add none or
return signatures only. BIP-174: a Signer "must only add data"; an Updater
adds scripts and derivations. Hosts: Sparrow's drongo `PSBT.java` rejects
replaced input scripts before merging; Electrum's `transaction.py` checks
UTXO consistency, not signatures.

## 7. Unknown fields in a single PSBT's review

The Spend review and every other inspection of one PSBT state the pairs
this crate does not read: one warning, Info, with the count across the
whole PSBT.

- `WarningKind::UnknownFields { count }`, `Level::Info`, worded "{n}
  unknown or proprietary fields: not read" ("1 unknown or proprietary
  field: not read").
- Counted: every pair in a global, input or output `unknown` or
  `proprietary` map, less BIP-373's MuSig2 pairs (`musig.rs`), which
  this crate reads. An `osk` record is counted: it is not read (F127).
- Info does not gate signing, and the Nonce check's headline is
  unchanged.

## 8. Code

### 8.1 Where it lives

- **The check**, in core/faraday-psbt/src/signer.rs (new),
  `faraday_psbt::signer`: `check(c, sent, returned, sent_bytes,
  returned_bytes, ctx) -> Report`, where `sent_bytes` and `returned_bytes`
  are the two files as the person has them (raw, base64 decoded where the
  file is text) and `sent` and `returned` their version 0 forms (a version
  2 file through `PsbtV2::to_v0`): the flow must pass both files' bytes, pure, with no key. `Report` holds the
  verdict, the facts, the findings as typed values (`Finding { level,
  at, kind }`, one `kind` per id in §6) and the Info lines. It calls
  `compare`, `verify_signatures`, `repeated_nonces`, the sighash rule,
  `finalize` on a copy, `partial_sig_verify`, and the inspection for
  the facts. Its first doc line says what it costs (CLAUDE.md).
- **`signer::nonce_view(sent, returned) -> Psbt`**: a copy of Sent
  carrying every signature of Returned as a partial or taproot
  signature, those read out of final fields included, with Sent's own
  origins. faraday-core calls it, runs `verify::deterministic` over it
  with the keys loaded (`verify::deterministic_all`, which also answers
  for a signature whose hash cannot be computed, so that which masters
  were given is known; `deterministic` leaves those out) (so a finalized input's signature reaches a key
  and a fingerprint), and hands the result to `with_nonces` (S6, S6u; the
  "Nonces recomputed" fact). The view also carries the fields §6.9
  states (scripts, UTXOs, internal key), so a signature hash can be
  computed where only Returned holds them. `Determinism` carries the
  leaf and the signature's bytes, so two signatures under one key in two
  leaves, or in two fields, are each judged on their own bytes.
- **The nonce lines**, in faraday-core's glue: `recompute_signatures_all`
  (`faraday/faraday-keys/src/psbt.rs`, and `Keys::recompute_signatures_all`
  in `session.rs`) through the session's keys, reached as
  `Session::recompute_all` (`faraday/faraday-core/src/wallet.rs`) (no new
  reveal, `docs/KEYS.md`), turned into S6 and the "Nonces recomputed"
  fact in `faraday/faraday-core/src/decode/check_words.rs` (every
  finding's, stated line's and field's text, in an exhaustive match). A
  key loaded or dropped re-runs the check while a report is kept: the
  press's `wallet_flow_sync` calls `signer_check_sync`, which compares
  the keys the report was made with against the keys now loaded.
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

Each row below is exact in `tests/all/signer.rs`: the verdict and the
whole list of findings, in order, at their levels.

In `tools/vectors/psbt/`, listed in its README. Existing pairs used as
they are:

| Sent | Returned | Expect |
|---|---|---|
| `wallet-musig-core-first.psbt` | `wallet-musig-core-first-signed.psbt` | clean: a nonce and a partial signature added (§6.4) |
| `wallet-tapmulti-first.psbt` | `wallet-tapmulti-signed.psbt` | clean, two script-path signatures |
| `demo-regtest.psbt` | `wallet-2of3.psbt` | Not the same transaction |

New, named signer-*.psbt, each a Sent and a Returned or a Returned
against an existing Sent. Two come from Bitcoin Core 31.1
(`walletprocesspsbt`, as the tapmulti and MuSig2 fixtures did); the
rest are written by a fixture writer in the test suite from those, one
change each, and committed:

| Fixture | From | Expect |
|---|---|---|
| core-sent | `wallet-2of3.psbt` plus the witness script Core added to output 1 | the Sent of core-signed and of every fixture below built from it (against `wallet-2of3.psbt` itself, core-signed is clean with one Info line for the witness script at Output 1, §6.9) |
| core-signed | `signer-core-sent.psbt` signed by Core holding the "zoo" key | clean |
| core-final | `demo-regtest.psbt` signed and finalized by Core | clean, 1 input finalized |
| output-changed | core-signed, output 0 one sat less | T7 and S1 (Core's signature no longer verifies over the changed transaction), verdict "{n} findings" |
| invalid | core-signed, a signature byte flipped | S1 |
| foreign-key | core-signed plus a valid signature under a key no origin names | S3 only (the signature verifies, so not S2) |
| sighash-none | `demo-regtest.psbt` signed with `NONE` | S4n |
| sighash-single | the same with `SINGLE` | S4 |
| nonce-reuse | two inputs signed under one key with one nonce | S5 |
| random-nonce | signed by the "abandon" key with a random nonce | S6 with "abandon" loaded; clean with "Nonces recomputed: None" without |
| sig-removed | `wallet-tapmulti-signed.psbt` less one signature, against itself as Sent | S7 and S9 (nothing was added) |
| sig-changed | core-signed, the signature remade with another valid nonce | S8 and S9 (nothing was added) |
| final-extra | core-final with an extra witness item | F1 |
| origin-removed | core-signed less an input's key origins | M3 |
| xpub-added | core-signed plus a global xpub | M1 |
| unknown-added | core-signed plus a global unknown pair | P1 |
| unknown-sent, unknown-changed | an unknown pair on Sent, its value two bytes longer on Returned | P2 |
| proprietary-added | core-signed plus an input proprietary pair | P3 |
| osk-added | core-signed plus an `osk` record | P3 |
| trailing | core-signed with eight bytes after the last map | refused at read (the parser refuses it, as it refuses a length written in more bytes than it needs, raw or base64; E1 is for bytes that parse with the Returned PSBT: a key written as 65 bytes in a version 0 file, or a version 2 key length written in more bytes than it needs, which the tests below build) |

Cases with no fixture file are built in the test from these fixtures, and
each asserts its exact findings at their levels: a signature added beside
final fields (S8, with "abandon" loaded S6 from a final field through
`nonce_view`); a script-path signature over a leaf Sent lacks (S3l); a
second signature under one key in another leaf judged on its own; a
`SINGLE` signature with no output at its index (S4n); a nonce or partial
signature keyed by more than a named participant, by an aggregate Sent
does not name, or by a leaf Sent does not hold (M1); a failing and a
removed MuSig2 pair; an empty script added or put in place of one (M1,
M2); a key written as 65 uncompressed bytes (E1); a map in another order
(no finding); Sent holding three of a 2-of-3 and the device finalizing
with the other two (S7, no F1).

### 9.2 The tests

- core/faraday-psbt/tests/all/signer.rs (new): every row of §9.1 gives
  its verdict and exactly its findings, at their levels (the Sent of the
  core-signed rows is core-sent); Swap of a clean pair gives S7 per
  signature and S9, and no clean verdict; a pair written with
  its maps in another order is clean; the two Core fixtures are clean.
- `core/faraday-psbt/tests/all/inspect.rs`: proprietary-added and
  unknown-added give one Info `UnknownFields` with count 1;
  `demo-regtest.psbt` gives none; the MuSig2 fixtures give none.
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

Moved to `docs/history/SIGNER-CHECK-LOG.md`.

### 10.1 Rules for the implementing agent

Moved to `docs/history/SIGNER-CHECK-LOG.md`.

### 10.2 Batches

Moved to `docs/history/SIGNER-CHECK-LOG.md`.

### 10.3 Edit list

Moved to `docs/history/SIGNER-CHECK-LOG.md`.

### 10.4 Questions for the owner

Moved to `docs/history/SIGNER-CHECK-LOG.md`.
