# krux's numbered QR parts, vendored

The code of krux (a Bitcoin signer on a Kendryte K210 board) that writes
and reads a QR code in numbered parts, for the crypto cross-check's `bbqr`
area (`tools/crosscheck/area_bbqr.py`, F79; ESTABLISHED §9.4, batch X23):
`qr.py` (`to_qr_codes`, `QRPartParser`, the sizes of the parts and the
"pMofN" form) and `bbqr.py` (`encode_bbqr`, `decode_bbqr`, the header),
copied unchanged. MIT: both files carry the licence in their first lines
and `LICENSE.md` is the repository's, from its root.

Source: `https://github.com/selfcustody/krux`, tag `v26.09.0`, commit
`2b20e3370169731a0d89839a4113ad9a1b732078`, folder `src/krux/`. Fetched
as `git archive` of the tag from a clone of the repository, whose commit
hash covers every object. The rest of the folder (the pages, the
hardware, the printers, `uUR` and the rest of `src/krux/`) is not copied;
`qr.py` imports `uUR` only inside the UR branches, which the area never
reaches, and nothing else of krux.

**The shims** (written for the cross-check, not krux's; `shims/`) stand
in for the three modules krux has in its firmware as C code or as
MicroPython's own, each a few lines over the standard library:

- `qrcode.py`: `encode(part)` returns the part's text in place of the
  picture the firmware draws, so the parts krux writes can be read.
- `base32.py`: `encode(data, padding)` and `decode(text)`, RFC 4648 in
  upper case, the two calls `bbqr.py` makes.
- `deflate.py`: `DeflateIO`, raw deflate over zlib. krux gives it no
  format and no window, which the firmware it pins reads as raw with a
  1,024-byte window (selfcustody/micropython
  `b11923f020e0caf2572e5874957f29128da2bb45`, `extmod/moddeflate.c`,
  `DEFLATEIO_DEFAULT_WBITS` 10; MicroPython's own documentation gives 256
  bytes). The shim writes with a 512-byte window, which a device reads, and reads with
  the largest (a device reads 1,024).

`gc` is MicroPython's and CPython's alike and needs no shim.

What krux reads and writes is narrower than Faraday's in places (BBQr
file types `P`, `T`, `J` and `U` only; "pMofN" of at most 99 parts; the
parts of a BBQr set decoded each on its own; a text file type decoded as
UTF-8) and looser in others (a repeated "pMofN" part with other text
replaces the first); the area's facts, in `tools/crosscheck/README.md`,
name each.

| File | SHA-256 |
|---|---|
| `LICENSE.md` | `5f6b49dbeb691eb1822f23a6857a9d5fa1e802c687f0819cc101e30fc7fb91b3` |
| `src/krux/bbqr.py` | `76a0f655f208a2b02dd5999e60795c140f31e065ce53efce96fc0e0416bb644e` |
| `src/krux/qr.py` | `e448e374e13dc81126fea500760d3285a07682eca8895005fef8e2dc73ed9ddc` |
| `shims/base32.py` (written here) | `0f62c41a9f6e652daa6e7a47b8e3450fa4b10c5a01d716c0f3046f04d3499a71` |
| `shims/deflate.py` (written here) | `f13eee136955b03502b2c4013f13b5c032588e0baabd910f9875fe9c2f947f19` |
| `shims/qrcode.py` (written here) | `481483384eb3464ef403557932ed2d450de48d3d36f38a8edcc14013a417aad8` |
