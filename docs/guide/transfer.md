# Transfer

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Transfer`
**Key:** transfer
**Words:** key, seed

Transfer carries a file or text between this computer and the offline device as QR codes. Send shows a file or text as codes for the other side to scan, and Receive reads codes with the camera.

## Send

**Key:** transfer.send

**Send** lists the files in the folder shown under it, newest first, each with its size. Choose a file to show it as codes; while the file is being read, **Reading** names it. **No files** shows when the folder is empty.

A file larger than a code can carry is marked as over that size and refused, with a line saying to carry it on a stick instead. An empty file is refused with a line saying it is empty.

## Send text

**Key:** transfer.send-text

Type text into the **Text** box, or put it there with **Paste**. It is shown as one code, or as an animated one when it does not fit. Under the box is its count of characters and its size, or the reason it is not shown. **Clear** empties the box, and **Show as QR** is offered while the text may be sent.

Text is refused when it is a secret, holds one, holds a code that may hold one, or is hex of a seed's length. Text over 16.4 KB is refused too: send it as a file.

## Receive

**Key:** transfer.receive

**Scan with the camera** reads codes and saves each whole file into that folder. A line under the buttons says that the file was saved, adding that it holds a secret where it does, or that it was not saved and why. **Open folder** shows the folder in the computer's file manager where it can.
