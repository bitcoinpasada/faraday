# Transfer

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Transfer`
**Key:** transfer

Carries a file or text between this computer and the offline device as QR codes. Send shows a file or text as codes to scan, and Receive reads codes with the camera.

## Send

**Key:** transfer.send

**Send** lists the files of the folder shown under it, newest first, each with its size. Choosing a file shows it as codes; while it is read, **Reading** names it. **No files** shows when the folder holds none.

A file over the largest size the code can carry is marked over that size and refused with a line saying it is larger and to carry it on a stick. An empty file is refused with a line saying it is empty.

## Send text

**Key:** transfer.send-text

Text typed or put in the **Text** box by **Paste** is shown as one code, or an animated one when it does not fit. Under the box is its count of characters and its size, or why it is not shown. **Clear** empties the box, and **Show as QR** is offered while the text may be sent.

Text is refused when it is a secret, holds one, holds a code that may hold one, or is hex of a seed's length. Text over 16.4 KB is refused: send it as a file.

## Receive

**Key:** transfer.receive

**Scan with the camera** reads codes and saves each whole file into that folder. A line under the buttons says that it was saved, adding that the file holds a secret where it does, or that it was not saved, with the reason. **Open folder** shows the folder in the computer's file manager where it can.
