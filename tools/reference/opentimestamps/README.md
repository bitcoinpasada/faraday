# python-opentimestamps, vendored

The `opentimestamps/core/` modules of python-opentimestamps that the
OpenTimestamps area of the crypto cross-check (F79) reads proofs with,
copied unchanged: `op.py`, `notary.py`, `serialize.py`, `timestamp.py`,
`dubious/notary.py` and the `__init__.py` files above them. Nothing in them
reaches the network when imported. The area's own code is
`tools/crosscheck/area_ots.py`.

Source: `https://github.com/opentimestamps/python-opentimestamps`, release
v0.5.0, commit `8c3dc6fbdf6fcba054e78643e9df5eccd8c9fce4`, the `opentimestamps/core/`
folder, `opentimestamps/__init__.py` and `LICENSE` (LGPL-3.0-or-later). It
was not checked here against the project's signatures: what pins the copy is
the commit hash and the digests below.

## Shims

Three modules the core imports are not vendored, and `shims/` stands in for
them (written for the cross-check, not part of python-opentimestamps; the
area checks their hashes against known values before anything else):

- `shims/Cryptodome/Hash/keccak.py`: Keccak-256 with the original padding
  (not SHA3-256), for `OpKECCAK256`, over the standard library only.
- `shims/Cryptodome/Hash/RIPEMD160.py`: RIPEMD-160 through the pure-Python
  hash of Bitcoin Core's test framework (`tools/reference/core/`), for
  `OpRIPEMD160` where `hashlib` has no `ripemd160`.
- `shims/bitcoin/core/__init__.py`: the four names `timestamp.py` imports
  from python-bitcoinlib (`CTransaction`, `SerializationError`, `b2lx`,
  `b2x`), which only its printing routine `str_tree` uses; python-bitcoinlib
  itself is not vendored.

## Digests

SHA-256 of every file. The same digests are in
`tools/crosscheck/references.sha256`, which the cross-check checks before it
imports anything, refusing to run on a mismatch.

| File | SHA-256 |
|---|---|
| `LICENSE` | `5218fc3db03e0f5ed0edec8912e124c493899e4be83a5bf60d9e5b1286f8f554` |
| `shims/Cryptodome/Hash/RIPEMD160.py` | `b0d07ab7dcea2f65e7ff9248baebb76ac703fcf835f30cc3834ed1fd8233d006` |
| `shims/Cryptodome/Hash/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `shims/Cryptodome/Hash/keccak.py` | `3b58c55158438359f360c38cd853da66c3c7211c495e9d69d33e4041def8a2ea` |
| `shims/Cryptodome/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `shims/bitcoin/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `shims/bitcoin/core/__init__.py` | `2f712f7fcca18aa8dfe4ce101febf6e97a44703cf7edd1060db85331fc0e4031` |
| `src/opentimestamps/__init__.py` | `733c6e6c0d2a0617c02aee7216ea89527e8daa301936079f09766be501e70ca9` |
| `src/opentimestamps/core/__init__.py` | `bd93cbc28f908648eb9cf473e83427794cc2b8637a198a0395a2fbd05063b19e` |
| `src/opentimestamps/core/dubious/__init__.py` | `d8738a12f0874e6c7dc863bfdb07e64ede1110888ad5dd01e6464e286f8e754b` |
| `src/opentimestamps/core/dubious/notary.py` | `aac57e38cb1634ef1aa09c8564b236a8cb92d5247cb0f257eeefd29c84a5e0b6` |
| `src/opentimestamps/core/notary.py` | `62111f56d553b5c074be6ba26a5defd6466b21fc6d5eca245767239843cd43f6` |
| `src/opentimestamps/core/op.py` | `203a632f30fe687f14c30b0945d8e709bdb1edbb1e75547b0c412f2085548bac` |
| `src/opentimestamps/core/serialize.py` | `5b4c2c02ce2a412294ca393d25224b7293d24c3ae9df37953c53345cc02fb745` |
| `src/opentimestamps/core/timestamp.py` | `8b66811f91d05218105e2fb08db3d864045755bc708134f85d176c8f53fbae91` |
