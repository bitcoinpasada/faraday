//! Your choices: one row per answer of a flow, in the pane at the right of
//! the wide layout (`docs/FLOW-CONTAINER.md` §4). [`rows`] draws the list
//! alone, so a sheet can draw the same rows.

use super::{Contained, Foot, answers};
use crate::strings::EN;
use crate::trail::Steps;
use crate::ui::{Region, Slot, Ui, pal::*};
use crate::{Action, Faraday};
use faraday_draw::tokens::{self, BODY_S, CHOICES_INSET, CHOICES_ROW, LABEL_L};
use faraday_text::Text;

/// One row of a flow's Your choices: what was chosen, and the step it
/// opens. Never a secret.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    /// What the row is about.
    pub(crate) label: &'static str,
    /// The answer, or `None` for "Not chosen yet".
    pub(crate) value: Option<Text>,
    /// Set the answer in the mono face: it is data.
    pub(crate) mono: bool,
    /// The step a press on the row opens (the flow's own number).
    pub(crate) opens: u8,
    /// The row stands for the step now open.
    pub(crate) current: bool,
}

/// How far a row's highlight and press reach past its text at each side.
const BLEED: f32 = 12.0;

/// Draws `list` from (x, y), `w` wide, a row of `CHOICES_ROW` each: the
/// label, and under it the answer, or "Not chosen yet" in the off colour.
/// The row marked current is drawn in the secondary container, as
/// Material's selected list item; a row whose entry in `presses` is `Some`
/// is a press over its whole row. Returns the height.
pub(crate) fn rows(
    ui: &mut Ui,
    list: &[Row],
    presses: &[Option<Action>],
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let lh = ui.line_as(BODY_S);
    let mut cy = y;
    for (row, press) in list.iter().zip(presses) {
        let (label_ink, value_ink) = if row.current {
            (
                Role::OnSecondaryContainer.paint(),
                Role::OnSecondaryContainer.paint(),
            )
        } else {
            (
                Role::OnSurfaceVariant.paint(),
                if row.value.is_some() {
                    Role::OnSurface.paint()
                } else {
                    crate::ui::off()
                },
            )
        };
        if row.current {
            ui.fill(
                x - BLEED,
                cy,
                w + 2.0 * BLEED,
                CHOICES_ROW,
                tokens::SHAPE_S,
                Role::SecondaryContainer.paint(),
            );
        }
        let top = cy + (CHOICES_ROW - 2.0 * lh) / 2.0;
        ui.text_as(x, top, BODY_S, label_ink, row.label);
        let value = row
            .value
            .as_ref()
            .map_or(EN.flow_not_chosen_yet, |v| v.as_str());
        let role = if row.mono && row.value.is_some() {
            BODY_S.mono()
        } else {
            BODY_S
        };
        let value = ui.fit_as(role, value, w);
        ui.text_as(x, top + lh, role, value_ink, &value);
        if let Some(action) = press {
            ui.press_box(x - BLEED, cy, w + 2.0 * BLEED, CHOICES_ROW, *action);
        }
        cy += CHOICES_ROW;
    }
    cy - y
}

/// The pane: the flow's rows, and what the flow gives for under them.
pub(crate) struct Pane<'a> {
    /// The app, for the flow's lookups.
    pub(crate) app: &'a Faraday,
    /// The flow.
    pub(crate) flow: &'a dyn Contained,
    /// Its steps as they stand.
    pub(crate) steps: &'a Steps,
    /// The pane's left edge, its width and the window's height.
    pub(crate) place: (f32, f32, f32),
    /// How far it is scrolled within itself.
    pub(crate) offset: f32,
}

impl Pane<'_> {
    /// Draws the pane on the surface with a divider at its start. A pane
    /// taller than the window scrolls within itself (`Slot::Choices`).
    pub(crate) fn draw(&self, ui: &mut Ui, foot: Foot) {
        let (px, pw, h) = self.place;
        ui.fill(px, 0.0, pw, h, 0.0, Role::Surface);
        ui.fill(px, 0.0, 1.0, h, 0.0, Role::OutlineVariant);
        let top = tokens::SIDE_PANEL_TOP;
        let clip = ui.rect(px + 1.0, top, pw - 1.0, h - top);
        let off = self.offset;
        let open = ui.region_open(Region::scrolled(Slot::Choices, clip, off));
        let mark = ui.presses_mark();
        let (x, w) = (px + CHOICES_INSET, pw - 2.0 * CHOICES_INSET);
        let mut y = top;
        ui.text_as(x, y, LABEL_L, Role::OnSurfaceVariant, EN.flow_your_choices);
        y += ui.line_as(LABEL_L) + tokens::GRID;
        let list = self.flow.rows(self.app);
        let presses: Vec<Option<Action>> = list
            .iter()
            .map(|r| answers::takes_press(self.steps, r).then(|| self.flow.open(self.app, r.opens)))
            .collect();
        y += rows(ui, &list, &presses, (x, y, w));
        y += foot(ui, x, y, w);
        // What is scrolled out of the pane takes no press.
        ui.clip_presses_since(mark, clip);
        ui.region_close(open, (y + CHOICES_INSET - h).max(0.0));
    }
}
