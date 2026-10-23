# Faraday user guide

**Kind:** Spec
**Status:** Sections 1 to 3 written, checked against the app's screens and strings
**Open:** sections 4 to 7 (Back it up, A wallet, Receive and spend, Restore), planned

Each step is one thing to do, in the order it is done. A path in bold is
what the screen shows: **Wallets → Add a key** is the Wallets row in the
sidebar, then the Add a key tile. The pictures are the app's own
screens, drawn at 1280 × 800, the size Faraday Desktop opens at; a stick
on a larger screen draws the same pages wider.

The guide uses public test data only. The key made in section 3 is from
a published dice transcript, and its fingerprint is given to compare
against. Never keep funds on a key whose words are in this guide.

Sections 4 to 7 name places Place 1, Place 2 and Place 3.

## 1. Get Faraday on a PC

Faraday runs from a USB stick on a PC. Faraday Starter, an app for
Windows and Linux, writes the stick and helps start the PC from it.
Faraday Desktop (step 1.11) runs Faraday in a window on testnet, for
trying it out.

What is needed: a PC with a USB port, and a USB stick of 256 MB or
larger whose contents can be lost.

### 1.1 Download the Starter

From the release page, download `Faraday-Starter-X.Y.Z-Windows.exe` or
`Faraday-Starter-X.Y.Z-Linux.tar.gz`, and `SHA256SUMS` and
`SHA256SUMS.asc`. Check them as the README's "Verifying a release" says.

### 1.2 Open the Starter

- **Windows:** open the `.exe`. Windows shows "Windows protected your
  PC" the first time: press *More info*, then *Run anyway*.
- **Linux:** extract the `.tar.gz` and open the file inside.

The Starter opens on its Home.

![The Starter's Home with no stick attached](guide/1-1-starter-home.png)

| On screen | Meaning |
|---|---|
| **This computer** | the PC's maker and model, and **Secure Boot: on** or **off** |
| **Checklist** | **1 Make a Faraday stick**, then **2 Start this computer from the stick**; the step to do now is highlighted |
| the button on the card | the next step for this PC: here **Make a Faraday stick** |

Press **Make a Faraday stick**.

### 1.3 Which stick?

Plug the stick in. Every USB disk attached is listed with its size and
what is on it. Nothing is chosen for you.

![Which stick? with two sticks listed](guide/1-2-which-stick.png)

Press the stick to write. Check the size: a backup drive is listed the
same way as a stick. A disk the computer is using is dimmed and cannot
be pressed.

### 1.4 What to write?

![What to write? for an empty stick](guide/1-3-what-to-write.png)

Tick **New Faraday stick** (Everything on it is lost), then press
**Continue**. Continue stays grey until it is ticked.

A stick that already holds Faraday also offers keeping its data; that
is for upgrading, and is not covered here.

### 1.5 Confirm

![Confirm, before the stick is written](guide/1-4-confirm.png)

Check **Stick**, **Size** and **On it** against the stick in your hand,
then press **Erase and write**. On Windows, Windows asks for
administrator permission: answer *Yes*. On Linux the system may ask
for your password.

### 1.6 Writing, then Written

![Writing](guide/1-5-writing.png)

The Starter writes the image, then reads it back to check it. Leave the
stick in until the result shows.

| Result | Meaning | Press |
|---|---|---|
| ![Written](guide/1-6-written.png) | **Written**: the stick holds Faraday and was read back. **In the boot menu** is roughly the name the PC's boot menu will show for the stick. | **Start this computer from the stick**, with the stick still in |
| ![Not written](guide/1-6-not-written.png) | **Not written**: the reason is in red. Nothing on the stick can be trusted. | **Try again**; if it fails again, use another stick |

### 1.7 Start this computer from the stick

Leave the stick in. **Start this computer from the stick** on the
Written page, or item 2 of Home's checklist, opens this page:

![Start this computer from the stick](guide/1-7-start-page.png)

Under **Write down**, copy **Boot menu key**, **BIOS key** and **Secure
Boot setting**, or press **Print** for a sheet with them and the steps.
The PC restarts later and the Starter is then off the screen. For a PC
the Starter does not know, the rows read **Not known** and the printed
sheet lists the keys to try.

The stick cannot start while Secure Boot is on. Back on Home (**Home**
at the top left), the card's button shows which case this PC is in:

| Secure Boot on | Secure Boot off |
|---|---|
| ![Home with Secure Boot on](guide/1-7-home-stick-secure-boot-on.png) | ![Home with Secure Boot off](guide/1-7-home-stick-secure-boot-off.png) |
| Button: **Restart into the BIOS**. Go to step 1.8. | Button: **Restart from the stick**. Go to step 1.9. |

On Windows the first checklist line reads **Faraday stick attached ·
version not read**: the Starter reads a stick's version on Windows only
with administrator permission. Nothing here needs it.

### 1.8 Restart into the BIOS (Secure Boot on)

1. Press **Restart into the BIOS**, on Home's card or as step 1 of the
   Start page. A sheet asks first:

   ![Restart into the BIOS sheet](guide/1-8-restart-into-bios.png)

   The QR code opens the same steps on a phone. Press **Restart**.
2. In the BIOS, turn Secure Boot off, at the **Secure Boot setting**
   written down. Turn Fast Boot off too if it hides USB devices. If the
   setting cannot be found, exit the BIOS without saving changes:
   Windows starts as before.
3. Save and exit. As the PC restarts, press the **Boot menu key** and
   choose the stick (the **In the boot menu** name from step 1.6).

### 1.9 Restart from the stick (Secure Boot already off)

Press **Restart from the stick**.

![Restart from the stick sheet](guide/1-9-restart-from-stick.png)

Press **Restart**. On Linux the system asks for your password, then the
PC starts from the stick by itself. On Windows, Windows' own menu
opens: choose *Use a device*, then the stick.

### 1.10 Before starting Windows again

When done with Faraday, turn Secure Boot back on in the BIOS before
starting Windows (the Start page's step 4, **Turn Secure Boot back on
before starting Windows**). Faraday's power sheet says the same (step
2.3).

### 1.11 Faraday Desktop, the alternative

Faraday Desktop is Faraday in a window, for trying it. It is published
beside the Starter as `faraday-desktop-X.Y.Z-testnet-only-linux-x86_64`
and `faraday-desktop-X.Y.Z-testnet-only-windows-x86_64.exe`. It runs on
an online computer, so it has none of the stick's protections: never
type a seed that holds real bitcoin into it.

![Faraday Desktop's Home, in Demo](guide/1-11-desktop-home.png)

It opens in **Demo · Testnet**, the pill at the top right. Demo takes
only the public sample seeds; **Load samples** loads them. The pill
opens the **Mode** sheet, where **Testnet**, **Signet** and **Regtest**
take any seed.

Sections 2 and 3 apply to Faraday Desktop too, with the differences
noted there.

## 2. First boot and Home

### 2.1 Home

The stick starts Faraday into memory and shows Home. A new stick holds
no files, so nothing is offered to import.

![Home on a new stick](guide/2-1-home.png)

| Tile | Opens |
|---|---|
| **Make a new wallet** | a wallet from new keys, single key or multisig (section 5) |
| **Restore an existing wallet** | a wallet from its backups (section 7) |
| **Spend from a backup, step by step** | spending from a stick, words or paper (section 6) |
| **Create a vault** | an encrypted file for keys and secrets, kept on a stick (section 4) |
| **Stick visit** | the files on the stick, while a stick is attached and no key is loaded |
| **Scan** | the camera, for a QR code |

### 2.2 The sidebar

The rows are **Home**, **Vaults**, **Files**, **Wallets**, **Tools**,
**Learn**, **Stick visit** (while a stick is attached) and **Settings**.
At its foot:

| On screen | Meaning |
|---|---|
| **Bring in · Open · Work · Write out** | the stage of the session; the current one is in blue. Pressing it opens Files. |
| **Files** | files brought in, or **none** |
| **Sticks** | **Boot stick** while the stick Faraday started from is in; **none** once it is pulled |
| **Keys** | the keys loaded, each by its fingerprint (once a key is loaded) |
| **Lock** | wipes the session from memory (once a key is loaded) |
| the power button | the power sheet |

The **?** at the top right opens Learn's page for the screen on show.

### 2.3 Power off

Press the power button at the foot of the sidebar.

![The power sheet on a stick started with Secure Boot off](guide/2-3-power.png)

On a PC started with Secure Boot off, the sheet leads with **Turn Secure
Boot back on in the BIOS before starting Windows.** and the QR code of
the steps. With keys loaded it also lists what is kept and what is
wiped from memory. Press **Power off**, **Restart**, or **Cancel**.

Faraday Desktop has no power sheet: close the window.

## 3. New key

A key is a list of words (a BIP-39 seed). This section makes one on the
device, writes it down and checks the copy. It loads the key; a wallet
from it is section 5, and backing it up is section 4.

The test data: EntropyLab's published BitBox dice transcript for twelve
words (`tools/reference/dice/README.md` names the source). Its first 70
rolls are:

```
1234112341223412334123441234152341263412314123421234132341243412354321
```

They make these words, with fingerprint `3dca6805`:

```
brand hobby ranch shoulder brass hockey ranch short brand hockey random torch
```

Rolls of a real die make other words and another fingerprint.

### 3.1 Wallets → Add a key

Press **Wallets** in the sidebar, then **Add a key** (Keys first,
wallets later).

![Wallets with no wallet](guide/3-1-wallets.png)

### 3.2 Pull the stick

A key loads only with no stick attached. The sheet **Pull the stick to
add a key** opens while the boot stick is in.

![Pull the stick to add a key](guide/3-2-pull-the-stick.png)

Pull the stick out. Faraday keeps running from memory, and Add a key
opens. A reminder line can show on this sheet, such as **On a TV? Stop:
a smart TV can record its screen.**; act on it before going on.

Faraday Desktop shows this sheet only while one of its sticks is
plugged in.

### 3.3 Start

The page is **Add a key**, one card per step. A finished card shows its
choice and **Change**.

![Add a key, the Start card](guide/3-3-start.png)

**Make a new key** is ticked. Press **Continue**.

**Steps only** and **Guided**, at the top right, switch a short
walk-through on each card off and on.

### 3.4 Length

![The Length card](guide/3-4-length.png)

**BIP-39 words** and **12 words** are chosen. Press **Continue**.
(24 words takes more rolls; the steps are the same.)

### 3.5 Randomness

Choose where the randomness comes from.

| Dice (this guide) | This device |
|---|---|
| ![Randomness, dice chosen](guide/3-5-randomness-dice.png) | ![Randomness, this device chosen](guide/3-5-randomness-device.png) |
| **Dice · Words chosen by the dice** (70 rolls) is ticked, under **Your own entropy, words verifiable by hand**. Each word can be worked out from the rolls by hand. | Open **Made by this device** and tick **This device**. The words come from the operating system's generator and cannot be checked. |
| Press **Continue**. Go to step 3.6. | Press **Continue**. Go to step 3.6, right-hand column. |

The other options (flip mode, coins, hashed dice as Coldcard and
SeedSigner do it, hex, cards, camera, a mix) follow the same steps with
their own entry card.

### 3.6 Rolls

| Dice | This device |
|---|---|
| ![Rolls, 24 of 70 in](guide/3-6-rolls.png) | ![Generator](guide/3-6-generator.png) |
| Type the rolls into the box, or press **Buttons** to press each face. Each word appears as its six rolls come in. | The **Generator** card shows **Source**, **Trust** (this device's software) and **Result** (Not checkable). Press **Make the words**, then go to step 3.7. |

For this guide, type the 70 rolls above. The 70th roll ends the entry;
the word list shows **torch** as word 12 (its checksum filled in).

In a word's first five rolls only 1 to 4 count. A 5 or a 6 there is not
taken, and the card says so:

![A 5 refused in a word's first five rolls](guide/3-6-rolls-five.png)

Roll again and type the new roll.

With the last roll in, the card ends with **Randomness check**:

![The randomness check with every roll in](guide/3-6-rolls-done.png)

| Verdict | Meaning | Press |
|---|---|---|
| **Fair: no cautions** | the counts and runs look like a fair die | **Continue** |
| a caution, with **Roll again** | the rolls look uneven (for example a long run of one face); a caution does not stop the key | **Roll again** to start the rolls over, or **Continue** to keep them |

**Undo** takes the last roll back; **Clear** removes every roll.

### 3.7 Key: lock it in

![The Key card before Lock in](guide/3-7-key.png)

The 12 words are shown. Check them against the words above. Press **Lock
in**.

![The Key card after Lock in](guide/3-7-locked-in.png)

**Locked in** shows, with **Fingerprint** `3dca6805` and the key's
account key. The key is now loaded: the sidebar lists it under **Keys**
as `3dca6805 New key 1`. Press **Continue**.

In Faraday Desktop's Demo, Lock in adds a public sample in place of the
words made, and the card names the sample; the fingerprint is the
sample's, not `3dca6805`.

### 3.8 Words: write them down

![The Words card with the words shown](guide/3-8-words.png)

Press **Show words**. Write the 12 words on paper, numbered, in order.
Check the copy word by word, then press **Hide words** and **I wrote
them down**.

### 3.9 Quiz

The quiz asks for words by number, four choices each, until all 12
are right.

![The quiz](guide/3-9-quiz.png)

Answer from the paper copy, not from memory.

| Answer | Screen | Press |
|---|---|---|
| right | the next question; **0 of 12 right** counts up | the next answer |
| wrong | ![A wrong answer](guide/3-9-quiz-wrong.png) **Not that one. Check your copy against the words** | **Show the words** to correct the copy, then **Ask it again** |
| all right | ![The quiz passed](guide/3-9-quiz-passed.png) **All 12 words right** | **Done** |

**Skip the quiz** asks once more before it skips.

### 3.10 After Done

![Wallets after Done](guide/3-10-after-done.png)

Faraday returns to Wallets. The key is loaded and shows in the sidebar
under **Keys**. It is in memory only: Lock or Power off wipes it, and
the paper copy is then the only one.

Next: back the key up (section 4), or make a wallet from it (section 5).

## 4. Back it up

Planned.

## 5. A wallet

Planned.

## 6. Receive and spend

Planned.

## 7. Restore

Planned.
