//! The QR vault tool in Tools (`docs/VAULT.md` §13): its three cards,
//! Contents, Passphrase and Code, and the Open card of the code made.

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::qr_tool::{Field, Pick, QrToolAction as Q};
use faraday_core::{Action, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};

pub(crate) fn qr_tool_tour(t: &mut Tour) -> Result<(), String> {
    for i in 0..8u8 {
        t.app
            .event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "QR vault")
        .ok_or("no QR vault tile")?;
    t.press(Action::Catalog(tile as u8));
    t.shot("qr-tool-contents")?;
    t.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_text(t, "the safe is behind the books");
    t.shot("qr-tool-contents-typed")?;
    t.press(Action::QrTool(Q::Next));
    type_text(t, "a passphrase");
    t.press(Action::QrTool(Q::Focus(Field::Again)));
    type_text(t, "a passphrase");
    t.press(Action::QrTool(Q::Focus(Field::Name)));
    type_text(t, "Spare");
    t.shot("qr-tool-passphrase")?;
    t.press(Action::QrTool(Q::Make));
    testkit::settle(&mut t.app);
    t.tick();
    scroll_to(t, Action::QrTool(Q::Another));
    t.shot("qr-tool-code")?;
    t.press(Action::QrTool(Q::File));
    let code = t
        .app
        .outbox
        .first()
        .map(|i| i.bytes.clone())
        .ok_or("no code in the Outbox")?;
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("qr-vault.pgp".to_string(), code.to_vec())]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.press(Action::Nav(Screen::Files));
    t.press(Action::QrTool(Q::OpenFile(0)));
    t.shot("qr-tool-open")?;
    type_text(t, "a passphrase");
    t.shot("qr-tool-open-typed")?;
    t.press(Action::QrTool(Q::OpenCode));
    testkit::settle(&mut t.app);
    t.tick();
    t.shot("qr-tool-opened")?;
    t.press(Action::QrTool(Q::Eye));
    t.shot("qr-tool-opened-shown")?;
    // A passphrase vault opened: its key is not loaded, then it is.
    const PASSPHRASE_CODE: &[u8] =
        include_bytes!("../../../../../tools/vectors/vault/qr/qr-passphrase.pgp");
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "passphrase.pgp".to_string(),
            PASSPHRASE_CODE.to_vec(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.press(Action::Nav(Screen::Files));
    t.press(Action::QrTool(Q::OpenFile(0)));
    type_text(t, "correct horse battery staple");
    t.press(Action::QrTool(Q::OpenCode));
    testkit::settle(&mut t.app);
    t.tick();
    t.shot("qr-tool-opened-pass-not-loaded")?;
    let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    t.app
        .session
        .keys
        .add_words(&testkit::compute(), words, "Twelve", None)
        .map_err(|e| format!("{e:?}"))?;
    t.press(Action::QrTool(Q::OpenCode));
    t.press(Action::Nav(Screen::Files));
    t.press(Action::QrTool(Q::OpenFile(0)));
    type_text(t, "correct horse battery staple");
    t.press(Action::QrTool(Q::OpenCode));
    testkit::settle(&mut t.app);
    t.tick();
    t.shot("qr-tool-opened-pass-add")?;
    save_item(t)
}

/// Save as a QR vault on a note in an open vault: the item's page, the
/// tool at its Passphrase step, and the dialog for a note too long.
fn save_item(t: &mut Tour) -> Result<(), String> {
    use faraday_core::vault::VaultAction as V;
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let note = |text: &str| Record::new(kind::NOTE).with(field::NOTE, text.as_bytes());
    let pass = testkit::VAULT_PASSPHRASE;
    let file = testkit::kdbx_vault(
        "Alpha",
        pass,
        1,
        vec![note("the safe is behind the books"), note(&"a".repeat(220))],
    )?;
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("a.kdbx".into(), file)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, pass);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..40 {
        t.tick();
        if !t.app.vaults().open.is_empty() {
            break;
        }
    }
    crate::tour::not_now_load(t);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(3)));
    t.press(Action::Vault(V::Item(0)));
    t.shot("qr-save-item-page")?;
    t.press(Action::Vault(V::QrSave));
    t.shot("qr-save-item-make")?;
    t.press(Action::Back);
    t.press(Action::Back);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(3)));
    t.press(Action::Vault(V::Item(1)));
    t.press(Action::Vault(V::QrSave));
    t.shot("qr-save-item-too-long")
}
