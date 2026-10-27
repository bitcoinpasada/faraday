//! A Raspberry Pi's panel, 480 × 640 at 286 dpi: no sidebar, Home as the
//! menu, every flow a page per step, a keyboard docked at the foot. What
//! a person can reach there by touch alone, scrolling where a page is
//! longer than the panel.

use faraday_core::keygen::{Group, Way};
use faraday_core::vault::VaultAction as V;
use faraday_core::vault::vstep;
use faraday_core::{Action, Faraday, OskPress, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

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
    // The frame that shows the press, and the tick after it, on which a
    // press that stretches a key does its work (`docs/SPEED.md` §9).
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
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
        Screen::WalletsStart,
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

/// Spend left the small Home grid (`docs/history/SIMPLIFY.md` §6.2): the
/// Wallets empty state and the Learn sheet reach it instead.
#[test]
fn homes_grid_has_no_spend_tile() {
    let mut app = panel();
    assert!(on_panel(&mut app, Action::Nav(Screen::Family)).is_none());
}

#[test]
fn a_wallet_is_made_by_touch_from_a_key_rolled_on_dice() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    // Create opens on Kind; Single key opens Use vaults, whose No moves
    // on to Keys: with nothing loaded, New key for its slot.
    tap(&mut app, Action::CKind(0));
    tap(&mut app, Action::CVaults(false));
    assert_eq!(app.screen, Screen::KeyGen);
    // New key opens with Start closed on Make a new key and Length open;
    // pressing a count takes it and opens Randomness (F103).
    assert_eq!(
        app.keygen().and_then(|k| k.open),
        Some(faraday_core::keygen::kstep::LENGTH),
        "Start is closed on its default"
    );
    tap(&mut app, Action::KWords(12));
    assert_eq!(
        app.keygen().and_then(|k| k.open),
        Some(faraday_core::keygen::kstep::SOURCE),
        "Length's Continue opens Randomness"
    );
    // Dice hashed into the words: its group opened first.
    tap(&mut app, Action::KGroup(Group::Computed as u8));
    tap(&mut app, Action::KWay(Way::DiceHashed.index()));
    // The panel starts on the dice's buttons: no keyboard needed.
    for i in 0..99u8 {
        tap(&mut app, Action::KRoll(i % 6 + 1));
    }
    // The entry card's Continue opens Key; Lock in adds the key and,
    // a single key being the whole wallet, makes it, and returns to
    // Create on Check, Back up wallet its foot, by itself (F103).
    tap(&mut app, Action::KNext);
    tap(&mut app, Action::KLock);
    assert_eq!(app.screen, Screen::Create, "the key fills its slot");
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::CHECK)
    );
    assert!(on_panel(&mut app, Action::CBackup).is_some());
    let made = app.create().and_then(|c| c.built);
    assert!(made.is_some(), "Lock in made the wallet");
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
    let typed = app.vaults().create.as_ref().unwrap().phrases[0]
        .1
        .text
        .clone();
    assert_eq!(typed.as_str(), "aa", "the keys type into the field");
}

#[test]
fn a_step_s_walk_through_opens_and_closes_on_a_tap_in_steps_only_too() {
    let mut app = panel();
    // A first start is Steps only (F82), and this walk-through opens on
    // a tap there too.
    assert!(!app.guided(), "Steps only is the first start");
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    // Create a vault opens on its second card, Name and passphrases.
    let about = Action::About(1);
    tap(&mut app, about);
    assert_eq!(app.about_open(), Some((Screen::CreateVault, 1)));
    tap(&mut app, about);
    assert_eq!(app.about_open(), None, "a second tap puts it away");
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

/// Each step of Create a vault and Make a new wallet, opened on its own:
/// its forward action is on the panel without scrolling.
fn forward_in_view(mut app: Faraday, h: u16) {
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    // The size, opened on its own; then Customise's three cards in its
    // place.
    tap(&mut app, Action::Vault(V::CStep(vstep::PRESET)));
    // A size, a cost or a space pressed moves on (F103): the first row
    // is the way on there.
    let next = Action::Vault(V::CSizeRow(0));
    assert!(in_view(&mut app, h, next), "Create a vault, the size");
    tap(&mut app, Action::Vault(V::CCustomise));
    for next in [V::CNext(vstep::WHERE), V::CPreset(0), V::CSize(0)] {
        let next = Action::Vault(next);
        assert!(in_view(&mut app, h, next), "Create a vault, {next:?}");
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
    // Back, by the bar, through the steps to Vaults (F104), then Home.
    for _ in 0..8 {
        if app.screen == Screen::CreateVault {
            tap(&mut app, Action::Nav(Screen::Vaults));
        }
    }
    assert_eq!(app.screen, Screen::Vaults);
    tap(&mut app, Action::Nav(Screen::Home));
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::CreateWallet);
    // Create opens on Kind. A row is Kind's forward action: Single key,
    // backup in shares, Single key (0) and Multisig (4), shown
    // before More kinds.
    let shares = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| k.shares())
        .expect("the backup in shares") as u8;
    for kind in [shares, 0, 4] {
        assert!(
            in_view(&mut app, h, Action::CKind(kind)),
            "Make a new wallet, Kind, row {kind}"
        );
    }
    tap(&mut app, Action::CKind(shares));
    assert!(
        in_view(&mut app, h, Action::CNext(faraday_core::cstep::QUORUM)),
        "Make a new wallet, the backup in shares' Quorum"
    );
}

/// Create opens on Kind; Single key opens Use vaults, whose No opens
/// Keys (a key loaded, so the card has a choice), whose head then
/// carries Kind with Change.
#[test]
fn kind_is_offered_on_the_keys_page_and_opens() {
    let mut app = panel();
    crate::common::add_key(&mut app, "bacon");
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KIND),
        "Create opens on Kind"
    );
    tap(&mut app, Action::CKind(0));
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::WHERE),
        "Single key opens Use vaults"
    );
    tap(&mut app, Action::CVaults(false));
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KEYS),
        "No opens Keys"
    );
    let kind = Action::CStep(faraday_core::cstep::KIND);
    assert!(
        on_panel(&mut app, kind).is_some(),
        "Kind is on the Keys page"
    );
    tap(&mut app, kind);
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::KIND),
        "Change opens Kind"
    );
}

/// Tapping the page's title shows the list of steps, from which any
/// opens.
#[test]
fn the_page_title_shows_the_steps() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::CreateWallet);
    tap(&mut app, Action::CStep(faraday_core::cstep::KIND));
    assert_eq!(app.create().and_then(|c| c.open), None);
    let check = Action::CStep(faraday_core::cstep::CHECK);
    assert!(on_panel(&mut app, check).is_some(), "every step is listed");
    tap(&mut app, Action::CStep(faraday_core::cstep::WHERE));
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(faraday_core::cstep::WHERE)
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
        Action::Nav(Screen::WalletsStart),
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

/// A flow's page title is the card's name alone, never a count next to
/// it that could be read as one of the card's own values; the step
/// count is its own muted "Step N of M" text beside the title
/// (`DESIGN.md` §4.1, `docs/history/SIMPLIFY.md` §2.6).
#[test]
fn a_flow_s_page_title_carries_no_count() {
    let mut app = panel();
    tap(&mut app, Action::Nav(Screen::Vaults));
    tap(&mut app, Action::Vault(V::Create));
    let texts = app.drawn_texts();
    // Create a vault is two cards, and opens on the second: the size
    // has its default (`docs/history/SIMPLIFY.md` §3.1). The title wraps onto
    // its own line or lines, with no count joined onto it.
    assert!(
        texts
            .iter()
            .any(|t| t == "Name and passphrases" || t == "Name and" || t == "passphrases"),
        "the title's words are drawn on their own: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains('·') && t.contains("of")),
        "the title still carries a count next to it: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "Step 2 of 2"),
        "the step count is drawn as its own, muted text: {texts:?}"
    );
}

/// The small Home leads with a tile per flow in progress, its name and
/// open step, opening it as it was left (`docs/FLOWS.md`, Flows in
/// progress).
#[test]
fn the_small_homes_first_tile_opens_the_flow_in_progress() {
    use faraday_core::in_progress::Going;
    let mut app = panel();
    let row = Action::Resume(Going::Create);
    assert!(on_panel(&mut app, row).is_none(), "nothing in progress");
    tap(&mut app, Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::CNext(faraday_core::cstep::KIND));
    let open = app.create().unwrap().open;
    tap(&mut app, Action::Nav(Screen::Home));
    assert_eq!(app.screen, Screen::Home);
    let (_, at) = on_panel(&mut app, row).expect("at the head of Home, in view");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Make a new wallet"),
        "its name: {texts:?}"
    );
    let (_, tiles) = app.where_offered(Action::Nav(Screen::Vaults)).unwrap();
    assert!(at < tiles, "before the other tiles");
    tap(&mut app, row);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().unwrap().open, open, "as it was left");
}

/// A lead that would start the flow in progress over is not drawn beside
/// its tile: Make a new wallet is on the small Home once, the tile.
#[test]
fn the_small_home_offers_a_flow_in_progress_once_its_tile() {
    use faraday_core::in_progress::Going;
    let mut app = panel();
    tap(&mut app, Action::CreateWallet);
    app.press(Action::CNext(faraday_core::cstep::KIND));
    tap(&mut app, Action::Nav(Screen::Home));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert_eq!(
        texts.iter().filter(|t| *t == "Make a new wallet").count(),
        1,
        "{texts:?}"
    );
    assert!(app.offers(Action::Resume(Going::Create)), "the tile");
    assert!(
        !app.offers(Action::CreateWallet),
        "no lead that starts it over"
    );
}

#[test]
fn every_step_s_forward_action_is_on_a_phone_without_scrolling() {
    forward_in_view(phone(), 1206);
}
