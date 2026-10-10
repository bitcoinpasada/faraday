//! A vault is named when it is made, and its file takes the name; an
//! open vault is locked or looked into from the list and from the
//! sidebar; a vault, larger than any QR transfer, is not offered as one.

use faraday_core::vaults::VaultAction as V;
use faraday_core::vaults::vstep;
use faraday_core::{Action, Faraday, Screen, testkit};
use osk_shell_api::{App, EntropyBytes, Event, Key};

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn settle(app: &mut Faraday) {
    for t in 1..20 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn started() -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app
}

/// Makes a vault from the Vaults tab, named `name` when it is not empty,
/// under the passphrase `test phrase`.
fn make(app: &mut Faraday, name: &str) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for step in [vstep::WHERE, vstep::COST, vstep::SIZE] {
        app.press(Action::Vault(V::CNext(step)));
    }
    if !name.is_empty() {
        app.press(Action::Vault(V::CName));
        type_text(app, name);
    }
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CGo));
    settle(app);
    assert_eq!(app.screen, Screen::Vaults, "the vault was made");
}

fn outbox_names(app: &Faraday) -> Vec<String> {
    app.outbox.iter().map(|i| i.name.clone()).collect()
}

#[test]
fn a_named_vaults_file_takes_its_name() {
    let mut app = started();
    make(&mut app, "Family savings!");
    assert_eq!(outbox_names(&app), ["Family-savings.ofv"]);
}

#[test]
fn an_unnamed_vault_is_vault_ofv_and_a_second_of_one_name_is_numbered() {
    let mut app = started();
    make(&mut app, "");
    make(&mut app, "");
    make(&mut app, "spare");
    make(&mut app, "spare");
    assert_eq!(
        outbox_names(&app),
        ["vault.ofv", "vault-2.ofv", "spare.ofv", "spare-2.ofv"]
    );
}

#[test]
fn a_vault_is_not_offered_as_a_qr_code() {
    let mut app = started();
    make(&mut app, "");
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(!app.offers(Action::QrOutbox(0)));
    app.press(Action::QrOutbox(0));
    assert_eq!(app.sheet, None);
    assert_eq!(app.toast_text(), None, "and no error either");
}

#[test]
fn an_open_vault_is_locked_or_looked_into_from_the_list_and_the_sidebar() {
    let mut app = started();
    make(&mut app, "home");
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app);
    assert_eq!(app.vaults.open.len(), 1);
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::Open(0))), "its contents");
    assert!(app.offers(Action::LockAsk), "and Lock");
    // The sidebar's Vaults row carries "1 open" and opens Vaults, from
    // where its contents are reached (`docs/SIMPLIFY.md` §1.6, revised).
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(app.offers(Action::Nav(Screen::Vaults)));
    app.press(Action::Nav(Screen::Vaults));
    assert_eq!(app.screen, Screen::Vaults);
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::Open(0))), "its contents, again");
}
