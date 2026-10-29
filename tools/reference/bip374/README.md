# BIP-374's reference implementation, vendored

Discrete log equality proofs' reference implementation, copied unchanged
from the BIPs repository, for the crypto cross-check
(`tools/crosscheck/area_silent_send.py`, F79). Its tests (`unittest`) are
part of the file and are not run.

Source: `https://github.com/bitcoin/bips`, commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` (master, 2026-10-23), file
`bip-0374/reference.py` (BIP-374 version 0.3.0). Fetched with `git clone`
at that commit, which checks every object against the commit hash. The BIP
is licensed BSD-2-Clause, which allows copying with the licence noted; the
file itself carries no other notice. It is the same file, byte for byte, as
the copy kept for reading in `tools/vectors/bip374/`.

The file imports `secp256k1lab`. The BIPs repository vendors that library
beside it (`bip-0374/secp256k1lab/`); the area puts the copy under
`tools/reference/bip375/deps/secp256k1lab/` on the import path instead, which
is the same files: every `secp256k1lab` file of `bip-0374/` and of
`bip-0375/deps/` has the same SHA-256 at this commit, and the BIP-352
folder's too (`tools/reference/bip352/secp256k1lab/`).

| File | SHA-256 |
|---|---|
| `reference.py` | `9081a7a899279acb11777b974e199f240b661ce89e93ae43f74de82130fb3ceb` |

The same digest is in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch. The vectors are `tools/vectors/bip374/`.
