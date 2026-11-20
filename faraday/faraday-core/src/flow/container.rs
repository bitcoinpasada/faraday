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
    /// A line under the chip, with a primary bar at its left: where the
    /// flow stands in something outside it.
    pub(crate) note: Option<&'a str>,
    /// What a press on the chip does; where it was drawn is left in
    /// `Ui::chip_at`.
    pub(crate) chip_tap: Option<Action>,
    /// The title the open step's card reads in place of the flow's own:
    /// where the top app bar has no room for it, the entry's step.
    pub(crate) step: Option<&'a str>,
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
    let full = !compact && flow.full_width(app);
    let pane_w = if compact || full {
        0.0
    } else {
        crate::screens::side_panel_w(area_w)
    };
    let room_w = area_w - pane_w;
    let view_h = page.h - page.foot_h;
    let steps = flow.steps(app);

    // A step just opened starts at its top, at once.
    // A fade of a step with a secret in it, drawn live: the card shows the
    // step left while it fades out, the page where it was.
    let out = app.live_out();
    let mut next = scroll;
    let mut moved = false;
    if next.follow && out.is_none() {
        next.follow = false;
        next.y = 0.0;
        moved = true;
    }

    // The card's place: the same for every step.
    let aligned = !compact && (full || flow.wide_content(app));
    let (top, x, w) = if compact {
        let m = crate::compact::M;
        (
            crate::compact::BAR_H,
            area_x + m,
            (room_w - 2.0 * m).min(COLUMN_MAX),
        )
    } else {
        let w = (room_w - 2.0 * MARGIN_WIDE).min(COLUMN_MAX);
        // Wide content stands at the navigation's edge, not centred.
        let x = if aligned {
            area_x + MARGIN_WIDE
        } else {
            area_x + (room_w - w) / 2.0
        };
        (tokens::PAGE_TOP_BAR, x, w)
    };
    // Where the work of a wide card ends: before the pane, or at the
    // area's end less the margin when there is none.
    let work = aligned.then_some({
        if full {
            area_x + area_w - MARGIN_WIDE
        } else {
            area_x + room_w - tokens::SPACER
        }
    });
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
        let cw = ui.chip_press(
            x,
            y,
            &chip,
            (
                Role::OnSecondaryContainer.paint(),
                Role::SecondaryContainer.paint(),
            ),
            page.chip_tap,
        );
        if page.chip_tap.is_some() {
            ui.chip_at = Some((x, y + tokens::TAG_H, cw));
        }
        y += if compact {
            36.0
        } else {
            tokens::TAG_H + tokens::PAD
        };
    }
    if let Some(note) = page.note {
        let (inset, gap) = if compact { (12.0, 14.0) } else { (14.0, 20.0) };
        let nh = ui.wrap_as(
            (x + inset, y, w - inset),
            BODY_S,
            Role::OnSurfaceVariant,
            note,
        );
        ui.fill(x, y, 3.0, nh, 1.5, Role::Primary.paint());
        y += nh + gap;
    }

    if let Some(s) = steps.as_ref() {
        let shown = out.filter(|k| s.order.contains(k)).or(s.open);
        let leaving = shown != s.open;
        let at = s.order.iter().position(|&k| Some(k) == shown);
        if let (Some(at), Some(&k)) = (at, shown.as_ref()) {
            // Steps before step 1 read their title alone.
            let heads = flow.heads(app).min(s.order.len());
            let count = if at < heads {
                String::new()
            } else if flow.count_fixed(app) {
                fill(
                    EN.flow_step_count,
                    &[
                        &(at + 1 - heads).to_string(),
                        &(s.order.len() - heads).to_string(),
                    ],
                )
            } else {
                fill(EN.flow_step_number, &[&(at + 1 - heads).to_string()])
            };
            let count_w = ui.measure_as(BODY_S, &count);
            let title_w = w - count_w - 10.0;
            let lh = ui.line_as(TITLE_M);
            let title = flow.title(app, k);
            let title = match page.step {
                Some(step) if Some(k) == s.open => step,
                _ => title.as_str(),
            };
            let th = ui.wrap_as((x, y, title_w), TITLE_M, Role::OnSurface, title);
            ui.text_right_as((x + w, y, lh), BODY_S, Role::OnSurfaceVariant, &count);
            y += th + 10.0;
            ui.page_view = compact.then_some((view_h + next.y, view_h - top));
            ui.flow_work = work;
            if leaving {
                y += outgoing(ui, |ui| body(ui, k, x, y, w));
            } else {
                y += body(ui, k, x, y, w);
            }
            ui.flow_work = None;
            ui.page_view = None;
        }
    }
    let max = (y + 24.0 - view_h).max(0.0);
    ui.region_close(view_open, max);
    // In screen space: the card's own place less the scroll, kept to the
    // page area under the bar and above any foot.
    let bar_end = if compact { top } else { tokens::APP_BAR_H };
    let (shown_top, shown_end) = ((card_top - next.y).max(bar_end), (y - next.y).min(view_h));
    ui.flow_card = Some((x, shown_top, w, (shown_end - shown_top).max(0.0)));

    if compact {
        bar(ui, page, area_x + area_w);
    } else if let (Some(s), false) = (steps.as_ref(), full) {
        let px = area_x + area_w - pane_w;
        super::choices::Pane {
            app,
            flow,
            steps: s,
            place: (px, pane_w, page.h),
            offset: app.scroll.choices_offset(),
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

/// Draws a card that is only fading out (`Faraday::live_out`): the page
/// takes no press, field or focus from it. Returns its height.
fn outgoing(ui: &mut Ui, draw: impl FnOnce(&mut Ui) -> f32) -> f32 {
    let (presses, fields, cut) = (ui.presses_mark(), ui.fields.len(), ui.cut.len());
    let (focus_box, focus_action) = (ui.focus_box, ui.focus_action);
    let h = draw(ui);
    ui.presses_rewind(presses);
    ui.fields.truncate(fields);
    ui.cut.truncate(cut);
    ui.focus_box = focus_box;
    ui.focus_action = focus_action;
    h
}
