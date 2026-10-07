# `cc` 1 — shell-only, build time only, for the macOS camera

**Where it is used:** `opensigner/shells/avfoundation`
(`opensigner-avfoundation`), as a build dependency under
`[target.'cfg(target_os = "macos")'.build-dependencies]`, and nowhere
else in this repository. The desktop shell depends on that crate under
`[target.'cfg(target_os = "macos")'.dependencies]`. Nothing it produces
runs on a user's machine except the object code it compiles from our own
source: `cc` itself is a build-time tool, absent from every binary, and
absent from a Linux, Windows, Android or Pi build altogether.

**Already in the graph.** `secp256k1-sys` builds libsecp256k1 from C
sources with `cc` on every platform, so every build of this workspace
already runs it (`Cargo.lock`: `cc` 1.4.5). Naming it in
`[workspace.dependencies]` adds no crate to the graph; it makes the
existing one usable by our own build script.

**Why it is needed.** `docs/PLANNING.md` §16.33 decided that no platform
gets a camera library: each shell captures through the operating
system's own interface with code we write and can read. On macOS that
interface is AVFoundation, which is Objective-C. Rust cannot call it
directly, so the capture is `opensigner-avfoundation/src/camera.m` —
about two hundred lines with ARC — behind a three-function C ABI. `cc`
is what compiles that file and links the frameworks.

**What it compiles.** One file, `src/camera.m`, with `-fobjc-arc`, then `AVFoundation`, `CoreMedia`, `CoreVideo` and
`Foundation` as link flags. Nothing is downloaded, nothing is generated,
and the build script does nothing at all on a non-macOS target.

**Why not an Objective-C binding crate.** The owner's rule is that a
library is acceptable only if it is minimal, vendored and small enough to
review in full. `objc2` and its AVFoundation framework crates are tens of
thousands of generated lines and a message-send macro layer; `block2`
adds another. They would replace two hundred lines of Objective-C we can
read with a dependency tree we cannot. The C ABI is four declarations
wide and both sides of it are in this repository.

**Cost.** One build-time crate with a small dependency tree
(`find-msvc-tools`, `shlex`), already built for `secp256k1-sys`. Build
time on macOS grows by one `.m` file. Nothing changes on any other
platform.

**Risk.** `cc` runs the system compiler at build time, which is what it
is for and what `secp256k1-sys` already has it do. It does not affect a
release binary's contents beyond compiling our own source. The exposure
this addition really adds is the `unsafe` in `opensigner-avfoundation`,
which is that crate's business: it is one of the two exceptions to the
workspace's `unsafe_code = "forbid"`, its `Cargo.toml` says so, every
block carries a `// SAFETY:` comment, and its public API is entirely
safe.

**Reopened by:** Windows capture (§16.33 defers it), which would be the
same arrangement with a Media Foundation `.cpp` file and the same crate;
or an Objective-C binding crate small enough to vendor and review, which
would make the `.m` file unnecessary.
