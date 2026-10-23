# BIP-327's reference implementation, vendored

MuSig2's reference implementation, copied unchanged from the BIPs
repository, for the crypto cross-check (`tools/crosscheck/area_musig.py`,
F79). It is self-contained: its own pure-Python curve arithmetic, the
standard library and nothing else.

Source: `https://github.com/bitcoin/bips`, commit
`c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` (master, 2026-10-23), file
`bip-0327/reference.py`. Fetched with `git clone` at that commit, which
checks every object against the commit hash.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `reference.py` | `1167993fe159c39c004f74d9046e931ae700a2ce9203c4cf58a51a000c1294e8` |
