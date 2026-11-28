//! The screen template: the navigation, the top app bar, the page, the sheets over it, the
//! on-screen keyboard, the snackbar and the tooltip (04 §4, §12.4). Its layouts: the small panel
//! (`Ui::compact`) has the page between the top app bar and the navigation bar at its foot, or
//! beside a rail; the wide window has the sidebar at its left and the page under the top app bar.
//! A strip of stick pills stands across the top where the model has one.

mod pills;
mod selftest;

pub use pills::{PillsModel, PillsPlace, StripPill};
pub use selftest::{SelftestFailed, draw_selftest_failed};

use faraday_atoms::hits::Shape;
use faraday_atoms::{Anchor, Icon, Pressed, Shown, Ui};
use faraday_molecules::status::{self, BusyModel};
use faraday_organisms::keyboard::{self, KeyboardModel};
use faraday_organisms::nav::{
    self, NavModel, RailModel, SearchBarModel, SidebarModel, TopBarModel, WaysModel,
};
use faraday_organisms::snackbar::{self, SnackbarModel};
use faraday_organisms::tooltip::{self, TipModel};
use faraday_tokens::dims::{
    APP_BAR_H, GAP_SMALL, KEYBOARD_BUTTON, KEYBOARD_BUTTON_GLYPH, KEYBOARD_BUTTON_TIP,
    KEYBOARD_BUTTON_Y, MARGIN_COMPACT, MENU_GAP, NAV_BAR_H, RAIL_W, SEARCH_BAR_H, SEARCH_FIELD_MAX,
};
use faraday_tokens::palette::{Role, on_selected, selected_or};
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

/// A note that stays until it is closed (what was plugged in at start, the keyboards waiting):
/// its words and the press of its ×.
pub struct Notice<'s, R> {
    pub text: Shown<'s>,
    pub close: R,
}

/// The wide window's keyboard button, in the top row with its right edge at `right`: it brings
/// the docked keyboard up and puts it away. `shown` is whether the keyboard is up.
pub struct KeyboardButton<'s, R> {
    pub shown: bool,
    pub press: R,
    pub tip: Shown<'s>,
    pub right: f32,
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
    /// The notices that stay, over the page and any sheet at the window's foot, the first lowest
    /// and the rest above it, the page's note above them all.
    pub notices: Vec<Notice<'s, R>>,
    /// The keyboard button of a wide window where a sheet has a field of its own.
    pub keyboard_button: Option<KeyboardButton<'s, R>>,
    /// A held app's frame (the holder draws the top app bar and the way in): no sidebar, rail,
    /// navigation bar or top app bar of its own, the page the whole window; the press is the
    /// holder's way back, the one Back and Escape take.
    pub held: Option<R>,
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
    // A held app has no frame of its own beside the page and above it.
    let held = frame.held.is_some();
    let side_w = match &frame.side {
        _ if held => 0.0,
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
    let bar_h = if !held && ui.compact && kb_h == 0.0 && !frame.nav.dests.is_empty() {
        NAV_BAR_H
    } else {
        0.0
    };
    let busy_h = frame
        .busy
        .as_ref()
        .map_or(0.0, |_| status::busy_height(ui) + MENU_GAP);
    let top = if held { 0.0 } else { APP_BAR_H };
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
    if let Some(back) = &frame.held {
        ui.back(back.clone());
    }
    if bar_h > 0.0 {
        nav::draw_bar(&frame.nav, ui, (h - strip - kb_foot - bar_h, w));
    }
    match &frame.side {
        _ if held => {}
        Side::None => {}
        Side::Rail(r) => nav::draw(r, ui, h - strip),
        Side::Sidebar(s) => nav::draw_sidebar(s, ui, h - strip),
    }

    // The top app bar over the page, the search in it on the wide layout, the pills in it where
    // they stand there.
    ui.ox = ox + side_w;
    let taken = frame.pills.as_ref().map_or(0.0, |p| pills::bar_room(p, ui));
    if !held {
        nav::draw_top_less(&frame.top, ui, area_w, taken);
    }
    if let (false, false, Some(search)) = (held, ui.compact, &frame.search) {
        let back = if frame.top.back.is_some() { 48.0 } else { 0.0 };
        let name = ui.measure_as(TITLE_L, frame.top.title.as_str());
        let from = MARGIN_COMPACT + back + name + frame.top.pill_w + MENU_GAP;
        let to = frame.top.right - taken - GAP_SMALL;
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
    // A sheet with a field of its own: the keyboard button stays within reach, over it.
    if let Some(b) = &frame.keyboard_button {
        ui.ox = ox;
        keyboard_button(b, ui);
    }
    // The keyboard over everything the page and its sheets drew, at the foot of the page's area.
    if let Some(k) = &frame.keyboard {
        keyboard::draw(&k.model, ui, (side_w, h - kb_h, area_w));
    }
    // What was plugged in at start, over the page and any sheet at the foot of the window (above
    // the keyboard), the first lowest; the page's note above them.
    ui.ox = ox;
    ui.oy = oy;
    let mut notices_foot = h - kb_h;
    for n in &frame.notices {
        let m = SnackbarModel {
            text: n.text.clone(),
        };
        notices_foot -= snackbar::draw_held(&m, ui, (w, notices_foot), n.close.clone());
    }
    if let Some(note) = note {
        let foot = (h - kb_foot - bar_h - busy_h).min(notices_foot);
        snackbar::draw_at(note, ui, (w, foot));
    }
    if let Some(tip) = &frame.tip {
        tooltip::draw(tip, ui);
    }
}

/// The wide window's keyboard button: a circle in the top row, its right edge at the model's
/// `right`, filled and edged in the selected tone while the keyboard is up.
fn keyboard_button<R: Pressed>(b: &KeyboardButton<'_, R>, ui: &mut Ui<'_, R>) {
    let (s, y) = (KEYBOARD_BUTTON, KEYBOARD_BUTTON_Y);
    let x = b.right - s;
    let tint = if b.shown {
        Role::SecondaryContainer
    } else {
        Role::Surface
    };
    ui.fill(x, y, s, s, s / 2.0, tint);
    ui.stroke(
        x,
        y,
        s,
        s,
        s / 2.0,
        selected_or(b.shown, Role::OutlineVariant),
    );
    ui.press(
        Shape::new(x, y, s, s, s / 2.0),
        Role::OnSurface.paint(),
        b.press.clone(),
    );
    ui.icon(
        x,
        y,
        s,
        Icon::Keyboard,
        KEYBOARD_BUTTON_GLYPH,
        on_selected(b.shown, Role::OnSurfaceVariant),
    );
    ui.tip_plain(
        (
            x - KEYBOARD_BUTTON_TIP,
            y - KEYBOARD_BUTTON_TIP,
            s + 2.0 * KEYBOARD_BUTTON_TIP,
            s + 2.0 * KEYBOARD_BUTTON_TIP,
        ),
        &b.press,
        b.tip.as_str(),
    );
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
            notices: Vec::new(),
            keyboard_button: None,
            held: None,
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

    /// Invariant: A held app's frame draws no sidebar, top app bar or navigation bar: the page has the whole window, and the holder's way back is the frame's back link.
    /// Regression: A held page drawn with its own sidebar or bar beside the holder's, or without the way back Back and Escape take.
    /// Owner: kit
    #[test]
    fn a_held_frame_gives_the_page_the_whole_window() {
        let (bare, _) = room_of(&model("Home", 3), true);
        let mut m = model("Home", 3);
        m.held = Some(P(7));
        let (room, out) = room_of(&m, true);
        assert_eq!(room.0, 640.0);
        assert!(room.1 > bare.1);
        assert!(!out.hits.iter().any(|h| h.press == P(0)));
        assert_eq!(out.back.map(|b| b.0), Some(P(7)));
    }

    /// Invariant: A notice that stays takes the press of its close over the page, and the page's note stands above it.
    /// Regression: The boot or keyboard notice drawn without a close, or under the note it should carry.
    /// Owner: kit
    #[test]
    fn a_notice_takes_its_close_press() {
        let mut m = model("Home", 0);
        m.notices = vec![
            Notice {
                text: Shown::plain("Keyboard in"),
                close: P(5),
            },
            Notice {
                text: Shown::plain("Wireless keyboard"),
                close: P(6),
            },
        ];
        let (_, out) = room_of(&m, false);
        let y = |p: u8| out.hits.iter().find(|h| h.press == P(p)).map(|h| h.rect.y);
        assert!(y(5).is_some() && y(6).is_some());
        assert!(y(6) < y(5));
    }

    /// Invariant: The wide window's keyboard button takes its press at the top row.
    /// Regression: The button missing over a sheet with a field of its own.
    /// Owner: kit
    #[test]
    fn the_wide_keyboard_button_takes_its_press() {
        let mut m = model("Home", 0);
        m.keyboard_button = Some(KeyboardButton {
            shown: false,
            press: P(8),
            tip: Shown::plain("Keyboard"),
            right: 600.0,
        });
        let (_, out) = room_of(&m, false);
        assert!(out.hits.iter().any(|h| h.press == P(8)));
    }

    /// Invariant: The failed self-test is the whole screen: it names the check and Exit takes its press.
    /// Regression: A failed self-test with no way out, or drawn under a frame.
    /// Owner: kit
    #[test]
    fn a_failed_selftest_offers_exit() {
        let mut c = canvas(160);
        let m = SelftestFailed {
            title: Shown::plain("Self-test failed"),
            check_row: Shown::plain("Check"),
            check: Shown::plain("sha256"),
            exit_label: Shown::plain("Exit"),
            exit: P(3),
        };
        let out = frame(&mut c, &at_rest(None), |ui| {
            draw_selftest_failed(&m, ui, (640.0, 480.0))
        });
        assert!(out.hits.iter().any(|h| h.press == P(3)));
    }
}
