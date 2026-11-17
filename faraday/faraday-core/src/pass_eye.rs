//! The eye on the BIP-39 passphrase fields: what is typed shows in the
//! clear only while it is open, it is closed by default and by a lock or
//! the idle warning, and a passphrase typed with a hardware keyboard waits
//! for the eye to be opened once, so the person sees what those keys gave.

/// The eye of the passphrase fields on the screen.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PassEye {
    /// The characters are drawn, not dots.
    pub shown: bool,
    /// A character of the passphrase being made came from a hardware key.
    pub hardware: bool,
    /// The eye has been opened since the first hardware key.
    pub opened: bool,
}

impl PassEye {
    /// Presses the eye.
    pub fn toggle(&mut self) {
        self.shown = !self.shown;
        self.opened |= self.shown;
    }

    /// Masks the characters, as a lock and the idle warning do.
    pub fn close(&mut self) {
        self.shown = false;
    }

    /// Forgets what the passphrase being made was typed with; the eye stays.
    pub fn reset(&mut self) {
        self.hardware = false;
        self.opened = false;
    }

    /// A hardware key's character went into a passphrase.
    pub fn typed_by_hardware(&mut self) {
        if !self.hardware {
            self.hardware = true;
            self.opened = false;
        }
    }

    /// Whether the step waits for the eye to be opened.
    pub fn waits(&self) -> bool {
        self.hardware && !self.opened
    }
}

use crate::Faraday;

/// How many characters the passphrase fields of the screen hold, where the
/// screen is Add a key, a Create slot's key or the seed check (New key
/// keeps its own count).
fn pass_len(app: &Faraday) -> Option<usize> {
    match app.screen {
        crate::Screen::Entry => {
            let e = app.entry.state();
            Some(e.passphrase.as_str().len() + e.again.as_str().len())
        }
        crate::Screen::Create => {
            let c = app.create.state()?;
            c.pass_slot?;
            Some(c.pass.as_str().len() + c.pass2.as_str().len())
        }
        _ if crate::seed_check::here(app) => Some(app.tools.seed_check.state()?.pass_len()),
        _ => None,
    }
}

/// The passphrase length before a hardware key, to hand to `after_key`.
pub(crate) fn before_key(app: &Faraday) -> Option<usize> {
    pass_len(app)
}

/// After a hardware key: a character that went into a passphrase field
/// makes the passphrase wait for the eye.
pub(crate) fn after_key(app: &mut Faraday, key: crate::KeyIn, before: Option<usize>) {
    if matches!(key, crate::KeyIn::Char(_))
        && let (Some(was), Some(now)) = (before, pass_len(app))
        && now > was
    {
        app.typing.pass_eye.typed_by_hardware();
    }
}

/// After an event: with no passphrase typed on those screens, nothing is
/// left to wait for, and leaving them closes the eye.
pub(crate) fn settle(app: &mut Faraday) {
    if app.screen == crate::Screen::KeyGen {
        return;
    }
    match pass_len(app) {
        None => {
            app.typing.pass_eye.reset();
            app.typing.pass_eye.close();
        }
        Some(0) => app.typing.pass_eye.reset(),
        Some(_) => {}
    }
}

/// A key was made or the entry given up: the eye closes and nothing is
/// left to wait for.
pub(crate) fn done(app: &mut Faraday) {
    app.typing.pass_eye.close();
    app.typing.pass_eye.reset();
}
