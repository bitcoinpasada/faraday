//! Appearance on Settings (`docs/MOTION.md` §3.6, §3.7): Light draws the
//! screens dark on light and Dark turns them back; every theme is offered
//! once, under its group, in order of its name; the choice is kept across
//! a lock with the other settings.

use faraday_core::ui::{Preset, ThemeGroup};
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

/// How light the page is in the middle of the screen's empty right
/// margin, 0 to 255, once the change has faded in: ticked for half a
/// second as a shell ticks.
fn page_light(app: &mut Faraday, clock: &mut u64) -> u8 {
    for _ in 0..30 {
        *clock += 16;
        app.event(Event::Tick { now_ms: *clock });
        while app.poll_command().is_some() {}
        let _ = app.frame();
    }
    let frame = app.frame();
    let i = (790 * usize::from(frame.width) + 1270) * 4;
    let px = &frame.rgba[i..i + 3];
    ((u16::from(px[0]) + u16::from(px[1]) + u16::from(px[2])) / 3) as u8
}

#[test]
fn light_draws_the_page_light_and_dark_turns_it_back() {
    let mut app = shown();
    let mut clock = 1_000;
    assert!(page_light(&mut app, &mut clock) > 192, "light at first");
    scroll_to(&mut app, &[Action::Theme(Preset::Dark)]);
    app.press(Action::Theme(Preset::Dark));
    assert!(page_light(&mut app, &mut clock) < 64, "dark");
    app.press(Action::Theme(Preset::Light));
    assert!(page_light(&mut app, &mut clock) > 192, "light again");
}

/// A first start is in Cobalt, on every shell.
#[test]
fn a_first_start_is_in_cobalt() {
    assert_eq!(shown().theme(), Preset::Cobalt);
}

#[test]
fn the_choice_is_kept_across_a_lock() {
    let mut app = shown();
    app.press(Action::Theme(Preset::Light));
    assert_eq!(restarted(&mut app).theme(), Preset::Light);
}

#[test]
fn reduce_motion_is_kept_across_a_lock() {
    let mut app = shown();
    scroll_to(&mut app, &[Action::ReduceMotion(true)]);
    app.press(Action::ReduceMotion(true));
    assert!(restarted(&mut app).reduce_motion());
}

/// The page's colour in the screen's empty right margin, and the
/// sidebar's at its left edge, once the change has faded in: two themes
/// may share a white page (`docs/DECISIONS.md` F39), not the pair.
fn page(app: &mut Faraday, clock: &mut u64) -> [[u8; 3]; 2] {
    let _ = page_light(app, clock);
    let frame = app.frame();
    let at = |x: usize| {
        let i = (790 * usize::from(frame.width) + x) * 4;
        [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
    };
    [at(1270), at(2)]
}

#[test]
fn every_theme_is_offered_under_its_group_in_order_of_its_name() {
    let mut app = shown();
    let tiles: Vec<Action> = Preset::ALL.into_iter().map(Action::Theme).collect();
    scroll_to(&mut app, &tiles);
    let texts = app.drawn_text_boxes();
    // Each group's label, in turn down the card: the first text drawn
    // that reads as it, since a theme may share a label's name.
    let label_y: Vec<i32> = ThemeGroup::ALL
        .into_iter()
        .map(|g| {
            let (_, (_, y, _, _)) = texts
                .iter()
                .find(|(t, _)| t == g.name())
                .unwrap_or_else(|| panic!("no {} label", g.name()));
            *y
        })
        .collect();
    assert!(label_y.is_sorted(), "groups out of order: {label_y:?}");
    // Each theme a tile of its own.
    let mut places: Vec<(u16, u16)> = Preset::ALL
        .into_iter()
        .map(|t| app.where_offered(Action::Theme(t)).expect("offered"))
        .collect();
    places.sort_unstable();
    places.dedup();
    assert_eq!(places.len(), Preset::ALL.len(), "two themes share a tile");
    for (g, group) in ThemeGroup::ALL.into_iter().enumerate() {
        let mut mine: Vec<(Preset, (u16, u16))> = Preset::ALL
            .into_iter()
            .filter(|t| t.group() == group)
            .map(|t| (t, app.where_offered(Action::Theme(t)).expect("offered")))
            .collect();
        assert!(!mine.is_empty(), "{} has no themes", group.name());
        for (t, (_, y)) in &mine {
            let y = i32::from(*y);
            assert!(y > label_y[g], "{} is above its group", t.name());
            if let Some(&next) = label_y.get(g + 1) {
                assert!(y < next, "{} is below its group", t.name());
            }
        }
        // In reading order, row by row and left to right, by name.
        mine.sort_by_key(|&(_, (x, y))| (y, x));
        let names: Vec<&str> = mine.iter().map(|(t, _)| t.name()).collect();
        assert!(names.is_sorted(), "{}: {names:?}", group.name());
    }
}

#[test]
fn every_theme_draws_its_own_page_and_is_kept_across_a_restart() {
    let mut app = shown();
    let tiles: Vec<Action> = Preset::ALL.into_iter().map(Action::Theme).collect();
    scroll_to(&mut app, &tiles);
    let mut clock = 1_000;
    let mut seen = Vec::new();
    for theme in Preset::ALL {
        let _ = app.frame();
        assert!(
            app.offers(Action::Theme(theme)),
            "{} not offered",
            theme.name()
        );
        app.press(Action::Theme(theme));
        assert_eq!(app.theme(), theme);
        let colour = page(&mut app, &mut clock);
        // Light and Cobalt open in the same scheme (`docs/THEMES.md` §13).
        let same = Preset::ALL
            .into_iter()
            .take_while(|t| *t != theme)
            .any(|t| t.theme() == theme.theme());
        assert!(
            same || !seen.contains(&colour),
            "{} draws a page already seen",
            theme.name()
        );
        seen.push(colour);
        assert_eq!(
            restarted(&mut app).theme(),
            theme,
            "{} not kept",
            theme.name()
        );
    }
}

#[test]
fn a_named_theme_is_kept_across_a_lock() {
    for theme in [Preset::Gruvbox, Preset::RosePine] {
        let mut app = shown();
        app.press(Action::Theme(theme));
        assert_eq!(restarted(&mut app).theme(), theme);
    }
}
