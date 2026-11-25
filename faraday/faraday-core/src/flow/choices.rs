//! Your choices: one row per answer of a flow, in the pane at the right of
//! the wide layout (`docs/FLOW-CONTAINER.md` §4). [`rows`] draws the list
//! alone, so a sheet can draw the same rows.

use super::{Contained, Foot, answers};
use crate::strings::EN;
use crate::trail::Steps;
use crate::ui::{Region, Slot, Ui, pal::*};
use crate::{Action, Faraday};
use faraday_draw::tokens::{self, BODY_S, CHOICES_INSET, CHOICES_ROW, LABEL_L};
use faraday_text::{Text, text};

/// A step's title as the container's heading reads it: text a flow may
/// build. Named here, where the text type is held, so `flow.rs` names none.
pub(crate) type Title = Text;

/// One row of a flow's Your choices: what was chosen, and the step it
/// opens. Never a secret.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    /// What the row is about.
    pub(crate) label: Text,
    /// The answer, or `None` for "Not chosen yet".
    pub(crate) value: Option<Text>,
    /// Set the answer in the mono face: it is data.
    pub(crate) mono: bool,
    /// The step a press on the row opens (the flow's own number). A flow
    /// whose checklist leads its items keeps an item it cannot open yet out
    /// of the press by giving its row an `opens` that is not in
    /// `Steps::order` (a row of an item not shown): `answers::takes_press`
    /// takes no press on a row whose step is not shown.
    pub(crate) opens: u8,
    /// The row stands for the step now open.
    pub(crate) current: bool,
    /// A key's row: its fingerprint, name and mark, set only where the flow
    /// says it; the row's `value` is not drawn then.
    pub(crate) mark: Option<KeyMark>,
    /// The value is an address: drawn whole in groups of four, the row
    /// growing to its lines (`Ui::address_whole`), never fitted.
    pub(crate) address: bool,
}

/// What a key's row draws: the fingerprint in the mono face, the name
/// fitted to what the others leave, and the mark whole in the primary role.
#[derive(Debug, Clone)]
pub(crate) struct KeyMark {
    /// The key's fingerprint, drawn whole.
    pub(crate) fp: Text,
    /// Its name, if it has one that may be shown; the part that is cut.
    pub(crate) name: Option<Text>,
    /// The mark, drawn whole after them; none for a key with no mark.
    pub(crate) mark: Option<&'static str>,
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
        let (label_ink, value_ink, quiet_ink) = if row.current {
            (
                Role::OnSecondaryContainer.paint(),
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
                Role::OnSurfaceVariant.paint(),
            )
        };
        let whole = row.address && row.value.is_some() && row.mark.is_none();
        let pad = (CHOICES_ROW - 2.0 * lh) / 2.0;
        let height = match (&row.value, whole) {
            (Some(v), true) => {
                (2.0 * pad + lh + ui.address_height(BODY_S.mono(), v.as_str(), w)).max(CHOICES_ROW)
            }
            _ => CHOICES_ROW,
        };
        if row.current {
            ui.fill(
                x - BLEED,
                cy,
                w + 2.0 * BLEED,
                height,
                tokens::SHAPE_S,
                Role::SecondaryContainer.paint(),
            );
        }
        let top = cy + (CHOICES_ROW - 2.0 * lh) / 2.0;
        ui.text_as(x, top, BODY_S, label_ink, row.label.as_str());
        let value = row
            .value
            .as_ref()
            .map_or(EN.flow_not_chosen_yet, |v| v.as_str());
        let role = if row.mono && row.value.is_some() {
            BODY_S.mono()
        } else {
            BODY_S
        };
        if let Some(k) = &row.mark {
            // Fingerprint (mono), then the name (cut to fit), then the mark.
            let sep = " · ";
            let fp_w = ui.measure_as(BODY_S.mono(), k.fp.as_str());
            ui.text_as(x, top + lh, BODY_S.mono(), value_ink, k.fp.as_str());
            let mut at = x + fp_w;
            let tail_w = k.mark.map_or(0.0, |m| {
                ui.measure_as(BODY_S, sep) + ui.measure_as(BODY_S, m)
            });
            if let Some(n) = &k.name {
                let lead = ui.measure_as(BODY_S, sep);
                let room = (x + w - at - tail_w - lead).max(0.0);
                let n = ui.fit_as(BODY_S, n.as_str(), room);
                if !n.as_str().is_empty() {
                    let full = text!("{}{}", sep, n.as_str());
                    at += ui.text_as(at, top + lh, BODY_S, value_ink, full.as_str());
                }
            }
            if let Some(m) = k.mark {
                let full = text!("{}{}", sep, m);
                ui.text_as(at, top + lh, BODY_S, Role::Primary.paint(), full.as_str());
            }
        } else if whole {
            ui.address_whole(
                (x, top + lh, w),
                BODY_S.mono(),
                (value_ink, quiet_ink),
                value,
            );
        } else {
            let value = ui.fit_as(role, value, w);
            ui.text_as(x, top + lh, role, value_ink, &value);
        }
        if let Some(action) = press {
            ui.press_box(x - BLEED, cy, w + 2.0 * BLEED, height, *action);
        }
        cy += height;
    }
    cy - y
}

/// The fewest rows of Your choices' region a pinned foot leaves above it.
const PIN_MIN_ROWS: f32 = 3.0;

/// The pane: the flow's rows, and what the flow gives for under them.
pub(crate) struct Pane<'a> {
    /// The app, for the flow's lookups.
    pub(crate) app: &'a Faraday,
    /// The flow.
    pub(crate) flow: &'a dyn Contained,
    /// Its steps as they stand.
    pub(crate) steps: &'a Steps,
    /// The wallet or key the flow is on, the first row, and its press.
    pub(crate) lead: Option<&'a (Row, Option<Action>)>,
    /// The open step's pick: its rows stand in place of Your choices.
    pub(crate) pick: Option<&'a super::Pick>,
    /// The last step's take-away: it stands in place of Your choices.
    pub(crate) take: Option<&'a super::TakeAway>,
    /// What the flow keeps at the pane's foot, outside what scrolls.
    pub(crate) pin: Option<&'a super::Pin<'a>>,
    /// The pane's left edge, its width and the window's height.
    pub(crate) place: (f32, f32, f32),
    /// Where its heading starts: the card's top where the pair is level.
    pub(crate) top: f32,
    /// How far it is scrolled within itself.
    pub(crate) offset: f32,
}

impl Pane<'_> {
    /// Draws the pane's rows on the page, which is the surface; the card's
    /// container draws the divider between the pane and the card. Returns
    /// the height the content needs from the pane's top, so the pair can
    /// end level. A pane taller than the window scrolls within itself
    /// (`Slot::Choices`).
    pub(crate) fn draw(&self, ui: &mut Ui, foot: Foot) -> f32 {
        let Some(pin) = self.pin else {
            return self.draw_scrolling(ui, foot, self.place);
        };
        // The flow's foot stands under a hairline: at the pane's foot where
        // the rows and what is under them are cut there and scroll above
        // it, directly under them where they fit.
        let (px, pw, h) = self.place;
        let band = pin.h + tokens::GAP * 2.0 + CHOICES_INSET;
        // A window so low that the region above it would hold under
        // `PIN_MIN_ROWS` rows: the foot stands in the scroll, after what is
        // under the rows.
        if h - band - self.top < CHOICES_ROW * PIN_MIN_ROWS {
            let mut with_pin = |ui: &mut Ui, x: f32, y: f32, w: f32| -> f32 {
                let n = foot(ui, x, y, w);
                (pin.draw)(ui, x, y + n + tokens::GAP, w);
                n + tokens::GAP + pin.h
            };
            return self.draw_scrolling(ui, &mut with_pin, self.place);
        }
        let used = self.draw_scrolling(ui, foot, (px, pw, h - band));
        let (x, w) = (px + CHOICES_INSET, pw - 2.0 * CHOICES_INSET);
        let top = (h - band).min(self.top + used);
        ui.rule(px, top, pw, Role::OutlineVariant);
        (pin.draw)(ui, x, top + tokens::GAP, w);
        top + band - self.top
    }

    /// The rows, scrolling within `place`.
    fn draw_scrolling(&self, ui: &mut Ui, foot: Foot, place: (f32, f32, f32)) -> f32 {
        if let Some(take) = self.take {
            return super::take_away::Pane {
                app: self.app,
                take,
                place,
                top: self.top,
                offset: self.offset,
            }
            .draw(ui, foot);
        }
        let (px, pw, h) = place;
        let top = self.top;
        let clip = ui.rect(px, top, pw, h - top);
        let off = self.offset;
        let open = ui.region_open(Region::scrolled(Slot::Choices, clip, off));
        let mark = ui.presses_mark();
        let (x, w) = (px + CHOICES_INSET, pw - 2.0 * CHOICES_INSET);
        let mut y = top;
        if let Some(pick) = self.pick {
            // The question is the heading; the rows are pick.rs's, each
            // group under its own heading.
            ui.text_as(x, y, LABEL_L, Role::OnSurfaceVariant, pick.question);
            y += ui.line_as(LABEL_L) + tokens::GRID;
            for g in &pick.groups {
                y += crate::pick::head(ui, x, y, w, g.head);
                y += crate::pick::rows(ui, x, y, w, &g.rows);
            }
            y += foot(ui, x, y, w);
            ui.clip_presses_since(mark, clip);
            ui.region_close(open, (y + CHOICES_INSET - h).max(0.0));
            return y + CHOICES_INSET - top;
        }
        if !(self.lead.is_none() && self.flow.lists_nothing(self.app)) {
            ui.text_as(x, y, LABEL_L, Role::OnSurfaceVariant, EN.flow_your_choices);
            y += ui.line_as(LABEL_L) + tokens::GRID;
        }
        let mut list = Vec::new();
        let mut presses = Vec::new();
        if let Some((row, press)) = self.lead {
            // What the row opens is drawn from where the row stands.
            if press.is_some() {
                ui.chip_at = Some((x, y + CHOICES_ROW - off, w));
            }
            list.push(row.clone());
            presses.push(*press);
        }
        let own = self.flow.rows(self.app);
        presses.extend(own.iter().map(|r| {
            answers::takes_press(self.steps, r).then(|| self.flow.open(self.app, r.opens))
        }));
        list.extend(own);
        y += rows(ui, &list, &presses, (x, y, w));
        y += foot(ui, x, y, w);
        // What is scrolled out of the pane takes no press.
        ui.clip_presses_since(mark, clip);
        ui.region_close(open, (y + CHOICES_INSET - h).max(0.0));
        y + CHOICES_INSET - top
    }
}

/// The sheet over a flow on the container: its rows, headed "Your choices",
/// as the pane draws them; a press on a row closes it and opens its step.
/// The window's width less a margin on compact, at most twice the board's
/// popup centred otherwise.
pub(crate) fn sheet(app: &crate::drawing::View, ui: &mut Ui, w: f32, h: f32) {
    let Some(flow) = app.step_flow() else {
        return;
    };
    let (Some(contained), Some(steps)) = (flow.contained(), flow.steps(app)) else {
        return;
    };
    // The pick's row is the pair's: where the sheet opens, the chip stays on
    // the card's head.
    let mut list: Vec<Row> = Vec::new();
    let mut presses: Vec<Option<Action>> = Vec::new();
    for r in contained.rows(app) {
        presses.push(
            answers::takes_press(&steps, &r)
                .then_some(Action::Choices(crate::action::ChoicesPress::Row(r.opens))),
        );
        list.push(r);
    }
    let place = if ui.compact {
        (tokens::GAP, w - 2.0 * tokens::GAP)
    } else {
        let sw = (w - 2.0 * tokens::BOARD_GAP).min(tokens::BOARD_POPUP_WIDTH * 2.0);
        ((w - sw) / 2.0, sw)
    };
    let pad = tokens::GRID * 2.0;
    crate::compact::scroll_sheet(ui, place, h, tokens::GAP, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += crate::compact::sheet_head(
            ui,
            x,
            cy,
            iw,
            faraday_draw::widgets::Icon::Checklist,
            Role::OnSurface,
            EN.flow_your_choices,
        );
        cy += rows(ui, &list, &presses, (x, cy, iw));
        cy += crate::compact::buttons(
            ui,
            x,
            cy,
            iw,
            &[(
                EN.nav_close,
                crate::ui::Style::Secondary,
                Action::Choices(crate::action::ChoicesPress::Close),
            )],
        );
        cy - y
    });
}
