//! A Raspberry Pi's panel, 480 × 640 at 286 dpi: no sidebar, Home as the
//! menu, every flow a page per step, a keyboard docked at the foot. What
//! a person can reach there by touch alone, scrolling where a page is
//! longer than the panel.

use faraday_core::keygen::{Group, Way};
use faraday_core::vaults::VaultAction as V;
use faraday_core::vaults::vstep;
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

/// Spend left the small Home grid (`docs/SIMPLIFY.md` §6.2): the
/// Wallets empty state and the Learn sheet reach it instead.
#[test]
fn homes_grid_has_no_spend_tile() {
    let mut app = panel();
    assert!(on_panel(&mut app, Action::Nav(Screen::Family)).is_none());
}

#[test]
fn a_wallet_is_made_by_touch_from_a_key_rolled_on_dice() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Start));
    tap(&mut app, Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    // Kind already defaults to single key, closed with Change: the flow
    // opens on Keys.
    tap(&mut app, Action::KeyGen(Some(0)));
    assert_eq!(app.screen, Screen::KeyGen);
    // Length (12 words) and Randomness (dice, by words) are already the
    // defaults, closed with Change: the page carries each at its head, and
    // Randomness's Change opens it, to choose dice hashed instead.
    tap(&mut app, Action::KStep(faraday_core::keygen::kstep::SOURCE));
    assert_eq!(
        app.keygen.as_ref().and_then(|k| k.open),
        Some(faraday_core::keygen::kstep::SOURCE),
        "Change opens Randomness"
    );
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
    let made = app.create.as_ref().and_then(|c| c.built);
    assert!(made.is_some(), "Keys' Continue made the wallet");
}

#[test]
fn a_passphrase_field_brings_the_keyboard_and_stays_above_it() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
    // It opens on Name and passphrases.
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
    // A first start is Guided (`docs/SIMPLIFY.md` §6.1); Steps only is a
    // choice, and this walk-through opens on a tap there too.
    assert!(app.guided, "Guided is the first start");
    app.press(Action::Guided(false));
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    // Create a vault opens on its second card, Name and passphrases.
    let about = Action::About(1);
    tap(&mut app, about);
    assert_eq!(app.about_open, Some((Screen::CreateVault, 1)));
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
    // The size, opened on its own; then Customise's three cards in its
    // place.
    tap(&mut app, Action::Vault(V::CStep(vstep::PRESET)));
    let next = Action::Vault(V::CNext(vstep::PRESET));
    assert!(in_view(&mut app, h, next), "Create a vault, the size");
    tap(&mut app, Action::Vault(V::CCustomise));
    for step in [vstep::WHERE, vstep::COST, vstep::SIZE] {
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
    // Kind defaults to single key and opens closed: its Change, at the
    // head of the Keys page, opens it.
    tap(&mut app, Action::CStep(faraday_core::cstep::KIND));
    // Taproot (kind 1) is behind More kinds now; the two always-shown
    // rows are Single key (0) and Multisig (4).
    tap(&mut app, Action::CMoreKinds);
    for kind in 0..2u8 {
        tap(&mut app, Action::CKind(kind));
        assert!(
            in_view(&mut app, h, Action::CNext(faraday_core::cstep::KIND)),
            "Create a wallet, Kind, with kind {kind} chosen"
        );
    }
}

/// A card closed on its default is reached from the page that opens:
/// Create a wallet opens on Keys, whose head carries Kind with Change.
#[test]
fn kind_is_offered_on_the_keys_page_and_opens() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Start));
    tap(&mut app, Action::CreateWallet);
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(faraday_core::cstep::KEYS),
        "it opens on Keys"
    );
    let kind = Action::CStep(faraday_core::cstep::KIND);
    assert!(
        on_panel(&mut app, kind).is_some(),
        "Kind is on the Keys page"
    );
    tap(&mut app, kind);
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(faraday_core::cstep::KIND),
        "Change opens Kind"
    );
}

/// Tapping the page's title shows the list of steps, from which any
/// opens.
#[test]
fn the_page_title_shows_the_steps() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Start));
    tap(&mut app, Action::CreateWallet);
    let keys = Action::CStep(faraday_core::cstep::KEYS);
    tap(&mut app, keys);
    assert_eq!(app.create.as_ref().and_then(|c| c.open), None);
    let back_up = Action::CStep(faraday_core::cstep::BACKUP);
    assert!(
        on_panel(&mut app, back_up).is_some(),
        "every step is listed"
    );
    tap(&mut app, keys);
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(faraday_core::cstep::KEYS)
    );
}

#[test]
fn every_step_s_forward_action_is_on_a_2_8_inch_panel_without_scrolling() {
    forward_in_view(panel(), H);
}

/// Two boxes overlap.
fn overlaps(a: (u16, u16, u16, u16), b: (u16, u16, u16, u16)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

/// Scan floats over the foot of Home; no tile's hit area is under it.
#[test]
fn scan_never_covers_a_tile() {
    let mut app = panel();
    let _ = app.frame();
    let scan = app.hit_box(Action::Scan).expect("Scan is offered");
    for action in [
        Action::Nav(Screen::Start),
        Action::Nav(Screen::Vaults),
        Action::Nav(Screen::Files),
        Action::Learn,
        Action::Nav(Screen::Catalog),
        Action::Nav(Screen::Settings),
    ] {
        if let Some(tile) = app.hit_box(action) {
            assert!(
                !overlaps(scan, tile),
                "{action:?}'s tile is under Scan: {tile:?} vs {scan:?}"
            );
        }
    }
}

/// A flow's page carries no pip row and no "Step N of M" line: the title
/// alone reads "{card} · {n} of {m}" (`DESIGN.md` §4.1).
#[test]
fn a_flow_s_page_title_carries_the_progress() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    let texts = app.drawn_texts();
    // Create a vault is two cards, and opens on the second: the size
    // has its default (`docs/SIMPLIFY.md` §3.1).
    assert!(
        texts.iter().any(|t| t == "Name and passphrases · 2 of 2"),
        "no title carries the progress: {texts:?}"
    );
    assert!(
        !texts
            .iter()
            .any(|t| t.starts_with("Step ") && t.contains(" of ")),
        "a \"Step N of M\" line is still drawn: {texts:?}"
    );
}

#[test]
fn every_step_s_forward_action_is_on_a_phone_without_scrolling() {
    forward_in_view(phone(), 1206);
}
