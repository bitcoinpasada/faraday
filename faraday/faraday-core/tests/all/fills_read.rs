//! What is written on a filled box reads against the fill under it, as
//! drawn (`docs/THEMES.md` §11.2, `docs/CODEX32.md` §7.5): every line of
//! text on a codex32 checksum square or on a step's badge, in the colour
//! it was drawn in, against the pixels round it on the frame, 4.5:1 or
//! more (7:1 at High contrast), in every theme Settings offers.

use std::collections::HashMap;

use faraday_core::Faraday;
use faraday_draw::Color;
use faraday_draw::canvas::InkKind;
use faraday_shell_api::App;
use faraday_ui::Preset;
use faraday_ui::pal::{CODEX32_CHECKSUM_FILL, INNER};

use super::codex32_make::both_typed_in;
use super::common::settle_n;

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

/// The colour most of the pixels just outside the box (x, y, w, h) are:
/// the fill a text was drawn on.
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

/// Every line of text the frame draws on `fill`: what it reads and how
/// far it stands out from the fill, as drawn.
fn on_fill(app: &mut Faraday, fill: Color) -> Vec<(String, f64)> {
    // Past any fade or slide: the frame as it rests.
    settle_n(app, app.now_ms() / 1000 + 1, 3);
    let ink = app.drawn_ink();
    let f = app.frame();
    let (width, height) = (i32::from(f.width), i32::from(f.height));
    ink.into_iter()
        .filter_map(|i| match i.kind {
            InkKind::Text { text, color, .. } => {
                let bg = under(
                    f.rgba,
                    width,
                    height,
                    (i.rect.x, i.rect.y, i.rect.w, i.rect.h),
                );
                (bg == fill).then(|| (text, ratio(over(color, bg), bg)))
            }
            _ => None,
        })
        .collect()
}

/// BIP 93's vector 2 typed share by share, in each theme: once the
/// entry is done, every character, box number and letter on a checksum
/// square, in the string's boxes and on the worksheet, reads on it, and
/// so does each later step's number on its badge.
#[test]
fn what_is_written_on_a_checksum_square_or_a_badge_reads_in_every_theme() {
    let mut short = Vec::new();
    for preset in Preset::ALL {
        let theme = preset.theme();
        let need = if theme.is_high_contrast() { 7.0 } else { 4.5 };
        let fill = theme.color(CODEX32_CHECKSUM_FILL).with_alpha(255);
        // The entry done: Share C's string, its checksum filled in, over
        // its worksheet, which keeps the last box in view.
        let mut app = both_typed_in(preset, 1280, 1600);
        let drawn = on_fill(&mut app, fill);
        // The string's checksum characters, and the worksheet's
        // characters and box numbers in its checksum squares.
        let has = |f: fn(&str) -> bool| drawn.iter().any(|(t, _)| f(t));
        assert!(
            has(|t| t.chars().all(|c| c.is_ascii_lowercase())),
            "{preset:?}: the string's checksum: {drawn:?}"
        );
        assert!(
            has(|t| t.chars().all(|c| c.is_ascii_uppercase())),
            "{preset:?}: a worksheet's character: {drawn:?}"
        );
        assert!(
            has(|t| t.parse::<u8>().is_ok_and(|n| n > 35)),
            "{preset:?}: a box number: {drawn:?}"
        );
        // The steps not reached yet: each one's number on its badge.
        let badges = on_fill(&mut app, theme.color(INNER).with_alpha(255));
        assert!(
            badges.iter().any(|(t, _)| t == "8"),
            "{preset:?}: Strings made's number on its badge: {badges:?}"
        );
        for (text, got) in drawn.into_iter().chain(badges) {
            if got < need {
                short.push(format!("{preset:?}: {text:?} is {got:.2}:1"));
            }
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}
