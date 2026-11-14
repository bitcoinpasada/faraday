# BIP-375 silent payments in a PSBT

BIP-375's test vectors, verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1/`
(the BIPs repository's `master` at commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1`, BIP-375 version 0.1.2,
vectors version 1.1), downloaded 2026-10-23. One line is left out of the
JSON as kept: the first entry of `notes`, which names the generator's
repository by its author's account; nothing reads `notes`. Putting that line
back gives the downloaded digest.

| File | Path in the BIPs repository | SHA-256 | Read by |
|---|---|---|---|
| `bip375_test_vectors.json` | `bip-0375/bip375_test_vectors.json` | downloaded `e516e1f4ac74a04f00f22f1d52dd9e83e12245169ae53e3b542a0db05b4bdfe1`; as kept here `72aaf9b4b37dbd215f85ba36566572ab40b031e055ec76769b4396dddac15acd` | `faraday/faraday-core/tests/all/psbt_v2.rs` |

Faraday does not sign silent payments. The vectors are 23 invalid and 20
valid version 2 PSBTs; the test reads the valid ones and checks that each
is refused when read for signing.
