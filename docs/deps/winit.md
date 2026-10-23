# winit

**Purpose.** The desktop window: creating the window, the event loop,
keyboard, mouse, touch and wheel input, scale factor and the frame pacing
call `Window::pre_present_notify` (`docs/MOTION.md`). Used by
`faraday/shells/window` (`input.rs`, `view.rs`, `pace.rs`), and directly
by `faraday/shells/desktop` and `faraday/shells/starter`, which run the
event loop.

**Where it ships.** The desktop app and the Starter, on Linux (Wayland and
X11), Windows and macOS. It is not in the stick or Pi images, which have no
window system.

**Why this crate.** Opening a window and receiving input on four platforms
is the work of the platform APIs (Wayland, X11, Win32, AppKit), each with
its own quirks. `winit` is the standard cross-platform layer, and
`softbuffer` (`docs/deps/softbuffer.md`) presents pixels in its windows.
Its platform crates (`windows-sys`, `x11rb`, `wayland-*`, `objc2` and the
Android crates it locks) are the bulk of the desktop dependency tree; that
is why the stack is exempted as one and kept out of every `core/` crate
and the device images (`docs/STRUCTURE-PASS.md` §6).

**Version and features.** `winit = "0.30"`, default features, in the
workspace `Cargo.toml`. It also brings `getrandom` into the graph, which
the desktop shell uses (`docs/deps/getrandom.md`).

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71).

**Reopen when** the 0.31 line is released and the shells move to it, when
a platform needs something `winit` does not give, or when the desktop
shells leave windows for a framebuffer.
