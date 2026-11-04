# QR vault vectors

QR vaults in the format of `docs/VAULT.md` §1.2 and §2.1 (an OpenPGP
message, RFC 9580), and the messages a reader refuses. They are Faraday's
own: made by this build, not by an outside program, so they are no outside
check of the format. Their own check is `tools/vault/open.py`, which opens
them with Sequoia's `sq decrypt` (`faraday/faraday-vault/tests/open_py.rs`);
Faraday's Rust reads them in `faraday/faraday-vault/tests/qr.rs`.

Every vault is 396 bytes, sealed at 64 MiB of memory (exponent 16, the
least a seal writes), 3 passes and 4 lanes. The passphrases and random
bytes are fixed and public; the words are BIP-39's test phrases, and no
coin is behind anything here.

| File | Passphrase | What it holds |
|---|---|---|
| `qr-seed-12.pgp` | `correct horse battery staple` | a 12-word seed (all `abandon` but the last word) |
| `qr-seed-24.pgp` | the same | a 24-word seed (`zoo` 23 times, then `vote`) |
| `qr-codex32.pgp` | the same | BIP 93 vector 2's share and the fingerprint of its key (`fab6868a`: the master key of the secret the share's set makes, `d1808e096b35b209ca12132b264662a5`) |
| `qr-passphrase.pgp` | the same | a BIP-39 passphrase and the fingerprint of its key |
| `qr-note.pgp` | the same | a note |
| `qr-empty.pgp` | the same | no item |
| `qr-nfd.pgp` | `café` in NFD (U+0065 U+0301) | a note; opens under the NFC form (U+00E9) too (§3) |
| `refused-skesk4-seipd1.pgp` | none: no passphrase opens it | a SKESK v4 and a SEIPD v1 as GnuPG writes their headers, with filler for a body (§1.2) |
| `refused-second-skesk.pgp` | `correct horse battery staple` | two SKESK packets |
| `refused-compressed.pgp` | `correct horse battery staple` | a compressed data packet where the SEIPD goes |
| `refused-trailing.pgp` | `correct horse battery staple` | a packet that is not padding after the SEIPD |

The refused files are 396 bytes too, with plaintexts shortened to fit; each
is refused by Faraday's Rust (`tests/qr.rs`) for the rule its name gives, before anything is decrypted. `sq` is not the check of that: `open.py` reads what `sq` opens.

How they are made:

    cargo run --locked -p faraday-vault --example qr_vectors -- tools/vectors/vault/qr

writes every file byte for byte as committed
(`faraday/faraday-vault/examples/qr_vectors.rs`).

By hand, from a clone of this repository:

    printf %s 'correct horse battery staple' > pw
    sq decrypt --password-file pw --signatures 0 tools/vectors/vault/qr/qr-note.pgp
