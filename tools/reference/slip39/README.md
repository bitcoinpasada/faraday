# python-shamir-mnemonic, vendored

Trezor's SLIP-39 reference implementation, copied unchanged, for the
crypto cross-check (`tools/crosscheck/area_slip39.py`, F79). Only the
modules the package's `__init__.py` imports are here; `cli.py` (which
needs `click`) and `recovery.py` are left out.

Source: `https://github.com/trezor/python-shamir-mnemonic`, tag `v0.3.0`,
commit `08bbb74bdcb58c918981836cb3ccf401e57ecfb4`, folder
`shamir_mnemonic/` and `LICENSE`. Fetched with `git clone` at that tag.
The tag is not signed: what pins the copy is the commit hash and the
digests below. The vectors it ships are `tools/vectors/slip39/vectors.json`.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `LICENSE` | `332f92f7f90a1c957b473b902a5daf39ad5eb179d8828e0304b4c40faa0123c0` |
| `shamir_mnemonic/__init__.py` | `efc4925a7f1c88c9d65d9788f9539a0aa11a4d86decc8b9e8240898bddfcb648` |
| `shamir_mnemonic/cipher.py` | `193c0bc5d90a28fd2f139e4d17e5a6e072666765d72eb527198ec71fb128ffdc` |
| `shamir_mnemonic/constants.py` | `434696830041652cc08d6f837b763d7333aec149c1b2fd976aa2bd20350a147b` |
| `shamir_mnemonic/rs1024.py` | `7527b6ab5b0de8ef90697699f24c4bd615dbc77ec988e90590bfcf80146e8391` |
| `shamir_mnemonic/shamir.py` | `a7a3db954f0a3638f3e98574c2da2e8b8af1338ed3bc33aef937afbfffce465f` |
| `shamir_mnemonic/share.py` | `3ee6f46415ba34dd8c1bc5f92601b17151854a35425e098d1de857ca714205c5` |
| `shamir_mnemonic/utils.py` | `9bf6d2c71b11788ac1adabe156e71e43365f8c17f99cadf19aa8e78f5a066ee4` |
| `shamir_mnemonic/wordlist.py` | `f246d5c397155a377cb7344f6a3986e197210469cd575c83fbbfae2bcfe09975` |
| `shamir_mnemonic/wordlist.txt` | `bcc4555340332d169718aed8bf31dd9d5248cb7da6e5d355140ef4f1e601eec3` |
