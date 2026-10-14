# lzma-rust2

**Purpose.** The Starter carries the release's images and the desktop
app as the `.xz` files the release publishes, and decompresses one
while it writes it to a stick or a card, or extracts it to run
(`docs/STARTER.md` §1.3, §4.1). `faraday/faraday-payload` is the only
crate that uses it, and only for decoding: the payload is compressed by
the `xz` program in the staging script, so nothing here compresses.

**Why this crate.** It is pure Rust, and with the default features off
it is safe Rust too: its one source of `unsafe` is the `optimization`
feature, and without it the crate builds under its own
`#![forbid(unsafe_code)]`. STARTER §1.3 rules out liblzma and any other C
decompressor (xz-utils 5.6.0 and 5.6.1 carried a backdoor in 2024), and
this is the pure-Rust decoder that meets the rest of what the Starter
needs, read in its source:

- It decodes what `xz -9e -T1` writes: an LZMA2 block with a 64 MiB
  dictionary and a CRC-64 check. The test vector in
  `faraday/faraday-payload/tests/vectors/` was made by the xz program
  with those options.
- It streams. `XzStream` is a sans-I/O decoder (an input slice in, an
  output slice out, as liblzma's `lzma_code`), so `faraday-payload`
  reads 64 KiB at a time and hands the writer at most 1 MiB at a time.
  The dictionary grows with the decoded data up to the size the stream
  declares (0.22 stopped it allocating the declared size up front), and
  a stream that declares more than a limit is refused before anything
  is allocated (`new_mem_limit`; `faraday-payload` sets 128 MiB).
- It checks the format's own integrity checks: the stream header's and
  footer's CRC-32, each block header's CRC-32, each block's check
  (CRC-32, CRC-64 or SHA-256), the index's CRC-32 and each index record
  against the block it describes, zero padding, and footer flags
  against header flags. A truncated stream is an error at the end of
  input, not a short success. It does not compare the footer's
  backward size with the index it read; `faraday-payload` does not rely
  on it, since a payload's own SHA-256 and size are checked against the
  manifest.
- It is maintained: a fork of `lzma-rust`, the Rust port of Tukaani's
  XZ for Java, which stopped in 2024, kept by one maintainer with
  several other contributors. Releases come every few weeks with a
  changelog that lists fixes to the decoder's handling of hostile
  input (fuzzing findings, allocation bounds); its tests compare
  against liblzma. `zip` and `sevenz-rust2` use it, among 87
  dependents.

**Cost.** Version 0.22 with `default-features = false, features =
["std", "xz"]`: one crate, `lzma-rust2`. `xz` brings `sha2` 0.11 for
the SHA-256 check type, which is the `sha2` already in the graph, so
nothing else enters it. The features left off are the encoder, lzip and
`optimization`; `std` gives `std::io::Error`s with the decoder's own
words in them, and its multithreaded readers are compiled and never
called. Apache-2.0, which `deny.toml` allows. The decoding path is
about 4,700 lines (`xz.rs`, `xz/reader.rs`, `lzma2_reader.rs`,
`lz/lz_decoder.rs`, `range_dec.rs`, `crc.rs`); the crate is 30,000 lines
with its encoders and tests. `supply-chain/config.toml` carries it as an
exemption: the decoding path was read for this choice, the rest of the
crate was not.

**Not chosen.**

- `lzma-rs` 0.3: pure Rust and `forbid(unsafe_code)`, but its xz
  decoder decodes each block into a `Vec` and checks it before writing
  any of it, and `xz -T1` writes one block for the whole file, so an
  image would be held in memory whole. Last release January 2023, last
  commit May 2024. Brings `crc` and `byteorder`.
- `xz4rust` 0.2: a port of XZ Embedded, streaming with a bounded
  buffer, but `unsafe` unless its `no_unsafe` feature is on, one
  author, about 60,000 downloads, and its SHA-256 check pulls `sha2`
  0.10, which would bring a second copy of the RustCrypto hashing stack
  (`docs/deps/argon2.md` says which five crates).
- `liblzma`, `xz2`, `lzma-sys`: bindings to the C library STARTER §1.3 rules
  out.
- `lzma-rust`: the unmaintained original this crate forked.
- `omnizip-lzma`, `oxiarc-lzma`: first published in 2026, each with
  under 20,000 downloads; not read further.

**Reopen when** a release moves the minor version (0.x releases have
come every few weeks: read the changelog's decoder entries and the
diff of the decoding path before taking one, and move the
`supply-chain` exemption with it), or when the payload stops being
`.xz`.
