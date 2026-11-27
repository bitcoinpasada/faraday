//! The screen template: the navigation, the top app bar, the page, the sheets over it and the
//! snackbar (04 §4, §12.4). Its layouts: the small panel (`Ui::compact`) has the page between the
//! top app bar and the navigation bar at its foot; the wide window has the page under the top app
//! bar. The wide window's rail and the sidebar are not built (the organisms' `nav::draw` is not),
//! so the wide layout draws no navigation.

use faraday_atoms::{Anchor, Pressed, Ui};
use faraday_molecules::status::{self, BusyModel};
use faraday_organisms::nav::{self, NavModel, SearchBarModel, TopBarModel, WaysModel};
use faraday_organisms::snackbar::{self, SnackbarModel};
use faraday_tokens::dims::{
    APP_BAR_H, GAP_SMALL, MARGIN_COMPACT, MENU_GAP, NAV_BAR_H, SEARCH_BAR_H, SEARCH_FIELD_MAX,
};
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::TITLE_L;

/// The search the top app bar holds on the wide layout, with Home's first actions beside it. The
/// small panel's page draws its own (the menu-page template's).
pub struct BarSearch<'s, A> {
    pub field: SearchBarModel<'s, A>,
    pub ways: Option<WaysModel<'s, A>>,
}

/// The screen template's model: the navigation, the top app bar with its pills, the busy line.
/// The template chooses the bar or, on the wide layout, none, by the room (04 V10).
pub struct FrameModel<'s, R> {
    pub nav: NavModel<'s, R>,
    pub top: TopBarModel<'s, R>,
    pub busy: Option<BusyModel<'s>>,
    /// The search in the top app bar, wide layout only.
    pub search: Option<BarSearch<'s, R>>,
}

/// The room the page has, `(width, height)` in units, read inside the body closure of [`draw`]:
/// the area between the top app bar and what stands at the foot.
pub fn page_room<A: Pressed>(ui: &Ui<'_, A>) -> (f32, f32) {
    let c = ui.seen_clip();
    (c.w as f32 / ui.f, c.h as f32 / ui.f)
}

/// Draws the frame: the page's anchor, the page under the sheets, the sheets over it (each with
/// its anchor, the lowest first) and the note on show. A sheet's presses are the only ones
/// taken: what is under it is frosted and its hits are cleared.
pub fn draw<'s, R: Pressed, O>(
    page: Anchor,
    frame: &FrameModel<'s, R>,
    ui: &mut Ui<'_, R>,
    body: impl FnOnce(&mut Ui<'_, R>),
    over: &[(Anchor, O)],
    draw_over: impl Fn(&O, &mut Ui<'_, R>),
    note: Option<&SnackbarModel<'s>>,
) {
    let (w, h) = (ui.c.width() as f32 / ui.f, ui.c.height() as f32 / ui.f);
    ui.set_page(page);
    ui.clear(Role::Surface);

    // What stands at the foot, under the page: the navigation bar on the small panel, then the
    // busy line.
    let bar_h = if ui.compact && !frame.nav.dests.is_empty() {
        NAV_BAR_H
    } else {
        0.0
    };
    let busy_h = frame
        .busy
        .as_ref()
        .map_or(0.0, |_| status::busy_height(ui) + MENU_GAP);
    let top = APP_BAR_H;
    let page_h = (h - bar_h - busy_h - top).max(0.0);

    // The page, under the top app bar, which the page scrolls under.
    let clip = ui.rect(0.0, top, w, page_h);
    ui.c.push_clip(clip);
    let oy = ui.oy;
    ui.oy = oy + top;
    ui.anchor(page);
    body(ui);
    ui.oy = oy;
    ui.c.pop_clip();

    if let Some(busy) = &frame.busy {
        let y = h - bar_h - busy_h + MENU_GAP / 2.0;
        status::draw_busy(busy, ui, (MARGIN_COMPACT, y, w - 2.0 * MARGIN_COMPACT));
    }
    if bar_h > 0.0 {
        nav::draw_bar(&frame.nav, ui, (h - bar_h, w));
    }

    // The top app bar over the page, the search in it on the wide layout.
    nav::draw_top(&frame.top, ui, w);
    if let (false, Some(search)) = (ui.compact, &frame.search) {
        let back = if frame.top.back.is_some() { 48.0 } else { 0.0 };
        let name = ui.measure_as(TITLE_L, frame.top.title.as_str());
        let from = MARGIN_COMPACT + back + name + frame.top.pill_w + MENU_GAP;
        let to = frame.top.right - GAP_SMALL;
        nav::draw_group(
            &search.field,
            search.ways.as_ref(),
            ui,
            ((from, to), w / 2.0),
            ((APP_BAR_H - SEARCH_BAR_H) / 2.0, SEARCH_FIELD_MAX),
        );
    }

    // The sheets, the lowest first: each frosts what is under it and takes the presses alone.
    for (id, sheet) in over {
        ui.clear_hits();
        ui.backdrop(*id);
        ui.add_sheet(*id);
        ui.layer += 1;
        draw_over(sheet, ui);
    }
    if let Some(note) = note {
        let foot = bar_h + busy_h;
        snackbar::draw_at(note, ui, (w, h - foot));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::Shown;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};
    use faraday_organisms::nav::Dest;

    fn model<'s>(title: &'static str, dests: usize) -> FrameModel<'s, P> {
        FrameModel {
            nav: NavModel {
                dests: (0..dests)
                    .map(|i| Dest {
                        icon: faraday_atoms::Icon::SEARCH.into(),
                        label: Shown::plain("Home"),
                        offered: faraday_atoms::Offered::Yes(P(i as u8)),
                        current: i == 0,
                        badge: nav::Badge::None,
                        flows: 0,
                        bar: true,
                    })
                    .collect(),
                many: Shown::plain("999+"),
            },
            top: TopBarModel {
                back: None,
                title: Shown::plain(title),
                pill_w: 0.0,
                item_name: false,
                in_sheet: false,
                under: None,
                right: 640.0,
                power: None,
            },
            busy: None,
            search: None,
        }
    }

    fn anchor(n: u32) -> Anchor {
        Anchor { scope: "t", n }
    }

    /// Invariant: A press on the page under a sheet is not taken while the sheet is open; the sheet's own press is.
    /// Regression: A page's buttons stay pressable through the scrim of a sheet over it.
    /// Owner: kit
    #[test]
    fn a_page_under_a_sheet_takes_no_press() {
        let mut c = canvas(160);
        let m = model("Home", 0);
        let out = frame(&mut c, &at_rest(None), |ui| {
            draw(
                anchor(1),
                &m,
                ui,
                |ui| {
                    ui.press(
                        faraday_atoms::hits::Shape::new(10.0, 10.0, 80.0, 40.0, 4.0),
                        faraday_atoms::ui::testing::page(),
                        P(1),
                    )
                },
                &[(anchor(2), P(2))],
                |p, ui| {
                    ui.press(
                        faraday_atoms::hits::Shape::new(10.0, 300.0, 80.0, 40.0, 4.0),
                        faraday_atoms::ui::testing::page(),
                        *p,
                    )
                },
                None,
            );
        });
        let presses: Vec<P> = out.hits.iter().map(|h| h.press).collect();
        assert!(!presses.contains(&P(1)));
        assert!(presses.contains(&P(2)));
        assert_eq!(out.sheets.len(), 1);
    }

    /// Invariant: On the small panel the navigation bar stands at the foot and takes the destinations' presses; the page's room is what it leaves.
    /// Regression: The bar drawn over the page's last rows, or its destinations unpressable.
    /// Owner: kit
    #[test]
    fn the_small_panels_bar_takes_each_destinations_press() {
        let mut c = canvas(160);
        let m = model("Home", 3);
        let mut room = (0.0, 0.0);
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = true;
            draw(
                anchor(1),
                &m,
                ui,
                |ui| room = page_room(ui),
                &[] as &[(Anchor, ())],
                |_, _| {},
                None,
            );
        });
        for i in 0..3u8 {
            assert!(out.hits.iter().any(|h| h.press == P(i)));
        }
        assert!(room.1 > 0.0 && room.1 < 480.0);
    }
}
