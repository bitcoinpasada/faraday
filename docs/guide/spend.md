# Spend

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Spend`
**Key:** spend

One card per step, from the wallet a transaction spends from to the signed result. The transaction is written in the online wallet and brought here as a PSBT; nothing is signed until Sign.

## Wallet

**Key:** spend.wallet

The wallet is the loaded one whose script the transaction's first input spends. Its name, shape and source are shown.

**Continue** goes on. With no loaded wallet matching, the card says so, and change cannot be verified until the wallet is loaded.

## Path

**Key:** spend.path

Shown only for a wallet with several ways to spend. Each path lists its keys, tagged as held here or not, and either **No wait** or the wait each coin must have sat for. A line says whether this transaction's sequence allows that path. Whether the wait has passed is known to the network only. **Continue** goes on.

## Transaction

**Key:** spend.transaction

The transaction as rows. **Sends** gives each recipient and amount; **Change** is shown only for an output derived from the loaded wallet, marked **Verified**. **Fee** gives the amount and its rate. **Spends** gives the total and the number of inputs, with **Amount unchecked** where an input's amount is not stated. **Txid** gives the id, **Unchanged by signing**, when every input is native SegWit or Taproot, and **Not known until signed** otherwise.

**Show the table** and **Hide the table** list each input and output. **No warnings** appears when none apply. **Register this wallet** appears for a wallet not yet registered. **Compare** sets a fee-bump copy beside this transaction. **Continue** is absent while the review stops the spend.

## Check

**Key:** spend.check

The wallet's addresses **Receive 0/0**, **Receive 0/1** and **Change 1/0**, to compare with the software that made the transaction. Matching addresses mean both read the same wallet. **Continue** goes on.

## Signers

**Key:** spend.signers

Shown for a wallet of several keys. Each key is listed with its fingerprint and marked **Signs here** when loaded on this device, or **Not here**. The count says how many keys are here against how many must sign.

**Load from vault** loads a key an open vault holds. **Load with a passphrase** does so for a seed kept without the passphrase this wallet uses. When the keys here are short, **Scan a key**, **Unlock** with a vault's name, and **Collect a signature** are offered. Keys cannot be loaded while a stick is attached. **Continue** goes on.

## Nonces

**Key:** spend.nonces

Shown for an **All keys · MuSig2** wallet. Each key adds a one-time nonce before anyone signs; its row reads **Nonce here** or **Waiting**, and **This device** marks the keys held here.

An Add from button for each file in Files brings in another signer's copy. **Show as QR** shows this device's copy. **Go first: share this device's nonce** goes to Sign. **Continue** goes on. Locking the session forgets the nonce.

## Sign

**Key:** spend.sign

**Sign with** a list of fingerprints signs with every key of the wallet loaded here, and each signature is checked before it is kept. Afterwards the card reads Signed with those keys. With no key of the session able to sign, the card says so. A multisig needs further signatures before the transaction is finished.

## Signatures for this transaction

**Key:** spend.collect

Shown for a wallet of several keys. Each key reads **Signed here**, **Verified**, **Waiting**, **In file, unconfirmed**, **Does not verify** or, once enough have signed, **Not needed**.

An Add from button for each file in Files adds the signatures of a copy of this transaction; a copy of another transaction is not offered. **Show as QR** and **Scan a copy** pass the PSBT to a cosigner and back. **Continue** appears once the signatures suffice.

## Finish

**Key:** spend.finish

Before enough keys have signed, the card reads Needs and the count still missing.

Two results follow. **Signed PSBT** goes back to the wallet that made the transaction, which finalises and broadcasts it. **Finished transaction** is ready to broadcast as it is. Each has **Make again**, or **Remove** while it waits for a stick, and the PSBT has **Show as QR**. **Decode it** takes the transaction apart. **Show the raw hex** and **Hide the raw hex** toggle its bytes.
