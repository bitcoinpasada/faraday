//! The boundary between Faraday Companion's core ([`Companion`]) and its
//! Android shell (`docs/COMPANION.md` §3). A copy of upstream's
//! `opensigner/opensigner-ffi` at d26b84e, for the Companion: the same
//! handle table, the same command encoding, the same split.
//!
//! - This module is **safe Rust**: the handle table, the touch, key and
//!   file mappings, and the encoding of a command into the integers and
//!   byte strings the shell reads back. It is unit-tested on the host.
//! - [`jni`] is the only place with `unsafe`: it converts JNI arguments to
//!   Rust values, calls in here, and converts back. Every block is a few
//!   lines with the invariant it relies on written above it.
//!
//! ## Handles
//!
//! A shell never holds a Rust pointer. [`create`] returns a 64-bit handle
//! that indexes a table of boxed [`Companion`]s, with a generation counter
//! in the high half: a handle used after [`destroy`], or a handle that was
//! never issued, resolves to nothing instead of to freed memory. Every
//! entry point tolerates a bad handle by doing nothing.
//!
//! The table is thread-local, because [`Companion`] is not `Send`: the
//! core is single-threaded by construction. The shell creates, drives and
//! frees it on the UI thread; a call from any other thread finds nothing.
//!
//! ## Commands
//!
//! [`poll`] returns one command code and parks that command's payload on
//! the instance, which the shell reads with [`command_kind`],
//! [`command_bytes`], [`command_name`], [`command_text`], [`command_on`],
//! [`command_descriptions`] and [`command_keys`]. It answers upstream's commands first and then the
//! Companion's own ([`CompanionCommand`]), under codes of their own.
//!
//! Only the commands a Companion gives reach the shell. Upstream's
//! interface also has commands for entropy, kept secrets, the secure
//! element, haptics and writing the clipboard; the Companion gives none of
//! them, and were one to come, [`poll`] would drop it (wiping what it
//! carried) rather than hand it over. So no code here can put text on the
//! clipboard (§6.3).
//!
//! ## The camera
//!
//! A camera frame goes twice: to the core, which draws the preview, and to
//! a `faraday-scanner` worker, which reads the codes in it on its own
//! thread. What the worker read comes back to the core on the next
//! [`tick`] as [`Event::Scanned`], and what it saw of a code as
//! [`CompanionEvent::Seen`], so nothing decodes on the UI thread.
//!
//! ## The frame
//!
//! [`frame_ptr`] hands out a pointer to the core-owned framebuffer, which
//! the JNI layer wraps in a direct `java.nio.ByteBuffer`. Nothing is
//! copied on the Rust side; the shell copies once, into its bitmap.
//!
//! ## Panics
//!
//! No panic may cross into Java. Every entry point wraps its work in
//! [`std::panic::catch_unwind`] and returns a neutral value instead; the
//! release profile sets `panic = "abort"`, so a panic there ends the
//! process before it can unwind at all.
//!
//! ## No logging
//!
//! Nothing here prints, and nothing here inspects what it carries.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use faraday_companion::{Closed, Companion, CompanionCommand, CompanionEvent};
use faraday_scanner::Scanner;
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, Event, FileKind, Key, SecureHardware, TouchPhase,
};
use zeroize::Zeroize;

pub mod jni;

// ---------------------------------------------------------------------------
// The integer vocabulary shared with the shell. Upstream's codes keep their
// numbers; the Companion's own follow them.
// ---------------------------------------------------------------------------

/// [`poll`]: no command is pending.
pub const CMD_NONE: i32 = 0;
/// [`poll`]: blit the frame ([`Command::Draw`]).
pub const CMD_DRAW: i32 = 1;
/// [`poll`]: leave ([`Command::Exit`]).
pub const CMD_EXIT: i32 = 2;
/// [`poll`]: ask the person for a file with the document picker
/// ([`Command::RequestFile`]); the kind is in [`command_kind`]. The shell
/// answers with exactly one [`file`], [`file_unavailable`] or
/// [`file_cancelled`].
pub const CMD_REQUEST_FILE: i32 = 3;
/// [`poll`]: store bytes where the person picks ([`Command::WriteFile`]);
/// the kind, the name hint and the bytes are in [`command_kind`],
/// [`command_name`] and [`command_bytes`]. The shell answers with exactly
/// one [`file_written`] or [`file_not_written`].
pub const CMD_WRITE_FILE: i32 = 4;
/// [`poll`]: start the camera ([`Command::CameraOn`]).
pub const CMD_CAMERA_ON: i32 = 6;
/// [`poll`]: stop the camera ([`Command::CameraOff`]).
pub const CMD_CAMERA_OFF: i32 = 7;
/// [`poll`]: read the clipboard ([`Command::RequestClipboard`]); the
/// kind is in [`command_kind`]. The shell answers with exactly one
/// [`clipboard`] or [`clipboard_unavailable`].
pub const CMD_REQUEST_CLIPBOARD: i32 = 14;
/// [`poll`]: keep the received files' store
/// ([`CompanionCommand::KeepReceived`]); the bytes are in
/// [`command_bytes`]. The shell keeps them, replacing what it kept, and
/// hands them back with [`received`] next time.
pub const CMD_KEEP_RECEIVED: i32 = 16;
/// [`poll`]: keep the screen on or let it sleep
/// ([`CompanionCommand::KeepScreenOn`]); which is in [`command_on`].
pub const CMD_KEEP_SCREEN_ON: i32 = 17;
/// [`poll`]: keep the theme chosen ([`CompanionCommand::KeepTheme`]);
/// its id is in [`command_text`]. The shell hands it back with [`theme`]
/// next time, and the page's colour ([`page_color`]) has changed.
pub const CMD_KEEP_THEME: i32 = 18;

/// [`poll`]: ask the phone's unlock and open the kept store
/// ([`CompanionCommand::UnlockKept`]); the prompt's title is in
/// [`command_text`]. The shell answers with exactly one [`kept`] or
/// [`kept_closed`]. What the store is opened for stays in the core: the
/// shell asks the same way for each.
pub const CMD_UNLOCK_KEPT: i32 = 19;
/// [`poll`]: keep the kept store ([`CompanionCommand::KeepKept`]); the
/// bytes are in [`command_bytes`], the two counts in
/// [`command_descriptions`] and [`command_keys`]. Empty bytes: delete the
/// store, its wrapped key and the counts.
pub const CMD_KEEP_KEPT: i32 = 20;
/// [`poll`]: hand one file to the system's share sheet
/// ([`CompanionCommand::Share`]); its name, media type and bytes are in
/// [`command_name`], [`command_text`] and [`command_bytes`].
pub const CMD_SHARE: i32 = 21;
/// [`poll`]: keep the computer and the maker chosen on Start from the
/// stick ([`CompanionCommand::KeepStart`]); both are in [`command_text`],
/// the computer's id, a newline, the maker's page id (either may be
/// empty). The shell hands the text back with [`start`] next time.
pub const CMD_KEEP_START: i32 = 22;
/// [`poll`]: keep the steps ticked on one guide page
/// ([`CompanionCommand::KeepSteps`]); they are in [`command_text`], the
/// page's id, a newline, and the step numbers separated by commas (none
/// for a page with no ticks, whose kept text the shell drops). The shell
/// hands each page's text back with [`steps`] next time.
pub const CMD_KEEP_STEPS: i32 = 23;

/// [`kept_closed`]: the person cancelled the unlock ([`Closed::Cancelled`]).
pub const CLOSED_CANCELLED: i32 = 0;
/// [`kept_closed`]: the phone has no screen lock ([`Closed::NoScreenLock`]).
pub const CLOSED_NO_SCREEN_LOCK: i32 = 1;
/// [`kept_closed`]: the store could not be opened ([`Closed::Unreadable`]).
pub const CLOSED_UNREADABLE: i32 = 2;

/// Returned by [`poll`] for an unknown handle or a caught panic.
pub const CMD_ERROR: i32 = -1;

/// [`command_kind`]: a PSBT ([`FileKind::Psbt`]).
pub const FILE_PSBT: i32 = 0;
/// [`command_kind`]: any file ([`FileKind::Any`]).
pub const FILE_ANY: i32 = 1;
/// [`command_kind`]: text ([`FileKind::Text`]).
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

/// [`key`]: [`Key::Escape`], the system back gesture, which the core reads
/// as "go back". The only key the shell sends.
pub const KEY_ESCAPE: i32 = 3;

/// How many bytes [`system`]'s certificate digest carries.
pub const CERT_LEN: usize = 32;

/// Most instances a process may hold at once. One is the real number; the
/// cap exists so a shell bug cannot grow the table without bound.
const MAX_INSTANCES: usize = 8;

// ---------------------------------------------------------------------------
// Mappings, all total and all testable.
// ---------------------------------------------------------------------------

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

/// Key code → [`Key`]: the back gesture and nothing else. The Companion
/// takes no typed text.
#[must_use]
pub fn key_of(code: i32) -> Option<Key> {
    (code == KEY_ESCAPE).then_some(Key::Escape)
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

/// [`kept_closed`]'s code → [`Closed`].
#[must_use]
pub fn closed_of(code: i32) -> Option<Closed> {
    match code {
        CLOSED_CANCELLED => Some(Closed::Cancelled),
        CLOSED_NO_SCREEN_LOCK => Some(Closed::NoScreenLock),
        CLOSED_UNREADABLE => Some(Closed::Unreadable),
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
    on: bool,
    /// [`CMD_KEEP_KEPT`]'s two counts: descriptions, keys.
    counts: (u32, u32),
}

impl Payload {
    /// Wipes the bytes the shell did not take: a store or a file is
    /// wiped, not just freed.
    fn wipe(&mut self) {
        if let Some(b) = self.bytes.as_mut() {
            b.zeroize();
        }
    }
}

fn none() -> Payload {
    Payload {
        kind: FILE_NONE,
        ..Payload::default()
    }
}

/// Splits one of upstream's commands into its code and the payload the
/// accessors serve. `None` for a command the Companion never gives a
/// phone: it is dropped here, and what it carried is wiped.
fn encode(command: Command) -> Option<(i32, Payload)> {
    Some(match command {
        Command::Draw => (CMD_DRAW, none()),
        Command::Exit => (CMD_EXIT, none()),
        Command::CameraOn => (CMD_CAMERA_ON, none()),
        Command::CameraOff => (CMD_CAMERA_OFF, none()),
        // The shell has a picker of its own, so it is asked for a file
        // and never for a listing.
        Command::RequestFile { kind } | Command::ReadFile { kind, .. } => (
            CMD_REQUEST_FILE,
            Payload {
                kind: file_kind_code(kind),
                ..none()
            },
        ),
        Command::RequestClipboard { kind } => (
            CMD_REQUEST_CLIPBOARD,
            Payload {
                kind: file_kind_code(kind),
                ..none()
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
                ..none()
            },
        ),
        // Never handed to the shell: the Companion writes no clipboard
        // (docs/COMPANION.md §6.3), keeps no secret and asks for no
        // entropy, haptics or settings.
        Command::WriteClipboard { mut text, .. } => {
            text.zeroize();
            return None;
        }
        Command::StoreSecret { mut blob } => {
            blob.zeroize();
            return None;
        }
        Command::StoreSettings { mut bytes } => {
            bytes.zeroize();
            return None;
        }
        Command::SecureMac { .. }
        | Command::Vibrate { .. }
        | Command::RequestEntropy
        | Command::LoadSecret
        | Command::ForgetSecret => return None,
    })
}

/// Splits one of the Companion's own commands into its code and payload;
/// `None` for one this shell does not take yet.
fn encode_companion(command: CompanionCommand) -> Option<(i32, Payload)> {
    Some(match command {
        CompanionCommand::KeepReceived { bytes } => (
            CMD_KEEP_RECEIVED,
            Payload {
                bytes: Some(bytes),
                ..none()
            },
        ),
        CompanionCommand::KeepScreenOn { on } => (CMD_KEEP_SCREEN_ON, Payload { on, ..none() }),
        CompanionCommand::KeepTheme { id } => (
            CMD_KEEP_THEME,
            Payload {
                text: Some(id),
                ..none()
            },
        ),
        CompanionCommand::KeepStart { computer, maker } => (
            CMD_KEEP_START,
            Payload {
                text: Some(format!("{computer}\n{maker}")),
                ..none()
            },
        ),
        CompanionCommand::KeepSteps { page, done } => (
            CMD_KEEP_STEPS,
            Payload {
                text: Some(format!(
                    "{page}\n{}",
                    done.iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                )),
                ..none()
            },
        ),
        CompanionCommand::UnlockKept { purpose: _, title } => (
            CMD_UNLOCK_KEPT,
            Payload {
                text: Some(title),
                ..none()
            },
        ),
        CompanionCommand::KeepKept {
            bytes,
            descriptions,
            keys,
        } => (
            CMD_KEEP_KEPT,
            Payload {
                bytes: Some(bytes),
                counts: (descriptions, keys),
                ..none()
            },
        ),
        CompanionCommand::Share { name, mime, bytes } => (
            CMD_SHARE,
            Payload {
                bytes: Some(bytes),
                name: Some(name),
                text: Some(mime),
                ..none()
            },
        ),
    })
}

// ---------------------------------------------------------------------------
// The handle table.
// ---------------------------------------------------------------------------

/// One live Companion plus the payload of its last polled command.
struct Instance {
    app: Companion,
    payload: Payload,
    /// Reads the codes in the camera's frames on a worker thread. Alive
    /// from the first frame until the camera is turned off.
    scanner: Option<Scanner>,
}

impl Instance {
    /// Parks `payload` for the accessors, wiping the one it replaces.
    fn park(&mut self, payload: Payload) {
        self.payload.wipe();
        self.payload = payload;
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.payload.wipe();
    }
}

/// A table entry. `generation` counts how many instances the slot has held,
/// so a handle from an earlier tenant never matches the current one.
struct Slot {
    generation: u32,
    instance: Option<Box<Instance>>,
}

thread_local! {
    /// The live Companions of this thread.
    static TABLE: RefCell<Vec<Slot>> = const { RefCell::new(Vec::new()) };
}

/// Generations use 31 bits, so that every handle is a positive `i64`.
const GENERATION_BITS: u32 = 0x7fff_ffff;

/// The generation a slot takes when it is reused. Never `0`.
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
/// nothing.
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
// The surface itself. Every function is total: a bad handle does nothing.
// ---------------------------------------------------------------------------

/// Creates a Companion for a `width × height` display at `dpi`, sends it
/// [`Event::Display`], and returns its handle. `0` means nothing was
/// created: a size that is not a positive `u16`, or a full table.
///
/// `inset_bottom` and `inset_top` are the strips of the frame at each
/// edge the person cannot use, in pixels; a shell that already padded its
/// view by the system bars passes zero.
#[must_use]
pub fn create(width: i32, height: i32, dpi: i32, inset_bottom: i32, inset_top: i32) -> i64 {
    let (Ok(width), Ok(height), Ok(dpi), Ok(inset_bottom), Ok(inset_top)) = (
        u16::try_from(width),
        u16::try_from(height),
        u16::try_from(dpi),
        u16::try_from(inset_bottom),
        u16::try_from(inset_top),
    ) else {
        return 0;
    };
    if width == 0 || height == 0 || dpi == 0 {
        return 0;
    }
    let mut app = Companion::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom,
        inset_top,
        // Touch only.
        buttons: 0,
        // The shell turns every frame upright.
        camera_fixed: true,
        // The Companion keeps no key; what backs the Keystore is the
        // shell's business, not the core's.
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
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

/// Drops the Companion a handle names, zeroizing what it holds. Freeing an
/// already-freed or unknown handle does nothing.
pub fn destroy(handle: i64) {
    let Some((index, generation)) = decode_handle(handle) else {
        return;
    };
    // Taken under the borrow and dropped outside it.
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

/// Delivers one of upstream's events.
fn send(handle: i64, event: Event) {
    with(handle, |i| i.app.event(event));
}

/// Delivers one of the Companion's own events.
fn send_companion(handle: i64, event: CompanionEvent) {
    with(handle, |i| i.app.companion_event(event));
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

/// Monotonic time in milliseconds. A negative value is ignored. What the
/// scanner worker read and saw since the last tick goes in first.
pub fn tick(handle: i64, now_ms: i64) {
    let Ok(now_ms) = u64::try_from(now_ms) else {
        return;
    };
    with(handle, |i| {
        while let Some(bytes) = i.scanner.as_ref().and_then(Scanner::poll) {
            i.app.event(Event::Scanned { bytes });
        }
        if let Some(seen) = i.scanner.as_ref().and_then(Scanner::poll_seen) {
            i.app.companion_event(CompanionEvent::Seen {
                width: seen.width,
                height: seen.height,
                corners: seen.corners,
                read: seen.read,
            });
        }
        i.app.event(Event::Tick { now_ms });
    });
}

/// A key press: [`KEY_ESCAPE`] for the back gesture, and nothing else.
pub fn key(handle: i64, code: i32) {
    if let Some(key) = key_of(code) {
        send(handle, Event::Key(key));
    }
}

/// The answer to [`CMD_REQUEST_FILE`]: the file's bytes, unmodified, and
/// the name its provider gave it where the shell could read one.
pub fn file(handle: i64, kind: i32, name: Option<String>, bytes: Vec<u8>) {
    let Some(kind) = file_kind_of(kind) else {
        let mut bytes = bytes;
        bytes.zeroize();
        return;
    };
    match name {
        Some(name) => send_companion(handle, CompanionEvent::Picked { name, bytes }),
        None => send(handle, Event::File { kind, bytes }),
    }
}

/// The answer to [`CMD_REQUEST_FILE`] when no file came: no picker, or a
/// read that failed.
pub fn file_unavailable(handle: i64, kind: i32) {
    if let Some(kind) = file_kind_of(kind) {
        send(handle, Event::FileUnavailable { kind });
    }
}

/// The answer to [`CMD_REQUEST_FILE`] when the person closed the picker
/// without choosing.
pub fn file_cancelled(handle: i64, kind: i32) {
    if let Some(kind) = file_kind_of(kind) {
        send(handle, Event::FileCancelled { kind });
    }
}

/// The answer to [`CMD_WRITE_FILE`]: the bytes are stored.
pub fn file_written(handle: i64, kind: i32) {
    if let Some(kind) = file_kind_of(kind) {
        send(handle, Event::FileWritten { kind });
    }
}

/// The answer to [`CMD_WRITE_FILE`] when nothing was stored.
pub fn file_not_written(handle: i64, kind: i32) {
    if let Some(kind) = file_kind_of(kind) {
        send(handle, Event::FileNotWritten { kind });
    }
}

/// The answer to [`CMD_REQUEST_CLIPBOARD`]: what the clipboard holds.
pub fn clipboard(handle: i64, kind: i32, text: String) {
    let Some(kind) = file_kind_of(kind) else {
        let mut text = text;
        text.zeroize();
        return;
    };
    send(handle, Event::Clipboard { kind, text });
}

/// The answer to [`CMD_REQUEST_CLIPBOARD`] when no text comes.
pub fn clipboard_unavailable(handle: i64, kind: i32) {
    if let Some(kind) = file_kind_of(kind) {
        send(handle, Event::ClipboardUnavailable { kind });
    }
}

/// One camera frame: 8-bit luma, `width × height` bytes, with the NV12
/// chroma plane beside it where the camera gave colour. To the scanner
/// worker for its codes, and to the core for the preview.
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

/// The wall-clock time, at start and whenever the app comes back to the
/// foreground: seconds since 1970 UTC and the phone's offset from UTC.
/// A time before 1970 is ignored.
pub fn time(handle: i64, unix_s: i64, offset_s: i32) {
    if let Ok(unix_s) = u64::try_from(unix_s) {
        send_companion(handle, CompanionEvent::Time { unix_s, offset_s });
    }
}

/// The received files' store the shell kept, as the core handed it over
/// in [`CMD_KEEP_RECEIVED`]. Sent once at start, by a shell with one.
pub fn received(handle: i64, bytes: Vec<u8>) {
    send_companion(handle, CompanionEvent::Received { bytes });
}

/// The theme the shell kept from [`CMD_KEEP_THEME`]. Sent once at start.
pub fn theme(handle: i64, id: String) {
    send_companion(handle, CompanionEvent::Theme { id });
}

/// The answers the shell kept from [`CMD_KEEP_START`], as that command
/// gave them. Sent once at start.
pub fn start(handle: i64, text: String) {
    let (computer, maker) = text.split_once('\n').unwrap_or((text.as_str(), ""));
    send_companion(
        handle,
        CompanionEvent::Start {
            computer: computer.to_string(),
            maker: maker.to_string(),
        },
    );
}

/// One guide page's ticked steps the shell kept from [`CMD_KEEP_STEPS`],
/// as that command gave them. Sent once at start for each page kept.
/// Anything that is not a step number is ignored.
pub fn steps(handle: i64, text: String) {
    let (page, done) = text.split_once('\n').unwrap_or((text.as_str(), ""));
    let done = done
        .split(',')
        .filter_map(|n| n.trim().parse().ok())
        .collect();
    send_companion(
        handle,
        CompanionEvent::Steps {
            page: page.trim().to_string(),
            done,
        },
    );
}

/// What About states from the system, sent once at start: the version
/// name and code, the signing certificate's SHA-256 ([`CERT_LEN`]
/// bytes; anything else is dropped and About says it was not reported),
/// and the permissions the manifest asks for, one a line, each a name, a
/// tab, and `1` when the system has granted it or `0` when not.
pub fn system(handle: i64, build: String, certificate: &[u8], permissions: &str) {
    let Ok(certificate_sha256) = <[u8; CERT_LEN]>::try_from(certificate) else {
        return;
    };
    let permissions = permissions
        .lines()
        .filter_map(|l| {
            let (name, granted) = l.split_once('\t')?;
            let name = name.trim();
            (!name.is_empty()).then(|| (name.to_string(), granted.trim() == "1"))
        })
        .collect();
    send_companion(
        handle,
        CompanionEvent::System {
            build,
            certificate_sha256,
            permissions,
        },
    );
}

/// The answer to [`CMD_UNLOCK_KEPT`]: the person gave the phone's unlock,
/// and these are the bytes the last [`CMD_KEEP_KEPT`] handed over,
/// decrypted; empty when nothing is kept yet.
pub fn kept(handle: i64, bytes: Vec<u8>) {
    send_companion(handle, CompanionEvent::Kept { bytes });
}

/// The answer to [`CMD_UNLOCK_KEPT`] when the store was not opened, and
/// why: one of the `CLOSED_*` codes. An unknown code is ignored.
pub fn kept_closed(handle: i64, why: i32) {
    if let Some(why) = closed_of(why) {
        send_companion(handle, CompanionEvent::KeptClosed { why });
    }
}

/// The app went to the background: the core wipes what it held of the
/// kept store. Not sent while the system's own unlock prompt is what
/// stopped the activity, or the unlock it answers would be dropped.
pub fn locked(handle: i64) {
    send_companion(handle, CompanionEvent::Locked);
}

/// The two counts the shell keeps beside the kept store, sent once at
/// start. A negative count is ignored.
pub fn kept_counts(handle: i64, descriptions: i32, keys: i32) {
    if let (Ok(descriptions), Ok(keys)) = (u32::try_from(descriptions), u32::try_from(keys)) {
        send_companion(handle, CompanionEvent::KeptCounts { descriptions, keys });
    }
}

/// One file (with the name the sharing app gave it, if any) or one text
/// (no name, its UTF-8) shared to the Companion.
pub fn shared_in(handle: i64, name: Option<String>, bytes: Vec<u8>) {
    send_companion(handle, CompanionEvent::SharedIn { name, bytes });
}

/// Takes the next pending command as a `CMD_*` code, parking its payload
/// for the accessors below: upstream's commands first, then the
/// Companion's own. [`CMD_NONE`] when both queues are empty, [`CMD_ERROR`]
/// for an unknown handle.
#[must_use]
pub fn poll(handle: i64) -> i32 {
    with(handle, |i| {
        while let Some(command) = i.app.poll_command() {
            // The camera is off: the worker ends here, and the frame and
            // any payload it still held are wiped with it.
            if command == Command::CameraOff {
                i.scanner = None;
            }
            if let Some((code, payload)) = encode(command) {
                i.park(payload);
                return code;
            }
        }
        while let Some(command) = i.app.poll_companion() {
            if let Some((code, payload)) = encode_companion(command) {
                i.park(payload);
                return code;
            }
        }
        i.park(Payload::default());
        CMD_NONE
    })
    .unwrap_or(CMD_ERROR)
}

/// The file kind of the last polled command, or [`FILE_NONE`].
#[must_use]
pub fn command_kind(handle: i64) -> i32 {
    with(handle, |i| i.payload.kind).unwrap_or(FILE_NONE)
}

/// The bytes of the last polled command, taken: a second call answers
/// `None`.
#[must_use]
pub fn command_bytes(handle: i64) -> Option<Vec<u8>> {
    with(handle, |i| i.payload.bytes.take()).flatten()
}

/// The name hint of the last polled command, taken.
#[must_use]
pub fn command_name(handle: i64) -> Option<String> {
    with(handle, |i| i.payload.name.take()).flatten()
}

/// The text of the last polled command ([`CMD_KEEP_THEME`]), taken.
#[must_use]
pub fn command_text(handle: i64) -> Option<String> {
    with(handle, |i| i.payload.text.take()).flatten()
}

/// Whether the last polled command says on ([`CMD_KEEP_SCREEN_ON`]).
#[must_use]
pub fn command_on(handle: i64) -> bool {
    with(handle, |i| i.payload.on).unwrap_or(false)
}

/// [`CMD_KEEP_KEPT`]'s count of wallet descriptions; `0` after any other
/// command.
#[must_use]
pub fn command_descriptions(handle: i64) -> i32 {
    with(handle, |i| clamp(i.payload.counts.0)).unwrap_or(0)
}

/// [`CMD_KEEP_KEPT`]'s count of BitLocker keys; `0` after any other
/// command.
#[must_use]
pub fn command_keys(handle: i64) -> i32 {
    with(handle, |i| clamp(i.payload.counts.1)).unwrap_or(0)
}

/// A count as the shell's `Int`.
fn clamp(n: u32) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// The colour of the page in the theme on show, as Android's opaque
/// ARGB: what the window behind the frame and the system bars are
/// painted, so the strips the frame does not cover match it. `0` for an
/// unknown handle.
#[must_use]
pub fn page_color(handle: i64) -> i32 {
    with(handle, |i| {
        let c = i.app.theme.color(faraday_ui::pal::BG);
        (0xff00_0000u32 | u32::from(c.r) << 16 | u32::from(c.g) << 8 | u32::from(c.b)) as i32
    })
    .unwrap_or(0)
}

/// Whether the theme on show is dark text on a light page, so the
/// system bars' icons are drawn dark.
#[must_use]
pub fn page_light(handle: i64) -> bool {
    with(handle, |i| i.app.theme.is_light()).unwrap_or(false)
}

/// The core-owned framebuffer as a pointer and a length in bytes:
/// premultiplied RGBA8888, row-major, `width × height × 4`.
///
/// **Lifetime.** The memory belongs to the core. It stays valid until the
/// next call that can touch this instance (any event, any poll, or
/// [`destroy`]), so a shell reads it before it calls anything else and
/// never keeps the pointer across a frame.
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
    use faraday_companion::{Act, Page, Theme};

    /// A phone's display, as the shell creates the core.
    fn phone() -> i64 {
        let handle = create(1080, 2340, 420, 0, 0);
        assert_ne!(handle, 0);
        drain(handle);
        handle
    }

    /// Every command code until the queues are empty.
    fn drain(handle: i64) -> Vec<i32> {
        let mut codes = Vec::new();
        loop {
            match poll(handle) {
                CMD_NONE | CMD_ERROR => return codes,
                code => codes.push(code),
            }
        }
    }

    fn press(handle: i64, act: Act) {
        with(handle, |i| i.app.press(act));
    }

    fn page(handle: i64) -> Option<Page> {
        with(handle, |i| i.app.page.clone())
    }

    #[test]
    fn handles_round_trip() {
        for index in [0usize, 1, 7, 4095] {
            for generation in [1u32, 2, GENERATION_BITS] {
                let handle = encode_handle(index, generation);
                assert!(handle > 0);
                assert_eq!(decode_handle(handle), Some((index, generation)));
            }
        }
        assert_eq!(next_generation(GENERATION_BITS), 1);
        assert_eq!(decode_handle(0), None);
        assert_eq!(decode_handle(-1), None);
    }

    #[test]
    fn a_freed_handle_is_dead_and_a_reused_slot_does_not_answer_it() {
        let first = phone();
        assert!(frame_ptr(first).is_some());
        destroy(first);
        assert_eq!(poll(first), CMD_ERROR);
        assert_eq!(frame_ptr(first), None);
        touch(first, 1, 1, TOUCH_DOWN);
        tick(first, 1);
        destroy(first);
        let second = phone();
        assert_ne!(first, second);
        assert_eq!(poll(first), CMD_ERROR);
        assert_ne!(poll(second), CMD_ERROR);
        destroy(second);
    }

    #[test]
    fn a_bad_size_creates_nothing() {
        assert_eq!(create(0, 2340, 420, 0, 0), 0);
        assert_eq!(create(1080, 2340, 0, 0, 0), 0);
        assert_eq!(create(70_000, 2340, 420, 0, 0), 0);
        assert_eq!(create(1080, -1, 420, 0, 0), 0);
    }

    #[test]
    fn the_display_gives_a_whole_frame_and_a_draw() {
        let handle = create(1080, 2340, 420, 0, 0);
        let (ptr, len) = frame_ptr(handle).expect("a frame after Display");
        assert!(!ptr.is_null());
        assert_eq!(len, 1080 * 2340 * 4);
        assert!(drain(handle).contains(&CMD_DRAW));
        destroy(handle);
    }

    #[test]
    fn the_clipboard_is_never_written_and_no_secret_reaches_the_shell() {
        for command in [
            Command::WriteClipboard {
                kind: FileKind::Text,
                text: String::from("bc1q"),
            },
            Command::StoreSecret { blob: vec![1; 8] },
            Command::StoreSettings { bytes: vec![1; 8] },
            Command::SecureMac { salt: [0; 32] },
            Command::RequestEntropy,
            Command::LoadSecret,
            Command::ForgetSecret,
            Command::Vibrate { ms: 10 },
        ] {
            assert!(encode(command).is_none());
        }
    }

    #[test]
    fn a_write_hands_the_shell_its_kind_name_and_bytes_once() {
        let handle = phone();
        with(handle, |i| {
            let (_, payload) = encode(Command::WriteFile {
                kind: FileKind::Psbt,
                name_hint: String::from("spend.psbt"),
                bytes: vec![7; 4],
            })
            .expect("a write reaches the shell");
            i.park(payload);
        });
        assert_eq!(command_kind(handle), FILE_PSBT);
        assert_eq!(command_name(handle).as_deref(), Some("spend.psbt"));
        assert_eq!(command_bytes(handle).as_deref(), Some(&[7u8; 4][..]));
        assert_eq!(command_bytes(handle), None, "bytes are handed over once");
        destroy(handle);
    }

    #[test]
    fn paste_asks_the_shell_for_the_clipboard_and_its_text_arrives() {
        let handle = phone();
        press(handle, Act::Receive);
        let codes = drain(handle);
        assert!(codes.contains(&CMD_CAMERA_ON));
        press(handle, Act::Paste);
        assert_eq!(poll(handle), CMD_REQUEST_CLIPBOARD);
        assert_eq!(command_kind(handle), FILE_TEXT);
        drain(handle);
        clipboard(handle, FILE_TEXT, String::from("Meet at noon."));
        assert!(matches!(page(handle), Some(Page::Shown(_))));
        // The scanner is left, so the camera goes off.
        assert!(drain(handle).contains(&CMD_CAMERA_OFF));
        destroy(handle);
    }

    #[test]
    fn a_picked_file_goes_as_codes_under_the_name_the_shell_read() {
        let handle = phone();
        press(handle, Act::Send);
        drain(handle);
        press(handle, Act::Choose);
        assert_eq!(poll(handle), CMD_REQUEST_FILE);
        assert_eq!(command_kind(handle), FILE_ANY);
        drain(handle);
        file(
            handle,
            FILE_ANY,
            Some(String::from("notes.txt")),
            b"Meet at noon.\n".to_vec(),
        );
        assert_eq!(page(handle), Some(Page::Send));
        let name = with(handle, |i| i.app.sending.as_ref().map(|s| s.name.clone()));
        assert_eq!(name.flatten().as_deref(), Some("notes.txt"));
        // The screen stays on while the codes show.
        let mut on = None;
        loop {
            match poll(handle) {
                CMD_NONE => break,
                CMD_KEEP_SCREEN_ON => on = Some(command_on(handle)),
                _ => {}
            }
        }
        assert_eq!(on, Some(true));
        destroy(handle);
    }

    #[test]
    fn a_kept_text_hands_the_shell_the_store_and_the_store_comes_back() {
        let handle = phone();
        time(handle, 1_791_200_000, 3600);
        press(handle, Act::Receive);
        press(handle, Act::Paste);
        drain(handle);
        clipboard(handle, FILE_TEXT, String::from("Meet at noon."));
        press(handle, Act::Keep);
        let mut store = None;
        loop {
            match poll(handle) {
                CMD_NONE => break,
                CMD_KEEP_RECEIVED => store = command_bytes(handle),
                _ => {}
            }
        }
        let store = store.expect("the store to keep");
        destroy(handle);

        let again = phone();
        received(again, store);
        let count = with(again, |i| i.app.received().len());
        assert_eq!(count, Some(1));
        destroy(again);
    }

    #[test]
    fn the_theme_chosen_is_kept_and_the_page_colour_follows_it() {
        let handle = phone();
        let before = page_color(handle);
        let at = Theme::ALL
            .iter()
            .position(|t| *t == Theme::Dark)
            .expect("Dark is offered");
        press(handle, Act::About);
        press(handle, Act::Theme(at));
        let mut kept = None;
        loop {
            match poll(handle) {
                CMD_NONE => break,
                CMD_KEEP_THEME => kept = command_text(handle),
                _ => {}
            }
        }
        assert_eq!(kept.as_deref(), Some("dark"));
        assert_ne!(page_color(handle), before);
        assert!(!page_light(handle));
        destroy(handle);

        let again = phone();
        theme(again, String::from("dark"));
        assert_eq!(page_color(again), page_color_of(Theme::Dark));
        destroy(again);
    }

    #[test]
    fn the_start_answers_are_kept_and_come_back() {
        let handle = phone();
        press(handle, Act::Start);
        press(handle, Act::Computer(0));
        let mut kept = None;
        loop {
            match poll(handle) {
                CMD_NONE => break,
                CMD_KEEP_START => kept = command_text(handle),
                _ => {}
            }
        }
        let kept = kept.expect("the shell is asked to keep the answer");
        assert_eq!(kept, "windows\n");
        destroy(handle);

        let again = phone();
        start(again, kept);
        assert_eq!(
            with(again, |i| i.app.computer),
            Some(Some(faraday_companion::Computer::Windows))
        );
        destroy(again);
    }

    #[test]
    fn the_ticked_steps_are_kept_and_come_back() {
        let handle = phone();
        let at = |handle| {
            with(handle, |i| {
                let Page::Doc(faraday_companion::Doc::Own(id)) = i.app.page else {
                    return Vec::new();
                };
                i.app.ticked(id).to_vec()
            })
            .unwrap_or_default()
        };
        press(handle, Act::Start);
        press(handle, Act::Computer(0));
        // The Windows guide is the first page Start from the stick lists.
        press(handle, Act::StartPage(0));
        press(handle, Act::Step(2));
        press(handle, Act::Step(1));
        assert_eq!(at(handle), vec![1, 2]);
        let mut kept = None;
        loop {
            match poll(handle) {
                CMD_NONE => break,
                CMD_KEEP_STEPS => kept = command_text(handle),
                _ => {}
            }
        }
        let kept = kept.expect("the shell is asked to keep the ticks");
        assert_eq!(kept, "windows\n1,2");
        destroy(handle);

        let again = phone();
        steps(again, kept);
        press(again, Act::Start);
        press(again, Act::Computer(0));
        press(again, Act::StartPage(0));
        assert_eq!(at(again), vec![1, 2]);
        destroy(again);
    }

    fn page_color_of(t: Theme) -> i32 {
        let c = t.color(faraday_ui::pal::BG);
        (0xff00_0000u32 | u32::from(c.r) << 16 | u32::from(c.g) << 8 | u32::from(c.b)) as i32
    }

    #[test]
    fn about_shows_the_certificate_and_permissions_the_shell_reported() {
        let handle = phone();
        system(
            handle,
            String::from("0.1.1 (1)"),
            &[0xab; CERT_LEN],
            "android.permission.CAMERA\t0\nandroid.permission.USE_BIOMETRIC\t1\n",
        );
        let s = with(handle, |i| i.app.system.clone()).flatten();
        let s = s.expect("About's facts");
        assert_eq!(s.certificate_sha256, [0xab; CERT_LEN]);
        assert_eq!(
            s.permissions,
            vec![
                (String::from("android.permission.CAMERA"), false),
                (String::from("android.permission.USE_BIOMETRIC"), true)
            ]
        );
        // A digest of the wrong length is not a certificate.
        let other = phone();
        system(other, String::from("0.1.1 (1)"), &[0xab; 20], "");
        assert_eq!(with(other, |i| i.app.system.is_none()), Some(true));
        destroy(handle);
        destroy(other);
    }

    #[test]
    fn a_camera_frame_whose_length_disagrees_is_dropped() {
        let handle = phone();
        press(handle, Act::Receive);
        drain(handle);
        camera_frame(handle, 4, 4, vec![0; 15], None);
        assert!(with(handle, |i| i.scanner.is_none()).unwrap_or(false));
        camera_frame(handle, 4, 4, vec![0; 16], Some(vec![128; 7]));
        assert!(with(handle, |i| i.scanner.is_some()).unwrap_or(false));
        // Leaving the scanner ends the worker.
        press(handle, Act::Back);
        drain(handle);
        assert!(with(handle, |i| i.scanner.is_none()).unwrap_or(false));
        destroy(handle);
    }

    #[test]
    fn only_the_back_key_is_a_key() {
        assert_eq!(key_of(KEY_ESCAPE), Some(Key::Escape));
        assert_eq!(key_of(0), None);
        assert_eq!(phase_of(TOUCH_UP), Some(TouchPhase::Up));
        assert_eq!(phase_of(3), None);
        assert_eq!(file_kind_of(FILE_NONE), None);
        assert_eq!(file_kind_code(FileKind::Any), FILE_ANY);
        assert_eq!(closed_of(CLOSED_UNREADABLE), Some(Closed::Unreadable));
        assert_eq!(closed_of(-1), None);
    }

    /// A wallet description another wallet wrote (`docs/INTEROP.md`).
    const DESCRIPTION: &str =
        include_str!("../../faraday-core/tests/vectors/interop/sparrow/descriptor-multipath.txt");

    /// One command as the shell reads it.
    struct Read {
        code: i32,
        bytes: Option<Vec<u8>>,
        name: Option<String>,
        text: Option<String>,
        counts: (i32, i32),
    }

    /// Every command until the queues are empty, with what each carried
    /// read as the shell reads it.
    fn drain_read(handle: i64) -> Vec<Read> {
        let mut out = Vec::new();
        loop {
            match poll(handle) {
                CMD_NONE | CMD_ERROR => return out,
                code => out.push(Read {
                    code,
                    bytes: command_bytes(handle),
                    name: command_name(handle),
                    text: command_text(handle),
                    counts: (command_descriptions(handle), command_keys(handle)),
                }),
            }
        }
    }

    /// The phone on Wallet descriptions, opened after the unlock with
    /// `store` (empty the first time).
    fn on_descriptions(store: Vec<u8>) -> i64 {
        let handle = phone();
        press(handle, Act::Descriptions);
        let asked = drain_read(handle);
        let title = asked
            .iter()
            .find(|c| c.code == CMD_UNLOCK_KEPT)
            .and_then(|c| c.text.clone());
        assert!(title.is_some_and(|t| !t.is_empty()), "the unlock's title");
        assert!(matches!(page(handle), Some(Page::Unlocking(_))));
        kept(handle, store);
        assert_eq!(page(handle), Some(Page::Descriptions));
        drain(handle);
        handle
    }

    #[test]
    fn a_description_kept_after_the_unlock_reaches_the_shell_and_opens_again() {
        let handle = on_descriptions(Vec::new());
        press(handle, Act::AddDescription);
        drain(handle);
        assert_eq!(poll(handle), CMD_NONE);
        press(handle, Act::Paste);
        drain(handle);
        clipboard(handle, FILE_TEXT, DESCRIPTION.to_string());
        let written = drain_read(handle)
            .into_iter()
            .rfind(|c| c.code == CMD_KEEP_KEPT)
            .expect("the kept store handed over");
        let store = written.bytes.expect("its bytes");
        assert!(!store.is_empty());
        assert_eq!(written.counts, (1, 0));
        destroy(handle);

        // The next start: the counts while locked, the store after the
        // unlock.
        let again = phone();
        kept_counts(again, 1, 0);
        assert_eq!(with(again, |i| i.app.kept().is_none()), Some(true));
        destroy(again);
        let again = on_descriptions(store);
        let count = with(again, |i| i.app.kept().map(|k| k.counts()));
        assert_eq!(count.flatten(), Some((1, 0)));
        destroy(again);
    }

    #[test]
    fn going_to_the_background_leaves_nothing_of_the_store_and_asks_again() {
        let handle = on_descriptions(Vec::new());
        locked(handle);
        assert_eq!(page(handle), Some(Page::Home));
        assert_eq!(with(handle, |i| i.app.kept().is_none()), Some(true));
        press(handle, Act::Descriptions);
        assert!(drain(handle).contains(&CMD_UNLOCK_KEPT));
        assert!(matches!(page(handle), Some(Page::Unlocking(_))));
        destroy(handle);
    }

    #[test]
    fn each_reason_the_store_stayed_closed_reaches_the_core() {
        for (code, why) in [
            (CLOSED_CANCELLED, Closed::Cancelled),
            (CLOSED_NO_SCREEN_LOCK, Closed::NoScreenLock),
            (CLOSED_UNREADABLE, Closed::Unreadable),
        ] {
            let handle = phone();
            press(handle, Act::Keys);
            drain(handle);
            // A code the core does not know leaves the question open.
            kept_closed(handle, 9);
            assert!(matches!(page(handle), Some(Page::Unlocking(_))));
            kept_closed(handle, code);
            assert_eq!(page(handle), Some(Page::Closed(why)));
            let codes = drain(handle);
            assert!(!codes.contains(&CMD_KEEP_KEPT));
            destroy(handle);
        }
    }

    #[test]
    fn a_shared_description_asks_for_the_unlock_and_shared_text_is_shown() {
        let handle = phone();
        shared_in(
            handle,
            Some(String::from("vault.txt")),
            DESCRIPTION.as_bytes().to_vec(),
        );
        assert!(drain(handle).contains(&CMD_UNLOCK_KEPT));
        destroy(handle);

        let handle = phone();
        shared_in(handle, None, b"Meet at noon.".to_vec());
        assert!(matches!(page(handle), Some(Page::Shown(_))));
        assert!(!drain(handle).contains(&CMD_UNLOCK_KEPT));
        destroy(handle);
    }

    #[test]
    fn share_hands_the_shell_one_file_with_its_name_and_type() {
        let handle = on_descriptions(Vec::new());
        press(handle, Act::AddDescription);
        press(handle, Act::Paste);
        drain(handle);
        clipboard(handle, FILE_TEXT, DESCRIPTION.to_string());
        drain(handle);
        press(handle, Act::Share);
        let share = drain_read(handle)
            .into_iter()
            .find(|c| c.code == CMD_SHARE)
            .expect("a share");
        // The description as it arrived, less the line's end.
        assert_eq!(share.bytes.as_deref(), Some(DESCRIPTION.trim().as_bytes()));
        assert!(share.name.is_some_and(|n| !n.is_empty()));
        assert_eq!(share.text.as_deref(), Some("text/plain"));
        destroy(handle);
    }

    #[test]
    fn deleting_a_store_it_cannot_read_tells_the_shell_with_empty_bytes() {
        let handle = phone();
        press(handle, Act::Descriptions);
        drain(handle);
        kept(handle, b"FCKS\x09".to_vec());
        assert_eq!(page(handle), Some(Page::Closed(Closed::Unreadable)));
        press(handle, Act::DeleteKeptStore);
        press(handle, Act::ConfirmDelete);
        let written = drain_read(handle)
            .into_iter()
            .rfind(|c| c.code == CMD_KEEP_KEPT)
            .expect("the store's deletion");
        assert_eq!(written.bytes.as_deref(), Some(&[][..]));
        assert_eq!(written.counts, (0, 0));
        destroy(handle);
    }

    #[test]
    fn a_caught_panic_returns_the_fallback() {
        assert_eq!(guard(CMD_ERROR, || panic!("boom")), CMD_ERROR);
        assert_eq!(guard(CMD_ERROR, || CMD_DRAW), CMD_DRAW);
    }
}
