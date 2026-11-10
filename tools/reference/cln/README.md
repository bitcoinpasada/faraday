# Core Lightning's own code, for the hsm_secret

The cross-check's `lightning` area asks Core Lightning's code what it
makes of a node's secret, and libsodium what it makes of an encrypted
hsm_secret, and compares the answers with Faraday's
(`tools/crosscheck/area_lightning.py`). `tools/crosscheck/run.sh` compiles
`main.c` with `cc` and the system libsodium (`libsodium-dev`) into one
small program that answers a line at a time.

| Part | What | Source | Version | Licence |
|---|---|---|---|---|
| Core Lightning | `common/hsm_secret.c`: the salt, limits and calls of `get_encryption_key`, `decrypt_hsm_secret` and `encrypt_legacy_hsm_secret`; `hsmd/libhsmd.c`: `node_key`, which cuts the seed to its first 32 bytes | <https://github.com/ElementsProject/lightning> | tag `v26.06.10`, commit `e8b1609e67575036bd80aec32605e830f8f651cc` | MIT |
| ccan | `crypto/sha256`, `crypto/hmac_sha256`, `crypto/hkdf_sha256`, `compiler/compiler.h`, `endian/endian.h`: the HKDF-SHA256 hsmd derives the node key with | the same commit's `ccan/` | the same | BSD-MIT, CC0 (`ccan/licenses/`) |
| libsodium | Argon2id (`crypto_pwhash`) and `crypto_secretstream_xchacha20poly1305`, linked from the system, not vendored | <https://libsodium.org> | the installed version, recorded in the stamp | ISC |

## What is vendored, and what is ours

- `ccan/`, `common/hsm_secret.c` and `hsmd/libhsmd.c` are copied byte for byte from the
  pinned commit. The HKDF files are compiled as they are.
- `common/hsm_secret.c` is not compiled: it needs Core Lightning's `tal`
  allocator and libwally. It is kept so that the functions `main.c`
  writes out can be read against it.
- `config.h` and `main.c` are Faraday's. `main.c` writes out
  `get_encryption_key`, `decrypt_hsm_secret` and `encrypt_legacy_hsm_secret`
  from `common/hsm_secret.c` with the same salt (`c-lightning` and five
  zero bytes), the same moderate limits and the same libsodium calls, the
  `tal` allocation and `mlock` of the secret left out. The node key is
  `node_key` of `hsmd/libhsmd.c` (HKDF-SHA256, info `nodeid`, a 4-byte
  salt counting from 0), which `main.c` calls through the compiled ccan
  code and the area retries on the curve. `hsmd/libhsmd.c` is not
  compiled either (it needs Core Lightning's `tal`, secp256k1 and wire
  code); it is the source of the 32-byte cut: `node_key` keys the HKDF
  with the first 32 bytes of `bip32_seed` ("Use first 32 bytes for node key
  derivation"), so `main.c` and the area take the first 32 bytes of the
  64-byte seed of the mnemonic form and of typed words.
- Core Lightning reads every other length over 32 as a mnemonic-form
  hsm_secret. `main.c` writes out the checks of `extract_mnemonic_secret`
  (the zero hash, the passphrase required or refused, the seed's SHA-256
  compared with the stored hash, ccan's `sha256`), and takes from the area
  what libwally gives it, which is not compiled: whether the words
  validate (`validate_mnemonic`) and their seed (`bip39_mnemonic_to_seed`),
  python-mnemonic's in the area. A NUL in the text is not compared: Core
  Lightning's `tal_strndup` stops at it and Faraday refuses the file.

`tools/crosscheck/references.sha256` holds the hash of each file.
