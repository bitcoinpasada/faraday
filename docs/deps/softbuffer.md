# softbuffer

**Purpose.** Putting a finished frame on the screen of the desktop app and
the Starter: `faraday/shells/window/src/view.rs` creates a `Context` and a
`Surface` for the `winit` window and copies the RGBA frame that
`faraday-draw` rendered into it. It is the "software blit" of the shell
(`docs/MOTION.md`, `docs/STARTER.md`).

**Where it ships.** The desktop app (`faraday/shells/desktop`) and the
Starter (`faraday/shells/starter`), both through `faraday/shells/window`.
It is not in the stick or Pi images, which write the framebuffer
(`faraday/shells/stick`).

**Why this crate.** Faraday draws every pixel itself in software
(`tiny-skia`), so the window needs a way to present a pixel buffer and no
GPU API (OpenGL, Vulkan, wgpu) would be used or wanted on a signer.
`softbuffer` is the pixel-buffer presenter that works on a `winit` window
across Wayland, X11, Windows and macOS, with no graphics driver.

**Version and features.** `softbuffer = "0.4"`, default features, in the
workspace `Cargo.toml`.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71).

**Reopen when** the window shell moves to a platform layer other than
`winit`, or when presenting through the compositor needs more than a pixel
copy.
