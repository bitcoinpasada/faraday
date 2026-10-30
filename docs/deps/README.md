# Dependencies

`just lint` runs `cargo vet --locked` (F71): every outside crate is
trusted by a publisher F71 names (rust-bitcoin's, RustCrypto's, dalek's
and others), covered by an imported audit, or an exemption whose page
in `docs/deps/` says "not reviewed". Every direct dependency has a page
there.
`Cargo.lock` pins every source by its checksum, and every build and
check passes `--locked`. `cargo deny check`, also in `just lint`, checks
advisories, licences, duplicates and sources against `deny.toml`.
