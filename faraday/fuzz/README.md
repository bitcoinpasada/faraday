# Fuzzing: Faraday's own targets

Three `cargo fuzz` targets for the parsers that live in Faraday's own
crates, each reading something the device did not write: a stick's
filesystem, an APK, another wallet's file. The parsers in the shared
crates (camera frames, PSBTs, descriptors, keys, JSON) are fuzzed from
`fuzz/` (`fuzz/README.md`).

This runs on a developer's machine and nowhere else. There is no hosted
CI, and no release step depends on it.

## Setup

A nightly toolchain and `cargo-fuzz`, both at user level; the workspace
itself stays on its pinned toolchain.

```
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
```

## Running

From the repository root:

```
just fuzz-faraday-list                  the targets
just fuzz-faraday fat                   one target, 60 seconds
just fuzz-faraday apk 300               one target, 300 seconds
```

`just fuzz-faraday` is `cargo +nightly fuzz run TARGET -- -max_total_time=N`
in this directory. Run `cargo` here directly for anything libFuzzer can do
that the recipe does not pass through.

This crate is its own workspace, so a sanitizer build shares neither the
lock file nor the target directory with the rest of the repository.
Everything it builds lands in `faraday/fuzz/target`.

## The targets

| Target | What it feeds |
|---|---|
| `fat` | `faraday_fat::Volume::open` with the input as a partition image: the boot sector, the FATs and the root directory. Every file listed is read, then a file is created, renamed and deleted, so the write paths walk the same structures. |
| `apk` | `faraday_apk`'s readers over the input: the manifest name check, PEM, the binary manifest, then the ZIP reader and the signing digest. The first byte picks a read on which a source flips a byte, as a stick can change a file between two reads. |
| `read_wallet` | `faraday_intake::wallet::read_wallet` with the input as text: a single-key file, a Caravan config or an Electrum wallet file. The JSON reader under it is fuzzed on its own in `fuzz/` (`json`). |

Inputs that once crashed a target belong in its corpus, so they keep being
run.
