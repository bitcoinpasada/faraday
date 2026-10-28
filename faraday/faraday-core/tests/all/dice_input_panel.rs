//! New key's dice and Create a vault's Generate with dice in the input
//! panel (`docs/LAYOUT.md` §16.3): the pad and what it fills stand
//! together at the window's foot and stay there as the page scrolls; a
//! face pressed there is a roll. On the Pi's panel the pad stays in the
//! card.

use faraday_core::keygen::{Way, kstep};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

/// A started app on a display `width` × `height` at `dpi`.
fn on(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.event(Event::Display(DisplayInfo {
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
    }));
    app
}

/// New key on such a display at its words, open on Entries by the
/// hashed dice procedure, rolls pressed one at a time, a frame drawn.
fn rolling_key(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = on(width, height, dpi);
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    let words = app.keygen().unwrap().words as u8;
    app.press(Action::KWords(words));
    app.press(Action::KNext);
    app.press(Action::KStep(kstep::SOURCE));
    app.press(Action::KWay(Way::DiceHashed.index()));
    if app.keygen().and_then(|k| k.open) != Some(kstep::ENTER) {
        app.press(Action::KNext);
    }
    // One at a time, as one-column windows start; wide ones start typed.
    app.press(Action::KTyping(false));
    let _ = app.frame();
    app
}

/// Presses `action` where the last frame drew it, as a finger would.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

/// The page scrolled down by `dy` with the pointer at (x, y), a frame
/// after.
fn scroll_page(app: &mut Faraday, (x, y): (u16, u16), dy: i16) {
    app.event(Event::Hover { x, y });
    app.event(Event::Scroll { x, y, dy });
    app.event(Event::ScrollEnd { x, y });
    app.settle();
    let _ = app.frame();
}

/// Where the last frame drew `action`, wholly in a window `w` × `h`
/// and in its lower `part`.
fn at_foot(app: &Faraday, action: Action, (w, h): (u16, u16), part: f32) -> (u16, u16, u16, u16) {
    let r = app
        .hit_box(action)
        .unwrap_or_else(|| panic!("{action:?} is drawn"));
    assert!(
        r.0 + r.2 <= w && r.1 + r.3 <= h,
        "{action:?} in the window: {r:?}"
    );
    let top = f32::from(h) * (1.0 - part);
    assert!(
        f32::from(r.1) >= top,
        "{action:?} at the window's foot: {r:?}"
    );
    r
}

#[test]
fn new_keys_pad_stands_at_the_foot_and_stays_as_the_page_scrolls() {
    for (w, h, dpi) in [
        (1280u16, 800u16, 160u16),
        (760, 1000, 160),
        (600, 1024, 170),
    ] {
        let mut app = rolling_key(w, h, dpi);
        let key = at_foot(&app, Action::KRoll(3), (w, h), 0.4);
        scroll_page(&mut app, (w / 2, h / 3), 600);
        let moved = at_foot(&app, Action::KRoll(3), (w, h), 0.4);
        assert_eq!(key, moved, "{w} × {h}: the pad stays as the page scrolls");
        tap(&mut app, Action::KRoll(3));
        assert_eq!(app.keygen().unwrap().dice.len(), 1, "{w} × {h}: a roll");
        let count = app
            .drawn_texts()
            .into_iter()
            .find(|t| t.starts_with("1 of "))
            .unwrap_or_else(|| panic!("{w} × {h}: the count says one roll"));
        let count_box = app
            .drawn_text_boxes()
            .into_iter()
            .find(|(t, _)| *t == count)
            .map(|b| b.1)
            .unwrap();
        let below = i32::from(key.1) - count_box.1;
        assert!(
            (0..i32::from(h) / 4).contains(&below),
            "{w} × {h}: the count stands over the pad: {count_box:?} {key:?}"
        );
    }
}

#[test]
fn on_the_pi_panel_new_keys_pad_stays_in_the_card() {
    let mut app = rolling_key(480, 640, 286);
    let key = app.hit_box(Action::KRoll(3)).expect("the pad is drawn");
    scroll_page(&mut app, (100, 100), 200);
    let moved = app.hit_box(Action::KRoll(3));
    assert_ne!(Some(key), moved, "the pad scrolls with the card");
}

#[test]
fn create_a_vaults_dice_pad_at_the_foot_types_the_rolls() {
    let (w, h) = (1280u16, 800u16);
    let mut app = on(w, h, 160);
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    let _ = app.frame();
    at_foot(&app, Action::Vault(V::DiceKey(3)), (w, h), 0.4);
    at_foot(&app, Action::Vault(V::DiceKey(0)), (w, h), 0.4);
    for f in [3, 5, 0, 6] {
        tap(&mut app, Action::Vault(V::DiceKey(f)));
    }
    let (_, rolls, _) = app.vaults().dice.as_ref().unwrap();
    assert_eq!(rolls.text.as_str(), "36", "3, 5, Backspace, 6");
}
