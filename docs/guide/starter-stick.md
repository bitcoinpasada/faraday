# Make a Faraday stick and a Pi card

**Kind:** Reference
**Status:** Checked against the app
**Starter page:** `Which`, `Disks`, `Mode`, `Confirm`, `Writing`, `Done`
**Key:** starter-stick

Make a Faraday stick writes Faraday onto a USB stick; Make a Pi card writes it onto an SD card or USB stick for a Raspberry Pi. Both go through the same pages, from the computer to the result.

## For which computer?

**Key:** starter-stick.which

Asked for a stick only, and only when the Starter has the images for both kinds of computer. Nothing is chosen unless pressed; the row that fits this computer is tagged **This computer**.

**A PC or an Intel Mac** and **A Mac with M1 or M2** each go on to the disks. A Pi card skips this page.

## Which stick?

**Key:** starter-stick.disks

Titled **Which stick?**, or **Which card?** for a Pi card, which lists SD cards as well as USB sticks. A disk is chosen by pressing its row; none is chosen for the person.

A row gives the maker and model, the size and what is on the disk: **Faraday stick** with its version, **Empty**, or the volume names. A disk this computer uses, or one smaller than the image, is dimmed with its reason and cannot be chosen. A Pi card page also names the boards and cameras the card is for.

**Image** lists the images that can be written, one ticked. **Not found** names a file to download and put in Downloads.

## What to write?

**Key:** starter-stick.mode

**New Faraday stick** (**New Faraday card** for a Pi card) writes the whole disk, and everything on it is lost. **Replace Faraday, keep data** is offered only on a Faraday stick and is told under Keep data.

**Name** is the name given to the data partition of a whole stick or card; empty writes none. It is written in capitals, at most 11 characters, and the names of the boot partitions, EFI, bootfs and FARADAYBOOT, are refused.

**Clear the whole stick first** is offered for a new stick only and writes zeros over every sector before the stick is made; it adds the time shown.

**Boot file** is offered for a stick: **The release's (unsigned)**, or **One I signed**, which lists the .efi files in Downloads to take one from. The file must be this release's own with a signature that holds.

**Continue** goes to Confirm.

## Replace Faraday, keep data

**Key:** starter-stick.keep-data

Writes only the boot partition of the stick, then reads it back. The data partition, with its vaults and settings, is not read and not written; this computer can read it while the stick is attached.

A stick that already holds this build is refused as **Already this build**. A stick whose version is not read on Windows is read first, which asks for administrator permission, and **Continue** waits for it. Replacing a newer Faraday with an older one is marked on Confirm and written only by **Hold to write the older release**. **Name** and **Clear the whole stick first** are not offered. A Pi card has no keep-data choice.

## Confirm

**Key:** starter-stick.confirm

Lists the **Stick** (**Card** for a Pi card) with its **Size**, **Device** and **Serial**, **On it**, what **Writes**, the **Boot file** for a stick, the **Name** and **Before writing** where chosen, the **Build** and the **SHA-256 of the image**.

**Erase and write** writes the whole disk; **Write** writes the boot partition under keep data. When the disk is no longer the one confirmed, **Choose again** returns to the disks.

## Writing

**Key:** starter-stick.writing

Shows the progress of **Clearing** where chosen, then **Written**, then **Reading back**. Nothing on the page is chosen, and the way back is closed. The window closes by itself when the write ends if it was asked to close.

## Written

**Key:** starter-stick.done

Titled **Written**, or **Not written** with the reason. Keep a stick in to start this computer from it, and pull it out to start another; a Pi card can be pulled out.

**In the boot menu** shows the entry the stick usually has there. **Start this computer from the stick** opens the start steps and is held in a virtual machine. **Done** returns to Home. After a failure **Try again** returns to the disks.
