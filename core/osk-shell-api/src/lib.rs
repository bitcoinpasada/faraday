//! The core ↔ shell contract for OpenSignerKit (`docs/PLANNING.md` §4.2,
//! amended by §16.3 and §16.4).
//!
//! A shell is deliberately dumb. It owns a display and some input devices,
//! and it talks to the core through exactly three calls on [`App`]:
//!
//! 1. [`App::event`] delivers one [`Event`] (touch, button, key, tick, …).
//! 2. [`App::poll_command`] drains the [`Command`]s the core wants done.
//! 3. [`App::frame`] paints the core-owned framebuffer and exposes it for
//!    blitting.
//!
//! Protocol:
//!
//! - The shell sends [`Event::Display`] **first**, exactly once, before any
//!   other event. The core allocates its framebuffer from it. The size is
//!   fixed for the session; there is no live resizing.
//! - After every event the shell calls [`App::poll_command`] until it
//!   returns `None`, and acts on each command in order.
//! - The frame is painted when it is read, from the layout the last event
//!   solved, not when the event lands (`docs/PLANNING.md` §16.119 rule 5).
//!   A shell that reads the frame once per frame it draws paints once per
//!   drawn frame rather than once per event.
//! - [`Command::Draw`] means "read [`App::frame`] and blit it now". The
//!   frame is premultiplied RGBA8888, row-major, four bytes per pixel, exactly
//!   `width × height × 4` bytes. A shell whose display wants another format
//!   (RGB565 on the Pi framebuffer, BGRA on some desktops) converts on
//!   blit; the core never changes format.
//! - The shell never interprets frame contents and never sees a seed, key
//!   or mnemonic word. It sees pixels, files the user chose, and, in later
//!   milestones, opaque storage blobs.
//!
//! File channel (`docs/PLANNING.md` §7, §16.16): the core asks with
//! [`Command::RequestFile`] and the shell answers with exactly one
//! [`Event::File`], [`Event::FileUnavailable`], [`Event::FileCancelled`]
//! or [`Event::FileList`] per
//! request, in order. A shell with a picker of its own (a phone's
//! document picker) or with nothing to list answers with the file; a
//! shell that can list what it holds (a card, a directory) answers with
//! [`Event::FileList`], the core shows the list, and the name the person
//! taps comes back as [`Command::ReadFile`], which the shell answers
//! the same way.
//! The two ways of not answering with a file are different facts about
//! the device and the core says different things about them:
//! [`Event::FileUnavailable`] is "there is no file channel here", which
//! the screen states; [`Event::FileCancelled`] is "the person closed the
//! picker", which it does not, because they know.
//! The bytes are the raw file content; the core parses them (a PSBT may be
//! binary or base64 text). [`Command::WriteFile`] hands the shell bytes to
//! store under a name of its choosing, guided by `name_hint`, and answers
//! it with exactly one [`Event::FileWritten`] or
//! [`Event::FileNotWritten`], so that a card that is full, a partition
//! that is not mounted or a picker the user cancelled reaches the screen
//! as a save that did not happen. The shell never reads a file the core
//! did not ask for and never writes one the user did not trigger (§5.3).
//!
//! Clipboard channel (`docs/PLANNING.md` §16.88): the core asks with
//! [`Command::RequestClipboard`] and the shell answers with exactly one
//! [`Event::Clipboard`] or [`Event::ClipboardUnavailable`] per request,
//! in order. The three ways of not answering with text — nothing on the
//! clipboard, no clipboard on this shell, content that is not text —
//! are one event, because the screen says the same thing about all of
//! them: "Nothing to paste". [`Command::WriteClipboard`] puts text on
//! the clipboard and is answered [`Event::ClipboardWritten`] or
//! [`Event::ClipboardNotWritten`].
//!
//! Both are emitted only for a user action: a tap on "Paste" or on
//! "Copy". The shell never reads the clipboard unasked, never watches
//! it, and never puts anything on it the core did not hand over. What
//! the core hands over is never a secret: seed words, a SeedQR, an
//! extended private key and a private key are refused on the way in and
//! carry no Copy row on the way out (`docs/DESIGN.md` §4.10).
//!
//! Camera channel (§4.4, §7): the core asks with [`Command::CameraOn`]
//! and the shell streams [`Event::CameraFrame`]s until
//! [`Command::CameraOff`], or answers [`Event::CameraUnavailable`] once
//! when it has no camera (or was refused one). Frames are 8-bit luma,
//! row-major, any size; a shell should downscale to about 640 pixels
//! wide before sending, since QR detection needs no more and the core
//! copies nothing it does not need. A frame may carry the NV12 chroma
//! plane beside its luma, and a shell that has colour should send it:
//! the core draws the viewfinder's preview in colour where the chroma
//! is there and in grey where it is not. The shell never interprets a
//! frame.
//!
//! A frame is the preview and nothing else. Reading the codes in it is
//! the shell's, because a decode costs more than the gap between frames
//! on a small board and the core has one thread: a core that decoded
//! would answer taps and paint at the decode rate. The shell decodes
//! its frames wherever it can afford to — a worker thread on a shell
//! that has threads — and sends each code it finds as one
//! [`Event::Scanned`], which the core routes exactly as it routes a
//! code from a file or a paste.
//!
//! Entropy channel (§5.2, §16.21): the core has no random number
//! generator. It asks with [`Command::RequestEntropy`] and the shell
//! answers with exactly one [`Event::Entropy`] carrying 32 bytes from its
//! best source (the OS RNG on desktop and phones, the hardware RNG on a
//! device). The core asks once at start and again whenever it rotates
//! its session key (on lock, on wipe). A shell that cannot answer leaves
//! the request unanswered; the core then keeps working without sealing
//! its secrets in memory and reports the session as weak. The bytes are
//! consumed into the session key and never echoed back.
//!
//! Settings channel (§6, §5.3): the six non-secret settings a person
//! chose survive a restart, and nothing else does. The core asks with
//! [`Command::StoreSettings`], which it emits only when the user changed
//! a setting, and the shell keeps the bytes wherever its platform keeps
//! application data. A shell that kept bytes last time hands them back
//! in one [`Event::Settings`], right after [`Event::Display`] and before
//! any input; one that kept nothing sends nothing. The bytes are the
//! core's own text format: the shell stores and returns them unread, and
//! never puts a key, a seed or a session in them, because the core never
//! puts one there.
//!
//! Kept-secret channel (§5.1, §6; `docs/PLANNING.md` §15 items 32 and 9):
//! a Tier B shell can keep one opaque blob for the core and can compute
//! an HMAC inside its secure element. The core asks with
//! [`Command::SecureMac`], [`Command::StoreSecret`],
//! [`Command::LoadSecret`] and [`Command::ForgetSecret`], and the shell
//! answers each with exactly one of the events named on it; a shell that
//! keeps a blob says so once, after [`Event::Display`], with
//! [`Event::SecretKept`]. The threat model is the reason the two halves
//! are separate. The blob is ciphertext whose key is derived from the
//! person's PIN *and* from a secure-element key that never leaves the
//! chip and needs the person's authentication for every use, so a copy of
//! the bytes cannot be attacked offline: every guess costs one
//! authentication on that one chip. The chip deletes its key after a set
//! number of failures, which makes every copy of the blob permanently
//! useless. The shell never learns what the blob holds, and the core
//! never learns the secure-element key.
//!
//! This crate is types only: `no_std` + `alloc`, no dependencies, nothing
//! here can carry a secret, so everything derives `Debug`. The one
//! exception is `Event::Entropy`: its bytes seed the session key, so they
//! travel in an [`EntropyBytes`], which prints nothing, hands them over
//! once, and wipes what it holds when it is dropped. The crate's one
//! dependency, `zeroize`, is there for that wipe.

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use zeroize::Zeroize;

/// What backs a shell's secure-element operations
/// ([`Command::SecureMac`] and the blob [`Command::StoreSecret`] keeps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureHardware {
    /// None: the shell has no secure element, and a secret cannot be
    /// kept on the device. Everything but the Android shell.
    None,
    /// A trusted execution environment: keys live outside the main
    /// operating system, in software the main processor runs apart.
    Tee,
    /// A separate chip: keys live on hardware of its own, with its own
    /// processor and memory.
    StrongBox,
}

/// What the platform's attestation says about the boot the device came
/// up from, reported in [`DisplayInfo::boot`].
///
/// A device whose bootloader is unlocked runs software nobody vouched
/// for, which weakens every promise its secure element makes about a key
/// kept on it: the chip still holds the key, but the system asking for it
/// is no longer the one the manufacturer signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootState {
    /// The shell cannot tell. Every shell but Android, and an Android
    /// device whose keystore attests nothing.
    Unknown,
    /// The running system is the one the manufacturer signed.
    Verified,
    /// It is not: an unlocked bootloader, a system signed by a key the
    /// owner installed, or a verification that ran and failed.
    Unverified,
}

/// What the shell knows about its display, sent once in [`Event::Display`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayInfo {
    /// Framebuffer width in pixels.
    pub width: u16,
    /// Framebuffer height in pixels.
    pub height: u16,
    /// Physical pixel density in dots per inch. Drives dp scaling and the
    /// size class; a shell that does not know should report its best guess
    /// (160 for a desktop window).
    pub dpi: u16,
    /// Height in pixels of the part of the frame's bottom edge the
    /// person cannot use: a phone's gesture bar, a panel's bezel
    /// overlap. What the app puts at the bottom stands this far clear
    /// of the edge. Zero on a shell that already excluded such an area
    /// from the frame it gave the core (the Android shell pads its view
    /// by the system bars), and zero on a panel or a window.
    pub inset_bottom: u16,
    /// The same at the top edge: a camera cutout, a status bar the
    /// shell draws under. The app bar starts below it. Zero where the
    /// shell handed the core only the usable area.
    pub inset_top: u16,
    /// Number of physical buttons the shell will report through
    /// [`Event::Button`]. Zero on touch-only devices. Buttons are optional
    /// accelerators; touch is always present.
    pub buttons: u8,
    /// Whether the camera's frames are always upright. True where the
    /// platform turns them for the shell (a phone) or where the camera
    /// is part of the machine and cannot be mounted another way (a
    /// laptop's webcam); false where the camera is a module someone
    /// mounted in a case they built, and the person may need to turn
    /// its frames. The core offers the camera-rotation setting only
    /// where this is false.
    pub camera_fixed: bool,
    /// What backs the shell's secure-element operations, and so whether
    /// a key can be kept on this device at all. [`SecureHardware::None`]
    /// everywhere but Android.
    pub secure: SecureHardware,
    /// What the platform's attestation says about the device's boot.
    /// [`BootState::Unknown`] on every shell that cannot tell, which is
    /// every shell but Android.
    pub boot: BootState,
    /// How much memory the device has, in MiB, where the shell can say.
    /// It decides which Argon2id memory cost the app recommends for an
    /// encrypted backup (`docs/PLANNING.md` §16.112): the recommendation
    /// follows the device's memory, and a shell that reports `None`
    /// gets the cost that opens anywhere. Nothing else reads it, and it
    /// is not a limit: the person may choose any of the offered costs.
    pub memory_mib: Option<u32>,
}

/// Phase of a touch (or mouse) contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    /// Contact began at the reported position.
    Down,
    /// Contact moved to the reported position.
    Move,
    /// Contact ended at the reported position.
    Up,
}

/// A physical button on a DIY build. Accelerators only (§4.4): every flow is
/// fully usable with touch alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonId {
    /// Move focus or scroll up.
    Up,
    /// Move focus or scroll down.
    Down,
    /// Move focus left.
    Left,
    /// Move focus right.
    Right,
    /// Activate the focused item.
    Select,
    /// Go one step toward Home.
    Back,
}

/// A physical keyboard key. Desktop and TUI shells only (§4.5); mobile
/// shells never emit [`Event::Key`], and PIN entry ignores it everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable character.
    Char(char),
    /// Delete the character before the cursor.
    Backspace,
    /// Confirm / done.
    Enter,
    /// Cancel / back.
    Escape,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Move focus to the next item.
    Tab,
    /// Move focus to the previous item: Tab with Shift held, which a
    /// shell reports as this rather than as a modifier of its own.
    BackTab,
}

/// What kind of file the core wants, so that a shell with a picker can
/// filter, and one without can find the right default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A PSBT, binary (`psbt\xff` magic) or base64 text.
    Psbt,
    /// Text: what a clipboard carries.
    Text,
    /// Anything.
    Any,
    /// A PNG image: a public code saved as a picture. A shell with a
    /// picker saves it as `image/png`, one without under a `.png` name.
    Png,
}

/// One file a shell that can list its files offers the core, in
/// [`Event::FileList`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// The name the shell reads the file back by, and the name the core
    /// shows: a plain file name, with no directory in it.
    pub name: String,
    /// The file's size in bytes.
    pub size: u64,
    /// When the file was last written, in seconds since the epoch, or
    /// `None` where the shell has no clock to say: a board with no
    /// real-time clock knows only that it is some time after boot.
    pub modified: Option<u64>,
}

/// The 32 bytes a shell answers [`Command::RequestEntropy`] with.
///
/// They seed the session key, so they are handled like key material
/// rather than like the rest of this crate's plain data: the bytes are
/// reachable only by consuming the value with [`into_bytes`], the
/// `Debug` prints `EntropyBytes(..)` so no shell can log them by
/// printing an event, and whatever is still held is wiped on drop.
///
/// [`into_bytes`]: EntropyBytes::into_bytes
#[derive(Clone, PartialEq, Eq)]
pub struct EntropyBytes([u8; 32]);

impl EntropyBytes {
    /// The event a shell builds from its random number generator.
    pub fn new(bytes: [u8; 32]) -> EntropyBytes {
        EntropyBytes(bytes)
    }

    /// The bytes, taken out of the value: the caller now owns them and
    /// owns wiping them.
    pub fn into_bytes(mut self) -> [u8; 32] {
        let bytes = self.0;
        self.0.zeroize();
        bytes
    }
}

impl fmt::Debug for EntropyBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EntropyBytes(..)")
    }
}

impl Drop for EntropyBytes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Shell → core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Display parameters. Must be the first event of a session and is
    /// sent exactly once.
    Display(DisplayInfo),
    /// A touch or mouse contact, in framebuffer pixels.
    Touch {
        /// Horizontal position in pixels from the left edge.
        x: u16,
        /// Vertical position in pixels from the top edge.
        y: u16,
        /// Which part of the contact this is.
        phase: TouchPhase,
    },
    /// A physical button press.
    Button(ButtonId),
    /// A physical keyboard key press (desktop/TUI only).
    Key(Key),
    /// A physical keyboard key coming up again (desktop/TUI only).
    ///
    /// Optional: a shell that cannot see a key released never sends it,
    /// and on that shell a hold cannot be held with a key (§4.15).
    KeyUp(Key),
    /// A scroll-wheel step at a position. `dy` is in pixels; positive means
    /// the content should move up (the user scrolls down).
    Scroll {
        /// Pointer x in pixels.
        x: u16,
        /// Pointer y in pixels.
        y: u16,
        /// Scroll amount in pixels.
        dy: i16,
    },
    /// A mouse wheel turned at a position. `dy` is in pixels, as for
    /// [`Event::Scroll`]: where the content should end up once it has
    /// moved, positive up. A core that animates glides there; one that
    /// does not treats it as a `Scroll`. A wheel that reports fractions
    /// of a notch sends them as the pixels they are worth.
    Wheel {
        /// Pointer x in pixels.
        x: u16,
        /// Pointer y in pixels.
        y: u16,
        /// Scroll amount in pixels.
        dy: i16,
    },
    /// The gesture that was sending [`Event::Scroll`] ended: the fingers
    /// lifted from a touchpad. Content that was moving may coast on with
    /// the speed it had. A wheel never sends it.
    ScrollEnd {
        /// Pointer x in pixels.
        x: u16,
        /// Pointer y in pixels.
        y: u16,
    },
    /// A pointer moved over the frame with no button down, in pixels.
    /// Optional: a shell with no pointer never sends it.
    Hover {
        /// Pointer x in pixels.
        x: u16,
        /// Pointer y in pixels.
        y: u16,
    },
    /// There is no pointer over the frame any more: it left the window,
    /// or it hid because a finger touched the panel.
    HoverEnd,
    /// Monotonic time, for timeouts and animations. The shell sends this at
    /// whatever rate it likes (typically 30–60 Hz while something animates,
    /// rarely otherwise); the core never assumes a rate.
    ///
    /// It must count every millisecond that passes, including time the
    /// device spent asleep and time the shell spent not ticking: the
    /// auto-lock and auto-wipe timeouts are wall-clock ones, and a
    /// clock that stops while the phone is in a pocket would hold a
    /// seed past the hour the person set. `CLOCK_BOOTTIME` on Linux,
    /// `SystemClock.elapsedRealtime` on Android.
    Tick {
        /// Milliseconds since an arbitrary epoch. Never decreases.
        now_ms: u64,
    },
    /// The answer to a [`Command::RequestFile`]: the raw content of the
    /// file the user chose.
    File {
        /// The kind that was requested.
        kind: FileKind,
        /// The file's bytes, unmodified.
        bytes: Vec<u8>,
    },
    /// The answer to a [`Command::RequestFile`] when this shell has no
    /// file channel at all: no picker, no card, nothing to read from.
    /// A read that failed is one of these too, since no file arrived.
    FileUnavailable {
        /// The kind that was requested.
        kind: FileKind,
    },
    /// The answer to a [`Command::RequestFile`] when the person closed
    /// the picker without choosing: the channel is there and they
    /// changed their mind. The core puts them back where they were and
    /// says nothing, which is what a cancelled action looks like
    /// everywhere else.
    FileCancelled {
        /// The kind that was requested.
        kind: FileKind,
    },
    /// The answer to a [`Command::RequestFile`] from a shell that can
    /// list what it holds: every file that could be what was asked for,
    /// newest first as the shell orders them, which is the order the
    /// core shows them in. An empty list is an answer too: the core
    /// shows the list's empty state. The core reads one of them with
    /// [`Command::ReadFile`].
    FileList {
        /// The kind that was requested.
        kind: FileKind,
        /// The files, newest first.
        entries: Vec<FileEntry>,
        /// Where the shell looked, named as a person would find it from
        /// another computer: a partition's label, a directory's name.
        /// `None` from a shell that has no such name to give. The core
        /// shows it where an empty list would otherwise say only that
        /// there is nothing.
        place: Option<String>,
    },
    /// The answer to a [`Command::WriteFile`]: the bytes are on the
    /// shell's storage under a name of its choosing.
    FileWritten {
        /// The kind that was written.
        kind: FileKind,
    },
    /// The answer to a [`Command::WriteFile`] when nothing was stored:
    /// the shell has no file channel, the user cancelled its picker, or
    /// the write failed.
    FileNotWritten {
        /// The kind that was not written.
        kind: FileKind,
    },
    /// The answer to a [`Command::RequestClipboard`]: what the
    /// clipboard holds, as text, unmodified.
    Clipboard {
        /// The kind that was requested.
        kind: FileKind,
        /// The clipboard's text.
        text: String,
    },
    /// The answer to a [`Command::RequestClipboard`] when no text
    /// comes: there is nothing on the clipboard, there is no clipboard
    /// on this shell, or what is on it is not text. The three are one
    /// event because the screen says the same thing about all of them.
    ClipboardUnavailable {
        /// The kind that was requested.
        kind: FileKind,
    },
    /// The answer to a [`Command::WriteClipboard`]: the text is on the
    /// clipboard.
    ClipboardWritten {
        /// The kind that was written.
        kind: FileKind,
    },
    /// The answer to a [`Command::WriteClipboard`] when nothing was
    /// put there: the shell has no clipboard, or the write failed.
    ClipboardNotWritten {
        /// The kind that was not written.
        kind: FileKind,
    },
    /// One camera frame, after [`Command::CameraOn`]: 8-bit luma,
    /// row-major, one byte per pixel, `width × height` bytes. Any size;
    /// about 640 pixels wide is plenty.
    ///
    /// The preview, and only the preview: the core turns it by the
    /// camera-rotation setting, reduces it and draws it in the
    /// viewfinder. Nothing is read from it. The codes come back as
    /// [`Event::Scanned`].
    CameraFrame {
        /// Frame width in pixels.
        width: u16,
        /// Frame height in pixels.
        height: u16,
        /// The pixels, black 0 to white 255.
        luma: Vec<u8>,
        /// The NV12 chroma plane for `luma`, when the device gives
        /// colour: `⌈width / 2⌉ × ⌈height / 2⌉` pairs of interleaved U
        /// and V bytes, row-major, one pair per 2 × 2 block of luma.
        /// `None` from a device with no colour to give — a `GREY`
        /// driver, a greyscale image — and then the preview is grey.
        chroma: Option<Vec<u8>>,
    },
    /// The payload of one code the shell read from its camera's
    /// frames, raw and unmodified: one event per code found, and only
    /// while the camera is on. The core routes it exactly as it routes
    /// a code it is given any other way, and ignores it off the scan
    /// screen, as it ignores a frame.
    Scanned {
        /// The code's bytes.
        bytes: Vec<u8>,
    },
    /// The answer to a [`Command::CameraOn`] when no frames will come:
    /// the shell has no camera, or access was refused. Sent once per
    /// `CameraOn`; the core then offers the file channel instead.
    CameraUnavailable,
    /// The answer to a [`Command::RequestEntropy`]: 32 bytes from the
    /// shell's random number generator. Sent once per request, never
    /// unasked.
    Entropy(EntropyBytes),
    /// Whether this shell is holding a kept secret. Sent once, after
    /// [`Event::Display`] and before any input, by a shell that can keep
    /// one; a shell that cannot sends nothing. The core shows the way
    /// back to a stored key only when this said `true`.
    SecretKept {
        /// Whether a blob is on the device.
        kept: bool,
    },
    /// The answer to a [`Command::SecureMac`]: HMAC-SHA256 of the salt
    /// under the secure element's key. Sent once per command.
    SecureMac {
        /// The 32-byte tag.
        mac: [u8; 32],
    },
    /// The answer to a [`Command::SecureMac`] when no tag will come:
    /// there is no secure hardware, the person cancelled the
    /// authentication, or the key is gone. Sent once per command.
    SecureUnavailable,
    /// The answer to a [`Command::StoreSecret`]: the bytes are kept.
    SecretStored,
    /// The answer to a [`Command::StoreSecret`] when nothing was kept.
    SecretNotStored,
    /// The answer to a [`Command::LoadSecret`]: the bytes the shell was
    /// last given to keep, unmodified.
    Secret {
        /// The blob, as it was stored.
        blob: Vec<u8>,
    },
    /// The answer to a [`Command::LoadSecret`] when no blob comes: none
    /// was kept, the hardware key that wraps it is gone, or the read
    /// failed.
    SecretUnavailable,
    /// The answer to a [`Command::ForgetSecret`]: the blob and every
    /// hardware key behind it are gone.
    SecretForgotten,
    /// The bytes of the last [`Command::StoreSettings`] the shell kept.
    /// Sent once, after [`Event::Display`] and before any input; a shell
    /// with nothing kept sends nothing. The core applies them whenever
    /// they arrive.
    Settings {
        /// What the shell was given to keep, unmodified.
        bytes: Vec<u8>,
    },
    /// Lock the session now, ahead of its timeout: the shell is leaving
    /// the foreground and the screen is about to belong to something
    /// else. A session with no PIN, or with no keys in it, has nothing
    /// to lock and this does nothing.
    ///
    /// The wipe timer is not this event's business. It runs on
    /// [`Event::Tick`], so a shell that stops ticking while it is away
    /// owes the core a tick carrying the wall-clock time that passed
    /// while it was gone, before it draws again.
    Lock,
}

/// Core → shell.
///
/// Later milestones add the general storage commands `StorageRead`,
/// `StorageWrite` and `StorageDelete` (§4.2). They are absent rather
/// than stubbed so that a shell cannot compile against an interface the
/// core does not honour yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Blit [`App::frame`] to the display now.
    Draw,
    /// Vibrate for the given number of milliseconds, if the device can.
    /// Never emitted for secret entry (§4.5).
    Vibrate {
        /// Duration in milliseconds.
        ms: u16,
    },
    /// The core is done; the shell should exit or return to its launcher.
    Exit,
    /// Get the bytes of a file. The shell answers with exactly one
    /// [`Event::File`], [`Event::FileCancelled`] or
    /// [`Event::FileUnavailable`], after any events it was already
    /// delivering; a shell with no file access answers
    /// `FileUnavailable` at once.
    RequestFile {
        /// What the core expects to parse.
        kind: FileKind,
    },
    /// Get the bytes of one file the shell listed in
    /// [`Event::FileList`], by the name it gave. The shell answers as it
    /// answers a [`Command::RequestFile`]; a name it did not list is an
    /// unavailable file and never a path it follows.
    ReadFile {
        /// What the core expects to parse.
        kind: FileKind,
        /// The name from the listing.
        name: String,
    },
    /// Store `bytes` as a file. The shell chooses where; `name_hint` is a
    /// plain file name with extension (`signed.psbt`) it should use when it
    /// has no better idea. Only ever emitted for a user action. The shell
    /// answers with exactly one [`Event::FileWritten`] or
    /// [`Event::FileNotWritten`], after any events it was already
    /// delivering; a shell with no file channel answers `FileNotWritten`
    /// at once.
    WriteFile {
        /// What the bytes are.
        kind: FileKind,
        /// Suggested file name.
        name_hint: String,
        /// The file content, unmodified.
        bytes: Vec<u8>,
    },
    /// Get what is on the clipboard, as text. The shell answers with
    /// exactly one [`Event::Clipboard`] or
    /// [`Event::ClipboardUnavailable`], after any events it was already
    /// delivering; a shell with no clipboard answers
    /// `ClipboardUnavailable` at once, and so does one whose clipboard
    /// is empty or holds something that is not text.
    ///
    /// Only ever emitted for a user action: the person tapped "Paste".
    /// The shell never reads the clipboard the core did not ask for and
    /// never watches it.
    RequestClipboard {
        /// What the core expects to parse.
        kind: FileKind,
    },
    /// Put `text` on the clipboard, replacing whatever was there. Only
    /// ever emitted for a user action: the person tapped "Copy". The
    /// shell answers with exactly one [`Event::ClipboardWritten`] or
    /// [`Event::ClipboardNotWritten`], after any events it was already
    /// delivering; a shell with no clipboard answers
    /// `ClipboardNotWritten` at once.
    ///
    /// The text is never a secret: the core copies addresses, extended
    /// public keys, descriptors, policies, transaction ids, signatures,
    /// signed transactions and encrypted backups, and nothing else
    /// (`docs/DESIGN.md` §4.10).
    WriteClipboard {
        /// What the text is.
        kind: FileKind,
        /// The text, unmodified.
        text: String,
    },
    /// Start the camera and stream [`Event::CameraFrame`]s, or answer
    /// [`Event::CameraUnavailable`] once. Idempotent while the camera is
    /// on.
    CameraOn,
    /// Stop the camera; no more frames after the shell has processed
    /// this. Harmless when the camera is off.
    CameraOff,
    /// Get 32 random bytes for the session key. The shell answers with
    /// exactly one [`Event::Entropy`], from its OS or hardware RNG; a
    /// shell without one does not answer, and the core carries on with
    /// a weak session (see the crate documentation).
    RequestEntropy,
    /// Keep `bytes` where the platform keeps application data, replacing
    /// whatever was kept before, and hand them back in
    /// [`Event::Settings`] next time. They are the settings the user
    /// chose and nothing else. Only ever emitted for a user action; a
    /// shell that keeps nothing drops it.
    StoreSettings {
        /// The settings, unmodified.
        bytes: Vec<u8>,
    },
    /// Compute HMAC-SHA256 over `salt` with a key held in the secure
    /// element that requires the person's authentication for every use.
    /// The shell answers with exactly one [`Event::SecureMac`] or
    /// [`Event::SecureUnavailable`]; a shell with no secure hardware
    /// answers `SecureUnavailable` at once. The key is created on first
    /// use and never leaves the element.
    ///
    /// The salt is a challenge derived from the PIN being tried, not a
    /// constant in the blob's header, so the answer is worth nothing for
    /// any other guess: a tag read out of one attempt does not turn the
    /// rest into an offline search (security review H2).
    SecureMac {
        /// The message to authenticate: the challenge for one guess.
        salt: [u8; 32],
    },
    /// Keep `blob`, wrapped once more by a hardware key of the shell's
    /// own, replacing whatever was kept before. The shell answers with
    /// exactly one [`Event::SecretStored`] or [`Event::SecretNotStored`].
    /// The bytes are opaque: the core has already encrypted everything
    /// secret in them.
    StoreSecret {
        /// The blob, unmodified.
        blob: Vec<u8>,
    },
    /// Hand back the kept blob. The shell answers with exactly one
    /// [`Event::Secret`] or [`Event::SecretUnavailable`].
    LoadSecret,
    /// Delete the blob and every hardware key behind it, so that no copy
    /// of the bytes can be tried again. The shell answers with exactly
    /// one [`Event::SecretForgotten`], whether or not it was holding
    /// anything.
    ForgetSecret,
}

/// A borrowed view of the core-owned framebuffer.
///
/// Premultiplied RGBA8888, row-major, top-left first, four bytes per pixel,
/// no row padding: `rgba.len() == width × height × 4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Pixel data.
    pub rgba: &'a [u8],
}

/// The whole application as the shell sees it.
pub trait App {
    /// Deliver one event. The first event of a session must be
    /// [`Event::Display`].
    fn event(&mut self, event: Event);

    /// Take the next pending command, or `None` when there are none. The
    /// shell calls this in a loop after every [`App::event`].
    fn poll_command(&mut self) -> Option<Command>;

    /// The current framebuffer, painted here from the layout the last
    /// event solved if it has not been painted since. Valid to read at
    /// any time after [`Event::Display`]; meaningful after a
    /// [`Command::Draw`], which means "read the frame and blit it now".
    /// A shell that reads it once per frame it draws paints once per
    /// drawn frame rather than once per event.
    fn frame(&mut self) -> Frame<'_>;
}
