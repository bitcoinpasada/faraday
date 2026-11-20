//! The quick paper backup's two screens, Write the words down and Check
//! my copy (`docs/BACKUP-FLOW.md` §19.2): the copy item split in two, one
//! card each, with More ways to back up under the item.

use super::*;
use crate::wallet_backup::quick::{Quick, Step};

/// The card of step `k` of the quick paper backup: its body, then the
/// foot at its end. Returns its height. A card fading out is the step
/// left, not the one now open.
pub(super) fn card(app: &View, ui: &mut Ui, k: u8, x: f32, y: f32, w: f32) -> f32 {
    let step = crate::wallet_backup::quick_step_at(k)
        .or_else(|| app.backup().map(|b| b.quick_step))
        .unwrap_or(Step::Write);
    let mut cy = y;
    cy += body(app, ui, step, x, cy, w);
    cy += foot(app, ui, step, x, cy, w);
    cy - y
}

/// Whether the copy item is done: the words typed back, and the SeedQR
/// scanned where the plan keeps one.
fn matched(app: &View) -> bool {
    app.backup_item_done(crate::plan::Item::Copy(0))
}

/// Whether the Krux code's check is done, by the board's own rule.
fn code_done(app: &View) -> bool {
    app.backup_item_done(crate::plan::Item::KefCode(0))
}

/// The open card's body: the screen's own, then More ways to back up.
fn body(app: &View, ui: &mut Ui, step: Step, x: f32, y: f32, w: f32) -> f32 {
    if app.backup().is_none() {
        return 0.0;
    }
    let mut cy = y;
    cy += match step {
        Step::Write => write_body(app, ui, x, cy, w),
        Step::Check => check_body(app, ui, x, cy, w),
        Step::Code => super::kef_code_block(app, ui, (x, cy, w), (0, false)) + T::GAP,
    };
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.quick_more_ways,
        Style::Ghost,
        Action::BQuick(Quick::More),
    );
    cy + BUTTON_H + T::GAP - y
}

/// Show the seed, the words' grid once shown, and under the grid the
/// place of the Blank template button.
fn write_body(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup() else {
        return 0.0;
    };
    let Some(key) = app.session.keys.info(b.key) else {
        return 0.0;
    };
    let word_count = usize::from(key.word_count.unwrap_or(0));
    let mut cy = y;
    // Show the seed, its SeedQR and the blank template, wrapping where
    // the column is narrow; the key's line under them.
    let (show, style, action) = if b.reveal {
        (EN.backup_hide_the_seed, Style::Secondary, Action::BReveal)
    } else {
        // The first press records that the seed was shown.
        (
            EN.backup_show_the_seed,
            Style::Tonal,
            Action::BQuick(Quick::Show),
        )
    };
    // The SeedQR shows with the seed, where the plan keeps one.
    let (qr, qr_style, qr_action) = if b.reveal && b.check.is_none() && app.copy_forms(b, 0).seedqr
    {
        (EN.quick_hide_seedqr, Style::Secondary, Action::BReveal)
    } else {
        (EN.quick_seedqr, Style::Tonal, Action::BQuick(Quick::SeedQr))
    };
    let mut actions = vec![
        (show, style, action),
        (qr, qr_style, qr_action),
        (
            EN.quick_template,
            Style::Ghost,
            Action::BQuick(Quick::Template),
        ),
    ];
    // The Krux code, once added, stands selected.
    if key.kef_offered() {
        let style = if b.quick_code {
            Style::Selected
        } else {
            Style::Tonal
        };
        actions.insert(2, (EN.backup_step_kef, style, Action::BQuick(Quick::Code)));
    }
    cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &actions);
    let (ix, iy) = (x, cy);
    let info = fill(
        EN.backup_words,
        &[
            &key_line(key.fingerprint, &key.label),
            &word_count.to_string(),
        ],
    );
    let info = ui.fit_as(BODY_S, &info, x + w - ix);
    ui.text_mid_as((ix, iy, BUTTON_H), BODY_S, Role::OnSurfaceVariant, &info);
    cy = iy + BUTTON_H + T::GAP;
    if b.reveal && b.check.is_none() {
        // The SeedQR first, where Show the SeedQR leaves the eye: the
        // words' grid is long.
        let seedqr = app.copy_forms(b, 0).seedqr;
        cy += seedqr_block(app, ui, (x, cy, w), seedqr, key.language);
        cy += super::super::parts::seed_word_grid(app, ui, (x, cy, w), b.key);
    }
    cy + T::GAP - y
}

/// Show the seed back to the first screen, and the typed check.
fn check_body(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup() else {
        return 0.0;
    };
    let mut cy = y;
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.backup_show_the_seed,
        Style::Tonal,
        Action::BQuick(Quick::Show),
    );
    cy += BUTTON_H + T::GAP;
    // The check left open waits; one closed with no match starts again.
    if b.check.is_none() && !matched(app) {
        ui.button(
            x,
            cy,
            None,
            BUTTON_H,
            EN.item_action_check_my,
            Style::Secondary,
            Action::BCheck,
        );
        cy += BUTTON_H + T::GAP;
    }
    cy += copy_check(app, ui, x, cy, w);
    cy + T::GAP - y
}

/// The foot: **I wrote them down** once the seed was shown, **Done** once
/// the words matched; a match not dated has Set the time first.
fn foot(app: &View, ui: &mut Ui, step: Step, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup() else {
        return 0.0;
    };
    let mut fy = y;
    if b.undated && !app.clock_checked() {
        ui.button(
            x,
            fy,
            None,
            BUTTON_H,
            EN.copy_check_set_the_time,
            Style::Secondary,
            Action::OpenSetTime,
        );
        fy += BUTTON_H + T::GAP + T::GAP_SMALL;
    }
    let next = match step {
        Step::Write if b.quick_shown => Some((EN.quick_wrote_them, Quick::Wrote)),
        Step::Check if matched(app) && b.quick_code => Some((EN.quick_next, Quick::Next)),
        Step::Check if matched(app) => Some((EN.backup_done, Quick::Done)),
        Step::Code if matched(app) && code_done(app) => Some((EN.backup_done, Quick::Done)),
        _ => None,
    };
    if let Some((label, q)) = next
        && next_button(ui, x, fy, w, label, Action::BQuick(q))
    {
        fy += T::TOUCH;
    }
    fy - y
}

/// The sheet of **Blank template**: the templates item's line, and Close.
pub(crate) fn template_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(b) = app.backup() else {
        return;
    };
    let line = templates_line(app, b);
    let actions = [(EN.nav_close, Style::Primary, Action::Cancel)];
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        (Icon::Info.into(), Role::Primary.paint(), EN.quick_template),
        &mut |ui, x, y, iw| ui.wrap_as((x, y, iw), BODY_M, Role::OnSurface, &line),
        &actions,
    );
}
