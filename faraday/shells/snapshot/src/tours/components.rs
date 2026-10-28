//! The components tour: faraday-ui's shared pieces drawn on their own,
//! each in the states a screen shows it in, on a 720 × 900 canvas at
//! 160 dpi, in the scheme `FARADAY_SNAP_THEME` names (the first start's
//! without it). For looking at a component before the screens use it.

use crate::tour::Tour;
use faraday_draw::Canvas;
use faraday_draw::widgets::Icon;
use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};
use faraday_ui::hold::HoldTone;
use faraday_ui::menu::{Menu, MenuLine, menu};
use faraday_ui::nav::{self, AppBar, Badge, Dest, Rail};
use faraday_ui::stick::{PillAt, StickState};
use faraday_ui::{Edit, Glyph, Role, Status, Theme, Ui, W};

/// The canvas the pieces are drawn on.
fn canvas() -> Canvas {
    Canvas::new(&DisplayInfo {
        width: 720,
        height: 900,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

/// The scheme the tour is taken in.
fn theme() -> Theme {
    std::env::var("FARADAY_SNAP_THEME")
        .ok()
        .and_then(|w| Theme::from_words(&w))
        .unwrap_or_default()
}

/// Holds at rest, half held and held to the end, a status tag of each
/// kind, every stick's pill in each state, guide steps, selected text,
/// the top app bar at rest and scrolled under, a rail, a segmented
/// button, switches, and a notice with a toast standing above it.
fn pieces(ui: &mut Ui<u8>) {
    ui.clear(Role::Surface);
    bar(ui, 0.0, 0.0, ("At rest", 40));
    bar(ui, 360.0, 30.0, ("Scrolled under", 41));
    let tones = [HoldTone::Error, HoldTone::Warning, HoldTone::Primary];
    for (row, held) in [0.0, 0.5, 1.0].into_iter().enumerate() {
        for (k, tone) in tones.into_iter().enumerate() {
            let at = (16.0 + k as f32 * 230.0, 540.0 + row as f32 * 56.0, 220.0);
            let action = if row == 1 && k == 0 { 1 } else { 2 + row as u8 };
            ui.hold_button(at, ("Hold to erase", tone), held, action);
        }
    }
    let mut x = 16.0;
    for (label, status) in [
        ("Done", Status::Ok),
        ("Waiting", Status::Warning),
        ("Refused", Status::Error),
        ("Idle", Status::Neutral),
    ] {
        x += ui.tag(x, 140.0, label, status) + 8.0;
    }
    let states = [StickState::Quiet, StickState::Asked, StickState::Inserted];
    for (row, state) in states.into_iter().enumerate() {
        for n in 0..8 {
            let at = PillAt {
                x: 16.0 + n as f32 * 86.0,
                y: 180.0 + row as f32 * 40.0,
                w: 80.0,
                h: 30.0,
                pad: 8.0,
                dot_r: 4.0,
                dot_room: 14.0,
                text: 13.0,
            };
            let label = if n == 0 {
                "Boot".to_string()
            } else {
                format!("Stick {n}")
            };
            let action = if row == 2 && n == 3 { 4 } else { 50 };
            ui.stick_pill(at, n, state, &label, Some(action));
        }
    }
    faraday_ui::parts::step_circle(ui, 16.0, 310.0, 1, false, Some(51));
    faraday_ui::parts::step_circle(ui, 64.0, 310.0, 2, true, Some(52));
    ui.edit = Edit {
        field: Some(53),
        caret: Some(3),
        anchor: Some(9),
    };
    let fh = ui.field_box((140.0, 304.0, 300.0), true, None);
    let text = "bc1qexample";
    ui.typed(
        156.0,
        304.0,
        fh,
        16.0,
        W::R,
        Role::OnSurface,
        text,
        260.0,
        true,
        53,
    );
    rail(ui);
    segments(ui);
    ui.switch(120.0, 470.0, false, 10);
    ui.switch(200.0, 470.0, true, 11);
    let notice = "Remove the stick to keep working";
    let up = ui.snackbar((0.0, 720.0, 900.0), notice, 1.0, 0.0);
    ui.snackbar((0.0, 720.0, 900.0 - up), "Time set to 14:30", 1.0, 0.0);
}

/// A top app bar 360 wide at `x`, the page scrolled `offset` under it,
/// its title and its way back.
fn bar(ui: &mut Ui<u8>, x: f32, offset: f32, (title, back): (&str, u8)) {
    ui.ox = x;
    ui.offset = offset;
    let bar = AppBar {
        back: Some(("Wallets", back)),
        title,
        back_tip: "Back",
    };
    nav::app_bar(ui, 360.0, &bar, 360.0);
    ui.ox = 0.0;
    ui.offset = 0.0;
}

/// A rail of two destinations, the first current.
fn rail(ui: &mut Ui<u8>) {
    let dest = |label, current, action| Dest {
        icon: Glyph::Icon(Icon::Done),
        label,
        action,
        current,
        badge: Badge::None,
    };
    let top = [dest("Home", true, 5), dest("Wallets", false, 6)];
    (ui.ox, ui.oy) = (640.0, 380.0);
    let rail = Rail {
        top: &top,
        under: &[],
        foot: &[],
    };
    nav::rail(ui, 260.0, &rail);
    (ui.ox, ui.oy) = (0.0, 0.0);
}

/// A segmented button of three, the first chosen.
fn segments(ui: &mut Ui<u8>) {
    let options = [("One", 7), ("Two", 8), ("Three", 9)];
    ui.segmented((120.0, 400.0, 40.0), 14.0, &options, Some(0), None);
}

/// The pieces, then again with the keyboard on a rail destination, the
/// middle segment, a switch and the way back of a scrolled bar: each
/// ring in its control's shape.
fn focused(ui: &mut Ui<u8>) {
    pieces(ui);
    ui.focused = Some(6);
    rail(ui);
    ui.focused = Some(8);
    segments(ui);
    ui.focused = Some(10);
    ui.switch(120.0, 470.0, false, 10);
    ui.focused = Some(41);
    bar(ui, 360.0, 30.0, ("Scrolled under", 41));
    ui.focused = None;
}

/// Material's menu over the page under what opened it, a heading over
/// each group, one item chosen and the arrows on another.
fn menu_popup(ui: &mut Ui<u8>) {
    ui.clear(Role::Surface);
    let lines = [
        MenuLine::Head("Place 1"),
        MenuLine::Item("Paper", 1, true),
        MenuLine::Item("Stick 1 · files", 2, false),
        MenuLine::Head("Place 2"),
        MenuLine::Item("Stick 2 · Vault 1", 3, false),
        MenuLine::Item("New stick", 4, false),
    ];
    let m = Menu {
        title: Some("Move to…"),
        icon: Glyph::Icon(Icon::Done),
        lines: &lines,
        at: Some(2),
        anchor: Some((400, 40, 40, 40)),
        cancel: ("Cancel", 9),
        shown: 1.0,
    };
    menu(ui, 720.0, 900.0, &m);
}

/// A picture: its name, what draws it, and the control a finger is on.
type Shot = (&'static str, fn(&mut Ui<u8>), Option<u8>);

/// The pieces at rest, with the keyboard on some, with a finger on a
/// stick's pill that is in, and the menu.
pub(crate) fn components_tour(t: &mut Tour) -> Result<(), String> {
    let theme = theme();
    let shots: [Shot; 4] = [
        ("components-1-rest", pieces, None),
        ("components-2-focused", focused, None),
        ("components-3-pressed", pieces, Some(4)),
        ("components-4-menu", menu_popup, None),
    ];
    for (name, draw, press) in shots {
        let mut c = canvas();
        let mut hits = Vec::new();
        {
            let mut ui: Ui<u8> = Ui::new(&mut c, 1.0, &mut hits, press);
            ui.theme = theme;
            draw(&mut ui);
        }
        t.save_canvas(name, &c)?;
    }
    Ok(())
}
