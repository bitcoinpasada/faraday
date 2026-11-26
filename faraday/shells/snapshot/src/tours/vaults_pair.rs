//! The Vaults pair tour: the session pane's marks and Save beside the vault
//! files, with none, one, a read-only and two vaults open, and with many
//! vault files (`docs/WORKSPACE.md` §2).

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};

const PHRASE: &str = "pair phrase";

fn files(t: &mut Tour, list: Vec<(String, Vec<u8>)>) {
    t.restart();
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(list),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(100));
}

fn held(c: &faraday_crypto::work::Compute, t: &mut Tour, long: bool) -> Result<(), String> {
    let name = if long {
        "A key with a very long name that does not fit its row"
    } else {
        "Bacon"
    };
    let keys = &mut t.app.session.keys;
    keys.add_words(
        c,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
        name,
        None,
    )
    .map_err(|e| format!("{e:?}"))?;
    keys.add_words(
        c,
        &testkit::test_words(testkit::TEST_SEEDS[1].0),
        "Second",
        None,
    )
    .map_err(|e| format!("{e:?}"))?;
    for kit in testkit::kits().into_iter().take(2) {
        t.app
            .session
            .add_wallet_confirmed(
                c,
                kit.name,
                &kit.descriptor,
                "Wallet",
                faraday_core::wallet::Origin::Named,
            )
            .map_err(|e| format!("{e:?}"))?;
    }
    Ok(())
}

fn open(t: &mut Tour, i: usize) -> Result<(), String> {
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(i)));
    type_text(t, PHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..40 {
        t.tick();
        if t.app.vaults().open.len() > i {
            return Ok(());
        }
    }
    Err(format!("vault {i} did not open"))
}

fn settle(t: &mut Tour) {
    for _ in 0..40 {
        t.tick();
    }
}

fn vault(id: u8) -> Result<(String, Vec<u8>), String> {
    let f = testkit::kdbx_vault("Main", PHRASE, id, Vec::new()).map_err(|e| String::from(&*e))?;
    Ok((format!("vault-{id}.kdbx"), f))
}

pub(crate) fn vaults_pair_tour(
    c: &faraday_crypto::work::Compute,
    t: &mut Tour,
) -> Result<(), String> {
    // No vault file.
    files(t, Vec::new());
    held(c, t, false)?;
    t.press(Action::Nav(Screen::Vaults));
    t.shot("none-file")?;
    // One vault open: the first key and wallet saved, the others not.
    files(t, vec![vault(1)?]);
    held(c, t, true)?;
    open(t, 0)?;
    t.press(Action::Vault(V::SaveKey(0)));
    settle(t);
    t.press(Action::Vault(V::SaveWallet(0)));
    settle(t);
    t.press(Action::Nav(Screen::Vaults));
    t.shot("one-open")?;
    // Two open.
    files(t, vec![vault(1)?, vault(2)?]);
    held(c, t, false)?;
    open(t, 0)?;
    open(t, 1)?;
    t.press(Action::Nav(Screen::Vaults));
    t.shot("two-open")?;
    // A read-only vault.
    let newer = testkit::kdbx_edited(
        "Newer",
        PHRASE,
        9,
        |_| {},
        |xml| {
            let at = xml.find("faraday.format").unwrap_or(0);
            let v = at + xml[at..].find("<Value>1</Value>").unwrap_or(0);
            format!(
                "{}<Value>99</Value>{}",
                &xml[..v],
                &xml[(v + "<Value>1</Value>".len()).min(xml.len())..]
            )
        },
    )
    .map_err(|e| String::from(&*e))?;
    files(t, vec![("newer.kdbx".to_string(), newer)]);
    held(c, t, false)?;
    open(t, 0)?;
    t.press(Action::Nav(Screen::Vaults));
    t.shot("read-only")?;
    // Many vault files, none open.
    let mut many = Vec::new();
    for id in 1..=14u8 {
        many.push(vault(id)?);
    }
    files(t, many);
    held(c, t, false)?;
    t.press(Action::Nav(Screen::Vaults));
    t.shot("many-files")?;
    Ok(())
}
