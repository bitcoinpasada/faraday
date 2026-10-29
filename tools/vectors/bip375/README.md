# BIP-375 silent payments in a PSBT

BIP-375's test vectors and the three files of its reference validator
that say what the vectors are checked against, verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1/`
(the BIPs repository's `master` at commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1`, BIP-375 version 0.1.2,
vectors version 1.1), downloaded 2026-10-23. To refresh a file, download
it again and check the digest changed for a reason. One line is left out of
the JSON as kept: the first entry of `notes`, which names the generator's
repository by its author's account; nothing reads `notes`. Putting that line
back gives the downloaded digest.

| File | Path in the BIPs repository | SHA-256 | Read by |
|---|---|---|---|
| `bip375_test_vectors.json` | `bip-0375/bip375_test_vectors.json` | downloaded `e516e1f4ac74a04f00f22f1d52dd9e83e12245169ae53e3b542a0db05b4bdfe1`; as kept here `72aaf9b4b37dbd215f85ba36566572ab40b031e055ec76769b4396dddac15acd` | `core/faraday-psbt/tests/all/silent.rs` |
| `validate_psbt.py` | `bip-0375/validator/validate_psbt.py` | `f361705d5d35b20db6f831ad7c60e276de3f6c578b9a8b71103f1a4337b9cbe7` | people; the cross-check runs the copy in `tools/reference/bip375/` |
| `inputs.py` | `bip-0375/validator/inputs.py` | `514c372e207d3e60d1807129413f26957c8c41ff9494b423468a7868bb1aeba2` | people; the cross-check runs the copy in `tools/reference/bip375/` |
| `bip352_crypto.py` | `bip-0375/validator/bip352_crypto.py` | `f7f4a64b7a38bb25ecff3eab75d47874ca9cc352b8b3d6143f8e07fcf8f69a99` | people; the cross-check runs the copy in `tools/reference/bip375/` |

The vectors are 23 invalid and 20 valid version 2 PSBTs, each with a
description, and supplementary data the file says is for diagnosis
only: the inputs' private keys, the shares and proofs, the outputs'
scripts. Two carry a `checks` list naming the one stage they test
(input eligibility). The validator's four stages are PSBT structure,
ECDH coverage (every share's BIP-374 proof), input eligibility and
output scripts. The three validator files here are for reading; the cross-check runs the
validator whole, with `test_runner.py`, `secp256k1lab` and the copy of
Bitcoin Core's test framework it imports, from `tools/reference/bip375/`
(`area_silent_send.py`), over these vectors and random PSBTs.

## Where Faraday is stricter, and where the BIP disagrees with itself

- **A stated key must be the input's.** BIP-375 asks the updater for
  a `PSBT_IN_BIP32_DERIVATION` so that a P2WPKH, P2SH-P2WPKH or P2PKH
  input's public key is known. The reference validator takes the first
  one as it is; Faraday takes a key only when the input's script
  commits to it (its HASH160 is the program), since otherwise a
  coordinator could state any key, with a share and proof that verify
  under it, and the scripts would pay outputs the recipient never finds.
  16 of the 20 valid vectors and most invalid ones pair a P2WPKH output
  with a key it does not commit to (the generator did not bind them).
  The test refuses each as published for that reason, then binds each
  such output to the key the PSBT states, which changes no outpoint and
  no key and so no share, proof or script, and checks every outcome
  against the vector.
- **The order of `k`.** BIP-375's text sorts the silent payment codes
  sharing a scan key to number them; the vectors and the validator
  number them in output order, and the valid vector "two sp outputs -
  output 0 uses label=3 / output 1 uses label=1" has its spend keys out
  of sorted order. Both are payments a BIP-352 receiver finds. Faraday
  accepts stated scripts that follow either order and sets scripts in
  output order, which reproduces every vector.
- **A redeem script and a previous transaction must be the ones the
  input spends.** The validator reads a P2SH input's redeem script and a
  P2PKH input's previous output without hashing either; Faraday refuses a
  redeem script that is not the one its P2SH script hashes, and a previous
  transaction whose id is not the outpoint's, since the key it takes from
  them would not be the one that signs. The cross-check counts both as known
  differences.
