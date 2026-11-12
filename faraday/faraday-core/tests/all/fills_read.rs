//! What is written on a filled box reads against the fill under it, as
//! drawn (`docs/THEMES.md` §11.2, `docs/CODEX32.md` §7.5): every line of
//! text on a codex32 checksum square or on a step's badge, in the colour
//! it was drawn in, against the pixels round it on the frame, 4.5:1 or
//! more (7:1 at High contrast), in every scheme Settings offers: each
//! theme in Light and Dark at Standard, Medium and High contrast.

use faraday_core::{Action, Faraday, Screen};
use faraday_draw::Color;
use faraday_draw::canvas::InkKind;
use faraday_shell_api::App;
use faraday_ui::Theme;
use faraday_ui::pal::Role;

use super::codex32_make::both_typed_in;
use super::common::{over, ratio, settle_n, under};

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
    // The entry done: Share C's string, its checksum filled in, over its
    // worksheet, which keeps the last box in view. Typed once; a scheme
    // chosen in Settings redraws the same screen in it.
    let mut app = both_typed_in(Theme::all().next().unwrap(), 1280, 1600);
    for theme in Theme::all() {
        let preset = theme;
        let need = if theme.is_high_contrast() { 7.0 } else { 4.5 };
        let fill = theme.color(Role::ChecksumContainer).with_alpha(255);
        app.press(Action::Theme(theme));
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
        let badges = on_fill(
            &mut app,
            theme.color(Role::SurfaceContainerHighest).with_alpha(255),
        );
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

/// Every line of text the frame draws that is fully opaque (a disabled
/// control is see-through and not held to the ratio, `docs/THEMES.md`
/// §8): what it reads and how far it stands out from the pixels round
/// it, as drawn.
fn all_text(app: &mut Faraday) -> Vec<(String, f64, Color, Color)> {
    settle_n(app, app.now_ms() / 1000 + 1, 3);
    let ink = app.drawn_ink();
    let f = app.frame();
    let (width, height) = (i32::from(f.width), i32::from(f.height));
    ink.into_iter()
        .filter_map(|i| match i.kind {
            InkKind::Text { text, color, .. } if color.a == 255 && !text.trim().is_empty() => {
                let bg = under(
                    f.rgba,
                    width,
                    height,
                    (i.rect.x, i.rect.y, i.rect.w, i.rect.h),
                );
                Some((text, ratio(color, bg), color, bg))
            }
            _ => None,
        })
        .collect()
}

/// A scene: the app made and brought to a screen in a scheme.
type Scene = fn(Theme) -> Faraday;

/// `app` drawn in `theme` with Reduce motion on, so every frame it draws
/// is at rest: a frame part of the way through a fade blends two
/// screens' pixels under text drawn in one.
fn in_theme(app: &mut Faraday, theme: Theme) {
    app.press(Action::ReduceMotion(true));
    app.press(Action::Theme(theme));
}

/// `app`'s window made tall enough for Settings to fit whole: nothing
/// scrolls, so no text is under the fade at a scrolled edge, where a
/// line passing out of view is meant to fade with what it is on.
fn tall(app: &mut Faraday) {
    use faraday_shell_api::{BootState, DisplayInfo, Event, SecureHardware};
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// Test key `n` typed in as a key.
fn key_in(app: &mut Faraday, n: usize) {
    use faraday_shell_api::{Event, Key};
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

/// A 2-of-3 over three keys typed in, its backup opened on the board.
fn the_board() -> Faraday {
    use faraday_core::create::NewKind;
    use faraday_core::{cstep, qstep};
    let mut app = faraday_core::testkit::started();
    for n in 0..3 {
        key_in(&mut app, n);
    }
    app.press(Action::CreateWallet);
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    crate::common::ckind(&mut app, multi);
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint.0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    app.press(Action::BQ(qstep::BOARD));
    app.settle();
    app
}

/// The screens where text sits on a fill of its own: Home's tiles and
/// lead card, Settings' chips and fields, the backup board's boxes,
/// pills and stick tags, the wallets' status badges and the wallet on
/// screen, a kind of wallet chosen (Material's selected state). Drawn in
/// each scheme Settings offers, every opaque line of text reads against
/// the pixels round it, 4.5:1 (7:1 at High contrast).
#[test]
fn what_the_screens_write_reads_on_their_fills_in_every_theme() {
    let mut short = Vec::new();
    for preset in Theme::all() {
        let need = if preset.is_high_contrast() { 7.0 } else { 4.5 };
        // Each scene made when it is drawn: five apps at once overflow a
        // test thread's stack.
        let scenes: [(&str, Scene); 5] = [
            ("home", |theme| {
                let mut app = faraday_core::testkit::started();
                in_theme(&mut app, theme);
                app
            }),
            ("settings", |theme| {
                let mut app = faraday_core::testkit::started();
                tall(&mut app);
                in_theme(&mut app, theme);
                app.press(Action::Nav(Screen::Settings));
                app
            }),
            ("board", |theme| {
                let mut app = the_board();
                in_theme(&mut app, theme);
                app
            }),
            ("wallets", |theme| {
                let mut app = the_board();
                in_theme(&mut app, theme);
                app.press(Action::Nav(Screen::Wallets));
                app
            }),
            // A kind chosen among the kinds of wallet: a selected row.
            ("kinds", |theme| {
                let mut app = faraday_core::testkit::started();
                in_theme(&mut app, theme);
                app.press(Action::CreateWallet);
                crate::common::ckind(&mut app, 0);
                app.press(Action::CStep(faraday_core::cstep::KIND));
                app
            }),
        ];
        for (scene, make) in scenes {
            let mut app = make(preset);
            let drawn = all_text(&mut app);
            assert!(!drawn.is_empty(), "{preset:?}: {scene} draws text");
            for (text, got, ink, bg) in drawn {
                if got < need {
                    short.push(format!(
                        "{preset:?} {scene}: {text:?} is {got:.2}:1 ({:02x}{:02x}{:02x} on {:02x}{:02x}{:02x})",
                        ink.r, ink.g, ink.b, bg.r, bg.g, bg.b
                    ));
                }
            }
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}
