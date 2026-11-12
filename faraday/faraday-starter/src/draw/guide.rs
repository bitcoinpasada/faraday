//! The Guide: its destination, the Starter's guide pages and the start
//! guides, and the sheet that shows a page of the guide
//! (`docs/GUIDE-IN-APP.md` §4.2, §6).

use super::*;
use crate::GUIDE_PLACES;
use faraday_guide::Span;

/// The Guide destination: the Starter's guide pages under their
/// heading, each a row that opens the page at its lead, then the start
/// guides under theirs. Returns where it ends.
pub(super) fn page(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    y += label(ui, x, y, S.starter_guide_head_starter);
    for (n, place) in GUIDE_PLACES.iter().enumerate() {
        let (Some(page), Ok(n)) = (faraday_guide::guide_page(place), u8::try_from(n)) else {
            continue;
        };
        y += nav_row(
            ui,
            x,
            y,
            w,
            Icon::Learn,
            &page.title,
            "",
            "",
            Some(StarterAction::GuidePage(n)),
        );
    }
    y += GAP;
    y += label(ui, x, y, S.starter_guide_head_start);
    for id in starter.start_guides() {
        y += guide_line(ui, x, y, w, id);
    }
    y
}

/// A paragraph of the guide in Body M, its bold spans in the medium
/// face, flowing as words from the line's left. Returns its height with
/// the gap under it.
fn paragraph(ui: &mut Ui, (x, y, w): (f32, f32, f32), para: &[Span]) -> f32 {
    let h = faraday_ui::flow::word_flow(
        ui,
        (x, y, w),
        BODY_M,
        para.iter().map(|s| (s.text.as_str(), s.bold)),
    );
    h + GAP / 2.0
}

/// The Guide sheet: in the Starter's look (`faraday_ui::sheet::scroll_sheet`),
/// the window's width less its margin on a compact window, centred and
/// at most `DIALOG_MAX` wide otherwise; the page of the guide the open
/// section belongs to, its title in Title L and its sections' headings
/// in Title S, and **Done** at its foot. Where its section stands is
/// noted for the scroll that opening it does (`Starter::render`); a
/// press beside the sheet closes it.
pub(super) fn sheet(starter: &Starter, ui: &mut Ui, w: f32, h: f32) {
    let margin = if starter.compact() {
        MARGIN_COMPACT
    } else {
        MARGIN_WIDE
    };
    let sw = if starter.compact() {
        w - 2.0 * margin
    } else {
        (w - 2.0 * margin).min(faraday_draw::tokens::DIALOG_MAX)
    };
    let key = starter.guide_at();
    let place = key.split('.').next().unwrap_or(key);
    let page = faraday_guide::guide_page(place);
    let outside = ui.outside.replace(StarterAction::Cancel.host());
    faraday_ui::sheet::scroll_sheet(
        ui,
        ((w - sw) / 2.0, sw),
        h,
        margin,
        faraday_draw::tokens::PAD,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += faraday_ui::sheet::sheet_head(
                ui,
                x,
                cy,
                iw,
                Icon::Learn,
                Role::OnSurface,
                S.starter_row_guide,
            );
            if let Some(page) = page {
                cy += ui.wrap_as((x, cy, iw), TITLE_L, Role::OnSurface, &page.title) + GAP;
                for (i, section) in page.sections.iter().enumerate() {
                    if section.key == key {
                        // The lead is the top of the page, its title.
                        let top = if i == 0 { 0.0 } else { cy - y };
                        starter.set_guide_top(top);
                    }
                    if i > 0 {
                        cy += ui.wrap_as((x, cy, iw), TITLE_S, Role::OnSurface, &section.heading)
                            + GAP / 2.0;
                    }
                    for para in &section.paragraphs {
                        cy += paragraph(ui, (x, cy, iw), para);
                    }
                    cy += GAP / 2.0;
                }
            }
            // Pinned to the sheet's foot.
            faraday_ui::sheet::buttons(
                ui,
                x,
                cy,
                iw,
                &[(S.starter_done, Style::Ghost, StarterAction::Cancel.host())],
            );
            cy - y
        },
    );
    ui.outside = outside;
}

/// The start guide sheet: the guide's own sheet ([`sheet`]) for a page
/// of the website's guides to starting a computer from the stick: its
/// title in Title L, its sections' headings in Title S, a step's number
/// in a circle with its text beside it, and **Done** at its foot.
pub(super) fn start_sheet(starter: &Starter, ui: &mut Ui, w: f32, h: f32) {
    let margin = if starter.compact() {
        MARGIN_COMPACT
    } else {
        MARGIN_WIDE
    };
    let sw = if starter.compact() {
        w - 2.0 * margin
    } else {
        (w - 2.0 * margin).min(faraday_draw::tokens::DIALOG_MAX)
    };
    let Some(page) = starter.start_shown().and_then(StartGuide::page) else {
        return;
    };
    let outside = ui.outside.replace(StarterAction::Cancel.host());
    faraday_ui::sheet::scroll_sheet(
        ui,
        ((w - sw) / 2.0, sw),
        h,
        margin,
        faraday_draw::tokens::PAD,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += faraday_ui::sheet::sheet_head(
                ui,
                x,
                cy,
                iw,
                Icon::Learn,
                Role::OnSurface,
                S.starter_row_guide,
            );
            cy += ui.wrap_as((x, cy, iw), TITLE_L, Role::OnSurface, page.title) + GAP;
            for section in page.sections {
                if !section.heading.is_empty() {
                    cy += ui.wrap_as((x, cy, iw), TITLE_S, Role::OnSurface, section.heading)
                        + GAP / 2.0;
                }
                // What follows a step, up to the next, is under it.
                let mut under = false;
                for para in section.paragraphs {
                    if let Some((n, text)) = faraday_guide::step(para) {
                        under = true;
                        let line = ui.line_as(BODY_M);
                        let circle_y = cy + (line - faraday_ui::parts::STEP_SIDE) / 2.0;
                        faraday_ui::parts::step_circle::<faraday_core::Action>(
                            ui, x, circle_y, n, false, None,
                        );
                        let at = (
                            x + faraday_ui::parts::STEP_INDENT,
                            cy,
                            iw - faraday_ui::parts::STEP_INDENT,
                        );
                        let h = ui.wrap_as(at, BODY_M, Role::OnSurface, text);
                        cy += h.max(faraday_ui::parts::STEP_SIDE + circle_y - cy) + GAP / 2.0;
                        continue;
                    }
                    let (px, pw) = if under {
                        (
                            x + faraday_ui::parts::STEP_INDENT,
                            iw - faraday_ui::parts::STEP_INDENT,
                        )
                    } else {
                        (x, iw)
                    };
                    cy += ui.wrap_as((px, cy, pw), BODY_M, Role::OnSurface, para) + GAP / 2.0;
                }
                cy += GAP / 2.0;
            }
            faraday_ui::sheet::buttons(
                ui,
                x,
                cy,
                iw,
                &[(S.starter_done, Style::Ghost, StarterAction::Cancel.host())],
            );
            cy - y
        },
    );
    ui.outside = outside;
}
