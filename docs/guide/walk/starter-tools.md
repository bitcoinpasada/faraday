# The Starter's other tools

**Kind:** Spec
**Status:** Written and checked against the app's code and strings; the pictures are named and not yet drawn
**Open:** the pictures, drawn once after the theming has landed

**You end with:** a file sent from this computer to Faraday by QR code
and one received back; a blank template printed and saved as a PDF; a
Faraday stick brought to the Starter's release with its data kept; and
where the rest of the Starter is.

**Before you start:** [Faraday Starter](starter-tour.md),
[Get Faraday on a stick](get-faraday.md) and
[Files, sticks and QR codes](files-and-sticks.md) (Faraday's end of a
QR code).

**You'll need:**

- Faraday Starter, open on a computer running Windows or Linux, with a
  camera for step 3.
- Faraday started from the stick on another computer, with a camera,
  beside this one.
- A printer, or a PDF viewer, for step 4.
- For step 5, a Faraday stick of an earlier release.

**Time:** about 30 minutes.

The Starter runs on a computer that is online. Nothing here reads a key
or a wallet: Transfer carries files, a blank template holds nothing of
any wallet, and keeping a stick's data leaves its data partition
unread.

## 1. Tools

*Picture to come: `starter-tools-tools.png`, the Starter's Tools with the Transfer and Blank templates cards.*

Press **Tools** on the rail. It holds two cards
([Faraday Starter](starter-tour.md#4-tools)):

| Card | Line | Button |
|---|---|---|
| **Transfer** | **Send and Receive** | **Open Transfer** (steps 2 and 3) |
| **Blank templates** | **Words, SeedQR and codex32** | **Open blank templates** (step 4) |

## 2. Send a file to Faraday

*Picture to come: `starter-tools-send.png`, Transfer with payment.psbt in Downloads and its size.*

Press **Open Transfer**. **Transfer** opens as a page of its own; the
way back at the top left returns to **Tools**. It has two parts,
**Send** and **Receive**.

**Send** lists this computer's Downloads folder, its path at the top,
the newest file first, each with its size. Put the file to send there,
for example the PSBT of [Spend](spend.md), `payment.psbt`; a file
dropped on the window is sent the same way.

Press the file. "Reading payment.psbt" shows while it is read, then the
file shows as a [QR code](../screens/qr-and-scan.md#a-qr-code): one
code, or a run of codes that repeats for a larger file. On Faraday,
press **Scan** on Home and hold the camera to this screen until the
whole file is read: it lands in Files
([Files, sticks and QR codes](files-and-sticks.md#6-qr-codes)).

| Send shows | Meaning |
|---|---|
| **No files** | Downloads is empty |
| **Over 256 KiB**, greyed | the file is too large for QR codes: carry it on a stick |
| "… is empty" | the file holds nothing |

## 3. Receive a file from Faraday

*Picture to come: `starter-tools-receive.png`, Transfer's Receive with a line "Saved …" under its buttons.*

On Faraday, show the file as a code: in Files, the file's **Show as
QR** ([Files, sticks and QR codes](files-and-sticks.md#6-qr-codes)),
for example the signed `payment.psbt`.

On the Starter, under **Receive**, press **Scan with the camera**. The
camera opens as **Receive into Downloads**. Hold Faraday's screen to it
until the whole file is read: "Saving …", then the camera stays open
for the next. Close it when done.

Under the buttons, a line per file this session, the newest first:

| Line | Meaning |
|---|---|
| "Saved …" | the file is in Downloads, at the path named. A file of the same name already there is kept, and the new one is saved beside it, "(2)" added to its name |
| "Saved …: this file holds a secret" | in the warning tone: saved only after [Save without encryption?](../screens/files.md#save-without-encryption)'s hold. It is on an online computer: delete it once it has served |
| "Not saved: …" | and why |

**Open folder** opens Downloads in this computer's file manager.
Nothing received goes into Faraday Demo's Files: Transfer is a pipe
between Downloads and the device.

## 4. Blank templates

*Picture to come: `starter-tools-templates.png`, Blank templates with Form, Words, Copy and Places, then Print and Save the PDF.*

A blank template is the sheet a seed is copied onto by hand. It is
printed here, on the computer with the printer, and holds nothing of
any wallet. Faraday's backup checklist asks for it by its words and
copy ([Back it up](back-up.md#46-the-checklist)).

Press **Open blank templates**. **Blank templates** lists its choices,
each a row with its value at the right; a row pressed opens a page of
its options, with **Continue** back.

| Row | Options | At first |
|---|---|---|
| **Form** | **BIP-39 words**, **codex32** | **BIP-39 words** |
| **Words** | **12**, **24** | **24** |
| **Copy** | **Words**, **SeedQR**, **Both** (the word lines, the SeedQR grid, or both on one sheet) | **Words** |
| **Places** | **Fewer** and **More**: one or more, filled in by hand | 2 |

For [Back it up](back-up.md)'s template: **Words** → **12** →
**Continue**. Then:

| Button | Does |
|---|---|
| **Print** | opens the PDF in this computer's PDF viewer. Print from there, at the sheet's actual size, not fitted to the page, so the SeedQR grid's squares keep their size |
| **Save the PDF** | saves it in Downloads, here `faraday-blank-template-12-words-words.pdf`: "Saved …", with **Open it**. A second save is named with "(2)" |

With **Form** on **codex32**, the rows are **Length**, **Copy**,
**Worksheets** and **Places**:

| Row | Options |
|---|---|
| **Length** | "48 characters", "74 characters", "127 characters" |
| **Copy** | **String**; at 48 characters also **Codex32QR**, **Both** and **Four Codex32QR grids** |
| **Worksheets** | **None**; except at 127 characters, **Dice, checksum and translation** |

A length that no longer allows a choice puts it back to **String** or
**None**. [codex32 shares](codex32.md) asks for 48 characters:
`faraday-blank-template-codex32-48-chars-string.pdf`.

## 5. Replace Faraday, keep the data

*Picture to come: `starter-tools-keep-data.png`, What to write? on a Faraday stick, Replace Faraday, keep data chosen.*

A new release reaches a Faraday stick from the Starter: Faraday itself
writes no boot partition. **Replace Faraday, keep data** writes the new
Faraday over the stick's boot partition and leaves its data partition,
with its vaults and settings, unread and unwritten.

1. Press **Make**, then **Make a Faraday stick**, as in
   [Get Faraday on a stick](get-faraday.md#13-which-stick).
2. **Which stick?** lists the stick as **Faraday stick ·** and its
   version. Press it.
3. **What to write?** offers **New Faraday stick** and **Replace
   Faraday, keep data** (**Only the boot partition is written**). Press
   the second. Under it: "This computer can read and change the data
   partition while the stick is attached.", and a link to the Learn page
   **Replacing Faraday on a stick**.
4. **Boot file**: **The release's (unsigned)**, or **One I signed**
   for a boot file signed with your own db key
   ([Your own Secure Boot keys](secure-boot-keys.md#10-the-starter-writes-the-signed-boot-file)).
   Press **Continue**.
5. **Confirm**: **Writes** reads **Replace Faraday, keep data**, with
   **Boot file**, **Build** and **SHA-256 of the image**. Check the
   stick, then press **Write**.
6. **Writing**, then **Written**, as in
   [Get Faraday on a stick](get-faraday.md#16-writing-then-written).

| Shown | Meaning |
|---|---|
| **Already this build** | the stick holds this release already; nothing to write |
| **Reading the stick's version** | on Windows, the version is being read; **Continue** waits |
| "This stick holds a newer Faraday: …" and **Only to go back to an older release.** | on Confirm: the Starter is older than the stick. **Write** is then a hold, **Hold to write the older release** |
| **The image is a boot partition alone** | under **New Faraday stick**, which cannot be chosen: the image this Starter writes is a boot partition alone, which only keeping the data can write |

Start the computer from the stick: Vaults lists the same vaults as
before.

## 6. A Pi card

*Picture to come: `starter-tools-pi-card.png`, Which card? with an SD card listed under the boards line.*

On **Make**, the **Pi card** card shows the Pi image's **Version …**,
or **Not in this Starter**, when **Make a Pi card** is grey.

Press **Make a Pi card**. **Which card?** opens with what the card is
for: "For a Raspberry Pi 3B, 3B+ or Zero 2 W with the Waveshare 2.8
inch screen" and "Cameras: a USB webcam, or Camera Module v1 or v2".
Then the steps are those of a stick: the card, **What to write?**
(**New Faraday card**), **Confirm** (**Card**), **Erase and write**,
**Writing**, and **Written**: "The card can be pulled out." With no
card attached: **No SD card or USB stick attached · 256 MB or larger**.

This guide does not cover the Pi itself.

## 7. The rest of the Starter

| What | Where |
|---|---|
| **Install**, **Update**, **Uninstall** | the top bar and the **⋮** menu ([Faraday Starter](starter-tour.md#6-install-update-appearance-and-the--menu)) |
| **Appearance**, **About** | the **⋮** menu (the same step) |
| **Remove the Faraday boot entry** | Home's **Boot entry** card, and the **⋮** menu, on Linux while the entry is in the boot list (the same step) |
| **Faraday Demo** | Home ([Faraday Demo](faraday-demo.md)) |
| **Learn** | the rail ([Faraday Starter](starter-tour.md#5-learn)) |
| **Start this computer from the stick** | Home's checklist ([Start a PC from the stick](start-from-the-stick.md)) |

> **With your own key**
>
> - Transfer only what Faraday shows as public: PSBTs, signed
>   transactions, wallet records. A key's words never go to this
>   computer.
> - Never photograph or scan a template once a seed is written on it.
>   Print it on a printer you trust.
> - USB sticks are not very reliable for long-term storage: name-brand
>   sticks are generally better, and cheap off-brand ones should not be
>   relied on to keep a backup for years. Keep a paper or metal backup
>   as well.
> - Before keeping a stick's data, check the Starter file as **About**
>   says. Plug the stick only into a computer you trust: while it is
>   attached, the computer can read the settings file and change
>   anything on the data partition.
>
> Why: [The air gap](../../learn/07-air-gap.md),
> [Backups](../../learn/04-backups.md),
> [Verifying](../../learn/06-verifying.md),
> [Replacing Faraday on a stick](../../learn/faraday/boot/replace-faraday.md).

Back to [the guide's index](../../GUIDE.md).
