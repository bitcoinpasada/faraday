# opensigner-avfoundation

Camera capture on macOS for the desktop shell: an `AVCaptureSession` on
the default video device, a bi-planar 4:2:0 output, and the Y plane
handed to the core as 8-bit luma. The macOS twin of `opensigner-v4l2`.

- `src/camera.m` — about two hundred lines of Objective-C with ARC, the
  only Objective-C in the repository. It exports three C functions;
  its header comment is the ABI, and `src/lib.rs` mirrors it.
- `src/lib.rs` — the `extern "C"` declarations, `Camera`, `Frame`,
  `Message`. Everything AVFoundation is behind
  `#[cfg(target_os = "macos")]`, so the crate builds to its
  documentation and its frame conversion on Linux and Windows.
- `src/convert.rs` — the stride-honouring copy and the integer
  downscale. No `unsafe`, so it is tested on every platform.
- `build.rs` — `cc` compiles the `.m` file and links AVFoundation,
  CoreMedia, CoreVideo and Foundation, on macOS only.

This crate and `opensigner-v4l2` are the two exceptions to the
workspace's `unsafe_code = "forbid"`, for the reason in
`docs/PLANNING.md` §16.33: reaching the operating system's camera has no
safe form in `std`, and a binding crate would be more code than this and
less readable. Every `unsafe` block carries a `// SAFETY:` comment and
the public API is entirely safe.

The camera permission needs an application bundle with an
`NSCameraUsageDescription`: `just mac-app` builds and ad-hoc signs one.
A bare binary run from Terminal asks for Terminal's permission instead,
which may or may not have been granted.
