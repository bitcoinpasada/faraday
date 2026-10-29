# Fuzzing

Eighteen `cargo fuzz` targets, one per parser of the shared crates that
reads something the device did not write: a camera frame, a scanned
payload, a PSBT from a coordinator, a key share, a file on storage. The
parsers in Faraday's own crates have theirs in `faraday/fuzz/`
(`faraday/fuzz/README.md`); `just fuzz-list` lists both.

The 2026-09-11 security review item M5 is why they exist. The Pi
shell is the only process on the board and is built with
`panic = "abort"`, so a panic in any of these is a poweroff in the middle
of a signing session.

This runs on a developer's machine and nowhere else. There is no hosted
CI and no GitHub Actions. The run before a release is `just fuzz-release`
(below, and `tools/README.md`, "Before a release").

## Setup

`cargo-fuzz`, and the dated nightly that `fuzz/rust-toolchain.toml` (and
`faraday/fuzz/rust-toolchain.toml`, which must say the same) pins, both at
user level; the workspace itself stays on its pinned stable toolchain. A
dated nightly keeps a run reproducible from its commit.

```
cargo install cargo-fuzz --locked
rustup toolchain install nightly-2026-10-13 --profile minimal
```

(The date is the one in the file; the recipes read it from there.)

## Running

```
just fuzz-list                 the targets of both directories
just fuzz classify             one target, 60 seconds
just fuzz decode_luma 300      one target, 300 seconds
```

`just fuzz` finds the directory the target is in and runs
`cargo +NIGHTLY fuzz run --fuzz-dir DIR TARGET -- -max_total_time=N` from
the repository root, NIGHTLY being the pinned one. Run `cargo +NIGHTLY fuzz`
with `--fuzz-dir fuzz` directly for anything libFuzzer can do that the
recipe does not pass through. The one worth knowing:

```
cargo +nightly-2026-10-13 fuzz run --fuzz-dir fuzz decode_luma -- -max_total_time=300 -fork=1 -ignore_crashes=1
```

`-fork=1 -ignore_crashes=1` keeps going after a crash instead of exiting
on the first one, which is what to use while a known crash is still open.
None is, as of 2026-10-23: a plain `just fuzz` on any target runs its
whole time.

This crate is its own workspace, so a sanitizer build shares neither the
lock file nor the target directory with the rest of the repository.
Everything it builds lands in `fuzz/target`. Its `Cargo.lock` is committed,
and `cargo-fuzz` has no `--locked` of its own, so the recipes run `cargo
metadata --locked` on it first: a lock that is out of date fails instead of
being changed. Its dependencies are vetted with the workspace's
(`docs/deps/libfuzzer-sys.md`; `just lint` runs `cargo vet --locked` on it).

To regenerate the lock (a dependency added or the workspace's lock
changed): copy the root `Cargo.lock` over it, run `cargo +NIGHTLY metadata
--manifest-path fuzz/Cargo.toml`, and commit it. Starting from the root lock
keeps every crate the targets share with the shipped code at the shipped
version, and keeps a newer release candidate of a pinned crate (the
`pkcs1` that breaks `rsa`) out. A new external crate then needs an
exemption in `supply-chain/config.toml` and a page in `docs/deps/`.

## Before a release

```
just fuzz-release              every target, 10 minutes each
just fuzz-corpus-export        then their corpora, minimized
```

`just fuzz-release [SECONDS]` runs `tools/fuzz-release.sh`: every target of
`fuzz/` and `faraday/fuzz/` for SECONDS each (600), as many at once as
there are cores. Any crash, out-of-memory or timeout fails it and names
the input. It is run on the build machine, whose cores are the point, and
the record is taken from there.

Each target keeps the corpus it has grown between runs, in
`~/.cache/faraday-fuzz/<target>/` on the machine that runs it (a synced
checkout is replaced each time, so the corpus is not kept in it); the
committed seeds in `corpus/<target>/` are read as well. With
`FARADAY_FUZZ_CORPUS=DIR`, the inputs in `DIR/<target>/` that add coverage
are merged into that corpus first: `DIR` is a checkout of the published
corpora, or `out/fuzz-corpus/`. The record, `out/fuzz-release.txt`, has
the date, the commit, the nightly and cargo-fuzz versions, the corpus the
run started from (its folder name and commit) and, for each target, its
seconds run, executions, corpus entries and bytes, and "no crashes". The
release notes quote it.

`just fuzz-corpus-export` (`tools/fuzz-corpus-export.sh`) minimizes each
target's kept corpus and seeds with `cargo fuzz cmin` into
`out/fuzz-corpus/<target>/`, and the store keeps that
minimized set in place of what it held, so the next run loads only it.
The folder is to be committed to the repository that
publishes the corpora, https://github.com/bitcoinpasada/faraday-fuzz-corpus
(one commit per release, tagged with the release's tag).

**Reproducing a release's run.** Check out the release's commit. Install
the nightly `fuzz/rust-toolchain.toml` names and the cargo-fuzz version
the record gives (`cargo install cargo-fuzz --locked --version X`). Check
out the corpus repository at the commit the release notes quote, and run
`FARADAY_FUZZ_CORPUS=<that checkout> just fuzz-release`. The locks are
committed, so the dependencies are the release's.

## Layout

- `fuzz_targets/` — one file per target.
- `corpus/<target>/` — the seed inputs, committed, all with an extension.
  A run adds its own entries beside them, named after their SHA-1 and
  ignored by git. (`just fuzz-release` grows its corpus elsewhere, above.)
- `dict/<target>.dict` — a libFuzzer dictionary, which `just fuzz` and
  `just fuzz-release` pass with `-dict=` (either fuzz directory may have
  one): the words a text format is made of. `classify` and `policy` have
  one; in two 60-second runs from the seeds, `classify` reached 12 to 17%
  more coverage with its dictionary, `policy` about 10% (within the
  spread between repeats). BIP-39's words for `bip39_words` and the UR
  words for `ur_receive` made no measurable difference, so those two
  have none (2026-10-29).
- `artifacts/<target>/` — an input that crashed. Committed ones have an
  extension and a name: they are the minimized reproducer of a crash,
  kept while it is open and kept after it is fixed, since an input that
  once killed the device is the one worth running again. What libFuzzer
  writes for itself (`crash-<sha1>`, `oom-…`, `slow-unit-…`) is ignored.

Reproduce a saved artifact with:

```
cargo +nightly fuzz run --fuzz-dir fuzz decode_luma fuzz/artifacts/decode_luma/rqrr-perspective-map.bin
```

## The targets

| Target | Entry point |
|---|---|
| `classify` | `faraday_codec::classify` |
| `seedqr` | `faraday_codec::seedqr::from_digits`, `from_entropy` |
| `ur_receive` | `faraday_codec::ur::Decoder::receive`, one part and a sequence |
| `psbt_parse` | `faraday_psbt::Psbt::parse_bytes` / `parse_base64`, then `inspect` with no key loaded and with the BIP-39 test vector key, then `finalize` on a copy |
| `policy` | `faraday_bip::policy::WalletPolicy::parse_any` |
| `message_verify` | `faraday_psbt::message::verify` over the test vector key's four addresses; `parse_signed` on the whole input, its result verified |
| `xkey` | `faraday_bip::xkey::decode_xpriv` / `decode_xpub`, `slip132::decode_xpub` |
| `pgp_armor` | `faraday_pgp::read_public` |
| `pgp_verify` | `faraday_pgp::verify_detached` with the input as the signature, the signed data and the certificate, against the GnuPG-made release-sums fixture (F134): what it takes is the fixture's own signature over the fixture's sums |
| `pgp_paperkey` | `faraday_pgp::paperkey::read_text`, `read_raw`, `line` and `check` on what reads, then `entries` |
| `pgp_restore` | `faraday_pgp::Certificate::read` on the whole input, then `restore` with the input as a paperkey's data against what read and against a Faraday key's certificate, and that key's own data with the input laid over it |
| `json` | `faraday_bip::json::parse`, written back with `quote` and read again; `faraday_bip::coldcard::parse` |
| `decode_luma` | `faraday_codec::decode_luma`, the frame's width and height taken from the first two bytes |
| `seed_parts` | SLIP-39 `Share::parse` and codex32 `Codex32::parse` on every line, then `recover` over what parsed (SLIP-39 only at iteration exponent 0, so a run stays fast) |
| `key_file` | a cosigner's key file: BSMS `SignerRecord::parse` and `verify`, `coldcard::cosigner_keys`, `slip132::key_line`, `PolicyKey::parse` |
| `transaction` | `faraday_psbt::transaction::read_raw` and `Spending::read` |
| `encodings` | `faraday_codec::encodings::read`, `from_hex` and `read_input` under each way of reading |
| `aezeed` | `faraday_bip::aezeed::decode_at_cost` at a cost of 16 over 24 word indices and a passphrase taken from the input |

One of them shapes the input before the call, because the code under
test rejects anything else in its first few lines and a fuzzer would
never reach the interesting part: `decode_luma` needs
`luma.len() == width * height`, so the target takes the two dimensions
from the first two bytes (1 to 256 each) and pads or truncates the rest
to fit.

`settings_read` (the deleted app's settings file) and `keep_open` (its kept-key
blob, `osk-keep`) went with the code they read when Faraday deleted
that app (`docs/DECISIONS.md` F68); the record below is kept as
it was run.

## 2026-09-11: what was run and what it found

Every target for at least 60 seconds, `decode_luma` and `psbt_parse` for
300, on one core each.

| Target | Time | Execs | Coverage | Result |
|---|---|---|---|---|
| `classify` | 60 s | 399 k | 811 | clean |
| `seedqr` | 60 s | 33.9 M | 272 | clean |
| `ur_receive` | 60 s | 1.62 M | 836 | clean |
| `policy` | 60 s | 6.58 M | 934 | clean |
| `message_verify` | 60 s | 1.95 M | 596 | clean |
| `settings_read` | 60 s | 4.42 M | 235 | clean |
| `xkey` | 60 s | 751 k | 303 | clean |
| `psbt_parse` | 300 s | 4.89 M | 4275 | clean |
| `decode_luma` | 300 s | 71.4 k | 1437 | crashed, one cause |
| `keep_open` | 120 s | — | 1296 | crashed, one cause |

### `rqrr` aborts on a hostile camera frame

`artifacts/decode_luma/rqrr-perspective-map.bin` (620 bytes, a 33×33
frame) and `rqrr-perspective-map-negative.bin` (1820 bytes).

`rqrr` 0.11.0 `Perspective::map` (`src/geometry.rs:55-58`) rounds the
transformed point and asserts it fits in an `i32`. A frame with a
near-degenerate capstone group drives the denominator towards zero, the
coordinate past `i32::MAX` or below `i32::MIN`, and the assertion fires.
The path is `PreparedImage::detect_grids` → `find_groupings` →
`Grid::from_group` → `setup_perspective` → `jiggle_perspective` →
`fitness_all` → `fitness_capstone` → `fitness_ring` → `fitness_cell`
(`src/identify/grid.rs:453`) → `Perspective::map`, so this is grid
detection and it happens before any payload is read. Ours is
`faraday_codec::decode_luma` (`core/faraday-codec/src/decode.rs:34`), which calls
`detect_grids` and has no way to refuse the frame first.

These are `assert!`, not `debug_assert!`, so a release build panics too.
On the Pi, with `panic = "abort"` and no process to catch it, a printed
sheet of paper held in front of the camera powers the device off. The
fuzzer reached it in 50 seconds from the QR fixtures as seeds.

**Fixed.** `rqrr` is vendored at `third_party/rqrr`, patched in from the
root manifest and from this crate's own `Cargo.toml`. `map` returns
`Option<Point>` and every caller — the fitness functions, the timing and
alignment scans, the bit sampler, `detect_grids` itself — rejects the
candidate grid instead of asserting, so a degenerate frame yields no grid
the way a frame with no code in it does. `third_party/rqrr/PATCHES.md`
has the detail. Both artifacts
run clean, and the two are kept as regression inputs.

### The kept-key blob's Argon2 cost is the blob's own word

`artifacts/keep_open/argon2-cost-unclamped.bin` (4 bytes) and
`argon2-cost-stall.bin` (8 bytes).

`keep::stretch` in the deleted app's core crate (`src/keep.rs:406-430`) builds
`Params::new(header.cost.memory_kib, header.cost.passes,
header.cost.lanes, …)` from the blob and then allocates
`vec![Block::default(); params.block_count()]`. Both numbers come from
the file. `header()` checks only the length and the version byte, so a
blob claiming 16 GiB of Argon2 memory asks for a 16 GiB allocation the
moment a PIN is typed against it; the 4-byte artifact does exactly that,
and the allocation failure aborts. The 8-byte artifact asks for 448 MiB
and 257 passes instead, which allocates but does not return: the device
hangs inside one PIN attempt.

Whoever can write the blob is on the wrong side of the Tier A threat
model already, but the cost of a wrong file here is the shell dying or
hanging rather than "that is not a blob this build wrote".

**Fixed.** `keep::header` returns `None` unless the memory, the passes
and the lanes are each between the `argon2` crate's minimum and
`DEVICE_PARAMS`, so a file that asks for work this build would not have
done is not a file this build reads. Both artifacts run clean, and both
headers are a unit test in core/osk-keep/src/lib.rs.

### The re-run

With both fixes in, on the same machine and the corpus the first pass
left behind:

| Target | Time | Execs | Coverage | Result |
|---|---|---|---|---|
| `decode_luma` | 300 s | 45.9 k | 1451 | clean |
| `keep_open` | 120 s | 22.0 k | 1093 | clean |

Each committed artifact also runs clean on its own, which is
the cheap check to repeat before touching the QR decoder:

```
cargo +nightly fuzz run --fuzz-dir fuzz decode_luma fuzz/artifacts/decode_luma/rqrr-perspective-map.bin
```

`keep_open` covers less than it did, because `header` now rejects a cost
the device would not have written before anything else looks at the blob.
