//! The eye on the passphrase fields: what is typed shows in the clear only while it is open, it is
//! closed by default and by a lock or the idle warning, and a passphrase typed with a hardware
//! keyboard waits for the eye to be opened once, so the person sees what those keys gave
//! (`main`'s `pass_field::eye::PassEye`).

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Eye {
    /// The characters are drawn, not dots.
    shown: bool,
    /// A character of the passphrase being made came from a hardware key.
    hardware: bool,
    /// The eye has been opened since the first hardware key.
    opened: bool,
}

impl Eye {
    /// Presses the eye.
    pub(crate) fn toggle(&mut self) {
        self.shown = !self.shown;
        self.opened |= self.shown;
    }

    /// Masks the characters, as a lock and the idle warning do.
    pub(crate) fn close(&mut self) {
        self.shown = false;
    }

    /// Forgets what the passphrase being made was typed with; the eye stays.
    pub(crate) fn reset(&mut self) {
        self.hardware = false;
        self.opened = false;
    }

    /// A hardware key's character went into a passphrase.
    pub(crate) fn typed_by_hardware(&mut self) {
        if !self.hardware {
            self.hardware = true;
            self.opened = false;
        }
    }

    /// Whether the characters are drawn.
    pub(crate) fn shown(&self) -> bool {
        self.shown
    }

    /// Whether the step waits for the eye to be opened.
    pub(crate) fn waits(&self) -> bool {
        self.hardware && !self.opened
    }
}
