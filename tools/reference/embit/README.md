# embit's BIP-39 code, vendored

The part of embit (Stepan Snigirev's Bitcoin library) that SeedSigner's
SeedQR reader and encoder call, for the crypto cross-check's `seedqr`
area (`tools/crosscheck/area_seedqr.py`, F160): `bip39.py`
(`mnemonic_from_bytes`, `mnemonic_to_bytes` with its checksum check),
the English wordlist and what they import, copied unchanged. MIT
(`LICENSE`).

Source: `embit` 0.8.0, the release SeedSigner pins in its hash-locked
`requirements.txt`, from PyPI: `embit-0.8.0.tar.gz`, SHA-256
`8bf4b10073c67400370ce523fb16f035fe759f6fdd987c579bdcc268d75ed770`
(the hash SeedSigner's `requirements.txt` names for it; the git tag is
`v0.8.0`, `84cce66fb831fa6d625fb73f28e03605f3c04e28`), folder
`src/embit/`. Not copied: everything else in embit (keys, PSBT, scripts,
descriptors, the secp256k1 bindings); `embit/misc.py` imports
`embit.util`, which the cross-check stands in for with
`tools/reference/seedsigner/shims/outside_imports.py`, nothing of it
being called.

| File | SHA-256 |
|---|---|
| `LICENSE` | `9ad8a650264e6409ba9ebacef1414e1636fb7515665306adddf4da6b0bb0e6e6` |
| `embit/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `embit/bip39.py` | `f19617b7ef97b80a085fd752a65a30f6d0f7e9e50a36633b2f2178e09be54076` |
| `embit/misc.py` | `757a60036ad83147d7f169274835d666b423272d4f1ea4c8b0dc925329db6f89` |
| `embit/wordlists/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `embit/wordlists/base.py` | `c4e144c64cbc7e46e8c9e66ad625cc4e2a04f3a13d71ae2c197413973e79ace1` |
| `embit/wordlists/bip39.py` | `c7398e797b743901822741e5a2c6010cb1ac09f7ea1bc562d6da0d8bf4d164e2` |
