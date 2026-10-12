# Test vectors

- `fixture.img.xz`: a stream made by the xz program (XZ Utils 5.8.3) with
  the options the release uses for its payload, so the decoder is tried on
  what `xz` itself writes: LZMA2 with a 64 MiB dictionary, one block, a
  CRC-64 check. The image inside is 1024 lines of text, 39936 bytes, which
  `tests/payload.rs` makes again to compare byte for byte. Made with:

  ```sh
  awk 'BEGIN{for(i=0;i<1024;i++) printf "%05d %08x faraday payload fixture\n", i, (i*2654435761)%4294967296}' > fixture.img
  xz -9e -T1 fixture.img
  ```

  `sha256sum` gives:

  ```
  c3a69d3c8e5599d5dc08dc6780f76b30a8e4d53082a85d40c6cd2effd28dfb0c  fixture.img
  d7e055e6f9f0d8a3c019d1e6b272b793dd607c3929a2a8f922cee3a5eae71c4f  fixture.img.xz
  ```

  `xz -lvv fixture.img.xz` shows `--lzma2=dict=64MiB` and `CRC64`.

- `zeros.img.xz`: 4 MiB of zeros, made the same way, so an image several
  times the decoder's output buffer is written in pieces:

  ```sh
  head -c 4194304 /dev/zero > zeros.img
  xz -9e -T1 zeros.img
  ```

  `sha256sum` gives:

  ```
  bb9f8df61474d25e71fa00722318cd387396ca1736605e1248821cc0de3d3af8  zeros.img
  236e07f110fe513771bfb64dc3670d1051209787b11ca55f60167cd0df9a64ee  zeros.img.xz
  ```
