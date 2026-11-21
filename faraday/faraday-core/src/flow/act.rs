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

    /// Whether the flow on show is drawn as the unit: on the wide layout,
    /// a flow on the container whose card is a column (not the full-width
    /// steps, nor the wide content that stands at the navigation's edge).
    /// A lookup.
    pub(crate) fn unit_on_show(&self) -> bool {
        !self.is_compact()
            && self
                .step_flow()
                .and_then(|f| f.contained())
                .is_some_and(|c| !c.full_width(self) && !c.wide_content(self))
    }

    /// Whether the window holds Guide's pane beside the unit: the room
    /// beside the navigation holds the unit at its narrowest and the pane.
    /// A lookup.
    pub(crate) fn unit_guide_fits(&self) -> bool {
        self.window().width - crate::screens::SIDEBAR_W >= super::unit::guide_min_room()
    }

    /// The unit's place in a `room` wide, as the flow now stands: shifted
    /// when Guide's pane has stood beside it in this flow.
    pub(crate) fn unit_place(&self, room: f32) -> super::unit::Place {
        super::unit::place(room, self.scroll.unit_shifted())
    }

    /// Before a frame: the unit stays where Guide's pane put it until the
    /// flow ends.
    pub(crate) fn unit_settle(&mut self) {
        let on = self.unit_on_show();
        let shifted = on && (self.scroll.unit_shifted() || self.guide_pane_shown());
        self.scroll.set_unit_shifted(shifted);
    }
}
