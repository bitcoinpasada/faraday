//! The pass_seeds tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_crypto::work::Compute;

/// Costs what `parse`, `to_seed`, `from_seed` and the rest it calls
/// cost. The backup flow's Bring in the sticks with the boot stick
/// taken for Place 1 (`docs/BACKUP-FLOW.md` §16): it is Stick 1, the
/// Boot stick, and the next stick is Stick 2. A vault of the three test
/// keys, the third kept without the passphrase its 2-of-3 uses it with,
/// and a seed of no wallet (F62): the Load sheet, Seeds with no
/// passphrase stored, a passphrase giving another key, and the right
/// one.
pub(crate) fn pass_seeds_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_bip::bip39::{Language, Mnemonic};
    use faraday_bip::bitcoin::bip32::DerivationPath;
    use faraday_core::vault::VaultAction as V;
    use faraday_vault::records::{self, field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let words = |p: &str| Mnemonic::parse(c, Language::English, p).map_err(|e| format!("{e:?}"));
    let key = |m: &Mnemonic, label: &str| {
        Record::new(kind::KEY)
            .with(field::KEY, &records::words_payload(m))
            .with(field::KEY_LABEL, label.as_bytes())
    };
    let p = "m/48'/1'/0'/2'";
    let shown = testkit::on_net("m/48h/1h/0h/2h").replacen("m/", "", 1);
    let third = words(&testkit::test_words(testkit::TEST_SEEDS[2].0))?;
    let seed = third
        .to_seed(c, b"kept apart")
        .map_err(|e| format!("{e:?}"))?;
    let master = faraday_bip::keys::MasterKey::from_seed(c, &seed, testkit::net());
    let path: DerivationPath = testkit::on_net(p).parse().map_err(|e| format!("{e:?}"))?;
    let trio = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,[{}/{shown}]{}/<0;1>/*))",
        testkit::key(0, p),
        testkit::key(1, p),
        master.fingerprint(),
        master.derive(c, &path).to_xpub(c)
    );
    let mut records = vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Trio")];
    for n in 0..3 {
        let m = words(&testkit::test_words(testkit::TEST_SEEDS[n].0))?;
        records.push(key(&m, testkit::TEST_SEEDS[n].1));
    }
    records.push(key(
        &words("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")?,
        "Spare",
    ));
    records.push(
        Record::new(kind::WALLET)
            .with(field::WALLET, trio.as_bytes())
            .with(field::WALLET_NAME, b"Trio"),
    );
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 1,
        lanes: 1,
    };
    let file = faraday_vault::create(
        c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[b"trio phrase".as_slice()],
        &[Contents { records }],
        &[0x5e; 32],
    )
    .map_err(|e| format!("{e:?}"))?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![("trio.fvlt".into(), file)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, "trio phrase");
    t.press(Action::Vault(V::Unlock));
    t.tick();
    if t.app.sheet != Some(faraday_core::Sheet::UnlockLoad) {
        return Err(format!(
            "the trio vault did not ask what to load: {:?}",
            t.app.vaults().unlock_error
        ));
    }
    t.shot("passseeds-load-sheet")?;
    let keys: Vec<usize> = t
        .app
        .vault_load_view_now(c)
        .ok_or("no Load sheet")?
        .keys
        .iter()
        .filter(|k| k.ticked)
        .map(|k| k.record)
        .collect();
    for r in keys {
        t.press(Action::Vault(V::LoadTick(r)));
    }
    t.press(Action::Vault(V::LoadTicked));
    t.press(Action::OpenWallet(0));
    t.press(Action::Vault(V::PassphraseKey(0)));
    t.shot("passseeds-list")?;
    let pick = t
        .app
        .vault_passphrase_seeds_now(c)
        .iter()
        .find(|s| s.line.ends_with(testkit::TEST_SEEDS[2].1))
        .map(|s| (s.vault, s.record))
        .ok_or("test key 3 is not listed")?;
    t.press(Action::Vault(V::PassphrasePick(pick.0, pick.1)));
    type_text(t, "wrong");
    t.press(Action::Vault(V::PromptGo));
    t.shot("passseeds-mismatch")?;
    type_text(t, "kept apart");
    t.press(Action::Vault(V::PromptGo));
    if t.app.sheet.is_some() || t.app.session.keys.len() != 3 {
        return Err("the right passphrase did not load test key 3".into());
    }
    t.shot("passseeds-loaded")?;
    println!("passseeds: a seed kept without its passphrase, found by its passphrase");
    Ok(())
}
