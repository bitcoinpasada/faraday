# miniz_oxide

**Purpose.** BBQr's `Z` encoding is raw DEFLATE under base32, and it is
the encoding Faraday OS and Coldcard use for anything of size, so reading
their transfers (`faraday/faraday-qr`, `docs/QR.md` §4) needs an inflater.
Only inflating: Faraday writes BBQr in encoding `2`, so nothing it sends
needs a decompressor at the other end, and nothing here compresses.

**Why this crate.** It is already in the graph: `png`, which reads QR
codes from PNG files in `faraday-files`, inflates its image data with
`miniz_oxide` 0.8. Taking the same version adds no crate to the build.
It is pure Rust, `#![forbid(unsafe_code)]`, and
`decompress_to_vec_with_limit` stops at a bound, which is how the 360 KiB
limit on a transfer is kept against a compression bomb.

**Cost.** None beyond the edge: `miniz_oxide` 0.8 with
`default-features = false, features = ["with-alloc"]`, the version `png`
resolves to.

**A second edge.** `faraday/faraday-apk` inflates an APK's
`AndroidManifest.xml` (`docs/APK-SIGNING.md` §5), up to 1 MiB, through
the crate's `core` interface so that it knows the stream ended and how
many bytes it used: the same `miniz_oxide` 0.8 with the same features, so
no crate enters the graph.

**Reopen when** `png` leaves the graph or moves to another major version
of `miniz_oxide`; then weigh the inflater on its own.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71).