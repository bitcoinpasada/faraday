# Multisig with other people and devices

**Kind:** Spec
**Status:** Written and checked against the app's code and strings; step 6 moved from `docs/GUIDE.md` (§5.6), its pictures in `docs/guide/` under their old names; the other pictures are named and not yet drawn
**Open:** the pictures, drawn once after the theming has landed

**You end with:** key `3dca6805` in a 2-of-3 wallet whose other two keys
are held by other people on their own devices, the wallet loaded on
Faraday with its first address checked, and a spend signed with your
key and sent back for theirs.

**Before you start:** [Make a key](new-key.md),
[Make a wallet](make-a-wallet.md) (the multisig steps) and
[Spend](spend.md).

**You'll need:**

- Faraday with key `3dca6805` loaded from its paper copy (typed, as
  [Make a wallet](make-a-wallet.md#51-load-the-keys), step 5.1, does)
  and no stick attached.
- Two cosigners, each with a key on a device of their own. In this
  walkthrough the public test keys `5d388376` (twenty-four times
  **zebra**) and `cf2e083d` (twenty-four times **summer**) stand for
  them.
- A coordinator: Sparrow, or another wallet that makes a multisig from
  its cosigners' keys, on an online computer.
- For the stick ways, a USB stick other than the Faraday stick.

**Time:** about 40 minutes.

A multisig wallet is made from each key's public part, its xpub, with
the fingerprint and path the key was derived at. A cosigner gives that
and keeps the words. A coordinator puts the xpubs together and writes
the transactions; each signer signs on its own device.

## 1. Give your key's xpub

Press **Home**, then **Make a new wallet**. Press **Multisig · native
SegWit**; on **Quorum**, **Signatures needed** 2 and **Keys** 3, then
**Continue**; on **Use vaults**, **No**.

On **Keys**, under **Key 1**, press **3dca6805 · New key 1**. The row
reads **3dca6805 · New key 1 · loaded here**, and three buttons give its
xpub out:

| Press | Gives |
|---|---|
| **Show xpub QR** | the key as a QR code, its fingerprint, kind and path labelled under it, for a coordinator or a cosigner's device with a camera |
| **Xpub PNG for the stick** | the same code as a picture, waiting for the stick |
| **Xpub file for the stick** | the key as a text file, waiting for the stick |

*Picture to come: `multisig-with-others-xpub-qr.png`, Key 1's xpub as a QR code, its label under it.*

The path is `m/48'/0'/0'/2'`, the standard for a native SegWit
multisig. In the coordinator, add this key as a cosigner by scanning
the code, or open the file from the stick. The xpub cannot spend.

## 2. Take the cosigners' keys

To make the same wallet on Faraday, fill Key 2 and Key 3 with the
cosigners' xpubs. Each slot offers:

| Press | Then |
|---|---|
| **Scan its xpub** | the camera opens; the cosigner shows their key's QR code. A bar names the file and the slot ("… is Key 2"), and the slot reads **5d388376 · a cosigner's xpub** |
| the button named after a file in Files ("From" and its name) | an xpub file copied in on a stick visit fills the slot |
| **Someone else adds it later** | the slot reads **A cosigner adds it later · waiting for its xpub file**; the wallet is made once that file is in |

*Picture to come: `multisig-with-others-keys.png`, Keys with Key 1 loaded here and Key 2 a cosigner's xpub.*

A file whose key cannot fill the slot stays in Files, and a bar says
why: its key has no fingerprint and path, it holds several keys, or its
path is not this kind's. Ask the cosigner for an export with the
fingerprint and the path.

With all three filled, press **Make the wallet**. **Check** shows the
**Descriptor** (**2 of 3 · native SegWit multisig · 3dca6805, 5d388376,
cf2e083d**), the **Path** and **The wallet's first addresses**. Receive
0/0 begins `bc1q rfhg t7dv` and ends `ecxs 0246 6p`, the same address
as in [Make a wallet](make-a-wallet.md#52-make-a-new-wallet): the same
three xpubs make the same wallet, whoever holds the words. Compare it
with the coordinator's first receive address, then press **It
matches**.

## 3. Or load the coordinator's wallet

Where the coordinator made the wallet, load its file instead of making
it here. The coordinator exports a descriptor, a wallet file or a BIP 129
record.

| By QR code | On a stick |
|---|---|
| Press **Scan** on Home and hold the coordinator's wallet QR code up to the camera. A bar says the file is in Files. | Save the export to the stick. With no key loaded, plug it in and press **Stick visit** on Home; the file comes in ticked. Press **Import 1 file**, then pull the stick. With a key loaded, a stick goes in only after a lock, which wipes the key ([Files, sticks and QR codes, step 2](files-and-sticks.md#2-a-stick-visit-bring-files-in)). |

Then press [**Files**](../screens/files.md) at the foot of the sidebar, and **Load the wallet**
on the file.

*Picture to come: `multisig-with-others-load.png`, Files with the coordinator's wallet file and Load the wallet.*

On the wallet's card, under **At a glance**, `3dca6805` reads **Can
sign here** and the other two **Cosigner · xpub only**. The card reads
**1 of 2 signatures possible here**. Compare **First receive address ·
0/0** with the coordinator's, character by character
([Receive](receive.md#54-show-the-wallet-to-a-watch-only-wallet)).

A wallet whose keys include none loaded here is not refused, but it
verifies no change and signs nothing.

## 4. A key path another key spends

A Taproot wallet with a script tree can also be spent by one key
alone, its key path. Where that key is not one of the wallet's own,
Faraday asks before adding the wallet, whichever way it came in:

| On the sheet | Meaning |
|---|---|
| **Another key spends this wallet alone** | the sheet's title |
| the wallet's name, then "this key is not one of its keys and can spend everything alone, without the other signatures", and the key | the key that can spend without the quorum |
| **Hold to add this wallet** | adds the wallet once held to the end |
| **Cancel** | adds nothing |

*Picture to come: `multisig-with-others-key-path.png`, the key path sheet over Files.*

Hold only when you know whose key it is and why the wallet has it.
Once added, the card keeps the line naming the key, and where it would
read **Can spend** it reads **Another key can spend**.

## 5. A first spend: register the wallet

A coordinator's PSBT can arrive before its wallet is loaded here. With
only key `3dca6805` loaded, bring the PSBT in with **Scan** on Home
and press **Sign** on it in **Files**. **Sign a
transaction** opens on its review.

The **Transaction** card names the problem, "input 0 spends from a
wallet of several keys that is not registered here; register the
wallet to sign", and nothing can be signed. Where the PSBT carries the
cosigners' xpubs, **Register this wallet** is under it:

| On the sheet | Meaning |
|---|---|
| **Register this wallet** | the sheet's title |
| **2 of 3, P2WSH, sorted** | the quorum, the script and the key order |
| a line per cosigner: fingerprint and path | the wallet's keys; **Yours** marks the one key `3dca6805` gives |
| **Name** | the wallet's name, from the PSBT's file name at first |
| **Hold to register** | adds the wallet once held to the end |
| **Cancel** | adds nothing |

*Picture to come: `multisig-with-others-register.png`, Register this wallet with its shape, three cosigners and Yours.*

Where both readings fit the transaction, the sheet adds **Most
coordinators sort the keys** over **Sorted** (chosen) and **In this
order**; keep the coordinator's. The hold adds the wallet only when its
keys give every input's address and one of them is yours; otherwise
the sheet names why:

| Line | Meaning |
|---|---|
| **These keys do not give input 0's address** | the keys and the transaction do not match |
| **Output 1 names this wallet but is not its address** | an output claims to be this wallet's and is not |
| **None of these keys is yours** | no key loaded here is one of the wallet's |
| "Key … at … has your fingerprint but is not your key" | a key under your fingerprint that your key does not give |

Do not register a wallet the sheet refuses. Where the PSBT carries no
xpubs, the card reads "Load this wallet's descriptor to sign": load it
as step 3 does.

## 6. Sign your part and send it back

With the wallet loaded, press **Wallets**, the wallet, then **Spend
from this wallet**, and bring the PSBT in as
[Spend](spend.md#63-write-the-payment-and-bring-it-here) does. Read
the transaction ([Spend, step 6.4](spend.md#64-read-the-transaction)).

**Who signs** lists every key: `3dca6805` **Signs here**, `5d388376` and
`cf2e083d` **Not here**. A cosigner's key does not come here: they sign
the same PSBT on their own device. Press **Continue**, then **Sign with
3dca6805**.

*Picture to come: `multisig-with-others-who-signs.png`, Who signs with one key here and two cosigners Not here.*

**The other signatures** lists the keys again: `3dca6805` **Signed
here**, the others **Waiting**, and **No cosigner's copy in Files**.
Then either:

| Send yours to the coordinator | Collect a cosigner's here |
|---|---|
| Press **Show as QR** and scan it into the coordinator, or take the file whose name ends `-part.psbt` on a stick. The coordinator joins the signatures and broadcasts. | Press **Scan a copy** and scan the cosigner's signed PSBT, or copy it into Files on a stick visit and press the button "Add from" and its name. The key's row reads **Verified**, and **Send it** has the finished transaction ([Spend, step 6.7](spend.md#67-send-it)). |

*Picture to come: `multisig-with-others-other-signatures.png`, The other signatures with 3dca6805 Signed here and the others Waiting.*

A copy of another transaction is not added: each copy is checked
against its key and this transaction. Until enough keys have signed,
**Send it** reads **Needs 1 more signature**.

## 7. Keys kept off the device

A key does not have to stay loaded: its SeedQR is scanned when it is
needed, as on a SeedSigner. **Settings → Session → Remove every key**
takes every key out of memory and leaves the wallet.

![Settings, Session](../5-6-settings.png)

![The card with no key loaded](../5-6-card-short.png)

The card reads **0 of 2 signatures possible here**, each key **Not
here**, and **A vault on a stick, a paper backup or a SeedQR may hold
them**. Press **Scan a key** (or **Scan a key for New wallet 1** in the
left column).

![Scan a key for New wallet 1](../5-6-camera.png)

The sheet lists the keys still missing. Hold a SeedQR up to the camera.
Faraday works out which key it is; no choice is made first.

| After | Screen | Meaning |
|---|---|---|
| Key 2's SeedQR | ![Key 2 added · 1 more](../5-6-one-more.png) **Key 2 added · 1 more** | the camera stays on for the next key |
| Key 1's SeedQR, from Place 1's paper | ![Key 1 added · Can spend](../5-6-can-spend.png) **Key 1 added · Can spend**, in a bar at the foot of the page that goes by itself | enough keys to sign; the camera closes. Key 3 reads **Cosigner · xpub only**. |
| a key loaded already | **Key … · already here** | scan another |
| a seed of no key of this wallet | "… is not one of New wallet 1's keys", on the passphrase sheet | a passphrase that gives one of the missing keys loads it; otherwise **Cancel** |
| anything else | **Not a SeedQR or a CompactSeedQR** | scan the seed's SeedQR |

No SeedQR carries a passphrase: a key with a passphrase is typed with
**Type the words instead**, which also takes keys with no SeedQR, such
as the test keys' twenty-four words.

The pictures of this step show [Make a wallet](make-a-wallet.md)'s
**New wallet 1**, whose three keys are all yours.

> **With your own key**
>
> - Check each cosigner's fingerprint with that person directly, not
>   through the coordinator.
> - Check the first address on Faraday against the coordinator before
>   receiving to the wallet.
> - Take receive addresses from Faraday or from the coordinator, never
>   from a web page or a message.
> - Read every output and the fee on Faraday's screen before signing.
>
> Why: [Multisig](../../learn/09-multisig.md),
> [Xpubs and privacy](../../learn/11-xpubs.md),
> [Coordinator files](../../learn/19-coordinators.md),
> [Spend paths and timelocks](../../learn/10-spend-paths.md),
> [Verifying](../../learn/06-verifying.md).

Next: [codex32 shares](codex32.md).
