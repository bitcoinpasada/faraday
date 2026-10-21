# `cc` 1 — shell-only, build time only, for the macOS and Windows cameras

**Where it is used:**

- `faraday/shells/avfoundation` (`faraday-avfoundation`), as a
  build dependency under
  `[target.'cfg(target_os = "macos")'.build-dependencies]`. The desktop
  shell depends on that crate under
  `[target.'cfg(target_os = "macos")'.dependencies]`.
- `faraday/shells/mediafoundation` (`faraday-mediafoundation`), as a
  plain build dependency: its build script runs on the build host (Linux,
  for the Windows build too, which cross-compiles) and compiles
  `src/camera.c` only when `CARGO_CFG_TARGET_OS` is `windows`, read at
  run time; on any other target it compiles nothing. `faraday-window`
  depends on that crate under
  `[target.'cfg(target_os = "windows")'.dependencies]`, so the Starter
  and the desktop app carry it on Windows only.

Nowhere else in this repository. Nothing it produces runs on a user's
machine except the object code it compiles from our own source: `cc`
itself is a build-time tool, absent from every binary, and compiles
nothing of ours for a Linux, Android or Pi build.

**Already in the graph.** `secp256k1-sys` builds libsecp256k1 from C
sources with `cc` on every platform, so every build of this workspace
already runs it (`Cargo.lock`: `cc` 1.4.5). Naming it in
`[workspace.dependencies]` adds no crate to the graph; it makes the
existing one usable by our own build script.

**Why it is needed.** no platform
gets a camera library: each shell captures through the operating
system's own interface with code we write and can read. On macOS that
interface is AVFoundation, which is Objective-C. Rust cannot call it
directly, so the capture is `faraday-avfoundation/src/camera.m` —
about two hundred lines with ARC — behind a three-function C ABI. `cc`
is what compiles that file and links the frameworks.

**What it compiles.** On macOS, one file, `src/camera.m`, with
`-fobjc-arc`, then `AVFoundation`, `CoreMedia`, `CoreVideo` and
`Foundation` as link flags. On Windows, one file, `src/camera.c`, with
mingw-w64's compiler in the Windows build container, then `mfuuid` (a
static library of GUIDs) and `ole32` as link flags; Media Foundation's
own DLLs are loaded at run time from the system folder, so Windows' N
editions, which lack them, still start the program. Nothing is
downloaded, nothing is generated, and each build script does nothing on
any other target.

**Why not an Objective-C binding crate.** The owner's rule is that a
library is acceptable only if it is minimal, vendored and small enough to
review in full. `objc2` and its AVFoundation framework crates are tens of
thousands of generated lines and a message-send macro layer; `block2`
adds another. They would replace two hundred lines of Objective-C we can
read with a dependency tree we cannot. The C ABI is four declarations
wide and both sides of it are in this repository.

**Cost.** One build-time crate with a small dependency tree
(`find-msvc-tools`, `shlex`), already built for `secp256k1-sys`. Build
time on macOS grows by one `.m` file and on Windows by one `.c` file.
Nothing changes on any other platform.

**Risk.** `cc` runs the system compiler at build time, which is what it
is for and what `secp256k1-sys` already has it do. It does not affect a
release binary's contents beyond compiling our own source. The exposure
this addition really adds is the `unsafe` in `faraday-avfoundation`
and `faraday-mediafoundation`, which is those crates' business: each is
an exception to the workspace's `unsafe_code = "forbid"`, its
`Cargo.toml` says so, every block carries a `// SAFETY:` comment, and
its public API is entirely safe.

**Reopened by:** an Objective-C or COM binding crate small enough to
vendor and review, which would make the `.m` or the `.c` file
unnecessary. (Windows capture, which §16.33 deferred, has since been
built the same way: `faraday-mediafoundation`, in C rather than C++.)
