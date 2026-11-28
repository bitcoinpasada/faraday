use faraday_kernel::AtLock;

use crate::Which;

/// The sheet on show. A lock leaves it (the next build starts over).
#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    sheet: Option<Which>,
    /// The Lock sheet on show is a mode change's: it says "Change to X?".
    #[lock(public)]
    mode_change: bool,
}

impl State {
    pub(crate) fn new(sheet: Option<Which>) -> State {
        State {
            sheet,
            mode_change: false,
        }
    }

    /// The sheet on show, if one is.
    #[must_use]
    pub fn sheet(&self) -> Option<Which> {
        self.sheet
    }

    /// Whether the Lock sheet on show asks for a mode change.
    #[must_use]
    pub fn mode_change(&self) -> bool {
        self.mode_change
    }

    pub(crate) fn set_mode_change(&mut self, on: bool) {
        self.mode_change = on;
    }

    pub(crate) fn show(&mut self, sheet: Option<Which>) {
        if sheet != Some(Which::LockAsk) {
            self.mode_change = false;
        }
        self.sheet = sheet;
    }
}
