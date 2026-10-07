# jni-sys

**Purpose.** The type definitions of `jni.h` for `opensigner-ffi`
(`docs/PLANNING.md` §4.7): `JNIEnv`, `jobject`, `jint`, `jlong`,
`jbyteArray`, and the `JNINativeInterface_` table whose function pointers
the JNI layer calls to copy a `byte[]`, copy a `String`, and wrap the
framebuffer in a direct `ByteBuffer`.

**Types, not code.** The crate is `#![no_std]` and contains no functions:
every item is a type alias, a constant or a `#[repr(C)]` struct of
function pointers. Nothing it ships runs. What it buys is the one thing
that is genuinely hard to write by hand and impossible to test on the
host: the exact ordinal position of each function in the JNI table. Get a
field's position wrong and the call goes to a different JVM function, with
no compiler error and no crash until it corrupts something.

**Not the `jni` crate.** §4.7 rules out a binding generator, and the `jni`
crate is one in spirit: it wraps the table in owned `JNIEnv`/`JObject`
types with lifetimes, exception handling and a `cesu8` decoder, and brings
`log`, `thiserror`, `combine` and a proc-macro with it. The shell needs
none of that. The whole JNI layer is under 400 lines including its safety
notes, and it is all readable in one sitting — which is the property that
matters for a signing device.

**Version.** `=0.3.0`, exactly, which has the flat `Option<fn>` table this
code reads and no dependencies at all. 0.4 moved the same functions into
proc-macro-generated version unions, and 0.3.1 is a compatibility shim
that takes its `_jobject` and a few other aliases from 0.4, so depending
on `0.3` rather than `=0.3.0` puts `jni-sys-macros`, `syn`, `quote` and
`proc-macro2` in this crate's graph — which §10.1 rules out for the core
and the FFI. The exact pin is the whole reason the graph stays clean, so
it stays until the table is declared here by hand.

`ndk-sys` asks for `0.3.0` too, so the pin does not fork the workspace;
the `jni` crate and 0.4 remain in `Cargo.lock` through
`android-activity`, which is winit's Android backend and is in no device
graph.

**Cost.** One crate, no runtime code, no allocation, no `std`.

**Reopen when** the FFI grows enough JNI surface that the table access
becomes error-prone, or if 0.3 stops being maintained; the fallback is to
declare the handful of fields we use in a `#[repr(C)]` struct here, with
the unused ones as opaque pointers to hold their positions.
