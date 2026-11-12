# Check a timestamp

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `OtsCheck`
**Key:** ots-check

Checks an OpenTimestamps proof (a `.ots` file) against a file and a Bitcoin block header, without a network. It opens from Tools and returns there.

## Proof

**Key:** ots-check.proof

The proof is chosen from Files, one button per file that starts as an OpenTimestamps proof. **No file in Files** shows when there is none. A proof that cannot be read shows the reason in one line.

## File

**Key:** ots-check.file

The file the proof stamps is chosen from Files, or only its SHA-256 is given. **SHA-256 of the file** takes 64 hex digits, and **Use** takes the typed value; **Scan** reads the SHA-256 from a code.

A SHA-256 given this way says nothing about the file itself. When the proof starts from another hash, the result is **The whole file is needed** and the file is chosen instead.

## Block header

**Key:** ots-check.header

The header of the block the proof names, 80 bytes, from a node that is trusted. It is chosen from Files, as a raw 80-byte file or its hex, typed as hex in **Header in hex** and taken with **Use**, or read with **Scan**.

A header that is not 80 bytes, whose target is not valid or easier than the network allows, or whose hash is above its own target is refused with the reason in one line.

## Result

**Key:** ots-check.result

**Not checked yet** shows until all three are given. The verdict is one of **The header commits to this file**, **The header commits to this SHA-256**, **No path through SHA-256 or RIPEMD-160 reaches this header**, **This is not the file the proof states** and **The whole file is needed**.

**Block hash**, **Height the proof states** and **Block time** follow a match. The height is written in the proof and not checked; the time is the one in the header. A path through SHA-1 or Keccak-256 is listed and not counted. A pending attestation, and one for Litecoin or Ethereum, is listed and not checked. The device holds no copy of the chain, so a match does not show that the header is in it.
