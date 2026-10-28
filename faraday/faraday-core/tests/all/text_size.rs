//! Text size (Settings › Appearance, `docs/TEXT-SIZE.md`): Standard,
//! Large and Larger draw text larger (Settings offers the first two),
//! the choice is kept across a lock,
//! larger text takes the one-column layout where the window is too small
//! for the wide one at that size, and at Larger the screens draw no text
//! cut off at a side or over other text.

use faraday_core::strings::EN;
use faraday_core::ui::TextSize;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// A string drawn and the box of its ink, pixels (x, y, w, h).
type DrawnText = (String, (i32, i32, i32, i32));

fn display(width: u16, height: u16, dpi: u16) -> Event {
    Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

/// A fresh app shown on a `width` × `height` display at `dpi`.
fn shown_on(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(display(width, height, dpi));
    app
}

/// The ink height, pixels, of the tallest `label` the frame draws.
fn ink_height(app: &mut Faraday, label: &str) -> i32 {
    app.drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == label)
        .map(|(_, b)| b.3)
        .max()
        .unwrap_or_else(|| panic!("{label:?} drawn"))
}

/// Settings on the Pi's panel at `size`: its first section's name's ink
/// height.
fn settings_label_at(size: TextSize) -> i32 {
    let mut app = shown_on(480, 640, 286);
    app.press(Action::TextSize(size));
    app.press(Action::Nav(Screen::Settings));
    ink_height(&mut app, EN.settings_mode)
}

#[test]
fn larger_text_draws_a_settings_label_taller_and_large_between() {
    let standard = settings_label_at(TextSize::Standard);
    let large = settings_label_at(TextSize::Large);
    let larger = settings_label_at(TextSize::Larger);
    assert!(
        standard < large && large < larger,
        "{standard} {large} {larger}"
    );
}

/// Settings offers Standard and Large; Larger is not offered until the
/// screens are refitted for it (`docs/TEXT-SIZE.md` §2).
#[test]
fn settings_offers_standard_and_large_under_appearance() {
    let mut app = shown_on(1280, 800, 160);
    app.press(Action::Nav(Screen::Settings));
    let _ = app.drawn_texts();
    for size in [TextSize::Standard, TextSize::Large] {
        assert!(app.offers(Action::TextSize(size)), "{size:?}");
    }
    assert!(!app.offers(Action::TextSize(TextSize::Larger)));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == EN.settings_text_size));
    for name in [EN.settings_text_standard, EN.settings_text_large] {
        assert!(texts.iter().any(|t| t == name), "{name}");
    }
    assert!(!texts.iter().any(|t| t == EN.settings_text_larger));
}

/// What the app keeps for the next process, as a shell carries it.
fn kept_of(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    kept
}

#[test]
fn large_is_kept_across_a_lock() {
    let mut app = shown_on(480, 640, 286);
    app.press(Action::TextSize(TextSize::Large));
    app.press(Action::Nav(Screen::Settings));
    let before = ink_height(&mut app, EN.settings_mode);
    let kept = kept_of(&mut app);
    let mut next = shown_on(480, 640, 286);
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    assert_eq!(next.text_size(), TextSize::Large);
    next.press(Action::Nav(Screen::Settings));
    assert_eq!(ink_height(&mut next, EN.settings_mode), before);
    assert!(before > settings_label_at(TextSize::Standard));
}

/// Larger, chosen where it can be (a render) and kept, comes back as
/// Large while Settings does not offer it.
#[test]
fn a_kept_larger_comes_back_as_large() {
    let mut app = shown_on(480, 640, 286);
    app.press(Action::TextSize(TextSize::Larger));
    let kept = kept_of(&mut app);
    let mut next = shown_on(480, 640, 286);
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    assert_eq!(next.text_size(), TextSize::Large);
}

#[test]
fn a_desktop_window_keeps_the_sidebar_at_standard_and_not_at_larger() {
    let mut app = shown_on(1280, 800, 160);
    assert!(!app.is_compact());
    app.press(Action::TextSize(TextSize::Larger));
    assert!(app.is_compact());
    app.press(Action::TextSize(TextSize::Standard));
    assert!(!app.is_compact());
    let mut wide = shown_on(1920, 1080, 160);
    wide.press(Action::TextSize(TextSize::Larger));
    assert!(!wide.is_compact());
}

/// `docs/TEXT-SIZE.md` §1's finding: a 1280 × 800 window at Display scale
/// 200 % is 640 × 400 dp, too small for the sidebar.
#[test]
fn display_scale_200_on_a_1280_by_800_window_lays_out_one_column() {
    let mut app = shown_on(1280, 800, 160);
    app.press(Action::Scale(200));
    assert!(app.is_compact());
    let (w, h) = app.size();
    assert!(
        (w - 640.0).abs() < 0.5 && (h - 400.0).abs() < 0.5,
        "{w} {h}"
    );
}

/// Text boxes, pixels, over one another by more than a pixel each way.
/// Two marks of a grid, each a character or two, sit close by design.
fn drawn_over(texts: &[DrawnText]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, (a, ra)) in texts.iter().enumerate() {
        for (b, rb) in &texts[i + 1..] {
            let marks = a.chars().count() < 3 && b.chars().count() < 3;
            if (a == b && ra == rb) || marks {
                continue;
            }
            let w = (ra.0 + ra.2).min(rb.0 + rb.2) - ra.0.max(rb.0);
            let h = (ra.1 + ra.3).min(rb.1 + rb.3) - ra.1.max(rb.1);
            if w > 1 && h > 1 {
                out.push(format!("text {a:?} drawn over {b:?} by {w} × {h} px"));
            }
        }
    }
    out
}

/// Test 5's check (`docs/TEXT-SIZE.md` §7): the text the page drawn now
/// has cut off at a side of its clip and, with `over`, drawn over other
/// text, from its top to its foot, each line named with `what`; and what
/// it offers to a press anywhere along it. A clip's top and foot are
/// where the page scrolls and are not counted.
fn text_cut_or_over(app: &mut Faraday, what: &str, over: bool) -> (Vec<String>, Vec<Action>) {
    let (w, h) = app
        .last_display_info()
        .map_or((0, 0), |d| (d.width, d.height));
    let (x, y, dy) = (w / 2, h / 2, (h / 4) as i16);
    let mut found: Vec<String> = Vec::new();
    let mut offered: Vec<Action> = Vec::new();
    let mut last = None;
    for _ in 0..40 {
        let (past, texts) = app.drawn_past_sides_and_text();
        let mut lines: Vec<String> = past
            .into_iter()
            .filter(|p| p.starts_with("text "))
            .collect();
        if over {
            lines.extend(drawn_over(&texts));
        }
        for p in lines {
            let line = format!("{what}: {p}");
            if !found.contains(&line) {
                found.push(line);
            }
        }
        offered.extend(app.drawn_actions());
        if last.as_ref() == Some(&texts) {
            break;
        }
        last = Some(texts);
        app.event(Event::Scroll { x, y, dy });
        app.settle();
    }
    for _ in 0..40 {
        app.event(Event::Scroll { x, y, dy: -dy });
    }
    app.settle();
    (found, offered)
}

/// The page drawn now, from its top to its foot, checked as
/// [`text_cut_or_over`] checks it: its findings added to `found`, what
/// it offers given back.
fn check(app: &mut Faraday, found: &mut Vec<String>, what: &str, over: bool) -> Vec<Action> {
    app.settle();
    let (cut, offered) = text_cut_or_over(app, what, over);
    found.extend(cut);
    offered
}

/// Home, Settings, Add a key's word entry, New key's first steps, the
/// lock sheet over Home and a Learn page at Larger on a `width` ×
/// `height` display at `dpi`: none with text cut at a side, and the
/// pages with no text over other text. A sheet's text stands over the
/// page under it by design, so a sheet is checked for its sides alone.
fn at_larger((width, height, dpi): (u16, u16, u16)) -> Vec<String> {
    use faraday_core::keygen::kstep;
    let mut found = Vec::new();
    let mut app = shown_on(width, height, dpi);
    app.press(Action::TextSize(TextSize::Larger));
    let on = format!("{width}×{height} at {dpi} dpi");
    app.press(Action::Nav(Screen::Home));
    check(&mut app, &mut found, &format!("{on}, Home"), true);
    app.press(Action::Nav(Screen::Settings));
    let mut offered = check(&mut app, &mut found, &format!("{on}, Settings"), true);
    // Appearance, where Text size is chosen, as its own page where
    // Settings lists its sections.
    let appearance = Action::SettingsSection(Some(4));
    if offered.contains(&appearance) {
        app.press(appearance);
        let what = format!("{on}, Settings › Appearance");
        offered.extend(check(&mut app, &mut found, &what, true));
    }
    for size in TextSize::OFFERED {
        assert!(offered.contains(&Action::TextSize(size)), "{on}: {size:?}");
    }
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry, "{on}");
    check(&mut app, &mut found, &format!("{on}, Add a key"), true);
    app.press(Action::KeyGen(None));
    for (step, name) in [
        (kstep::LENGTH, "Length"),
        (kstep::START, "Start"),
        (kstep::SOURCE, "Randomness"),
    ] {
        app.press(Action::KStep(step));
        let what = format!("{on}, New key › {name}");
        check(&mut app, &mut found, &what, true);
    }
    app.press(Action::Learn);
    check(&mut app, &mut found, &format!("{on}, Learn"), false);
    app.press(Action::Cancel);
    app.press(Action::Nav(Screen::Home));
    app.sheet = Some(Sheet::Lock);
    check(
        &mut app,
        &mut found,
        &format!("{on}, the lock sheet"),
        false,
    );
    found
}

#[test]
fn at_larger_the_smallest_panel_draws_no_text_cut_off_or_over_other_text() {
    let found = at_larger((240, 320, 143));
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn at_larger_a_desktop_window_draws_no_text_cut_off_or_over_other_text() {
    let found = at_larger((1280, 800, 160));
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// Test 6 (`docs/TEXT-SIZE.md` §7): at Larger on the smallest panel,
/// Add a key's letter keys are all on the screen, and a touch on each
/// one a word can start with types its letter.
#[test]
fn at_larger_on_the_smallest_panel_each_word_key_is_on_screen_and_types_its_letter() {
    use faraday_shell_api::TouchPhase;
    let mut app = shown_on(240, 320, 143);
    app.press(Action::TextSize(TextSize::Larger));
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::Entry(None));
    app.settle();
    let mut typed = 0;
    for c in 'a'..='z' {
        let _ = app.frame();
        // BIP-39 has no word that starts with x: its key is drawn
        // disabled and takes no press.
        if c == 'x' {
            assert!(!app.offers(Action::EntryKey(c)), "x is disabled");
            continue;
        }
        let Some((x, y, w, h)) = app.hit_box(Action::EntryKey(c)) else {
            panic!("the {c} key is drawn");
        };
        assert!(
            u32::from(x) + u32::from(w) <= 240 && u32::from(y) + u32::from(h) <= 320,
            "the {c} key ({x}, {y}, {w} × {h}) is on the panel"
        );
        let (x, y) = app.where_offered(Action::EntryKey(c)).unwrap();
        for phase in [TouchPhase::Down, TouchPhase::Up] {
            app.event(Event::Touch { x, y, phase });
        }
        let _ = app.frame();
        let prefix: String = faraday_core::testkit::entry(&app)
            .keys
            .as_ref()
            .expect("word entry")
            .prefix()
            .iter()
            .collect();
        assert_eq!(prefix, c.to_string(), "the {c} key");
        typed += 1;
        app.press(Action::EntryKeyBack);
    }
    assert_eq!(typed, 25);
}
