# Nonce check

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `NonceCheck`
**Key:** nonce-check

Nonce check looks at the signatures of a signed PSBT or a finished transaction you bring in, and signs nothing. It refuses a signature that is not of this transaction, or that shares a nonce with another.

## The result

**Key:** nonce-check.result

The headline is green when every signature is sound, amber when some could not be checked and red when one is wrong. **Nonce check · a signature is wrong**, **Nonce check · final fields fail the script** and **Nonce check · signing data does not match its output** are the red ones. **Nonce check · {} not checked: the outputs they spend are not here** names inputs whose spent output is missing; a raw transaction states none, so it is checked against a PSBT of the same transaction in Files.

If two signatures share a nonce, a red line for that key says **one nonce on inputs**, because a shared nonce gives the key away.

## Inputs

**Key:** nonce-check.inputs

Each input is a row with its key and its verdict. **Valid · nonce recomputed** gives the nonce rule that reproduces the signature, **low R**, **first** or **BIP-340**; the check signs the input again with a key loaded here. **Valid · signed elsewhere, its nonce needs that device's key** is a signature under a key not loaded here. **Valid · not the nonce this key makes: do not broadcast** is a signature that none of those rules reproduces.

**Invalid: not this key's over this transaction** is a signature over something else. **Valid · no key origin: its nonce is not recomputed** and **Valid · finalized: its nonce is not recomputed** are signatures that cannot be matched to a loaded key. **Hash preimage · not a signature** marks an input that holds a hash preimage, and **Not checked** gives the reason a signature was not checked.

## Nonce history

**Key:** nonce-check.history

With a vault open and **Nonce history** on in Settings, the check compares each signature's key and nonce with those the vault holds. **Not seen before in this vault** is the clean result. A nonce used before on another transaction is a red line, **nonce used before, on another transaction**, and the transaction is not finished. A clean history does not show that a signer is honest.

## What can be done

**Key:** nonce-check.done

Below the result, when no finished transaction is offered, you see why: **Not finished: more signatures are wanted**, or the reason a signature is refused. When the transaction is sound, **Save to a stick** writes it to a stick, **Remove** takes it off the Outbox when it already waits there for a stick, and **Show as QR** shows it as a code. A finished transaction also offers **Decode it**, and **Show the raw hex** with **Hide the raw hex**. A refused transaction offers none of them.
