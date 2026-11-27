//! The screen template: the navigation, the top app bar, the page, the sheets over it, the
//! on-screen keyboard, the snackbar and the tooltip (04 §4, §12.4). Its layouts: the small panel
//! (`Ui::compact`) has the page between the top app bar and the navigation bar at its foot, or
//! beside a rail; the wide window has the sidebar at its left and the page under the top app bar.
//! A strip of stick pills stands across the top where the model has one.

mod pills;

pub use pills::{PillsModel, PillsPlace, StripPill, bar_room as pills_bar_room};

use faraday_atoms::{Anchor, Pressed, Ui};
use faraday_molecules::status::{self, BusyModel};
use faraday_organisms::keyboard::{self, KeyboardModel};
use faraday_organisms::nav::{
    self, NavModel, RailModel, SearchBarModel, SidebarModel, TopBarModel, WaysModel,
};
use faraday_organisms::snackbar::{self, SnackbarModel};
use faraday_organisms::tooltip::{self, TipModel};
use faraday_tokens::dims::{
    APP_BAR_H, GAP_SMALL, MARGIN_COMPACT, MENU_GAP, NAV_BAR_H, RAIL_W, SEARCH_BAR_H,
    SEARCH_FIELD_MAX,
};
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::TITLE_L;

/// The search the top app bar holds on the wide layout, with Home's first actions beside it. The
/// small panel's page draws its own (the menu-page template's).
pub struct BarSearch<'s, A> {
    pub field: SearchBarModel<'s, A>,
    pub ways: Option<WaysModel<'s, A>>,
}

/// What stands at the window's left edge, beside the page.
pub enum Side<'s, R> {
    /// Nothing: the small panel's page is the whole width.
    None,
    /// The rail of a window of the rail's class (`RAIL_W` wide).
    Rail(RailModel<'s, R>),
    /// The wide window's sidebar (its own width).
    Sidebar(SidebarModel<'s, R>),
}

/// The on-screen keyboard: on the small panel it takes the foot and the page stands above it; on
/// the wide window it stands over the page's foot, which keeps its height and is drawn `lift`
/// units up to keep the focused field in view.
pub struct Keyboard<'s, R> {
    pub model: KeyboardModel<'s, R>,
    pub lift: f32,
}

/// The screen template's model: the navigation, the top app bar with its pills, the busy line.
/// The template chooses the bar or, on the wide layout, none, by the room (04 V10).
pub struct FrameModel<'s, R> {
    pub nav: NavModel<'s, R>,
    pub top: TopBarModel<'s, R>,
    pub busy: Option<BusyModel<'s>>,
    /// The search in the top app bar, wide layout only.
    pub search: Option<BarSearch<'s, R>>,
    /// The rail or the sidebar.
    pub side: Side<'s, R>,
    /// The keyboard, where one is up.
    pub keyboard: Option<Keyboard<'s, R>>,
    /// The stick pills, in a strip or in the top app bar.
    pub pills: Option<PillsModel<'s, R>>,
    /// The tooltip on show (`Tips::model` over the anchors the last frame carried).
    pub tip: Option<TipModel<'s>>,
}

/// The room the page has, `(width, height)` in units, read inside the body closure of [`draw`]:
/// the area between the top app bar and what stands at the foot.
pub fn page_room<A: Pressed>(ui: &Ui<'_, A>) -> (f32, f32) {
    let c = ui.seen_clip();
    (c.w as f32 / ui.f, c.h as f32 / ui.f)
}

/// Draws the frame: the page's anchor, the page under the sheets, the sheets over it (each with
/// its anchor, the lowest first), the keyboard, the note on show and the tooltip. A sheet's
/// presses are the only ones taken: what is under it is frosted and its hits are cleared.
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

    // The strip of pills across the top; everything else stands below it.
    let strip = frame
        .pills
        .as_ref()
        .map_or(0.0, |p| pills::strip_height(p, ui, w));
    let side_w = match &frame.side {
        Side::None => 0.0,
        Side::Rail(_) => RAIL_W,
        Side::Sidebar(s) => s.w,
    };
    let area_w = w - side_w;
    let kb_h = frame
        .keyboard
        .as_ref()
        .map_or(0.0, |k| keyboard::height(k.model.kind, ui.compact));
    // The small panel's keyboard takes the foot, so the page stands above it; the wide one
    // stands over the page.
    let kb_foot = if ui.compact { kb_h } else { 0.0 };

    // What stands at the foot, under the page: the navigation bar on the small panel (not while
    // the keyboard takes the foot), then the busy line.
    let bar_h = if ui.compact && kb_h == 0.0 && !frame.nav.dests.is_empty() {
        NAV_BAR_H
    } else {
        0.0
    };
    let busy_h = frame
        .busy
        .as_ref()
        .map_or(0.0, |_| status::busy_height(ui) + MENU_GAP);
    let top = APP_BAR_H;
    let page_h = (h - strip - kb_foot - bar_h - busy_h - top).max(0.0);

    // The page, under the top app bar, which the page scrolls under, beside the rail or sidebar.
    let (ox, oy) = (ui.ox, ui.oy);
    let lift = frame.keyboard.as_ref().map_or(0.0, |k| k.lift);
    ui.ox = ox + side_w;
    ui.oy = oy + strip;
    let clip = ui.rect(0.0, top, area_w, page_h);
    ui.c.push_clip(clip);
    ui.oy += top - if ui.compact { 0.0 } else { lift };
    ui.anchor(page);
    body(ui);
    ui.c.pop_clip();

    if let Some(busy) = &frame.busy {
        let y = h - strip - kb_foot - bar_h - busy_h + MENU_GAP / 2.0;
        ui.oy = oy + strip;
        status::draw_busy(busy, ui, (MARGIN_COMPACT, y, area_w - 2.0 * MARGIN_COMPACT));
    }
    ui.ox = ox;
    ui.oy = oy + strip;
    if bar_h > 0.0 {
        nav::draw_bar(&frame.nav, ui, (h - strip - kb_foot - bar_h, w));
    }
    match &frame.side {
        Side::None => {}
        Side::Rail(r) => nav::draw(r, ui, h - strip),
        Side::Sidebar(s) => nav::draw_sidebar(s, ui, h - strip),
    }

    // The top app bar over the page, the search in it on the wide layout, the pills in it where
    // they stand there.
    ui.ox = ox + side_w;
    nav::draw_top(&frame.top, ui, area_w);
    if let (false, Some(search)) = (ui.compact, &frame.search) {
        let back = if frame.top.back.is_some() { 48.0 } else { 0.0 };
        let name = ui.measure_as(TITLE_L, frame.top.title.as_str());
        let from = MARGIN_COMPACT + back + name + frame.top.pill_w + MENU_GAP;
        let to = frame.top.right - GAP_SMALL;
        nav::draw_group(
            &search.field,
            search.ways.as_ref(),
            ui,
            ((from, to), area_w / 2.0),
            ((APP_BAR_H - SEARCH_BAR_H) / 2.0, SEARCH_FIELD_MAX),
        );
    }
    if let Some(p) = frame
        .pills
        .as_ref()
        .filter(|p| matches!(p.place, PillsPlace::Bar { .. }))
    {
        pills::draw(p, ui, area_w);
    }
    // The strip, over the page and under any sheet, which frosts it.
    ui.ox = ox;
    ui.oy = oy;
    if let Some(p) = frame
        .pills
        .as_ref()
        .filter(|p| matches!(p.place, PillsPlace::Strip))
    {
        pills::draw(p, ui, w);
    }

    // The sheets, the lowest first: each frosts what is under it and takes the presses alone.
    for (id, sheet) in over {
        ui.clear_hits();
        ui.backdrop(*id);
        ui.add_sheet(*id);
        ui.layer += 1;
        draw_over(sheet, ui);
    }
    // The keyboard over everything the page and its sheets drew, at the foot of the page's area.
    if let Some(k) = &frame.keyboard {
        keyboard::draw(&k.model, ui, (side_w, h - kb_h, area_w));
    }
    if let Some(note) = note {
        let foot = kb_foot + bar_h + busy_h;
        snackbar::draw_at(note, ui, (w, h - foot));
    }
    if let Some(tip) = &frame.tip {
        tooltip::draw(tip, ui);
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
            side: Side::None,
            keyboard: None,
            pills: None,
            tip: None,
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

    fn room_of(m: &FrameModel<'_, P>, compact: bool) -> ((f32, f32), faraday_atoms::FrameOut<P>) {
        let mut c = canvas(160);
        let mut room = (0.0, 0.0);
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = compact;
            draw(
                anchor(1),
                m,
                ui,
                |ui| room = page_room(ui),
                &[] as &[(Anchor, ())],
                |_, _| {},
                None,
            );
        });
        (room, out)
    }

    /// Invariant: A rail at the window's left leaves the page the width beside it, and takes its destinations' presses.
    /// Regression: The page drawn under the rail, or the rail's destinations unpressable.
    /// Owner: kit
    #[test]
    fn a_rail_takes_its_width_from_the_page() {
        let plain = model("Home", 0);
        let (bare, _) = room_of(&plain, true);
        let mut m = model("Home", 0);
        let mut rail_nav = model("Home", 2).nav;
        rail_nav.many = Shown::plain("999+");
        m.side = Side::Rail(RailModel {
            nav: rail_nav,
            under: Vec::new(),
            foot: Vec::new(),
            button: None,
            mark: false,
        });
        let (room, out) = room_of(&m, true);
        assert!(room.0 < bare.0);
        assert!(out.hits.iter().any(|h| h.press == P(0)));
        assert!(out.hits.iter().any(|h| h.press == P(1)));
    }

    /// Invariant: On the small panel the keyboard takes the foot and the page stands above it; its keys take presses.
    /// Regression: The keyboard drawn over the page's last rows, or its keys unpressable.
    /// Owner: kit
    #[test]
    fn the_small_panels_keyboard_takes_the_foot() {
        fn act(_: keyboard::OskPress) -> P {
            P(9)
        }
        let (bare, _) = room_of(&model("Home", 0), true);
        let mut m = model("Home", 0);
        m.keyboard = Some(Keyboard {
            model: KeyboardModel {
                kind: keyboard::KeyboardKind::Pin,
                keys: keyboard::Keys::default(),
                compact: true,
                done: faraday_atoms::Shown::plain("Done"),
                enabled: keyboard::ALL_KEYS,
                scramble: None,
                act,
            },
            lift: 0.0,
        });
        let (room, out) = room_of(&m, true);
        let kb = keyboard::height(keyboard::KeyboardKind::Pin, true);
        assert!((bare.1 - room.1 - kb).abs() < 0.5);
        assert!(out.hits.iter().any(|h| h.press == P(9)));
    }

    /// Invariant: A strip of stick pills across the top pushes the page down by its height, and a pill with a press takes it.
    /// Regression: Pills drawn over the page's first rows, or unpressable where the shell's sticks are its own.
    /// Owner: kit
    #[test]
    fn a_strip_of_pills_stands_above_the_page() {
        let (bare, _) = room_of(&model("Home", 0), true);
        let mut m = model("Home", 0);
        m.pills = Some(PillsModel {
            pills: vec![StripPill {
                n: 1,
                state: faraday_molecules::stick_pill::StickState::Inserted,
                long: Shown::plain("Stick 1 in"),
                short: Shown::plain("Stick 1"),
                number: Shown::plain("1"),
                offered: faraday_atoms::Offered::Yes(P(7)),
            }],
            place: PillsPlace::Strip,
        });
        let (room, out) = room_of(&m, true);
        assert!(room.1 < bare.1);
        assert!(out.hits.iter().any(|h| h.press == P(7)));
    }
}
