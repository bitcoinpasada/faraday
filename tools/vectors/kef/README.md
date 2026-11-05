# Krux's KEF vectors

Test data of Krux's KEF (`docs/KEF.md`) as `vectors.json`, hex throughout.
MIT (`LICENSE.md`, Krux's, from its repository's root).

Source: `https://github.com/selfcustody/krux`, tag `v26.09.0`, commit
`2b20e3370169731a0d89839a4113ad9a1b732078`, files `tests/test_kef.py`
(`CTR_ENCRYPTED_KEF`, `GCM_ENCRYPTED_KEF`, `CTR_ENTROPY`, and the envelopes
of `test_grind_alternate_decryption_key`) and `tests/test_encryption.py`
(`TEST_WORDS_ENTROPY`), fetched as the files of that commit. The values were
copied as Krux writes them and not changed. The passphrases and counts are
the ones those tests open them with. Each answer was also worked out here
by a script of Faraday's over `hashlib` and pyca/cryptography, before
`faraday-kef`'s tests used the file.

| Key | What |
|---|---|
| `ctr` | a version 15 envelope, passphrase `key`, 10,000 rounds, opening to 32 bytes of entropy |
| `gcm` | a version 20 envelope, passphrase `test key`, 100,000 rounds, opening to 16 bytes of entropy |
| `grind` | a version 5 envelope of the id `a1e6c7e4` and 10,000 rounds: `right` opens it to `I'm 16 raw bytes`; `wrong_that_passes` is a wrong passphrase that passes its 3-byte check and gives other bytes |

The ECB and CBC envelopes of the same test files do not authenticate under
the passphrases tried and are not kept.

| File | SHA-256 |
|---|---|
| `vectors.json` | `3a394fa98b0c046c9205de12f9ea3477f74eb31c6b2b834139a85e91142960ba` |
