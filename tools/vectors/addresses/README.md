# Addresses, descriptor checksums and single-key accounts

`vectors.json` holds the published answers the crypto cross-check's
`addr` area (`tools/crosscheck/area_addr.py`) checks its references and
Faraday with, before any random case:

- `segwit_valid`: BIP-350's valid native segwit addresses with the output
  script each pays, and BIP-173's of version 0.
- `segwit_invalid`: BIP-350's and BIP-173's invalid addresses, and
  BIP-173's valid addresses of version 1 and above, which carry the
  checksum BIP-350 replaced and are invalid now.
- `descriptor_checksums`: BIP-380's checksum and character-set cases.
- `accounts`: BIP-49's, BIP-84's and BIP-86's test accounts: the words,
  the account's extended public key as the BIP writes it (SLIP-132 for
  BIP-49 and BIP-84) and its addresses.

`extract.py` wrote it from the BIPs' texts in the bitcoin/bips repository
at commit `c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` (2026-10-22); the
values were taken from them, not retyped:

| File | SHA-256 |
|---|---|
| `bip-0049.mediawiki` | `307b62faf7b9927233b5112e93d591272ba0c48303b5dbb6f0bbfbaf2347e695` |
| `bip-0084.mediawiki` | `1900feec6cafca65b8c09906ca0658d2d742b4c9b44cb15678996985b6bfe627` |
| `bip-0086.mediawiki` | `d8d01dee331da07c2562615bc1f064c1868ec3fce61184973da76d1196c7f5b0` |
| `bip-0173.mediawiki` | `b19eab06b264bbd15ad7f7d640fa34de8f4a8809fba8a6ecc3e165545b27a543` |
| `bip-0350.mediawiki` | `63634b06aa8bae88b31929674736e74964c4598684269ac2b0b140c43a7a0dec` |
| `bip-0380.mediawiki` | `34e6510bb2eba9445a68eacf3d24d5a4e5f24481477488d5552f674ac81dd5fe` |

To refresh: download the files at a newer commit into a folder, check
the digests changed for a reason, update them in `extract.py`, and run
`python3 -I tools/vectors/addresses/extract.py FOLDER`.
