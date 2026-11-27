# adler2

**Purpose.** The Adler-32 checksum that ends the zlib stream of every PNG
picture `faraday_codec::png` writes (RFC 1950 §8.2): a QR code, a
labelled QR picture, a greyscale picture.

**Why this crate.** It replaces a function of Faraday's own (`adler32` in
`core/faraday-codec/src/png.rs`) with the crate `png`'s inflater already
takes, so a checksum that a PNG reader verifies is worked out by code
others wrote and use. Pure Rust, `#![forbid(unsafe_code)]`, no
dependencies.

**Cost.** None beyond the edge: `adler2` 2.0.1 with
`default-features = false` (so `no_std`) is in the lock through
`miniz_oxide`; `faraday-codec` is a new edge to it. The calls are
`Adler32::new`, `write_slice` and `checksum`.

**Reopen when** `miniz_oxide` leaves the graph or moves off `adler2`;
then weigh the checksum on its own.

**Review.** Version 2.0.1 is covered by audits imported from other organisations (`supply-chain/imports.lock`: Google's full audit of 2.0.0, Mozilla's and Zcash's deltas to 2.0.1); not read here.
