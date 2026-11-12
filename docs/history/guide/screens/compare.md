# Compare two PSBTs

**Kind:** History
**Status:** Checked against the app's code and strings (`faraday/faraday-core/src/compare/screen.rs`, `faraday/faraday-core/src/compare/words.rs`, `faraday/faraday-core/src/catalog.rs`); no picture yet
**Screen:** `Compare`
**Group:** Tools

**Opened from:** **Tools → Compare two PSBTs**, which needs no key and no
wallet; or **Compare** in the ⋮ of a PSBT's or a finished transaction's row
in [Files](files.md#a-file-from-the-stick), From the stick or For the stick,
which opens it with that file as the Second.

Two PSBTs, or finished transactions, side by side: how they differ, and
whether a replacement only raises the fee of an original. It lists
differences and does not judge them. Nothing on it signs or changes a file.
For one transaction sent to a signing device and the PSBT that came back,
see [Check a signer](signer-check.md).

## On this screen

Three cards, one open at a time; a closed card reads what was chosen.

| Control | What it does |
|---|---|
| **First** | the first PSBT or finished transaction: a row for each in Files, by name, with "{n} signatures" and the start of its transaction id, or "Finished · 3dca6805" for a finished transaction. "No PSBT or transaction in Files" where there is none |
| **Second** | the same; the files that spend an input the First spends come first, then "Other transactions" |
| **Scan** | on each card: reads it from a QR code instead ("Scan the first PSBT", "Scan the second PSBT"), named "Scanned" and kept off Files. Bytes that are neither read "Not a PSBT or a transaction" |
| **Change** | on a closed card: opens it to choose again |
| **Result** | the facts, the fee bump and the differences |
| **Swap** | exchanges the two and compares again |
| **Choose again** | back to the card for the Second |
| **Check a signer** | only where the two are one unsigned transaction: opens [Check a signer](signer-check.md) with the First as sent and the Second as returned, checked |
| **Load 0f1a2b3c** | where a key not loaded is named in the fee bump: opens Add a key and comes back to the result, which is worked out again |
| **Done** | back to where the comparison was opened |

Opened from a file's ⋮, where exactly one other transaction in Files spends
an input of it, that one is the First and the comparison has run.

## The result

| Line | Meaning |
|---|---|
| **Transaction** | the transaction id, where the two are one unsigned transaction; otherwise **First** and **Second** each with its own |
| **Fee** | each fee as "2,140 sat · 11.3 sat/vB", **First** and **Second**; "Not known" where an input's amount is not |
| **Inputs**, **Outputs** | "{n} in both", "{n} first only", "{n} second only" |
| **Differences** | a line for each difference, with no tone: "Version: 1 first, 2 second", "Input {outpoint}: first only", "Output to {address}: {amount}, first only", and so on; "No differences" where there are none |

### Fee bump

Shown where the two are an original and its replacement: the same inputs
spent, one of them replacing the other.

| Result | Meaning |
|---|---|
| "Only the fee is raised" | ok: the same payments to the same addresses; the added fee comes from inputs added that are the person's own, or from change |
| "More than the fee changed" | error: the lines say what: an input dropped or added that is not the person's, a sequence, version or locktime changed, a payment changed, dropped or added, or a fee that did not rise |
| "Cannot tell whether only the fee is raised" | warning: the lines say what could not be read: an input's amount not stated, a change output of a key not loaded, an added input of a key not loaded |

The facts under the lines: **Fee** of the original and the replacement,
"Inputs added" where the replacement adds any, and "Replace-by-fee" as
"Signalled by the original" or "Not signalled by the original".

The result is a statement and never stops anything. Read the transaction on
[Sign a transaction](sign.md) before signing.

## On a narrow window

Each card is a page of its own; the Result page scrolls.

## Why

- [Verifying](../../learn/06-verifying.md)
