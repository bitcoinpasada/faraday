# python-mnemonic, vendored

Trezor's BIP-39 reference implementation, copied unchanged, for the
crypto cross-check: Seed XOR's words (`tools/crosscheck/area_xor.py`) and
BIP-85's child words (`tools/crosscheck/area_bip85.py`), F79.

Source: `https://github.com/trezor/python-mnemonic`, tag `v0.21`, commit
`d4b106cdec196202d44628026fcb8fedc8ea50c1`, folder `src/mnemonic/` and
`LICENSE`. Fetched with `git clone` at that tag. The tag is not signed:
what pins the copy is the commit hash and the digests below.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `LICENSE` | `d5e3c7c62a84e80073201e2f6e5130e9e6804fa05f8ac4f8b26a13c7d3969697` |
| `mnemonic/__init__.py` | `c6524bc6498b80de332bf58d100d3aec11042fbb559085aa0bc25955b2eaf68d` |
| `mnemonic/mnemonic.py` | `42daf1fb4f3b43eb40284add75d832e384bbe3aaa6d00ae68c8850b8a0a6d75b` |
| `mnemonic/wordlist/chinese_simplified.txt` | `5c5942792bd8340cb8b27cd592f1015edf56a8c5b26276ee18a482428e7c5726` |
| `mnemonic/wordlist/chinese_traditional.txt` | `417b26b3d8500a4ae3d59717d7011952db6fc2fb84b807f3f94ac734e89c1b5f` |
| `mnemonic/wordlist/czech.txt` | `7e80e161c3e93d9554c2efb78d4e3cebf8fc727e9c52e03b83b94406bdcc95fc` |
| `mnemonic/wordlist/english.txt` | `2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda` |
| `mnemonic/wordlist/french.txt` | `ebc3959ab7801a1df6bac4fa7d970652f1df76b683cd2f4003c941c63d517e59` |
| `mnemonic/wordlist/italian.txt` | `d392c49fdb700a24cd1fceb237c1f65dcc128f6b34a8aacb58b59384b5c648c2` |
| `mnemonic/wordlist/japanese.txt` | `2eed0aef492291e061633d7ad8117f1a2b03eb80a29d0e4e3117ac2528d05ffd` |
| `mnemonic/wordlist/korean.txt` | `9e95f86c167de88f450f0aaf89e87f6624a57f973c67b516e338e8e8b8897f60` |
| `mnemonic/wordlist/portuguese.txt` | `2685e9c194c82ae67e10ba59d9ea5345a23dc093e92276fc5361f6667d79cd3f` |
| `mnemonic/wordlist/russian.txt` | `07f11af3f07fd13d8d74859f4448d8bca8f1d9d336dc4842531ecea083103a26` |
| `mnemonic/wordlist/spanish.txt` | `46846a5a0139d1e3cb77293e521c2865f7bcdb82c44e8d0a06a2cd0ecba48c0b` |
| `mnemonic/wordlist/turkish.txt` | `a7dc9c77913726106c7b8baa022b7e17601d118acf40aa60ab1fbc9c91b383ac` |
