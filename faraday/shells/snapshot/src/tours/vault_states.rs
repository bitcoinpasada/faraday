//! The vault states tour: what Unlock and the vault's head say of a
//! vault that is not simply Faraday's own and fresh.

use crate::tour::{Tour, scroll, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, Sheet, StorageEvent};
use faraday_shell_api::App;
use faraday_vault::kdbx::xml::{Document, Entry};
use faraday_vault::records::{Record, field, kind};

/// The small panel's keyboard put away, so the form under it is seen;
/// nothing where the window has none up.
fn hide_keyboard(t: &mut Tour) {
    // A frame first, so a field focused since the last one does not bring it back.
    let _ = t.app.frame();
    t.press(Action::Osk(faraday_core::OskPress::Hide));
}

/// The passphrase of the vectors other programs saved.
const FOREIGN: &str = "kdbx vector passphrase";

macro_rules! vector {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/vectors/kdbx/",
            $name
        ))
        .to_vec()
    };
}

/// A fresh process holding only `files`, at the Light cost.
fn only(t: &mut Tour, files: Vec<(String, Vec<u8>)>, kept: Vec<(String, Vec<u8>)>) {
    t.restart();
    t.app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept,
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
}

/// Open the first vault under `phrase` and wait for the answer.
fn unlock(t: &mut Tour, phrase: &str) {
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, phrase);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..30 {
        t.tick();
        if t.app.sheet == Some(Sheet::OlderCopy)
            || !t.app.vaults().open.is_empty()
            || t.app.vaults().unlock_error.is_some()
        {
            break;
        }
    }
}

/// What the lock asked to keep, as the tour's pump took it.
fn saved(t: &mut Tour) -> Result<Vec<(String, Vec<u8>)>, String> {
    let sheet = t.app.sheet;
    t.boxes
        .take()
        .map(|b| b.2)
        .ok_or_else(|| format!("the lock saved nothing (sheet {sheet:?})"))
}

/// A file whose empty document `edit` changes and whose XML `tweak`
/// rewrites.
fn edited(
    id: u8,
    edit: impl FnOnce(&mut Document),
    tweak: impl FnOnce(String) -> String,
) -> Result<Vec<u8>, String> {
    testkit::kdbx_edited("Shared", testkit::VAULT_PASSPHRASE, id, edit, tweak)
}

/// The page of the vault `file` (unlocked) once its Load sheet is put away.
fn contents_of(t: &mut Tour, file: Vec<u8>, name: &str, shot: &str) -> Result<(), String> {
    only(t, vec![(name.into(), file)], Vec::new());
    unlock(t, testkit::VAULT_PASSPHRASE);
    crate::tour::not_now_load(t);
    t.press(Action::Nav(Screen::VaultContents));
    t.shot(shot)
}

/// The vault open to read only, an item not read, and Create vault's file
/// name.
fn unreadable_and_read_only(t: &mut Tour) -> Result<(), String> {
    // Read to only, for each reason.
    let newer = edited(
        21,
        |_| {},
        |xml| {
            let at = xml.find("faraday.format").unwrap_or(0);
            let one = "<Value>1</Value>";
            match xml[at..].find(one) {
                Some(v) => format!(
                    "{}<Value>99</Value>{}",
                    &xml[..at + v],
                    &xml[at + v + one.len()..]
                ),
                None => xml,
            }
        },
    )?;
    contents_of(t, newer, "newer.kdbx", "vault-read-only-newer")?;
    let none = "<RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID>";
    let odd = edited(
        22,
        |_| {},
        |xml| xml.replace(none, &format!("{none}{none}")),
    )?;
    contents_of(t, odd, "odd.kdbx", "vault-read-only-settings")?;

    // An item this build could not read.
    let damaged = edited(
        23,
        |doc| {
            let mut e = Entry::new([60; 16]);
            let _ = e.set_field("Title", "Old exchange login", false);
            let _ = e.custom_mut().set("faraday.kind", "rocket");
            let _ = doc.root_mut().push_entry(e);
        },
        |xml| xml,
    )?;
    contents_of(t, damaged, "damaged.kdbx", "vault-not-read")?;
    // Scrolled to the section itself.
    crate::tour::scroll(t, 3000);
    t.shot("vault-not-read-section")?;

    // Create vault names its file with `.kdbx`.
    only(t, Vec::new(), Vec::new());
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    t.press(Action::Vault(V::CName));
    type_text(t, "Family");
    crate::tour::scroll(t, 90);
    t.shot("create-vault-file-name")?;
    Ok(())
}

/// A vault that also needs a key file and a YubiKey's secret.
fn keyed_unlock(t: &mut Tour) -> Result<(), String> {
    // A vault that also needs a key file and a YubiKey's secret: the two
    // buttons, the dialog, the chip and the field with a refusal, and what
    // a wrong try says.
    let keyed = testkit::kdbx_vault_keyed(
        "Keyed",
        testkit::VAULT_PASSPHRASE,
        3,
        (Some([7; 32]), Some([0xab; 20])),
    )?;
    only(
        t,
        vec![
            ("keyed.kdbx".into(), keyed),
            ("backup.keyx".into(), vec![7; 32]),
            ("notes.txt".into(), b"Notes".to_vec()),
        ],
        Vec::new(),
    );
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    t.shot("unlock-key-buttons")?;
    t.press(Action::Vault(V::KeyFileAsk));
    t.shot("unlock-key-file-dialog")?;
    t.press(Action::Vault(V::KeyFileChoose));
    t.press(Action::Vault(V::YubiKeyAdd));
    type_text(t, "abcd");
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASE);
    t.press(Action::Vault(V::Unlock));
    hide_keyboard(t);
    scroll(t, 300);
    t.shot("unlock-key-file-secret-refused")?;
    t.press(Action::Vault(V::FocusYubiKey));
    for _ in 0..4 {
        t.app.event(faraday_shell_api::Event::Key(
            faraday_shell_api::Key::Backspace,
        ));
    }
    type_text(t, "0000000000000000000000000000000000000000");
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..30 {
        t.tick();
        if t.app.vaults().unlock_error.is_some() {
            break;
        }
    }
    hide_keyboard(t);
    t.shot("unlock-key-file-secret-wrong")?;
    Ok(())
}

/// A wrong passphrase stays on Unlock and says so.
fn wrong_passphrase(t: &mut Tour, own: &[u8]) -> Result<(), String> {
    only(t, vec![("vault.kdbx".into(), own.to_vec())], Vec::new());
    unlock(t, "not the passphrase");
    if !t.app.vaults().open.is_empty() {
        return Err("a wrong passphrase opened the vault".into());
    }
    if t.app.vaults().unlock_error.is_none() {
        return Err("a wrong passphrase gave no answer".into());
    }
    t.shot("unlock-wrong-passphrase")?;
    Ok(())
}

pub(crate) fn vault_states_tour(t: &mut Tour) -> Result<(), String> {
    let own = testkit::test_vault()?;
    wrong_passphrase(t, &own)?;

    keyed_unlock(t)?;

    // A file another program saved: no padding, other settings.
    only(
        t,
        vec![(
            "other.kdbx".into(),
            vector!("pykeepass-aes-argon2id-gzip.kdbx"),
        )],
        Vec::new(),
    );
    unlock(t, FOREIGN);
    crate::tour::not_now_load(t);
    t.press(Action::Nav(Screen::VaultContents));
    t.shot("vault-saved-by-another")?;

    // A file with a history opens as any other: nothing asks about it.
    let history = vector!("pykeepass-chacha20-argon2id-plain-history.kdbx");
    only(t, vec![("history.kdbx".into(), history)], Vec::new());
    unlock(t, FOREIGN);
    if t.app.sheet == Some(Sheet::OlderCopy) {
        return Err("a vault with a history stopped at Open this copy".into());
    }
    // A vault of Faraday's sealed here, then its copy from before opened
    // in the same power-on: the dialog says it is older.
    let fresh = testkit::kdbx_vault("Main", testkit::VAULT_PASSPHRASE, 1, Vec::new())?;
    only(t, vec![("vault.kdbx".into(), fresh.clone())], Vec::new());
    unlock(t, testkit::VAULT_PASSPHRASE);
    crate::tour::not_now_load(t);
    t.press(Action::Entry(None));
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    t.press(Action::EntryAdd);
    t.press(Action::Vault(V::SaveKey(0)));
    t.press(Action::Lock);
    testkit::settle(&mut t.app);
    t.pump();
    let kept = saved(t)?;
    only(t, vec![("vault.kdbx".into(), fresh.clone())], kept);
    unlock(t, testkit::VAULT_PASSPHRASE);
    // Not reached yet: the second power-on opens without the dialog, so
    // the shot is taken only when it stops there.
    if t.app.sheet == Some(Sheet::OlderCopy) {
        t.shot("open-this-copy-older")?;
    }
    t.press(Action::Vault(V::OlderOpen));

    unreadable_and_read_only(t)?;

    // Change passphrase and Memory on an open vault.
    let main = testkit::test_vault()?;
    only(t, vec![("vault.kdbx".into(), main)], Vec::new());
    unlock(t, testkit::VAULT_PASSPHRASE);
    crate::tour::not_now_load(t);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::ChangePassphrase));
    type_text(t, "a new passphrase");
    t.press(Action::Vault(V::FocusField(1)));
    type_text(t, "a new passphrase 2");
    t.shot("vault-change-passphrase")?;
    t.press(Action::Vault(V::FormCancel));
    t.press(Action::Vault(V::MemoryAsk));
    t.shot("vault-memory")?;
    t.press(Action::Vault(V::MemoryMachine(4)));
    crate::tour::scroll_to(t, Action::Vault(V::MemoryMachine(4)));
    t.shot("vault-memory-ticked")?;
    t.press(Action::Vault(V::FormCancel));

    // A vault changed past its size: the Lock sheet says it cannot be sealed.
    only(t, vec![("vault.kdbx".into(), own)], Vec::new());
    unlock(t, testkit::VAULT_PASSPHRASE);
    crate::tour::not_now_load(t);
    t.press(Action::Entry(None));
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    t.press(Action::EntryAdd);
    t.press(Action::Vault(V::SaveKey(0)));
    let id = t.app.vaults().open[0].id;
    let used = t.app.vaults().records(id).used(&testkit::compute());
    let slot = t.app.vaults().open[0]
        .facts()
        .size
        .ok_or("the test vault is not a Faraday seal")?;
    let filler = Record::new(kind::ENTRY)
        .with(field::TITLE, b"filler")
        .with(field::NOTES, "x".repeat(slot - used + 1_000).as_bytes());
    testkit::vault_edit(&mut t.app, id, |s| s.insert(id, filler));
    let before = (t.app.vaults().open.len(), t.app.sheet);
    t.press(Action::LockAsk);
    if t.app.sheet != Some(Sheet::LockAsk) {
        return Err(format!(
            "no lock sheet: before {before:?}, sheet {:?}, open {}, changes {:?}, error {:?}",
            t.app.sheet,
            t.app.vaults().open.len(),
            t.app.vaults().open.first().map(|o| o.changes),
            t.app.vaults().unlock_error
        ));
    }
    t.shot("vault-lock-cannot-seal")?;

    // The next process after an idle lock that lost a vault's changes.
    let kept = vec![
        ("idle".to_string(), b"1800000".to_vec()),
        ("idle-unsaved".to_string(), b"1".to_vec()),
    ];
    only(t, Vec::new(), kept);
    t.shot("locked-changes-lost")?;
    Ok(())
}
