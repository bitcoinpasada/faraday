//! The flow container's page: the current step's card at one fixed place,
//! scrolled within the page area, with Your choices at its right on the
//! wide layout (`docs/FLOW-CONTAINER.md` §2, §4).

use super::{Contained, Foot, Room, Scroll, past_end};
use crate::strings::{EN, fill};
use crate::ui::{Moves, Region, Ui, pal::*};
use crate::{Action, Faraday};
use faraday_draw::tokens::{self, BODY_S, COLUMN_MAX, MARGIN_COMPACT, MARGIN_WIDE, TITLE_M};

/// What the container needs of the screen besides the flow's declaration.
pub(crate) struct Page<'a> {
    /// The page area's left edge and width.
    pub(crate) area: (f32, f32),
    /// The window's height in design units.
    pub(crate) h: f32,
    /// What a panel at the page's foot takes of that height.
    pub(crate) foot_h: f32,
    /// The way back, on the top app bar over the flow.
    pub(crate) back: Option<(&'a str, Action)>,
    /// The flow's name, on the top app bar over it.
    pub(crate) heading: &'a str,
    /// A chip at the head of the card: where the flow goes next.
    pub(crate) chip: Option<&'a str>,
    /// How far Your choices is scrolled within its pane.
    pub(crate) choices_y: f32,
    /// Whether a scroll step moves the rows already drawn.
    pub(crate) moves: Moves,
}

/// Draws the flow's current step: its chip, heading and `body`, which
/// draws the step's controls (by the flow's own number) at (x, y) in a
/// width and returns their height; then, on the wide layout, Your choices
/// with `foot` under its rows. Returns the scroll as it should be next,
/// whether the frame must be drawn again because it moved, and where the
/// card stands.
pub(crate) fn contained(
    ui: &mut Ui,
    app: &Faraday,
    flow: &dyn Contained,
    page: &Page,
    scroll: Scroll,
    body: &mut dyn FnMut(&mut Ui, u8, f32, f32, f32) -> f32,
    foot: Foot,
) -> (Scroll, bool, Room) {
    let (area_x, area_w) = page.area;
    let compact = ui.compact;
    let pane_w = if compact {
        0.0
    } else {
        crate::screens::side_panel_w(area_w)
    };
    let room_w = area_w - pane_w;
    let view_h = page.h - page.foot_h;
    let steps = flow.steps(app);

    // A step just opened starts at its top, at once.
    let mut next = scroll;
    let mut moved = false;
    if next.follow {
        next.follow = false;
        next.y = 0.0;
        moved = true;
    }

    // The card's place: the same for every step.
    let (top, x, w) = if compact {
        let m = crate::compact::M;
        (
            crate::compact::BAR_H,
            area_x + m,
            (room_w - 2.0 * m).min(COLUMN_MAX),
        )
    } else {
        let w = (room_w - 2.0 * MARGIN_WIDE).min(COLUMN_MAX);
        (tokens::PAGE_TOP_BAR, area_x + (room_w - w) / 2.0, w)
    };
    let clip_top = if compact { top } else { 0.0 };
    let clip = ui.rect(area_x, clip_top, room_w, view_h - clip_top);
    let view_open = ui.region_open(Region::page(clip).from_zero(next.y).moves(page.moves));
    let mut y = if compact { top + 10.0 } else { top };
    if !compact {
        // The top app bar over the page names the flow and carries its way
        // back and Cancel; the card keeps what is the flow's: its chip.
        crate::screens::bar_head(ui, page.back, page.heading);
    }
    let card_top = y;
    if let Some(chip) = page.chip {
        let chip = ui.fit_as(BODY_S, chip, w - 34.0);
        ui.chip_press(
            x,
            y,
            &chip,
            (
                Role::OnSecondaryContainer.paint(),
                Role::SecondaryContainer.paint(),
            ),
            None,
        );
        y += if compact {
            36.0
        } else {
            tokens::TAG_H + tokens::PAD
        };
    }

    if let Some(s) = steps.as_ref() {
        let at = s.order.iter().position(|&k| Some(k) == s.open);
        if let (Some(at), Some(&k)) = (at, s.open.as_ref()) {
            let count = if flow.count_fixed(app) {
                fill(
                    EN.flow_step_count,
                    &[&(at + 1).to_string(), &s.order.len().to_string()],
                )
            } else {
                fill(EN.flow_step_number, &[&(at + 1).to_string()])
            };
            let count_w = ui.measure_as(BODY_S, &count);
            let title_w = w - count_w - 10.0;
            let lh = ui.line_as(TITLE_M);
            let title = flow.name(app, k);
            let th = ui.wrap_as((x, y, title_w), TITLE_M, Role::OnSurface, title);
            ui.text_right_as((x + w, y, lh), BODY_S, Role::OnSurfaceVariant, &count);
            y += th + 10.0;
            ui.page_view = compact.then_some((view_h + next.y, view_h - top));
            y += body(ui, k, x, y, w);
            ui.page_view = None;
        }
    }
    let max = (y + 24.0 - view_h).max(0.0);
    ui.region_close(view_open, max);
    ui.column = None;
    // In screen space: the card's own place less the scroll, kept to the
    // page area under the bar and above any foot.
    let bar_end = if compact { top } else { tokens::APP_BAR_H };
    let (shown_top, shown_end) = ((card_top - next.y).max(bar_end), (y - next.y).min(view_h));
    ui.flow_card = Some((x, shown_top, w, (shown_end - shown_top).max(0.0)));

    if compact {
        bar(ui, page, area_x + area_w);
    } else if let Some(s) = steps.as_ref() {
        let px = area_x + area_w - pane_w;
        super::choices::Pane {
            app,
            flow,
            steps: s,
            place: (px, pane_w, page.h),
            offset: page.choices_y,
        }
        .draw(ui, foot);
    }

    let mut again = false;
    if past_end(next.y, max) {
        next.y = max;
        again = true;
    }
    let room = Room {
        area_x,
        area_w: room_w,
        x,
        w,
    };
    (next, again || moved, room)
}

/// The small panel's top app bar, over whatever has scrolled under it:
/// the way back, the flow's name, and at its right end the ?, the keyboard
/// and Cancel (`compact::flow_bar_tools`).
fn bar(ui: &mut Ui, page: &Page, w: f32) {
    let back = page
        .back
        .unwrap_or((EN.flow_home, Action::Nav(crate::Screen::Home)));
    let head = faraday_ui::nav::AppBar {
        back: Some(back),
        title: page.heading,
        back_tip: EN.tip_back,
    };
    let right = ui.bar_tools_x.unwrap_or(w - MARGIN_COMPACT);
    faraday_ui::nav::app_bar(ui, w, &head, right);
}
