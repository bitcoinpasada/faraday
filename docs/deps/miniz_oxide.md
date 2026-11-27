# miniz_oxide

**Purpose.** BBQr's `Z` encoding is raw DEFLATE under base32, and it is
the encoding Faraday OS and Coldcard use for anything of size, so reading
their transfers (`core/faraday-qr`, `docs/QR.md` §4) needs an inflater.
Only inflating: Faraday writes BBQr in encoding `2`, so nothing it sends
needs a decompressor at the other end.

**Why this crate.** It is already in the graph: `png`, which reads QR
codes from PNG files in `faraday-files`, inflates its image data with
`miniz_oxide` 0.8. Taking the same version adds no crate to the build.
It is pure Rust, `#![forbid(unsafe_code)]`, and
the 360 KiB limit on a transfer is kept against a compression bomb by
inflating into one wiping buffer of that size with `decompress` (anything
but a finished stream is refused) and copying the result once, exact size,
into another: `decompress_to_vec_with_limit` grows its output and frees the
smaller blocks unwiped, which a seed sent as BBQr must not leave.

**Cost.** None beyond the edge: `miniz_oxide` 0.8 with
`default-features = false, features = ["with-alloc"]`, the version `png`
resolves to.

**A second edge, built.** `faraday-vault` takes it as a runtime dependency to read the gzip body of a
KDBX database a KeePass app saved with compression (`docs/VAULT.md`
§7.1): the raw DEFLATE stream is inflated into a buffer sized from the
length the gzip trailer states, which is bounded by the largest vault
(4 MiB) before it is allocated, and the CRC-32 and the length are
checked.

**A third edge, built.** `faraday-kef` takes it to inflate only: Krux's
compressed codes (versions 7, 12, 16 and 21, `docs/KEF.md` §4.1, §10) are
read, never written, by `decompress` into one buffer of 100 KiB, wiped, and
copied into one of the result's exact size, so the output does not grow
and leave a freed copy of the contents.

**What is wiped.** The decompressor (`DecompressorOxide`) holds up to 64 bits
of the compressed stream and, for a dynamic-Huffman block, its code lengths.
`faraday-kef` overwrites it with a fresh one as it drops; the BBQr and KDBX
gzip inflaters drop theirs as it is.

**Reopen when** `png` leaves the graph or moves to another major version
of `miniz_oxide`; then weigh the inflater on its own.

**Review.** Version 0.8.9, the one this edge takes, is covered by audits imported from other organisations (`supply-chain/imports.lock`); not read here. Version 0.9.1, which `flate2` takes for `png`'s encoder, is an exemption, not reviewed by a human; its diff from 0.8.9 was read by an AI model for the project, recorded under `ai-says-safe-to-deploy`, which grants no trust (F71, `supply-chain/audits.toml`).