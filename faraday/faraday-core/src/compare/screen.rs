//! `Screen::Compare`: the cards of Compare two PSBTs
//! (`docs/COMPARE.md` §2.7). Drawing reads the state, the Inbox items'
//! `PsbtSeen` and the words kept; it works nothing out.

use super::words::Fact;
use super::{ComparePress, CompareState, RESULT, Slot, Tone};
use crate::decode::check::Origin;
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, flow};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, GAP, GAP_SMALL, ICON_GLYPH, PAD, READING_PAGE_W, SHAPE_S, SPACER,
    TITLE_M, TITLE_S, TOUCH,
};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{button_rows, section_label};

/// The three cards over the state kept.
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.compare() else {
        return;
    };
    let titles = [
        EN.compare_fact_first,
        EN.compare_fact_second,
        EN.compare_result,
    ];
    let cards: Vec<flow::Card> = (0..=RESULT)
        .map(|n| {
            let side = match n {
                0 => s.side(Slot::First),
                1 => s.side(Slot::Second),
                _ => None,
            };
            flow::Card {
                title: titles[usize::from(n)].to_string(),
                summary: side.map_or_else(String::new, |d| {
                    if d.differs {
                        fill(EN.signer_file_differs, &[&d.name])
                    } else {
                        d.name.clone()
                    }
                }),
                mono: false,
                done: if n == RESULT {
                    s.shown.is_some()
                } else {
                    side.is_some()
                },
                open: s.open == n,
                default: side.is_some(),
                toggle: Action::Compare(ComparePress::Card(n)),
                walkthrough: None,
            }
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + SPACER + PAD,
        w: (cw - (SPACER + PAD) - (SPACER + PAD)).min(READING_PAGE_W),
        h,
        back: Some(app.named_back((
            crate::screens::decode_back(s.back),
            Action::Compare(ComparePress::Back),
        ))),
        heading: EN.catalog_compare_two_psbts,
        guided: false,
        switch: false,
        wrap_closed: true,
        total_fixed: true,
        note: None,
        chip: None,
        chip_tap: None,
        moves: crate::ui::Moves::No,
    };
    let scroll = s.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.compare() {
                Some(s) => card(app_ref, s, ui, i as u8, (x, y, w)),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Compare, next, again);
}

/// What the open card holds.
fn card(app: &View, s: &CompareState, ui: &mut Ui, n: u8, at: (f32, f32, f32)) -> f32 {
    match n {
        0 => choose(app, s, ui, Slot::First, at),
        1 => choose(app, s, ui, Slot::Second, at),
        _ => result(s, ui, at),
    }
}

/// The First or Second card: a row per file, Scan under them.
fn choose(
    app: &View,
    s: &CompareState,
    ui: &mut Ui,
    slot: Slot,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let all = super::files(&app.inbox, &app.outbox);
    let (same, other) = super::rows(&all, slot, (s.first.as_ref(), s.second.as_ref()));
    let mut cy = y;
    if all.is_empty() {
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.compare_no_files);
        cy += ui.line_as(BODY_S) + PAD;
    }
    // A copy whose file was replaced is no row's file: any can be picked.
    let chosen = s.side(slot).filter(|d| !d.differs).map(|d| &d.origin);
    for n in &same {
        cy += row(ui, s, (x, cy, w), &all[*n], chosen, slot, *n);
    }
    if !other.is_empty() {
        section_label(ui, x, cy, EN.signer_other_transactions);
        cy += ui.line_as(TITLE_S) + GAP;
        for n in &other {
            cy += row(ui, s, (x, cy, w), &all[*n], chosen, slot, *n);
        }
    }
    if let Some(why) = &s.refused[usize::from(slot.card())] {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, why) + GAP;
    }
    cy += GAP_SMALL;
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.signer_scan,
        Style::Secondary,
        Action::Compare(ComparePress::Scan(slot)),
    );
    cy + BUTTON_H + GAP_SMALL - y
}

/// One option row (DESIGN §4.2): the file's name and its line, the
/// chosen one selected.
fn row(
    ui: &mut Ui,
    s: &CompareState,
    (x, y, w): (f32, f32, f32),
    file: &(bool, usize, &crate::Item),
    chosen: Option<&Origin>,
    slot: Slot,
    place: usize,
) -> f32 {
    let (outbox, _, item) = file;
    let on = chosen
        == Some(&Origin::File {
            outbox: *outbox,
            name: faraday_text::Text::of(&item.name),
        });
    let line = s.line(item);
    let size = TITLE_S;
    let (name_h, line_h) = (ui.line_as(size), ui.line_as(BODY_S));
    let h = if ui.compact {
        (name_h + line_h + PAD).max(TOUCH)
    } else {
        TOUCH
    };
    ui.fill(
        x,
        y,
        w,
        h,
        SHAPE_S,
        selected_or(on, Role::SurfaceContainerHighest),
    );
    ui.stroke(x, y, w, h, SHAPE_S, selected_or(on, Role::OutlineVariant));
    ui.press_box(x, y, w, h, Action::Compare(ComparePress::Pick(slot, place)));
    let (ink, muted) = (
        on_selected(on, Role::OnSurface),
        on_selected(on, Role::OnSurfaceVariant),
    );
    let left = x + PAD;
    if ui.compact {
        let name = ui.fit_as(size, &item.name, w - PAD - PAD);
        ui.text_as(left, y + GAP, size, ink, &name);
        ui.text_as(left, y + GAP + name_h, BODY_S, muted, &line);
    } else {
        let room = ui.measure_as(BODY_S, &line);
        let name = ui.fit_as(size, &item.name, w - PAD - PAD - room - PAD);
        ui.text_mid_as((left, y, h), size, ink, &name);
        ui.text_right_as((x + w - PAD, y, h), BODY_S, muted, &line);
    }
    h + GAP
}

/// A fact (DESIGN §4.11): its label over its value, or over a line per
/// labelled value. Returns the height.
fn fact(ui: &mut Ui, (x, y, w): (f32, f32, f32), f: &Fact) -> f32 {
    let mut cy = y + ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, f.label) + GAP_SMALL;
    let room = f
        .values
        .iter()
        .filter_map(|v| v.label)
        .map(|l| ui.measure_as(BODY_S, l) + PAD)
        .fold(0.0, f32::max);
    for v in &f.values {
        if let Some(l) = v.label {
            ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, l);
        }
        cy += ui.wrap_as((x + room, cy, w - room), BODY_M, Role::OnSurface, &v.text) + GAP_SMALL;
    }
    ui.rule(x, cy, w, Role::OutlineVariant);
    cy + GAP - y
}

/// The fee bump result: a card with the result in its tone.
fn bump_result(ui: &mut Ui, (x, y, w): (f32, f32, f32), line: &str, tone: Tone) -> f32 {
    let (role, icon) = match tone {
        Tone::Ok => (Role::Ok, Icon::Success),
        Tone::Warn => (Role::Warning, Icon::Warning),
        Tone::Err => (Role::Error, Icon::Error),
    };
    let text_x = x + PAD + ICON_GLYPH + PAD;
    let text_w = x + w - PAD - text_x;
    let text_h = ui.wrap_height_as(TITLE_M, line, text_w);
    let card_h = (text_h + PAD + PAD).max(TOUCH + PAD);
    ui.card(x, y, w, card_h, Role::OutlineVariant);
    ui.icon(
        x + PAD,
        y + (card_h - ICON_GLYPH) / 2.0,
        ICON_GLYPH,
        icon,
        PAD,
        role,
    );
    ui.wrap_as(
        (text_x, y + (card_h - text_h) / 2.0, text_w),
        TITLE_M,
        role,
        line,
    );
    card_h + PAD
}

/// The Result card, top down (`docs/COMPARE.md` §2.7): the facts, the fee
/// bump, the differences, the actions. The words were made with the
/// result; this reads them.
fn result(s: &CompareState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(shown) = &s.shown else {
        return 0.0;
    };
    let mut cy = y;
    for f in &shown.facts {
        cy += fact(ui, (x, cy, w), f);
    }
    if let Some(b) = &shown.bump {
        section_label(ui, x, cy, EN.compare_fee_bump_label);
        cy += ui.line_as(TITLE_S) + GAP;
        cy += bump_result(ui, (x, cy, w), b.result, b.tone());
        for line in &b.lines {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, line) + GAP;
        }
        for f in &b.facts {
            cy += fact(ui, (x, cy, w), f);
        }
    }
    section_label(ui, x, cy, EN.compare_differences_label);
    cy += ui.line_as(TITLE_S) + GAP;
    for line in &shown.differences {
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, line) + GAP;
    }
    cy += GAP_SMALL;
    let mut actions = vec![(
        EN.signer_swap.to_string(),
        Style::Secondary,
        Action::Compare(ComparePress::Swap),
    )];
    if shown.can_check {
        actions.push((
            EN.catalog_check_a_signer.to_string(),
            Style::Secondary,
            Action::Compare(ComparePress::CheckSigner),
        ));
    }
    // A key the result names and that is not loaded: Add a key, then back
    // here.
    for (n, fp) in shown.load.iter().enumerate() {
        actions.push((
            fill(EN.signer_load, &[&fp.to_string()]),
            Style::Secondary,
            Action::Compare(ComparePress::Load(n)),
        ));
    }
    actions.push((
        EN.signer_choose_again.to_string(),
        Style::Ghost,
        Action::Compare(ComparePress::Again),
    ));
    actions.push((
        EN.app_done.to_string(),
        Style::Primary,
        Action::Compare(ComparePress::Back),
    ));
    cy += button_rows(ui, x, cy, w, &actions);
    cy - y
}
