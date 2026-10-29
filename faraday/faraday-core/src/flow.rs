//! The step-card column every flow is drawn with, the way Create vault
//! is in the prototype (`docs/history/PLAN.md` §9.1): one card open at a time, the
//! others closed to a line that says what was chosen, and the column
//! scrolled so the open card is in view. A card is a flat section on the
//! page divided from the next by a rule, with no fill and no box
//! (`docs/DECISIONS.md` F39).
//!
//! In Guided mode a card can carry a short written walk-through, shown at
//! the top of the card while it is open (`docs/WALLETS.md` §6). Steps only
//! hides it; the controls are the same in both.

use crate::strings::EN;
use crate::strings::fill;
use crate::ui::Shape;
use faraday_draw::widgets::Icon;

use crate::Action;
use crate::ui::Disclosure;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens;
use faraday_draw::tokens::{
    BODY_M, BODY_S, LABEL_L, LABEL_M, MARGIN_COMPACT, SEGMENTED_H, TITLE_M,
};

/// How far in from a step's left edge its body is drawn, under its title.
pub const BODY_INDENT: f32 = 58.0;
/// How far in from a step's right edge its body stops.
pub const BODY_RIGHT: f32 = 18.0;

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
    /// The card's value has a default, so a flow may open closed on it
    /// at entry; closed, it draws a **Change** text button at its right
    /// in place of the chevron.
    pub default: bool,
    /// What pressing its header does.
    pub toggle: Action,
    /// The walk-through shown in Guided mode while it is open.
    pub guide: Option<String>,
    /// The Learn page under the walk-through, one press away.
    pub learn: Option<crate::learn::Topic>,
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
    /// The way back, on the top app bar over the flow.
    pub back: Option<(&'a str, Action)>,
    /// The flow's name, on the top app bar over it.
    pub heading: &'a str,
    /// Whether Guided mode is on.
    pub guided: bool,
    /// Whether Steps only · Guided is shown at the head of the column.
    pub switch: bool,
    /// A line at the head of the column, about the page as a whole.
    pub note: Option<&'a str>,
    /// A chip at the head of the column: where the flow goes next, or
    /// what it works on.
    pub chip: Option<&'a str>,
    /// What a press on the chip does; where it was drawn is left in
    /// [`Ui::chip_at`].
    pub chip_tap: Option<Action>,
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
    column_wide(ui, col, None, cards, scroll, body, foot)
}

/// [`column_foot`] with the open card holding wide content that runs on
/// to `wide_end` (`docs/LAYOUT.md` §14): the rules above and under that
/// card run to `wide_end`; its head and summary stay in the column.
/// `None` draws every rule at the column's width.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn column_wide(
    ui: &mut Ui,
    col: &Column,
    wide_end: Option<f32>,
    cards: &[Card],
    scroll: Scroll,
    body: &mut dyn FnMut(&mut Ui, usize, f32, f32, f32) -> f32,
    foot: Option<Foot>,
) -> (Scroll, bool) {
    if ui.compact {
        return paged(ui, col, cards, scroll, body, foot);
    }
    let clip = ui.rect(col.area_x, 0.0, col.area_w, col.h);
    ui.c.push_clip(clip);
    let top = tokens::PAGE_TOP_BAR - scroll.y;
    let mut y = top;
    let (x, w) = (col.x, col.w);
    // The top app bar over the page names the flow and carries its way
    // back and Cancel (`docs/LAYOUT.md` §3, DESIGN §4.1, §4.13); the flow's own
    // head keeps what is the flow's: its chip at the left and Steps only ·
    // Guided at the right.
    crate::screens::bar_head(ui, col.back, col.heading);
    let mut bx = x + w;
    let mut row = 0.0;
    if col.switch {
        bx -= guided_width(ui);
        guided_choice(ui, (bx, y), col.guided);
        row = SEGMENTED_H;
    }
    // The chip beside the switch where it clears it, else on its own line
    // under it.
    if let Some(chip) = col.chip {
        let (cx, cy) = if x + ui.chip_width(chip) + tokens::GAP <= bx {
            row = f32::max(row, tokens::TAG_H);
            (x, y + (row - tokens::TAG_H) / 2.0)
        } else {
            let cy = y + row + tokens::GAP;
            row += tokens::GAP + tokens::TAG_H;
            (x, cy)
        };
        let cw = ui.chip(
            cx,
            cy,
            chip,
            Role::OnSecondaryContainer,
            Role::SecondaryContainer,
        );
        if let Some(a) = col.chip_tap {
            let h = tokens::TAG_H;
            ui.press(
                Shape::new(cx, cy, cw, h, tokens::shape_full(h)),
                Role::OnSurface.paint(),
                a,
            );
            ui.chip_at = Some((cx, cy + h, cw));
        }
    }
    if row > 0.0 {
        y += row + tokens::PAD;
    }
    if let Some(note) = col.note {
        let nh = ui.wrap_as(
            (x + 14.0, y, w - 14.0),
            BODY_S,
            Role::OnSurfaceVariant,
            note,
        );
        ui.fill(x, y, 3.0, nh, 1.5, Role::Primary.paint());
        y += nh + 20.0;
    }

    let inner_x = x + BODY_INDENT;
    let inner_w = w - BODY_INDENT - BODY_RIGHT;
    let mut open_top = None;
    let mut open_body = None;
    // A card opening grows and the one closing shrinks
    // (`docs/MOTION.md` §3.5); what follows them moves with them. Where
    // everything will be once they are done is what the page measures.
    let disclosure = ui.disclosure;
    let mut settled = 0.0;
    let open_rule_w = wide_end.map_or(w, |end| (end - x).max(w));
    let mut after_open = false;
    for (i, card) in cards.iter().enumerate() {
        let card_top = y;
        if card.open {
            open_top = Some(y + settled - top);
        }
        let guide = card.guide.as_deref().filter(|_| col.guided && card.open);
        let learn = card.learn.filter(|_| guide.is_some());
        // Measured in a pass that draws nothing and keeps no hits, so the
        // card's surface can go down before what is on it.
        let mut draw_open = |ui: &mut Ui| -> f32 {
            let mut used = 0.0;
            if let Some(text) = guide {
                used += guide_text(ui, (inner_x, y + 56.0, inner_w), text);
                if let Some(t) = learn {
                    used += learn_link(ui, inner_x, y + tokens::GUIDE_CARD_HEAD + used, inner_w, t);
                }
                used += 18.0;
            }
            used + body(ui, i, inner_x, y + 56.0 + used, inner_w)
        };
        let body_h = if card.open {
            let mark = ui.presses_mark();
            ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
            let bh = draw_open(ui);
            ui.c.pop_clip();
            ui.presses_rewind(mark);
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
        // A step is a flat section on the page, a rule above it
        // (`docs/DECISIONS.md` F39): no fill and no box. The open card's
        // rules, above it and under it, run as wide as what it holds.
        let rule_w = if card.open || after_open {
            open_rule_w
        } else {
            w
        };
        after_open = card.open;
        ui.rule(x, card_top, rule_w, Role::OutlineVariant);
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
        ui.text_mid_as(
            (x + 58.0, card_top, 56.0),
            TITLE_M,
            Role::OnSurface,
            &card.title,
        );
        // A closed card with a default shows a Change text button at the
        // right, in place of the chevron, over the value it is set to.
        let change = !card.open && card.default;
        let change_w = if change {
            ui.measure_as(LABEL_L, EN.flow_change)
        } else {
            0.0
        };
        if !card.open {
            let tw = ui.measure_as(TITLE_M, &card.title);
            let right_reserved = if change { 28.0 + change_w } else { 60.0 };
            let room = w - 58.0 - tw - right_reserved;
            let role = if card.mono { BODY_S.mono() } else { BODY_S };
            let s = ui.fit_as(role, &card.summary, room);
            let edge = if change {
                x + w - 18.0 - change_w - 10.0
            } else {
                x + w - 46.0
            };
            ui.text_right_as((edge, card_top, 56.0), role, Role::OnSurfaceVariant, &s);
        }
        if change {
            ui.text_right_as(
                (x + w - 18.0, card_top, 56.0),
                LABEL_L,
                Role::Primary,
                EN.flow_change,
            );
        } else {
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
                Role::OnSurfaceVariant,
            );
        }
        ui.press_box(x, card_top, w, 56.0, card.toggle);
        y += card_h;
    }
    // The rule under the last step closes the list of them.
    if !cards.is_empty() {
        let rule_w = if after_open { open_rule_w } else { w };
        ui.rule(x, y, rule_w, Role::OutlineVariant);
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
    if past_end(next.y, max) {
        next.y = max;
        again = true;
    }
    (next, again)
}

/// Whether a page's offset `y` is past its end `max`, both in design
/// units. The frame rounds an offset to whole pixels at or under the end
/// (`Faraday::whole_offset`), which may be a hair over `max` as a float:
/// that is not past it, or the page would clamp and draw again for ever.
pub(crate) fn past_end(y: f32, max: f32) -> bool {
    y > max + 1e-3
}

/// The step flow on a small panel: the open step is the page. A bar
/// back, the flow's name, a strip of the steps to go between them, then
/// the step's controls; with no step open, the steps as a list, and the
/// foot under them.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn paged(
    ui: &mut Ui,
    col: &Column,
    cards: &[Card],
    scroll: Scroll,
    body: &mut dyn FnMut(&mut Ui, usize, f32, f32, f32) -> f32,
    foot: Option<Foot>,
) -> (Scroll, bool) {
    use crate::compact::{BAR_H, M};
    let w = col.area_x + col.area_w;
    let open = cards.iter().position(|c| c.open);
    // A step just opened starts at its top.
    let mut next = scroll;
    if next.follow {
        next.follow = false;
        next.y = 0.0;
    }
    let clip = ui.rect(0.0, BAR_H, w, col.h - BAR_H);
    ui.c.push_clip(clip);
    let top = BAR_H - next.y;
    let mut y = top + 10.0;
    let inner = w - 2.0 * M;
    if let Some(chip) = col.chip {
        let chip = ui.fit_as(BODY_S, chip, inner - 34.0);
        let cw = ui.chip(
            M,
            y,
            &chip,
            Role::OnSecondaryContainer,
            Role::SecondaryContainer,
        );
        if let Some(a) = col.chip_tap {
            ui.press(
                Shape::new(M, y, cw, 26.0, tokens::shape_full(26.0)),
                Role::OnSurface.paint(),
                a,
            );
            ui.chip_at = Some((M, y + 26.0, cw));
        }
        y += 36.0;
    }

    match open {
        Some(i) => {
            // Each earlier card closed on its default: its value, and
            // Change, which opens it.
            let mut defaults = false;
            for c in cards[..i].iter().filter(|c| c.default && !c.open) {
                let cw = ui.measure_as(LABEL_L, EN.flow_change);
                let line = format!("{} · {}", c.title, c.summary);
                let role = if c.mono { BODY_S.mono() } else { BODY_S };
                let line = ui.fit_as(role, &line, inner - cw - 16.0);
                ui.text_mid_as((M, y, 34.0), role, Role::OnSurfaceVariant, &line);
                ui.text_right_as((M + inner, y, 34.0), LABEL_L, Role::Primary, EN.flow_change);
                ui.press_box(M, y, inner, 34.0, c.toggle);
                y += 36.0;
                defaults = true;
            }
            if defaults {
                ui.rule(M, y, inner, Role::OutlineVariant);
                y += 12.0;
            }
            // The title says where the page is; tapping it closes the
            // card, which leaves the list of steps, any of which opens.
            // The step count sits beside it, muted, so it is never read
            // as a value next to the card's own numbers (`docs/history/SIMPLIFY.md`
            // §2.6).
            let card = &cards[i];
            let count = fill(
                EN.flow_step_count,
                &[&(i + 1).to_string(), &cards.len().to_string()],
            );
            let count_w = ui.measure_as(BODY_S, &count);
            let title_w = inner - count_w - 10.0;
            let lh = ui.line_as(TITLE_M);
            let th = ui.wrap_as((M, y, title_w), TITLE_M, Role::OnSurface, &card.title);
            ui.text_right_as((M + inner, y, lh), BODY_S, Role::OnSurfaceVariant, &count);
            ui.press_box(M, y, inner, th, card.toggle);
            y += th + 10.0;
            y += body(ui, i, M, y, inner);
            // The walk-through comes after the controls, behind a tap:
            // on a small panel it would otherwise take the room the
            // controls need. The tap is the choice there, so it is offered
            // in Steps only too. A flow with no Guided switch (the Spend
            // tab) has a lead that is its own text, always shown.
            if let Some(text) = card.guide.as_deref() {
                if col.switch {
                    let label = EN.flow_about_this_step;
                    y += about(ui, M, y, inner, label, i as u8, (text, card.learn));
                } else {
                    y += 8.0;
                    let gh = ui.wrap_as(
                        (M + 12.0, y, inner - 12.0),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        text,
                    );
                    ui.fill(M, y, 3.0, gh, 1.5, Role::Primary.paint());
                    y += gh + 8.0;
                }
            }
        }
        None => {
            if let Some(note) = col.note {
                let nh = ui.wrap_as(
                    (M + 12.0, y, inner - 12.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    note,
                );
                ui.fill(M, y, 3.0, nh, 1.5, Role::Primary.paint());
                y += nh + 14.0;
            }
            for (i, card) in cards.iter().enumerate() {
                // Each step a row on the page, a rule above it (F39).
                ui.rule(M, y, inner, Role::OutlineVariant);
                ui.badge(M + 10.0, y + 13.0, &(i + 1).to_string(), card.done, false);
                let tw = inner - 70.0;
                let title = ui.fit_as(LABEL_L, &card.title, tw);
                ui.text_as(M + 46.0, y + 8.0, LABEL_L, Role::OnSurface, &title);
                let role = if card.mono { BODY_S.mono() } else { BODY_S };
                let summary = ui.fit_as(role, &card.summary, tw);
                ui.text_as(M + 46.0, y + 29.0, role, Role::OnSurfaceVariant, &summary);
                ui.icon(
                    M + inner - 24.0,
                    y + 16.0,
                    20.0,
                    Icon::ChevronRight,
                    9.0,
                    Role::OnSurfaceVariant,
                );
                ui.press_box(M, y, inner, 52.0, card.toggle);
                y += 52.0;
            }
            if !cards.is_empty() {
                ui.rule(M, y, inner, Role::OutlineVariant);
                y += 8.0;
            }
            if let Some(foot) = foot {
                y += 6.0 + foot(ui, M, y + 6.0, inner);
            }
        }
    }
    let content_h = y - top + 24.0;
    ui.c.pop_clip();
    ui.column = None;

    // The bar goes over whatever has scrolled under it.
    let back = col
        .back
        .unwrap_or((EN.flow_home, Action::Nav(crate::Screen::Home)));
    // Material's small top app bar, as every page's (`compact::bar`):
    // the ?, the keyboard and Cancel go over its right end
    // (`compact::flow_bar_tools`).
    let head = faraday_ui::nav::AppBar {
        back: Some(back),
        title: col.heading,
        back_tip: EN.tip_back,
    };
    let right = ui.bar_tools_x.unwrap_or(w - MARGIN_COMPACT);
    faraday_ui::nav::app_bar(ui, w, &head, right);

    let max = (content_h - (col.h - BAR_H)).max(0.0);
    ui.report_scroll(clip, max);
    let mut again = false;
    if past_end(next.y, max) {
        next.y = max;
        again = true;
    }
    (next, again || scroll.follow)
}

/// A walk-through behind a tap, after a small panel's controls: a row
/// that opens it and closes it, and the text while open. `k` is the
/// step's place, or [`crate::ABOUT_PAGE`]. Returns its height.
pub(crate) fn about(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    k: u8,
    (text, learn): (&str, Option<crate::learn::Topic>),
) -> f32 {
    let open = ui.about_open == Some(k);
    let action = Action::About(k);
    let mut cy = y + 8.0;
    ui.rule(x, cy, w, Role::OutlineVariant);
    cy += 6.0;
    ui.icon(x, cy + 8.0, 20.0, Icon::Info, 11.0, Role::OnSurfaceVariant);
    ui.text_mid_as((x + 26.0, cy, 36.0), LABEL_M, Role::OnSurfaceVariant, label);
    ui.icon(
        x + w - 22.0,
        cy + 8.0,
        20.0,
        if open {
            Icon::ChevronUp
        } else {
            Icon::ChevronRight
        },
        9.0,
        Role::OnSurfaceVariant,
    );
    ui.press_box(x, cy, w, 36.0, action);
    cy += 42.0;
    if open {
        let gh = ui.wrap_as(
            (x + 12.0, cy, w - 12.0),
            BODY_S,
            Role::OnSurfaceVariant,
            text,
        );
        ui.fill(x, cy, 3.0, gh, 1.5, Role::Primary.paint());
        cy += gh;
        if let Some(t) = learn {
            cy += learn_link(ui, x, cy, w, t);
        }
        cy += 8.0;
    }
    cy - y
}

/// A Guided text's link to its Learn page: a ghost button under the text,
/// labelled with the page's title, as the backup's Extra copies line has.
/// Returns its height with the gap above it.
pub(crate) fn learn_link(ui: &mut Ui, x: f32, y: f32, w: f32, topic: crate::learn::Topic) -> f32 {
    let label = topic.label();
    let bw = (ui.measure_as(LABEL_L, label) + tokens::GUIDE_LINK_PAD).min(w);
    ui.button(
        x,
        y + tokens::GUIDE_LINK_ABOVE,
        Some(bw),
        tokens::GUIDE_LINK_H,
        label,
        crate::ui::Style::Ghost,
        Action::LearnTopic(topic),
    );
    tokens::GUIDE_LINK_ABOVE + tokens::GUIDE_LINK_H
}

/// The width of [`guided_choice`]. Arithmetic.
pub(crate) fn guided_width(ui: &Ui) -> f32 {
    ui.segment_width(LABEL_L.size, &[EN.flow_steps_only, EN.flow_guided]) * 2.0
}

/// Steps only or Guided from (x, y): Material's segmented button, its
/// fill sliding between them as the setting changes. A flow's head and
/// Settings draw this one control for the one setting.
pub(crate) fn guided_choice(ui: &mut Ui, (x, y): (f32, f32), guided: bool) {
    let options = [
        (EN.flow_steps_only, Action::Guided(false)),
        (EN.flow_guided, Action::Guided(true)),
    ];
    let chosen = Some(usize::from(guided));
    let slide = Some(ui.guided_shown);
    ui.segmented((x, y, SEGMENTED_H), LABEL_L.size, &options, chosen, slide);
}

/// A Guided text from (x, y), `w` wide: its words in body text beside a
/// bar in the accent, as a flow's open card and Settings' cards show it.
/// Returns its height.
pub(crate) fn guide_text(ui: &mut Ui, (x, y, w): (f32, f32, f32), text: &str) -> f32 {
    use tokens::{GUIDE_BAR, GUIDE_INDENT};
    let tx = x + GUIDE_INDENT;
    let gh = ui.wrap_as((tx, y, w - GUIDE_INDENT), BODY_M, Role::OnSurface, text);
    let bar = Role::Primary.paint();
    ui.fill(x, y, GUIDE_BAR, gh, GUIDE_BAR / 2.0, bar);
    gh
}
