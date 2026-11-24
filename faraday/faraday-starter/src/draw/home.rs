//! The three destinations' pages (`docs/STARTER.md` §3.1, §3.1d): Home's
//! This computer card with its checklist and the cards about this
//! computer after it, Make's two cards and Tools' two, in a grid whose
//! columns are at least `CARD_MIN_W` wide and never wider.

use super::*;
use faraday_draw::Rect;
use faraday_draw::tokens::{
    BODY_M, BUTTON_H, CARD_MIN_W, GAP, GAP_SMALL, ICON_GLYPH_SMALL, LABEL_L, LIST_2, LIST_PANE,
    MARGIN_COMPACT, MARGIN_WIDE, PAD, SHAPE_L, TITLE_M, TITLE_S,
};

use crate::Class;
use crate::decide::{Summary, Tone};

/// A checklist row's height: a two-line list item.
const ROW_H: f32 = LIST_2;

/// What a card holds, drawn at (x, y), `w` wide: returns its height.
type Body<'a> = dyn FnMut(&mut Ui, f32, f32, f32) -> f32 + 'a;

/// A card: its heading and what it holds.
struct CardSpec<'a> {
    heading: &'a str,
    body: Box<Body<'a>>,
}

/// What `draw` takes in height, drawn where nothing shows and keeping
/// no press.
fn measured(ui: &mut Ui, draw: &mut dyn FnMut(&mut Ui) -> f32) -> f32 {
    let mark = (ui.presses_mark(), ui.primaries);
    ui.c.push_clip(Rect::new(0, 0, 0, 0));
    let h = draw(ui);
    ui.c.pop_clip();
    ui.presses_rewind(mark.0);
    ui.primaries = mark.1;
    h
}

/// A card's height with `body` inside it, `w` wide.
fn card_height(ui: &mut Ui, w: f32, body: &mut Body) -> f32 {
    let inner = w - 2.0 * PAD;
    PAD + ui.line_as(TITLE_S) + measured(ui, &mut |ui| body(ui, 0.0, 0.0, inner)) + PAD
}

/// A card `h` tall at (x, y), `w` wide, with its heading and `body`.
fn draw_card(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32), heading: &str, body: &mut Body) {
    ui.card(x, y, w, h, Role::OutlineVariant);
    ui.text_as(x + PAD, y + PAD, TITLE_S, Role::OnSurfaceVariant, heading);
    let head_h = ui.line_as(TITLE_S);
    body(ui, x + PAD, y + PAD + head_h, w - 2.0 * PAD);
}

/// The grid's gutter: the page's margin.
fn gutter(starter: &Starter) -> f32 {
    if starter.compact() {
        MARGIN_COMPACT
    } else {
        MARGIN_WIDE
    }
}

/// How many columns the grid has in a page `w` wide: one up to the
/// expanded class, then as many of `CARD_MIN_W` or more as fit.
fn columns(starter: &Starter, w: f32) -> usize {
    if starter.class() != Class::Expanded {
        return 1;
    }
    let g = gutter(starter);
    (((w + g) / (CARD_MIN_W + g)).floor() as usize).max(1)
}

/// `cards` in the grid from (x, y), `w` wide: each row as tall as its
/// tallest card, a row with fewer cards than columns leaving the rest
/// empty. Returns where it ends.
fn grid(
    starter: &Starter,
    ui: &mut Ui,
    (x, y0, w): (f32, f32, f32),
    cards: &mut [CardSpec],
) -> f32 {
    let g = gutter(starter);
    let cols = columns(starter, w);
    let cw = (w - (cols as f32 - 1.0) * g) / cols as f32;
    let mut y = y0;
    for row in cards.chunks_mut(cols) {
        let h = row
            .iter_mut()
            .map(|c| card_height(ui, cw, &mut *c.body))
            .fold(0.0, f32::max);
        for (i, c) in row.iter_mut().enumerate() {
            let cx = x + i as f32 * (cw + g);
            draw_card(ui, (cx, y, cw, h), c.heading, &mut *c.body);
        }
        y += h + g;
    }
    y
}

/// A card's one button: as wide as its label, or on a compact window the
/// card's width. Returns the height it took.
fn card_button(
    starter: &Starter,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    label: &str,
    style: Style,
    action: StarterAction,
) -> f32 {
    let label = ui.fit_as(LABEL_L, label, w - 24.0);
    let bw = starter.compact().then_some(w);
    ui.button(x, y, bw, BUTTON_H, &label, style, action.host());
    BUTTON_H
}

/// A line of a card: wrapped, then a gap. Returns its height.
fn card_line(ui: &mut Ui, (x, y, w): (f32, f32, f32), s: &str, tone: Role) -> f32 {
    ui.wrap_as((x, y, w), BODY_M, tone, s) + 4.0
}

// ---------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------

/// Home: the This computer card with its checklist, what holds a
/// restart and the wireless receiver's line, then the cards about this
/// computer. At the expanded class the two are panes side by side
/// (`home_panes`); below it, the same two panes one under the other
/// (`home_stack`), as the drawing has them on a phone.
pub(super) fn home(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut cards = home_cards(starter);
    if starter.class() == Class::Expanded {
        return home_panes(starter, ui, (x, y0, w), &mut cards);
    }
    home_stack(starter, ui, (x, y0, w), &mut cards)
}

/// This computer's pane: the card's body, then at its foot what is
/// attached that holds a restart and the wireless receiver's line.
fn computer_pane(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    y += computer_body(starter, ui, x, y, w);
    if !starter.restart_holds().is_empty() || starter.receiver_line().is_some() {
        y += GAP / 2.0;
        y += label(ui, x, y, S.starter_group_screens);
    }
    y += concern_block(
        starter,
        ui,
        (x, y, w),
        (&starter.restart_holds(), starter.facts().mac().is_some()),
    );
    y += receiver_line(starter, ui, x, y, w);
    y - y0
}

/// The groups of the right pane: each one's heading and body, a rule
/// between them. Returns the height of what is inside the pane's padding.
fn groups(ui: &mut Ui, (x, y0, w): (f32, f32, f32), cards: &mut [CardSpec], show: bool) -> f32 {
    let mut y = y0;
    for (i, c) in cards.iter_mut().enumerate() {
        if i > 0 {
            y += GAP;
            if show {
                ui.rule(x, y, w, Role::OutlineVariant);
            }
            y += GAP;
        }
        if show {
            ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, c.heading);
        }
        y += ui.line_as(TITLE_S);
        if show {
            y += (c.body)(ui, x, y, w);
        } else {
            y += measured(ui, &mut |ui| (c.body)(ui, 0.0, 0.0, w));
        }
    }
    y - y0
}

/// A pane `h` tall at (x, y), `w` wide: a tonal surface with no edge,
/// as the drawing has it.
fn pane(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.fill(x, y, w, h, SHAPE_L, Role::SurfaceContainer);
}

/// A pane's header: its name in the pane's top left. Returns its height.
fn pane_header(ui: &mut Ui, (x, y): (f32, f32), s: &str) -> f32 {
    ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, s);
    ui.line_as(TITLE_S)
}

/// The right pane's height for `inner` wide groups.
fn right_height(ui: &mut Ui, inner: f32, cards: &mut [CardSpec]) -> f32 {
    PAD + measured(ui, &mut |ui| {
        ui.line_as(TITLE_S) + groups(ui, (0.0, 0.0, inner), cards, false)
    }) + PAD
}

/// The right pane at (x, y), `w` by `h`: its header, then the groups.
fn draw_right(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32), cards: &mut [CardSpec]) {
    pane(ui, (x, y, w, h));
    let head_h = pane_header(ui, (x + PAD, y + PAD), S.starter_this_starter);
    groups(ui, (x + PAD, y + PAD + head_h, w - PAD - PAD), cards, true);
}

/// Home at the expanded class: This computer's pane at the left, a list
/// pane wide, and one pane of the other cards' groups at the right, the
/// rest of the width, both as tall as the taller needs (§3.1d). Returns
/// where it ends.
fn home_panes(
    starter: &Starter,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    cards: &mut [CardSpec],
) -> f32 {
    let g = gutter(starter);
    let left_w = LIST_PANE.min(w - g - CARD_MIN_W);
    let right_w = w - g - left_w;
    let rx = x + left_w + g;
    let mut left = |ui: &mut Ui, bx: f32, by: f32, iw: f32| computer_pane(starter, ui, bx, by, iw);
    let left_h = card_height(ui, left_w, &mut left);
    let right_h = right_height(ui, right_w - PAD - PAD, cards);
    let h = left_h.max(right_h);
    draw_left(ui, (x, y, left_w, h), &mut left);
    draw_right(ui, (rx, y, right_w, h), cards);
    y + h
}

/// Home below the expanded class: the same two panes, This computer's
/// and the Starter's, one under the other in the one column, each as
/// tall as it needs. Returns where it ends.
fn home_stack(
    starter: &Starter,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    cards: &mut [CardSpec],
) -> f32 {
    let g = gutter(starter);
    let mut left = |ui: &mut Ui, bx: f32, by: f32, iw: f32| computer_pane(starter, ui, bx, by, iw);
    let left_h = card_height(ui, w, &mut left);
    draw_left(ui, (x, y, w, left_h), &mut left);
    let y2 = y + left_h + g;
    let right_h = right_height(ui, w - PAD - PAD, cards);
    draw_right(ui, (x, y2, w, right_h), cards);
    y2 + right_h
}

/// This computer's pane at (x, y), `w` by `h`: its header, then `body`.
fn draw_left(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32), body: &mut Body) {
    pane(ui, (x, y, w, h));
    let head_h = pane_header(ui, (x + PAD, y + PAD), S.starter_computer);
    body(ui, x + PAD, y + PAD + head_h, w - PAD - PAD);
}

/// The cards after This computer, those that apply (§3.1).
fn home_cards(starter: &Starter) -> Vec<CardSpec<'_>> {
    let mut cards = Vec::new();
    if starter.offers_desktop() {
        cards.push(CardSpec {
            heading: S.starter_card_desktop,
            body: Box::new(|ui, x, y, w| desktop_body(starter, ui, x, y, w)),
        });
    }
    if starter.update_offered() {
        cards.push(CardSpec {
            heading: S.starter_row_update,
            body: Box::new(|ui, x, y, w| update_body(starter, ui, x, y, w)),
        });
    }
    if starter.entry_offered() {
        cards.push(CardSpec {
            heading: S.starter_card_entry,
            body: Box::new(|ui, x, y, w| entry_body(starter, ui, x, y, w)),
        });
    }
    cards.push(CardSpec {
        heading: S.starter_group_settings,
        body: Box::new(settings_body),
    });
    cards
}

/// Settings' two rows: Appearance and About, plain rows with an arrow,
/// reached from Home only (§3.1d).
fn settings_body(ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    for (head, sub, action) in [
        (
            S.starter_row_appearance,
            S.starter_row_appearance_sub,
            StarterAction::Appearance,
        ),
        (
            S.starter_row_about,
            S.starter_row_about_sub,
            StarterAction::About,
        ),
    ] {
        y += row::plain_row(ui, (x, y, w), (head, sub), action);
    }
    y - y0
}

/// Faraday Demo's card: its version, which is this Starter's, and Open
/// (§5.3, F126).
fn desktop_body(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    y += card_line(
        ui,
        (x, y, w),
        &fill(S.starter_card_version, &[crate::VERSION]),
        Role::OnSurfaceVariant,
    );
    if let Some(why) = starter.desktop_failed.as_ref() {
        y += card_line(ui, (x, y, w), &reason_text(why), Role::Warning);
    }
    y += GAP;
    y += card_button(
        starter,
        ui,
        (x, y, w),
        S.starter_row_desktop,
        Style::Secondary,
        StarterAction::OpenDesktop,
    );
    y - y0
}

/// The update card (§3.5): the two versions, and Update.
fn update_body(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let installed = starter
        .installed
        .as_ref()
        .map_or("", |i| i.version.as_str());
    let line = fill(S.starter_card_update_line, &[installed, crate::VERSION]);
    y += card_line(ui, (x, y, w), &line, Role::OnSurfaceVariant);
    y += GAP;
    y += card_button(
        starter,
        ui,
        (x, y, w),
        S.starter_bar_update,
        Style::Secondary,
        StarterAction::Install,
    );
    y - y0
}

/// The boot entry's card (§3.1c): where it is, or why its removal did
/// not happen, and Remove.
fn entry_body(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let failed = starter.entry_remove_failed.as_ref().map(reason_text);
    let (line, tone, style) = if starter.entry_removing {
        (
            S.starter_removing_entry.to_string(),
            Role::OnSurfaceVariant,
            Style::Secondary.off(),
        )
    } else if let Some(why) = failed {
        (why, Role::Warning, Style::Secondary)
    } else {
        (
            S.starter_remove_entry_sub.to_string(),
            Role::OnSurfaceVariant,
            Style::Secondary,
        )
    };
    y += card_line(ui, (x, y, w), &line, tone);
    y += GAP;
    y += card_button(
        starter,
        ui,
        (x, y, w),
        S.starter_remove_entry,
        style,
        StarterAction::RemoveEntry,
    );
    y - y0
}

// ---------------------------------------------------------------------
// This computer
// ---------------------------------------------------------------------

/// What the card says about this computer: its name, and a line for each
/// fact, with its tone.
fn computer_facts(starter: &Starter) -> (String, Vec<(String, Role)>) {
    let c = &starter.computer;
    let mac = starter.facts().mac();
    let row = mac.and(macs::mac(&c.model));
    let name = match row {
        // On a Mac, its name from the table; the identifier is a line.
        Some(m) => m.name.to_string(),
        None => format!("{} {}", c.maker.trim(), c.model.trim()),
    };
    let name = if name.trim().is_empty() {
        S.starter_computer_unknown.to_string()
    } else {
        name.trim().to_string()
    };
    // A Mac shows no Secure Boot line: before the T2 chip its firmware
    // has no such setting, and Startup Security is not read.
    let mut lines: Vec<(String, Role)> = match mac {
        Some(_) => {
            let mut l = Vec::new();
            if row.is_some() {
                l.push((c.model.trim().to_string(), Role::OnSurfaceVariant));
            }
            if row.is_some_and(|m| m.t2) {
                l.push((S.starter_mac_t2.to_string(), Role::OnSurface));
            }
            l
        }
        None => Vec::new(),
    };
    match mac {
        Some(macs::Start::NotYet) => {
            lines.push((S.starter_cannot_start_mac_yet.to_string(), Role::Warning))
        }
        Some(macs::Start::Firmware32) => {
            lines.push((
                S.starter_cannot_start_mac_firmware.to_string(),
                Role::Warning,
            ));
        }
        Some(macs::Start::Intel { .. }) => {}
        None => {}
    }
    lines.extend(restart_lines(starter));
    (name, lines)
}

/// The icon and colour of a check's tone: a mark besides the colour, so
/// the colour is not all that tells it.
fn tone_mark(tone: Tone) -> (Icon, Role) {
    match tone {
        Tone::Good => (Icon::Success, Role::Ok),
        Tone::Step => (Icon::Warning, Role::Warning),
        Tone::Bad => (Icon::Error, Role::Error),
    }
}

/// One check: its mark, its label and what it found beside it (under it
/// when the card is too narrow for both), the value in the tone's
/// colour. A check with no label is one sentence. Returns its height.
fn check_row(ui: &mut Ui, (x, y, w): (f32, f32, f32), label: &str, value: &str, tone: Tone) -> f32 {
    let (icon, role) = tone_mark(tone);
    let line = ui.line_as(BODY_M);
    ui.icon(
        x,
        y + (line - ICON_GLYPH_SMALL) / 2.0,
        ICON_GLYPH_SMALL,
        icon,
        ICON_GLYPH_SMALL,
        role,
    );
    let tx = x + ICON_GLYPH_SMALL + GAP_SMALL * 2.0;
    let tw = w - (tx - x);
    let h = if label.is_empty() {
        ui.wrap_as((tx, y, tw), BODY_M, Role::OnSurface, value)
    } else {
        let lw = ui
            .measure_as(BODY_M, S.starter_check_secure_boot)
            .max(ui.measure_as(BODY_M, S.starter_check_start_mode))
            + GAP;
        ui.text_as(tx, y, BODY_M, Role::OnSurfaceVariant, label);
        ui.wrap_as((tx + lw, y, tw - lw), BODY_M, Role::OnSurface, value)
    };
    h.max(line) + GAP_SMALL
}

/// The summary and the checks of this computer (§3.1): one line saying
/// whether it can start Faraday, then a row for each check, then each
/// thing attached that holds a restart. A Mac has none. Returns its
/// height.
fn checks(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let rows = starter.checks();
    if rows.is_empty() {
        return 0.0;
    }
    let (summary, tone) = match starter.summary() {
        Summary::Can => (S.starter_check_can, Tone::Good),
        Summary::CanWithoutCamera => (S.starter_check_can_no_camera, Tone::Step),
        Summary::Cannot => (S.starter_check_cannot, Tone::Bad),
    };
    let mut y = y0 + GAP_SMALL;
    y += check_row(ui, (x, y, w), "", summary, tone) + GAP_SMALL;
    for r in &rows {
        y += check_row(ui, (x, y, w), r.label, r.value, r.tone);
    }
    for c in starter.restart_holds() {
        y += check_row(ui, (x, y, w), "", &concern_text(c), Tone::Bad);
    }
    y - y0
}

/// Why the card offers what it does (§3.1): the restart that did not
/// happen, and the encryption product found.
fn restart_lines(starter: &Starter) -> Vec<(String, Role)> {
    let c = &starter.computer;
    let mut lines = Vec::new();
    let card = starter.card();
    if card == Card::Firmware {
        // Why the card offers the firmware's settings, not the stick.
        if c.secure_boot == Some(false)
            && let Some(why) = &starter.stick_restart_failed
        {
            lines.push((
                fill(S.starter_stick_restart_failed, &[&reason_text(why)]),
                Role::Warning,
            ));
        }
        if let Some(why) = &starter.restart_failed {
            lines.push((reason_text(why), Role::Error));
        }
    }
    if card == Card::Rescue
        && let Some(name) = &starter.other
    {
        lines.push((fill(S.starter_other_found, &[name]), Role::Warning));
    }
    lines
}

/// The card's button for this computer's state, and whether it can be
/// pressed now.
fn computer_button(starter: &Starter) -> (&'static str, bool) {
    let free = starter.restart_holds().is_empty();
    match starter.card() {
        Card::Make => (S.starter_item_make, true),
        Card::Firmware => (S.starter_restart, free),
        Card::Stick => (S.starter_restart_stick, free),
        Card::StartMac => (S.starter_item_start_mac, true),
        Card::Rescue => (S.starter_other_confirm, true),
    }
}

/// The This computer card's content: its name and facts, the checklist,
/// and the screen's one primary button. Returns its height.
fn computer_body(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let (name, lines) = computer_facts(starter);
    let mut y = y0;
    // A Mac's name can be long (one identifier, several years): it wraps.
    y += ui
        .wrap_as((x, y, w), TITLE_M, Role::OnSurface, &name)
        .max(20.0)
        + 6.0;
    for (l, tone) in &lines {
        y += card_line(ui, (x, y, w), l, *tone);
    }
    y += checks(starter, ui, x, y, w);
    y += GAP * 2.0;
    y += checklist(starter, ui, x, y, w);
    y += GAP;
    let (label, offered) = computer_button(starter);
    let style = Style::Primary.when(offered);
    y += card_button(starter, ui, (x, y, w), label, style, StarterAction::Next);
    y - y0
}

/// The checklist (§3.1): its label, then a row for each item that
/// applies, the next step marked. Returns its height.
fn checklist(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let facts = starter.facts();
    let next = facts.next_step();
    let mut y = y0;
    y += label(ui, x, y, S.starter_checklist);
    ui.rule(x, y, w, Role::OutlineVariant);
    for (n, row) in facts.checklist().iter().enumerate() {
        let action = StarterAction::Item(row.item);
        let is_next = next == Some(row.item);
        let tx = x + 44.0;
        let state = state_text(&row.state);
        // The title and the state wrap, and the row grows with them.
        let none = Role::OnSurface.paint();
        let row_h = row::list_text(
            ui,
            (tx, y, w - 80.0),
            ROW_H,
            (item_label(starter, row.item), none),
            (state.as_str(), none),
            false,
        );
        let layer = (x - 6.0, y + 2.0, w + 12.0, row_h - 4.0, SHAPE_S);
        if is_next {
            // The next step: Material's selected state.
            ui.fill(
                layer.0,
                layer.1,
                layer.2,
                layer.3,
                layer.4,
                Role::SecondaryContainer,
            );
        }
        let lit = is_next;
        let fg = faraday_ui::pal::on_selected(
            lit,
            if row.done {
                Role::OnSurfaceVariant
            } else {
                Role::OnSurface
            },
        );
        // The pressed layer in the row's content colour, over its
        // selected fill when it is the next step and under its content.
        let shape = Shape::new(layer.0, layer.1, layer.2, layer.3, layer.4);
        ui.press_to(shape, (x - 6.0, y, w + 12.0, row_h), fg, action.host());
        ui.badge(
            x + 4.0,
            y + (row_h - 26.0) / 2.0,
            &(n + 1).to_string(),
            row.done,
            is_next,
        );
        let muted = faraday_ui::pal::on_selected(lit, Role::OnSurfaceVariant);
        row::list_text(
            ui,
            (tx, y, w - 80.0),
            ROW_H,
            (item_label(starter, row.item), fg),
            (state.as_str(), muted),
            true,
        );
        let (cx, cy) = (x + w - 24.0, y + (row_h - 20.0) / 2.0);
        ui.icon(cx, cy, 20.0, Icon::ChevronRight, 9.0, muted);
        y += row_h;
        let under = w - 44.0;
        if row.item == Item::MakeStick
            && let Some(check) = starter.boot_check()
        {
            y += super::line(
                ui,
                tx,
                y,
                under,
                super::boot_text(check),
                super::boot_tone(check),
            );
        }
        if row.item == Item::SecureBootOn {
            y += link(ui, tx, y, under, StartGuide::SecureBootBitLocker);
        }
        ui.rule(x, y, w, Role::OutlineVariant);
    }
    y - y0
}

// ---------------------------------------------------------------------
// Make and Tools
// ---------------------------------------------------------------------

/// Make (§3.1d): the Faraday stick card and the Pi card.
pub(super) fn make(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let g = gutter(starter);
    let mut cards = vec![
        CardSpec {
            heading: S.starter_card_stick,
            body: Box::new(|ui, x, y, w| stick_body(starter, ui, x, y, w)),
        },
        CardSpec {
            heading: S.starter_card_pi,
            body: Box::new(|ui, x, y, w| pi_body(starter, ui, x, y, w)),
        },
    ];
    grid(starter, ui, (x, y0, w), &mut cards) - g
}

/// The Faraday stick card: the attached stick's version, or none, and
/// Make a Faraday stick, primary until a stick is attached.
fn stick_body(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let row = starter
        .facts()
        .checklist()
        .into_iter()
        .find(|r| r.item == Item::MakeStick);
    let line = row.as_ref().map_or_else(
        || S.starter_state_no_stick.to_string(),
        |r| state_text(&r.state),
    );
    let attached = row.is_some_and(|r| r.done);
    let mut y = y0;
    y += card_line(ui, (x, y, w), &line, Role::OnSurfaceVariant);
    if let Some(check) = starter.boot_check() {
        y += card_line(
            ui,
            (x, y, w),
            super::boot_text(check),
            super::boot_tone(check),
        );
    }
    y += GAP;
    let style = if attached {
        Style::Secondary
    } else {
        Style::Primary
    };
    y += card_button(
        starter,
        ui,
        (x, y, w),
        S.starter_item_make,
        style,
        StarterAction::Make(Target::Stick),
    );
    if starter.can_check_stick() {
        y += GAP;
        y += card_button(
            starter,
            ui,
            (x, y, w),
            S.starter_stick_check,
            Style::Secondary,
            StarterAction::CheckStick,
        );
    }
    if let Some(why) = starter.check_failure() {
        y += GAP;
        y += card_line(ui, (x, y, w), &super::reason_text(why), Role::Error);
    }
    y - y0
}

/// The Pi card: the found Pi image's version, or that none was found,
/// and Make a Pi card, off without the image.
fn pi_body(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let image = starter.image(Target::Pi);
    let has = image.is_some();
    let line = match image.and_then(|i| i.build) {
        Some(build) => fill(S.starter_card_version, &[&build]),
        None => S.starter_card_pi_none.to_string(),
    };
    let mut y = y0;
    y += card_line(ui, (x, y, w), &line, Role::OnSurfaceVariant);
    y += GAP;
    let style = Style::Secondary.when(has);
    y += card_button(
        starter,
        ui,
        (x, y, w),
        S.starter_row_pi,
        style,
        StarterAction::Make(Target::Pi),
    );
    y - y0
}

/// Tools (§3.1d): the Transfer card and the Blank templates card.
pub(super) fn tools(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let g = gutter(starter);
    let mut cards = vec![
        CardSpec {
            heading: S.starter_row_transfer,
            body: Box::new(|ui, x, y, w| {
                tool_body(
                    starter,
                    ui,
                    (x, y, w),
                    S.starter_card_transfer_line,
                    (
                        S.starter_open_transfer,
                        Style::Primary,
                        StarterAction::Transfer,
                    ),
                )
            }),
        },
        CardSpec {
            heading: S.starter_row_templates,
            body: Box::new(|ui, x, y, w| {
                tool_body(
                    starter,
                    ui,
                    (x, y, w),
                    S.starter_card_templates_line,
                    (
                        S.starter_open_templates,
                        Style::Secondary,
                        StarterAction::Templates,
                    ),
                )
            }),
        },
    ];
    grid(starter, ui, (x, y0, w), &mut cards) - g
}

/// A Tools card: a line and its button.
fn tool_body(
    starter: &Starter,
    ui: &mut Ui,
    at: (f32, f32, f32),
    line: &str,
    (label, style, action): (&str, Style, StarterAction),
) -> f32 {
    let (x, y0, w) = at;
    let mut y = y0;
    y += card_line(ui, (x, y, w), line, Role::OnSurfaceVariant);
    y += GAP;
    y += card_button(starter, ui, (x, y, w), label, style, action);
    y - y0
}
