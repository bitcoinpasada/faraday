# BIP-390 musig() descriptor vectors

`bip-0390.mediawiki` is the BIP itself, downloaded 2026-09-12, SHA-256
`11825211de6e295dcc6ad55045ed51ea8e23d24f4d5f60aa3b18e9d0b26ff795`.
`descriptors.txt` is its "Test Vectors" section extracted into one line
per descriptor: `valid` with the scripts it pays to on the lines below
it, `invalid` on its own.

`core/osk-bip/tests/musig.rs` reads `descriptors.txt`. Every invalid
descriptor must be refused. Of the six valid ones, three are built and
compared against their scripts; the other three name shapes this crate
does not build — a participant written as a WIF private key, a taproot
script tree, and a `musig()` inside a tap leaf — and the test requires
those to be refused rather than read as some other wallet.

To refresh: download the BIP again, check the digest changed for a
reason, and re-extract the bullet lists.
