# libfuzzer-sys

**Purpose.** The fuzz targets: every file under `fuzz/fuzz_targets/` and
`faraday/fuzz/fuzz_targets/` is a `fuzz_target!` that `cargo fuzz` builds
against LLVM's libFuzzer, which this crate links and drives
(`fuzz/README.md`, `faraday/fuzz/README.md`). It brings `arbitrary`, which
the targets here do not use, and `cc`, already in the graph for
`secp256k1-sys` (`docs/deps/cc.md`).

**Where it ships.** Nowhere. It is a dependency of the two fuzz crates
only, which are workspaces of their own, excluded from the root one. No
image, app or release file is built from them, and `just lint` holds that
no shipped binary compiles `Compute::unchecked`, which the targets use.

**Why this crate.** It is what `cargo fuzz` generates and requires. There
is no smaller way to link libFuzzer into a Rust target.

**Version.** `libfuzzer-sys = "0.4"` in each fuzz crate's `Cargo.toml`,
locked at 0.4.13 in each crate's committed `Cargo.lock` with `arbitrary`
1.5.0. Each lock starts from the root `Cargo.lock`, so every crate the
fuzz targets share with the shipped code is the shipped version.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71)
for it and for `arbitrary`. `just lint` runs `cargo vet --locked` on each
fuzz crate against the same store as the workspace.

**Reopen when** a fuzz crate is built into anything that ships, or when the
targets move to a harness other than libFuzzer.
