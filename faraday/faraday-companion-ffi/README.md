# faraday-companion-ffi

The JNI entry points Faraday Companion's Android shell
(`faraday/shells/android`) calls to drive its core, `faraday-companion`
(`docs/COMPANION.md` §3). A copy of upstream's
`opensigner/opensigner-ffi` at d26b84e: the same handle table, the same
command encoding and the same rules on `unsafe`, for `Companion` instead
of `OpenSigner`. Builds as `libfaraday_companion_ffi.so`, which
`just companion-lib` copies into the app as `libfaraday_companion.so`, and
as an `rlib` so the safe half has host unit tests.

Read `src/lib.rs` first: it is the whole contract in safe Rust.
`src/jni.rs` is the Java-facing wrapper around it.

## Why this crate is allowed `unsafe`

Every other Faraday crate inherits `unsafe_code = "forbid"` from
`[workspace.lints]`. This one does not inherit the workspace lints,
because a `forbid` cannot be lifted by the crate that needs it, and a JNI
entry point is a function the JVM calls with raw pointers. As upstream:

- All the `unsafe` lives in `src/jni.rs`. `src/lib.rs` (the handle
  table, the touch, key and file mappings, the command encoding, the
  framebuffer lookup) is ordinary safe Rust with unit tests.
- `unsafe_op_in_unsafe_fn` is `deny`, so every operation that needs it
  sits in its own `unsafe { }` block with a `// SAFETY:` line naming the
  invariant it stands on.
- What the unsafe code does is four conversions and nothing else: read a
  `byte[]` into a `Vec<u8>`, read a `String`, build a `byte[]` or a
  `String` to return, and wrap the core's framebuffer in a direct
  `java.nio.ByteBuffer`. No pointer arithmetic, no transmute, no manual
  allocation and no `Send`/`Sync` assertion.

## What differs from upstream's

- `create` takes the display only: the Companion has no tier, keeps no
  key, and asks nothing of the secure element or the boot state.
- `poll` hands the shell upstream's commands the Companion gives (draw,
  the camera, the document picker and creator, reading the clipboard),
  then the Companion's own: keep the received files' store, keep the
  screen on, keep the theme. Upstream's commands for entropy, kept
  secrets, the secure element, haptics, settings and writing the
  clipboard are never handed over; one that came would be dropped and
  what it carried wiped.
- `file` carries the picked file's name, so the codes go under it.
- Camera frames go to a `faraday-scanner` worker as well as to the core,
  so the codes are read off the UI thread; what the worker read arrives
  as `Event::Scanned`, and what it saw of a code as the Companion's
  `Seen`, which Receive outlines.
- `page_color` and `page_light` give the shell the theme's page colour
  for the window and the system bars.

## Handles, panics, logging

As upstream: a handle is a slot index and a generation, so a stale handle
names nothing; the table is thread-local, so the core runs on the UI
thread only; every entry point runs inside `catch_unwind` and the release
profile aborts on panic; nothing here logs or looks at what it carries.
