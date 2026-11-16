//! `Screen::SignerCheck`: the cards of Check a signer
//! (`docs/SIGNER-CHECK.md` §4, §11.5). Drawing reads the state, the
//! items' `PsbtSeen` and the words kept; it works nothing out.

use super::check::{self, CheckState, Facts, Origin, RESULT, SignerPress, Slot, Tone};
use super::diff_words::Fact;
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, flow};
use faraday_bip::keys::Fingerprint;
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
                        String::from(&*fill(EN.signer_file_differs, &[&d.name]))
                    } else {
                        String::from(&*d.name)
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
                toggle: Action::SignerCheck(SignerPress::Card(n)),
                lead: None,
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

/// The Sent or Returned card: a row per PSBT or finished transaction in
/// Files, Scan under them.
fn choose(app: &View, s: &CheckState, ui: &mut Ui, slot: Slot, (x, y, w): (f32, f32, f32)) -> f32 {
    let all = check::files(&app.inbox, &app.outbox);
    let (same, other) = check::rows(&all, slot, (s.sent.as_ref(), s.returned.as_ref()));
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
        Action::SignerCheck(SignerPress::Scan(slot)),
    );
    cy + BUTTON_H + GAP_SMALL - y
}

/// One option row (DESIGN §4.2): the file's name and its line, the
/// chosen one selected. The line is read from the item's `PsbtSeen` or
/// the id `signer_check_sync` kept.
fn row(
    ui: &mut Ui,
    s: &CheckState,
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

/// A result card: the line in its tone, with its icon. Returns the
/// height with the gap under it.
fn tone_card(ui: &mut Ui, (x, y, w): (f32, f32, f32), line: &str, tone: Tone) -> f32 {
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

/// A key that signed and whose seed is not loaded: its fingerprint, what
/// it lacks and **Load the seed** (Material 3 Lists, a list item with a
/// trailing text button).
fn seed_row(ui: &mut Ui, (x, y, w): (f32, f32, f32), fp: &Fingerprint, n: usize) -> f32 {
    let (name_h, line_h) = (ui.line_as(TITLE_S), ui.line_as(BODY_S));
    let h = (name_h + line_h + PAD).max(TOUCH);
    let label = EN.signer_load_the_seed;
    let bw = (ui.measure_as(BODY_M, label) + 2.0 * PAD).max(TOUCH);
    let text_y = y + (h - name_h - line_h) / 2.0;
    ui.text_as(x, text_y, TITLE_S.mono(), Role::OnSurface, &fp.to_string());
    ui.text_as(
        x,
        text_y + name_h,
        BODY_S,
        Role::OnSurfaceVariant,
        EN.signer_seed_not_loaded,
    );
    ui.button(
        x + w - bw,
        y + (h - BUTTON_H) / 2.0,
        Some(bw),
        BUTTON_H,
        label,
        Style::Ghost,
        Action::SignerCheck(SignerPress::Seed(n)),
    );
    ui.rule(x, y + h, w, Role::OutlineVariant);
    h + GAP_SMALL
}

/// The sheet of ways to load the seed of a key that signed: Type the
/// words, Scan a SeedQR and Open a vault, each coming back to the Result
/// (Material 3 Dialogs, a simple dialog of list rows).
pub(crate) fn seed_dialog(app: &View, ui: &mut Ui, w: f32, h: f32) {
    use faraday_draw::tokens::DIALOG_MAX;
    let Some(fp) = app.signer_check().and_then(|s| s.seed) else {
        return;
    };
    let title = fill(EN.signer_seed_of, &[&fp.to_string()]);
    let head = (Icon::Keys.into(), Role::Primary.paint(), title.as_str());
    let ways = [
        (EN.keygen_type_the_words, SignerPress::SeedWords),
        (EN.keygen_scan_a_seedqr, SignerPress::SeedScan),
        (EN.signer_open_a_vault, SignerPress::SeedVault),
    ];
    let actions = [(EN.vault_cancel, Style::Secondary, Action::Cancel)];
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for (label, press) in ways {
                ui.fill(x, cy, iw, TOUCH, SHAPE_S, Role::SurfaceContainerHighest);
                ui.text_mid_as((x + PAD, cy, TOUCH), BODY_M, Role::OnSurface, label);
                ui.press_box(x, cy, iw, TOUCH, Action::SignerCheck(press));
                cy += TOUCH + GAP_SMALL;
            }
            cy - GAP_SMALL - y
        },
        &actions,
    );
}

/// The Result card, top down (SIGNER-CHECK §11.5): the fee bump, the
/// verdict in its tone, the facts, the findings, what is stated, the
/// differences, the actions. The words were made with the result; this
/// reads them.
fn result(s: &CheckState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(shown) = &s.shown else {
        return 0.0;
    };
    let mut cy = y;
    if let Some(b) = &shown.bump {
        section_label(ui, x, cy, EN.compare_fee_bump_label);
        cy += ui.line_as(TITLE_S) + GAP;
        cy += tone_card(ui, (x, cy, w), b.result, b.tone());
        for line in &b.lines {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, line) + GAP;
        }
        // The last fact's rule is the divider under the section.
        for f in &b.facts {
            cy += fact(ui, (x, cy, w), f);
        }
    }
    match &shown.verdict {
        Some((line, tone)) => cy += tone_card(ui, (x, cy, w), line, *tone),
        None => {
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_M,
                Role::OnSurfaceVariant,
                EN.signer_needs_two_psbts,
            ) + PAD;
        }
    }
    match &shown.facts {
        Facts::Check(facts) => {
            for (label, value) in facts {
                cy += kv(ui, x, cy, w, label, value, Role::OnSurface);
            }
        }
        Facts::Compare(facts) => {
            for f in facts {
                cy += fact(ui, (x, cy, w), f);
            }
        }
    }
    for (n, fp) in shown.seeds.iter().enumerate() {
        cy += seed_row(ui, (x, cy, w), fp, n);
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
    section_label(ui, x, cy, EN.compare_differences_label);
    cy += ui.line_as(TITLE_S) + GAP;
    for line in &shown.differences {
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, line) + GAP;
    }
    cy += GAP_SMALL;
    let mut actions = Vec::new();
    if shown.qr {
        actions.push((
            EN.decode_show_as_qr.to_string(),
            Style::Secondary,
            Action::SignerCheck(SignerPress::Qr),
        ));
    }
    actions.push((
        EN.signer_swap.to_string(),
        Style::Secondary,
        Action::SignerCheck(SignerPress::Swap),
    ));
    // A key the result names and that is not loaded: Add a key, then back
    // here.
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
