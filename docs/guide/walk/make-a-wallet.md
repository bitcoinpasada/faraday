# Make a wallet, single key or multisig

**Kind:** Reference
**Status:** Written, checked against the app's screens and strings, and pictured from `just guide-pictures` (`docs/guide/pictures/`)

**You end with:** three keys loaded and a 2-of-3 multisig wallet,
`New wallet 1`, made from them, its card read and its backup planned.

**Before you start:** [Make a key](new-key.md) and
[Back it up](back-up.md).

**You'll need:**

- Faraday with no key loaded (press **Lock** if one is) and no stick
  attached.
- The paper copy of key `3dca6805` from section 4.
- Two more public test keys: twenty-four times **zebra**
  (fingerprint `5d388376`) and twenty-four times **summer**
  (`cf2e083d`).
- Sparrow, or another watch-only wallet, on an online computer, for
  [Receive](receive.md) next.

**Time:** about 20 minutes.

The places of this section are Place 1, Place 2 and Place 3.

### 5.1 Load the keys

Press **Wallets**, then **Add a key**. The page of section 3 opens;
press **Change** on **Start**, then **Type the words**.

![Add a key with Start on Type the words and the Words card open, 12 chosen](../pictures/5-1-type-words.png)

| On screen | Meaning |
|---|---|
| **Start** **Type the words** | the choice made; **Change** opens Start again |
| **Words** **12 · 15 · 18 · 21 · 24** | how many words, 12 ticked; typing past the boxes adds more |
| **Other languages** | word lists other than English |
| **0 words** | how many words are typed |
| **Continue** | checks the words and opens **Key** |

Start's other choices load a key the same way: **Scan a SeedQR** reads
one with the camera, **Type codex32 strings** opens the codex32 form,
and **Other forms** lists SLIP-39 shares and Seed XOR parts. A key
scanned from [Home](../screens/home.md)'s **Scan**, or typed for Restore (section 7), opens
**Add a key** with the forms as tabs: **BIP-39 words**, **SLIP-39
shares**, **codex32** and **Seed XOR parts**. On **codex32** each
string typed or scanned is checked at once. This section needs none of
it; for codex32 strings in full, see Learn's page **Backups in other
forms**.

| A string in | Check a string |
|---|---|
| ![Add a key · codex32 with one string in, Make the rest outlined](../pictures/5-1-codex32.png) | ![Check a string with Checksum fails and the final row, Show the worksheet outlined](../pictures/5-1-check-a-string.png) |
| **1 in** and **Share A · id name · 2 needed**. Once 2 strings of the set are in, the key loads. **Make the rest** makes the set's other strings from the ones typed; **Add a share** makes one more string at an unused index. Where an open vault keeps strings of the same set, the line **1 more of this set in open vaults** says so, and they count. | **Check a string**, then **Check it**: **Checksum holds** or **Checksum fails**, with the checksum worksheet's final row and, where it is not `SECRETSHARE32`, its cells marked and the first box named (**Final row differs at box 36**); **Show the worksheet** shows every row, **Final row only** the last again. Nothing is loaded or kept. **Back to adding strings** returns. |

| The worksheet | Make the rest |
|---|---|
| ![The string's checksum worksheet, Final row only outlined](../pictures/5-1-worksheet.png) | ![New key on Scheme: From A, C, Length 48, Threshold 2, Strings 3](../pictures/5-1-make-the-rest.png) |
| **Show the worksheet** shows every row of the string's checksum worksheet as the booklet draws it, each box of the string numbered in its corner, to compare row by row with the paper; drag it, or use the wheel or the arrow keys, to see the rest. **Final row only** goes back. | **Make the rest**, then the second string: **New key** opens on **Scheme**: **From** A, C, **Length** 48, **Threshold** 2, and **Strings** to choose. **Worksheets**, **Strings** and **Strings made** follow, as in step 4.11. **Add a share** opens the same way on **Index**, offering the next unused one. |

With **Suggest corrections** ticked, a string whose checksum fails
gets a correction to compare with the paper; Learn's page covers it.
A key loaded from corrected strings keeps a line beside its
fingerprint, **Share entry corrected** (**I mistyped**) or **Share
repairs unchecked**:

| Add a key's Key card | Wallets |
|---|---|
| ![Add a key's Key card listing Share A and Share C, Lock in outlined](../pictures/5-1-key-card.png) | ![Wallets with key fab6868a under Keys without a wallet, marked Share entry corrected](../pictures/5-1-wallets-mark.png) |
| From **Type codex32 strings**, **codex32** reads **2 in** and **Key** lists the strings in (**Share A · id name · 2 needed**, **Share C · id name · 2 needed**). **Lock in** loads the key and opens Wallets. | Under **Keys without a wallet**, and on the key's card (**Key fab6868a is loaded. No wallet uses it.**): **fab6868a · Share entry corrected**. |

Type key `3dca6805`'s 12 words from the paper, then press **Continue**.

![The Key card with the 12 words hidden and No passphrase, Lock in outlined](../pictures/5-1-words-key.png)

**Key** shows the 12 words hidden (**Show words** shows them), with
**Passphrase** and **Passphrase again** (**No passphrase** unless one
is typed in both). Press **Lock in**.

![Wallets with no wallet and key 3dca6805 under Keys in the sidebar](../pictures/5-1-key-loaded.png)

Faraday returns to Wallets, and the sidebar lists `3dca6805` under
**Keys**. The fingerprint proves the words typed are the test words.
Press **Add a key** again, **Change** on Start and **Type the words**,
press **24**, type **zebra** twenty-four times, **Continue** and **Lock
in**; then the same with **summer**. The sidebar lists three keys:
`3dca6805`, `5d388376`, `cf2e083d`.

### 5.2 Make a new wallet

Press **Home**, then [**Make a new wallet**](../screens/make-a-wallet.md).

| Kind | Quorum |
|---|---|
| ![Kind opened again with Multisig · native SegWit chosen, that row outlined](../pictures/5-2-kind.png) | ![Quorum at 2 of 3, Account 0, Continue outlined](../pictures/5-2-quorum.png) |
| Kind opens with **Single key, backup in shares (codex32)** ticked; **More kinds** lists the others. Press **Multisig · native SegWit** (**3 keys, any 2 sign**): **Quorum** opens. **Change** on Kind, as in the picture, shows it chosen; pressed again, it moves on. | **Signatures needed** 2 and **Keys** 3 (**-** and **+** change them). The line under them says what the quorum means: **Any 2 of the 3 keys can spend. Losing 1 key still leaves the money spendable; 2 keys together can take it.** **Account** 0. Press **Continue**. |

On **Use vaults**, press **No**: **Keys** opens.

| Keys | Check |
|---|---|
| ![Keys with all three keys chosen, Make the wallet outlined](../pictures/5-2-keys.png) | ![Check with the descriptor, the path and the wallet's first addresses](../pictures/5-2-check.png) |
| Each **Key** row offers the keys loaded. Press `3dca6805` for Key 1, `5d388376` for Key 2, `cf2e083d` for Key 3; the row reads `3dca6805 · 3dca6805 · loaded here`, and **Clear** empties it. **Show xpub QR**, **Xpub PNG for the stick** and **Xpub file for the stick** give a key's public part to another wallet; **Add a passphrase** opens **Passphrase** for the key; **New key** makes one here. Press **Make the wallet**. | **Descriptor** reads **2 of 3 · native SegWit multisig · 3dca6805, 5d388376, cf2e083d** and a checksum; **Path** **m/48'/0'/0'/2'**. **The wallet's first addresses**: **Receive 0/0**, **Receive 0/1** and **Change 1/0**. With these test keys, Receive 0/0 begins `bc1q rfhg t7dv` and ends `ecxs 0246 6p`. Press **It matches**. |

Then **Back up** (step 5.5).

### 5.3 The wallet card

Press **Wallets**, then the wallet in the left column
([the card](../screens/wallets.md#the-card)).

![The card of New wallet 1, Not backed up, Can spend, Show wallet QR outlined](../pictures/5-3-card.png)

| On screen | Meaning |
|---|---|
| **New wallet 1**, **2 of 3 · native SegWit multisig** | the name and the kind; **Rename** changes the name; **New wallet from this one** opens Make a new wallet filled with its kind, quorum and keys |
| **Made here** | the wallet was made on this device |
| **Not backed up** | no backup plan yet (step 5.5) |
| **2 of 2 signatures possible here**, **Can spend** | the quorum is 2, and 2 of the keys that sign are loaded |
| **At a glance** | the wallet, each key (**Can sign here** while loaded), and the backup (**No backup plan** and **Back up**) |
| **First receive address · 0/0** | the address to compare (step 5.4) |
| **Spend from this wallet** | section 6 |
| **Show wallet QR** | the wallet description as a QR code ([Receive, step 5.4](receive.md#54-show-the-wallet-to-a-watch-only-wallet)) |
| **Back up** | the backup board (step 5.5) |
| **Remove from session** | takes the wallet out of memory |

### 5.5 Back it up

Press **Back up** on the card. The board opens with the standard way
for a 2-of-3: one place per key, each keeping one seed's words and the
whole description.

![Back up New wallet 1: three places, each keeping one seed's words and the whole description on paper, Customize backup outlined](../pictures/5-5-board.png)

Each place reads **Sees the balance** and **Spends with 1 more place**.
The three lines read **Any one place lost: the rest rebuild the wallet**
**Yes**, **One place found: can spend** **No** and **One place found:
sees the balance** **Yes**. **Add watch-only software** adds the
watch-only wallet as a place that keeps the description.

After **Customize backup**, a drop that changes what a place alone can
do [asks first](../screens/back-up-wallet.md#keep-it-or-take-it-off):

| Seeing the balance | Spending |
|---|---|
| ![Place 3 can see the wallet's balance, Take it off outlined](../pictures/5-5-sees.png) | ![Anyone with access to Place 1 can spend, Take it off outlined](../pictures/5-5-spend.png) |
| **Place 3 can see the wallet's balance**: what was dropped there completes the description at that place (here **Share 1 and seed cf2e083d together make the whole description.**). | **Anyone with access to Place 1 can spend**: Place 1 now keeps as many seeds as a spend needs (**Place 1 keeps 2 of the 2 seeds a spend needs.**). |
| **Keep it** keeps the drop; **Take it off** undoes it. | the same |

The rest is section 4: the checklist, a copy checked per seed, any
vaults, and the sticks, each place in its turn.

> **With your own key**
>
> - Check the first address on Faraday against the watch-only wallet
>   before using either
>   ([Receive, step 5.4](receive.md#54-show-the-wallet-to-a-watch-only-wallet)).
> - Take receive addresses from Faraday or from the watch-only wallet,
>   never from a web page or a message.
> - For a multisig wallet, check each cosigner's fingerprint with that
>   person directly, not through the coordinator.
>
> Why: [Verifying](../../learn/06-verifying.md),
> [Multisig](../../learn/09-multisig.md),
> [Coordinator files](../../learn/19-coordinators.md).
