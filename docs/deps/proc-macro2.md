# proc-macro2

**Purpose.** Token types for `tools/costs-lint`, which parses the
workspace's Rust sources with `syn` and reports who takes, mints and keeps
the compute token (`docs/COMPUTE.md` §8). `proc-macro2` is `syn`'s token
layer; the lint uses its `span-locations` feature to report a line and
column for each finding.

**Where it ships.** Nowhere. `costs-lint` is a tool run by `just lint`
(`justfile`) and never runs on a device. No crate of the device, desktop or
Starter builds depends on `proc-macro2` directly; it reaches them only if
a procedural macro in a third-party dependency is compiled for the build
machine.

**Why this crate.** It is the only token type `syn` and `quote` accept.
There is no alternative to choose.

**Version and features.** `proc-macro2 = { version = "1",
default-features = false, features = ["span-locations"] }` in
`tools/costs-lint/Cargo.toml`. `span-locations` is the one added feature;
`proc-macro` is off because the lint is an ordinary binary, not a macro.

**Review.** Trusted by publisher (dtolnay) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.

**Reopen when** `costs-lint` stops parsing Rust sources (for instance a
rustc-based lint replaces it).
