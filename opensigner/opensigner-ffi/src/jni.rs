//! The JNI entry points for the Android shell (`docs/PLANNING.md` §4.7).
//!
//! Every function here is the Java-side name of a call in the crate root,
//! mapped by JNI's own convention: package `app.opensigner`, class `Core`,
//! so `Core.pollCommand` is `Java_app_opensigner_Core_pollCommand`. There
//! are no overloads, so no argument-type suffixes.
//!
//! This module is the only place in any shell crate with `unsafe`. What it
//! does with it is narrow: it copies a `byte[]` into a `Vec<u8>`, copies a
//! `String` into a `String`, builds a `byte[]` or a `String` to hand back,
//! and wraps the core's framebuffer in a direct `java.nio.ByteBuffer`.
//! Nothing else. Each block is a few lines with the invariant it stands on
//! written above it.
//!
//! # Safety
//!
//! The contract is the same for every entry point, so it is stated once
//! here rather than repeated seventeen times. Each function may only be
//! called by the JVM as the native implementation of the matching method
//! on `app.opensigner.Core`. That gives it:
//!
//! - `env`: a live `JNIEnv` for the calling thread, valid for the call.
//! - object arguments: live local references of the declared Java type, or
//!   null. Every function checks for null.
//! - `handle`: any `jlong`. It is *not* trusted — it is looked up in the
//!   crate's handle table, and a handle that names nothing does nothing.
//!
//! Calling any of these from anywhere else is undefined behaviour.
//!
//! Panics never leave: each body runs inside [`crate::guard`], which
//! catches an unwind and returns a neutral value instead. Unwinding into a
//! Java frame would be undefined behaviour, and the release profile's
//! `panic = "abort"` (§5.4) ends the process before that can happen.

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

/// `Core.new(width, height, dpi, insetBottom, insetTop, tier, version,
/// cameraFixed, secure, boot): Long` — creates an app, sends it `Event::Display`, and
/// returns its handle; `0` on failure.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "system" fn Java_app_opensigner_Core_new(
    env: *mut JNIEnv,
    _this: jobject,
    width: jint,
    height: jint,
    dpi: jint,
    inset_bottom: jint,
    inset_top: jint,
    tier: jint,
    version: jstring,
    camera_fixed: jboolean,
    secure: jint,
    boot: jint,
    memory_mib: jint,
) -> jlong {
    crate::guard(0, || {
        // SAFETY: `version` is the JVM's `String?` argument to this method.
        let version = unsafe { read_string(env, version) };
        crate::create(
            width,
            height,
            dpi,
            inset_bottom,
            inset_top,
            tier,
            version.as_deref(),
            camera_fixed != 0,
            secure,
            boot,
            memory_mib,
        )
    })
}

/// `Core.free(handle)` — drops the app, zeroizing what it holds.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_free(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::destroy(handle));
}

/// `Core.touch(handle, x, y, phase)` — a contact at a framebuffer pixel.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_touch(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    x: jint,
    y: jint,
    phase: jint,
) {
    crate::guard((), || crate::touch(handle, x, y, phase));
}

/// `Core.scroll(handle, x, y, dy)` — a scroll in framebuffer pixels.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_scroll(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    x: jint,
    y: jint,
    dy: jint,
) {
    crate::guard((), || crate::scroll(handle, x, y, dy));
}

/// `Core.tick(handle, nowMs)` — monotonic milliseconds.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_tick(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    now_ms: jlong,
) {
    crate::guard((), || crate::tick(handle, now_ms));
}

/// `Core.key(handle, code, ch)` — a key press; the Android shell sends only
/// `KEY_ESCAPE`, for the system back gesture.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_key(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    code: jint,
    ch: jint,
) {
    crate::guard((), || crate::key(handle, code, ch));
}

/// `Core.file(handle, kind, bytes)` — the answer to a file request.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_file(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
    bytes: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `bytes` is the JVM's `ByteArray` argument to this method.
        if let Some(bytes) = unsafe { read_bytes(env, bytes) } {
            crate::file(handle, kind, bytes);
        }
    });
}

/// `Core.fileUnavailable(handle, kind)` — no file is coming.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_fileUnavailable(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::file_unavailable(handle, kind));
}

/// `Core.fileCancelled(handle, kind)` — the picker closed with nothing
/// chosen.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_fileCancelled(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::file_cancelled(handle, kind));
}

/// `Core.clipboard(handle, kind, text)` — what the clipboard holds.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_clipboard(
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

/// `Core.clipboardUnavailable(handle, kind)` — no text is coming.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_clipboardUnavailable(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::clipboard_unavailable(handle, kind));
}

/// `Core.clipboardWritten(handle, kind)` — the text is on the clipboard.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_clipboardWritten(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::clipboard_written(handle, kind));
}

/// `Core.clipboardNotWritten(handle, kind)` — nothing was put there.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_clipboardNotWritten(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::clipboard_not_written(handle, kind));
}

/// `Core.lock(handle)` — the app is no longer in the foreground.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_lock(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::lock(handle));
}

/// `Core.fileWritten(handle, kind)` — the bytes were stored.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_fileWritten(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::file_written(handle, kind));
}

/// `Core.fileNotWritten(handle, kind)` — nothing was stored.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_fileNotWritten(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kind: jint,
) {
    crate::guard((), || crate::file_not_written(handle, kind));
}

/// `Core.settings(handle, bytes)` — the settings the shell kept from an
/// earlier session, handed back once at start.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_settings(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    bytes: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `bytes` is the JVM's `ByteArray` argument to this method.
        if let Some(bytes) = unsafe { read_bytes(env, bytes) } {
            crate::settings(handle, bytes);
        }
    });
}

/// `Core.entropy(handle, bytes)` — 32 bytes from the shell's RNG. The copy
/// this side made is erased as soon as the core has consumed it.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_entropy(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    bytes: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `bytes` is the JVM's `ByteArray` argument to this method.
        if let Some(mut bytes) = unsafe { read_bytes(env, bytes) } {
            crate::entropy(handle, &bytes);
            bytes.zeroize();
        }
    });
}

/// `Core.cameraFrame(handle, width, height, luma, chroma)` — one frame:
/// its luma, and the NV12 chroma plane where the device gave colour.
/// `chroma` may be null, and then the preview is grey.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_cameraFrame(
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
            let chroma = unsafe { read_bytes(env, chroma) };
            crate::camera_frame(handle, width, height, luma, chroma);
        }
    });
}

/// `Core.cameraUnavailable(handle)` — there is no camera.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_cameraUnavailable(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::camera_unavailable(handle));
}

/// `Core.secureMac(handle, mac)` — the tag the secure element computed.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_secureMac(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    mac: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `mac` is the JVM's `ByteArray` argument to this method.
        if let Some(mac) = unsafe { read_bytes(env, mac) } {
            crate::secure_mac(handle, &mac);
        }
    });
}

/// `Core.secureUnavailable(handle)` — no tag will come.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_secureUnavailable(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::secure_unavailable(handle));
}

/// `Core.secretKept(handle, kept)` — whether a blob is on the device.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_secretKept(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    kept: jboolean,
) {
    crate::guard((), || crate::secret_kept(handle, kept != 0));
}

/// `Core.secret(handle, blob)` — the blob the shell was keeping.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_secret(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
    blob: jbyteArray,
) {
    crate::guard((), || {
        // SAFETY: `blob` is the JVM's `ByteArray` argument to this method.
        if let Some(blob) = unsafe { read_bytes(env, blob) } {
            crate::secret(handle, blob);
        }
    });
}

/// `Core.secretUnavailable(handle)` — no blob comes back.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_secretUnavailable(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::secret_unavailable(handle));
}

/// `Core.secretStored(handle)` — the bytes are kept.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_secretStored(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::secret_stored(handle));
}

/// `Core.secretNotStored(handle)` — nothing was kept.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_secretNotStored(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::secret_not_stored(handle));
}

/// `Core.secretForgotten(handle)` — the blob and its keys are gone.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_secretForgotten(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) {
    crate::guard((), || crate::secret_forgotten(handle));
}

/// `Core.pollCommand(handle): Int` — the next command's code.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_pollCommand(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jint {
    crate::guard(crate::CMD_ERROR, || crate::poll(handle))
}

/// `Core.commandKind(handle): Int` — the last command's file kind.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_commandKind(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jint {
    crate::guard(crate::FILE_NONE, || crate::command_kind(handle))
}

/// `Core.commandBytes(handle): ByteArray?` — the last command's bytes, once.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_commandBytes(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jbyteArray {
    crate::guard(ptr::null_mut(), || match crate::command_bytes(handle) {
        // SAFETY: `env` is the JVM's environment pointer for this call.
        Some(bytes) => unsafe { new_bytes(env, &bytes) },
        None => ptr::null_mut(),
    })
}

/// `Core.commandName(handle): String?` — the last command's name hint.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_commandName(
    env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jstring {
    crate::guard(ptr::null_mut(), || match crate::command_name(handle) {
        // SAFETY: `env` is the JVM's environment pointer for this call.
        Some(name) => unsafe { new_string(env, &name) },
        None => ptr::null_mut(),
    })
}

/// `Core.commandText(handle): String?` — the last command's text.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_commandText(
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

/// `Core.commandMs(handle): Int` — the last command's duration.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_commandMs(
    _env: *mut JNIEnv,
    _this: jobject,
    handle: jlong,
) -> jint {
    crate::guard(0, || crate::command_ms(handle))
}

/// `Core.frame(handle): ByteBuffer?` — a direct buffer over the core's
/// framebuffer: premultiplied RGBA8888, `width × height × 4` bytes, no copy
/// on this side. See [`crate::frame_ptr`] for the lifetime rule: the shell
/// reads it before its next call and never keeps it.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_app_opensigner_Core_frame(
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
