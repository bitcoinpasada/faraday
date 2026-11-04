# Check a signer

**Kind:** Reference
**Status:** Checked against the app's code and strings (`faraday/faraday-core/src/decode/check_screen.rs`, `faraday/faraday-core/src/decode/check.rs`, `faraday/faraday-core/src/catalog.rs`); no picture yet
**Screen:** `SignerCheck`
**Group:** Tools

**Opened from:** **Tools → Check a signer**, which needs no key and no
wallet; or **Check a signer** in the ⋮ of a PSBT's row in
[Files](files.md#a-file-from-the-stick), From the stick or For the stick,
which opens it with that file as the PSBT returned.

The PSBT sent to a signing device compared with the one that came back:
whether the device only added signatures. Nothing on it signs or changes a
file.

## On this screen

Three cards, one open at a time; a closed card reads what was chosen.

| Control | What it does |
|---|---|
| **Sent** | the PSBT that went to the device: a row for each PSBT in Files, by name, with "{n} signatures" and the start of its transaction id; **Scan** reads it from a QR code instead, named "Scanned" and kept off Files |
| **Returned** | the PSBT that came back, the sent one's transaction first, then "Other transactions"; **Scan** as on Sent |
| **Change** | on a closed card: opens it to choose again |
| **Result** | the verdict, its facts, the findings and what is stated |
| **Show as QR** | the returned PSBT as a [QR code](qr-and-scan.md#a-qr-code) |
| **Swap** | exchanges the two and checks again |
| **Choose again** | back to the card for the PSBT returned |
| **Done** | back to where the check was opened |

When one transaction has exactly two files in Files, with different
numbers of signatures, the one with fewer is taken as sent and the other as
returned, both cards are closed, and the check has run.

## The result

| Line | Meaning |
|---|---|
| "Only signatures were added" | ok: the transaction is the one sent and the only differences are signatures that verify, under keys the sent PSBT names. "; 1 input finalized from them" where the device finished an input |
| "Only nonces and signatures were added" | ok: the same for a MuSig2 PSBT, which also gains public nonces |
| "{n} findings" | warning where all are cautions, error otherwise: each finding names its place (Transaction, Input 0, Output 1) and its level, Blocked, Danger or Caution |
| "Not the same transaction" | error: no input in common; both transaction ids are shown |

The facts: Transaction, Signatures added, Inputs finalized, Nonces
recomputed, Sends, Fee and Change. "Nonces recomputed" reads "1 of 2" where
a loaded key signed the input again here and made the same signature; "None:
0f1a2b3c not loaded" says whose seed is missing, and the button **Load 0f1a2b3c**
opens Add a key and comes back to the result, which is checked again. A
nonce not recomputed is not cleared. Stated lists what was seen and is not
a finding.

A clean verdict says only that signatures were added. Read the transaction on
[Sign a transaction](sign.md) before signing.

## On a narrow window

Each card is a page of its own; the Result page scrolls.

## Why

- [Verifying](../../learn/06-verifying.md#checking-a-signer)
- [Nonces](../../learn/13-nonces.md)
