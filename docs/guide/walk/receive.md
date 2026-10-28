# Receive

**Kind:** Spec
**Status:** Text moved from `docs/GUIDE.md` (§5.4 and §6.1), written and checked against the app's screens and strings; the pictures are in `docs/guide/` and keep their names
**Open:** the walkthrough's form (result line, what is needed, time, "With your own key"); the pictures, drawn once after the theming has landed

### 5.4 Show the wallet to a watch-only wallet

A watch-only wallet, such as Sparrow on an online computer, shows the
balance and makes the transactions Faraday signs. It needs the wallet
description, which holds no key that can spend. Press **Show wallet
QR**.

![The wallet QR](../5-4-wallet-qr.png)

The sheet is **New wallet 1 · wallet descriptor**, **Public · safe to
scan**, with the descriptor written under the code. **UR**, **BBQr**
and **Parts** (numbered codes of plain text) are the formats, the one
chosen ticked; use the one the other wallet reads.
**PNG for the stick** puts the code on a stick as a picture.

1. In Sparrow: File, New Wallet, then scan the wallet QR.
2. Compare Sparrow's first receive address with **First receive
   address · 0/0** on the card, character by character.
3. Press **Done**.

| Result | Meaning |
|---|---|
| the addresses match | both programs derive the same addresses from the same description |
| they differ | something was scanned or chosen wrongly; do not receive to this wallet until they match |

A match does not prove the backups are right: that is the copy check
of section 4. The description shows the wallet's whole history to
whoever has it: do not post it.

### 6.1 Receive: check the address

Faraday never sends; it shows which addresses are the wallet's. In
Sparrow, open the Receive tab: it shows the next unused address and its
QR code. On Faraday, press **Scan** (at the foot of [Home](../screens/home.md), or **Tools →
Check an address**) and hold Sparrow's code up to the camera.

![An address of New wallet 1, found](../6-1-found.png)

| Line under the picture | Meaning | Do |
|---|---|---|
| "That address is New wallet 1's receive address 0" | the address derives from the wallet's keys, as Sparrow's does | give it to the payer |
| "That address is not among the first 100 of any loaded wallet" | no loaded wallet makes this address in its first 100 receive and change addresses | do not receive to it until the wallet is found |
| "That address is not on Mainnet" | the address is for another network | the same |
| "That address fails its checksum: a character in it is wrong" | the code was read but the address is not valid | scan it again |

The camera stays on for the next address; **Cancel** closes it.

Without a camera, compare the address character by character with the
card's **First receive address · 0/0** (receive address 0), as in step
5.4. For receive address 0 the wallet here reads `bc1q rfhg t7dv … 0246
6p`.

A match proves the address belongs to the wallet whose keys are loaded
here. It does not prove the payer sends to it: a payer copies the address
from Sparrow, and malware on that computer can change what is copied.
Compare what the payer received with what Faraday found.

> **With your own key**
>
> - Check the address on Faraday's screen, not only on the computer.
>
> Why: [Verifying](../../learn/06-verifying.md).
