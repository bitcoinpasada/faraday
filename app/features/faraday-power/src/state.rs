use faraday_kernel::AtLock;

use crate::Which;

/// The sheet on show. A lock leaves it (the next build starts over).
#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    sheet: Option<Which>,
    /// The Lock sheet on show is a mode change's, and the mode it asks for (`Mode::index`): it
    /// says "Change to X?". Public data, and still dropped with the sheet.
    #[lock(public)]
    mode_change: Option<u8>,
}

impl State {
    pub(crate) fn new(sheet: Option<Which>) -> State {
        State {
            sheet,
            mode_change: None,
        }
    }

    /// The sheet on show, if one is.
    #[must_use]
    pub fn sheet(&self) -> Option<Which> {
        self.sheet
    }

    /// The mode the Lock sheet on show asks for, when it is a mode change's.
    #[must_use]
    pub fn mode_change(&self) -> Option<u8> {
        self.mode_change
    }

    pub(crate) fn set_mode_change(&mut self, mode: Option<u8>) {
        self.mode_change = mode;
    }

    pub(crate) fn show(&mut self, sheet: Option<Which>) {
        if sheet != Some(Which::LockAsk) {
            self.mode_change = None;
        }
        self.sheet = sheet;
    }
}
