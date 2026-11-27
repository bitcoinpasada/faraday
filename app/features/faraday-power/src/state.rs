use faraday_kernel::AtLock;

use crate::Which;

/// The sheet on show. A lock leaves it (the next build starts over).
#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    sheet: Option<Which>,
}

impl State {
    pub(crate) fn new(sheet: Option<Which>) -> State {
        State { sheet }
    }

    /// The sheet on show, if one is.
    #[must_use]
    pub fn sheet(&self) -> Option<Which> {
        self.sheet
    }

    pub(crate) fn show(&mut self, sheet: Option<Which>) {
        self.sheet = sheet;
    }
}
