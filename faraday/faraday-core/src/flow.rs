//! The step-card column every flow is drawn with, the way Create vault
//! is in the prototype (`PLAN.md` §9.1): one card open at a time, the
//! others closed to a line that says what was chosen, and the column
//! scrolled so the open card is in view.
//!
//! In Guided mode a card can carry a short written walk-through, shown at
//! the top of the card while it is open (`docs/WALLETS.md` §6). Steps only
//! hides it; the controls are the same in both.

use osk_ui::widgets::Icon;

use crate::Action;
use crate::ui::Disclosure;
use crate::ui::pal::*;
use crate::ui::{Ui, W};

/// One card of a flow.
pub struct Card {
    /// The card's title.
    pub title: String,
    /// The line shown while it is closed.
    pub summary: String,
    /// Set the summary in the mono face: it is data.
    pub mono: bool,
    /// Closed as done.
    pub done: bool,
    /// Open now.
    pub open: bool,
    /// What pressing its header does.
    pub toggle: Action,
    /// The walk-through shown in Guided mode while it is open.
    pub guide: Option<String>,
}

/// Where the column is scrolled, and whether to bring the open card into
/// view on this frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scroll {
    /// Design units scrolled.
    pub y: f32,
    /// The open card changed.
    pub follow: bool,
}

/// What a column drawing needs besides its cards.
pub struct Column<'a> {
    /// The left edge of the area the column may draw and clip in.
    pub area_x: f32,
    /// That area's width.
    pub area_w: f32,
    /// The column's own left edge.
    pub x: f32,
    /// The column's width.
    pub w: f32,
    /// The window's height in design units.
    pub h: f32,
    /// The back link above the title.
    pub back: Option<(&'a str, Action)>,
    /// The title.
    pub heading: &'a str,
    /// Whether Guided mode is on.
    pub guided: bool,
    /// Whether the Guided/Full switch is shown beside the title.
    pub switch: bool,
    /// A line under the title, about the page as a whole.
    pub note: Option<&'a str>,
}

/// Draws the cards. `body` draws the open card's controls at (x, y) in a
/// width and returns their height. Returns the scroll as it should be
/// next, and whether the frame must be drawn again because it moved.
pub fn column(
    ui: &mut Ui,
    col: &Column,
    cards: &[Card],
    scroll: Scroll,
    body: &mut dyn FnMut(&mut Ui, usize, f32, f32, f32) -> f32,
) -> (Scroll, bool) {
    column_foot(ui, col, cards, scroll, body, None)
}

/// What is drawn under a column's last card: at (x, y) in a width,
/// returning its height.
pub type Foot<'a> = &'a mut dyn FnMut(&mut Ui, f32, f32, f32) -> f32;

/// [`column`], with `foot` drawn under the last card at (x, y) in the
/// column's width, returning its height: what comes after the flow.
pub fn column_foot(
    ui: &mut Ui,
    col: &Column,
    cards: &[Card],
    scroll: Scroll,
    body: &mut dyn FnMut(&mut Ui, usize, f32, f32, f32) -> f32,
    foot: Option<Foot>,
) -> (Scroll, bool) {
    let clip = ui.rect(col.area_x, 0.0, col.area_w, col.h);
    ui.c.push_clip(clip);
    let top = 28.0 - scroll.y;
    let mut y = top;
    let (x, w) = (col.x, col.w);
    if let Some((label, action)) = col.back {
        ui.icon(x - 4.0, y, 16.0, Icon::ChevronLeft, 10.0, MUTED);
        let lw = ui.text(x + 14.0, y, 12.0, W::R, MUTED, label);
        ui.hit(x - 4.0, y - 4.0, lw + 30.0, 24.0, action);
        y += 22.0;
    }
    ui.text(x, y, 26.0, W::S, TEXT, col.heading);
    // Guided or Steps only, beside the title.
    let mut bx = x + w;
    let switch: &[(&str, bool, bool)] = if col.switch {
        &[
            ("Steps only", !col.guided, false),
            ("Guided", col.guided, true),
        ]
    } else {
        &[]
    };
    // Where each option is, then the pill, sliding between them, then
    // the labels over it.
    let mut places = Vec::new();
    for &(label, on, guided) in switch {
        let bw = ui.measure(12.0, W::S, label) + 24.0;
        bx -= bw;
        places.push((bx, bw, label, on, guided));
        bx -= 4.0;
    }
    if let [(sx, sw, ..), (gx, gw, ..)] = places[..] {
        let t = ui.guided_shown;
        ui.fill(
            sx + (gx - sx) * t,
            y + 2.0,
            sw + (gw - sw) * t,
            30.0,
            8.0,
            INNER,
        );
    }
    for (bx, bw, label, on, guided) in places {
        ui.text_mid(
            bx + 12.0,
            y + 2.0,
            30.0,
            12.0,
            W::S,
            if on { TEXT } else { MUTED },
            label,
        );
        ui.hit(bx, y + 2.0, bw, 30.0, Action::Guided(guided));
    }
    y += 52.0;
    if let Some(note) = col.note {
        let nh = ui.wrap(x + 14.0, y, w - 14.0, 13.0, W::R, MUTED, note);
        ui.fill(x, y, 3.0, nh, 1.5, ACCENT.with_alpha(140));
        y += nh + 20.0;
    }

    let inner_x = x + 58.0;
    let inner_w = w - 76.0;
    let mut open_top = None;
    let mut open_body = None;
    // A card opening grows and the one closing shrinks
    // (`docs/MOTION.md` §3.5); what follows them moves with them. Where
    // everything will be once they are done is what the page measures.
    let disclosure = ui.disclosure;
    let mut settled = 0.0;
    for (i, card) in cards.iter().enumerate() {
        let card_top = y;
        if card.open {
            open_top = Some(y + settled - top);
        }
        let guide = card.guide.as_deref().filter(|_| col.guided && card.open);
        // Measured in a pass that draws nothing and keeps no hits, so the
        // card's surface can go down before what is on it.
        let mut draw_open = |ui: &mut Ui| -> f32 {
            let mut used = 0.0;
            if let Some(text) = guide {
                let gh = ui.wrap(
                    inner_x + 14.0,
                    y + 56.0,
                    inner_w - 14.0,
                    14.0,
                    W::R,
                    TEXT,
                    text,
                );
                ui.fill(inner_x, y + 56.0, 3.0, gh, 1.5, ACCENT.with_alpha(140));
                used += gh + 18.0;
            }
            used + body(ui, i, inner_x, y + 56.0 + used, inner_w)
        };
        let body_h = if card.open {
            let mark = ui.hits.len();
            ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
            let bh = draw_open(ui);
            ui.c.pop_clip();
            ui.hits.truncate(mark);
            bh
        } else {
            0.0
        };
        let full_h = 56.0 + if card.open { body_h + 18.0 } else { 0.0 };
        let card_h = match disclosure {
            Some(d) if card.open && d.open == Some(i) => 56.0 + (body_h + 18.0) * d.shown,
            Some(Disclosure {
                closing: Some((c, h)),
                shown,
                ..
            }) if c == i && !card.open => 56.0 + (h + 18.0) * (1.0 - shown),
            _ => full_h,
        };
        settled += full_h - card_h;
        if card.open {
            open_body = Some(body_h);
        }
        let edge = if card.open {
            ACCENT.with_alpha(100)
        } else {
            LINE
        };
        ui.card(x, card_top, w, card_h, edge);
        if card.open && card_h < full_h {
            let rect = ui.rect(x, card_top, w, card_h);
            ui.reveal(rect);
            draw_open(ui);
            ui.unreveal();
        } else if card.open {
            draw_open(ui);
        }
        ui.badge(
            x + 18.0,
            card_top + 15.0,
            &(i + 1).to_string(),
            card.done,
            card.open,
        );
        ui.text_mid(x + 58.0, card_top, 56.0, 15.0, W::S, TEXT, &card.title);
        if !card.open {
            let tw = ui.measure(15.0, W::S, &card.title);
            let room = w - 58.0 - tw - 60.0;
            let face = if card.mono { W::M } else { W::R };
            let s = ui.fit(13.0, face, &card.summary, room);
            ui.text_right(x + w - 46.0, card_top, 56.0, 13.0, face, MUTED, &s);
        }
        ui.icon(
            x + w - 38.0,
            card_top + 18.0,
            20.0,
            if card.open {
                Icon::ChevronUp
            } else {
                Icon::ChevronRight
            },
            10.0,
            DIM,
        );
        ui.hit(x, card_top, w, 56.0, card.toggle);
        y += card_h + 10.0;
    }
    let mut foot_top = None;
    if let Some(foot) = foot {
        foot_top = Some(y - top);
        y += 6.0 + foot(ui, x, y + 6.0, w);
    }
    let content_h = y + settled - top + 24.0;
    ui.c.pop_clip();
    ui.column = Some((cards.iter().position(|c| c.open), open_body.unwrap_or(0.0)));

    let mut next = scroll;
    let mut again = false;
    let max = (content_h - col.h).max(0.0);
    ui.report_scroll(clip, max);
    if next.follow {
        next.follow = false;
        // With every card closed, the foot is what comes into view.
        if let Some(t) = open_top.or(foot_top) {
            let want = (t - 100.0).clamp(0.0, max);
            if (want - next.y).abs() > 0.5 {
                // It glides there.
                ui.follow_to = Some(want);
            }
        }
    }
    if next.y > max {
        next.y = max;
        again = true;
    }
    (next, again)
}
