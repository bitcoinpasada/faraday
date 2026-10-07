# rqrr

**Purpose.** QR detection and decoding from camera luma frames
(`osk-codec::decode`, `docs/PLANNING.md` §4.4: a shell decodes on a worker
beside the core, which keeps the preview and the routing; §16.36).

**Why this crate.** Pure Rust, no `unsafe`, the only maintained Rust QR
*decoder* (the others are `quircs`, a binding to the C library quirc,
and `rxing`, a port of ZXing with a much larger tree). It handles
rotation, perspective and uneven lighting, which a hand-held camera
frame needs, and it returns raw bytes (`decode_to`), which SeedQR's
binary CompactSeedQR payloads need (a NUL byte or a line feed must
survive).

**Cost.** Version 0.11 with `default-features = false` (no `image`), and
since the trim below no dependencies at all: the copy in the tree
compiles against `std` and nothing else. It needs `std`
(`std::io::Write`, `std::error::Error`), so `osk-codec` enables it only
behind its `decode` feature and is the one core crate that is not
`no_std` in that configuration. Recorded as a deviation in
`docs/PLANNING.md` §16.20; the Pi and desktop shells run with `std`, so
nothing ships differently today.

**What the trim took out.** Twelve crates left the graphs with the two
dependencies `rqrr` had: `g2p` (Galois-field arithmetic, with the `g2gen`
proc-macro that generated its tables at build time, and `g2poly`) and
`lru` (with `hashbrown`, `foldhash`, `equivalent` and `allocator-api2`
behind it). `g2gen` was the only proc-macro in a device graph, and with
it went `syn 2`, `quote` and `proc-macro2` from the core, FFI and Pi
graphs. Measured with
`cargo tree -p <root> -e normal --prefix none | sed 's/ (.*//' | sort -u | wc -l`:

| Graph | Before | After |
|---|--:|--:|
| `opensigner-core` | 72 | 60 |
| `opensigner-ffi` | 75 | 63 |
| `opensigner-pi` | 76 | 64 |
| `opensigner-desktop` | 145 | 137 |

Both "before" columns are taken after the `argon2` 0.6 move of the same
day, which took five crates off each graph on its own. The desktop graph
keeps `hashbrown` and `equivalent`, which `indexmap` also brings, and
keeps the proc-macro crates, which `winit` needs.

**Vendored.** Since 2026-09-11 the crate is a copy in the tree,
`third_party/rqrr`, reached through a path in the root manifest's
workspace dependency, so that a project depending on `osk-codec` by git
reaches it too (§16.138). Fuzzing `osk_codec::decode_luma` found
an `assert!` in `Perspective::map` that a camera frame reaches during
grid detection (security review M5a); on the Pi, built with
`panic = "abort"`, that is a sheet of paper that powers the device off,
and the fix had to be in the decoder. `third_party/rqrr/OPENSIGNER.md`
lists what changed and the panic sites still in the copy.

The copy is also where the crate count went: the trim above, and a pass
over the panic sites a hostile frame can reach, which a dependency cannot
be given.

One attribution the security review got wrong: `log` is not a dependency
of this crate and never was. It is in the graph because `tiny-skia` calls
`log::warn!` and has no feature that turns that off.

**Reopen when** a `no_std` decoder appears, or when the bare-metal
target lands and the decoder must move to the shell side of the
contract.
