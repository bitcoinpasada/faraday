//! The files_wide tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};

/// Files on a wide window with no stick in, the Inbox holding a PSBT, a
/// wallet file and a vault, the vault open and files in the Outbox: In
/// Faraday with the Inbox summary, the vault row and the Outbox groups, On
/// the stick waiting for one; then a stick in while the open vault holds a
/// secret, where it says to lock.
pub(crate) fn files_wide_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::wallet::FileKind as K;
    let files = testkit::files().map_err(|e| String::from(&*e))?;
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    let item = |name: &str, kind: K, len: usize| faraday_core::Item {
        name: faraday_text::Text::of(name),
        bytes: vec![b'#'; len].into(),
        kind,
        secret: false,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    };
    for i in [
        item("family-savings-descriptor.txt", K::Wallet, 620),
        item("family-savings-wallet.json", K::Wallet, 1900),
        item("family-savings-backup.pdf", K::Other, 48_000),
    ] {
        t.app.outbox.push(i);
    }
    t.app.outbox.push(faraday_core::Item {
        secret: true,
        ..item("seed-words.txt", K::Words, 160)
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASE);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    crate::tour::not_now_load(t);
    t.press(Action::Nav(Screen::Files));
    t.shot("files-no-stick")?;
    if t.app.offers(Action::InboxSummary) {
        t.press(Action::InboxSummary);
        t.shot("files-summary-open")?;
        t.press(Action::InboxSummary);
    }
    t.sticks(true);
    // The stick's own sheet, put off: Files shows the stick waiting on Lock.
    if t.app.sheet.is_some() {
        t.press(Action::Cancel);
    }
    t.press(Action::Nav(Screen::Files));
    t.shot("files-stick-locked")
}
