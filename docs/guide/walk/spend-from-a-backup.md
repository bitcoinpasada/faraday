# Spend from a backup, step by step

**Kind:** Reference
**Status:** Written, checked against the app's screens and strings, and pictured from `just guide-pictures` (`docs/guide/pictures/`)

**You end with:** a spend signed from what one place's envelope holds,
with nothing loaded beforehand, and everything sealed and put back.

**Before you start:** [Back it up](back-up.md), which leaves Place 1's
envelope. [Spend](spend.md) has the same spend from a wallet already
loaded.

**You'll need:**

- The Faraday stick, and the PC it starts. Faraday freshly started:
  nothing loaded.
- Place 1's envelope from [Back it up](back-up.md): the stick with
  Vault 1 (`vault.fvlt`) and its passphrase, the 12 words and the SeedQR
  on paper, and the wallet sheet.
- Sparrow on an online computer, and a camera on the PC that runs
  Faraday.

**Time:** about 40 minutes.

[Home](../screens/home.md)'s **Spend from a backup, step by step** is for someone spending
for the first time from what a backup left. It is one column of cards,
from what is in the envelope to the envelope packed again, each with a
line on what to do and **More about this** for the rest. It uses the
same controls as the rest of Faraday: a spend started here shows in
Wallets too, and the signatures are the same.

## 1. Open it

On **Home**, press [**Spend from a backup, step by step**](../screens/spend-tab.md). The page is
**Spend**, with a note at the top: this page takes someone spending
for the first time through every step. The link at the top left goes
back to **Home**.

With a wallet or a seed already loaded, Spend opens on a shorter page
instead: each wallet loaded, how many of its keys are here, and
**Spend from it**. **Walk through from the start** opens the column
below.

## 2. How to spend bitcoin

![Spend with its note at the top and nine cards, from How to spend bitcoin to Put everything away](../pictures/spend-from-a-backup-map.png)

The page lists its cards closed, each with a word on where it stands:
**How to spend bitcoin**, **What do you have?**, **Check the money is
really there**, **Write the payment**, **Bring the transaction here**,
**Read the transaction**, **Sign it**, **Send it** and **Put everything
away**.

The first card is the map: six stops, each marked **Paper**,
**Offline** or **Online**, from **What you have** to **Send it**. Three
happen on this computer and three on the online one. Under it, one
line is always shown: the words are the money; never photograph them
or type them into a phone, a website or a password manager.

Press **Start**.

## 3. What do you have?

The card begins with what starting from the stick means (no network,
everything forgotten at power off), then: scan or type everything in
the envelope, one thing at a time, in any order. Faraday works out the
wallet from it.

![What do you have? with nothing in yet, Copy files in from a stick grey until a stick is in](../pictures/spend-from-a-backup-what-do-you-have.png)

| On the card | Does |
|---|---|
| **Scan the QR codes you have** | the camera, titled **Scan what you have**: reads one code after another until **Done** |
| **Type the words you have** | **Add a key**, which returns here once the key is in |
| **Copy files in from a stick** | the stick visit, for a vault or a file on a stick. Grey until a stick is in (**Plug the stick in**) |
| **Not sure what you have?** | a list of what each thing in an envelope looks like, and what it is |

For this walkthrough, bring the vault in:

1. Plug in Vault 1's stick and press **Copy files in from a stick**.
   The [stick visit](../screens/stick-visit.md) lists `vault.fvlt`, ticked. Press **Import 1 file**,
   then pull the stick.
2. Back on the card, **Unlock vault.fvlt** heads the vault's row: its
   name, **Copied in from a stick** and its size, and how long it
   takes to unlock ("Unlocks in about 0.5 s", with its memory and
   passes). **Or bring in what is on paper** keeps the other ways in
   under it.
3. Type the vault's passphrase under **Passphrase** and press
   **Unlock**; beside it, the memory this computer has free. While a stick is attached the field reads **Remove the
   stick, then type the passphrase**.

![Unlock vault.fvlt with the passphrase typed, Unlock outlined](../pictures/spend-from-a-backup-unlock.png)

The vault opens on its list, headed "vault.fvlt · Vault · open":
**Wallets with their seeds**, with **New wallet 1** (**1 of 1 key in
this vault**) and **Ready to sign once loaded**; "Each seed and wallet"
with a count lists what the vault holds. Press **Load wallet with
keys**.

![What do you have? with vault.fvlt open, New wallet 1 Ready to sign once loaded, Load wallet with keys outlined](../pictures/spend-from-a-backup-vault-open.png)

The wallet opens and the column moves on by itself to the next card;
**What do you have?** closes on "New wallet 1 · Single key · native
SegWit". Back on this card,
**Spend from** lists each wallet loaded with how many of its keys are
here; pressing one chooses it. **Start over** forgets what came in.

The other ways in, from the same envelope:

| Brought in | Card | Then |
|---|---|---|
| The wallet sheet's QR code, scanned | **Wallet description read**; the wallet opens | scan or type the seed as well, before signing |
| The SeedQR, scanned | the sheet **SeedQR read**: press **Use without a passphrase**. The camera comes back with **Seed 3dca6805 read**, and **What came in** lists the seed by its fingerprint | the card asks **Anything else in the envelope?**: **Scan more**, **Type more words** or **That's everything** |
| The 12 words, typed with **Type the words you have** | **Add a key**: type them, **Continue**, **Lock in**; the card comes back with the seed under **What came in** | the same question |

**That's everything** opens a single-key native SegWit wallet from the
seed. Words alone do not say which kind of wallet they made, so the next
card tries each kind in turn (step 4).

## 4. Check the money is really there

![Check the money is really there with Show the wallet as a QR code and three Receive and three Change addresses, Yes, my money shows outlined](../pictures/spend-from-a-backup-check.png)

| On the card | Does |
|---|---|
| **Show the wallet as a QR code** | the wallet's description, for Sparrow: in Sparrow, File, New Wallet, then scan it |
| **Receive** and **Change** | the wallet's first three addresses on each chain; **Receive** 0 is the first address Sparrow shows |
| **Show more addresses** | five more on each chain |
| **Yes, my money shows** | the next card |

Load the wallet in Sparrow and compare its first receive address with
Receive 0, character by character. They match: press **Yes, my money
shows**. They differ: stop; something was scanned or typed wrongly.

With a wallet made from seeds alone, the card is **Check the money is
there, on the online computer**, with the kind tried named over the
addresses, and **No money shows**:

| Press | Then |
|---|---|
| **Yes, my money shows** | the next card |
| **No money shows** | the next kind replaces the wallet, the one tried struck through above |
| **Is there a word or sentence kept apart from the list?**: **Yes, type it** or **No** | asked once, for one key: a passphrase for the seed, then the same kind again |
| after every kind, **None of these show money.**: **Scan more** or **Type a passphrase** | back to **What do you have?**, the camera or the seed's passphrase |

## 5. Write the payment

On the online computer, in Sparrow's Send tab: the address to pay, the
amount and a fee. Then *Create Transaction*, *Finalize Transaction for
Signing* and *Show QR*. Press **I have the PSBT**.

## 6. Bring the transaction here

![Bring the transaction here with Scan, and Copy it in from a stick grey until a stick is in, Scan outlined](../pictures/spend-from-a-backup-bring.png)

| By QR code | On a stick |
|---|---|
| Press [**Scan**](../screens/qr-and-scan.md#scan) and hold Sparrow's code up to the camera; an animated code is read as it loops. | Save the `.psbt` onto a stick and press **Copy it in from a stick**, grey until the stick is in. With the vault open, Faraday reads a stick only after a lock: plugging it in asks to lock (**Faraday asks to lock first when a stick goes in**). The lock seals the vault and wipes the keys; copy the PSBT in, pull the stick, and unlock with the same passphrase. The column comes back on this card. |

A PSBT already in Files is listed under **In Files**, with **Use this
one**. Once one is in, the card reads **Spends from New wallet 1 ·
Single key · native SegWit**. Press **Read it**.

## 7. Read, sign and send

The spend's own cards follow, as in [Spend](spend.md):

| Card | Do |
|---|---|
| **Read the transaction** | read what leaves, what comes back and the fee, and compare them with what was typed into Sparrow ([Spend, step 6.4](spend.md#64-read-the-transaction)); then **Continue** |
| **Sign it** | press **Sign with 3dca6805**. Signing cannot be taken back |
| **Send it** | **Show the finished transaction as QR**, or the signed PSBT's **Show as QR**; in Sparrow, *Scan QR*, then *Broadcast Transaction* ([Spend, step 6.7](spend.md#67-send-it)) |

![Send it with the signed PSBT and the finished transaction for the stick, and the nonce check, the PSBT's Show as QR outlined](../pictures/spend-from-a-backup-send.png)

A wallet of several keys adds **Who signs** before **Sign it**: each
key **Signs here** or **Not here**, with **Scan a key** and **Unlock**
for another vault in Files ([Multisig with other people and
devices](multisig-with-others.md), step 6). When the wallet came from
its description, that card is **Add the seeds that can sign**: one seed
at a time, typed or scanned.

The txid Sparrow reports after broadcasting must be the one Faraday
showed. If it differs, what was sent is not what was signed here.

## 8. Put everything away

![Put everything away listing scanned-1-signed.psbt and scanned-1-final.txn, with Lock, Power off and Start over, Lock outlined](../pictures/spend-from-a-backup-away.png)

The card lists the files waiting for the stick, then **Lock**, **Power
off** and **Start over**.

1. Press **Lock**. The sheet **Lock** lists what is **Kept** and what
   is **Wiped from memory**; press **Lock** on it (**Back to the
   vault** leaves everything open). The vault is sealed again with the keys inside it,
   and no key is left in memory.
2. Signing with a vault open records the amounts signed inside it, so
   the lock leaves a changed copy of the vault for the stick. Plug Vault
   1's stick in: the [stick visit](../screens/stick-visit.md) writes what waits. The stick
   already holds `vault.fvlt`, so the visit asks: **Replace** writes the
   changed vault over it and keeps that record; **Keep both** writes it
   beside it as `vault-2.fvlt`; leaving it out changes nothing else.
3. Press **Power off**, take the stick out, and put it back in the
   envelope with the papers.

> **With your own key**
>
> - Read every output and the fee on Faraday's screen before signing.
> - Compare Receive 0 with Sparrow's first address before trusting the
>   balance Sparrow shows.
>
> Why: [Inheritance](../../learn/12-inheritance.md),
> [Verifying](../../learn/06-verifying.md),
> [Mistakes and scams](../../learn/08-scams.md),
> [Vaults](../../learn/faraday/vaults.md).

Next: [Vaults, passwords and codes](vaults-and-passwords.md).
