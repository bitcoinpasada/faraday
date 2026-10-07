# BIP-352 silent payments

BIP-352's text, its send-and-receive vectors and its reference
implementation, verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/master/`, downloaded
2026-09-19. To refresh a file, download it again and check the digest
changed for a reason.

| File | Path in the BIPs repository | SHA-256 | Read by |
|---|---|---|---|
| `bip-0352.mediawiki` | `bip-0352.mediawiki` | `ff03af7d6d0afd423c9250659718944bc2973201b5fabf35dd565c913d38b08b` | people |
| `send_and_receive_test_vectors.json` | `bip-0352/send_and_receive_test_vectors.json` | `f5f9ed4afd76a1b76f3c70b1cbe67532f89abbe559f8e02d7fc3d8ecb93af4a1` | `core/osk-bip/tests/silent.rs` |
| `reference.py` | `bip-0352/reference.py` | `a0bc88e8024dff880e6c89a1330557b6177b0dfa284931033cd0a77d1d387912` | people |

`send_and_receive_test_vectors.json` is a list of 28 cases, each with a
`comment`, a `sending` list and a `receiving` list. A sending case gives
the inputs with their private keys and the recipients' addresses, and
expects the public key taken from each input and one of several
orderings of the taproot output keys the sender makes. A receiving case
gives the same inputs without their private keys, the taproot output
keys of the transaction, the wallet's scan and spend private keys and
the labels it has handed out, and expects the addresses the wallet
publishes and which of the outputs it finds. Case 27 expects a count
rather than a list, because it has 2,324 outputs and `K_max` stops the
scan at 2,323.

`core/osk-bip/tests/silent.rs` reads every case from both sides: the
sender's outputs are built from the input private keys and the
recipients' addresses, and the receiver then finds those same outputs
with the label each was paid to.

`reference.py` is the BIP's own implementation, kept here for reading.
It is not run: it imports a vendored `secp256k1lab`, a `bech32m` module
and a `bitcoin_utils` module that are not part of this download, and the
vectors in the file beside it are that implementation's own published
output. What this tree checks itself against instead is those vectors,
in full, and its own sending side: a payment this module makes is a
payment this module finds, under the label it was made to.
