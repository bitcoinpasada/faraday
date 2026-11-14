# Dependencies

`just lint` runs `cargo vet --locked` (F71): every outside crate is
trusted by a publisher F71 names (rust-bitcoin's, RustCrypto's, dalek's
and others), covered by an imported audit, or an exemption whose page
in `docs/deps/` says "not reviewed". Every direct dependency has a page
there.
Most entries are exemptions: on 2026-11-14, 298 exemptions beside 103
trusted-publisher entries and 16 audits of Faraday's own. An exemption is
a crate taken at the version it was added, not a review of it.
`Cargo.lock` pins every source by its checksum, and every build and
check passes `--locked`. `cargo deny check`, also in `just lint`, checks
advisories, licences, duplicates and sources against `deny.toml`.
