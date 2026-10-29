# BIP-375's reference validator, vendored

The BIPs repository's runnable validator for silent payments in a PSBT,
with the Python it imports, copied unchanged for the crypto cross-check
(`tools/crosscheck/area_silent_send.py`, F79). The area imports
`test_runner.py` and calls its `validate_bip375_psbt`, which runs the
validator's four checks (PSBT structure, ECDH coverage with every share's
BIP-374 proof, input eligibility, output scripts) as the BIP's own runner
does; its `deps/dleq.py` is the validator's copy of BIP-374's proof code,
and its `validator/bip352_crypto.py` is the BIP's script arithmetic, which
the area also calls.

Source: `https://github.com/bitcoin/bips`, commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` (master, 2026-10-23), folder
`bip-0375/` (BIP-375 version 0.1.2). Fetched with `git clone` at that
commit, which checks every object against the commit hash. Licences: BIP-375 is
BSD-2-Clause (its header), which allows copying with the notice kept, and
the validator files carry no other notice; the two files of
`deps/bitcoin_test/` carry Bitcoin Core's MIT notice in their headers;
`deps/secp256k1lab/` is secp256k1lab's, MIT, its `COPYING` kept. Every file is copied with its path
inside `bip-0375/` unchanged. Left out, as nothing imports them: the BIP's
`README.md`, `bip375_test_vectors.json` (kept as `tools/vectors/bip375/`),
the library's `pyproject.toml`, `README.md`, `CHANGELOG.md`,
`.python-version`, `py.typed` and `.github/` (a workflow file of the
library, which this repository does not run).

`tools/vectors/bip375/` keeps three of the validator's files for reading
(the same digests as `validate_psbt.py`, `inputs.py` and `bip352_crypto.py`
below). The BIP's own runner takes the vectors as `python test_runner.py -f
tools/vectors/bip375/bip375_test_vectors.json`, which passes all 43.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `test_runner.py` | `c36f61f1c7e207c44d095729570529134cc2bcfbe9141da44b2bca1cadc12d9f` |
| `validator/bip352_crypto.py` | `f7f4a64b7a38bb25ecff3eab75d47874ca9cc352b8b3d6143f8e07fcf8f69a99` |
| `validator/inputs.py` | `514c372e207d3e60d1807129413f26957c8c41ff9494b423468a7868bb1aeba2` |
| `validator/psbt_bip375.py` | `c2d82bd14a9088f37942db66dd22d7fc55a2b6749be97c2374610135337e0855` |
| `validator/validate_psbt.py` | `f361705d5d35b20db6f831ad7c60e276de3f6c578b9a8b71103f1a4337b9cbe7` |
| `deps/dleq.py` | `6110f7d38bafe2b3890edc4ef004e468a1bdc46003e26d013a80e82ea92ad05d` |
| `deps/bitcoin_test/messages.py` | `f0402cef1a304fd7c9dd4eae3c933803adc5b450cb28f5c428f6431be5047b88` |
| `deps/bitcoin_test/psbt.py` | `277d6c5385f66d2bab0a6edfca41d71a4d57c4947f0b683b9f7d9d7401a0286a` |
| `deps/secp256k1lab/COPYING` | `8f13f6dc042fbc546d7217ed87f1639a9f95ba8ba0248364899564c5d4f32576` |
| `deps/secp256k1lab/src/secp256k1lab/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `deps/secp256k1lab/src/secp256k1lab/bip340.py` | `428ee8110175b32b67d22a88efe07a44cede2419d0f547949164f2a8f9d76b96` |
| `deps/secp256k1lab/src/secp256k1lab/ecdh.py` | `8ee93a3442bce4c438bd6373565071a09fd7c1075e998b7e90b51ae17a8df0e1` |
| `deps/secp256k1lab/src/secp256k1lab/keys.py` | `25a7c9d4d70dec7acea57245a529ef5f8454616d075a31fe27964d3e237c2860` |
| `deps/secp256k1lab/src/secp256k1lab/secp256k1.py` | `3a3316ea6717f8dd6d89eb65e80a09265551955d5c9eb872fae3e12504cc63b0` |
| `deps/secp256k1lab/src/secp256k1lab/util.py` | `3aeccf814f0bd67cdfb0791a6bf054b367c580942acd44942fdf505e5a1af707` |
