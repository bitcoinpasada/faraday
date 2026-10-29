//! What the components write reads against the fill they were drawn on
//! (`docs/THEMES.md` §11.2): each component is drawn on the canvas in
//! every scheme (ten themes, Light and Dark, Standard, Medium and High
//! contrast), and every line of text is measured in the colour it was
//! drawn in against the pixels round it on the frame, 4.5:1 or more, 7:1
//! at High contrast. The control a press is on is drawn pressed, so a
//! state layer is part of the fill (each scheme presses a different
//! control, every control in some scheme); a pressed one is held to 4:1
//! and, at High contrast, 6.5:1.

#[cfg(test)]
use super::*;
use crate::keyboard::{Keys, OskPress};
use crate::nav::{self, AppBar, Badge, Chip, Dest, Rail};
use faraday_draw::Canvas;
use faraday_draw::canvas::InkKind;
use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::KeyboardKind;
use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};
use std::collections::HashMap;

/// WCAG relative luminance.
fn luminance(c: Color) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio.
fn ratio(a: Color, b: Color) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `ink` laid over the opaque `under`.
fn over(ink: Color, under: Color) -> Color {
    let a = u32::from(ink.a);
    let mix = |f: u8, b: u8| ((u32::from(f) * a + u32::from(b) * (255 - a) + 127) / 255) as u8;
    Color::rgb(
        mix(ink.r, under.r),
        mix(ink.g, under.g),
        mix(ink.b, under.b),
    )
}

/// A canvas of 640 × 480 pixels at 160 dpi.
fn canvas() -> Canvas {
    Canvas::new(&DisplayInfo {
        width: 640,
        height: 480,
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

/// The colour most of the pixels just outside the box are: the fill a
/// text was drawn on.
fn under(rgba: &[u8], width: i32, height: i32, (x, y, w, h): (i32, i32, i32, i32)) -> Color {
    let mut seen: HashMap<(u8, u8, u8), usize> = HashMap::new();
    let mut take = |px: i32, py: i32| {
        if px >= 0 && py >= 0 && px < width && py < height {
            let i = ((py * width + px) * 4) as usize;
            *seen.entry((rgba[i], rgba[i + 1], rgba[i + 2])).or_default() += 1;
        }
    };
    for px in x - 1..=x + w {
        take(px, y - 1);
        take(px, y + h);
    }
    for py in y..y + h {
        take(x - 1, py);
        take(x + w, py);
    }
    let ((r, g, b), _) = seen
        .into_iter()
        .max_by_key(|&(c, n)| (n, c))
        .expect("a box on the frame");
    Color::rgb(r, g, b)
}

/// A scene: drawn on a cleared page in the scheme the `Ui` is set to.
type Scene = fn(&mut Ui<u8>);

/// Every line of text `scene` draws in `theme` with `pressed` pressed:
/// what it reads and how far it stands out from what is under it, as
/// drawn. Text drawn see-through (a control not available now) is left
/// out: WCAG asks it no contrast.
fn read(theme: Theme, scene: Scene, pressed: Option<u8>) -> Vec<(String, f64)> {
    let mut canvas = canvas();
    canvas.record_ink(true);
    let mut hits = Vec::new();
    {
        let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, pressed);
        ui.theme = theme;
        ui.clear(Role::Surface);
        scene(&mut ui);
    }
    let ink = canvas.ink().to_vec();
    let f = canvas.frame();
    let (width, height) = (i32::from(f.width), i32::from(f.height));
    ink.into_iter()
        .filter_map(|i| match i.kind {
            InkKind::Text { text, color, .. } if color.a == 255 => {
                let bg = under(
                    f.rgba,
                    width,
                    height,
                    (i.rect.x, i.rect.y, i.rect.w, i.rect.h),
                );
                Some((text, ratio(over(color, bg), bg)))
            }
            _ => None,
        })
        .collect()
}

/// Buttons of every style, a segmented button, the floating action, a
/// field's chip, pills and badges, cards with their text, a code box, a
/// word pill, and the checkbox list.
fn controls(ui: &mut Ui<u8>) {
    let styles = [
        ("Filled", Style::Primary),
        ("Tonal", Style::Tonal),
        ("Outlined", Style::Secondary),
        ("Text", Style::Ghost),
        ("Chosen", Style::Selected),
    ];
    let mut x = 16.0;
    for (i, (label, style)) in styles.into_iter().enumerate() {
        x += ui.button(x, 16.0, None, 44.0, label, style, 1 + i as u8) + 8.0;
    }
    ui.segmented(
        (16.0, 72.0, 40.0),
        13.0,
        &[("One", 6), ("Two", 7), ("Three", 8)],
        Some(1),
        None,
    );
    ui.fab(620.0, 140.0, Icon::Done, "Floating", 9);
    ui.field_action(16.0, 120.0, 220.0, 44.0, "Paste", 10);
    ui.pill(16.0, 180.0, 26.0, "Pill");
    ui.word_pill(100.0, 180.0, "abandon", 11);
    ui.code_box(220.0, 180.0, 13.0, "bc1q0example");
    ui.badge(16.0, 220.0, "1", false, false);
    ui.badge(56.0, 220.0, "2", false, true);
    ui.badge(96.0, 220.0, "3", true, false);
    ui.checkbox(136.0, 224.0, true, true);
    ui.checkbox(176.0, 224.0, false, true);
    ui.card(16.0, 260.0, 280.0, 90.0, Role::OutlineVariant);
    ui.text(32.0, 270.0, 15.0, W::S, Role::OnSurface, "Card title");
    ui.text(
        32.0,
        296.0,
        13.0,
        W::R,
        Role::OnSurfaceVariant,
        "Supporting text",
    );
    parts::section_label(ui, 320.0, 270.0, "A section");
    let rows = [
        ("First".to_string(), true, true, 12),
        ("Second".to_string(), false, true, 13),
    ];
    ui.multi_list_valued(
        320.0,
        300.0,
        300.0,
        &rows,
        &["a value".into(), "b value".into()],
    );
    ui.progress((16.0, 380.0, 200.0), Some(0.4), None);
}

/// Settings › Appearance (`docs/THEMES.md` §12): the theme chips, Mode
/// and Contrast with the scheme on screen chosen, and a switch row on
/// and off, at rest: their codes are past the controls pressed.
fn appearance(ui: &mut Ui<u8>) {
    let now = ui.theme;
    let code = |t: Theme| 20 + t.row() as u8;
    let h = ui.appearance((16.0, 16.0, 600.0), Some(128.0), now, code);
    ui.switch_row((16.0, 32.0 + h, 300.0), "Reduce motion", false, 14);
    ui.switch_row((320.0, 32.0 + h, 300.0), "Reduce motion", true, 15);
}

/// A rail with a current destination and a count badge, the top app bar
/// with a way back and its actions, a status chip, an expanded rail's
/// row, and the navigation bar.
fn navigation(ui: &mut Ui<u8>) {
    let dest = |label, current, action, badge| Dest {
        icon: Glyph::Icon(Icon::Done),
        label,
        action,
        current,
        badge,
    };
    let top = [
        dest("Home", true, 1, Badge::None),
        dest("Wallets", false, 2, Badge::Count(3)),
    ];
    nav::rail(
        ui,
        400.0,
        &Rail {
            top: &top,
            under: &[],
            foot: &[],
        },
    );
    ui.ox = 80.0;
    {
        nav::app_bar(
            ui,
            560.0,
            &AppBar {
                back: Some(("Wallets", 3)),
                title: "A wallet",
                back_tip: "Back",
            },
            400.0,
        );
        nav::icon_action(ui, 560.0, Icon::Done, 4, true, "Done");
        nav::help(ui, 500.0, 5, "Learn");
        nav::more(ui, 440.0, 6, "More");
        nav::status_chip(
            ui,
            (16.0, 100.0, 300.0),
            &Chip {
                icon: Some(Glyph::Icon(Icon::Done)),
                head: ("Signing", W::S),
                tail: "2 of 3",
                action: Some(7),
            },
        );
        nav::destination_row(
            ui,
            (16.0, 150.0, 240.0),
            &dest("Settings", true, 8, Badge::None),
            Some("end"),
        );
        nav::destination_row(
            ui,
            (16.0, 200.0, 240.0),
            &dest("Learn", false, 9, Badge::Dot),
            None,
        );
        let bar = [
            dest("Home", true, 10, Badge::None),
            dest("Wallets", false, 11, Badge::Count(12)),
            dest("Settings", false, 12, Badge::None),
        ];
        nav::bar(ui, 400.0, 560.0, &bar);
    }
    ui.ox = 0.0;
}

/// A dialog: its head, text under it and its actions; and a sheet.
fn dialog(ui: &mut Ui<u8>) {
    sheet::dialog(
        ui,
        (640.0, 480.0, 400.0),
        (Glyph::Icon(Icon::Done), Role::Secondary.paint(), "A dialog"),
        &mut |ui, x, y, w| ui.wrap(x, y, w, 14.0, W::R, Role::OnSurfaceVariant, "What it says."),
        &[("Cancel", Style::Ghost, 1), ("Continue", Style::Primary, 2)],
    );
}

/// A sheet: its head, a key and its value, and its buttons.
fn bottom_sheet(ui: &mut Ui<u8>) {
    sheet::kv_sheet(
        ui,
        640.0,
        480.0,
        (Icon::Done, Role::Secondary.paint(), "A sheet"),
        "Supporting line",
        &[("Key", "Value".to_string(), Role::OnSurface.paint())],
        &[("Close", Style::Tonal, 1)],
    );
}

/// A text field's label kept outside it, a field at rest and one
/// focused in error with its error line, buttons [`BUTTON_H`] tall, and
/// list items of one, two and three lines, the second chosen (its code,
/// [`CHOSEN_ITEM`], is among the controls pressed).
///
/// [`BUTTON_H`]: faraday_draw::tokens::BUTTON_H
/// The press code of the chosen list item in [`fields_and_lists`].
const CHOSEN_ITEM: u8 = 4;

fn fields_and_lists(ui: &mut Ui<u8>) {
    use crate::roles::ListItem;
    use faraday_draw::tokens::BUTTON_H;
    let top = 16.0 + ui.field_label(16.0, 16.0, "Amount");
    ui.field_label(320.0, 16.0, "Address");
    ui.field_box((16.0, top, 280.0), false, None);
    let why = Some("Not an address on this network");
    let fh = ui.field_box((320.0, top, 280.0), true, why);
    let by = top + fh + 16.0;
    let mut x = 16.0;
    for (label, style, a) in [
        ("Continue", Style::Primary, 1),
        ("Back", Style::Secondary, 2),
    ] {
        x += ui.button(x, by, None, BUTTON_H, label, style, a) + 8.0;
    }
    let items = [
        ("Settings", "", 3),
        ("Spending wallet", "2 of 3 · Native SegWit", CHOSEN_ITEM),
        (
            "Family vault",
            "Three keys at three places, each a different kind, so that no one loss or theft opens it",
            5,
        ),
    ];
    let mut ly = by + 56.0;
    for (k, (headline, supporting, action)) in items.into_iter().enumerate() {
        let it = ListItem {
            headline,
            supporting,
            trailing: Some(("Now", Role::OnSurfaceVariant.paint())),
            action,
        };
        ly += ui.list_item((16.0, ly, 300.0), &it, k == 1);
    }
}

/// The shared pieces: a hold of each tone at rest and held to its end, a status
/// tag of each kind, every stick's pill quiet, asked for and in, a guide
/// step to do and one ticked, a field with part of what it holds
/// selected, and a snackbar at the foot.
fn pieces(ui: &mut Ui<u8>) {
    use crate::hold::HoldTone;
    use crate::stick::{PillAt, StickState};
    let tones = [HoldTone::Error, HoldTone::Warning, HoldTone::Primary];
    for (k, tone) in tones.into_iter().enumerate() {
        let x = 16.0 + k as f32 * 200.0;
        ui.hold_button((x, 16.0, 190.0), ("Hold to erase", tone), 0.0, 1);
        ui.hold_button((x, 340.0, 190.0), ("Hold to erase", tone), 1.0, 2);
    }
    let mut x = 16.0;
    for (label, status) in [
        ("Done", Status::Ok),
        ("Waiting", Status::Warning),
        ("Refused", Status::Error),
        ("Idle", Status::Neutral),
    ] {
        x += ui.tag(x, 76.0, label, status) + 8.0;
    }
    let states = [StickState::Quiet, StickState::Asked, StickState::Inserted];
    for (row, state) in states.into_iter().enumerate() {
        for n in 0..8 {
            let at = PillAt {
                x: 16.0 + n as f32 * 76.0,
                y: 110.0 + row as f32 * 40.0,
                w: 70.0,
                h: 30.0,
                pad: 8.0,
                dot_r: 4.0,
                dot_room: 14.0,
                text: 13.0,
            };
            let label = if n == 0 { "Boot" } else { "Stick" };
            let action = if row == 2 {
                3 + n as u8
            } else {
                11 + row as u8
            };
            ui.stick_pill(at, n, state, label, Some(action));
        }
    }
    parts::step_circle(ui, 16.0, 240.0, 1, false, Some(13));
    parts::step_circle(ui, 60.0, 240.0, 2, true, Some(13));
    ui.edit = Edit {
        field: Some(14),
        caret: Some(3),
        anchor: Some(9),
    };
    let fh = ui.field_box((320.0, 236.0, 280.0), true, None);
    let at = (336.0, 236.0, fh);
    ui.typed(
        at.0,
        at.1,
        at.2,
        16.0,
        W::R,
        Role::OnSurface,
        "bc1qexample",
        240.0,
        true,
        14,
    );
    ui.snackbar((0.0, 640.0, 480.0), "Time set to 14:30", 1.0, 0.0);
}

/// Material's menu as a popup, under group headings, one item chosen and
/// the arrow keys on another.
fn menu_popup(ui: &mut Ui<u8>) {
    use crate::menu::{Menu, MenuLine, menu};
    let lines = [
        MenuLine::Head("Place 1"),
        MenuLine::Item("Move here", 1, true),
        MenuLine::Item("Copy here", 2, false),
        MenuLine::Head("Place 2"),
        MenuLine::Item("Move here", 3, false),
    ];
    let m = Menu {
        title: Some("Stick 1"),
        icon: Glyph::Icon(Icon::Done),
        lines: &lines,
        at: Some(2),
        anchor: None,
        cancel: ("Cancel", 4),
        shown: 1.0,
    };
    menu(ui, 640.0, 480.0, &m);
}

/// The on-screen keyboard, Shift on.
fn keyboard(ui: &mut Ui<u8>) {
    keyboard::keyboard(
        ui,
        0.0,
        640.0,
        200.0,
        KeyboardKind::Passphrase,
        Keys {
            shift: true,
            symbols: false,
            ..Keys::default()
        },
        true,
        |p: OskPress| match p {
            OskPress::Char(_) => 1,
            _ => 2,
        },
    );
}

#[test]
fn what_the_components_write_reads_on_what_they_are_drawn_on_in_every_scheme() {
    let scenes: [(&str, Scene); 9] = [
        ("controls", controls),
        ("pieces", pieces),
        ("menu", menu_popup),
        ("fields and lists", fields_and_lists),
        ("appearance", appearance),
        ("navigation", navigation),
        ("dialog", dialog),
        ("sheet", bottom_sheet),
        ("keyboard", keyboard),
    ];
    let mut short = Vec::new();
    for (n, theme) in Theme::all().enumerate() {
        let need = if theme.is_high_contrast() { 7.0 } else { 4.5 };
        for (name, scene) in scenes {
            // At rest, and with one control pressed: the control for this
            // scheme steps through all of them as the schemes go by.
            for pressed in [None, Some(1 + (n % 13) as u8)] {
                let lines = read(theme, scene, pressed);
                assert!(lines.len() >= 2, "{name} draws text");
                // A pressed control has its content colour laid over its
                // fill at 0.10 (`docs/THEMES.md` §8), which costs a
                // tonal fill up to a tenth of its contrast for as long
                // as it is pressed: 4:1 and, at High contrast, 6.5:1.
                // A chosen control held down keeps about 3.8:1 in Dark at
                // Standard contrast, its tonal fill under the layer
                // (owner, 2026-10-24: the pressed-state contrast call).
                let chosen_held = name == "fields and lists"
                    && pressed == Some(CHOSEN_ITEM)
                    && theme.mode == crate::Mode::Dark;
                let floor = match (pressed, need > 4.5) {
                    (None, _) => need,
                    (Some(_), false) if chosen_held => 3.8,
                    (Some(_), false) => 4.0,
                    (Some(_), true) => 6.5,
                };
                for (text, got) in lines {
                    if got < floor {
                        short.push(format!(
                            "{theme:?} {name} (pressed {pressed:?}): {text:?} is {got:.2}:1"
                        ));
                    }
                }
            }
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}

/// A stick's dot and edge, its tone, stand out from every surface at
/// 3:1 or more in every scheme, so which stick a pill is reads from its
/// tone wherever the pill is drawn.
#[test]
fn every_sticks_tone_stands_out_from_every_surface_in_every_scheme() {
    let surfaces = [
        Role::Surface,
        Role::SurfaceDim,
        Role::SurfaceBright,
        Role::SurfaceContainerLowest,
        Role::SurfaceContainerLow,
        Role::SurfaceContainer,
        Role::SurfaceContainerHigh,
        Role::SurfaceContainerHighest,
    ];
    let mut short = Vec::new();
    for theme in Theme::all() {
        for n in 0..8 {
            for s in surfaces {
                let got = ratio(theme.color(pal::stick(n)), theme.role(s));
                if got < 3.0 {
                    short.push(format!("{theme:?}: stick {n} on {s:?} is {got:.2}:1"));
                }
            }
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}

/// A dialog whose body is taller than the window, scrolled: under its
/// top edge the content fades into the dialog's own container, not into
/// the page's colour, in every scheme that fades.
#[test]
fn a_scrolled_dialog_fades_into_its_own_container() {
    use faraday_draw::tokens::PAD;
    let mut short = Vec::new();
    for theme in Theme::all().filter(|t| !t.is_high_contrast()) {
        let mut canvas = canvas();
        let mut hits = Vec::new();
        {
            let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
            ui.theme = theme;
            ui.clear(Role::Surface);
            ui.offset = 60.0;
            sheet::dialog(
                &mut ui,
                (640.0, 480.0, 400.0),
                (Glyph::Icon(Icon::Done), Role::Secondary.paint(), "A dialog"),
                &mut |_, _, _, _| 2000.0,
                &[("Close", Style::Ghost, 1)],
            );
        }
        let got = pixel(&canvas, 320.0, PAD + 2.0);
        let want = theme.role(Role::SurfaceContainerHigh);
        let near = |a: u8, b: u8| a.abs_diff(b) <= 2;
        if !(near(got.r, want.r) && near(got.g, want.g) && near(got.b, want.b)) {
            short.push(format!(
                "{theme:?}: under the top edge is {got:?}, not {want:?}"
            ));
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}

/// The top app bar is the page's colour while the page is at its top
/// and surface-container once the page is scrolled under it.
#[test]
fn the_top_app_bar_turns_surface_container_once_the_page_scrolls_under_it() {
    for (offset, want) in [(0.0, Role::Surface), (30.0, Role::SurfaceContainer)] {
        let mut canvas = canvas();
        let mut hits = Vec::new();
        {
            let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
            ui.clear(Role::Surface);
            ui.offset = offset;
            let bar = AppBar {
                back: None,
                title: "Home",
                back_tip: "Back",
            };
            nav::app_bar(&mut ui, 640.0, &bar, 640.0);
        }
        assert_eq!(pixel(&canvas, 600.0, 4.0), Theme::default().role(want));
    }
}

/// A finger on a region shows it is pressed; a finger on a held secret
/// leaves what it shows as drawn.
#[test]
fn a_pressed_region_shows_it_and_a_held_secret_reads_as_drawn() {
    let page = Theme::default().role(Role::Surface);
    for (held, changed) in [(false, true), (true, false)] {
        let mut canvas = canvas();
        let mut hits = Vec::new();
        {
            let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, Some(1));
            ui.clear(Role::Surface);
            if held {
                ui.hit_held(100.0, 100.0, 120.0, 40.0, 1);
            } else {
                ui.hit(100.0, 100.0, 120.0, 40.0, 1);
            }
        }
        assert_eq!(pixel(&canvas, 160.0, 120.0) != page, changed, "held {held}");
    }
}

/// A menu's chosen item is filled in secondary-container; the others
/// stand on the menu's own surface-container.
#[test]
fn a_menus_chosen_item_is_filled_in_the_selected_colour() {
    let mut canvas = canvas();
    canvas.record_ink(true);
    let mut hits = Vec::new();
    {
        let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
        ui.clear(Role::Surface);
        use crate::menu::{Menu, MenuLine, menu};
        let lines = [
            MenuLine::Item("Move here", 1, true),
            MenuLine::Item("Copy here", 2, false),
        ];
        let m = Menu {
            title: None,
            icon: Glyph::Icon(Icon::Done),
            lines: &lines,
            at: None,
            anchor: None,
            cancel: ("Cancel", 4),
            shown: 1.0,
        };
        menu(&mut ui, 640.0, 480.0, &m);
    }
    let at = |label: &str| {
        canvas
            .ink()
            .iter()
            .find_map(|i| match &i.kind {
                InkKind::Text { text, .. } if text == label => Some(i.rect),
                _ => None,
            })
            .expect("drawn")
    };
    let (chosen, other) = (at("Move here"), at("Copy here"));
    let theme = Theme::default();
    let beside = |r: faraday_draw::Rect| pixel(&canvas, (r.x - 4) as f32, (r.y + r.h / 2) as f32);
    assert_eq!(beside(chosen), theme.role(Role::SecondaryContainer));
    assert_eq!(beside(other), theme.role(Role::SurfaceContainer));
}

/// A notice that stays at the foot and a toast over it: the toast stands
/// above the notice, neither over the other.
#[test]
fn a_toast_over_a_notice_stands_clear_of_it() {
    let mut canvas = canvas();
    canvas.record_ink(true);
    let mut hits = Vec::new();
    {
        let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
        ui.clear(Role::Surface);
        let up = ui.snackbar((0.0, 640.0, 480.0), "Remove the stick", 1.0, 0.0);
        ui.snackbar((0.0, 640.0, 480.0 - up), "Time set to 14:30", 1.0, 0.0);
    }
    let rect = |label: &str| {
        canvas
            .ink()
            .iter()
            .find_map(|i| match &i.kind {
                InkKind::Text { text, .. } if text == label => Some(i.rect),
                _ => None,
            })
            .expect("drawn")
    };
    let (notice, toast) = (rect("Remove the stick"), rect("Time set to 14:30"));
    assert!(toast.bottom() < notice.y, "{toast:?} over {notice:?}");
    assert!(notice.bottom() <= 480);
}

/// A toast that comes up while Home's Scan floats at the foot of a small
/// panel leaves Scan in sight and pressable: the toast stands above it.
#[test]
fn a_toast_leaves_the_floating_button_in_sight() {
    let mut canvas = canvas();
    canvas.record_ink(true);
    let mut hits = Vec::new();
    {
        let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
        ui.clear(Role::Surface);
        ui.fab(344.0, 464.0, Icon::Scan, "Scan", 1);
        ui.snackbar((0.0, 360.0, 480.0), "USB Receiver is in use", 1.0, 0.0);
    }
    let rect = |label: &str| {
        canvas
            .ink()
            .iter()
            .find_map(|i| match &i.kind {
                InkKind::Text { text, .. } if text == label => Some(i.rect),
                _ => None,
            })
            .expect("drawn")
    };
    let (scan, toast) = (rect("Scan"), rect("USB Receiver is in use"));
    assert!(toast.bottom() < scan.y, "{toast:?} over {scan:?}");
    let press = hits
        .iter()
        .find(|(_, a)| *a == 1)
        .expect("Scan takes a press");
    assert!(press.0.y > toast.bottom(), "{:?} under the toast", press.0);
}

/// The tooltips' own pairs: plain on inverse-surface, rich on its
/// container, and its edge against the page.
#[test]
fn the_tooltips_read_in_every_scheme() {
    use crate::tooltip::{
        TIP_PLAIN_FILL, TIP_PLAIN_INK, TIP_RICH_EDGE, TIP_RICH_FILL, TIP_RICH_INK,
    };
    let mut short = Vec::new();
    for theme in Theme::all() {
        let need = if theme.is_high_contrast() { 7.0 } else { 4.5 };
        let c = |p: Paint| theme.color(p);
        for (what, fg, bg, min) in [
            ("plain", TIP_PLAIN_INK, TIP_PLAIN_FILL, need),
            ("rich", TIP_RICH_INK, TIP_RICH_FILL, need),
            ("rich edge", TIP_RICH_EDGE, Role::Surface.paint(), 3.0),
        ] {
            let got = ratio(c(fg), c(bg));
            if got < min {
                short.push(format!("{theme:?}: {what} tooltip is {got:.2}:1"));
            }
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}

/// Settings' Appearance card is sized from its rows' widths before they
/// are drawn: the height worked out is the height drawn, wide and on a
/// small panel, at every width the rows wrap at.
#[test]
fn the_appearance_rows_are_as_tall_as_worked_out() {
    let mut canvas = canvas();
    let mut hits = Vec::new();
    let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
    for w in [180.0, 240.0, 320.0, 412.0, 520.0, 600.0] {
        for label in [None, Some(128.0)] {
            let drawn = ui.appearance((0.0, 0.0, w), label, Theme::default(), |_| 1);
            assert_eq!(ui.appearance_h(w, label), drawn, "{w} {label:?}");
        }
    }
}

/// The pixel at (x, y) of `canvas`, as a colour.
fn pixel(canvas: &Canvas, x: f32, y: f32) -> Color {
    let f = canvas.frame();
    let i = ((y as usize) * usize::from(f.width) + x as usize) * 4;
    Color::rgb(f.rgba[i], f.rgba[i + 1], f.rgba[i + 2])
}

/// Settings' theme chips (`docs/THEMES.md` §12): every chip is filled in
/// its own theme's primary-container in the mode and contrast on screen;
/// the chosen one alone is ringed in its own primary, with the page
/// between ring and fill, and a chip not chosen has no edge.
#[test]
fn the_chosen_theme_chip_alone_is_ringed_in_its_own_primary() {
    use faraday_draw::tokens::{FILTER_CHIP_H, THEME_CHIP_RING, THEME_CHIP_RING_GAP};
    let mut short = Vec::new();
    for theme in Theme::all() {
        let mut canvas = canvas();
        let mut hits = Vec::new();
        let mut at = Vec::new();
        {
            let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
            ui.theme = theme;
            ui.clear(Role::Surface);
            for (k, name) in ThemeName::ALL.into_iter().enumerate() {
                let (x, y) = (24.0, 16.0 + k as f32 * (FILTER_CHIP_H + 12.0));
                let w = ui.theme_chip(x, y, name, name == theme.name, k as u8);
                at.push((name, x, y + FILTER_CHIP_H / 2.0, w));
            }
        }
        let page = theme.role(Role::Surface);
        for (name, x, mid, w) in at {
            let own = Theme::new(name, theme.mode, theme.contrast);
            let fill = pixel(&canvas, x + w - 4.0, mid);
            if fill != own.role(Role::PrimaryContainer) {
                short.push(format!("{theme:?}: {name:?}'s chip is filled {fill:?}"));
            }
            let ring = pixel(
                &canvas,
                x - THEME_CHIP_RING_GAP - THEME_CHIP_RING / 2.0,
                mid,
            );
            let want = if name == theme.name {
                own.role(Role::Primary)
            } else {
                page
            };
            if ring != want {
                short.push(format!("{theme:?}: beside {name:?}'s chip is {ring:?}"));
            }
            if pixel(&canvas, x - 1.0, mid) != page {
                short.push(format!(
                    "{theme:?}: no page between {name:?}'s ring and fill"
                ));
            }
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}

/// The chosen theme chip and one not chosen, pressed: each label keeps
/// to the pressed floor of the components' test on its own fill with
/// its own state layer over it, in every scheme.
#[test]
fn a_pressed_theme_chip_keeps_its_label_readable_in_every_scheme() {
    let mut short = Vec::new();
    for theme in Theme::all() {
        let floor = if theme.is_high_contrast() { 6.5 } else { 4.0 };
        let other = ThemeName::ALL
            .into_iter()
            .find(|&n| n != theme.name)
            .expect("ten themes");
        let other = Theme::new(other, theme.mode, theme.contrast);
        for pressed in [theme, other] {
            let lines = read(theme, appearance, Some(20 + pressed.row() as u8));
            let name = pressed.name.name();
            let Some(&(_, got)) = lines.iter().find(|(t, _)| t == name) else {
                short.push(format!("{theme:?}: {name} not drawn"));
                continue;
            };
            if got < floor {
                short.push(format!("{theme:?}: {name} pressed is {got:.2}:1"));
            }
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}

/// A toast too long for a narrow panel's width breaks onto more lines
/// inside its container rather than running off the screen, every word
/// of it drawn, in inverse-on-surface.
#[test]
fn a_long_toast_wraps_inside_a_narrow_panel() {
    let text = "Sample wallet and its keys wait for the stick to be put back in";
    let mut canvas = canvas();
    canvas.record_ink(true);
    let mut hits = Vec::new();
    {
        let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
        ui.clear(Role::Surface);
        ui.snackbar((0.0, 268.0, 480.0), text, 1.0, 0.0);
    }
    let lines: Vec<_> = canvas
        .ink()
        .iter()
        .filter_map(|i| match &i.kind {
            InkKind::Text { text, color, .. } => Some((text.clone(), *color, i.rect)),
            _ => None,
        })
        .collect();
    assert!(lines.len() > 1, "{lines:?}");
    let ink = Theme::default().role(Role::InverseOnSurface);
    for (_, color, rect) in &lines {
        assert_eq!(*color, ink);
        assert!(rect.x >= 0 && rect.right() <= 268, "{rect:?} off the panel");
        assert!(rect.bottom() <= 480, "{rect:?} below the foot");
    }
    let words: Vec<String> = lines.iter().map(|(t, _, _)| t.clone()).collect();
    assert_eq!(words.join(" "), text);
}

/// Every line of text `scene` draws, with where it was drawn, in the
/// default theme.
fn texts(scene: Scene) -> Vec<(String, faraday_draw::Rect)> {
    let mut canvas = canvas();
    canvas.record_ink(true);
    let mut hits = Vec::new();
    {
        let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
        ui.clear(Role::Surface);
        scene(&mut ui);
    }
    canvas
        .ink()
        .iter()
        .filter_map(|i| match &i.kind {
            InkKind::Text { text, .. } => Some((text.clone(), i.rect)),
            _ => None,
        })
        .collect()
}

/// A field in error says what is wrong in a line under the field, in
/// the error colour, and the field takes that line's room; a field at
/// rest shows no such line. Its label stands above it, outside.
#[test]
fn a_field_in_error_says_why_under_it() {
    use faraday_draw::tokens::FIELD_H;
    let why = "Not an address on this network";
    let mut canvas = canvas();
    canvas.record_ink(true);
    let mut hits = Vec::new();
    let (rest, wrong, top) = {
        let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
        ui.clear(Role::Surface);
        let top = 16.0 + ui.field_label(16.0, 16.0, "Address");
        let rest = ui.field_box((16.0, top, 280.0), false, None);
        let wrong = ui.field_box((320.0, top, 280.0), false, Some(why));
        (rest, wrong, top)
    };
    assert!(wrong > rest, "{wrong} {rest}");
    let error = Theme::default().role(Role::Error);
    let mut said = false;
    for i in canvas.ink() {
        if let InkKind::Text { text, color, .. } = &i.kind {
            if text == "Address" {
                assert!(
                    i.rect.bottom() as f32 <= top,
                    "the label is inside the field"
                );
            }
            if text == why {
                said = true;
                assert_eq!(*color, error);
                assert!(i.rect.x >= 320, "the error line is not under its field");
                assert!(
                    i.rect.y as f32 >= top + FIELD_H,
                    "{:?} not under the field",
                    i.rect
                );
                assert!(
                    i.rect.bottom() as f32 <= top + wrong,
                    "{:?} past its room",
                    i.rect
                );
            }
        }
    }
    assert!(said, "the error is not shown");
}

/// A list item's supporting text too long for one line takes a second
/// inside its item, cut with an ellipsis rather than running past its
/// end, under its headline; a short one stays on one line.
#[test]
fn a_list_items_long_supporting_text_keeps_inside_its_item() {
    let lines = texts(fields_and_lists);
    let find = |s: &str| {
        lines
            .iter()
            .filter(|(t, _)| t.starts_with(s))
            .map(|(_, r)| *r)
            .collect::<Vec<_>>()
    };
    assert_eq!(find("2 of 3").len(), 1, "{lines:?}");
    let first = find("Three keys");
    assert_eq!(first.len(), 1, "{lines:?}");
    let head = find("Family vault");
    assert_eq!(head.len(), 1, "{lines:?}");
    assert!(head[0].y < first[0].y, "{head:?} not over {first:?}");
    let second: Vec<_> = lines
        .iter()
        .filter(|(t, r)| r.y > first[0].y && r.x == first[0].x && t.ends_with('…'))
        .collect();
    assert_eq!(second.len(), 1, "{lines:?}");
    for r in [first[0], second[0].1] {
        assert!(r.x >= 16 && r.right() <= 316, "{r:?} past the item");
    }
}

/// At Larger a list item's headline too long for one line is shown
/// whole on two, inside its item, and the item grows to hold them over
/// its supporting line; at Standard the same headline is one line.
#[test]
fn at_larger_a_list_items_long_headline_takes_two_lines_inside_its_item() {
    use crate::TextSize;
    use crate::roles::ListItem;
    let head = "Savings for the family";
    let drawn = |size: TextSize| {
        let mut canvas = canvas();
        canvas.record_ink(true);
        let mut hits = Vec::new();
        let h = {
            let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
            ui.set_text_size(size);
            ui.clear(Role::Surface);
            let it = ListItem {
                headline: head,
                supporting: "2 of 3",
                trailing: None,
                action: 1,
            };
            ui.list_item((16.0, 16.0, 240.0), &it, false)
        };
        let lines: Vec<(String, faraday_draw::Rect)> = canvas
            .ink()
            .iter()
            .filter_map(|i| match &i.kind {
                InkKind::Text { text, .. } => Some((text.clone(), i.rect)),
                _ => None,
            })
            .collect();
        (h, lines)
    };
    let (_, standard) = drawn(TextSize::Standard);
    assert!(standard.iter().any(|(t, _)| t == head), "{standard:?}");
    let (h, larger) = drawn(TextSize::Larger);
    let heads: Vec<_> = larger.iter().filter(|(t, _)| t != "2 of 3").collect();
    assert_eq!(heads.len(), 2, "{larger:?}");
    let words: Vec<&str> = heads.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(words.join(" "), head);
    let sup = larger.iter().find(|(t, _)| t == "2 of 3").map(|(_, r)| *r);
    let sup = sup.expect("the supporting line is shown");
    for (_, r) in &heads {
        assert!(r.x >= 16 && r.right() <= 256, "{r:?} past the item");
        assert!(r.bottom() <= sup.y + 1, "{r:?} over the supporting line");
    }
    assert!(
        sup.bottom() as f32 <= 16.0 + h,
        "{sup:?} past the item's foot"
    );
}
