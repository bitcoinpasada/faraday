//! Settings › Appearance (`docs/THEMES.md` §12, §13; `docs/MOTION.md`
//! §3.5): Theme, Mode and Contrast each change what is drawn and are kept
//! across a lock; every theme is offered once, by its name; Reduce
//! motion is a switch whose choice is kept.

use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn shown() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Appearance");
    app
}

/// Scrolls Settings down, as a person would, until a frame offers every
/// one of `actions` at once.
fn scroll_to(app: &mut Faraday, actions: &[Action]) {
    for _ in 0..40 {
        let _ = app.frame();
        if actions.iter().all(|&a| app.offers(a)) {
            return;
        }
        app.event(Event::Scroll {
            x: 640,
            y: 400,
            dy: 120,
        });
    }
    panic!("Settings never shows {actions:?} at once");
}

/// A fresh start given what `app` keeps.
fn restarted(app: &mut Faraday) -> Faraday {
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    let mut next = Faraday::new();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    next
}

/// The frame once a change has faded in: ticked for half a second as a
/// shell ticks.
fn settled(app: &mut Faraday, clock: &mut u64) -> Vec<u8> {
    for _ in 0..30 {
        *clock += 16;
        app.event(Event::Tick { now_ms: *clock });
        while app.poll_command().is_some() {}
        let _ = app.frame();
    }
    app.frame().rgba.to_vec()
}

/// How light the page is in the middle of the screen's empty right
/// margin, 0 to 255, once the change has faded in.
fn page_light(app: &mut Faraday, clock: &mut u64) -> u8 {
    let rgba = settled(app, clock);
    let i = (790 * 1280 + 1270) * 4;
    let px = &rgba[i..i + 3];
    ((u16::from(px[0]) + u16::from(px[1]) + u16::from(px[2])) / 3) as u8
}

/// The page's colour in the screen's empty right margin, and the
/// sidebar's at its left edge, once the change has faded in.
fn page(app: &mut Faraday, clock: &mut u64) -> [[u8; 3]; 2] {
    let rgba = settled(app, clock);
    let at = |x: usize| {
        let i = (790 * 1280 + x) * 4;
        [rgba[i], rgba[i + 1], rgba[i + 2]]
    };
    [at(1270), at(2)]
}

/// The scheme on screen with `f` applied.
fn with(app: &Faraday, f: impl FnOnce(&mut Theme)) -> Theme {
    let mut t = app.theme();
    f(&mut t);
    t
}

#[test]
fn dark_draws_the_page_dark_and_light_turns_it_back() {
    let mut app = shown();
    let mut clock = 1_000;
    assert!(page_light(&mut app, &mut clock) > 192, "light at first");
    let dark = with(&app, |t| t.mode = Mode::Dark);
    scroll_to(&mut app, &[Action::Theme(dark)]);
    app.press(Action::Theme(dark));
    assert!(page_light(&mut app, &mut clock) < 64, "dark");
    let light = with(&app, |t| t.mode = Mode::Light);
    let _ = app.frame();
    assert!(app.offers(Action::Theme(light)), "Light offered in Dark");
    app.press(Action::Theme(light));
    assert!(page_light(&mut app, &mut clock) > 192, "light again");
}

/// A first start is in Cobalt, Light, Standard, on every shell.
#[test]
fn a_first_start_is_in_cobalt_light_at_standard_contrast() {
    assert_eq!(
        shown().theme(),
        Theme::new(ThemeName::Cobalt, Mode::Light, Contrast::Standard)
    );
}

#[test]
fn each_contrast_level_changes_what_is_drawn_and_is_kept() {
    let mut app = shown();
    let mut clock = 1_000;
    let mut seen = vec![settled(&mut app, &mut clock)];
    for contrast in [Contrast::Medium, Contrast::High, Contrast::Standard] {
        let pick = with(&app, |t| t.contrast = contrast);
        scroll_to(&mut app, &[Action::Theme(pick)]);
        app.press(Action::Theme(pick));
        assert_eq!(app.theme().contrast, contrast);
        let frame = settled(&mut app, &mut clock);
        if contrast != Contrast::Standard {
            assert!(!seen.contains(&frame), "{contrast:?} draws as before");
        }
        seen.push(frame);
        assert_eq!(restarted(&mut app).theme(), pick, "{contrast:?} not kept");
    }
}

#[test]
fn every_theme_is_offered_once_in_the_mode_and_contrast_on_screen() {
    let mut app = shown();
    let dark_high = Theme::new(ThemeName::Cobalt, Mode::Dark, Contrast::High);
    app.press(Action::Theme(dark_high));
    let chips: Vec<Action> = ThemeName::ALL
        .into_iter()
        .map(|name| Action::Theme(Theme { name, ..dark_high }))
        .collect();
    scroll_to(&mut app, &chips);
    let texts = app.drawn_texts();
    let mut places: Vec<(u16, u16)> = chips
        .iter()
        .map(|&a| app.where_offered(a).expect("offered"))
        .collect();
    places.sort_unstable();
    places.dedup();
    assert_eq!(
        places.len(),
        ThemeName::ALL.len(),
        "two themes share a chip"
    );
    for name in ThemeName::ALL {
        assert!(
            texts.iter().any(|t| t == name.name()),
            "{} not named",
            name.name()
        );
    }
}

#[test]
fn every_theme_draws_its_own_page_and_is_kept_across_a_restart() {
    let mut app = shown();
    let mut clock = 1_000;
    let mut seen = Vec::new();
    for mode in Mode::ALL {
        // The mode first, from its segment; then each theme's chip in it.
        let in_mode = with(&app, |t| t.mode = mode);
        scroll_to(&mut app, &[Action::Theme(in_mode)]);
        app.press(Action::Theme(in_mode));
        for name in ThemeName::ALL {
            let pick = Theme::new(name, mode, Contrast::Standard);
            scroll_to(&mut app, &[Action::Theme(pick)]);
            app.press(Action::Theme(pick));
            assert_eq!(app.theme(), pick);
            let colour = page(&mut app, &mut clock);
            assert!(
                !seen.contains(&colour),
                "{pick:?} draws a page already seen"
            );
            seen.push(colour);
            assert_eq!(restarted(&mut app).theme(), pick, "{pick:?} not kept");
        }
    }
}

#[test]
fn reduce_motion_is_a_switch_and_is_kept_across_a_lock() {
    let mut app = shown();
    assert!(!app.reduce_motion(), "off at first");
    scroll_to(&mut app, &[Action::ReduceMotion(true)]);
    assert!(!app.offers(Action::ReduceMotion(false)), "one switch");
    app.press(Action::ReduceMotion(true));
    let _ = app.frame();
    assert!(app.offers(Action::ReduceMotion(false)), "and it turns off");
    assert!(restarted(&mut app).reduce_motion());
    app.press(Action::ReduceMotion(false));
    assert!(!restarted(&mut app).reduce_motion());
}
