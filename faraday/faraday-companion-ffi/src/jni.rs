//! The JNI entry points for Faraday Companion's Android shell
//! (`docs/COMPANION.md` §3), copied from upstream's
//! `opensigner/opensigner-ffi/src/jni.rs` at d26b84e: its four
//! conversions verbatim, its entry points for the Companion's calls.
//!
//! Every function here is the Java-side name of a call in the crate root,
//! mapped by JNI's own convention: package `app.faradayoffline.companion`,
//! class `Core`, so `Core.pollCommand` is
//! `Java_app_faradayoffline_companion_Core_pollCommand`. There are no
//! overloads, so no argument-type suffixes.
//!
//! This module is the only place in the crate with `unsafe`. What it does
//! with it is narrow: it copies a `byte[]` into a `Vec<u8>`, copies a
//! `String` into a `String`, builds a `byte[]` or a `String` to hand back,
//! and wraps the core's framebuffer in a direct `java.nio.ByteBuffer`.
//! Nothing else. Each block is a few lines with the invariant it stands on
//! written above it.
//!
//! # Safety
//!
//! The contract is the same for every entry point, so it is stated once
//! here. Each function may only be called by the JVM as the native
//! implementation of the matching method on
//! `app.faradayoffline.companion.Core`. That gives it:
//!
//! - `env`: a live `JNIEnv` for the calling thread, valid for the call.
//! - object arguments: live local references of the declared Java type, or
//!   null. Every function checks for null.
//! - `handle`: any `jlong`. It is *not* trusted: it is looked up in the
//!   crate's handle table, and a handle that names nothing does nothing.
//!
//! Calling any of these from anywhere else is undefined behaviour.
//!
//! Panics never leave: each body runs inside [`crate::guard`], which
//! catches an unwind and returns a neutral value instead.

// JNI's exported symbols are the Java method names, which are camelCase.
#![allow(non_snake_case)]
// The safety contract is identical for all of them and is stated in the
// module documentation above.
#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, CString, c_void};
use std::ptr;

use jni_sys::{
    JNIEnv, JNINativeInterface_, jboolean, jbyteArray, jint, jlong, jobject, jsize, jstring,
};
use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// Conversions. Four kinds of value cross the boundary and this is all of it.
// ---------------------------------------------------------------------------

/// The JVM's function table for this thread, or `None` if `env` is not one.
unsafe fn functions(env: *mut JNIEnv) -> Option<JNINativeInterface_> {
    if env.is_null() {
        return None;
    }
    // SAFETY: `env` is the JVM's environment pointer for this call, so it
    // points at a pointer to the function table, which the JVM keeps valid
    // for at least the duration of the call. The table is `Copy`.
    let table = unsafe { *env };
    if table.is_null() {
        return None;
    }
    // SAFETY: as above, `table` is the JVM's own function table.
    Some(unsafe { *table })
}

/// Copies a Java `byte[]` into a `Vec<u8>`. `None` for a null array.
unsafe fn read_bytes(env: *mut JNIEnv, array: jbyteArray) -> Option<Vec<u8>> {
    // SAFETY: `env` is the caller's environment pointer, passed through.
    let jni = unsafe { functions(env) }?;
    let (length, region) = (jni.GetArrayLength?, jni.GetByteArrayRegion?);
    if array.is_null() {
        return None;
    }
    // SAFETY: `array` is a live reference to a `byte[]` from the JVM.
    let len = unsafe { length(env, array) };
    let mut bytes = vec![0u8; usize::try_from(len).ok()?];
    if len > 0 {
        // SAFETY: the region asked for is exactly the array's own length,
        // `bytes` has that many elements, and `jbyte` (i8) and `u8` have
        // the same size and alignment.
        unsafe { region(env, array, 0, len, bytes.as_mut_ptr().cast::<i8>()) };
    }
    Some(bytes)
}

/// Builds a Java `byte[]` holding `bytes`. Null if the JVM cannot allocate.
unsafe fn new_bytes(env: *mut JNIEnv, bytes: &[u8]) -> jbyteArray {
    // SAFETY: `env` is the caller's environment pointer, passed through.
    let jni = unsafe { functions(env) };
    let (Some(jni), Ok(len)) = (jni, jsize::try_from(bytes.len())) else {
        return ptr::null_mut();
    };
    let (Some(new), Some(fill)) = (jni.NewByteArray, jni.SetByteArrayRegion) else {
        return ptr::null_mut();
    };
    // SAFETY: `env` is the JVM's environment pointer. A failed allocation
    // returns null with an exception pending, which the next check catches.
    let array = unsafe { new(env, len) };
    if array.is_null() {
        return array;
    }
    if len > 0 {
        // SAFETY: `array` was just created with exactly `len` elements, and
        // `bytes` has `len` of them.
        unsafe { fill(env, array, 0, len, bytes.as_ptr().cast::<i8>()) };
    }
    array
}

/// Copies a Java `String` into a Rust one. `None` for a null string.
unsafe fn read_string(env: *mut JNIEnv, string: jstring) -> Option<String> {
    // SAFETY: `env` is the caller's environment pointer, passed through.
    let jni = unsafe { functions(env) }?;
    let (get, release) = (jni.GetStringUTFChars?, jni.ReleaseStringUTFChars?);
    if string.is_null() {
        return None;
    }
    // SAFETY: `string` is a live reference to a `java.lang.String`. A null
    // `isCopy` is allowed and means "do not tell me".
    let chars = unsafe { get(env, string, ptr::null_mut()) };
    if chars.is_null() {
        return None;
    }
    // SAFETY: the JVM returns a NUL-terminated buffer that stays valid
    // until the matching release below.
    let owned = unsafe { CStr::from_ptr(chars) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: `chars` is exactly what `GetStringUTFChars` returned for
    // `string`, released once.
    unsafe { release(env, string, chars) };
    Some(owned)
}

/// Builds a Java `String`. Null for a string with an interior NUL, which
/// none of the core's name hints has, or if the JVM cannot allocate.
unsafe fn new_string(env: *mut JNIEnv, value: &str) -> jstring {
    // SAFETY: `env` is the caller's environment pointer, passed through.
    let jni = unsafe { functions(env) };
    let (Some(jni), Ok(text)) = (jni, CString::new(value)) else {
        return ptr::null_mut();
    };
    let Some(new) = jni.NewStringUTF else {
        return ptr::null_mut();
    };
    // SAFETY: `text` is NUL-terminated and outlives the call, which copies
    // it into a Java string.
    unsafe { new(env, text.as_ptr()) }
}

// ---------------------------------------------------------------------------
// Entry points.
// ---------------------------------------------------------------------------

/// `Core.new(width, height, dpi, insetBottom, insetTop): Long`: creates a
/// Companion, sends it `Event::Display`, and returns its handle; `0` on
/// failure.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_new(
    _env: *mut JNIEnv,
    _this: jobject,
    width: jint,
    height: jint,
    dpi: jint,
    inset_bottom: jint,
    inset_top: jint,
) -> jlong {
    crate::guard(0, || {
        crate::create(width, height, dpi, inset_bottom, inset_top)
    })
}

/// `Core.free(handle)`: drops the Companion, zeroizing what it holds.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_free(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::destroy(handle));
}

/// `Core.touch(handle, x, y, phase)`: a contact at a framebuffer pixel.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_touch(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    x: jint,
    y: jint,
    phase: jint,
) {
    crate::guard((), || crate::touch(handle, x, y, phase));
}

/// `Core.scroll(handle, x, y, dy)`: a scroll in framebuffer pixels.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_scroll(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    x: jint,
    y: jint,
    dy: jint,
) {
    crate::guard((), || crate::scroll(handle, x, y, dy));
}

/// `Core.tick(handle, nowMs)`: monotonic milliseconds.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_tick(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    now_ms: jlong,
) {
    crate::guard((), || crate::tick(handle, now_ms));
}

/// `Core.key(handle, code)`: the back gesture.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_key(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    code: jint,
) {
    crate::guard((), || crate::key(handle, code));
}

/// `Core.file(handle, kind, name, bytes)`: the answer to a file request,
/// with the name the document provider gave, or null.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_file(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
    name: jstring,
    bytes: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `name` and `bytes` are the JVM's `String?` and
        // `ByteArray` arguments to this method.
        let name = unsafe { read_string(env, name) };
        // SAFETY: as above, for `bytes`.
        if let Some(bytes) = unsafe { read_bytes(env, bytes) } {
            crate::file(handle, kind, name, bytes);
        }
    });
}

/// `Core.fileUnavailable(handle, kind)`: no file is coming.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_fileUnavailable(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::file_unavailable(handle, kind));
}

/// `Core.fileCancelled(handle, kind)`: the picker closed with nothing
/// chosen.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_fileCancelled(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::file_cancelled(handle, kind));
}

/// `Core.fileWritten(handle, kind)`: the bytes were stored.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_fileWritten(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::file_written(handle, kind));
}

/// `Core.fileNotWritten(handle, kind)`: nothing was stored.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_fileNotWritten(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::file_not_written(handle, kind));
}

/// `Core.clipboardUnavailable(handle, kind)`: no text is coming.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_clipboardUnavailable(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::clipboard_unavailable(handle, kind));
}

/// `Core.clipboard(handle, kind, text)`: what the clipboard holds.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_clipboard(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
    text: jstring,
) {
    crate::guard((), || {
        // SAFETY: `text` is the JVM's `String` argument to this method.
        if let Some(text) = unsafe { read_string(env, text) } {
            crate::clipboard(handle, kind, text);
        }
    });
}

/// `Core.cameraFrame(handle, width, height, luma, chroma)`: one frame:
/// its luma, and the NV12 chroma plane where the device gave colour.
/// `chroma` may be null, and then the preview is grey.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_cameraFrame(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    width: jint,
    height: jint,
    luma: jbyteArray,
    chroma: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `luma` and `chroma` are the JVM's `ByteArray`
        // arguments to this method; `read_bytes` answers `None` for the
        // null the Kotlin side passes when there is no colour.
        if let Some(luma) = unsafe { read_bytes(env, luma) } {
            // SAFETY: as above, for `chroma`.
            let chroma = unsafe { read_bytes(env, chroma) };
            crate::camera_frame(handle, width, height, luma, chroma);
        }
    });
}

/// `Core.cameraUnavailable(handle)`: there is no camera.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_cameraUnavailable(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::camera_unavailable(handle));
}

/// `Core.time(handle, unixS, offsetS)`: the wall-clock time.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_time(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    unix_s: jlong,
    offset_s: jint,
) {
    crate::guard((), || crate::time(handle, unix_s, offset_s));
}

/// `Core.received(handle, bytes)`: the received files' store the shell
/// kept.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_received(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    bytes: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `bytes` is the JVM's `ByteArray` argument to this method.
        if let Some(bytes) = unsafe { read_bytes(env, bytes) } {
            crate::received(handle, bytes);
        }
    });
}

/// `Core.theme(handle, id)`: the theme the shell kept.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_theme(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    id: jstring,
) {
    crate::guard((), || {
        // SAFETY: `id` is the JVM's `String` argument to this method.
        if let Some(id) = unsafe { read_string(env, id) } {
            crate::theme(handle, id);
        }
    });
}

/// `Core.start(handle, text)`: the answers the shell kept for Start from
/// the stick.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_start(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    text: jstring,
) {
    crate::guard((), || {
        // SAFETY: `text` is the JVM's `String` argument to this method.
        if let Some(text) = unsafe { read_string(env, text) } {
            crate::start(handle, text);
        }
    });
}

/// `Core.steps(handle, text)`: one guide page's ticked steps the shell
/// kept.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_steps(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    text: jstring,
) {
    crate::guard((), || {
        // SAFETY: `text` is the JVM's `String` argument to this method.
        if let Some(text) = unsafe { read_string(env, text) } {
            crate::steps(handle, text);
        }
    });
}

/// `Core.system(handle, build, certificate, permissions)`: About's facts
/// from the package manager.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_system(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    build: jstring,
    certificate: jbyteArray,
    permissions: jstring,
) {
    crate::guard((), || {
        // SAFETY: `build` and `permissions` are the JVM's `String`
        // arguments to this method, and `certificate` its `ByteArray`.
        let build = unsafe { read_string(env, build) };
        // SAFETY: as above, for `certificate`.
        let certificate = unsafe { read_bytes(env, certificate) };
        // SAFETY: as above, for `permissions`.
        let permissions = unsafe { read_string(env, permissions) };
        if let (Some(build), Some(certificate)) = (build, certificate) {
            crate::system(
                handle,
                build,
                &certificate,
                permissions.as_deref().unwrap_or(""),
            );
        }
    });
}

/// `Core.kept(handle, bytes)`: the kept store, opened after the unlock.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_kept(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    bytes: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `bytes` is the JVM's `ByteArray` argument to this method.
        if let Some(bytes) = unsafe { read_bytes(env, bytes) } {
            crate::kept(handle, bytes);
        }
    });
}

/// `Core.keptClosed(handle, why)`: the kept store was not opened.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_keptClosed(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    why: jint,
) {
    crate::guard((), || crate::kept_closed(handle, why));
}

/// `Core.locked(handle)`: the app went to the background.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_locked(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::locked(handle));
}

/// `Core.keptCounts(handle, version, descriptions)`: the version and the
/// count kept beside the store.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_keptCounts(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    version: jint,
    descriptions: jint,
) {
    crate::guard((), || crate::kept_counts(handle, version, descriptions));
}

/// `Core.sharedIn(handle, name, bytes)`: one file or text shared to the
/// app; `name` is null for text.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_sharedIn(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    name: jstring,
    bytes: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `name` and `bytes` are the JVM's `String?` and
        // `ByteArray` arguments to this method.
        let name = unsafe { read_string(env, name) };
        // SAFETY: as above, for `bytes`.
        if let Some(bytes) = unsafe { read_bytes(env, bytes) } {
            crate::shared_in(handle, name, bytes);
        }
    });
}

/// `Core.pollCommand(handle): Int`: the next command's code.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_pollCommand(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jint {
    crate::guard(crate::CMD_ERROR, || crate::poll(handle))
}

/// `Core.commandKind(handle): Int`: the last command's file kind.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_commandKind(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jint {
    crate::guard(crate::FILE_NONE, || crate::command_kind(handle))
}

/// `Core.commandBytes(handle): ByteArray?`: the last command's bytes,
/// once. The copy this side held is wiped once the Java array is made.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_commandBytes(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jbyteArray {
    crate::guard(ptr::null_mut(), || match crate::command_bytes(handle) {
        Some(mut bytes) => {
            // SAFETY: `env` is the JVM's environment pointer for this call.
            let array = unsafe { new_bytes(env, &bytes) };
            bytes.zeroize();
            array
        }
        None => ptr::null_mut(),
    })
}

/// `Core.commandName(handle): String?`: the last command's name hint.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_commandName(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jstring {
    crate::guard(ptr::null_mut(), || match crate::command_name(handle) {
        // SAFETY: `env` is the JVM's environment pointer for this call.
        Some(text) => unsafe { new_string(env, &text) },
        None => ptr::null_mut(),
    })
}

/// `Core.commandText(handle): String?`: the last command's text.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_commandText(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jstring {
    crate::guard(ptr::null_mut(), || match crate::command_text(handle) {
        // SAFETY: `env` is the JVM's environment pointer for this call.
        Some(text) => unsafe { new_string(env, &text) },
        None => ptr::null_mut(),
    })
}

/// `Core.commandOn(handle): Boolean`: whether the last command says on.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_commandOn(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jboolean {
    jboolean::from(crate::guard(false, || crate::command_on(handle)))
}

/// `Core.commandDescriptions(handle): Int`: the kept store's count of
/// wallet descriptions.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_commandDescriptions(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jint {
    crate::guard(0, || crate::command_descriptions(handle))
}

/// `Core.pageColor(handle): Int`: the page's colour, opaque ARGB.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_pageColor(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jint {
    crate::guard(0, || crate::page_color(handle))
}

/// `Core.pageLight(handle): Boolean`: dark text on a light page.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_pageLight(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jboolean {
    jboolean::from(crate::guard(false, || crate::page_light(handle)))
}

/// `Core.frame(handle): ByteBuffer?` — a direct buffer over the core's
/// framebuffer: premultiplied RGBA8888, `width × height × 4` bytes, no copy
/// on this side. See [`crate::frame_ptr`] for the lifetime rule: the shell
/// reads it before its next call and never keeps it.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_faradayoffline_companion_Core_frame(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jobject {
    crate::guard(ptr::null_mut(), || {
        // SAFETY: `env` is the JVM's environment pointer for this call.
        let jni = unsafe { functions(env) };
        let (Some(jni), Some((pixels, len))) = (jni, crate::frame_ptr(handle)) else {
            return ptr::null_mut();
        };
        let (Some(wrap), Ok(capacity)) = (jni.NewDirectByteBuffer, jlong::try_from(len)) else {
            return ptr::null_mut();
        };
        // SAFETY: `pixels` points at `len` initialised bytes owned by the
        // core, which stay put until this instance is next called (the rule
        // on `frame_ptr`). The JVM only reads them, and does not free them:
        // a direct buffer made this way has no deallocator attached.
        unsafe { wrap(env, pixels.cast_mut().cast::<c_void>(), capacity) }
    })
}
