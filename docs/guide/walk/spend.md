# Spend

**Kind:** Spec
**Status:** Text moved from `docs/GUIDE.md` (§6.2 to §6.8 and the opening of §6), written and checked against the app's screens and strings; the pictures are in `docs/guide/` and keep their names
**Open:** the walkthrough's form (result line, what is needed, time, "With your own key"); the pictures, drawn once after the theming has landed

What you will do: check a receive address of section 5's 2-of-3 before
money is sent to it; then spend from it: the watch-only wallet writes the
transaction (a PSBT), Faraday reads it, two keys sign, and the signed
transaction goes back to the watch-only wallet, by QR code or on a stick.

What you need:

- Faraday as section 5 left it: **New wallet 1** loaded, with keys
  `3dca6805` and `5d388376` (**Can spend**).
- Sparrow on an online computer, with the wallet loaded (step 5.4), and
  a camera on the PC that runs Faraday. For the stick, a USB stick other
  than the Faraday stick.

The guide's transactions spend a coin that does not exist: each pays
0.015 BTC to `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu` (BIP 84's
first test address) from an input of 0.1 BTC. Faraday reads and signs
them; the network would refuse them.

### 6.2 Spend from this wallet

Press **Wallets**, then **New wallet 1**. The card's first button is
**Spend from this wallet**.

![The card, before a spend](../6-2-card.png)

The page **Spend from New wallet 1** opens, one card per step.

![Load the wallet in Sparrow](../6-2-sparrow.png)

| On screen | Meaning |
|---|---|
| **I have the PSBT from Sparrow ready: Scan it** | for a wallet already in Sparrow: the camera, then step 6.4 |
| **Show the wallet QR** | the wallet description, as in step 5.4 |
| **Receive address 0** | the address Sparrow lists first |
| **It is loaded** | Sparrow has the wallet; the next card opens |

Press **It is loaded**. The link at the top left is named after the
card before the one open (**Load the wallet in Sparrow**, once the next
card is open): it opens that card again, with what was done there kept.
On the first card it goes back to the wallet's card.

### 6.3 Write the payment, and bring it here

![Write the payment in Sparrow](../6-3-write.png)

In Sparrow, on the Send tab: the address to pay, the amount and a fee;
then *Create Transaction*, *Finalize Transaction for Signing* and *Show
QR*. Bring the PSBT to Faraday one of two ways:

| By QR code | On a stick |
|---|---|
| ![Scanning, 2 of 8 parts](../6-3-scanning.png) | ![A stick is attached](../6-3-stick-attached.png) |
| Press **Scan the PSBT** and hold Sparrow's animated code up to the camera. The line under the picture counts the parts (**UR: 2 of 8 · missing 3, 4, …**); the camera closes when every part is in. Go to step 6.4. | In Sparrow, *Save Transaction* to the stick. **On a stick** is grey until a stick is in (**Plug the stick in**). Plug the stick in: **A stick is attached** lists what the lock keeps and what it wipes. Press **Lock and use stick**. |

**Lock and use stick** wipes the keys and the wallet. What was not in a
vault is listed in red, **· nowhere else**: here both keys and the
wallet, which then come back only from their backups (section 7). With
the wallet and its keys in a vault, **Unlock** brings them back.

After the lock the stick visit opens:

![The stick visit](../6-3-visit.png)

**On the stick** lists the PSBT (`payment.psbt`) and the wallet's
description (`new-wallet-1-descriptor.txt`), both ticked. Press **Import
2 files**, pull the stick, and load the wallet and its keys again
(section 7, or **Unlock** for a vault). The card then reads **Sign the
PSBT from the stick**, and a bar above it, **payment.psbt in Files ·
Review**:

![The card with a PSBT in Files](../6-3-card-stick.png)

Press **Sign the PSBT from the stick**: the review of step 6.4 opens at
once. This picture, step 6.5's right-hand column and the stick
pictures of steps 6.6 and 6.7 show the wallet as section 7 restores
it, named **new-wallet-1-descriptor**, with key `3dca6805` alone; its
PSBT pays a fee of 1,850 sat. The other pictures of this section show
the QR way, with both keys.

### 6.4 Read the transaction

![Read the transaction](../6-4-transaction.png)

The card leads with one line, **Read what leaves, what comes back and
the fee before signing.**, and **Learn: Verifying**, which opens Learn's
page on what each row proves.

| Row | Here | Meaning |
|---|---|---|
| **Sends** | 0.01500000 to `bc1q cr8t … 6fyu` | what leaves the wallet, and to where |
| **Change** | 0.08497860 to 1/0, **Verified** | what comes back: change address 1/0 of this wallet, derived again from its keys |
| **Fee** | 0.00002140, 2,140 sat · 11.3 sat/vB | what the miners get |
| **Spends** | 0.10000000, 1 input | the coins spent |
| **Txid** | `1757 6707 … d6ee`, **Unchanged by signing** | the transaction's id, the same in Sparrow |
| the chips | **No warnings**, **Mainnet** | nothing found to warn of; the network |

Compare the address, the amount and the fee with what was typed into
Sparrow. **Show the table** lists each input and output whole:

![The table](../6-4-table.png)

Press **Continue**. Step 6.8 has what the card shows when something is
wrong.

**Verified** proves the change goes back to this wallet. Nothing proves
the address paid is the one meant: Faraday cannot know it, so compare it
with the payee's.

### 6.5 The keys that sign, and Sign it

The next card, **Add the seeds that can sign**, lists the wallet's keys:

| Both keys here | One key here |
|---|---|
| ![Two keys sign here](../6-5-signers.png) | ![One key here](../6-5-signers-short.png) |
| `3dca6805` and `5d388376` read **Signs here**, `cf2e083d` **Not here**; **2 of 2 signatures here** and **Can spend**. | `3dca6805` reads **Signs here**, the others **Not here**; **1 of 2 signatures here · 1 more needed**. **Scan a key** loads the second key (step 6.6). |

With both keys here, press **Continue**; with one, step 6.6 first.

![Sign it](../6-5-sign.png)

Press **Sign with 3dca6805, 5d388376**. Signing cannot be taken back:
press it only when step 6.4 read right. The next card, **The other
signatures**, reads **2 of 2** and closes by itself.

### 6.6 The second key

Both keys sign on this Faraday. With one here, load the second before
signing: on **Add the seeds that can sign**, press **Scan a key** and
hold that key's SeedQR up to the camera, as in
[Multisig with other people and devices](multisig-with-others.md#7-keys-kept-off-the-device),
step 7 (**Type the
words instead** for a key with no SeedQR).

| Scan a key | The second key in |
|---|---|
| ![Scan a key for the wallet](../6-6-scan-key.png) | ![Two keys sign here](../6-6-signers-two.png) |
| **Scan a key for new-wallet-1-descriptor** lists the keys still away (**Key 2 · 5d388376**, **Key 3 · cf2e083d**). | **Key 2 added · Can spend**, in a bar at the foot of the page; the row of `5d388376` reads **Signs here**, and the card **2 of 2 signatures here** and **Can spend**. |

Then **Continue** and **Sign with 3dca6805, 5d388376**: one press signs
with both keys, and **The other signatures** reads **2 of 2**.

### 6.7 Send it

![Send it](../6-7-send.png)

| Card | What it is | Press |
|---|---|---|
| **Signed PSBT** (For the wallet that wrote it · not finalised) | the PSBT with every signature, for Sparrow | **Show as QR** |
| **Finished transaction** (Ready to broadcast) | the transaction the network takes, with its txid and size | **Show the finished transaction as QR**, under the cards |

Both wait **For the stick** as well (**Remove** takes one off). Under the
cards, **Nonce check · 2 signatures valid · 2 recomputed here** says the
two signatures were checked here.

| Signed PSBT | Finished transaction |
|---|---|
| ![The signed PSBT as an animated code](../6-7-qr-signed.png) | ![The finished transaction as one code](../6-7-qr-tx.png) |
| In Sparrow, on the same transaction: *Scan QR*, hold it up to the webcam, then *Broadcast Transaction*. **UR** and **BBQr** are the formats; **2/s** to **5/s** the speed; **Small** to **Large** the size. | One code, for a wallet or a block explorer that reads a whole transaction. |

The finished transaction is safe to hand to anyone: changing where it
pays breaks its signatures. Whatever broadcasts it should report the
txid Faraday showed (`1757 6707 … d6ee`); if it reports another, what
was sent is not what was signed here.

On the stick way, the files take the PSBT's name
(`payment-signed.psbt`, `payment-final.txn`):

| Send it | Files |
|---|---|
| ![Send it, from the stick](../6-7-stick-send.png) | ![Files, waiting for a stick](../6-7-write-out.png) |
| Both cards read **For the stick**; under **Nonce check**, one row per signature, **Valid · nonce recomputed, low R**. | **Files** lists them under **For the stick**, each **Ready**, with **Waiting for a stick**. Plug a stick in: the stick visit writes them. In Sparrow, open the signed PSBT from the stick (*File → Open Transaction → File…*), then *Broadcast Transaction*. |

Then **Put everything away**:

![Put everything away](../6-7-away.png)

It lists the files waiting for the stick (`scanned-1-signed.psbt`,
`scanned-1-final.txn`). **Lock** wipes the session and keeps them; plug
a stick in and the stick visit writes them (**Write 2 files**). Then
**Power off**.

### 6.8 When the transaction is not right

The card of step 6.4 says so in its rows and under them, each warning
in its own words, in red for danger. A PSBT opened from Files (its
**Sign**) opens **Sign a transaction**, the same review:

| A fee of 0.012 BTC | Another wallet's PSBT |
|---|---|
| ![The fee warnings](../6-8-fee.png) | ![Change unverified](../6-8-other-wallet.png) |
| **fee 1200000 is 80% of the amount sent** and **fee rate is about 6349 sat/vB**. | **Wallet** reads **No loaded wallet matches**; the change is a second **Sends** row and **Change** reads **None**, with **output 1 claims to be change of a wallet you are using, but the script is not that address**. |

| On the card | Meaning | Do |
|---|---|---|
| A second **Sends** row, and "output … claims to be change of a wallet you are using, but the script is not that address" | an output claims to be change but does not derive from this wallet's keys; it is shown as a payment | do not sign: the "change" may pay someone else |
| "fee … is …% of the amount sent" | the fee is a large share of the amount, or above 0.01 BTC; Review stops at **Hold to pay this fee** | do not sign; write the payment again in Sparrow with a lower fee. Hold only for a child paying for its parent, a large batch or a consolidation |
| "fee rate is about … sat/vB" | 1,000 sat/vB or more | the same |
| **Wallet** with **No loaded wallet matches**; on its card, "No loaded wallet matches this transaction. Change cannot be verified until its wallet is loaded." | the PSBT spends from a wallet not loaded here | load the right wallet, or the right PSBT |
| "input … spends from a wallet of several keys that is not registered here; register the wallet to sign", with **Register this wallet** under it | the wallet this PSBT spends is not registered here, and the PSBT names its cosigners' keys | press **Register this wallet**: check the shape (**2 of 3, P2WSH, sorted**), the cosigners and the one marked **Yours**, then hold **Hold to register**; the card can then be signed |
| the same, with "Load this wallet's descriptor to sign" under it | the wallet is not registered here and the PSBT does not carry its keys | load the wallet's descriptor (Files, a code, or **Restore an existing wallet**) |
| **Amount unchecked** on **Spends** | an input's amount is only stated, not proved by its previous transaction | do not sign a transaction of several inputs; Faraday refuses one, with no way past |
| **No key in this session signs this transaction** | none of the keys here is one of the transaction's | load the wallet's keys |

Declining costs nothing: press **Cancel** at the top right, and nothing
is signed.

> **With your own key**
>
> - Read every output and the fee on Faraday's screen before signing.
>
> Why: [Verifying](../../learn/06-verifying.md),
> [Mistakes and scams](../../learn/08-scams.md).
