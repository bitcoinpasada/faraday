# opensigner-ffi

The C ABI over `OpenSigner`, and the JNI entry points the Android shell
calls. Builds as `libopensigner.so` (a `cdylib`) for Android and iOS, and
as an `rlib` so the safe half has host unit tests.

Read `src/lib.rs` first: it is the whole contract in safe Rust. `src/jni.rs`
is the Java-facing wrapper around it.

## Why this crate is allowed `unsafe`

Every other crate in the workspace inherits `unsafe_code = "forbid"` from
`[workspace.lints]`. This one does not inherit the workspace lints at all,
because a `forbid` cannot be lifted by the crate that needs it, and this
crate needs it: a JNI entry point is by definition a function the JVM calls
with raw pointers.

That is the only reason. The exemption is kept as small as the language
allows:

- All the `unsafe` lives in `src/jni.rs`. `src/lib.rs` — the handle table,
  the tier, key, touch and file mappings, the command encoding, the
  framebuffer lookup — is ordinary safe Rust with unit tests.
- `unsafe_op_in_unsafe_fn` is `deny`, so an `unsafe fn` gets no free pass:
  every operation that needs it sits in its own `unsafe { }` block with a
  `// SAFETY:` line naming the invariant it stands on.
- What the unsafe code does is four conversions and nothing else: read a
  `byte[]` into a `Vec<u8>`, read a `String`, build a `byte[]` or a
  `String` to return, and wrap the core's framebuffer in a direct
  `java.nio.ByteBuffer`. There is no pointer arithmetic, no transmute, no
  manual allocation and no `Send`/`Sync` assertion.

## Handles, not pointers

`create` boxes an `OpenSigner` and returns a 64-bit handle: a slot index in
the low half and a generation counter in the high half. `destroy` bumps the
generation, so a handle reused after free resolves to nothing rather than
to freed memory, and every entry point answers an unknown handle by doing
nothing. A shell can therefore be wrong about lifetime without being
unsound about it.

The table is thread-local: `OpenSigner` holds an `Rc`, so it is not `Send`,
and the shell drives it from one thread (on Android, the UI thread). A call
from another thread finds no instance. Asserting thread-safety with an
`unsafe impl Send` would have been the alternative, and it would have been
a lie.

## Commands

`poll` returns a small integer for the next command and parks that
command's payload on the instance; `command_kind`, `command_bytes`,
`command_name` and `command_ms` read it back. Splitting it this way keeps
every function's signature to integers plus one array, which is what makes
the JNI side short enough to read: no structs cross the boundary, and no
allocation protocol has to be agreed on.

`command_bytes` *takes* the bytes rather than copying them, so a file the
core asked the shell to write exists in one place afterwards.

## The frame

`frame_ptr` returns a pointer to the core's framebuffer and its length;
the JNI layer hands it to `NewDirectByteBuffer`. Nothing is copied on the
Rust side, and Kotlin's `Bitmap.copyPixelsFromBuffer` is the only copy in
the path from `tiny-skia` to the screen. The core's premultiplied RGBA8888
is byte-for-byte what an `ARGB_8888` bitmap wants.

The memory belongs to the core and is valid until the next call on that
handle. The core in fact allocates its canvas once and never moves it; the
rule is stated tightly anyway so a later core can reallocate without
breaking a shell.

## Panics

Unwinding out of a Rust frame into a Java one is undefined behaviour, so
every entry point runs inside `catch_unwind` and returns a neutral value —
`0` for a handle, `CMD_ERROR` for a poll, null for an object, nothing at
all for the event calls.

The release profile sets `panic = "abort"` (`docs/PLANNING.md` §5.4), which
ends the process before an unwind can start; the guard is what makes debug
builds and host tests behave the same way, and what keeps the property true
if the profile ever changes.

## What never crosses

Pixels, the bytes of files the user chose, and 32 bytes of entropy going
in. No seed, no mnemonic, no key. Nothing here logs, and nothing here looks
at what it is carrying (§5.3). The entropy copy is zeroized as soon as the
core has consumed it.
