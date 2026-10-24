//! Make a new wallet's Single key, backup in shares (`docs/CODEX32.md`
//! §6.9): offered on Kind between Single key and Multisig, its Quorum
//! the codex32 scheme, its one slot opening Make codex32 shares with
//! that scheme fixed, and the wallet a single-key native SegWit one whose
//! Check names the set. Restore does not offer the kind.
//!
//! The tests marked "keygen hook" need the two lines in
//! `keygen/mod.rs` that hand New key the scheme and Create the set.

use faraday_core::codex32::{C32, Card, WAYS, Way};
use faraday_core::create::NewKind;
use faraday_core::keygen::kstep;
use faraday_core::{Action, Faraday, Screen, cstep};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
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

#[test]
fn the_kind_is_offered_on_kind_between_single_key_and_multisig() {
    let mut app = app();
    app.press(Action::CreateWallet);
    app.press(Action::CStep(cstep::KIND));
    let texts = app.drawn_texts();
    let at = |name: &str| texts.iter().position(|t| t == name);
    let (single, shares, multi) = (
        at("Single key · native SegWit").expect("Single key"),
        at("Single key, backup in shares").expect("the backup in shares, before More kinds"),
        at("Multisig · native SegWit").expect("Multisig"),
    );
    assert!(single < shares && shares < multi, "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t == "One key, its backup in 3 shares, any 2 rebuild it"),
        "its line follows the scheme: {texts:?}"
    );
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

/// Quorum at 3 of 5, Continue, No to vaults: Keys open.
fn on_keys(app: &mut Faraday) {
    app.press(Action::CM(1));
    app.press(Action::CN(1));
    app.press(Action::CN(1));
    app.press(Action::CNext(cstep::QUORUM));
    app.press(Action::CVaults(false));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
}

#[test]
fn its_slot_offers_only_make_codex32_shares() {
    let mut app = app();
    crate::common::add_key(&mut app, "bacon");
    let mut app = on_quorum_of(app);
    on_keys(&mut app);
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

/// Keygen hook: the slot opens Make codex32 shares on the identifier,
/// the Quorum's scheme fixed.
#[test]
#[ignore = "needs the keygen hook (keygen/mod.rs: keygen_open, keygen_codex32_lock)"]
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

/// Keygen hook: BIP 93's vector 2 typed in at 2 of 3 makes a native
/// SegWit wallet of its one key, whose Check names the set.
#[test]
#[ignore = "needs the keygen hook (keygen/mod.rs: keygen_open, keygen_codex32_lock)"]
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
    app.press(Action::CStep(cstep::CHECK));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "codex32 name · 2 of 3"),
        "Check's Backup row: {texts:?}"
    );
}
