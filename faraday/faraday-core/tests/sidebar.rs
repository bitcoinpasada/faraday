//! The sidebar names what is loaded in two groups, seeds and wallets,
//! as far up as the last tab. A group that does not fit shows its first
//! rows and "+ N more", which opens the whole list; each row opens what
//! it names.

use faraday_core::{Action, Faraday, Screen, Sheet};
use osk_bip::bip39::{Language, Mnemonic};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// `seeds` seeds and `wallets` single-key wallets over the first of them,
/// on Files.
fn with(seeds: u8, wallets: usize) -> Faraday {
    with_height(seeds, wallets, 768)
}

/// `with`, at a chosen panel height. Learn's row (§1.4) still costs the
/// sidebar's list room until §6.2 takes Spend out, so a few tests need
/// more height than the panel's own 768 to fit their counts; §6.2
/// returns them to 768.
fn with_height(seeds: u8, wallets: usize, height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..seeds {
        let m = Mnemonic::from_entropy(Language::English, &[i + 1; 16]).unwrap();
        let words: Vec<&str> = m
            .indices()
            .iter()
            .map(|&w| Language::English.words()[usize::from(w)])
            .collect();
        app.session
            .add_words(&words.join(" "), &format!("Seed {i}"), None)
            .unwrap();
    }
    let text = (wallets > 0).then(|| {
        faraday_core::create::NewKind::NativeSegwit
            .key_text(&app.session.keys[0].master)
            .unwrap()
    });
    for a in 0..wallets {
        let text = text.as_deref().unwrap();
        let text = text.replace("/0h]", &format!("/{a}h]"));
        let d = format!("wpkh({text}/<0;1>/*)");
        // The account number differs only in the origin: enough for a
        // wallet of its own in the list.
        app.session
            .add_wallet(&format!("Wallet {a}"), &d, "test")
            .unwrap_or_else(|e| panic!("{d}: {}", e.text()));
    }
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

fn fp(app: &Faraday, k: usize) -> [u8; 4] {
    app.session.keys[k].master.fingerprint().0
}

#[test]
fn the_sidebar_offers_learn_and_opens_the_learn_sheet() {
    let mut app = with(0, 0);
    assert!(app.offers(Action::Learn));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
}

/// Each of the four stages of the session strip in turn.
#[test]
fn the_session_strip_names_the_current_stage() {
    let mut app = with(0, 0);
    assert_eq!(app.session_stage(), "Open");
    app.session
        .add_words("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", "k", None)
        .unwrap();
    assert_eq!(app.session_stage(), "Work");
    app.outbox.clear();
    app.session.keys.clear();
    app.outbox.push(faraday_core::Item {
        name: "a.psbt".into(),
        bytes: Vec::new(),
        kind: faraday_core::wallet::FileKind::Psbt,
        secret: false,
        picture: None,
    });
    assert_eq!(app.session_stage(), "Write out");
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            id: "a".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
        },
    ]));
    assert_eq!(app.session_stage(), "Bring in");
}

#[test]
fn a_few_are_each_named_and_open_what_they_name() {
    let mut app = with_height(3, 2, 900);
    for k in 0..3 {
        assert!(
            app.offers(Action::ExploreKey(fp(&app, k))),
            "seed {k} not listed"
        );
    }
    for w in 0..2 {
        assert!(app.offers(Action::OpenWallet(w)), "wallet {w} not listed");
    }
    assert!(
        !app.offers(Action::Explore),
        "a '+ more' with room to spare"
    );
    let first = fp(&app, 0);
    app.press(Action::ExploreKey(first));
    assert_eq!(app.screen, Screen::Explore);
    assert_eq!(app.explore.as_ref().and_then(|e| e.key), Some(first));
}

#[test]
fn many_show_their_first_rows_and_how_many_more() {
    let app = with_height(30, 12, 900);
    assert!(app.offers(Action::ExploreKey(fp(&app, 0))));
    assert!(
        !app.offers(Action::ExploreKey(fp(&app, 29))),
        "every seed is listed: the corner would cover the tabs"
    );
    assert!(app.offers(Action::Explore), "no '+ more' for seeds");
    assert!(app.offers(Action::OpenWallet(0)));
    assert!(
        app.offers(Action::Nav(Screen::Wallets)),
        "no '+ more' for wallets"
    );
}

#[test]
fn a_wallet_the_seeds_here_sign_for_stays_in_view_among_many() {
    let mut app = with(2, 0);
    let text_of = |app: &Faraday, k: usize, a: usize| {
        faraday_core::create::NewKind::NativeSegwit
            .key_text(&app.session.keys[k].master)
            .unwrap()
            .replace("/0h]", &format!("/{a}h]"))
    };
    // Twelve wallets over the second seed, then one over the first.
    for a in 0..12 {
        let d = format!("wpkh({}/<0;1>/*)", text_of(&app, 1, a));
        app.session
            .add_wallet(&format!("Watched {a}"), &d, "test")
            .unwrap();
    }
    let d = format!("wpkh({}/<0;1>/*)", text_of(&app, 0, 0));
    app.session.add_wallet("Ready", &d, "test").unwrap();
    // The second seed leaves: its twelve wallets are watch-only now.
    app.session.keys.remove(1);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(
        app.offers(Action::OpenWallet(12)),
        "the wallet that can sign is cut off"
    );
    assert!(app.offers(Action::Nav(Screen::Wallets)), "no '+ more'");
}
