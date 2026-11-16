# Check a timestamp

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `OtsCheck`
**Key:** ots-check

Check a timestamp tests an OpenTimestamps proof (a `.ots` file) against a file and a Bitcoin block header, without a network. It opens from Tools and returns there.

## Proof

**Key:** ots-check.proof

Choose the proof from Files, which offers one button per file that starts as an OpenTimestamps proof. **No file in Files** shows when there is none. A proof that cannot be read shows the reason in one line.

## File

**Key:** ots-check.file

Choose the file the proof stamps from Files, or give only its SHA-256. **SHA-256 of the file** takes 64 hex digits, and **Use** takes the typed value; **Scan** reads the SHA-256 from a code.

A SHA-256 given this way says nothing about the file itself. When the proof starts from another hash, the result is **The whole file is needed**, and you choose the file instead.

## Block header

**Key:** ots-check.header

This is the 80-byte header of the block the proof names, taken from a node you trust. Choose it from Files, as a raw 80-byte file or its hex; or type the hex in **Header in hex** and press **Use**; or read it with **Scan**.

A header is refused, with the reason in one line, if it is not 80 bytes, if its target is not valid or is easier than the network allows, or if its hash is above its own target.

## Result

**Key:** ots-check.result

**Not checked yet** shows until all three are given. The verdict is one of **The header commits to this file**, **The header commits to this SHA-256**, **No path through SHA-256 or RIPEMD-160 reaches this header**, **This is not the file the proof states** and **The whole file is needed**.

**Block hash**, **Height the proof states** and **Block time** follow a match. The height is written in the proof and not checked; the time is the one in the header. A path through SHA-1 or Keccak-256 is listed and not counted. A pending attestation, and one for Litecoin or Ethereum, is listed and not checked. The device holds no copy of the chain, so a match does not show that the header is in it.
