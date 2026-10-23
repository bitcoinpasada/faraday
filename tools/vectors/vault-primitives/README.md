# The vault's primitives: published answers

`vectors.json` holds the published answers the cross-check's `vault`
area (`tools/crosscheck/area_vault.py`) checks its own HKDF and ChaCha20
against, and checks Faraday's XChaCha20-Poly1305 and Argon2id with,
before any random case. `extract.py` wrote it from the texts below; the
values were taken from them, not retyped.

| Key | Source | SHA-256 of the text |
|---|---|---|
| `hkdf_sha256` | RFC 5869 Appendix A, test cases 1 to 3 (the SHA-256 ones), `https://www.rfc-editor.org/rfc/rfc5869.txt` | `7a40eb3835b35fc947eb12a2ed614db079d43b26e50dbc537c31fba16397089c` |
| `chacha20_block` | RFC 8439 Appendix A.1, the five block-function vectors, `https://www.rfc-editor.org/rfc/rfc8439.txt` | `25bef70fbf7a07ff45c2fe4cb7c6ce954eac687413d8610603268b4e4415324c` |
| `xchacha20poly1305` | draft-irtf-cfrg-xchacha-03 Appendix A.3.1, `https://www.ietf.org/archive/id/draft-irtf-cfrg-xchacha-03.txt` | `fa796b50265eeee383d40e82fed880267c7835e1b3d64c50c4f06162adaa1cfd` |
| `argon2id` | the known answer in `tools/vectors/kdf/README.md` (argon2-cffi and OpenSSL agree on it) | — |

Downloaded 2026-10-23. To refresh: download the texts into a folder,
check the digests changed for a reason, update them in `extract.py`, and
run `python3 -I tools/vectors/vault-primitives/extract.py FOLDER`.
