//! The C ABI over [`OpenSigner`], and the thin JNI layer the Android shell
//! calls (`docs/PLANNING.md` §4.7, §16.28).
//!
//! The shell contract ([`osk_shell_api`]) is three calls — deliver an event,
//! drain the commands, read the frame — so the boundary is a handful of
//! plain integer functions and no generated marshalling code. This crate is
//! split in two:
//!
//! - This module is **safe Rust**: the handle table, the tier and key
//!   mappings, and the encoding of a [`Command`] into the integers and byte
//!   strings the shell reads back. It is unit-tested on the host.
//! - [`jni`] is the only place with `unsafe`: it converts JNI arguments to
//!   Rust values, calls in here, and converts back. Every block is a few
//!   lines with the invariant it relies on written above it.
//!
//! ## Handles
//!
//! A shell never holds a Rust pointer. [`create`] returns a 64-bit handle
//! that indexes a table of boxed [`OpenSigner`]s, with a generation counter
//! in the high half: a handle used after [`destroy`], or a handle that was
//! never issued, resolves to nothing instead of to freed memory. Every
//! entry point tolerates a bad handle by doing nothing.
//!
//! The table is thread-local, because [`OpenSigner`] is not `Send`: the
//! core is single-threaded by construction. A shell therefore creates,
//! drives and frees one app on one thread — on Android, the UI thread. A
//! call from any other thread finds no instance and does nothing.
//!
//! ## Commands
//!
//! [`poll`] returns one command code and remembers that command's payload
//! on the instance, which the shell then reads with [`command_kind`],
//! [`command_bytes`], [`command_name`] and [`command_ms`]. Encoding the
//! payload this way keeps every function's signature to integers and one
//! array, so the JNI layer needs no structs and no allocation protocol.
//!
//! ## The frame
//!
//! [`frame_ptr`] hands out a pointer to the core-owned framebuffer, which
//! the JNI layer wraps in a direct `java.nio.ByteBuffer`. Nothing is
//! copied on the Rust side; the shell copies once, into its bitmap. See
//! [`frame_ptr`] for the lifetime rule.
//!
//! ## Panics
//!
//! No panic may cross into Java: unwinding through a foreign frame is
//! undefined behaviour. Every entry point wraps its work in
//! [`std::panic::catch_unwind`] and returns an error value instead. The
//! release profile sets `panic = "abort"` (§5.4), so a panic there ends
//! the process before it can unwind at all; the guard is what makes debug
//! builds and host tests behave the same way.
//!
//! ## No logging
//!
//! Nothing here prints, and nothing here inspects what it carries: file
//! bytes, camera luma and entropy pass straight through (§5.3).

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;

use opensigner_core::{AssuranceTier, BuildInfo, OpenSigner};
use opensigner_scanner::Scanner;
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, FileKind, Key, SecureHardware,
    TouchPhase,
};

pub mod jni;

// ---------------------------------------------------------------------------
// The integer vocabulary shared with the shell.
// ---------------------------------------------------------------------------

/// [`poll`]: no command is pending.
pub const CMD_NONE: i32 = 0;
/// [`poll`]: blit the frame ([`Command::Draw`]).
pub const CMD_DRAW: i32 = 1;
/// [`poll`]: leave ([`Command::Exit`]).
pub const CMD_EXIT: i32 = 2;
/// [`poll`]: ask the user for a file ([`Command::RequestFile`], and
/// [`Command::ReadFile`], which a shell with a picker of its own
/// answers the same way); the kind is in [`command_kind`].
pub const CMD_REQUEST_FILE: i32 = 3;
/// [`poll`]: store bytes ([`Command::WriteFile`]); the kind, the name hint
/// and the bytes are in [`command_kind`], [`command_name`] and
/// [`command_bytes`]. The shell answers with exactly one [`file_written`]
/// or [`file_not_written`].
pub const CMD_WRITE_FILE: i32 = 4;
/// [`poll`]: 32 random bytes, please ([`Command::RequestEntropy`]).
pub const CMD_REQUEST_ENTROPY: i32 = 5;
/// [`poll`]: start the camera ([`Command::CameraOn`]).
pub const CMD_CAMERA_ON: i32 = 6;
/// [`poll`]: stop the camera ([`Command::CameraOff`]).
pub const CMD_CAMERA_OFF: i32 = 7;
/// [`poll`]: vibrate ([`Command::Vibrate`]); the duration is in
/// [`command_ms`].
pub const CMD_VIBRATE: i32 = 8;
/// [`poll`]: keep the settings ([`Command::StoreSettings`]); the bytes
/// are in [`command_bytes`]. The shell writes them where it keeps
/// application data and hands them back with [`settings`] next time.
pub const CMD_STORE_SETTINGS: i32 = 9;
/// [`poll`]: authenticate `salt` inside the secure element
/// ([`Command::SecureMac`]); the 32-byte salt is in [`command_bytes`].
/// The shell answers with exactly one [`secure_mac`] or
/// [`secure_unavailable`].
pub const CMD_SECURE_MAC: i32 = 10;
/// [`poll`]: keep the blob ([`Command::StoreSecret`]); the bytes are in
/// [`command_bytes`]. The shell answers with exactly one
/// [`secret_stored`] or [`secret_not_stored`].
pub const CMD_STORE_SECRET: i32 = 11;
/// [`poll`]: hand back the kept blob ([`Command::LoadSecret`]). The
/// shell answers with exactly one [`secret`] or [`secret_unavailable`].
pub const CMD_LOAD_SECRET: i32 = 12;
/// [`poll`]: delete the blob and the hardware keys behind it
/// ([`Command::ForgetSecret`]). The shell answers with exactly one
/// [`secret_forgotten`].
pub const CMD_FORGET_SECRET: i32 = 13;
/// [`poll`]: read the clipboard ([`Command::RequestClipboard`]); the
/// kind is in [`command_kind`]. The shell answers with exactly one
/// [`clipboard`] or [`clipboard_unavailable`].
pub const CMD_REQUEST_CLIPBOARD: i32 = 14;
/// [`poll`]: put text on the clipboard ([`Command::WriteClipboard`]);
/// the kind and the text are in [`command_kind`] and [`command_text`].
/// The shell answers with exactly one [`clipboard_written`] or
/// [`clipboard_not_written`].
pub const CMD_WRITE_CLIPBOARD: i32 = 15;

/// Returned by [`poll`] for an unknown handle or a caught panic. A shell
/// that sees it stops draining and, in practice, has nothing left to do.
pub const CMD_ERROR: i32 = -1;

/// [`command_kind`]: the core wants a PSBT ([`FileKind::Psbt`]).
pub const FILE_PSBT: i32 = 0;
/// [`command_kind`]: any file ([`FileKind::Any`]).
pub const FILE_ANY: i32 = 1;
/// [`command_kind`]: text, which is what a clipboard carries
/// ([`FileKind::Text`]).
pub const FILE_TEXT: i32 = 2;
/// [`command_kind`]: a PNG image ([`FileKind::Png`]).
pub const FILE_PNG: i32 = 3;
/// [`command_kind`]: the last command carried no file kind.
pub const FILE_NONE: i32 = -1;

/// [`touch`]: contact began ([`TouchPhase::Down`]).
pub const TOUCH_DOWN: i32 = 0;
/// [`touch`]: contact moved ([`TouchPhase::Move`]).
pub const TOUCH_MOVE: i32 = 1;
/// [`touch`]: contact ended ([`TouchPhase::Up`]).
pub const TOUCH_UP: i32 = 2;

/// [`key`]: a printable character, whose code point is the second argument.
pub const KEY_CHAR: i32 = 0;
/// [`key`]: [`Key::Backspace`].
pub const KEY_BACKSPACE: i32 = 1;
/// [`key`]: [`Key::Enter`].
pub const KEY_ENTER: i32 = 2;
/// [`key`]: [`Key::Escape`], which the core reads as "go back".
pub const KEY_ESCAPE: i32 = 3;
/// [`key`]: [`Key::Up`].
pub const KEY_UP: i32 = 4;
/// [`key`]: [`Key::Down`].
pub const KEY_DOWN: i32 = 5;
/// [`key`]: [`Key::Left`].
pub const KEY_LEFT: i32 = 6;
/// [`key`]: [`Key::Right`].
pub const KEY_RIGHT: i32 = 7;
/// [`key`]: [`Key::Tab`].
pub const KEY_TAB: i32 = 8;

/// [`create`]: no secure element ([`SecureHardware::None`]).
pub const SECURE_NONE: i32 = 0;
/// [`create`]: a trusted execution environment ([`SecureHardware::Tee`]).
pub const SECURE_TEE: i32 = 1;
/// [`create`]: a separate chip ([`SecureHardware::StrongBox`]).
pub const SECURE_STRONGBOX: i32 = 2;

/// [`create`]: the shell cannot tell how the device booted
/// ([`BootState::Unknown`]).
pub const BOOT_UNKNOWN: i32 = 0;
/// [`create`]: the running system is the signed one ([`BootState::Verified`]).
pub const BOOT_VERIFIED: i32 = 1;
/// [`create`]: it is not ([`BootState::Unverified`]).
pub const BOOT_UNVERIFIED: i32 = 2;

/// How many bytes a [`Command::SecureMac`] salt and an
/// [`Event::SecureMac`] tag carry.
pub const MAC_LEN: usize = 32;

/// How many bytes an [`Event::Entropy`] carries.
pub const ENTROPY_LEN: usize = 32;

/// Most instances a process may hold at once. One is the real number; the
/// cap exists so a shell bug cannot grow the table without bound.
const MAX_INSTANCES: usize = 8;

// ---------------------------------------------------------------------------
// Mappings, all total and all testable.
// ---------------------------------------------------------------------------

/// Tier code → [`AssuranceTier`]. An unknown code is an error rather than a
/// default: the tier is a security statement the user reads, so guessing it
/// would be worse than refusing to start.
#[must_use]
pub fn tier_of(code: i32) -> Option<AssuranceTier> {
    match code {
        0 => Some(AssuranceTier::A),
        1 => Some(AssuranceTier::B),
        2 => Some(AssuranceTier::C),
        3 => Some(AssuranceTier::D),
        _ => None,
    }
}

/// Touch-phase code → [`TouchPhase`].
#[must_use]
pub fn phase_of(code: i32) -> Option<TouchPhase> {
    match code {
        TOUCH_DOWN => Some(TouchPhase::Down),
        TOUCH_MOVE => Some(TouchPhase::Move),
        TOUCH_UP => Some(TouchPhase::Up),
        _ => None,
    }
}

/// Key code (plus a code point for [`KEY_CHAR`]) → [`Key`]. A control
/// character or an unpaired surrogate is not a key and yields `None`.
#[must_use]
pub fn key_of(code: i32, ch: i32) -> Option<Key> {
    match code {
        KEY_CHAR => u32::try_from(ch)
            .ok()
            .and_then(char::from_u32)
            .filter(|c| !c.is_control())
            .map(Key::Char),
        KEY_BACKSPACE => Some(Key::Backspace),
        KEY_ENTER => Some(Key::Enter),
        KEY_ESCAPE => Some(Key::Escape),
        KEY_UP => Some(Key::Up),
        KEY_DOWN => Some(Key::Down),
        KEY_LEFT => Some(Key::Left),
        KEY_RIGHT => Some(Key::Right),
        KEY_TAB => Some(Key::Tab),
        _ => None,
    }
}

/// Secure-hardware code → [`SecureHardware`]. An unknown code is an
/// error rather than a default, as the tier is: what backs the secure
/// element decides whether a key may be kept on the device at all.
#[must_use]
pub fn secure_of(code: i32) -> Option<SecureHardware> {
    match code {
        SECURE_NONE => Some(SecureHardware::None),
        SECURE_TEE => Some(SecureHardware::Tee),
        SECURE_STRONGBOX => Some(SecureHardware::StrongBox),
        _ => None,
    }
}

/// Verified-boot code → [`BootState`]. An unknown code is
/// [`BootState::Unknown`]: what the shell could not say, and what About
/// then states, rather than a device that refuses to start.
#[must_use]
pub fn boot_of(code: i32) -> BootState {
    match code {
        BOOT_VERIFIED => BootState::Verified,
        BOOT_UNVERIFIED => BootState::Unverified,
        _ => BootState::Unknown,
    }
}

/// File-kind code → [`FileKind`].
#[must_use]
pub fn file_kind_of(code: i32) -> Option<FileKind> {
    match code {
        FILE_PSBT => Some(FileKind::Psbt),
        FILE_TEXT => Some(FileKind::Text),
        FILE_ANY => Some(FileKind::Any),
        FILE_PNG => Some(FileKind::Png),
        _ => None,
    }
}

/// [`FileKind`] → the code the shell reads from [`command_kind`].
#[must_use]
pub fn file_kind_code(kind: FileKind) -> i32 {
    match kind {
        FileKind::Psbt => FILE_PSBT,
        FileKind::Text => FILE_TEXT,
        FileKind::Any => FILE_ANY,
        FileKind::Png => FILE_PNG,
    }
}

// ---------------------------------------------------------------------------
// The payload of the last polled command.
// ---------------------------------------------------------------------------

/// What [`poll`] parked for the accessors. Reset on every poll, so a shell
/// that reads an accessor after the wrong command gets a neutral answer
/// rather than a stale one.
#[derive(Default)]
struct Payload {
    kind: i32,
    bytes: Option<Vec<u8>>,
    name: Option<String>,
    text: Option<String>,
    ms: i32,
}

/// Splits a command into its code and the payload the accessors serve.
fn encode(command: Command) -> (i32, Payload) {
    let none = Payload {
        kind: FILE_NONE,
        ..Payload::default()
    };
    match command {
        Command::Draw => (CMD_DRAW, none),
        Command::Exit => (CMD_EXIT, none),
        Command::CameraOn => (CMD_CAMERA_ON, none),
        Command::CameraOff => (CMD_CAMERA_OFF, none),
        Command::RequestEntropy => (CMD_REQUEST_ENTROPY, none),
        Command::Vibrate { ms } => (
            CMD_VIBRATE,
            Payload {
                ms: i32::from(ms),
                ..none
            },
        ),
        // A shell reached over this boundary has a picker of its own
        // (Android's document picker), so it is asked for a file and
        // never for a listing. A `ReadFile` that reaches it anyway is a
        // request for a file of that kind, which it can answer.
        Command::RequestFile { kind } | Command::ReadFile { kind, .. } => (
            CMD_REQUEST_FILE,
            Payload {
                kind: file_kind_code(kind),
                ..none
            },
        ),
        Command::RequestClipboard { kind } => (
            CMD_REQUEST_CLIPBOARD,
            Payload {
                kind: file_kind_code(kind),
                ..none
            },
        ),
        Command::WriteClipboard { kind, text } => (
            CMD_WRITE_CLIPBOARD,
            Payload {
                kind: file_kind_code(kind),
                text: Some(text),
                ..none
            },
        ),
        Command::LoadSecret => (CMD_LOAD_SECRET, none),
        Command::ForgetSecret => (CMD_FORGET_SECRET, none),
        Command::SecureMac { salt } => (
            CMD_SECURE_MAC,
            Payload {
                bytes: Some(salt.to_vec()),
                ..none
            },
        ),
        Command::StoreSecret { blob } => (
            CMD_STORE_SECRET,
            Payload {
                bytes: Some(blob),
                ..none
            },
        ),
        Command::StoreSettings { bytes } => (
            CMD_STORE_SETTINGS,
            Payload {
                bytes: Some(bytes),
                ..none
            },
        ),
        Command::WriteFile {
            kind,
            name_hint,
            bytes,
        } => (
            CMD_WRITE_FILE,
            Payload {
                kind: file_kind_code(kind),
                bytes: Some(bytes),
                name: Some(name_hint),
                ..none
            },
        ),
    }
}

// ---------------------------------------------------------------------------
// The handle table.
// ---------------------------------------------------------------------------

/// One live app plus the payload of its last polled command.
struct Instance {
    app: OpenSigner,
    payload: Payload,
    /// Reads the codes in the camera's frames on a worker thread, so
    /// that a decode never holds up the UI thread the core runs on.
    /// Alive from the first frame until the camera is turned off.
    scanner: Option<Scanner>,
}

/// A table entry. `generation` counts how many instances the slot has held,
/// so a handle from an earlier tenant never matches the current one.
struct Slot {
    generation: u32,
    instance: Option<Box<Instance>>,
}

thread_local! {
    /// The live apps of this thread. Thread-local rather than global
    /// because [`OpenSigner`] is not `Send`; see the crate documentation.
    static TABLE: RefCell<Vec<Slot>> = const { RefCell::new(Vec::new()) };
}

/// Generations use 31 bits, so that every handle is a positive `i64` and a
/// shell can treat "not positive" as "no app".
const GENERATION_BITS: u32 = 0x7fff_ffff;

/// The generation a slot takes when it is reused. Never `0`, so that a
/// handle always carries a generation a live slot could have.
fn next_generation(generation: u32) -> u32 {
    match (generation + 1) & GENERATION_BITS {
        0 => 1,
        next => next,
    }
}

/// Packs a slot index and its generation into the handle a shell holds.
/// The index is stored one-based so that `0` is never a valid handle.
#[must_use]
pub fn encode_handle(index: usize, generation: u32) -> i64 {
    let index = index as u64 + 1;
    ((u64::from(generation & GENERATION_BITS) << 32) | index) as i64
}

/// Unpacks a handle. `None` for `0`, for a negative value, and for anything
/// whose index half is out of range.
#[must_use]
pub fn decode_handle(handle: i64) -> Option<(usize, u32)> {
    let handle = u64::try_from(handle).ok()?;
    let index = (handle & 0xffff_ffff) as usize;
    let generation = (handle >> 32) as u32 & GENERATION_BITS;
    index.checked_sub(1).map(|i| (i, generation))
}

/// Runs `f` over the instance a handle names. `None` when the handle names
/// nothing: freed, never issued, from an earlier tenant of the slot, or
/// created on another thread.
fn with<R>(handle: i64, f: impl FnOnce(&mut Instance) -> R) -> Option<R> {
    let (index, generation) = decode_handle(handle)?;
    TABLE.with_borrow_mut(|slots| {
        let slot = slots.get_mut(index)?;
        if slot.generation != generation {
            return None;
        }
        slot.instance.as_mut().map(|i| f(i))
    })
}

/// Catches a panic from `f` and answers `fallback` instead, so that nothing
/// unwinds into the caller's foreign frame.
fn guard<R>(fallback: R, f: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

// ---------------------------------------------------------------------------
// Version strings.
// ---------------------------------------------------------------------------

/// [`BuildInfo`] holds a `&'static str`, and the shell's version is only
/// known at run time, so the string is interned: leaked once and reused for
/// every later instance with the same version. A process sees one or two
/// distinct versions in its life, so the set stays tiny.
fn intern(version: &str) -> &'static str {
    static VERSIONS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut versions = VERSIONS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(found) = versions.iter().find(|v| **v == version) {
        return found;
    }
    let leaked: &'static str = Box::leak(version.to_owned().into_boxed_str());
    versions.push(leaked);
    leaked
}

// ---------------------------------------------------------------------------
// The surface itself. Every function is total: a bad handle does nothing.
// ---------------------------------------------------------------------------

/// Creates an app for a `width × height` display at `dpi`, sends it
/// [`Event::Display`], and returns its handle. `0` means the app was not
/// created: an unknown tier, a size that is not a positive `u16`, or a full
/// table.
///
/// `inset_bottom` and `inset_top` are the strips of the frame at each
/// edge the person cannot use, in pixels; a shell that already padded
/// its view by the system bars passes zero, and the core then treats the
/// whole frame as usable.
///
/// `version` is what the About screen shows; `None` falls back to this
/// crate's version. `camera_fixed` says the shell's camera frames are
/// always upright, which a phone's are. `secure` is one of
/// [`SECURE_NONE`], [`SECURE_TEE`] and [`SECURE_STRONGBOX`]: what backs
/// the shell's secure element, and so whether a key may be kept on the
/// device. `boot` is one of [`BOOT_UNKNOWN`], [`BOOT_VERIFIED`] and
/// [`BOOT_UNVERIFIED`]: what the platform's attestation says about the
/// device's boot, which About states and which cautions the person
/// before a key is kept. `memory_mib` is how much memory the device has
/// in MiB, or a value of zero or less where the shell cannot say; it
/// decides which Argon2id memory cost Settings recommends for an
/// encrypted backup (`docs/PLANNING.md` §16.112).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn create(
    width: i32,
    height: i32,
    dpi: i32,
    inset_bottom: i32,
    inset_top: i32,
    tier: i32,
    version: Option<&str>,
    camera_fixed: bool,
    secure: i32,
    boot: i32,
    memory_mib: i32,
) -> i64 {
    let (Ok(width), Ok(height), Ok(dpi), Ok(inset_bottom), Ok(inset_top)) = (
        u16::try_from(width),
        u16::try_from(height),
        u16::try_from(dpi),
        u16::try_from(inset_bottom),
        u16::try_from(inset_top),
    ) else {
        return 0;
    };
    let (Some(tier), Some(secure), true, true) = (
        tier_of(tier),
        secure_of(secure),
        width > 0 && height > 0,
        dpi > 0,
    ) else {
        return 0;
    };
    let build = BuildInfo {
        version: intern(version.unwrap_or(env!("CARGO_PKG_VERSION"))),
        core_hash: None,
    };
    let mut app = OpenSigner::new(tier, build);
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom,
        inset_top,
        // Touch only: no DIY buttons on a phone.
        buttons: 0,
        camera_fixed,
        secure,
        boot: boot_of(boot),
        memory_mib: u32::try_from(memory_mib).ok().filter(|mib| *mib > 0),
    }));
    let instance = Box::new(Instance {
        app,
        payload: Payload::default(),
        scanner: None,
    });

    TABLE.with_borrow_mut(|slots| {
        if let Some((index, slot)) = slots
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.instance.is_none())
        {
            slot.generation = next_generation(slot.generation);
            slot.instance = Some(instance);
            return encode_handle(index, slot.generation);
        }
        if slots.len() >= MAX_INSTANCES {
            return 0;
        }
        slots.push(Slot {
            generation: 1,
            instance: Some(instance),
        });
        encode_handle(slots.len() - 1, 1)
    })
}

/// Drops the app a handle names, zeroizing everything it holds. Freeing an
/// already-freed or unknown handle does nothing. The handle is dead
/// afterwards: the slot's generation moves on.
pub fn destroy(handle: i64) {
    let Some((index, generation)) = decode_handle(handle) else {
        return;
    };
    // Taken under the borrow and dropped outside it: `OpenSigner`'s
    // zeroizing drop is real work, and nothing should run it while the
    // table is borrowed.
    let instance = TABLE.with_borrow_mut(|slots| {
        let slot = slots.get_mut(index)?;
        if slot.generation != generation {
            return None;
        }
        slot.generation = next_generation(slot.generation);
        slot.instance.take()
    });
    drop(instance);
}

/// Delivers one event.
fn send(handle: i64, event: Event) {
    with(handle, |i| i.app.event(event));
}

/// A touch contact at a framebuffer pixel. An unknown phase is ignored.
pub fn touch(handle: i64, x: i32, y: i32, phase: i32) {
    let (Some(phase), Ok(x), Ok(y)) = (phase_of(phase), u16::try_from(x), u16::try_from(y)) else {
        return;
    };
    send(handle, Event::Touch { x, y, phase });
}

/// A scroll of `dy` framebuffer pixels at a pixel position; positive `dy`
/// moves the content up.
pub fn scroll(handle: i64, x: i32, y: i32, dy: i32) {
    let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
        return;
    };
    let dy = dy.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
    send(handle, Event::Scroll { x, y, dy });
}

/// Monotonic time in milliseconds. A negative value is ignored rather than
/// wrapped: the core's contract is that time never goes backwards.
pub fn tick(handle: i64, now_ms: i64) {
    let Ok(now_ms) = u64::try_from(now_ms) else {
        return;
    };
    // Whatever the worker read since the last tick, first: a code is
    // routed on the tick that follows the frame it was in, and the
    // shell drains the commands after this call either way.
    with(handle, |i| {
        while let Some(bytes) = i.scanner.as_ref().and_then(Scanner::poll) {
            i.app.event(Event::Scanned { bytes });
        }
        i.app.event(Event::Tick { now_ms });
    });
}

/// A key press. Mobile shells send only [`KEY_ESCAPE`], for the system back
/// gesture; text goes through the core's own keyboard (§4.5).
pub fn key(handle: i64, code: i32, ch: i32) {
    let Some(key) = key_of(code, ch) else {
        return;
    };
    send(handle, Event::Key(key));
}

/// The answer to [`CMD_REQUEST_FILE`]: the file's bytes, unmodified.
pub fn file(handle: i64, kind: i32, bytes: Vec<u8>) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::File { kind, bytes });
}

/// The answer to [`CMD_REQUEST_FILE`] from a shell with no file channel
/// at all, and from one whose read failed.
pub fn file_unavailable(handle: i64, kind: i32) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::FileUnavailable { kind });
}

/// The answer to [`CMD_REQUEST_FILE`] when the person closed the picker
/// without choosing.
pub fn file_cancelled(handle: i64, kind: i32) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::FileCancelled { kind });
}

/// The answer to [`CMD_WRITE_FILE`]: the bytes are stored.
pub fn file_written(handle: i64, kind: i32) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::FileWritten { kind });
}

/// The answer to [`CMD_WRITE_FILE`] when nothing was stored: the user
/// cancelled the picker, or the write failed.
pub fn file_not_written(handle: i64, kind: i32) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::FileNotWritten { kind });
}

/// The answer to [`CMD_REQUEST_CLIPBOARD`]: what the clipboard holds,
/// as text.
pub fn clipboard(handle: i64, kind: i32, text: String) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::Clipboard { kind, text });
}

/// The answer to [`CMD_REQUEST_CLIPBOARD`] when no text comes: nothing
/// on the clipboard, no clipboard on this shell, or content that is not
/// text.
pub fn clipboard_unavailable(handle: i64, kind: i32) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::ClipboardUnavailable { kind });
}

/// The answer to [`CMD_WRITE_CLIPBOARD`]: the text is on the clipboard.
pub fn clipboard_written(handle: i64, kind: i32) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::ClipboardWritten { kind });
}

/// The answer to [`CMD_WRITE_CLIPBOARD`] when nothing was put there.
pub fn clipboard_not_written(handle: i64, kind: i32) {
    let Some(kind) = file_kind_of(kind) else {
        return;
    };
    send(handle, Event::ClipboardNotWritten { kind });
}

/// The answer to [`CMD_REQUEST_ENTROPY`]: exactly [`ENTROPY_LEN`] bytes from
/// the shell's random number generator. A wrong length is dropped, and the
/// core carries on with a weak session rather than a short one.
pub fn entropy(handle: i64, bytes: &[u8]) {
    let Ok(bytes) = <[u8; ENTROPY_LEN]>::try_from(bytes) else {
        return;
    };
    send(handle, Event::Entropy(EntropyBytes::new(bytes)));
}

/// The shell is leaving the foreground: lock the session now rather
/// than waiting for its timeout. A session with no PIN or no keys has
/// nothing to lock and this does nothing.
pub fn lock(handle: i64) {
    send(handle, Event::Lock);
}

/// The settings this shell kept from an earlier session, as the core
/// wrote them. Sent once, after the display and before any input; a
/// shell with nothing kept sends nothing.
pub fn settings(handle: i64, bytes: Vec<u8>) {
    send(handle, Event::Settings { bytes });
}

/// One camera frame: 8-bit luma, `width × height` bytes, with the NV12
/// chroma plane beside it where the camera gave colour. A chroma plane
/// of the wrong length is dropped and the preview is grey.
pub fn camera_frame(handle: i64, width: i32, height: i32, luma: Vec<u8>, chroma: Option<Vec<u8>>) {
    let (Ok(width), Ok(height)) = (u16::try_from(width), u16::try_from(height)) else {
        return;
    };
    if luma.len() != usize::from(width) * usize::from(height) {
        return;
    }
    let pairs = usize::from(width).div_ceil(2) * usize::from(height).div_ceil(2) * 2;
    let chroma = chroma.filter(|uv| uv.len() == pairs);
    with(handle, |i| {
        // The frame goes twice: to the core, which draws the preview,
        // and to the worker, which reads the codes in it. One copy,
        // since the core takes the original.
        i.scanner
            .get_or_insert_with(Scanner::new)
            .offer(width, height, luma.clone());
        i.app.event(Event::CameraFrame {
            width,
            height,
            luma,
            chroma,
        });
    });
}

/// The answer to [`CMD_CAMERA_ON`] when no frames will come.
pub fn camera_unavailable(handle: i64) {
    send(handle, Event::CameraUnavailable);
}

/// The answer to [`CMD_SECURE_MAC`]: exactly [`MAC_LEN`] bytes of
/// HMAC-SHA256 computed inside the secure element. A wrong length is
/// dropped, so a short tag is never mistaken for a real one.
pub fn secure_mac(handle: i64, mac: &[u8]) {
    if let Ok(mac) = <[u8; MAC_LEN]>::try_from(mac) {
        send(handle, Event::SecureMac { mac });
    }
}

/// The answer to [`CMD_SECURE_MAC`] when no tag will come: no secure
/// hardware, a cancelled authentication, or the key is gone.
pub fn secure_unavailable(handle: i64) {
    send(handle, Event::SecureUnavailable);
}

/// Whether this shell is holding a kept blob. Sent once, after the
/// display and before any input, by a shell that can keep one.
pub fn secret_kept(handle: i64, kept: bool) {
    send(handle, Event::SecretKept { kept });
}

/// The answer to [`CMD_STORE_SECRET`]: the bytes are kept.
pub fn secret_stored(handle: i64) {
    send(handle, Event::SecretStored);
}

/// The answer to [`CMD_STORE_SECRET`] when nothing was kept.
pub fn secret_not_stored(handle: i64) {
    send(handle, Event::SecretNotStored);
}

/// The answer to [`CMD_LOAD_SECRET`]: the blob, as it was stored.
pub fn secret(handle: i64, blob: Vec<u8>) {
    send(handle, Event::Secret { blob });
}

/// The answer to [`CMD_LOAD_SECRET`] when no blob comes.
pub fn secret_unavailable(handle: i64) {
    send(handle, Event::SecretUnavailable);
}

/// The answer to [`CMD_FORGET_SECRET`]: the blob and the hardware keys
/// behind it are gone.
pub fn secret_forgotten(handle: i64) {
    send(handle, Event::SecretForgotten);
}

/// Takes the next pending command as a `CMD_*` code, parking its payload
/// for the accessors below. [`CMD_NONE`] when the queue is empty,
/// [`CMD_ERROR`] for an unknown handle.
#[must_use]
pub fn poll(handle: i64) -> i32 {
    with(handle, |i| match i.app.poll_command() {
        Some(command) => {
            // The camera is off: the worker ends here, and the frame
            // and any payload it still held are wiped with it.
            if command == Command::CameraOff {
                i.scanner = None;
            }
            let (code, payload) = encode(command);
            i.payload = payload;
            code
        }
        None => {
            i.payload = Payload::default();
            CMD_NONE
        }
    })
    .unwrap_or(CMD_ERROR)
}

/// The file kind of the last polled command, or [`FILE_NONE`].
#[must_use]
pub fn command_kind(handle: i64) -> i32 {
    with(handle, |i| i.payload.kind).unwrap_or(FILE_NONE)
}

/// The bytes of the last polled command, taken: a second call answers
/// `None`. Taking them means the shell's copy is the only one, so a written
/// file is not held in Rust memory after it has been handed over.
#[must_use]
pub fn command_bytes(handle: i64) -> Option<Vec<u8>> {
    with(handle, |i| i.payload.bytes.take()).flatten()
}

/// The name hint of the last polled command. Taken, like the bytes:
/// one command, one read.
#[must_use]
pub fn command_name(handle: i64) -> Option<String> {
    with(handle, |i| i.payload.name.take()).flatten()
}

/// The text of the last polled command ([`CMD_WRITE_CLIPBOARD`]).
/// Taken, like the bytes: one command, one read.
#[must_use]
pub fn command_text(handle: i64) -> Option<String> {
    with(handle, |i| i.payload.text.take()).flatten()
}

/// The duration in milliseconds of the last polled command, or `0`.
#[must_use]
pub fn command_ms(handle: i64) -> i32 {
    with(handle, |i| i.payload.ms).unwrap_or(0)
}

/// The core-owned framebuffer as a pointer and a length in bytes:
/// premultiplied RGBA8888, row-major, `width × height × 4`.
///
/// The frame is painted here, from the layout the last event solved, if
/// nothing has read it since (`docs/PLANNING.md` §16.119 rule 5), so a
/// shell that reads it once per frame it draws paints once per drawn
/// frame rather than once per event.
///
/// **Lifetime.** The memory belongs to the core. It stays valid until the
/// next call that can touch this instance — any event, any poll, or
/// [`destroy`] — so a shell reads it (or copies out of it) before it calls
/// anything else, and never keeps the pointer across a frame. In practice
/// the core allocates its canvas once, on [`Event::Display`], and never
/// moves it; the rule is stated tightly anyway so that a later core is free
/// to reallocate.
///
/// `None` for an unknown handle.
#[must_use]
pub fn frame_ptr(handle: i64) -> Option<(*const u8, usize)> {
    with(handle, |i| {
        let frame = i.app.frame();
        (frame.rgba.as_ptr(), frame.rgba.len())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_round_trip() {
        for index in [0usize, 1, 7, 4095] {
            for generation in [1u32, 2, GENERATION_BITS] {
                let handle = encode_handle(index, generation);
                assert!(handle > 0);
                assert_eq!(decode_handle(handle), Some((index, generation)));
            }
        }
    }

    #[test]
    fn generations_stay_positive_and_skip_zero() {
        assert_eq!(next_generation(1), 2);
        assert_eq!(next_generation(GENERATION_BITS), 1);
        assert!(encode_handle(0, GENERATION_BITS) > 0);
    }

    #[test]
    fn zero_and_negative_handles_name_nothing() {
        assert_eq!(decode_handle(0), None);
        assert_eq!(decode_handle(-1), None);
        assert_eq!(decode_handle(i64::MIN), None);
    }

    #[test]
    fn a_freed_handle_is_dead() {
        let handle = create(
            320,
            568,
            160,
            0,
            0,
            2,
            Some("test"),
            true,
            SECURE_NONE,
            BOOT_UNKNOWN,
            0,
        );
        assert_ne!(handle, 0);
        // Display was sent, so there is a frame and at least a Draw.
        assert!(frame_ptr(handle).is_some());
        destroy(handle);
        assert_eq!(poll(handle), CMD_ERROR);
        assert_eq!(frame_ptr(handle), None);
        // Every event is a no-op rather than a crash.
        touch(handle, 1, 1, TOUCH_DOWN);
        tick(handle, 1);
        destroy(handle);
    }

    #[test]
    fn a_reused_slot_does_not_answer_the_old_handle() {
        let first = create(
            320,
            568,
            160,
            0,
            0,
            2,
            None,
            true,
            SECURE_NONE,
            BOOT_UNKNOWN,
            0,
        );
        destroy(first);
        let second = create(
            320,
            568,
            160,
            0,
            0,
            2,
            None,
            true,
            SECURE_NONE,
            BOOT_UNKNOWN,
            0,
        );
        assert_ne!(first, second);
        assert_eq!(poll(first), CMD_ERROR);
        assert_ne!(poll(second), CMD_ERROR);
        destroy(second);
    }

    #[test]
    fn a_bad_tier_size_or_secure_code_creates_nothing() {
        assert_eq!(
            create(
                320,
                568,
                160,
                0,
                0,
                4,
                None,
                true,
                SECURE_NONE,
                BOOT_UNKNOWN,
                0
            ),
            0
        );
        assert_eq!(
            create(
                320,
                568,
                160,
                0,
                0,
                -1,
                None,
                true,
                SECURE_NONE,
                BOOT_UNKNOWN,
                0
            ),
            0
        );
        assert_eq!(
            create(
                0,
                568,
                160,
                0,
                0,
                2,
                None,
                true,
                SECURE_NONE,
                BOOT_UNKNOWN,
                0
            ),
            0
        );
        assert_eq!(
            create(
                320,
                568,
                0,
                0,
                0,
                2,
                None,
                true,
                SECURE_NONE,
                BOOT_UNKNOWN,
                0
            ),
            0
        );
        assert_eq!(
            create(
                70_000,
                568,
                160,
                0,
                0,
                2,
                None,
                true,
                SECURE_NONE,
                BOOT_UNKNOWN,
                0
            ),
            0
        );
        assert_eq!(
            create(320, 568, 160, 0, 0, 2, None, true, 7, BOOT_UNKNOWN, 0),
            0
        );
    }

    #[test]
    fn display_produces_a_frame_and_a_draw() {
        let handle = create(
            320,
            568,
            160,
            0,
            0,
            2,
            Some("0.1.0-test"),
            true,
            SECURE_NONE,
            BOOT_UNKNOWN,
            0,
        );
        let (ptr, len) = frame_ptr(handle).expect("a frame after Display");
        assert!(!ptr.is_null());
        assert_eq!(len, 320 * 568 * 4);
        let mut codes = Vec::new();
        loop {
            let code = poll(handle);
            if code == CMD_NONE {
                break;
            }
            codes.push(code);
        }
        assert!(codes.contains(&CMD_DRAW));
        assert!(codes.contains(&CMD_REQUEST_ENTROPY));
        destroy(handle);
    }

    #[test]
    fn command_encoding_covers_every_variant() {
        let (code, payload) = encode(Command::Draw);
        assert_eq!((code, payload.kind, payload.ms), (CMD_DRAW, FILE_NONE, 0));

        let (code, payload) = encode(Command::Vibrate { ms: 40 });
        assert_eq!((code, payload.ms), (CMD_VIBRATE, 40));

        let (code, payload) = encode(Command::RequestFile {
            kind: FileKind::Psbt,
        });
        assert_eq!((code, payload.kind), (CMD_REQUEST_FILE, FILE_PSBT));
        assert!(payload.bytes.is_none() && payload.name.is_none());

        // A shell over this boundary picks the file itself, so a read
        // of a listed name reaches it as a request for a file of that
        // kind, which it can answer.
        let (code, payload) = encode(Command::ReadFile {
            kind: FileKind::Psbt,
            name: String::from("today.psbt"),
        });
        assert_eq!((code, payload.kind), (CMD_REQUEST_FILE, FILE_PSBT));

        let (code, payload) = encode(Command::WriteFile {
            kind: FileKind::Any,
            name_hint: String::from("signed.psbt"),
            bytes: vec![1, 2, 3],
        });
        assert_eq!((code, payload.kind), (CMD_WRITE_FILE, FILE_ANY));
        assert_eq!(payload.name.as_deref(), Some("signed.psbt"));
        assert_eq!(payload.bytes.as_deref(), Some(&[1u8, 2, 3][..]));

        let (code, payload) = encode(Command::StoreSettings {
            bytes: b"opensigner-settings 1\n".to_vec(),
        });
        assert_eq!((code, payload.kind), (CMD_STORE_SETTINGS, FILE_NONE));
        assert_eq!(payload.name, None);
        assert_eq!(
            payload.bytes.as_deref(),
            Some(&b"opensigner-settings 1\n"[..])
        );

        let (code, payload) = encode(Command::SecureMac { salt: [9; MAC_LEN] });
        assert_eq!(code, CMD_SECURE_MAC);
        assert_eq!(payload.bytes.as_deref(), Some(&[9u8; MAC_LEN][..]));

        let (code, payload) = encode(Command::StoreSecret { blob: vec![4; 5] });
        assert_eq!(code, CMD_STORE_SECRET);
        assert_eq!(payload.bytes.as_deref(), Some(&[4u8; 5][..]));

        for (command, expected) in [
            (Command::Exit, CMD_EXIT),
            (Command::CameraOn, CMD_CAMERA_ON),
            (Command::CameraOff, CMD_CAMERA_OFF),
            (Command::RequestEntropy, CMD_REQUEST_ENTROPY),
            (Command::LoadSecret, CMD_LOAD_SECRET),
            (Command::ForgetSecret, CMD_FORGET_SECRET),
        ] {
            assert_eq!(encode(command).0, expected);
        }
    }

    #[test]
    fn accessors_are_neutral_for_an_unknown_handle() {
        assert_eq!(command_kind(0), FILE_NONE);
        assert_eq!(command_ms(0), 0);
        assert_eq!(command_name(0), None);
        assert_eq!(command_bytes(0), None);
        assert_eq!(poll(0), CMD_ERROR);
    }

    #[test]
    fn write_file_payload_reaches_the_shell_once() {
        // The Sign flow is long; drive the payload path through `encode`
        // and the accessors instead, which is what the shell sees.
        let handle = create(
            320,
            568,
            160,
            0,
            0,
            2,
            None,
            true,
            SECURE_NONE,
            BOOT_UNKNOWN,
            0,
        );
        with(handle, |i| {
            let (_, payload) = encode(Command::WriteFile {
                kind: FileKind::Psbt,
                name_hint: String::from("signed.psbt"),
                bytes: vec![7; 4],
            });
            i.payload = payload;
        });
        assert_eq!(command_kind(handle), FILE_PSBT);
        assert_eq!(command_name(handle).as_deref(), Some("signed.psbt"));
        assert_eq!(command_bytes(handle).as_deref(), Some(&[7u8; 4][..]));
        assert_eq!(command_bytes(handle), None, "bytes are handed over once");
        destroy(handle);
    }

    #[test]
    fn key_mapping_rejects_control_characters_and_unknown_codes() {
        assert_eq!(key_of(KEY_CHAR, 'a' as i32), Some(Key::Char('a')));
        assert_eq!(key_of(KEY_CHAR, '\n' as i32), None);
        assert_eq!(key_of(KEY_CHAR, 0xd800), None);
        assert_eq!(key_of(KEY_CHAR, -1), None);
        assert_eq!(key_of(KEY_ESCAPE, 0), Some(Key::Escape));
        assert_eq!(key_of(99, 0), None);
    }

    #[test]
    fn tier_touch_and_file_mappings_are_total() {
        assert_eq!(tier_of(2), Some(AssuranceTier::C));
        assert_eq!(tier_of(9), None);
        assert_eq!(phase_of(TOUCH_UP), Some(TouchPhase::Up));
        assert_eq!(phase_of(3), None);
        assert_eq!(file_kind_of(FILE_ANY), Some(FileKind::Any));
        assert_eq!(file_kind_of(FILE_NONE), None);
        assert_eq!(file_kind_code(FileKind::Psbt), FILE_PSBT);
        assert_eq!(secure_of(SECURE_STRONGBOX), Some(SecureHardware::StrongBox));
        assert_eq!(secure_of(SECURE_TEE), Some(SecureHardware::Tee));
        assert_eq!(secure_of(-1), None);
        assert_eq!(boot_of(BOOT_VERIFIED), BootState::Verified);
        assert_eq!(boot_of(BOOT_UNVERIFIED), BootState::Unverified);
        assert_eq!(boot_of(-1), BootState::Unknown);
    }

    #[test]
    fn entropy_of_the_wrong_length_is_dropped() {
        let handle = create(
            320,
            568,
            160,
            0,
            0,
            2,
            None,
            true,
            SECURE_NONE,
            BOOT_UNKNOWN,
            0,
        );
        entropy(handle, &[0u8; 16]);
        entropy(handle, &[0u8; ENTROPY_LEN]);
        destroy(handle);
    }

    #[test]
    fn a_camera_frame_whose_length_disagrees_is_dropped() {
        let handle = create(
            320,
            568,
            160,
            0,
            0,
            2,
            None,
            true,
            SECURE_NONE,
            BOOT_UNKNOWN,
            0,
        );
        camera_frame(handle, 4, 4, vec![0; 15], None);
        camera_frame(handle, 4, 4, vec![0; 16], None);
        // A chroma plane that is not the one this luma calls for is
        // dropped, and the frame is still delivered in grey.
        camera_frame(handle, 4, 4, vec![0; 16], Some(vec![128; 7]));
        camera_frame(handle, 4, 4, vec![0; 16], Some(vec![128; 8]));
        destroy(handle);
    }

    #[test]
    fn a_caught_panic_returns_the_fallback() {
        assert_eq!(guard(CMD_ERROR, || panic!("boom")), CMD_ERROR);
        assert_eq!(guard(CMD_ERROR, || CMD_DRAW), CMD_DRAW);
    }
}
