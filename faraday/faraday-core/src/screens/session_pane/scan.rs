//! The sidebar's Scan on the wide layout: the viewfinder, then what was
//! read, in the session pane's place, the page beside it live
//! (`docs/WORKSPACE.md` §4). The scan is the sheet's own state, one
//! camera; only where it is drawn differs. Nothing here holds a secret: the
//! frame and the card are drawn from the scan state and leave with it.

use super::{PAD, card_surface};
use crate::drawing::View;
use crate::screens::sheets::{read_card, read_card_leave, read_card_leaves, read_card_ways};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Faraday, Sheet};
use faraday_draw::tokens::{
    BODY_L, GAP, LIST_PAD, SCAN_PANE_ASPECT, SCAN_PANE_MIN_H, SCAN_PANE_RADIUS,
};

/// Whether the scan on screen stands in the session pane: the sidebar's
/// Scan, pressed on this page, the last frame having stood the pane beside
/// it, and no camera to choose between (the sheet has the chooser). A
/// lookup.
pub(crate) fn on(app: &Faraday) -> bool {
    !app.is_compact()
        && app.sheet == Some(Sheet::Scan)
        && app.cameras().len() <= 1
        && app.camera.pane_seen() == Some(app.screen)
        && app.scan().is_some_and(|s| s.pane == Some(app.screen))
}

/// The card where This session's stands among a menu page's sections
/// (Home), as tall as what it holds. Returns that height.
pub(super) fn block(app: &View, ui: &mut Ui, (x, y): (f32, f32), w: f32) -> f32 {
    let h = crate::menu::measure(ui, &mut |ui| content(app, ui, (x, y, w), f32::INFINITY));
    card_surface(ui, (x, y, w, h));
    content(app, ui, (x, y, w), h);
    h
}

/// What the pane holds while it scans, in a card `w` wide with at most
/// `room` of height: the title, the camera's picture with Paste and
/// Cancel under it, or the read card with its ways. Returns the height it
/// takes.
pub(super) fn content(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), room: f32) -> f32 {
    let Some(sc) = app.scan() else { return 0.0 };
    let (cx, cw) = (x + PAD + LIST_PAD, w - 2.0 * (PAD + LIST_PAD));
    let mut cy = y + PAD;
    cy += ui.wrap_as(
        (cx, cy, cw),
        BODY_L,
        Role::OnSurface,
        EN.scan_scan_a_qr_code,
    ) + GAP;
    if let Some(card) = sc.card.as_ref() {
        let stick_in = !app.may_load_keys();
        cy = read_card(ui, (x, cy - PAD, w), card, stick_in);
        let ways = read_card_ways(card, stick_in);
        if !ways.is_empty() {
            cy += crate::screens::button_rows(ui, cx, cy, cw, &ways) + GAP;
        }
        let words = (EN.read_again, read_card_leave(card));
        let leaves = read_card_leaves(words, stick_in);
        cy += crate::screens::button_rows(ui, cx, cy, cw, &leaves);
        return cy + PAD - y;
    }
    // Paste and the samples stand as in the sheet (`docs/QR.md` §8 item 5).
    crate::screens::scan_demo_offers(app, ui);
    let ways = [
        (EN.scan_paste, Style::Secondary, Action::ScanPaste),
        (EN.scan_cancel, Style::Secondary, Action::Cancel),
    ];
    let ways = &ways[usize::from(!app.scan_pastes())..];
    let under = ui.button_height(crate::screens::BUTTON_H) + GAP + PAD;
    let h = (cw * SCAN_PANE_ASPECT).min((room - (cy - y) - under).max(SCAN_PANE_MIN_H));
    let fw = h / SCAN_PANE_ASPECT;
    let fx = cx + (cw - fw) / 2.0;
    ui.fill(
        fx,
        cy,
        fw,
        h,
        SCAN_PANE_RADIUS,
        Role::SurfaceContainerHighest,
    );
    crate::screens::scan_frame::draw(ui, (fx, cy, fw, h), sc);
    cy += h + GAP;
    cy += crate::screens::button_rows(ui, cx, cy, cw, ways);
    cy + PAD - y
}
