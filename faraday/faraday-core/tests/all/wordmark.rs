//! The name in a header: FARADAY with the word of the app it heads after
//! it, DEMO, STARTER or OFFLINE (`docs/DECISIONS.md` F16, F165).

use faraday_core::ui::pal::Role;
use faraday_core::ui::{NameSuffix, Ui};
use faraday_draw::canvas::Canvas;
use faraday_draw::geom::Rect;
use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

/// The width of the name drawn with `suffix`, and how many pixels of the
/// canvas to its right changed from the background.
fn drawn(suffix: NameSuffix) -> (f32, usize) {
    let mut canvas = Canvas::new(&DisplayInfo {
        width: 900,
        height: 80,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    });
    let mut hits = Vec::new();
    let width = {
        let mut ui = Ui::new(&mut canvas, 1.0, &mut hits, None);
        ui.clear(faraday_draw::Color::rgb(255, 255, 255));
        ui.wordmark_named(10.0, 20.0, 40.0, Role::OnSurface, suffix)
    };
    let frame = canvas.frame();
    let alone = {
        let mut c2 = Canvas::new(&DisplayInfo {
            width: 900,
            height: 80,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        });
        let mut h2 = Vec::new();
        let mut ui = Ui::new(&mut c2, 1.0, &mut h2, None);
        ui.clear(faraday_draw::Color::rgb(255, 255, 255));
        let w = ui.wordmark_named(10.0, 20.0, 40.0, Role::OnSurface, NameSuffix::None);
        (w, 10 + w as usize)
    };
    let from = alone.1 + 2;
    let mut inked = 0;
    for y in 0..frame.height as usize {
        for x in from..frame.width as usize {
            let i = (y * frame.width as usize + x) * 4;
            if frame.rgba[i] < 250 {
                inked += 1;
            }
        }
    }
    (width, inked)
}

#[test]
fn every_suffix_adds_its_word_after_the_name_and_none_adds_nothing() {
    let (none_w, none_ink) = drawn(NameSuffix::None);
    assert_eq!(none_ink, 0);
    for suffix in [NameSuffix::Starter, NameSuffix::Demo, NameSuffix::Offline] {
        let (w, ink) = drawn(suffix);
        assert!(w > none_w * 1.5, "{suffix:?}: {w} against {none_w}");
        assert!(ink > 500, "{suffix:?} drew {ink} pixels");
    }
}

/// Faraday Demo, or the device for `demo` false, on Testnet (in Practice
/// where `practice`), showing Home in a window `w` by `h` pixels at `dpi`.
fn home(demo: bool, practice: bool, size: (u16, u16, u16)) -> faraday_core::Faraday {
    home_on(demo, practice, faraday_core::testkit::NET, size)
}

/// As [`home`], on `net`.
fn home_on(
    demo: bool,
    practice: bool,
    net: faraday_bip::keys::Network,
    (w, h, dpi): (u16, u16, u16),
) -> faraday_core::Faraday {
    use faraday_core::{Action, DESKTOP_MODES, Screen, testkit};
    use faraday_shell_api::{App, Event};
    let mut app = faraday_core::Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: w,
        height: h,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.session = faraday_core::wallet::Session::on(net);
    if demo {
        app.online = true;
        app.clipboard = true;
        app.set_modes(DESKTOP_MODES);
        if practice {
            testkit::into_demo(&mut app);
        }
    }
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

/// The boxes of the name's ink in the frame `app` draws, and of every
/// other thing drawn in the band of the top bar it stands in.
fn name_and_bar(app: &mut faraday_core::Faraday) -> (Vec<Rect>, Vec<Rect>) {
    use faraday_draw::canvas::InkKind;
    let ink = app.drawn_ink();
    let name: Vec<Rect> = ink
        .iter()
        .filter(|i| i.kind == InkKind::Wordmark)
        .map(|i| i.rect)
        .collect();
    let bottom = name.iter().map(|r| r.y + r.h).max().unwrap_or(0);
    let rest = ink
        .iter()
        .filter(|i| i.kind != InkKind::Wordmark && i.rect.y < bottom)
        .map(|i| i.rect)
        .collect();
    (name, rest)
}

#[test]
fn faraday_demos_narrow_top_bar_draws_its_name_clear_of_everything_in_the_bar() {
    // The Pi panel and Narrow, as Faraday Demo's Size menu previews them,
    // and a phone-wide window, in Practice and on Testnet.
    for size in [(268, 358, 160), (400, 800, 160), (390, 844, 160)] {
        for practice in [true, false] {
            let mut app = home(true, practice, size);
            name_clear(&mut app, size.0, &format!("{size:?} {practice}"));
        }
    }
}

/// The name in `app`'s top bar, drawn clear of everything else in the
/// bar and of the mode pill, inside a window `width` wide.
fn name_clear(app: &mut faraday_core::Faraday, width: u16, case: &str) -> Vec<Rect> {
    let (name, rest) = name_and_bar(app);
    assert!(!name.is_empty(), "{case}: no name in the bar");
    let pill = app
        .hit_box(faraday_core::Action::ModeSheet)
        .map(|(x, y, w, h)| Rect::new(x.into(), y.into(), w.into(), h.into()));
    for n in &name {
        assert!(
            n.x >= 0 && n.x + n.w <= i32::from(width),
            "{case}: the name leaves the window: {n:?}"
        );
        for r in rest.iter().chain(pill.iter()) {
            assert!(
                n.intersect(r).is_empty(),
                "{case}: the name {n:?} is drawn over {r:?}"
            );
        }
    }
    name
}

#[test]
fn the_pis_panel_reads_faraday_offline_on_one_line_without_the_mode_pill() {
    use faraday_bip::keys::Network;
    let mut app = home_on(false, false, Network::Mainnet, (480, 640, 286));
    let name = name_clear(&mut app, 480, "mainnet");
    assert_eq!(name.len(), 1, "FARADAY OFFLINE in one line: {name:?}");
}

#[test]
fn the_pis_panel_puts_offline_under_faraday_beside_the_mode_pill() {
    let mut app = home(false, false, (480, 640, 286));
    let name = name_clear(&mut app, 480, "testnet");
    assert_eq!(name.len(), 2, "OFFLINE under FARADAY: {name:?}");
    assert!(name[1].y > name[0].y, "OFFLINE under FARADAY: {name:?}");
}
