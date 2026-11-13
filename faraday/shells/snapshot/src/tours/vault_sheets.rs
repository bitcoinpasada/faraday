//! The vault sheets tour: a seed with a codex32 set and a BIP-85 origin
//! copied and moved to another vault, kept or left out at a save, and
//! trimmed (`docs/VAULT.md` §7).

use crate::tour::{Tour, not_now_load, type_text};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent, bstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};
use faraday_vault::Record;
use faraday_vault::child::{ChildOf, payload as child_payload};
use faraday_vault::codex32::{SetFacts, State, set_payload};
use faraday_vault::kdbx::xml::Entry;
use faraday_vault::records::{field, kind};

const KEYS: usize = 0;
const ENTRIES: usize = 2;

fn set_facts() -> SetFacts {
    SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 1,
        strings: vec![(b'a', State::Placed), (b'c', State::Made)],
    }
}

fn origin() -> ChildOf {
    ChildOf {
        parent: [1, 2, 3, 4],
        language: 0,
        words: 24,
        index: 7,
    }
}

fn seed() -> Result<Record, String> {
    let c = testkit::compute();
    let words = Mnemonic::parse(&c, Language::English, &testkit::test_words("summer"))
        .map_err(|e| format!("{e:?}"))?;
    let mut r = Record::new(kind::KEY)
        .with(field::KEY, &faraday_vault::records::words_payload(&words))
        .with(field::KEY_LABEL, b"Savings");
    r.push(
        field::KEY_CODEX32,
        &set_payload(&set_facts()).ok_or("the set did not encode")?,
    );
    r.push(
        field::KEY_CHILD,
        &child_payload(&origin()).ok_or("the origin did not encode")?,
    );
    Ok(r)
}

fn entry(title: &str) -> Record {
    Record::new(kind::ENTRY)
        .with(field::TITLE, title.as_bytes())
        .with(field::PASSWORD, b"hunter2")
}

/// The vault `at` of the Vaults list opened.
fn unlock(t: &mut Tour, at: usize) -> Result<(), String> {
    let before = t.app.vaults().open.len();
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(at)));
    type_text(t, testkit::VAULT_PASSPHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..40 {
        t.tick();
        if t.app.vaults().open.len() > before {
            break;
        }
    }
    not_now_load(t);
    if t.app.vaults().open.len() == before + 1 {
        Ok(())
    } else {
        Err(format!("vault {at} did not open"))
    }
}

fn item(t: &mut Tour, from: usize, category: usize, at: usize) {
    testkit::vault_current(&mut t.app, from);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(category)));
    t.press(Action::Vault(V::Item(at)));
}

/// Costs a map of the plan (through `seed_files_at`). The four vaults
/// of the tour open, one of them locked again.
fn setup(t: &mut Tour) -> Result<(), String> {
    let pass = testkit::VAULT_PASSPHRASE;
    let alpha = testkit::kdbx_vault("Alpha", pass, 1, vec![seed()?, entry("Mail")])
        .map_err(|e| String::from(&*e))?;
    let beta = testkit::kdbx_vault("Beta", pass, 2, Vec::new()).map_err(|e| String::from(&*e))?;
    let newer = testkit::kdbx_edited(
        "Newer",
        pass,
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
    )
    .map_err(|e| String::from(&*e))?;
    let locked =
        testkit::kdbx_vault("Gamma", pass, 3, Vec::new()).map_err(|e| String::from(&*e))?;
    let tagged = testkit::kdbx_edited(
        "Saved",
        pass,
        7,
        |doc| {
            let mut e = Entry::new([3; 16]);
            e.set_field("Title", "Kept", false).unwrap();
            doc.root_mut().push_entry(e).unwrap();
        },
        |xml| {
            let at = xml.find("</Entry>").unwrap_or(0);
            format!("{}<Tags>x</Tags>{}", &xml[..at], &xml[at..])
        },
    )
    .map_err(|e| String::from(&*e))?;
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("a.kdbx".into(), alpha),
            ("b.kdbx".into(), beta),
            ("c.kdbx".into(), newer),
            ("d.kdbx".into(), locked),
            ("e.kdbx".into(), tagged),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(100));
    for at in 0..5 {
        unlock(t, at)?;
    }
    // Gamma locked again, so it is known and locked.
    testkit::vault_lock_one(&mut t.app, 3);
    for _ in 0..20 {
        t.tick();
    }
    if t.app.vaults().open.len() != 4 {
        return Err("Gamma was not locked".into());
    }
    // The locked vault listed, as its row on the sheet comes from there.
    t.press(Action::Nav(Screen::Vaults));
    for _ in 0..40 {
        t.tick();
    }
    Ok(())
}

/// Costs a map of the plan (through `seed_files_at`). The Copy to another
/// vault sheet from a seed with a set and an origin (an open vault, a
/// read only one, a locked one, a new one), Move's dialog with a tick
/// off, Copy's dialog for an entry KeePass saved with data left behind,
/// the seed's page with its two Remove buttons and their dialog, the
/// Keep sheet of a seed saved from the Keys page, and the secret sheet
/// with its ticks.
pub(crate) fn vault_sheets_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    setup(t)?;
    // The seed's page, with both Remove buttons.
    item(t, 0, KEYS, 0);
    t.shot("vault-seed-page")?;
    // The page's own pane: on a wide panel the seed's detail is the right one.
    let (w, h) = t.size();
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: w * 4 / 5,
        y: h / 2,
        dy: 1200,
    });
    t.shot("vault-seed-page-foot")?;
    t.press(Action::Vault(V::TrimAsk(field::KEY_CODEX32)));
    t.shot("vault-trim-dialog")?;
    t.press(Action::Vault(V::TrimCancel));
    // Copy and Move.
    t.press(Action::Vault(V::CopyTo));
    t.shot("vault-copy-sheet")?;
    crate::tour::scroll(t, 240);
    t.shot("vault-copy-sheet-down")?;
    crate::tour::scroll(t, 400);
    t.shot("vault-copy-sheet-foot")?;
    t.press(Action::Vault(V::CopyKeep(0)));
    t.shot("vault-copy-sheet-one-off")?;
    t.press(Action::Vault(V::CopyCancel));
    t.press(Action::Vault(V::MoveTo));
    t.shot("vault-move-sheet")?;
    t.press(Action::Vault(V::CopyKeep(1)));
    t.press(Action::Vault(V::CopyPick(0)));
    t.shot("vault-move-dialog")?;
    t.press(Action::Vault(V::CopyCancel));
    // The entry KeePass saved with data this build does not read.
    item(t, 3, ENTRIES, 0);
    t.press(Action::Vault(V::CopyTo));
    t.press(Action::Vault(V::CopyPick(0)));
    t.shot("vault-copy-unread-dialog")?;
    t.press(Action::Vault(V::CopyCancel));
    keep_sheet(t)?;
    secret_sheet(c, t)?;
    more_menu(t)
}

/// The open vault's ⋮ (`docs/VAULT.md` §3.2): the menu, Key file, Remove
/// YubiKey secret and Keep where a removal waits, with the head line, for
/// a vault opened with a key file and a YubiKey's secret.
fn more_menu(t: &mut Tour) -> Result<(), String> {
    let keyed = testkit::kdbx_vault_keyed(
        "Keyed",
        testkit::VAULT_PASSPHRASE,
        3,
        (Some([7; 32]), Some([0xab; 20])),
    )
    .map_err(|e| String::from(&*e))?;
    t.restart();
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("keyed.kdbx".into(), keyed),
            ("backup.keyx".into(), vec![7; 32]),
            ("spare.keyx".into(), vec![9; 32]),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(100));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    t.press(Action::Vault(V::KeyFileAsk));
    t.press(Action::Vault(V::KeyFileChoose));
    t.press(Action::Vault(V::YubiKeyAdd));
    type_text(t, "abababababababababababababababababababab");
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..40 {
        t.tick();
        if !t.app.vaults().open.is_empty() {
            break;
        }
    }
    not_now_load(t);
    if t.app.vaults().open.is_empty() {
        return Err("the keyed vault did not open".into());
    }
    t.press(Action::Nav(Screen::VaultContents));
    t.shot("vault-more-head")?;
    t.press(Action::Vault(V::MoreAsk));
    t.shot("vault-more-menu")?;
    t.press(Action::Vault(V::VKeyAsk));
    t.shot("vault-more-key-file")?;
    t.press(Action::Vault(V::KeyFileCancel));
    t.press(Action::Vault(V::MoreAsk));
    t.press(Action::Vault(V::YubiKeyRemoveAsk));
    t.shot("vault-more-remove-yubikey")?;
    t.press(Action::Vault(V::TrimGo));
    t.shot("vault-more-removal-pending")?;
    t.press(Action::Vault(V::MoreAsk));
    t.press(Action::Vault(V::YubiKeyRemoveAsk));
    t.shot("vault-more-keep-yubikey")?;
    t.press(Action::Vault(V::TrimCancel));
    Ok(())
}

/// A key typed on Add a key, a child of a BIP-85 key with a codex32 set,
/// saved into the open Alpha from its page.
fn keep_sheet(t: &mut Tour) -> Result<(), String> {
    keep_sheet_as(t, "vault-keep-sheet")
}

/// The same, its picture named `name`.
fn keep_sheet_as(t: &mut Tour, name: &str) -> Result<(), String> {
    t.press(Action::Entry(None));
    t.type_key(1);
    let k = t.app.session.keys.len() - 1;
    t.app.session.keys.set_child_of(k, Some(origin()));
    t.app.session.keys.set_codex32(k, Some(set_facts()));
    testkit::vault_current(&mut t.app, 0);
    t.press(Action::Vault(V::SaveKey(k)));
    t.shot(name)?;
    t.press(Action::Vault(V::KeepCancel));
    Ok(())
}

/// Costs a map of the plan (through `seed_files_at`). The backup's secret
/// sheet for a seed with a set and an origin, the test vault open.
fn secret_sheet(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.restart();
    t.load_kit()?;
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::Backup(0));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    crate::tour::seed_files_at(c, t, 0);
    // Marked once the plan is made, as the plan does not take them in.
    let k = t.app.session.keys.len() - 1;
    t.app.session.keys.set_child_of(k, Some(origin()));
    t.app.session.keys.set_codex32(k, Some(set_facts()));
    let vault = testkit::files()
        .map_err(|e| String::from(&*e))?
        .into_iter()
        .find(|(n, _)| n == "vault.kdbx")
        .ok_or("no test vault")?;
    let mut inbox: Vec<(String, Vec<u8>)> = t
        .app
        .inbox()
        .iter()
        .map(|i| (i.name.clone(), i.bytes.to_vec()))
        .collect();
    inbox.push(vault);
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    unlock(t, 0)?;
    t.press(Action::Nav(Screen::Backup));
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::VAULT));
    t.press(Action::BKey(k));
    t.press(Action::BStep(bstep::FILES));
    t.press(Action::BFile);
    t.shot("vault-secret-sheet")?;
    crate::tour::scroll(t, 240);
    t.shot("vault-secret-sheet-down")?;
    crate::tour::scroll(t, 400);
    t.shot("vault-secret-sheet-foot")
}
