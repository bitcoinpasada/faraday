# BIP-374 discrete log equality proofs

BIP-374's test vectors and its reference implementation, verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1/`
(the BIPs repository's `master` at commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1`, BIP-374 version 0.3.0),
downloaded 2026-10-23. To refresh a file, download it again and check
the digest changed for a reason.

| File | Path in the BIPs repository | SHA-256 | Read by |
|---|---|---|---|
| `test_vectors_generate_proof.csv` | `bip-0374/test_vectors_generate_proof.csv` | `d5c23db2eca645f63502de45802bd9b5630ff2213bd9c5874dc23df5c275ce35` | `core/faraday-bip/tests/all/dleq.rs` |
| `test_vectors_verify_proof.csv` | `bip-0374/test_vectors_verify_proof.csv` | `e3b10044308a4fe4947abc1cb0e49e02910c97a489a88f4b2dfdacd6f1a9a508` | `core/faraday-bip/tests/all/dleq.rs` |
| `reference.py` | `bip-0374/reference.py` | `9081a7a899279acb11777b974e199f240b661ce89e93ae43f74de82130fb3ceb` | people; the cross-check runs the copy in `tools/reference/bip374/` |

The generation file has 11 rows: a secret `a`, a point `B`, auxiliary
randomness `r`, a generator `G` (not always secp256k1's own) and a
message `m`, and the 64-byte proof expected, or `INVALID`. Rows 8, 9 and
10 are the failures: `a = 0`, `a = n` and `B` the point at infinity.

The verification file has 17 rows: `G`, `A`, `B`, `C`, a proof and a
message, and whether the proof holds. Rows 8 to 16 fail: points
swapped, a bit flipped in the proof or the message, and a proof whose
`e` or `s` equals the curve order.

`reference.py` is the BIP's own implementation. This copy is for reading;
the cross-check runs the same file, vendored with its pinned source and
`secp256k1lab`, from `tools/reference/bip374/` (`area_silent_send.py`), over
both files here and random proofs.
