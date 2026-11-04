# The vault's primitives: published answers

`vectors.json` holds the Argon2id known answer the cross-check's `vault`
and `kdbx` areas (`tools/crosscheck/area_vault.py`, `area_kdbx.py`) check
Faraday's Argon2id against before any random case. `extract.py` wrote it
from `tools/vectors/kdf/README.md`; the values were taken from that
text, not retyped (argon2-cffi and OpenSSL agree on the answer).

To refresh: change the known answer there and run
`python3 -I tools/vectors/vault-primitives/extract.py`.
