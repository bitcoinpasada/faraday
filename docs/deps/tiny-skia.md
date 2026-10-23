# tiny-skia

**Purpose.** The canvas of `faraday-draw` (`core/faraday-draw/src/canvas.rs`):
a `tiny_skia::Pixmap` holds the premultiplied RGBA frame, and the filling and
stroking of paths, gradients, masks and clipping the screens use are
tiny-skia's. Every screen of the device, the desktop app and the Starter
is drawn through it.

**Where it ships.** In every build that draws: the stick and Pi images, the
desktop app and the Starter, and the snapshot tool (`faraday-snapshot`).
`core/faraday-draw` is its only direct dependant.

**Why this crate.** Rasterising anti-aliased paths and text outlines is a
large piece of code of its own. `tiny-skia` is a pure-Rust port of the
subset of Skia that is needed, with no C code, no GPU and no system
library, and it is `no_std` with a float library, so it builds for the
stick, the Pi and the desktop alike with the same output (the snapshot
tests depend on that).

**Version and features.** `tiny-skia = { version = "0.12",
default-features = false, features = ["no-std-float"] }`.

- `no-std-float` takes float functions from `libm`, so the crate does not
  need `std`.
- `std`, `simd` and `png-format` are off: no PNG code and no CPU-specific
  vector code, so the same pixels come out on every target.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71).

**Reopen when** a rasteriser in `core/` replaces it, when the pixel output
must change, or when a release of the crate moves the path or text
rendering the snapshots record.
