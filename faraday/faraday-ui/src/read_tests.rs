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
            },
            400.0,
        );
        nav::icon_action(ui, 560.0, Icon::Done, 4, true);
        nav::help(ui, 500.0, 5);
        nav::more(ui, 440.0, 6);
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
    let scenes: [(&str, Scene); 6] = [
        ("controls", controls),
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
                let floor = match (pressed, need > 4.5) {
                    (None, _) => need,
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
