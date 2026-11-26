//! The flow container's page: the current step's card at one fixed place,
//! scrolled within the page area, with Your choices at its right on the
//! wide layout (`docs/FLOW-CONTAINER.md` §2, §4).

use super::{Contained, Foot, Room, Scroll, past_end};
use crate::screens::hub_layout;
use crate::strings::{EN, fill};
use crate::ui::{Moves, Region, Ui, pal::*};
use crate::{Action, Faraday};
use faraday_draw::tokens::{self, BODY_S, COLUMN_MAX, MARGIN_COMPACT, MARGIN_WIDE, TITLE_M};

/// A foot the flow declares for Your choices' pane: its height and its
/// drawing at (x, y) in a width. The pane draws it pinned at its foot with a
/// hairline over it; the rows and what the flow gives under them scroll above.
pub(crate) struct Pin<'a> {
    /// The drawing's height.
    pub(crate) h: f32,
    /// Draws it at (x, y) in a width.
    pub(crate) draw: &'a dyn Fn(&mut Ui, f32, f32, f32),
}

/// What the container needs of the screen besides the flow's declaration.
pub(crate) struct Page<'a> {
    /// The page area's left edge and width.
    pub(crate) area: (f32, f32),
    /// The window's height in design units.
    pub(crate) h: f32,
    /// What a panel at the page's foot takes of that height.
    pub(crate) foot_h: f32,
    /// What the flow keeps at the foot of Your choices' pane, outside what
    /// scrolls there.
    pub(crate) pin: Option<Pin<'a>>,
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

/// Whether the pair stands with its pane at the card's left in a page
/// `area_w` wide: a wide layout, a step that is not full width, and room
/// for the pair. A step with a pick ([`Contained::pick`]) draws no rows in
/// its card where this is true: the pane holds them. A lookup.
pub(crate) fn pair_left(app: &Faraday, flow: &dyn Contained, compact: bool, area_w: f32) -> bool {
    !compact
        && !flow.full_width(app)
        && app.unit_on_show()
        && app.unit_place(area_w).is_some_and(|u| u.pane_left())
}

/// The open step's pick, where the pane stands (`stands`).
fn open_pick(
    app: &Faraday,
    flow: &dyn Contained,
    steps: Option<&crate::trail::Steps>,
    stands: bool,
) -> Option<super::Pick> {
    flow.pick(app, steps?.open.filter(|_| stands)?)
}

/// The open step's take-away, where the pane stands (`stands`).
fn open_take(
    app: &Faraday,
    flow: &dyn Contained,
    steps: Option<&crate::trail::Steps>,
    stands: bool,
) -> Option<super::TakeAway> {
    flow.take_away(app, steps?.open.filter(|_| stands)?)
}

/// The wallet or key row that leads Your choices, where the pane stands
/// (`stands`) and neither a pick nor a take-away takes its place.
fn lead_row(
    app: &Faraday,
    flow: &dyn Contained,
    stands: bool,
    (pick, take): (&Option<super::Pick>, &Option<super::TakeAway>),
) -> Option<(super::Row, Option<crate::Action>)> {
    (stands && pick.is_none() && take.is_none())
        .then(|| flow.pick_row(app))
        .flatten()
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
    // A column card and Your choices stand as one unit, centred in the room
    // (`screens::hub_layout`); the rest keep the pane at the area's right edge.
    let unit = (!compact && app.unit_on_show())
        .then(|| app.unit_place(area_w))
        .flatten();
    let pane_w = if compact || full {
        0.0
    } else if let Some(u) = unit {
        u.pane_w
    } else {
        super::act::PANES.pane_beside_work(area_w)
    };
    // The pair stands with Your choices at the left of the card when the
    // room holds it; the card's room then runs from the divider to the
    // pair's end and a spacer.
    let left = unit.is_some_and(|u| u.pane_left());
    let room_w = match unit {
        Some(u) if left => u.card_x + u.card_w + u.gap,
        Some(u) => u.pane_x,
        None => area_w - pane_w,
    };
    let clip_x = match unit {
        Some(u) if left => area_x + u.card_x - u.gap / 2.0,
        _ => area_x,
    };
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

    // The flow's one card, raised (F331): the pair, and on a window with a
    // rail the card of a flow with one column.
    let raised = !compact && steps.is_some() && (left || (full && crate::screens::in_flow(app)));

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
        let w = unit.map_or((room_w - 2.0 * MARGIN_WIDE).min(COLUMN_MAX), |u| u.card_w);
        // Wide content stands at the navigation's edge, not centred.
        let x = if let Some(u) = unit {
            area_x + u.card_x
        } else if aligned {
            area_x + MARGIN_WIDE
        } else {
            area_x + (room_w - w) / 2.0
        };
        // A raised card stands a padding above its content.
        let lower = f32::from(u8::from(raised)) * tokens::PAD / 2.0;
        (tokens::PAGE_TOP_BAR + lower, x, w)
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
    // Where the pair stands, its outline's top and foot are the card's
    // limits: nothing scrolls past them.
    let (clip_top, clip_end) = hub_layout::limits(left || raised, compact, top, view_h);
    let columns = unit
        .filter(|_| left)
        .map_or((x, work.unwrap_or(x + w)), |u| (area_x + u.pane_x, x + w));
    let card = RaisedCard::draw(ui, app, raised.then_some(columns), (top, clip_end));
    let seen = (clip_x, area_x + room_w);
    let (seen, tail) = card.view(seen, (clip_top, clip_end));
    let (view_x, view_y, view_w, view_rows) = seen;
    let clip = ui.rect(view_x, view_y, view_w, view_rows);
    let region = Region::page(clip).from_zero(next.y).moves(page.moves);
    let view_open = ui.region_open(card.under(region));
    let mut y = if compact { top + 10.0 } else { top };
    if !compact {
        // The top app bar over the page names the flow and carries its way
        // back and Cancel; the card keeps what is the flow's: its chip.
        crate::screens::bar_head(ui, page.back, page.heading);
    }
    let card_top = y;
    // Where the pair stands, a chip naming the wallet or key is Your
    // choices' first row instead.
    // The open step's pick takes the pane's place: the rows at the left,
    // the card holding Next. A step fading out keeps Your choices.
    let pick = open_pick(app, flow, steps.as_ref(), left && !full && out.is_none());

    // The last step's take-away takes the pane's place too.
    let take = open_take(app, flow, steps.as_ref(), left && !full && out.is_none());

    let lead = lead_row(app, flow, left && !full && steps.is_some(), (&pick, &take));
    if let (Some(chip), None) = (page.chip, lead.as_ref()) {
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
            // The same line whether the card is raised or not: `view` keeps
            // under the content what its flat part leaves out of it.
            ui.flow_end = Some(clip_end - tokens::FLOW_END_PAD);
            if leaving {
                y += outgoing(ui, |ui| body(ui, k, x, y, w));
            } else {
                y += body(ui, k, x, y, w);
            }
            ui.flow_work = None;
            ui.flow_end = None;
            ui.page_view = None;
        }
    }
    let max = (y + tail - (view_y + view_rows)).max(0.0);
    ui.region_close(view_open, max);
    // In screen space: the card's own place less the scroll, kept to the
    // page area under the bar and above any foot.
    let bar_end = if compact { top } else { tokens::APP_BAR_H };
    let (shown_top, shown_end) = ((card_top - next.y).max(bar_end), (y - next.y).min(view_h));
    ui.flow_card = Some((x, shown_top, w, (shown_end - shown_top).max(0.0)));
    ui.flow_card_raised = card.drawn.is_some();

    // Level with the card where the pair stands side by side.
    let top = if left {
        card_top
    } else {
        tokens::SIDE_PANEL_TOP
    };
    // How far down Your choices reaches, where it is the card's left side.
    let mut pane_end = 0.0;
    if compact {
        bar(ui, page, area_x + area_w);
    } else if let (Some(s), false) = (steps.as_ref(), full) {
        let px = match unit {
            Some(u) => area_x + u.pane_x,
            None => area_x + area_w - pane_w,
        };
        let pane_h = super::choices::Pane {
            app,
            flow,
            steps: s,
            lead: lead.as_ref(),
            pick: pick.as_ref(),
            take: take.as_ref(),
            pin: page.pin.as_ref(),
            place: (px, pane_w, if left { clip_end } else { page.h }),
            top,
            offset: app.scroll.choices_offset(),
        }
        .draw(ui, foot);
        // Where the pair stands, Your choices is the session pane's place
        // for the change of page: the pane glides into it and back.
        if left {
            ui.pane = Some((px, top, pane_w, pane_h.min(clip_end - top).max(0.0)));
        }
        let gap = unit.map_or(tokens::SPACER, |u| u.gap);
        let at = if left {
            px + pane_w + gap / 2.0
        } else {
            px - gap / 2.0
        };
        let end = hub_layout::level(top + pane_h, y - next.y, view_h);
        hub_layout::divider(ui, (at, top, end));
        pane_end = top + pane_h;
    }
    card.settle(ui, app, (y, pane_end), max > 0.0, (view_h, clip_end));

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

/// A flow's raised card as drawn: it stands under the content, which alone
/// says where it ends, so it is drawn to where it ended last, and a frame
/// that finds another end is drawn again before it is shown.
struct RaisedCard {
    /// Where its foot was drawn, and its left and right edges; none where
    /// no card is raised.
    drawn: Option<(f32, f32, f32)>,
}

impl RaisedCard {
    /// Draws it, where one is raised, round the columns from `from` to
    /// `to`, a padding outside them, for content that starts at `top`.
    fn draw(
        ui: &mut Ui,
        app: &Faraday,
        columns: Option<(f32, f32)>,
        (top, clip_end): (f32, f32),
    ) -> Self {
        let drawn = columns.map(|(from, to)| {
            let foot = app.scroll.card_foot().get();
            let (left, right) = (from - tokens::PAD, to + tokens::PAD);
            hub_layout::card(ui, (left, top - tokens::PAD), (right, foot.min(clip_end)));
            (foot, left, right)
        });
        RaisedCard { drawn }
    }

    /// The region the content scrolls in, between `seen.0` and `seen.1`
    /// across and from `top` to `clip_end`, and the room kept under the
    /// content. Under a raised card it lies in the card's flat part: from
    /// edge to edge at most, and a corner's radius inside its top and
    /// foot, so a scroll step that moves the rows drawn moves nothing of
    /// the card's shape.
    fn view(
        &self,
        (from, to): (f32, f32),
        (top, clip_end): (f32, f32),
    ) -> ((f32, f32, f32, f32), f32) {
        let Some((_, left, right)) = self.drawn else {
            return ((from, top, to - from, clip_end - top), tokens::FLOW_END_PAD);
        };
        let (from, to) = (from.max(left), to.min(right));
        let top = top - tokens::PAD / 2.0 + tokens::SHAPE_L;
        let end = clip_end - tokens::SHAPE_L;
        let tail = tokens::FLOW_END_PAD - tokens::SHAPE_L;
        ((from, top, to - from, end - top), tail)
    }

    /// `region` as it stands on the card: its edges fade into the card.
    fn under(&self, region: Region) -> Region {
        match self.drawn {
            Some(_) => region
                .on(Role::SurfaceContainerLow)
                .fades_into(Role::SurfaceContainerLow),
            None => region,
        }
    }

    /// After the content, which ends at `y` beside a pane that ends at
    /// `pane_end`: keeps where the card ends. A card whose content scrolls
    /// runs to its limit, so that a scroll step leaves its foot where it
    /// is.
    fn settle(
        &self,
        ui: &mut Ui,
        app: &Faraday,
        (y, pane_end): (f32, f32),
        scrolls: bool,
        (view_h, clip_end): (f32, f32),
    ) {
        let Some((was, ..)) = self.drawn else {
            return;
        };
        let foot = if scrolls {
            clip_end
        } else {
            (hub_layout::level(pane_end, y, view_h) + tokens::PAD).min(clip_end)
        };
        if (foot - was).abs() > 0.1 {
            app.scroll.card_foot().set(foot);
            ui.spoiled = true;
        }
    }
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
