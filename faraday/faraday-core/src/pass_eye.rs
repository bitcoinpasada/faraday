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
