//! A Raspberry Pi's panel, 480 × 640 at 286 dpi: no sidebar, Home as the
//! menu, every flow a page per step, a keyboard docked at the foot. What
//! a person can reach there by touch alone, scrolling where a page is
//! longer than the panel.

use faraday_core::keygen::{Group, Way};
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, OskPress, Screen};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

const W: u16 = 480;
const H: u16 = 640;

thread_local! {
    /// The size of the panel the test drew on, pixels.
    static SIZE: std::cell::Cell<(u16, u16)> = const { std::cell::Cell::new((W, H)) };
}

/// The app as the Pi shell starts it, drawn once.
fn panel() -> Faraday {
    SIZE.with(|s| s.set((W, H)));
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: W,
        height: H,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Where `action` can be pressed on the panel as it is drawn now.
fn on_panel(app: &mut Faraday, action: Action) -> Option<(u16, u16)> {
    let _ = app.frame();
    let (w, h) = SIZE.with(std::cell::Cell::get);
    app.where_offered(action).filter(|&(x, y)| x < w && y < h)
}

/// Scrolls the page down until `action` is on the panel. Panics when it
/// never comes into view.
fn find(app: &mut Faraday, action: Action) -> (u16, u16) {
    for _ in 0..80 {
        if let Some(at) = on_panel(app, action) {
            return at;
        }
        app.event(Event::Scroll {
            x: W / 2,
            y: H / 2,
            dy: 60,
        });
    }
    panic!("{action:?} is never on the panel");
}

/// A finger on `action`, wherever it has scrolled to.
fn tap(app: &mut Faraday, action: Action) {
    let (x, y) = find(app, action);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    let _ = app.frame();
}

#[test]
fn homes_grid_offers_learn_and_not_add_a_key() {
    let mut app = panel();
    find(&mut app, Action::Learn);
    assert!(
        on_panel(&mut app, Action::Entry(None)).is_none(),
        "Add a key is reached from Wallets now, not Home"
    );
    tap(&mut app, Action::Learn);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Learn));
}

#[test]
fn home_offers_every_place_without_a_sidebar() {
    let mut app = panel();
    assert_eq!(app.screen, Screen::Home);
    for place in [
        Screen::Start,
        Screen::Family,
        Screen::Vaults,
        Screen::Files,
        Screen::Catalog,
        Screen::Settings,
    ] {
        find(&mut app, Action::Nav(place));
    }
    tap(&mut app, Action::Nav(Screen::Settings));
    assert_eq!(app.screen, Screen::Settings);
    tap(&mut app, Action::Nav(Screen::Home));
    assert_eq!(app.screen, Screen::Home, "the bar goes back to Home");
}

#[test]
fn a_wallet_is_made_by_touch_from_a_key_rolled_on_dice() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Start));
    tap(&mut app, Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    tap(&mut app, Action::CKind(0));
    tap(&mut app, Action::CNext(0));
    tap(&mut app, Action::KeyGen(Some(0)));
    assert_eq!(app.screen, Screen::KeyGen);
    tap(&mut app, Action::KWords(12));
    // Dice hashed into the words: its group opened first.
    tap(&mut app, Action::KGroup(Group::Computed as u8));
    tap(&mut app, Action::KWay(Way::DiceHashed.index()));
    tap(&mut app, Action::KNext);
    // The panel starts on the dice's buttons: no keyboard needed.
    for i in 0..99u8 {
        tap(&mut app, Action::KRoll(i % 6 + 1));
    }
    tap(&mut app, Action::KNext);
    tap(&mut app, Action::KNext);
    tap(&mut app, Action::KNext);
    for _ in 0..30 {
        let Some(slot) = app
            .keygen
            .as_ref()
            .and_then(|k| k.quiz.as_ref())
            .filter(|q| q.state() != opensigner_core::quiz::QuizState::Passed)
            .map(|q| q.correct_slot() as u8)
        else {
            break;
        };
        tap(&mut app, Action::KQuiz(slot));
    }
    tap(&mut app, Action::KAdd);
    assert_eq!(app.screen, Screen::Create, "the key fills its slot");
    tap(&mut app, Action::CNext(faraday_core::cstep::KEYS));
    tap(&mut app, Action::CMake);
    let made = app.create.as_ref().and_then(|c| c.built);
    assert!(made.is_some(), "Make the wallet made it");
}

#[test]
fn a_passphrase_field_brings_the_keyboard_and_stays_above_it() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
    for step in 0..3 {
        tap(&mut app, Action::Vault(V::CNext(step)));
    }
    let key = Action::Osk(OskPress::Char('a'));
    assert!(
        on_panel(&mut app, key).is_some(),
        "the first passphrase is up to type"
    );
    // Put away, then another field pressed: the keyboard comes back, and
    // the field it covered moves up above it.
    tap(&mut app, Action::Osk(OskPress::Hide));
    assert!(on_panel(&mut app, key).is_none());
    let field = Action::Vault(V::CFocus(0, true));
    tap(&mut app, field);
    let (_, key_y) = on_panel(&mut app, key).expect("the keyboard is up");
    let (_, field_y) = on_panel(&mut app, field).expect("the field is still on the panel");
    assert!(
        field_y < key_y,
        "the field is above the keyboard, not under it"
    );
    tap(&mut app, key);
    tap(&mut app, key);
    let typed = app.vaults.create.as_ref().unwrap().phrases[0]
        .1
        .text
        .clone();
    assert_eq!(typed.as_str(), "aa", "the keys type into the field");
}

#[test]
fn a_step_s_walk_through_opens_and_closes_on_a_tap_in_steps_only_too() {
    let mut app = panel();
    assert!(!app.guided, "Steps only, as a first start is");
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    let about = Action::About(0);
    tap(&mut app, about);
    assert_eq!(app.about_open, Some((Screen::CreateVault, 0)));
    tap(&mut app, about);
    assert_eq!(app.about_open, None, "a second tap puts it away");
}

/// The app on a phone's panel, 5 inches at 9:16, as `faraday --panel 5`
/// shows it on a laptop of 277 pixels an inch, drawn once.
fn phone() -> Faraday {
    SIZE.with(|s| s.set((678, 1206)));
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 678,
        height: 1206,
        dpi: 277,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Where `action` is on the first screenful, with no scrolling.
fn in_view(app: &mut Faraday, h: u16, action: Action) -> bool {
    let _ = app.frame();
    app.where_offered(action).is_some_and(|(_, y)| y < h)
}

/// Each step of Create a vault and Create a wallet, opened on its own:
/// its forward action is on the panel without scrolling.
fn forward_in_view(mut app: Faraday, h: u16) {
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    for step in 0..3 {
        let next = Action::Vault(V::CNext(step));
        assert!(in_view(&mut app, h, next), "Create a vault, step {step}");
        tap(&mut app, next);
    }
    // The last step opens on its first passphrase with the keyboard up,
    // which leaves no room for the bar: put away, the bar is back.
    if on_panel(&mut app, Action::Osk(OskPress::Hide)).is_some() {
        tap(&mut app, Action::Osk(OskPress::Hide));
    }
    assert!(
        in_view(&mut app, h, Action::Vault(V::CGo)),
        "Create vault, on the last step"
    );
    // Back, by the bar, to Vaults and then Home.
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Nav(Screen::Home));
    tap(&mut app, Action::Nav(Screen::Start));
    tap(&mut app, Action::CreateWallet);
    for kind in 0..2u8 {
        tap(&mut app, Action::CKind(kind));
        assert!(
            in_view(&mut app, h, Action::CNext(faraday_core::cstep::KIND)),
            "Create a wallet, Kind, with kind {kind} chosen"
        );
    }
}

#[test]
fn every_step_s_forward_action_is_on_a_2_8_inch_panel_without_scrolling() {
    forward_in_view(panel(), H);
}

#[test]
fn every_step_s_forward_action_is_on_a_phone_without_scrolling() {
    forward_in_view(phone(), 1206);
}
