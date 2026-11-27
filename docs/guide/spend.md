# Sign a transaction

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Spend`
**Key:** spend
**Words:** psbt, fingerprint, multisig

Signing is one card per step, from the wallet a transaction spends from to the signed result. You make the transaction in your online wallet and bring it here as a PSBT; nothing is signed until the Sign card.

## Wallet

**Key:** spend.wallet

Shows which loaded wallet the transaction spends from: the one whose script its first input spends, with its name, shape and source. If that is not the wallet you meant, stop here.

**Continue** goes to the next card. If no loaded wallet matches, the card says so, and change cannot be verified until the wallet is loaded.

## Path

**Key:** spend.path

Shown only for a wallet with several ways to spend. Each path lists its keys, marked as held here or not, and either **No wait** or how long each coin must have sat before that path can spend it. A line says whether this transaction's sequence allows the path. Only the network knows whether the wait has passed. **Continue** goes to the next card.

## Transaction

**Key:** spend.transaction

The transaction, row by row. Check it against what you meant to send; nothing is signed yet, and if anything differs, stop.

**Sends** gives each recipient and amount. **Change** is shown only for an output derived from the loaded wallet, marked **Verified**. **Fee** gives the amount and its rate. **Spends** gives the total and the number of inputs, with **Amount unchecked** where an input's amount is not stated. **Txid** gives the id: **Unchanged by signing** when every input is native SegWit or Taproot, **Not known until signed** otherwise.

**Show the table** and **Hide the table** list each input and output. **No warnings** appears when none apply. **Register this wallet** appears for a wallet not yet registered. **Compare** opens Check a signer with this transaction as the first, to set a fee-bump copy beside it. **Continue** is absent while the review stops the spend.

## Check

**Key:** spend.check

A wallet you have already checked since the device started has this card done.

The wallet's addresses **Receive 0/0**, **Receive 0/1** and **Change 1/0**, for you to compare with the software that made the transaction. If they match, both are reading the same wallet. **Continue** goes to the next card.

## Signers

**Key:** spend.signers

Shown for a wallet of several keys. Each key is listed with its fingerprint and marked **Signs here** when it is loaded on this device, or **Not here**. The count says how many keys are here against how many must sign.

**Load from vault** loads a key that an open vault holds. **Load with a passphrase** loads a seed kept without the passphrase this wallet uses. When too few keys are here, **Scan a key**, **Unlock** with a vault's name, and **Collect a signature** are offered. Keys cannot be loaded while a stick is attached. A spend that came from a stick says which of its four steps it is at: **Step 1 of 4 · Pull the stick**, **Step 2 of 4 · Load the key**, **Step 3 of 4 · Sign** and **Step 4 of 4 · Write to the stick**. **Continue** goes to the next card.

## Nonces

**Key:** spend.nonces

Shown for an **All keys · MuSig2** wallet. Every key adds a one-time nonce before anyone signs; each row reads **Nonce here** or **Waiting**, and **This device** marks the keys held here.

An Add from button for each file in Files brings in another signer's copy. **Show as QR** shows this device's copy. **Go first: share this device's nonce** goes to Sign. **Continue** goes to the next card. Keep the session unlocked until the signatures come back: locking forgets the nonce.

## Sign

**Key:** spend.sign

**Sign with**, followed by the fingerprints, signs with every key of the wallet that is loaded here, and each signature is checked before it is kept. Afterwards the card reads Signed with those keys. If no loaded key can sign, the card says so and offers the ways to load the wallet's key: **Type the words**, **Scan a SeedQR** and **Open** a vault. A key that is not the wallet's is refused. With a stick attached, the keys load once it is out. A multisig needs further signatures before the transaction is finished.

## Signatures for this transaction

**Key:** spend.collect

Shown for a wallet of several keys. Each key reads **Signed here**, **Verified**, **Waiting**, **In file, unconfirmed**, **Does not verify** or, once enough have signed, **Not needed**.

An Add from button for each file in Files adds the signatures from a copy of this transaction; a copy of a different transaction is not offered. **Show as QR** and **Scan a copy** pass the PSBT to a cosigner and back. **Continue** appears once there are enough signatures.

## Finish

**Key:** spend.finish

Until enough keys have signed, the card reads Needs and the number still missing.

Then there are two results. **Signed PSBT** goes back to the wallet that made the transaction, which finalises and broadcasts it. **Finished transaction** is ready to broadcast as it is. Each has **Make again**, or **Remove** while it waits for a stick, and both have **Show as QR**. The button at the foot, **Lock and write to the stick**, locks, so the keys leave memory, and when the stick comes back it writes the waiting files and checks them, with no trip through Files; it then offers **Sign another**, **Lock** and **Power off**. **Decode it** takes the transaction apart. **Show the raw hex** and **Hide the raw hex** show and hide its bytes.
