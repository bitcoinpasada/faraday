//! Your choices as a sheet: what the bar's button and the sheet's rows do
//! (`docs/FLOW-CONTAINER.md` §4).

use crate::action::ChoicesPress;
use crate::{Faraday, Sheet};

pub(crate) use super::choices::sheet;

impl Faraday {
    /// A press of the bar's **Your choices** button or of the sheet: the
    /// sheet opens or closes; a row closes it and presses the step's own
    /// `StepFlow::open`.
    pub(crate) fn choices_act(&mut self, press: ChoicesPress) {
        match press {
            ChoicesPress::Open => {
                if self.choices_offered() {
                    self.sheet = Some(Sheet::Choices);
                }
            }
            ChoicesPress::Close => {
                if self.sheet == Some(Sheet::Choices) {
                    self.sheet = None;
                }
            }
            ChoicesPress::Row(k) => {
                if self.sheet == Some(Sheet::Choices) {
                    self.sheet = None;
                }
                if let Some(flow) = self.step_flow() {
                    let open = flow.open(self, k);
                    self.act_inner(open);
                }
            }
        }
    }

    /// Whether the top app bar carries the **Your choices** button: a
    /// flow on the container on a one-column layout, or on the wide one
    /// where the open step takes the whole room and has no pane. A lookup.
    pub(crate) fn choices_offered(&self) -> bool {
        let Some(flow) = self.step_flow().and_then(|f| f.contained()) else {
            return false;
        };
        self.nav() != crate::nav::Nav::ExpandedRail || flow.full_width(self)
    }

    /// Whether a contained flow is on screen, which keeps the guide out
    /// of the pane's place (Your choices keeps the right edge). A lookup.
    pub(crate) fn contained_on_show(&self) -> bool {
        self.step_flow().and_then(|f| f.contained()).is_some()
    }
}
