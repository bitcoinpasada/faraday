//! Load loads the wallets ticked in the Inbox and not the ones unticked;
//! a potential wallet made at once with the passphrase of a key already
//! loaded is that key's wallet, not the bare seed's.

use crate::common::type_text;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
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
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn two_wallets() -> Vec<(String, Vec<u8>)> {
    testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| {
            n == "spending-wallet.txt" || (n.starts_with("savings-") && !n.contains("psbt"))
        })
        .collect()
}

#[test]
fn load_loads_only_the_wallets_left_ticked() {
    for unticked in 0..2 {
        let c = testkit::compute();
        let mut app = with_inbox(two_wallets());
        let found = app.inbox_found_now(&c);
        assert_eq!(found.to_load().len(), 2, "two wallets in the Inbox");
        let left_keys = found.to_load()[1 - unticked].keys.len();
        app.press(Action::InboxChoose(unticked));
        app.press(Action::InboxLoad);
        assert_eq!(app.session.wallets.len(), 1, "unticked {unticked}");
        assert_eq!(
            app.session.wallets[0].policy().keys().len(),
            left_keys,
            "the wallet loaded is the one left ticked"
        );
    }
}

#[test]
fn continue_at_once_with_a_loaded_keys_passphrase_makes_that_keys_wallet() {
    let c = testkit::compute();
    let words = testkit::test_words("summer");
    let mut app = with_inbox(vec![(
        "summer-words.txt".to_string(),
        format!("{words}\n").into_bytes(),
    )]);
    let bare = app.inbox_found_now(&c).potential()[0].fingerprint;
    // The key with the passphrase is already loaded.
    let loaded = app
        .session
        .keys
        .add_words_with(&c, &words, "TREZOR", "summer", None)
        .unwrap();
    assert_ne!(loaded, bare);

    app.press(Action::PotentialOpen(bare.0));
    app.press(Action::PotentialType);
    type_text(&mut app, "TREZOR");
    // No pause: the fingerprint of the passphrase is not shown yet.
    assert!(app.potential().unwrap().with.is_none());
    app.press(Action::PotentialKind(1));
    app.press(Action::PotentialMake);

    assert_eq!(app.session.keys.len(), 1, "a key was added");
    assert_eq!(app.session.wallets.len(), 1, "no wallet was made");
    let fp = format!(
        "{:02x}{:02x}{:02x}{:02x}",
        loaded.0[0], loaded.0[1], loaded.0[2], loaded.0[3]
    );
    assert!(
        app.session.wallets[0]
            .policy()
            .to_descriptor()
            .contains(&fp),
        "the wallet is not the passphrase key's"
    );
}
