# paperkey fixtures

What paperkey 1.6 and GnuPG 2.4 make, for the restore of a GPG key from
its paperkey (`docs/GPG-PAPERKEY.md` §8). No file is copied from
paperkey's repository: paperkey's own check keys (`checks/papertest-*`)
are GPL-2.0, so the keys here are made by GnuPG, one of them of the same
shape as `papertest-eddsa`.

`make.sh` made every file, with paperkey 1.6 and GnuPG 2.4.8 on the build
machine, from the repository root:

    tools/vectors/paperkey/make.sh

Each `gpg` call runs in a throwaway home with `--faked-system-time
1780000000!` (2026-05-28), so every key was made at that time. The keys
are random: a new run makes other keys, and every file but
`faraday-key.raw` changes (paperkey's comments carry the day it ran).
None of these keys guards anything.

| File | Made by | SHA-256 | Read by |
|---|---|---|---|
| `faraday-key.txt` | `paperkey --secret-key` of the Faraday key `examples/paperkey_inputs.rs` writes (seeds with leading zero bytes), comments included | `d74f960d834041d661f511062348eaff4f55f95cc3e5b105431b32072c5ab2e7` | test 2 |
| `faraday-key.raw` | the same with `--output-type raw` | `322f484f674dec9befe2acdbe8b003f9d6d51f1c934ab22e7129f1f124ac5c38` | tests 2 and 4 |
| `gpg-default.asc` | `gpg --quick-generate-key 'Default <default@example.com>'` (an Ed25519 primary and a cv25519 encryption subkey), `gpg --armor --export` | `8a508ec6265452dd65f0f1164da522463afec206f60e057d78f88a3ff39e09e7` | test 6 |
| `gpg-rsa3072.asc` | `gpg --quick-generate-key 'Rsa <rsa@example.com>' rsa3072`, `gpg --armor --export` | `a2c78f4ea6b029db8db06f116454ae9129da3cfb8199db206f4f9424674bb160` | test 7 |
| `gpg-protected.asc` | `gpg --passphrase 'test passphrase' --quick-generate-key 'Protected <protected@example.com>' ed25519 cert never`, `gpg --quick-add-key FPR ed25519 sign never`, `gpg --armor --export` (key `DBA4AF1787A098FB07AD679BC479EDCFB954B2E6`) | `d7c2f3fc13b32dd9cf69e8ee9a02964c52e6b4c07ec83fb38e64e43678ee9b0d` | test 8 |
| `gpg-protected.txt` | `gpg --export-secret-keys` of that key, then `paperkey --secret-key` | `1cffd32e2443f068f5008c487f1d474f84ca665c4e9c30e74fd88b4bba0710ad` | test 8 |
| `gpg-unprotected.asc` | as `gpg-protected.asc`, with an empty passphrase (key `846AF8B7D8B862ED05A47A1BB9E86C4571ECB9E2`) | `ff9d1a895dc39fa81d78f85c7dffdd87b88ac25c24ddb4273b5dceee4fcb7a8a` | test 9 |
| `gpg-unprotected.txt` | its secret key through `paperkey --secret-key` | `8d56394f82de7a6d1b63e57e54f5ce1c04f68b1b655456ce713d400f1a1e30e6` | test 9 |
| `gpg-no-subkey.asc` | `gpg --quick-generate-key 'Single <single@example.com>' ed25519 cert,sign never` (the shape of paperkey's `papertest-eddsa`), `gpg --armor --export` | `dfa29df5d7e343bd4854443d90bc569ba26d593407ff753a34d494d280693133` | test 5 |
| `gpg-no-subkey.txt` | its secret key through `paperkey --secret-key` | `0b4bf204563c1c07417b6d14ae8171538ab8c8014b6d07045f9d6c85effc94a5` | test 5 |
| `make.sh` | written here | `ab6832ea6e7cb171df421e5cd1834d053600977d2e6906e7cc1de618ea5b9a86` | people |

The tests are in `faraday/faraday-pgp/tests/paperkey.rs`. Tests 3, 4,
11 and 13 to 15 run paperkey and GnuPG themselves, on what Faraday
writes, and need no fixture.
