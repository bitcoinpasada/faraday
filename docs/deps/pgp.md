# pgp

**Purpose.** rPGP, the Rust OpenPGP library (`pgp`), with `rand` for its
random source, as an outside reader and sealer of the QR vault's OpenPGP
message in the cross-check (F190 point 11, F193; `docs/ESTABLISHED.md` §9.5,
`docs/VAULT.md` §10). `tools/crosscheck/rpgp/` is a small program that opens
a message under a passphrase and prints the literal data in hex, and seals
one from a passphrase, a memory exponent, a chunk size octet and a plaintext
(an RFC 9580 SKESK v6 and SEIPD v2, AES-256, OCB, Argon2 S2K); the `qrvault` area runs it
beside Sequoia's `sq` and a PyCA judge (`tools/crosscheck/area_qrvault.py`).
It judges the format only: it runs RustCrypto, as the app does, so it is not
counted for the primitives. Its tree is 155 packages in the program's own
`Cargo.lock`.

**Where it ships.** Nowhere. The program is a workspace of its own, excluded
from the root one, built by `tools/crosscheck/run.sh` into `out/` and run by
the cross-check on a developer's or the build machine. `cargo tree -i pgp`
in the root workspace finds no package: no app, image or release file
links it. It is test-only.

**Why this crate.** A writer and reader of RFC 9580 messages that Faraday's
authors did not write, in the language of the code it judges; Sequoia over
Nettle is the other, and `sq` 1.3.1 cannot seal this format, so rPGP is the
one outside sealer. No smaller library writes the version 2 data packet.

**Version.** `pgp = "0.21"` and `rand = "0.8"`, locked in the program's
committed `Cargo.lock` (`pgp` 0.21.0, `rand` 0.8.8), built with `--frozen`
after one `cargo fetch --locked` (`docs/DEVELOPING.md`). `rand` supplies the
random bytes the program's seals draw (the salt, the IV and the session
key) through `rand::thread_rng`; no key of Faraday's is ever made with it.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71,
F211) for it and for every package of its tree that nothing else in the store
covers. `just lint` runs `cargo vet --locked` on the program's workspace
against the same store as the others (the `cargo-vet-crosscheck-rpgp`
check), and `just vet-refresh` keeps its exemptions. The program is test-only
and holds test data only.

**Reopen when** the program is built into anything that ships, or when a
change of the library's reading makes the area's known differences
(`tools/crosscheck/README.md`) wrong.
