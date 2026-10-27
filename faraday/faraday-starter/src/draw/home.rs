//! The three destinations' pages (`docs/STARTER.md` §3.1, §3.1d): Home's
//! This computer card with its checklist and the cards about this
//! computer after it, Make's two cards and Tools' two, in a grid whose
//! columns are at least `CARD_MIN_W` wide and never wider.

use super::*;
use faraday_draw::Rect;
use faraday_draw::tokens::{
    BODY_M, BUTTON_H, CARD_MIN_W, GAP, LABEL_L, LIST_2, MARGIN_COMPACT, MARGIN_WIDE, PAD, TITLE_M,
    TITLE_S,
};

use crate::Class;

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
    let mark = (ui.hits.len(), ui.reach.len(), ui.primaries);
    ui.c.push_clip(Rect::new(0, 0, 0, 0));
    let h = draw(ui);
    ui.c.pop_clip();
    ui.hits.truncate(mark.0);
    ui.reach.truncate(mark.1);
    ui.primaries = mark.2;
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
/// restart, then the cards about this computer in the grid.
pub(super) fn home(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let g = gutter(starter);
    let mut y = y0;
    let mut body = |ui: &mut Ui, bx: f32, by: f32, iw: f32| computer_body(starter, ui, bx, by, iw);
    let h = card_height(ui, w, &mut body);
    draw_card(ui, (x, y, w, h), S.starter_computer, &mut body);
    y += h + g;
    y += concern_block(starter, ui, x, y, w, &starter.restart_holds());
    y += receiver_line(starter, ui, x, y, w);
    y = grid(starter, ui, (x, y, w), &mut home_cards(starter));
    y - g
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
    cards
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
            Style::Disabled,
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
        None => vec![(secure_boot_text(c.secure_boot), Role::OnSurface)],
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
        None if c.arm => lines.push((S.starter_cannot_start_arm.to_string(), Role::Warning)),
        None if !c.uefi => lines.push((S.starter_cannot_start_bios.to_string(), Role::Warning)),
        None => {}
    }
    lines.extend(restart_lines(starter));
    (name, lines)
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
    y += GAP * 2.0;
    y += checklist(starter, ui, x, y, w);
    y += GAP;
    let (label, offered) = computer_button(starter);
    let style = if offered {
        Style::Primary
    } else {
        Style::Disabled
    };
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
        let pressed = ui.is_pressed(action.host());
        if pressed {
            ui.wash(
                x - 6.0,
                y + 2.0,
                w + 12.0,
                ROW_H - 4.0,
                8.0,
                Role::OnSurface.at(STATE_PRESS),
            );
        } else if is_next {
            // The next step: Material's selected state.
            let (rx, rw) = (x - 6.0, w + 12.0);
            ui.fill(rx, y + 2.0, rw, ROW_H - 4.0, 8.0, Role::SecondaryContainer);
        }
        ui.badge(
            x + 4.0,
            y + (ROW_H - 26.0) / 2.0,
            &(n + 1).to_string(),
            row.done,
            is_next,
        );
        let tx = x + 44.0;
        let lit = is_next && !pressed;
        let fg = faraday_ui::pal::on_selected(
            lit,
            if row.done {
                Role::OnSurfaceVariant
            } else {
                Role::OnSurface
            },
        );
        let muted = faraday_ui::pal::on_selected(lit, Role::OnSurfaceVariant);
        two_lines(
            ui,
            (tx, y, ROW_H),
            (w - 80.0, w - 80.0),
            (item_label(starter, row.item), fg),
            (state_text(&row.state).as_str(), muted),
        );
        let (cx, cy) = (x + w - 24.0, y + (ROW_H - 20.0) / 2.0);
        ui.icon(cx, cy, 20.0, Icon::ChevronRight, 9.0, muted);
        ui.hit(x - 6.0, y, w + 12.0, ROW_H, action.host());
        y += ROW_H;
        if row.item == Item::SecureBootOn {
            y += link(ui, tx, y, w - 44.0, LearnId::SecureBootBitLocker);
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
    y - y0
}

/// The Pi card: the Pi image's version, or that this Starter has none,
/// and Make a Pi card, off without the image.
fn pi_body(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let has = starter.image(Target::Pi).is_some();
    let line = match (&starter.payload, has) {
        (Some(p), true) => fill(S.starter_card_version, &[&p.version]),
        _ => S.starter_card_pi_none.to_string(),
    };
    let mut y = y0;
    y += card_line(ui, (x, y, w), &line, Role::OnSurfaceVariant);
    y += GAP;
    let style = if has {
        Style::Secondary
    } else {
        Style::Disabled
    };
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
