# BIP-32 test vectors

`invalid-keys.txt` is BIP-32's fifth test vector: sixteen extended keys a
parser must reject, one per line, the key first and the BIP's own reason
after it.

Source: `https://raw.githubusercontent.com/bitcoin/bips/master/bip-0032.mediawiki`,
downloaded 2026-09-10, SHA-256
`e5e00a8289db2f681052cf24a745320afc225e66b25d1e489a7c884d2fc7f11f`.

The list was taken from the `===Test vector 5===` section verbatim; the
keys were not retyped. To refresh it, download that file again, check the
digest changed for a reason, and re-extract the bullet list.

`core/faraday-bip/tests/all/bip32.rs` reads the file: every key in it must be
rejected by `faraday_bip::xkey`, and the reason the decoder gives must be the
one the BIP states. Vectors 1 to 4 stay in that test file, since they are
key/child pairs rather than a list.

`vectors.json` is vectors 1 to 4 (each seed, and each chain's path,
`xpub` and `xprv`, a hardened step written `h`), which the crypto
cross-check's `bip32` area (`tools/crosscheck/area_bip32.py`) checks its
own derivation and Faraday's against. `extract.py` wrote it from the
same file, taken from the bitcoin/bips repository at commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` (2026-10-22), whose SHA-256 is
the one above: `python3 -I tools/vectors/bip32/extract.py bip-0032.mediawiki`.
