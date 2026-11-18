# proptest

**Purpose.** The property tests of the backup forms
(`faraday/faraday-keys/tests/round_trips.rs`): for random seeds,
passphrases, share counts and thresholds, every backup form restores to the
same wallet, and a split restores from every sufficient subset and refuses
every insufficient one. It makes the random inputs and shrinks a failing one
to a small case. It brings `rand`, `rand_xorshift`, `regex-syntax`,
`unarray` and `core_detect`; `bitflags`, `num-traits`, `chacha20`,
`getrandom` and `rand_core` were already in the graph.

**Where it ships.** Nowhere. It is a `[dev-dependencies]` entry of
`faraday-keys` and `faraday-plan`, compiled for their tests only, so no
image, app or release file contains it. Its `rand` (0.10) is not the
`rand` the rPGP cross-check program uses (0.8.8).

**Why this crate.** It is the Rust property-testing library the ecosystem
uses; the alternative is a hand-written generator and no shrinking. The
features are cut to `std`: no `fork` and no `timeout` (no child processes,
no `tempfile`, no `wait-timeout`), no `bit-set`. A test
sets a fixed number of cases, a fixed seed and no failure persistence, so a
run is the same everywhere and writes no `proptest-regressions/` files.

**Version.** `proptest = { version = "1.9", default-features = false,
features = ["std"] }` in the root `Cargo.toml`, locked at 1.12.0 with
`rand` 0.10.4, `rand_xorshift` 0.5.0, `regex-syntax` 0.8.12, `unarray`
0.1.4 and `core_detect` 1.0.0.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71)
for each of the six, under `safe-to-run` as the other dev-dependency
(`bip39`) is. No imported audit covers them. `cargo vet --locked` and
`cargo deny check` pass.

**Reopen when** it is asked for a feature beyond `std`, or a shipped crate
depends on it.
