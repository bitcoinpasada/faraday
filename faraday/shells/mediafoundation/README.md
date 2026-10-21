# faraday-mediafoundation

Camera capture on Windows for the desktop app and the Starter: the video
capture devices from Media Foundation, one opened as a source reader
giving NV12 near 1280 x 720, and its Y plane handed to the core as 8-bit
luma with the UV plane as the preview's colour. The Windows twin of
`opensigner-avfoundation`.

- `src/camera.c` — Media Foundation through COM in C, the only C of
  Faraday's own. It exports two functions; its header comment is the
  ABI, and `src/lib.rs` mirrors it. Media Foundation's DLLs are loaded
  from the system folder when the camera is used, not imported, so the
  program still starts on Windows' N editions, which lack them.
- `src/lib.rs` — the `extern "C"` declarations, `Camera`, `devices`,
  `Device`, `Frame`, `Message`. Everything Media Foundation is behind
  `#[cfg(target_os = "windows")]`, so the crate builds to its
  documentation and its frame conversion on Linux and macOS.
- `src/convert.rs` — the stride-honouring copy and the integer
  downscale, as `opensigner-avfoundation`'s. No `unsafe`, so it is
  tested on every platform.
- `build.rs` — `cc` compiles the `.c` file and links `mfuuid` and
  `ole32`, when the target is Windows only (read at run time: the
  Windows build cross-compiles from Linux).

This crate is one of the exceptions to the workspace's
`unsafe_code = "forbid"`, for the reason in `docs/PLANNING.md` §16.33:
reaching the operating system's camera has no safe form in `std`, and a
binding crate would be more code than this and less readable. Every
`unsafe` block carries a `// SAFETY:` comment and the public API is
entirely safe.

Windows' camera privacy setting ("Let desktop apps access your camera")
decides whether the program gets the camera; a refusal arrives as
`Message::Unavailable`, as no camera does.
