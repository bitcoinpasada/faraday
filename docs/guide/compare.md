# Compare two PSBTs

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Compare`
**Key:** compare

Lists how two PSBTs or transactions differ, in three cards: **First**, **Second** and **Result**. Nothing is judged and nothing is signed.

## First

**Key:** compare.first

The first transaction; where the two are an original and its replacement, the original. Each PSBT or transaction in Files is a row, with its name and its number of signatures and short transaction id, or **Finished** for a finished transaction; **No PSBT or transaction in Files** shows when there is none. **Scan** takes it from a QR code, and a code that is neither a PSBT nor a transaction is refused with **Not a PSBT or a transaction**.

## Second

**Key:** compare.second

The second transaction, chosen as the first is; where the two are an original and its replacement, the replacement. Rows of a different transaction than the first card's are listed under **Other transactions**.

## Result

**Key:** compare.result

**Transaction**, **Fee**, **Inputs** and **Outputs** are facts of each side: the fee in sats and in sat/vB, **Not known** where the input amounts are not stated, and the inputs and outputs counted as in both, first only and second only. **Differences** has a line for each difference in the version, the locktime, the inputs' sequences and the outputs' amounts, or **No differences**.

## Fee bump

**Key:** compare.fee-bump

**Fee bump** appears when the two are different unsigned transactions that spend at least one outpoint in common. It reads **Only the fee is raised** when the replacement spends the same inputs, pays the same amounts to the same addresses and takes the extra fee from change or from an input of a loaded wallet; otherwise **More than the fee changed** or **Cannot tell whether only the fee is raised**, with a line for each cause.

## Actions

**Key:** compare.actions

**Swap** exchanges the two. **Check a signer** is offered where the two are one unsigned transaction, and opens that check with the first as sent and the second as returned. A **Load** button with a fingerprint is offered for a key the result names and that is not loaded. **Choose again** reopens **Second**, and **Done** leaves.
