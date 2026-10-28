# Your own Secure Boot keys

**Kind:** Spec
**Status:** Written and checked against the app's code and strings; the pictures are named and not yet drawn
**Open:** the pictures, drawn once after the theming has landed

**You end with:** your own Secure Boot keys, PK, KEK and db, made in a
vault kept on two sticks at two places; their enrolment files on a
stick; release 0.1.8's boot program signed with your db key; and a
Faraday stick that Faraday Starter wrote with that signed boot file, to
start with Secure Boot on where your db key is enrolled.

**Before you start:** [Vaults, passwords and codes](vaults-and-passwords.md)
(creating and unlocking a vault), [Files, sticks and QR codes](files-and-sticks.md)
(a stick visit), [Get Faraday on a stick](get-faraday.md) and
[Start a PC from the stick](start-from-the-stick.md).

**You'll need:**

- Faraday started from the stick, with no stick attached but the
  Faraday stick.
- Two USB sticks other than the Faraday stick, formatted FAT32, one for
  each place the keys are kept at.
- A phone or watch showing the time to the minute: the certificates and
  the enrolment files carry the time they were made.
- A computer with Faraday Starter and the release's files:
  `faraday-0.1.8-BOOTX64.EFI`, `SHA256SUMS` and `SHA256SUMS.asc`.
- For enrolling: a computer whose firmware setup takes keys of your own,
  and, where Windows uses BitLocker, its recovery key.

**Time:** about 45 minutes, and the computer's firmware setup.

The owner name, release and passphrases on this page are examples only.
Secure Boot keys cannot be made again from seed words, so Faraday makes
them inside the backup: planned at two places or more, written to each,
and used for nothing that signs until every place is written. In
Faraday Demo, Practice makes no Secure Boot keys: **Make the key** does
nothing there ([Faraday Demo](starter-tour.md)).

Faraday never writes a setting of the computer: the keys are enrolled
in the firmware's own setup screen, from the files Faraday writes to a
stick.

## 1. Start

Press **Tools**, then **Secure Boot keys** (**PK, KEK and db; sign an
EFI image**). With no vault open, the backup flow opens at once. With a
vault open, **Vault contents** opens on **Secure Boot** instead: press
**Make keys** there ([Vault contents](../screens/vault.md)). An empty
vault offers **Make Secure Boot keys** too.

The flow's cards are those of [A GPG key](gpg.md#1-start): **Where it
will be kept**, **Bring in the sticks**, **Vaults**, **Make the key**,
**Lock**, **Write the sticks** and **Done**.

## 2. Where, the sticks and the vault

*Picture to come: `secure-boot-keys-where.png`, Where it will be kept with Place 1 and Place 2 each keeping a stick with Vault 1.*

These steps are the same as a GPG key's ([the backup flow](../screens/backup-flow.md)):

1. **Where it will be kept**: Vault 1, new, on a stick at **Place 1**
   and another at **Place 2**. Two places or more; **Add a place** for a
   third. Press **Continue**
   ([A GPG key, step 2](gpg.md#2-where-it-will-be-kept)).
2. **Bring in the sticks**: **Insert the stick for Vault 1 (Place 1)**,
   **This is the stick for Vault 1 (Place 1)**, pull it; then Place 2's
   stick, with a hold where it reads as the first
   ([A GPG key, step 3](gpg.md#3-bring-in-the-sticks)).
3. **Vaults**: **Create Vault 1**, its passphrase, **Create vault**
   ([A GPG key, step 4](gpg.md#4-the-vault)).

## 3. Make the keys

*Picture to come: `secure-boot-keys-make.png`, Make the key with Owner name Example Org.*

Under **Owner name**, type `Example Org`. The name goes into each
certificate, as `Example Org PK`, `Example Org KEK` and `Example Org
db`; left empty, the certificates read `Faraday PK` and so on.

Press **Make the key**. The first time after power-on, **Set the time**
opens first ([Set the time](../screens/settings.md#set-the-time)): type the next minute and press **Set** when the phone's
minute changes to it
([Vaults, passwords and codes](vaults-and-passwords.md#6-the-codes-and-the-time)).
Then press **Make the key** again. **Making the key** shows while the
three RSA keys are made.

*Picture to come: `secure-boot-keys-made.png`, Secure Boot keys made, the owner GUID and the three certificates' names.*

The card reads **Secure Boot keys made**, in the ok tone:

| Row | What it is |
|---|---|
| **Owner GUID** | the GUID the keys are listed under in the firmware |
| PK, KEK, db | each certificate's name: `Example Org PK`, `Example Org KEK`, `Example Org db` |
| **In Vault 1** | the vault the keys are in |

Press **Continue**.

## 4. Lock, and write the sticks

The **Lock** card lists what the lock keeps (**Kept**) and wipes
(**Wiped from memory**). Press **Lock and write**.

**Write the sticks** asks for **Insert the stick for Vault 1 (Place
1)**, then for Place 2, as [A GPG key, step 6](gpg.md#6-lock-and-write-the-sticks)
does: the first stick is refused for Place 2, and the second takes
**Hold: this is a second stick**. Each copy is written, read back and
compared (**Written and checked on** the stick's name).

The enrolment files, made with the keys, go on the last stick written,
once every other place is written. With the keys made in the flow, they
are the **Windows-compatible** set (step 6).

*Picture to come: `secure-boot-keys-done.png`, Done with both sticks written and the files on each.*

**Done** reads **Written**, with a line per stick and the files written
to it. Press **Unlock Vault 1**, type the passphrase and press
**Unlock**.

## 5. The keys' page

*Picture to come: `secure-boot-keys-page.png`, Owner keys in Vault 1, Windows-compatible chosen.*

The keys' page is **Owner keys** (**PK, KEK, db**) under **Secure
Boot** ([Secure Boot keys](../screens/vault.md#secure-boot-keys)):

| Row or button | What it is |
|---|---|
| **Owner GUID**, PK, KEK, db | the owner GUID and each certificate's name |
| **Enrolment adds** | **Windows-compatible** or **Own keys only**, and the line under them (step 6) |
| **Backup** | **Written to every place** |
| **Enrolment files** | the enrolment files for the choice made, in Files for the stick |
| **Sign or check an image** | the EFI images in Files (steps 8 and 9) |
| **Hold to delete from vault** | removes the keys from the vault open |

Until every place is written, **Backup** reads "Written to 1 of 2
places" with **Continue the backup**, and **Back up first** stands in
place of **Enrolment files** and of each image's **Sign**. Keys in a
vault unlocked from its stick sign at once.

## 6. The enrolment files

**Enrolment adds** decides what the files put in the firmware beside
your own keys:

| Choice | The line under it |
|---|---|
| **Windows-compatible** | "Yours, with Microsoft's KEK CAs and Windows CAs. Not Microsoft's third-party CA: a card or controller whose firmware it signed may not start" |
| **Own keys only** | "Your PK, KEK and db only. Windows will not start, nor any card or controller whose firmware Microsoft signed", in the warning tone |

Choose, then press **Enrolment files**. Nine files go into Files for
the stick:

| File | What it is |
|---|---|
| `PK.auth`, `KEK.auth`, `db.auth` | each store's signed update: PK signed by PK, KEK by PK, db by KEK |
| `PK.esl`, `KEK.esl`, `db.esl` | each store's signature list |
| `PK.cer`, `KEK.cer`, `db.cer` | your three certificates |

Firmware setups differ in which of the three forms they take. Write the
files to a stick on the next stick visit
([Files, sticks and QR codes](files-and-sticks.md#3-write-files-to-a-stick)).

## 7. Enrol the keys

Enrolling is done in the computer's firmware setup, from the stick,
not by Faraday. Where the setting is, and which files a setup takes,
differs by maker and model; Faraday has no page per maker for it yet.

- Add the db key beside Microsoft's where the setup allows it; then
  both Windows and Faraday start with Secure Boot on. On some laptops,
  removing Microsoft's keys stops the graphics firmware loading.
- Enrolling a key changes what BitLocker measures exactly as turning
  Secure Boot off does, and Windows may ask for its recovery key at
  its next start: have it ready
  ([What Secure Boot and BitLocker are](../../learn/faraday/boot/secure-boot-bitlocker.md)).
- Surface computers and Macs take no keys of their own.

The release's boot program carries no signature, so a computer that
enforces your keys starts the stick only once its boot file is signed
with your db key (steps 8 to 10).

## 8. Sign a release's boot program

On the computer, download release 0.1.8's `faraday-0.1.8-BOOTX64.EFI`,
`SHA256SUMS` and `SHA256SUMS.asc`, and put the three on a stick: the
Faraday stick's data partition or another stick. On Faraday, copy the
three in on a stick visit
([Files, sticks and QR codes](files-and-sticks.md#2-a-stick-visit-bring-files-in)).

Unlock Vault 1, open **Secure Boot**, **Owner keys**, and press **Sign
or check an image**.

*Picture to come: `secure-boot-keys-images.png`, Sign or check an image with faraday-0.1.8-BOOTX64.EFI listed.*

**Sign or check an image** lists each EFI image in Files with **Check**
and **Sign**; "No EFI image from the stick" while there is none. Press
**Sign** on `faraday-0.1.8-BOOTX64.EFI`.

*Picture to come: `secure-boot-keys-sign.png`, Sign faraday-0.1.8-BOOTX64.EFI, its SHA-256 and the release line.*

The page reads "Sign faraday-0.1.8-BOOTX64.EFI", with the file's
SHA-256 in groups of four and, under it, what Faraday found:

| Line | Then |
|---|---|
| "The boot program of release 0.1.8, checked against its signed sums" | **Hold to sign** and **Back** |
| **No signed sums from a Faraday release are beside this file** | copy in the release's `SHA256SUMS` and `SHA256SUMS.asc` |
| **The release's sums are not signed by Faraday's release key** | the sums are not the release's: download them again |
| "This file is not the boot program of release 0.1.8" | the file is not the one the sums list: a changed file, or one already signed |
| "Release 0.1.7 is older than this Faraday (0.1.8); sign a release at least as new" | an older release; sign it on that release's Faraday |
| **This file is not an EFI image** | not a boot program |

With a refusal, the line is in the error tone and only **Back** is
offered. The SHA-256 is the plain SHA-256 of the unsigned file, the one
`SHA256SUMS` lists: compare it with the release's line.

Hold **Hold to sign** until it fills. The signed file goes into Files
for the stick under the same name ("faraday-0.1.8-BOOTX64.EFI, signed,
waits for the stick"). Write it to a stick.

## 9. Check an image

**Check** on an image's row says under the list what its signature is,
against this vault's db key:

| Result | Meaning |
|---|---|
| **Signed by this vault's db key** | signed with your db key, unchanged since |
| **Signed by another key** | signed, by a key other than this db key |
| **Changed since it was signed** | the signature no longer holds over the file |
| **Not signed** | no signature: the release's own file reads so |
| **Its signature could not be read** | a signature Faraday cannot read |

**Back to the keys** closes the images. A signed file is checked once it
is copied in from a stick.

## 10. The Starter writes the signed boot file

On the computer, put the signed `faraday-0.1.8-BOOTX64.EFI` in the
Downloads folder. In Faraday Starter, press **Make**, then **Make a
Faraday stick**; on **For which computer?**, **A PC or an Intel Mac**;
on **Which stick?**, the stick
([Get Faraday on a stick](get-faraday.md)).

*Picture to come: `secure-boot-keys-boot-file.png`, What to write? with One I signed chosen and the signed file taken.*

On **What to write?**, choose **New Faraday stick** or, for a Faraday
stick, **Replace Faraday, keep data**. Under **Boot file**:

| Choice | What it is |
|---|---|
| **The release's (unsigned)** | ticked at first |
| **One I signed** | **BOOTX64.EFI signed with your db key**: lists the `.efi` files in Downloads, the newest first ("In" and the folder; "No .efi file in" and the folder while there is none) |

Press **One I signed**, then the file; a file dropped on the window is
taken too. **Checking against this release** shows while the Starter
checks it, then:

| Row | What it is |
|---|---|
| **Certificate name** | the name on the signing certificate: `Example Org db` |
| **Certificate SHA-256: compare it with your db key's** | the certificate's SHA-256 |
| **File** | the file taken |

The name is whatever the file's maker chose; the SHA-256 says whose key
signed it. Compare it with the SHA-256 of `db.cer` from step 6
(`sha256sum db.cer`). A file the Starter refuses says why:

| Refusal | Meaning |
|---|---|
| **This is not this release's boot file** | another release's program, or one changed |
| **This boot file is not signed** | no signature |
| **This boot file has no signature that holds** | a signature that does not hold over the program |
| **This boot file holds more than the release's program and one signature** | something more than one signature added |

**Continue** waits until a signed file is taken. **Confirm** lists the
**Boot file** with the certificate's name and SHA-256; press **Erase and
write** (a new stick) or **Write** (keep data). The Starter writes the
release's own program with your signature in place of the release's
file; it reads the file again before writing, and refuses it if it
changed after it was chosen.

Without the Starter, once the stick holds the release, copy the signed
file over `EFI/BOOT/BOOTX64.EFI` on its boot partition, `FARADAYBOOT`
(Faraday's README, "With Secure Boot").

## 11. Start with Secure Boot on

On the computer whose firmware holds your db key, start from the stick
with Secure Boot on ([Start a PC from the stick](start-from-the-stick.md),
leaving out turning it off). Each new release's boot program is signed
again, as in steps 8 to 10, on a Faraday no newer than that release:
the Faraday stick you have signs the next release's.

> **With your own key**
>
> - Make the keys on the Faraday stick, not in Faraday Demo, which runs
>   on a computer with a network.
> - Never enrol keys you have not backed up: wait until the key's page
>   reads **Written to every place**, then unlock the vault once from
>   each stick on a fresh start
>   ([Before you rely on a key](before-you-rely.md)). Without the db
>   key, no new release can be signed for that computer.
> - Keep Microsoft's keys (**Windows-compatible**) on a computer that
>   runs Windows.
> - Have the BitLocker recovery key at hand before enrolling.
> - Compare the certificate's SHA-256 on What to write with your
>   `db.cer` before writing a stick.
>
> Why: [What Secure Boot and BitLocker are](../../learn/faraday/boot/secure-boot-bitlocker.md),
> [Replacing Faraday on a stick](../../learn/faraday/boot/replace-faraday.md),
> [Vaults](../../learn/faraday/vaults.md),
> [Where keys are kept](../../learn/faraday/keys-kept.md),
> [Time and TOTP codes](../../learn/faraday/time.md).

Next: [Sign and check a message](messages.md).
