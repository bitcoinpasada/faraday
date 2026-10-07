# BIP-32 test vector 5

`invalid-keys.txt` is BIP-32's fifth test vector: sixteen extended keys a
parser must reject, one per line, the key first and the BIP's own reason
after it.

Source: `https://raw.githubusercontent.com/bitcoin/bips/master/bip-0032.mediawiki`,
downloaded 2026-09-10, SHA-256
`e5e00a8289db2f681052cf24a745320afc225e66b25d1e489a7c884d2fc7f11f`.

The list was taken from the `===Test vector 5===` section verbatim; the
keys were not retyped. To refresh it, download that file again, check the
digest changed for a reason, and re-extract the bullet list.

`core/osk-bip/tests/bip32.rs` reads the file: every key in it must be
rejected by `osk_bip::xkey`, and the reason the decoder gives must be the
one the BIP states. Vectors 1 to 4 stay in that test file, since they are
key/child pairs rather than a list.
