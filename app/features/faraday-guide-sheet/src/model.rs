use faraday_organisms::Shown;
use faraday_organisms::guide::GuideSheetModel;

use crate::Press;

/// What `present` works out for the screen (04 §2): the guide, none while it is closed.
pub struct Model<'s> {
    pub(crate) guide: Option<GuideSheetModel<'s, Press>>,
    /// The guide stands as the supporting pane, not as the sheet.
    pub(crate) pane: Option<PaneModel<'s>>,
}

/// What the pane has beyond the sheet: its close button's tip.
pub struct PaneModel<'s> {
    pub(crate) close: Shown<'s>,
}

impl Model<'_> {
    /// The guide is open, as the sheet or the pane.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.guide.is_some()
    }

    /// The guide stands as the pane.
    #[must_use]
    pub fn is_pane(&self) -> bool {
        self.pane.is_some()
    }
}
