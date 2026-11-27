# aezeed and Lightning node key vectors

Gathered from `tools/reference/aezeed/README.md`, which names the
sources, their commits and how each answer was produced, for the
cross-check's `lightning` area (`tools/crosscheck/area_lightning.py`).

| Key | What |
|---|---|
| `aezeed` | LND's two published cipher seeds, enciphered at scrypt N = 16 as its test package sets it, and the same seeds with a third (internal version 1) at a released LND's N = 32768; each decodes to `entropy` |
| `lnd_node` | LND's node identity key, `m/1017'/coin'/6'/0/0` of `entropy`, on mainnet (coin 0) and elsewhere (coin 1) |
| `ldk_node` | ldk-node's node keys of five BIP-39 phrases, 12 and 24 words, with and without a passphrase, from `lightning` 0.2.6's `KeysManager` |
