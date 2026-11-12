# Check a signer

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `SignerCheck`
**Key:** signer-check

Compares the PSBT sent to a signing device with the one that came back, in three cards: **Sent**, **Returned** and **Result**. The result says what the signer changed.

## Sent

**Key:** signer-check.sent

The PSBT that went to the signer. Each PSBT in Files is a row, with its name, its number of signatures and a short form of its transaction id; **No PSBT in Files** shows when there is none. **Scan** takes it from a QR code instead. A row of a different transaction than the other card's is listed under **Other transactions**.

## Returned

**Key:** signer-check.returned

The PSBT the signer gave back, chosen as the sent one is. The check runs once both are chosen. A file that was changed or is no longer in Files is marked **changed or no longer in Files**.

## Result

**Key:** signer-check.result

The verdict comes first. **Only signatures were added** is clean: the transaction is the one sent, and the only differences are verified signatures and inputs finalized from them. **Only nonces and signatures were added** is the same for a MuSig2 PSBT, which also gains public nonces. Otherwise it reads **1 finding** or a count of findings, in red when the worst is Danger or Blocked and amber otherwise, or **Not the same transaction**. A clean verdict does not say the transaction was the one wanted: read it in Spend.

## Facts and findings

**Key:** signer-check.findings

**Signatures added**, **Inputs finalized** and **Nonces recomputed** count what changed; **Sends**, **Fee** and **Change** are those of the transaction. Each finding names its place, **Transaction**, an input or an output, and its level, **Blocked**, **Danger**, **Caution** or **Info**. **Stated** lists lines that are no finding.

## Actions

**Key:** signer-check.actions

**Show as QR** shows the returned PSBT as a code. **Swap** exchanges the two PSBTs and checks again. A **Load** button with a fingerprint is offered for a key that signed and is not loaded; it opens Add a key and returns here. **Choose again** reopens **Returned**, and **Done** leaves.
