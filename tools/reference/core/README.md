# Bitcoin Core's functional-test framework, vendored

The pure-Python modules of Bitcoin Core's functional-test framework that
the crypto cross-check (F79) reads Bitcoin with, copied unchanged:
transactions, their serialisation, txid and wtxid (`messages.py`), the
three signature hashes (`script.py`: `LegacySignatureHash`,
`SegwitV0SignatureHash`, `TaprootSignatureHash`), BIP-173 and BIP-350's
reference code as Core keeps it (`segwit_addr.py`), Base58Check
(`address.py`) and BIP-380's descriptor checksum (`descriptors.py`).
The other files are what those import: `key.py`, `script_util.py`,
`util.py`, `coverage.py`, `authproxy.py` and the three
`crypto/` modules. Nothing in them reaches the network when imported.

Used by `tools/crosscheck/area_addr.py`, `area_tx.py`, `area_psbt.py`
and `area_msg.py`.

Source: `https://github.com/bitcoin/bitcoin`, tag `v31.1`, commit
`9be056a8a72b624dae9623b2f7bded92c2a21c91`, folder
`test/functional/test_framework/` and the top-level `COPYING` (MIT).
Fetched as the archive of that commit. The tag object
`bfa6a4b79cd4c1562fd32857e3147763efae37fb` is signed; GitHub reported
its signature valid on 2026-10-23. It was not checked here against the
release signers' keys: what pins the copy is the commit hash and the
digests below.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `COPYING` | `b028769f3852a9368ab10bd754ff01ebb741f84a2fa658c9aff82a631bc6ecfc` |
| `test_framework/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `test_framework/address.py` | `63e83cdd2054afbccff8932729dc65cf8feafc8ce3bd9b0071d492058d12bf4f` |
| `test_framework/authproxy.py` | `736246f2495578e0b1f38775ae44c59514e5bd45bf83b427d90b2370916d1a1a` |
| `test_framework/coverage.py` | `e301f58d9be13a18d061bfdad130b4a40cfbf6ea6f741c36259e611c8ecc6eaa` |
| `test_framework/crypto/ripemd160.py` | `99da6dafa9804d3ca5490a736354027b10ec3d517206caad6ea28c0e097f6b9a` |
| `test_framework/crypto/secp256k1.py` | `cab75b7335f839b88eb8517b417715858e538aaf0071f6c01ec4eb3e1867f246` |
| `test_framework/crypto/siphash.py` | `7af3111be53124afb78a607cb9bfe3fdd5a64717c95a279d5962cda9a0853ab1` |
| `test_framework/descriptors.py` | `8c2a4490c1b2a09c6c6e209eda090b160214d8156e20d6db671459cb4090a16a` |
| `test_framework/key.py` | `1f48819cbecacb5916a32a4930b4576ab362c190fa552342378e5c809a6bae77` |
| `test_framework/messages.py` | `c786651a517a49dd084b1d76072f755d40c5483f5be53b7322ca4f3403548b26` |
| `test_framework/script.py` | `24bdbfc18cfa1084c1c985218c0067c2e4c06cbd83488e5c08f2167a12bf3abf` |
| `test_framework/script_util.py` | `bbc66982f39e345aa986b52c3d197b0e1c71788cb99cd352ad0c06d0e91673bc` |
| `test_framework/segwit_addr.py` | `fbeb2a01fdae651877262c4902d237dd28d56f0e35b0fc137d3dd97854b18b08` |
| `test_framework/util.py` | `2e8499f47175e6a4df9508a37c30dc24ff90de8bc1ea117ff054f99c77bc8aa2` |
