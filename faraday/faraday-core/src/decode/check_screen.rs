//! `Screen::SignerCheck`: the cards of Check a signer
//! (`docs/SIGNER-CHECK.md` §4). Drawing reads the state, the Inbox
//! items' `PsbtSeen` and the report kept; it works nothing out.

use super::check::{self, CheckState, Origin, RESULT, SignerPress, Slot, Tone};
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
use faraday_psbt::Level;
use faraday_ui::parts::{button_rows, section_label};
use faraday_ui::sheet::kv;

/// The three cards over the state kept.
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.signer_check() else {
        return;
    };
    let titles = [EN.signer_sent, EN.signer_returned, EN.signer_result];
    let cards: Vec<flow::Card> = (0..=RESULT)
        .map(|n| {
            let side = match n {
                0 => s.side(Slot::Sent),
                1 => s.side(Slot::Returned),
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
                    s.report.is_some()
                } else {
                    side.is_some()
                },
                open: s.open == n,
                default: side.is_some(),
                toggle: Action::SignerCheck(SignerPress::Card(n)),
                guide: None,
                learn: None,
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
            Action::SignerCheck(SignerPress::Back),
        ))),
        heading: EN.catalog_check_a_signer,
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
            match app_ref.signer_check() {
                Some(s) => card(app_ref, s, ui, i as u8, (x, y, w)),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::SignerCheck, next, again);
}

/// What the open card holds.
fn card(app: &View, s: &CheckState, ui: &mut Ui, n: u8, at: (f32, f32, f32)) -> f32 {
    match n {
        0 => choose(app, s, ui, Slot::Sent, at),
        1 => choose(app, s, ui, Slot::Returned, at),
        _ => result(s, ui, at),
    }
}

/// The Sent or Returned card: a row per PSBT in Files, Scan under them.
fn choose(app: &View, s: &CheckState, ui: &mut Ui, slot: Slot, (x, y, w): (f32, f32, f32)) -> f32 {
    let all = check::files(&app.inbox, &app.outbox);
    let (same, other) = check::rows(&all, slot, (s.sent.as_ref(), s.returned.as_ref()));
    let mut cy = y;
    if all.is_empty() {
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.signer_no_psbt);
        cy += ui.line_as(BODY_S) + PAD;
    }
    // A copy whose file was replaced is no row's file: any can be picked.
    let chosen = s.side(slot).filter(|d| !d.differs).map(|d| &d.origin);
    for n in &same {
        cy += row(ui, (x, cy, w), &all[*n], chosen, slot, *n);
    }
    if !other.is_empty() {
        section_label(ui, x, cy, EN.signer_other_transactions);
        cy += ui.line_as(TITLE_S) + GAP;
        for n in &other {
            cy += row(ui, (x, cy, w), &all[*n], chosen, slot, *n);
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
        Action::SignerCheck(SignerPress::Scan(slot)),
    );
    cy + BUTTON_H + GAP_SMALL - y
}

/// One option row (DESIGN §4.2): the file's name and its line, the
/// chosen one selected. The line is read from the item's `PsbtSeen`.
fn row(
    ui: &mut Ui,
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
            name: item.name.clone(),
        });
    let line = item.psbt().map_or_else(String::new, |p| {
        let id = faraday_ui::short(&p.txid.to_string());
        if p.sigs == 1 {
            fill(EN.signer_row_one, &[&id])
        } else {
            fill(EN.signer_row, &[&p.sigs.to_string(), &id])
        }
    });
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
    ui.press_box(
        x,
        y,
        w,
        h,
        Action::SignerCheck(SignerPress::Pick(slot, place)),
    );
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

/// The Result card, top down (SIGNER-CHECK §4): the verdict in its tone,
/// the facts, the findings, what is stated, the actions. The words were
/// made with the report; this reads them.
fn result(s: &CheckState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let (Some(report), Some(shown)) = (&s.report, &s.shown) else {
        return 0.0;
    };
    let (line, tone) = check::verdict_line(report);
    let (role, icon) = match tone {
        Tone::Ok => (Role::Ok, Icon::Success),
        Tone::Warn => (Role::Warning, Icon::Warning),
        Tone::Err => (Role::Error, Icon::Error),
    };
    let mut cy = y;
    let text_x = x + PAD + ICON_GLYPH + PAD;
    let text_w = x + w - PAD - text_x;
    let text_h = ui.wrap_height_as(TITLE_M, &line, text_w);
    let card_h = (text_h + PAD + PAD).max(TOUCH + PAD);
    ui.card(x, cy, w, card_h, Role::OutlineVariant);
    ui.icon(
        x + PAD,
        cy + (card_h - ICON_GLYPH) / 2.0,
        ICON_GLYPH,
        icon,
        PAD,
        role,
    );
    ui.wrap_as(
        (text_x, cy + (card_h - text_h) / 2.0, text_w),
        TITLE_M,
        role,
        &line,
    );
    cy += card_h + PAD;
    for (label, value) in &shown.facts {
        cy += kv(ui, x, cy, w, label, value, Role::OnSurface);
    }
    if shown.has_findings {
        section_label(ui, x, cy, EN.signer_findings_label);
        cy += ui.line_as(TITLE_S) + GAP;
        if shown.findings.is_empty() {
            ui.text_as(x, cy, BODY_M, Role::OnSurfaceVariant, EN.signer_no_findings);
            cy += ui.line_as(BODY_M) + PAD;
        }
        let room = ICON_GLYPH + GAP;
        for (level, text) in &shown.findings {
            let role = match level {
                Level::Blocked | Level::Danger => Role::Error,
                Level::Caution | Level::Info => Role::Warning,
            };
            let used = ui.wrap_as((x + room, cy, w - room), BODY_M, role, text);
            ui.icon(x, cy, ICON_GLYPH, Icon::Warning, GAP, role);
            cy += used.max(ICON_GLYPH) + GAP;
        }
    }
    if !shown.stated.is_empty() {
        cy += GAP_SMALL;
        section_label(ui, x, cy, EN.signer_stated_label);
        cy += ui.line_as(TITLE_S) + GAP;
        for text in &shown.stated {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurfaceVariant, text) + GAP;
        }
    }
    cy += GAP_SMALL;
    let mut actions = vec![
        (
            EN.decode_show_as_qr.to_string(),
            Style::Secondary,
            Action::SignerCheck(SignerPress::Qr),
        ),
        (
            EN.signer_swap.to_string(),
            Style::Secondary,
            Action::SignerCheck(SignerPress::Swap),
        ),
    ];
    // A key that signed and is not loaded: Add a key, then back here.
    for (n, fp) in shown.load.iter().enumerate() {
        actions.push((
            fill(EN.signer_load, &[&fp.to_string()]),
            Style::Secondary,
            Action::SignerCheck(SignerPress::Load(n)),
        ));
    }
    actions.push((
        EN.signer_choose_again.to_string(),
        Style::Ghost,
        Action::SignerCheck(SignerPress::Again),
    ));
    actions.push((
        EN.app_done.to_string(),
        Style::Primary,
        Action::SignerCheck(SignerPress::Back),
    ));
    cy += button_rows(ui, x, cy, w, &actions);
    cy - y
}
