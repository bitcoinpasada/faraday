# `libc` 0.2 — shell-only, for the V4L2 camera

**Where it is used:** `opensigner/shells/v4l2` (`opensigner-v4l2`) and
nowhere else. The Linux desktop shell depends on that crate under
`[target.'cfg(target_os = "linux")'.dependencies]`; the Pi shell depends
on it directly. No core crate depends on `libc`, and no release binary
of a core crate carries it.

**Why it is needed.** `docs/PLANNING.md` §16.33 decided that no platform
gets a camera library: each shell captures through the operating
system's own interface with code we write and can read. On Linux that
interface is V4L2, which is `ioctl`, `mmap` and `poll` on a
`/dev/video*` node. The standard library offers none of the three, and
`std::os::fd` gives only the file descriptor. `libc` supplies the four
function declarations (`ioctl`, `mmap`, `munmap`, `poll`), the `pollfd`
struct, and the handful of constants (`PROT_READ`, `MAP_SHARED`,
`POLLIN`, `EAGAIN`, `EINTR`).

**What it is not used for.** The V4L2 ABI itself — the `#[repr(C)]`
structs, the FourCC codes and the `ioctl` request numbers — is declared
in `opensigner-v4l2`'s own `abi.rs`, not taken from `libc`, so that the
32-bit and 64-bit layouts and the request numbers derived from them are
visible and tested in this repository.

**Why not a camera crate.** The owner's rule is that a library is
acceptable only if it is minimal, vendored and small enough to review in
full. `v4l` and its relatives are not: they bring a `v4l2-sys` bindgen
layer, a device-enumeration model and a format abstraction, none of
which this app needs. The whole capture path here is about four hundred
lines including the ABI declarations and its tests.

**Cost.** One crate, no transitive dependencies, `default-features =
false` (no `std` feature). It was already in the build graph through
`getrandom` and `winit` on the desktop side; this is the first time the
workspace names it directly.

**Risk.** `libc` is the most-used crate in the ecosystem and is
maintained by the Rust project. The exposure it adds is the `unsafe`
this repository writes against it, not the crate itself: every `unsafe`
block lives in `opensigner-v4l2`, that crate is the single exception to
the workspace's `unsafe_code = "forbid"`, its `Cargo.toml` says so, and
every block carries a `// SAFETY:` comment. Its public API is entirely
safe — no raw pointer, file descriptor or mapped buffer crosses it.

**Reopened by:** a Linux target with no V4L2 device (a Wayland-only
desktop behind a portal-only camera stack), which would need a different
interface and a fresh decision under §16.33.
