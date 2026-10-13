//! What the Companion's core and its shell say to each other beyond
//! upstream's [`osk_shell_api`]: the shell reports through
//! [`crate::Companion::companion_event`] and takes what the core asks
//! for from [`crate::Companion::poll_companion`]. Each type is named so
//! the JNI layer (`faraday-companion-ffi`, batch A3) can encode it.

/// Shell → core, beside [`osk_shell_api::Event`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionEvent {
    /// The wall-clock time, when the app starts and whenever it comes
    /// back to the foreground: what a kept file's "when" is read from.
    Time {
        /// Seconds since 1970-01-01 UTC.
        unix_s: u64,
        /// The phone's offset from UTC at that moment, in seconds, so
        /// the time is shown as the phone's clock shows it.
        offset_s: i32,
    },
    /// The received files' store, as the shell last kept it from a
    /// [`CompanionCommand::KeepReceived`]: sent once at start, after
    /// [`osk_shell_api::Event::Display`], by a shell that has one. A
    /// shell with nothing kept sends nothing.
    Received {
        /// The bytes, as they were handed to the shell.
        bytes: Vec<u8>,
    },
    /// What About states from the system (`docs/COMPANION.md` §4.8),
    /// read at run time: sent once at start.
    System {
        /// The app's version name and code, as the package says them:
        /// "0.1.1 (3)".
        build: String,
        /// The SHA-256 of the APK's signing certificate, the digest
        /// `apksigner verify --print-certs` prints.
        certificate_sha256: [u8; 32],
        /// The permissions the app's manifest asks for, by their names
        /// (`android.permission.CAMERA`), each with whether the system
        /// has granted it.
        permissions: Vec<(String, bool)>,
    },
    /// The answer to [`osk_shell_api::Command::RequestFile`] from a shell
    /// that knows the picked file's name: sent in place of
    /// [`osk_shell_api::Event::File`], so the codes go under that name.
    Picked {
        /// The name the document provider gives it.
        name: String,
        /// The file's bytes, unmodified.
        bytes: Vec<u8>,
    },
    /// The theme a [`CompanionCommand::KeepTheme`] kept, sent once at
    /// start by a shell that has one.
    Theme {
        /// [`faraday_ui::Theme::id`].
        id: String,
    },
    /// What the shell's scanner worker saw of a code in the camera's
    /// frame (`faraday-scanner`'s `Seen`), so Receive can outline it.
    Seen {
        /// The frame's width, in pixels.
        width: u16,
        /// The frame's height, in pixels.
        height: u16,
        /// The code's corners in the frame: top-left, top-right,
        /// bottom-right, bottom-left.
        corners: [(u16, u16); 4],
        /// It read.
        read: bool,
    },
}

/// Core → shell, beside [`osk_shell_api::Command`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionCommand {
    /// Keep these bytes as the received files' store, replacing what
    /// was kept, and hand them back in [`CompanionEvent::Received`] next
    /// time. The whole store, after each change.
    KeepReceived {
        /// The store ([`crate::store`]'s format).
        bytes: Vec<u8>,
    },
    /// Keep the screen on (`true`) while a code is shown, or let it
    /// sleep again (`false`).
    KeepScreenOn {
        /// Whether the screen stays on.
        on: bool,
    },
    /// Keep the theme chosen on About, and hand it back in
    /// [`CompanionEvent::Theme`] next time.
    KeepTheme {
        /// [`faraday_ui::Theme::id`].
        id: String,
    },
}
