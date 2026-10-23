# png

**Purpose.** Reading and writing PNG pictures with the full format:
decoding a QR picture found on a stick (`read_qr_png` and `qr_in_png` in
`faraday/faraday-files`, which normalises greyscale, palette and 16-bit
pictures to 8-bit before the QR decoder sees them), writing the
snapshot tour's screenshots (`faraday/shells/snapshot`), and reading back
what `faraday_codec::png` writes in tests (`core/faraday-codec`,
`faraday/faraday-starter`, dev-dependencies only).

**Where it ships.** `faraday-files` is reached through `faraday-storage`
and `faraday-disk`, so the decoder is in the stick and Pi image, the
desktop app and the Starter. The encoder use in `faraday-snapshot` and the
dev-dependencies are tools and tests only.

**Why this crate.** Writing a PNG needs no crate: `faraday_codec::png`
writes QR pictures and greyscale pictures with stored deflate blocks, its
own CRC-32 and Adler-32 (`core/faraday-codec/src/png.rs`). Reading is
different: a picture arrives from a stick or a phone in any colour type,
bit depth, interlacing and compression, and an in-house inflate and
unfilter would be a parser of untrusted input written for one use. `png` is
the maintained pure-Rust decoder (image-rs), and the decode is counted as
costly work through the compute token (`docs/COMPUTE.md`).

**Version and features.** `png = "0.18"`, default features, in the
workspace `Cargo.toml`.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71).

**Reopen when** `faraday-files` reads only pictures `faraday_codec::png`
wrote (the in-house writer then needs a reader and this crate goes from the
device builds), or when the decoder is found to need unsafe code or a
feature the device builds should not have.
