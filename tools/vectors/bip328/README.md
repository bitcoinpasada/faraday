# BIP-328 synthetic xpub vectors

`vectors.txt` is BIP-328's "Test Vectors" section: three sets of
participant keys, the aggregate public key each produces, and the
synthetic extended public key that carries it. One case per line — the
keys comma-separated in the order the BIP lists them, the aggregate key,
the xpub.

Source:
`https://raw.githubusercontent.com/bitcoin/bips/master/bip-0328.mediawiki`,
downloaded 2026-09-12. The values were taken from the bullet list; no key
was retyped.

`core/osk-bip/tests/musig.rs` reads the file: `aggregate_xpub` must
produce both the aggregate key and the extended key of every case.
