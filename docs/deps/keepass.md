# keepass

**Purpose.** keepass-rs, the Rust KeePass library, as an outside reader and
writer of the vault's KDBX file in the cross-check (F193; `docs/ESTABLISHED.md`
§9.5). `tools/crosscheck/keepass-rs/` is a small program that reads a KDBX
file under a passphrase and prints it as JSON, and writes one from a JSON
description; the `vault` area runs it beside pykeepass (`tools/crosscheck/keepass_rs.py`).
It judges the file format only: it runs RustCrypto, as the app does, so it
is not counted for the primitives. Its tree is 94 packages in the program's
own `Cargo.lock`. It reads and writes the JSON with `serde_json`, which has
its page (`docs/deps/serde_json.md`).

**Where it ships.** Nowhere. The program is a workspace of its own, excluded
from the root one, built by `tools/crosscheck/run.sh` into `out/` and run by
the cross-check on a developer's or the build machine. `cargo tree -i keepass`
in the root workspace finds no package: no app, image or release file
links it.

**Why this crate.** A reader and writer of KDBX 4 that Faraday's authors did
not write, in the language of the code it judges; pykeepass and
`keepassxc-cli` are the other two. No smaller library reads the format.

**Version.** `keepass = "=0.17.0"` with default features off and
`save_kdbx4` on, pinned exactly, locked in the program's committed
`Cargo.lock`. It also reads Twofish, AES-KDF and Salsa20, which Faraday
refuses (`docs/VAULT.md` §4); the area makes such files with it and expects
Faraday to refuse them.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71)
for it and for every package of its tree that nothing else in the store
covers. `just lint` runs `cargo vet --locked` on the program's workspace
against the same store as the others (the `cargo-vet-crosscheck-keepass-rs`
check), and `just vet-refresh` keeps its exemptions.

**Reopen when** the program is built into anything that ships, or when a
change of the library's reading makes the area's known differences
(`tools/crosscheck/README.md`) wrong.
