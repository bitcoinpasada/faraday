# python-bip380, vendored

Antoine Poinsot's pure-Python output script descriptors with Miniscript
(P2WSH, and Tapscript inside `tr()`): parsing a miniscript from text or
from a script, its script, its type and sanity properties, and its
smallest non-malleable satisfaction. Copied unchanged, for the crypto
cross-check (F79): `tools/crosscheck/area_miniscript.py` (the scripts a
descriptor pays to) and `area_sign.py` (the satisfactions a finalized
input carries), through `tools/crosscheck/miniscript_ref.py`.

Source: `https://github.com/darosior/python-bip380`, commit
`fb61971d9128e663f110ea2734c1d023e7e0266b` (2024-02-27, "Merge PR #33",
version 0.0.3 with the Tapscript support), folder `bip380/` and
`LICENSE` (MIT). Fetched with `git clone`. The commit is signed; its
signature was checked with `gpg --verify` on 2026-10-26 against the key
`590B 7292 695A FFA5 B672 CBB2 E13F C145 CD3F 4304` as Bitcoin Core's
`guix.sigs` repository publishes it (`builder-keys/darosior.gpg`): a
good signature by that key, whose published copy had expired before
the commit was signed. What pins the copy is the commit hash and the
digests below.

The library calls itself a work in progress. One defect bears on the
cross-check: a `multi_a`'s satisfaction is written with the first key's
signature deepest on the stack, which BIP 342's `OP_CHECKSIGADD` reads
the other way (BIP 379: the last key's signature first). `area_sign.py`
satisfies a leaf that is one `multi_a` by BIP 379's rule instead, and
uses no `multi_a` inside a larger leaf.

`shims/` is written here, not upstream. python-bip380 imports
`coincurve` and `bip32`, two packages over libsecp256k1:
`shims/coincurve/` gives its two key types and its tweak over the BIPs
repository's `secp256k1lab` (`tools/reference/bip352/`);
`shims/bip32/` refuses every extended key (the cross-check derives keys
with `tools/crosscheck/btc.py` and passes raw keys) and reads a key
origin's path.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `LICENSE` | `0f7e0681f02aade15536c248bffb86c5b2d28294ba62acbbc42bcc5a2fb11e81` |
| `bip380/__init__.py` | `e0664a8b5de50d30f6e5806419a9216721106564c447f39640c34fa07fd43367` |
| `bip380/key.py` | `237ebffd4c3ddd225add12f9ad1d3ceda8b20b6dee174cc52bac3f0c7963270a` |
| `bip380/descriptors/__init__.py` | `8de356cfb1caf65a78a8f97ebdd22aa5c4a330cda44a0e503c90b09bd35086db` |
| `bip380/descriptors/checksum.py` | `65de52d12fd2d9cf1ce973b571f450fd7cf683052d29fca5e7c7eb6d1af98646` |
| `bip380/descriptors/errors.py` | `848a51137d200346690594badf85182011ac1773fed797482af8a9f3f98373f9` |
| `bip380/descriptors/parsing.py` | `eca2e9fcf974f99fae91e1241e5b0ae5314bc59da2b71cb02c7d85fa4526a6c7` |
| `bip380/descriptors/utils.py` | `66da19e3678791961ebf579b85a02281c10cee4ad32a79163b222fd583e48da4` |
| `bip380/miniscript/__init__.py` | `66f5794ac2840026ae49e1264be5b825ae0819948592f28cd08eef1d9ad14e0e` |
| `bip380/miniscript/errors.py` | `64400bace344ca61aa3c5364dce175980747272be61cb19775b25b6b7e279d25` |
| `bip380/miniscript/fragments.py` | `71672f212414f7e02d030bb98c9f6c9bf94cdbffdc5ae9213167dd78c336af3d` |
| `bip380/miniscript/parsing.py` | `bbc9be177953f68373095aab22242d58a36fe030fceefd94ce53ca71e51e0f8f` |
| `bip380/miniscript/property.py` | `677d48b9d55b37fe792462a21709bb8ea325ac11e8315173227553574fbe451a` |
| `bip380/miniscript/satisfaction.py` | `0ebfa2dd1f96badfa9e002b2278e28a6050f18b99c612e0d1fb4109eac69b7ab` |
| `bip380/utils/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `bip380/utils/bignum.py` | `00a01acc27db1cd72aa802b6f9c33ec288fe5b582c633d41b54ac5c4ed5edc2e` |
| `bip380/utils/hashes.py` | `7ecf887e9c115e7f92dc7ec103f1e1316196d543d5a0f8bb788d4a1f51ccf428` |
| `bip380/utils/script.py` | `911be91fd65446294e88d3fc789624604171a66029c342bf0054699a76e132e8` |
| `shims/bip32/__init__.py` | `7f536c027095f0f26886da3b429989b25375563e85c2ee4289f968acd7400a6b` |
| `shims/bip32/utils.py` | `b8dbacab47471fbaafb9c78a264909512d38a2fc8ee83e9fd7d12d7cb5ce8328` |
| `shims/coincurve/__init__.py` | `621519739974c94eab2ed3942d283fd41ade06c44e549cde95aeb8c61f2e4212` |
