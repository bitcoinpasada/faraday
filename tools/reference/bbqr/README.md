# Coinkite's BBQr library (Python), vendored

The Python package Coinkite publishes beside its BBQr specification,
for the crypto cross-check's `bbqr` area (`tools/crosscheck/area_bbqr.py`,
F79): `split_qrs` and `join_qrs` and what they import, copied unchanged.
Public domain (`UNLICENSE.md`, from the repository's root). Standard
library only, but for one name.

Source: `https://github.com/coinkite/BBQr`, commit
`8dc7ef07d0d520763cc0001a885ca8d29ac8719a` (master, 2026-07-27),
folder `python/bbqr/`. Fetched with `git fetch` of that commit, which
checks every object against the commit hash. The specification beside
it, `BBQr.md`, had SHA-256
`eda18acb6922350a2d407121332fdbf2af91843c4599c27425b55b66959f93cb`.

Not copied: `cli.py` (a command line over `click`) and `tables.py` (a
table printer over `pyqrcode`), which the two functions do not import.

**The shim.** `utils.version_to_chars` reads one table from pyqrcode,
`pyqrcode.tables.data_capacity[version]["L"][2]`: how many alphanumeric
characters a code of each version holds at level L. pyqrcode is not
vendored; `shims/pyqrcode/` holds that one table, ISO/IEC 18004's
numbers, and the area checks each against Nayuki's encoder
(`tools/reference/qrcodegen/`) before it runs.

| File | SHA-256 |
|---|---|
| `bbqr/__init__.py` | `2f25e90b40f2910b54c180b89583c64dbf95f9ddbf4c5177f5a9bb610b86ed99` |
| `bbqr/consts.py` | `600147b178c25ff8a8a133d05c9324770fd6f4c7de4ec43c16743df2c27b1f46` |
| `bbqr/join.py` | `6a9c98770633ee4b52b31e26fca38bf286ba3f0e12b71157fc4a4916c1670a1d` |
| `bbqr/split.py` | `9752dea8390d577bef8779261b1a24abfeaeed242ef4f403fb9fc10a6d627cdf` |
| `bbqr/utils.py` | `98ee24c30254f1e87080984a73a105b893b2aa6b1587a979d812c49cf34881e9` |
| `bbqr/version.py` | `994f2bdb3acdab73d1342a3b0cb695faa33d1950607d1136a6883d7c7ffff263` |
| `UNLICENSE.md` | `8d4e9b4aa336afd26a386bc2ad8c3da4f8ebb8f7e9685c09bdba6b4e75894dad` |
| `shims/pyqrcode/__init__.py` (written here) | `1f107fa2f5760d874aeb14209f20b7286f5efaf926a6b60bfe4b75f688b6e9da` |
| `shims/pyqrcode/tables.py` (written here) | `c2187543186a5a75d63a7f740b10a5cbfc42090acf69ad9af31b3c43415e3878` |
