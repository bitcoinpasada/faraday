//! A stick plugged in while a vault is open (`docs/SECURITY-MODEL.md`):
//! after **Not now**, with the stick still attached, the vault's
//! passphrase and entry fields take no typing, an entry is not opened to
//! edit, and no key is loaded, by **Load**, by **Load with a passphrase**
//! or otherwise; each says why. Pulled, they work again.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const STICK_LINE: &str = "Keys load only with no stick attached";
const ENTRY_LINE: &str = "Fields of a vault entry take no typing while a stick is attached";

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

/// The test vault unlocked from a flow's way in, so nothing in it is
/// loaded, on its contents.
fn opened() -> Faraday {
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
    for i in 0..6u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x60 + i; 32],
        )));
    }
    app.storage(StorageEvent::Restored {
        inbox: vec![kit_file("vault.fvlt")],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault did not unlock");
    app.press(Action::Nav(Screen::VaultContents));
    app
}

fn stick() -> Vec<StickInfo> {
    vec![StickInfo {
        serial: None,
        id: "a".into(),
        label: "STICK".into(),
        boot: false,
        files: Vec::new(),
    }]
}

/// A stick plugged in, and Not now on the sheet it brings.
fn attach(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(stick()));
    if app.sheet == Some(Sheet::Lock) {
        app.press(Action::NotNow);
    }
    assert!(!app.sticks().is_empty());
}

fn said_stick(app: &Faraday) -> bool {
    app.toast_text().is_some_and(|t| t == STICK_LINE)
}

fn said_entry(app: &Faraday) -> bool {
    app.toast_text().is_some_and(|t| t == ENTRY_LINE)
}

fn prompt_text(app: &Faraday) -> String {
    app.vaults()
        .prompt
        .as_ref()
        .map(|p| p.text.text.to_string())
        .unwrap_or_default()
}

/// The vault's key shown, a stick attached.
fn on_key_with_stick() -> Faraday {
    let mut app = opened();
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
    attach(&mut app);
    app
}

#[test]
fn with_a_stick_attached_load_with_a_passphrase_takes_no_typing_and_loads_nothing() {
    let mut app = on_key_with_stick();
    let before = app.session.keys.len();
    app.press(Action::Vault(V::LoadWithPassphrase));
    assert!(said_stick(&app), "{:?}", app.toast_text());
    type_text(&mut app, "typed with a stick in");
    assert_eq!(prompt_text(&app), "", "the passphrase field took typing");
    app.press(Action::Vault(V::FocusPrompt));
    type_text(&mut app, "typed with a stick in");
    assert_eq!(prompt_text(&app), "", "the passphrase field took typing");
    app.press(Action::Vault(V::PromptGo));
    assert_eq!(
        app.session.keys.len(),
        before,
        "a key loaded with a stick in"
    );
}

#[test]
fn with_a_stick_attached_load_loads_no_key() {
    let mut app = on_key_with_stick();
    let before = app.session.keys.len();
    app.press(Action::Vault(V::Load));
    assert_eq!(
        app.session.keys.len(),
        before,
        "a key loaded with a stick in"
    );
    assert!(said_stick(&app), "{:?}", app.toast_text());
}

#[test]
fn with_a_stick_attached_an_entrys_fields_take_no_typing() {
    let mut app = opened();
    app.press(Action::Vault(V::Category(2)));
    attach(&mut app);
    app.press(Action::Vault(V::Add));
    type_text(&mut app, "Mail");
    app.press(Action::Vault(V::FocusField(2)));
    assert!(said_entry(&app), "{:?}", app.toast_text());
    type_text(&mut app, "hunter22");
    let form = app.vaults().form.as_ref().expect("the entry's form");
    assert!(
        form.fields.iter().all(|f| f.2.text.is_empty()),
        "an entry's field took typing with a stick in"
    );
}

#[test]
fn with_a_stick_attached_the_keepass_passphrase_takes_no_typing() {
    let mut app = opened();
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Mail");
    app.press(Action::Vault(V::FormSave));
    app.press(Action::Vault(V::Item(0)));
    attach(&mut app);
    app.press(Action::Vault(V::ExportKdbx));
    type_text(&mut app, "keepass passphrase");
    assert_eq!(prompt_text(&app), "", "the KeePass passphrase took typing");
}

#[test]
fn with_a_stick_attached_an_entry_is_not_opened_to_edit() {
    let mut app = opened();
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Mail");
    app.press(Action::Vault(V::FocusField(2)));
    type_text(&mut app, "hunter22");
    app.press(Action::Vault(V::FormSave));
    app.press(Action::Vault(V::Item(0)));
    attach(&mut app);
    app.press(Action::Vault(V::Edit));
    assert!(said_entry(&app), "{:?}", app.toast_text());
    assert!(
        app.vaults().form.is_none(),
        "the entry's password was put in a form with a stick in"
    );
}

#[test]
fn pulled_the_fields_take_typing_again() {
    let mut app = on_key_with_stick();
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Vault(V::LoadWithPassphrase));
    type_text(&mut app, "now");
    assert_eq!(prompt_text(&app), "now");
}
