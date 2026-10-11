# Fuzzing

Ten `cargo fuzz` targets, one per parser that reads something the device
did not write: a camera frame, a scanned payload, a PSBT from a
coordinator, a file on storage. `docs/PLANNING.md` §12 asks for them;
the 2026-09-11 security review item M5 is why they exist now. The Pi
shell is the only process on the board and is built with
`panic = "abort"`, so a panic in any of these is a poweroff in the middle
of a signing session.

This runs on a developer's machine and nowhere else. There is no hosted
CI, no GitHub Actions, and no release step that depends on it.

## Setup

A nightly toolchain and `cargo-fuzz`, both at user level; the workspace
itself stays on the pinned 1.97.0.

```
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
```

## Running

```
just fuzz-list                 the targets
just fuzz classify             one target, 60 seconds
just fuzz decode_luma 300      one target, 300 seconds
```

`just fuzz` is `cargo +nightly fuzz run TARGET -- -max_total_time=N` in
this directory. Run `cargo` here directly for anything libFuzzer can do
that the recipe does not pass through. The one worth knowing:

```
cargo +nightly fuzz run decode_luma -- -max_total_time=300 -fork=1 -ignore_crashes=1
```

`-fork=1 -ignore_crashes=1` keeps going after a crash instead of exiting
on the first one, which is what to use while a known crash is still open.
None is, as of 2026-09-11: a plain `just fuzz` on any target runs its
whole time.

This crate is its own workspace — the root `Cargo.toml` excludes it — so
a sanitizer build shares neither the lock file nor the target directory
with the rest of the repository. Everything it builds lands in
`fuzz/target`.

## Layout

- `fuzz_targets/` — one file per target.
- `corpus/<target>/` — the seed inputs, committed, all with an extension.
  A run adds its own entries beside them, named after their SHA-1 and
  ignored by git.
- `artifacts/<target>/` — an input that crashed. Committed ones have an
  extension and a name: they are the minimized reproducer of a crash,
  kept while it is open and kept after it is fixed, since an input that
  once killed the device is the one worth running again. What libFuzzer
  writes for itself (`crash-<sha1>`, `oom-…`, `slow-unit-…`) is ignored.

Reproduce a saved artifact with:

```
cd fuzz && cargo +nightly fuzz run decode_luma artifacts/decode_luma/rqrr-perspective-map.bin
```

## The targets

| Target | Entry point |
|---|---|
| `classify` | `osk_codec::classify` |
| `seedqr` | `osk_codec::seedqr::from_digits`, `from_entropy` |
| `ur_receive` | `osk_codec::ur::Decoder::receive`, one part and a sequence |
| `psbt_parse` | `osk_psbt::Psbt::parse_bytes` / `parse_base64`, then `inspect` with no key loaded and with the BIP-39 test vector key |
| `policy` | `osk_bip::policy::WalletPolicy::parse_any` |
| `message_verify` | `osk_psbt::message::verify` over the test vector key's four addresses |
| `settings_read` | `opensigner_core::settings::read` |
| `keep_open` | `osk_keep::header`, `challenge`, `open`, `count_attempt` |
| `xkey` | `osk_bip::xkey::decode_xpriv` / `decode_xpub`, `slip132::decode_xpub` |
| `decode_luma` | `osk_codec::decode_luma`, the frame's width and height taken from the first two bytes |

Two of them shape the input before the call, because the code under test
rejects anything else in its first few lines and a fuzzer would never
reach the interesting part:

- `decode_luma` needs `luma.len() == width * height`, so the target takes
  the two dimensions from the first two bytes (1 to 256 each) and pads or
  truncates the rest to fit.
- `keep_open` needs a 262-byte blob whose records authenticate, so the
  target writes one at start-up under a known PIN and a known element tag
  and lays the input over the front of a copy. A short input tampers with
  the header of two valid records; a full-length one replaces everything.
  `keep::header` is also called on the raw input, unshaped, which is the
  gate an arbitrary file on the data partition actually meets.

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
`osk_codec::decode_luma` (`core/osk-codec/src/decode.rs:34`), which calls
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
the way a frame with no code in it does. `third_party/rqrr/OPENSIGNER.md`
has the detail, `docs/PLANNING.md` §16.59 the decision. Both artifacts
run clean, and the two are kept as regression inputs.

### The kept-key blob's Argon2 cost is the blob's own word

`artifacts/keep_open/argon2-cost-unclamped.bin` (4 bytes) and
`argon2-cost-stall.bin` (8 bytes).

`opensigner_core::keep::stretch` (`src/keep.rs:406-430`) builds
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
headers are a unit test in `core/osk-keep/src/lib.rs`.

### The re-run

With both fixes in, on the same machine and the corpus the first pass
left behind:

| Target | Time | Execs | Coverage | Result |
|---|---|---|---|---|
| `decode_luma` | 300 s | 45.9 k | 1451 | clean |
| `keep_open` | 120 s | 22.0 k | 1093 | clean |

Each of the four committed artifacts also runs clean on its own, which is
the cheap check to repeat before touching either parser:

```
cd fuzz && cargo +nightly fuzz run decode_luma artifacts/decode_luma/rqrr-perspective-map.bin
```

`keep_open` covers less than it did, because `header` now rejects a cost
the device would not have written before anything else looks at the blob.
