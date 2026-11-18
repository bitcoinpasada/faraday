//! The quick paper backup's two screens, Write the words down and Check
//! my copy (`docs/BACKUP-FLOW.md` §19.2): the copy item split in two, one
//! card each, with More ways to back up under the item.

use super::*;
use crate::wallet_backup::quick::{Quick, Step};

/// The screen, drawn in the column `col` [`draw`] of the checklist made.
pub(super) fn draw(
    app: &View,
    ui: &mut Ui,
    col: &flow::Column,
    (x0, cw, h): (f32, f32, f32),
    (panel_w, pick): (f32, bool),
) {
    let Some(b) = app.backup() else {
        return;
    };
    let title = match b.quick_step {
        Step::Write => EN.quick_write_title,
        Step::Check => EN.quick_check_title,
    };
    let cards = [flow::Card {
        title: String::from(title),
        summary: String::new(),
        mono: false,
        done: false,
        open: true,
        default: false,
        toggle: Action::BQuick(Quick::Card),
        lead: None,
    }];
    let mut body = |ui: &mut Ui, _: usize, x: f32, y: f32, w: f32| body(app, ui, x, y, w);
    let mut foot = |ui: &mut Ui, x: f32, y: f32, w: f32| foot(app, ui, x, y, w);
    let (next, again) =
        flow::column_wide(ui, col, None, &cards, b.scroll, &mut body, Some(&mut foot));
    app.scroll(Scrolled::Backup, next, again);
    if !ui.compact {
        backup_panel(app, ui, x0 + cw - panel_w, panel_w, h);
    }
    if pick && let Some((cx, cy, chip_w)) = ui.chip_at {
        let right = if ui.compact {
            x0 + cw
        } else {
            x0 + cw - panel_w
        } - T::PAD;
        wallet_pick(app, ui, cx, cy + T::GAP_SMALL, chip_w, right);
    }
}

/// Whether the key's words matched in this backup.
fn matched(app: &View) -> bool {
    let Some(b) = app.backup() else {
        return false;
    };
    app.session.keys.info(b.key).is_some_and(|k| {
        b.checked
            .contains(&(k.fingerprint.0, crate::plan::CopyForm::Words))
    })
}

/// The open card's body: the screen's own, then More ways to back up.
fn body(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup() else {
        return 0.0;
    };
    let mut cy = y;
    cy += match b.quick_step {
        Step::Write => write_body(app, ui, x, cy, w),
        Step::Check => check_body(app, ui, x, cy, w),
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
    let bw = ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        if b.reveal {
            EN.backup_hide_the_seed
        } else {
            EN.backup_show_the_seed
        },
        if b.reveal {
            Style::Secondary
        } else {
            Style::Tonal
        },
        // The first press records that the seed was shown.
        if b.reveal {
            Action::BReveal
        } else {
            Action::BQuick(Quick::Show)
        },
    );
    // The Blank template button goes beside Show the seed, after `bw`.
    let (ix, iy) = if ui.compact {
        (x, cy + T::TOUCH)
    } else {
        (x + bw + T::GAP, cy)
    };
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
fn foot(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
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
    let next = match b.quick_step {
        Step::Write if b.quick_shown => Some((EN.quick_wrote_them, Quick::Wrote)),
        Step::Check if matched(app) => Some((EN.backup_done, Quick::Done)),
        _ => None,
    };
    if let Some((label, q)) = next
        && next_button(ui, x, fy, w, label, Action::BQuick(q))
    {
        fy += T::TOUCH;
    }
    fy - y
}
