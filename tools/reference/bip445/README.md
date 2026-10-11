# BIP 445's reference implementation, vendored

The FROST signing reference and the pure-Python curve library it needs,
copied unchanged so that `tools/scripts/threshold-reference.py` can play
the second share of a fixture spend and check this tree's partial
signatures against another implementation (`docs/PLANNING.md` §16.103).

Nothing here is built or run by `just`: it is Python, and the default
recipe stays Rust. `just threshold-reference` runs the check.

| What | Where it came from |
|---|---|
| `frost_ref/` | `bitcoin/bips` BIP 445's `reference.py`, as the repository's `frost_ref` package |
| `secp256k1lab/secp256k1lab/` | `secp256k1lab`, the pure-Python curve library the reference imports |
| `COPYING` | `secp256k1lab`'s MIT licence, which both carry |

Commit: `bb5396f90d45ba5a954cbfd1af80f1b45e02b229`.

SHA-256 of every file, so a change to the vendored copy is visible:

| File | Digest |
|---|---|
| `COPYING` | `8f13f6dc042fbc546d7217ed87f1639a9f95ba8ba0248364899564c5d4f32576` |
| `frost_ref/__init__.py` | `b9eba892243f3b33857e6de57e1b26796cc21b800826271610f5ed9c027576ad` |
| `frost_ref/signing.py` | `412fcb5f7e48fb781be1ab51d3d66b985e1639c461eeff6c41011037f8f52bfe` |
| `secp256k1lab/secp256k1lab/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `secp256k1lab/secp256k1lab/bip340.py` | `428ee8110175b32b67d22a88efe07a44cede2419d0f547949164f2a8f9d76b96` |
| `secp256k1lab/secp256k1lab/keys.py` | `25a7c9d4d70dec7acea57245a529ef5f8454616d075a31fe27964d3e237c2860` |
| `secp256k1lab/secp256k1lab/secp256k1.py` | `e68560295918b5aee07f38a2dbe5d8fce355ed46b50cfe3240c7d91508dc0b04` |
| `secp256k1lab/secp256k1lab/util.py` | `3aeccf814f0bd67cdfb0791a6bf054b367c580942acd44942fdf505e5a1af707` |
