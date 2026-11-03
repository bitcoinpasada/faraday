# Coinkite's BBQr test files

The files Coinkite's BBQr repository publishes as test data, unchanged,
at commit `8dc7ef07d0d520763cc0001a885ca8d29ac8719a` (the commit
`tools/reference/bbqr/README.md` names), folder `test_data/`, fetched as
`git archive` of that commit from a clone, whose commit hash covers every
object. Public domain (`UNLICENSE.md`, from the repository's root).
Transactions and PSBTs of Coinkite's test wallets; no private key or seed.

How Coinkite's tests use them (`python/tests/test_join.py`,
`test_encoding.py`, and `js/tests/`), and how the cross-check's `bbqr`
area (`tools/crosscheck/area_bbqr.py`, batch X23) holds Faraday to them:

- `real-scan.txt` is 26 lines, the parts of one set as a phone scanned
  them, in order. Coinkite's `join_qrs` reads it as file type `U` whose
  data contains `Zlib compressed` and `PSBT`. Faraday reads the set in
  that order and in reverse and must give the same type and bytes.
- The ten PSBTs and transactions are the inputs of Coinkite's compression
  test, which encodes each with `Z` and decodes it back to the same
  bytes. The area splits each in every encoding (`2`, `H`, `Z`) at the
  version limits it uses elsewhere, as type `P` for `.psbt` and `T` for
  `.txn`, and Faraday must read each set back to the file's bytes.

| File | SHA-256 |
|---|---|
| `1in1000out.psbt` | `7e064fd32aad5cf0bc4ac79fe80a8a27fb832506c2fcdc04b2591c4a87a7ca26` |
| `1in100out.psbt` | `c38b8d114150bf8908bef19d1cbb94430a9ea05bfcab2a629051820ba91245df` |
| `1in10out.psbt` | `045b95a5c8275a3f8562840d7369cc9eb7d7d81c162a881c48b683902bdfc698` |
| `1in20out.psbt` | `645e7bf99b52045d9a683b19d342cf9cc55c9482c50a88c1b1eb23297273aba1` |
| `1in2out.psbt` | `2929866d35ee77eeca2f7addc3d17d81fdf777ac3208374a76f8f06ce9abf222` |
| `devils-txn.txn` | `6de209567aabaa3f18276be099199eaf1e9c16100507228a92b12c67b197dd6f` |
| `finalized-by-ckcc.txn` | `ea0807625402361eb6168e6577bdac837793773c7bbf5bb9050fca4d93a865f7` |
| `last.txn` | `3b4f221677384aa22a7bba33711a44c5788e604bf7287d8a4ece1b2dee77b3e5` |
| `nfc-result.txn` | `226018f757e5c5d85924b5fbfa1951c26f1d8f197d8eae99f1136137a762fc13` |
| `real-scan.txt` | `920d332fd9e3dc2dfafa560dfaa0fca071578c6a3881aa7b257c69b221045d67` |
| `signed.txn` | `a8964f7ab05ad4d23620ee762b97eac9a8db1c084fb1214a2d8f65786e0da1d7` |
| `UNLICENSE.md` | `8d4e9b4aa336afd26a386bc2ad8c3da4f8ebb8f7e9685c09bdba6b4e75894dad` |
