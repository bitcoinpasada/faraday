# BIP-352's reference implementation, vendored

Silent payments' reference implementation and the modules it imports,
copied unchanged from the BIPs repository, for the crypto cross-check
(`tools/crosscheck/area_silent.py`, F79). `secp256k1lab/` is the
pure-Python curve library the BIP's folder carries; it is laid out as
there (`secp256k1lab/src/secp256k1lab/`) because `reference.py` puts that
path on its own import path.

Source: `https://github.com/bitcoin/bips`, commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` (master, 2026-10-23), folder
`bip-0352/`. Fetched with `git clone` at that commit, which checks every
object against the commit hash. This `reference.py` is newer than the
copy kept for reading in `tools/vectors/bip352/`.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `bech32m.py` | `b38e16a9a2f3960945773ccad020d15cd27dd1871c5a9dd26147a8fa9b14df5b` |
| `bitcoin_utils.py` | `80dd73fd9aafea15a37879124bafe4a59aa78f76de2d7d778ed0a62f2c49e7b5` |
| `reference.py` | `ca44bd5826c238aa6385c65ba31dc5a09a37790eaf4ee75e12959a1d1353124a` |
| `ripemd160.py` | `99da6dafa9804d3ca5490a736354027b10ec3d517206caad6ea28c0e097f6b9a` |
| `secp256k1lab/COPYING` | `8f13f6dc042fbc546d7217ed87f1639a9f95ba8ba0248364899564c5d4f32576` |
| `secp256k1lab/src/secp256k1lab/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `secp256k1lab/src/secp256k1lab/bip340.py` | `428ee8110175b32b67d22a88efe07a44cede2419d0f547949164f2a8f9d76b96` |
| `secp256k1lab/src/secp256k1lab/ecdh.py` | `8ee93a3442bce4c438bd6373565071a09fd7c1075e998b7e90b51ae17a8df0e1` |
| `secp256k1lab/src/secp256k1lab/keys.py` | `25a7c9d4d70dec7acea57245a529ef5f8454616d075a31fe27964d3e237c2860` |
| `secp256k1lab/src/secp256k1lab/secp256k1.py` | `3a3316ea6717f8dd6d89eb65e80a09265551955d5c9eb872fae3e12504cc63b0` |
| `secp256k1lab/src/secp256k1lab/util.py` | `3aeccf814f0bd67cdfb0791a6bf054b367c580942acd44942fdf505e5a1af707` |
