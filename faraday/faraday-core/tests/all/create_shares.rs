//! Make a new wallet's Single key, backup in shares (`docs/CODEX32.md`
//! §6.9): offered on Kind between Single key and Multisig, its Quorum
//! the codex32 scheme, its one slot opening Make codex32 shares with
//! that scheme fixed, and the wallet a single-key native SegWit one whose
//! Check names the set. Restore does not offer the kind.

use faraday_core::codex32::{C32, Card, WAYS, Way};
use faraday_core::create::NewKind;
use faraday_core::keygen::kstep;
use faraday_core::{Action, Faraday, Screen, cstep};
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware, TouchPhase,
};
use faraday_ui::keyboard::OskPress;

fn shares_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Shares)
        .expect("the backup in shares is a kind") as u8
}

fn app() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    app
}

/// Create opened, the backup in shares chosen: Quorum open.
fn on_quorum() -> Faraday {
    on_quorum_of(app())
}

/// [`on_quorum`] on `app`.
fn on_quorum_of(mut app: Faraday) -> Faraday {
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    app.press(Action::CKind(shares_index()));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::QUORUM));
    app
}

const NAME: &str = "Single key, backup in shares (codex32)";
const TIP: &str = "codex32 is the only method that makes a seed and its shares fully by hand, the checksum included.";

#[test]
fn make_a_new_wallet_opens_on_kind_with_the_backup_in_shares_first_and_ticked() {
    let mut app = app();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    let c = app.create().expect("Make a new wallet");
    assert_eq!(c.open, Some(cstep::KIND), "Kind is open");
    assert_eq!(c.kind, NewKind::Shares, "the backup in shares is ticked");
    let texts = app.drawn_texts();
    let at = |name: &str| texts.iter().position(|t| t == name);
    let (shares, single, multi) = (
        at(NAME).expect("the backup in shares"),
        at("Single key · native SegWit").expect("Single key"),
        at("Multisig · native SegWit").expect("Multisig"),
    );
    assert!(shares < single && single < multi, "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t == "One key, its backup in 3 shares, any 2 rebuild it"),
        "its line follows the scheme: {texts:?}"
    );
}

#[test]
fn next_with_nothing_changed_opens_the_backup_in_shares_quorum() {
    for next in [Action::CNext(cstep::KIND), Action::CKind(shares_index())] {
        let mut app = app();
        app.press(Action::CreateWallet);
        app.press(next);
        let c = app.create().unwrap();
        assert_eq!(c.kind, NewKind::Shares);
        assert_eq!(c.open, Some(cstep::QUORUM), "{next:?}");
        let texts = app.drawn_texts();
        assert!(
            texts.iter().any(|t| t == "How many to rebuild it"),
            "{texts:?}"
        );
    }
}

/// Make a new wallet open on Kind, with where "codex32" is drawn in the
/// row's name, in pixels, and the clock a second on.
fn on_kind() -> (Faraday, (u16, u16)) {
    let mut app = app();
    app.press(Action::CreateWallet);
    let now = app.now_ms() + 1000;
    app.event(Event::Tick { now_ms: now });
    let (_, (x, y, w, h)) = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == NAME)
        .expect("the row's name");
    // "codex32" is the name's last word but the bracket: its middle is
    // near 35 of the name's 39 characters in.
    let word = ((x + w * 885 / 1000) as u16, (y + h / 2) as u16);
    let _ = app.frame();
    (app, word)
}

/// Whether the tooltip's text is drawn, its lines read in order.
fn tip_shows(app: &mut Faraday) -> bool {
    app.drawn_texts().join(" ").contains(TIP)
}

fn tick(app: &mut Faraday, ms: u64) {
    let now = app.now_ms() + ms;
    app.event(Event::Tick { now_ms: now });
}

#[test]
fn the_pointer_resting_on_codex32_shows_its_tooltip_and_leaving_hides_it() {
    let (mut app, (x, y)) = on_kind();
    app.event(Event::Hover { x, y });
    tick(&mut app, 300);
    assert!(!tip_shows(&mut app), "not before the delay");
    tick(&mut app, 300);
    assert!(tip_shows(&mut app), "after it");
    app.event(Event::Hover { x, y: y + 200 });
    assert!(!tip_shows(&mut app), "the pointer gone, the tooltip goes");
    app.event(Event::Hover { x, y });
    tick(&mut app, 600);
    assert!(tip_shows(&mut app));
    app.event(Event::HoverEnd);
    assert!(!tip_shows(&mut app));
    let c = app.create().unwrap();
    assert_eq!(
        (c.kind, c.open),
        (NewKind::Shares, Some(cstep::KIND)),
        "hovering pressed nothing"
    );
}

#[test]
fn the_pointer_resting_on_the_info_button_shows_the_tooltip() {
    let (mut app, _) = on_kind();
    let (x, y) = app
        .where_offered(Action::CKindAbout)
        .expect("the info button");
    app.event(Event::Hover { x, y });
    tick(&mut app, 300);
    assert!(!tip_shows(&mut app), "not before the delay");
    tick(&mut app, 300);
    assert!(tip_shows(&mut app), "after it");
    app.event(Event::HoverEnd);
    assert!(!tip_shows(&mut app));
}

/// A finger or a click on (x, y), down and up.
fn tap_at(app: &mut Faraday, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
}

#[test]
fn a_tap_on_the_info_button_shows_the_tooltip_at_once_and_a_press_elsewhere_hides_it() {
    let (mut app, _) = on_kind();
    let at = app
        .where_offered(Action::CKindAbout)
        .expect("the info button");
    tap_at(&mut app, at);
    assert!(tip_shows(&mut app), "at once");
    assert_eq!(
        app.create().unwrap().open,
        Some(cstep::KIND),
        "the row is not chosen"
    );
    tick(&mut app, 5000);
    assert!(tip_shows(&mut app), "it stays to be read");
    app.event(Event::Key(Key::Escape));
    assert!(!tip_shows(&mut app), "Escape takes it off");
    tap_at(&mut app, at);
    assert!(tip_shows(&mut app));
    app.event(Event::Touch {
        x: 5,
        y: 5,
        phase: TouchPhase::Down,
    });
    assert!(!tip_shows(&mut app), "a press elsewhere takes it off");
}

#[test]
fn a_tap_on_the_row_beside_the_info_button_chooses_the_row() {
    let (mut app, _) = on_kind();
    let row = app
        .where_offered(Action::CKind(shares_index()))
        .expect("the row");
    tap_at(&mut app, row);
    tick(&mut app, 100);
    assert_eq!(app.create().unwrap().open, Some(cstep::QUORUM));
    assert!(!tip_shows(&mut app));
}

#[test]
fn the_keyboard_on_the_info_button_shows_its_tooltip_and_escape_hides_it() {
    let (mut app, _) = on_kind();
    let mut shown = false;
    for _ in 0..200 {
        app.event(Event::Key(Key::Tab));
        tick(&mut app, 600);
        if tip_shows(&mut app) {
            shown = true;
            break;
        }
    }
    assert!(shown, "Tab reaches the info button, and the tooltip shows");
    assert_eq!(
        app.create().unwrap().open,
        Some(cstep::KIND),
        "nothing pressed"
    );
    app.event(Event::Key(Key::Escape));
    assert!(!tip_shows(&mut app), "Escape takes it off");
    assert_eq!(app.screen, Screen::Create, "and does nothing else");
    // Enter presses the button the keyboard is on: the tooltip at once,
    // the row not chosen.
    app.event(Event::Key(Key::Enter));
    assert!(tip_shows(&mut app));
    assert_eq!(app.create().unwrap().open, Some(cstep::KIND));
    app.event(Event::Key(Key::Escape));
    app.event(Event::Key(Key::Tab));
    tick(&mut app, 600);
    assert!(!tip_shows(&mut app), "the keyboard moved on");
}

#[test]
fn a_long_press_on_codex32_shows_its_tooltip_and_chooses_nothing() {
    let (mut app, (x, y)) = on_kind();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    tick(&mut app, 600);
    assert!(tip_shows(&mut app), "held, the tooltip shows");
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    assert_eq!(
        app.create().unwrap().open,
        Some(cstep::KIND),
        "the lift presses nothing"
    );
    tick(&mut app, 3000);
    assert!(tip_shows(&mut app), "it stays to be read");
    app.event(Event::Touch {
        x: 5,
        y: 5,
        phase: TouchPhase::Down,
    });
    assert!(!tip_shows(&mut app), "the next press takes it off");
}

#[test]
fn a_tap_on_codex32_chooses_the_row() {
    let (mut app, (x, y)) = on_kind();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    tick(&mut app, 100);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    tick(&mut app, 600);
    assert_eq!(app.create().unwrap().open, Some(cstep::QUORUM));
    assert!(!tip_shows(&mut app));
}

#[test]
fn the_kind_is_hidden_while_make_codex32_shares_is_not_offered() {
    let shares = usize::from(shares_index());
    for more in [false, true] {
        let hidden = NewKind::create_rows(more, false);
        assert!(!hidden.contains(&shares), "hidden, More kinds {more}");
        let shown = NewKind::create_rows(more, true);
        assert!(shown.contains(&shares), "shown, More kinds {more}");
        assert_eq!(hidden.len() + 1, shown.len(), "nothing else changes");
    }
    assert_eq!(
        NewKind::create_rows(false, false),
        vec![0, 4],
        "Single key and Multisig before More kinds"
    );
}

#[test]
fn restore_does_not_offer_the_kind() {
    let mut app = app();
    app.press(Action::RestoreWallet);
    let _ = app.frame();
    assert!(!app.offers(Action::RKind(shares_index())));
}

#[test]
fn its_quorum_is_the_codex32_scheme() {
    let mut app = on_quorum();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "How many to rebuild it"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t == "How many shares"), "{texts:?}");
    assert!(
        app.offers(Action::CAccount(0)),
        "the Account row at Quorum's foot"
    );
    let scheme = |app: &Faraday| app.create().map(|c| (c.share_k, c.share_n));
    assert_eq!(scheme(&app), Some((2, 3)), "2 of 3 to begin with");
    for _ in 0..3 {
        app.press(Action::CM(-1));
    }
    assert_eq!(
        scheme(&app),
        Some((2, 3)),
        "never fewer than 2 to rebuild it"
    );
    for _ in 0..12 {
        app.press(Action::CM(1));
    }
    assert_eq!(scheme(&app), Some((9, 9)), "at most 9; the shares follow");
    for _ in 0..40 {
        app.press(Action::CN(1));
    }
    assert_eq!(scheme(&app), Some((9, 31)), "at most 31 shares");
    for _ in 0..40 {
        app.press(Action::CN(-1));
    }
    assert_eq!(
        scheme(&app),
        Some((9, 9)),
        "never fewer shares than rebuild it"
    );
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Any 9 of the 9 shares rebuild the key."),
        "{texts:?}"
    );
    // Kind's row says the scheme chosen.
    assert_eq!(
        NewKind::Shares.create_line((2, 3), (9, 9)),
        "One key, its backup in 9 shares, any 9 rebuild it"
    );
}

/// Quorum at 3 of 5, Continue, No to vaults: Keys, whose one way on is
/// Make codex32 shares, opens it in its place.
fn on_keys(app: &mut Faraday) {
    app.press(Action::CM(1));
    app.press(Action::CN(1));
    app.press(Action::CN(1));
    app.press(Action::CNext(cstep::QUORUM));
    app.press(Action::CVaults(false));
    assert_eq!(app.screen, Screen::KeyGen);
    assert!(app.keygen().unwrap().codex32.is_some());
}

/// [`on_keys`], then its Cancel with nothing entered and the Keys card
/// opened by hand.
fn on_keys_card(app: &mut Faraday) {
    on_keys(app);
    app.press(Action::CancelFlow);
    assert_eq!(app.screen, Screen::Create, "nothing entered: no question");
    app.press(Action::CStep(cstep::KEYS));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
}

#[test]
fn its_slot_offers_only_make_codex32_shares() {
    let mut app = app();
    crate::common::add_key(&mut app, "bacon");
    let mut app = on_quorum_of(app);
    on_keys_card(&mut app);
    let fp = app.session.keys[0].fingerprint.0;
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Make codex32 shares"),
        "{texts:?}"
    );
    assert!(app.offers(Action::KeyGen(Some(0))));
    assert!(
        !app.offers(Action::CSlotHere(0, fp)),
        "a key of words is not this kind's key"
    );
    // Put there all the same, it does not make the wallet.
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    let c = app.create().unwrap();
    assert!(c.built.is_none());
    assert!(c.error.is_some(), "the reason is shown");
}

/// The slot opens Make codex32 shares on the identifier,
/// the Quorum's scheme fixed.
#[test]
fn its_slot_opens_make_codex32_shares_with_the_scheme_fixed() {
    let mut app = on_quorum();
    on_keys(&mut app);
    app.press(Action::KeyGen(Some(0)));
    let k = app.keygen().expect("New key");
    assert_eq!(k.open, Some(kstep::CODEX32));
    let m = k.codex32.as_ref().expect("Make codex32 shares");
    assert!(m.fixed());
    assert_eq!(m.k_n(), (3, 5), "the scheme Quorum chose");
    assert_eq!(m.open(), Some(Card::Identifier), "Scheme is skipped");
}

fn key(app: &mut Faraday, ch: char) {
    app.press(Action::Codex32(C32::Key(OskPress::Char(ch))));
}

/// BIP 93's vector 2 typed in at 2 of 3 makes a native
/// SegWit wallet of its one key, whose Check names the set.
#[test]
fn the_set_made_fills_the_slot_and_check_names_it() {
    let all = [
        "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
        "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
        "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg",
    ];
    let mut app = on_quorum();
    app.press(Action::CNext(cstep::QUORUM));
    app.press(Action::CVaults(false));
    app.press(Action::KeyGen(Some(0)));
    for ch in "name".chars() {
        key(&mut app, ch);
    }
    app.press(Action::Codex32(C32::Next));
    let w = WAYS.iter().position(|w| *w == Way::Typed).unwrap();
    app.press(Action::Codex32(C32::Way(w as u8)));
    app.press(Action::Codex32(C32::Next));
    for s in &all[..2] {
        for ch in s[9..35].chars() {
            key(&mut app, ch);
        }
    }
    app.press(Action::Codex32(C32::Next));
    let _ = app.frame();
    app.press(Action::Codex32(C32::Next));
    for s in &all {
        for ch in s[3..].chars() {
            key(&mut app, ch);
        }
        let _ = app.frame();
        app.press(Action::Codex32(C32::Next));
    }
    let _ = app.frame();
    app.press(Action::Codex32(C32::Lock));
    let c = app.create().expect("create");
    let i = c.built.expect("Lock in made the wallet");
    let fp = app.session.keys[0].fingerprint.0;
    let set = app.create_shares_set(i).expect("the set, for Back up");
    assert_eq!(
        (
            set.fingerprint,
            set.identifier.as_str(),
            set.k,
            set.n,
            set.length
        ),
        (fp, "name", 2, 3, 48)
    );
    assert!(
        matches!(
            app.session.wallets[i].policy().template(),
            faraday_bip::policy::Template::Single {
                script: faraday_bip::keys::ScriptType::NativeSegwit
            }
        ),
        "single-sig native SegWit, one xpub"
    );
    app.press(Action::Codex32(C32::Next));
    assert_eq!(app.screen, Screen::Create);
    // Lock in opened Check.
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::CHECK));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "codex32 name · 2 of 3"),
        "Check's Backup row: {texts:?}"
    );
}
