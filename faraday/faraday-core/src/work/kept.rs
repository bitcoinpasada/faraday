//! What the app keeps worked out follows what it was worked out from: a
//! change to a vault's records, a plan's answers or a wallet's policy is
//! what the next frame shows and the next press reads (`docs/SPEED.md`
//! §3.3).

use faraday_crypto::work::Compute;
use faraday_shell_api::App;

use super::budget::{heavy, unlock_test_vault};
use crate::wallet::{Origin, Wallet, read_wallet};
use crate::{Action, Screen, testkit};

/// A wallet's shape, as the session names it.
fn shape_of(c: &Compute, descriptor: &str) -> String {
    crate::vault::screen::policy_shape(read_wallet(c, descriptor).expect("a test wallet"))
}

#[test]
fn a_wallet_record_changed_in_an_open_vault_shows_as_it_now_is() {
    let c = crate::testkit::compute();
    use faraday_vault::records::field;
    let (mut app, _) = heavy();
    unlock_test_vault(&mut app);
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    let v = app.vaults.current;
    let row = app
        .vault_rows_now(&c, v)
        .into_iter()
        .find(|r| !r.seed)
        .expect("the test vault keeps a wallet");
    let other = testkit::kits()
        .into_iter()
        .find(|k| shape_of(&c, &k.descriptor) != row.detail)
        .expect("a test wallet of another shape");
    app.vaults.open[v].contents.records[row.record].set(field::WALLET, other.descriptor.as_bytes());
    let _ = app.frame();
    let now = app
        .vault_rows_now(&c, v)
        .into_iter()
        .find(|r| r.record == row.record)
        .expect("the record is listed");
    assert_eq!(
        now.detail,
        shape_of(&c, &other.descriptor),
        "the vault's list shows the wallet the record now holds"
    );
}

#[test]
fn the_checklist_follows_a_plan_changed_since_the_last_frame() {
    let c = crate::testkit::compute();
    use crate::plan::{Answers, Preset};
    let (mut app, w) = heavy();
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, Preset::PaperVault);
    app.press(Action::BChecklist);
    let _ = app.frame();
    let shape = app.plan_shape_now(w);
    let before = app.backup_items_now(&c);
    let mut changed = false;
    for preset in Preset::ALL {
        let answers = Answers::preset(&shape, preset);
        app.backup.as_mut().expect("the backup is open").answers = answers.clone();
        let _ = app.frame();
        let want: Vec<u8> = crate::plan::checklist(&c, &shape, &answers)
            .into_iter()
            .map(crate::bstep::of)
            .collect();
        assert_eq!(
            app.backup_items_now(&c),
            want,
            "{preset:?}: the checklist is the one the plan as it is makes"
        );
        changed |= want != before;
    }
    assert!(changed, "a preset made another checklist");
}

#[test]
fn a_wallets_address_follows_its_network_and_its_policy() {
    let c = crate::testkit::compute();
    let kits = testkit::kits();
    let (one, two) = (&kits[0], &kits[1]);
    let mut w = Wallet::new(
        one.name.to_string(),
        read_wallet(&c, &one.descriptor).expect("a test wallet"),
        String::new(),
        Origin::Named,
    );
    let fresh = |d: &str, net: faraday_bip::keys::Network| {
        read_wallet(&c, d)
            .expect("a test wallet")
            .address_at(&c, net, false, 0)
            .expect("an address")
            .to_string()
    };
    for net in [
        testkit::NET,
        faraday_bip::keys::Network::Regtest,
        testkit::NET,
    ] {
        assert_eq!(w.address_on(&c, net, false, 0), fresh(&one.descriptor, net));
    }
    w.set_policy(read_wallet(&c, &two.descriptor).expect("a test wallet"));
    assert_eq!(
        w.address_on(&c, testkit::NET, false, 0),
        fresh(&two.descriptor, testkit::NET),
        "the address is the new policy's"
    );
}

#[test]
fn a_vault_holds_a_seed_typed_from_any_list_and_never_another_seed() {
    let c = crate::testkit::compute();
    use faraday_bip::bip39::{Language, Mnemonic};
    let (mut app, _) = heavy();
    unlock_test_vault(&mut app);
    let v = app.vaults.current;
    let words = |i: usize| {
        Mnemonic::parse(
            &c,
            Language::English,
            &testkit::test_words(testkit::TEST_SEEDS[i].0),
        )
        .expect("a test seed")
    };
    // The vault keeps test key 1; test key 2 is another seed.
    let kept = words(0);
    let spanish = Mnemonic::from_indices(Language::Spanish, kept.indices()).expect("Spanish");
    let other = words(1);
    for (m, holds) in [(spanish, true), (other, false), (kept, true)] {
        app.session
            .add_mnemonic(&c, &m, "", "", None)
            .expect("the seed loads");
        let k = app.session.keys.len() - 1;
        let _ = app.frame();
        assert_eq!(
            app.vault_holds_seed(&c, v, k, false),
            holds,
            "{:?}: the vault holds this seed",
            m.language()
        );
    }
}

/// Another seed whose fingerprint looks up the same vault record is not
/// said to be kept there: the record's fingerprint finds it, its master
/// key's whole id says whose it is (F65). A real four-byte collision
/// cannot be made here, so the record's kept fingerprint is set to the
/// other seed's, as a collision would leave it.
#[test]
fn a_seed_whose_fingerprint_finds_a_record_of_another_seed_is_not_kept_there() {
    let c = crate::testkit::compute();
    use crate::vault::screen::{key_fingerprint, vault_has_key};
    use crate::wallet::fp_text;
    use faraday_bip::bip39::{Language, Mnemonic};
    use faraday_vault::records::kind;
    let (mut app, _) = heavy();
    unlock_test_vault(&mut app);
    let v = app.vaults.current;
    let words = |i: usize| {
        Mnemonic::parse(
            &c,
            Language::English,
            &testkit::test_words(testkit::TEST_SEEDS[i].0),
        )
        .expect("a test seed")
    };
    let kept = app
        .session
        .add_mnemonic(&c, &words(0), "", "", None)
        .expect("key 1");
    let other = app
        .session
        .add_mnemonic(&c, &words(1), "", "", None)
        .expect("key 2");
    let (k1, k2) = (app.session.keys.len() - 2, app.session.keys.len() - 1);
    assert!(
        vault_has_key(&c, &app, v, &app.session.keys[k1]),
        "the vault keeps key 1"
    );
    assert!(!vault_has_key(&c, &app, v, &app.session.keys[k2]));
    // The vault's record of key 1 now looks up as key 2's fingerprint.
    let record_fp = app.vaults.open[v]
        .contents
        .of(kind::KEY)
        .find_map(|(_, r)| key_fingerprint(&c, &app, r))
        .expect("the vault's key record");
    assert_eq!(record_fp, fp_text(kept));
    for e in app.vaults.fingerprints.borrow_mut().iter_mut() {
        if e.1 == record_fp {
            e.1 = fp_text(other);
        }
    }
    assert!(
        !vault_has_key(&c, &app, v, &app.session.keys[k2]),
        "a fingerprint alone does not make another seed kept"
    );
}

/// A wallet slot is offered a vault's seed only when the seed's account
/// key at the slot's path is the slot's key: the fingerprint finds the
/// record, the key derived says whose it is (F65). A real four-byte
/// collision cannot be made here, so the record's kept fingerprint is set
/// to another slot's, as a collision would leave it.
#[test]
fn a_vault_seed_whose_fingerprint_matches_a_slot_but_not_its_key_is_not_offered() {
    let c = crate::testkit::compute();
    use crate::vault::screen::key_fingerprint;
    use crate::wallet::{Slot, fp_text};
    use faraday_vault::records::kind;
    let (mut app, _) = heavy();
    unlock_test_vault(&mut app);
    let v = app.vaults.current;
    let record_fp = app.vaults.open[v]
        .contents
        .of(kind::KEY)
        .find_map(|(_, r)| key_fingerprint(&c, &app, r))
        .expect("the vault's key record");
    let savings = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .expect("Savings");
    let wallet = Wallet::new(
        savings.name.to_string(),
        read_wallet(&c, &savings.descriptor).expect("a test wallet"),
        String::new(),
        Origin::Named,
    );
    let slots = app.session.slots(&wallet);
    let fp_of = |s: &Slot| s.fingerprint.map(fp_text);
    let own = slots
        .iter()
        .find(|s| fp_of(s).as_deref() == Some(record_fp.as_str()))
        .expect("Savings has the vault's seed");
    assert!(
        app.vault_key_for(&c, own).is_some(),
        "the vault's seed is offered for its own slot"
    );
    let other = slots
        .iter()
        .find(|s| s.fingerprint.is_some() && fp_of(s).as_deref() != Some(record_fp.as_str()))
        .expect("Savings has another seed");
    let other_fp = other.fingerprint.expect("a fingerprint");
    // The vault's record now looks up as the other slot's fingerprint.
    for e in app.vaults.fingerprints.borrow_mut().iter_mut() {
        if e.1 == record_fp {
            e.1 = fp_text(other_fp);
        }
    }
    assert!(
        app.vault_key_for(&c, &Slot::bare(other_fp)).is_some(),
        "the fingerprint alone finds the record"
    );
    assert!(
        app.vault_key_for(&c, other).is_none(),
        "a record whose key at the slot's path is another is not offered"
    );
}

/// The checklist's vault item is done by a vault locked since it was
/// open only when its summary holds each seed by the seed's whole master
/// id: done for the seeds it held, not done from a summary with no ids,
/// and not done for a seed another seed's entry only shares the
/// fingerprint of. A real four-byte collision cannot be made here, so
/// another seed's entry is given the seed's fingerprint, as a collision
/// would leave it.
#[test]
fn the_checklists_locked_vault_is_done_by_each_seeds_whole_id() {
    let c = crate::testkit::compute();
    use crate::plan::Preset;
    use crate::wallet::fp_text;
    let (mut app, w) = heavy();
    unlock_test_vault(&mut app);
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, Preset::PaperVault);
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    let (own, _) = app.backup_vault_keys(0);
    assert!(!own.is_empty(), "the plan puts seeds into its vault");
    let o = app
        .backup_vault_open(&c, 0)
        .expect("an open vault holds the plan's vault");
    let name = app.vaults.open[o].name.clone();
    assert!(app.vault_lock_one(o), "the vault locks");
    assert_eq!(
        app.backup_vault_fits(&c, 0),
        Some(name.clone()),
        "the locked vault is done"
    );
    let salt = app
        .vault_files()
        .iter()
        .find(|f| f.name == name)
        .expect("the vault's file")
        .header
        .salt;
    let at = app
        .vaults
        .summaries
        .iter()
        .position(|s| s.salt == salt)
        .expect("its summary");

    // A summary kept before the ids were: not done.
    let ids = app.vaults.summaries[at].ids.clone();
    for id in &mut app.vaults.summaries[at].ids {
        *id = None;
    }
    assert_eq!(app.backup_vault_fits(&c, 0), None, "nothing is verified");
    app.vaults.summaries[at].ids = ids;

    // Another seed's entry under the first seed's fingerprint, and the
    // first seed's entry gone.
    let fp = fp_text(own[0]);
    let s = &mut app.vaults.summaries[at];
    let mine = s.keys.iter().position(|k| *k == fp).expect("the seed");
    let other = (0..s.keys.len())
        .find(|&i| i != mine && s.ids[i].is_some())
        .expect("another seed in the vault");
    s.keys[other] = fp;
    s.keys.remove(mine);
    s.ids.remove(mine);
    assert_eq!(
        app.backup_vault_fits(&c, 0),
        None,
        "a fingerprint alone does not make the vault done"
    );
}

#[test]
fn a_psbt_waiting_in_files_is_offered_on_its_wallets_card_once_the_wallet_loads() {
    let c = crate::testkit::compute();
    use crate::StorageEvent;
    let kits = testkit::kits();
    let savings = kits.iter().find(|k| k.id == "savings").expect("Savings");
    let mut app = testkit::started();
    let _ = app.session.set_network(testkit::NET);
    let psbt = testkit::unsigned(savings).expect("Savings' spend");
    app.storage(StorageEvent::Restored {
        inbox: vec![("savings-unsigned.psbt".into(), psbt.to_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    // Another wallet first: the PSBT is not its.
    let other = kits.iter().find(|k| k.id == "spending").expect("Spending");
    let w = app
        .session
        .add_wallet(&c, other.name, &other.descriptor, "test", Origin::Named)
        .expect("Spending loads");
    app.press(Action::Nav(Screen::Wallets));
    let _ = app.frame();
    assert_eq!(app.psbt_for(&c, w), None, "the PSBT is not Spending's");
    let w = app
        .session
        .add_wallet(&c, savings.name, &savings.descriptor, "test", Origin::Named)
        .expect("Savings loads");
    let _ = app.frame();
    assert_eq!(app.psbt_for(&c, w), Some(0), "the PSBT is Savings'");
    assert_ne!(
        app.spend_button(&c, w).0,
        crate::strings::EN.family_spend_from_this,
        "Savings' card offers to sign the PSBT in Files"
    );
}

#[test]
fn a_spend_opened_before_its_seeds_were_loaded_signs_with_them_once_they_are() {
    let c = crate::testkit::compute();
    use crate::StorageEvent;
    let kits = testkit::kits();
    let savings = kits.iter().find(|k| k.id == "savings").expect("Savings");
    let mut app = testkit::started();
    let _ = app.session.set_network(testkit::NET);
    app.session
        .add_wallet(&c, savings.name, &savings.descriptor, "test", Origin::Named)
        .expect("Savings loads");
    let psbt = testkit::unsigned(savings).expect("Savings' spend");
    app.storage(StorageEvent::Restored {
        inbox: vec![("savings-unsigned.psbt".into(), psbt.to_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::StartSpend(0));
    let _ = app.frame();
    assert!(
        app.spend
            .as_ref()
            .is_some_and(|s| s.inspection.participating_keys.iter().all(|fp| !app
                .session
                .keys
                .iter()
                .any(|k| k.fingerprint() == *fp))),
        "no seed of the spend is here yet"
    );
    for (word, l) in &testkit::TEST_SEEDS[..2] {
        let _ = app
            .session
            .add_words(&c, &testkit::test_words(word), l, None);
    }
    app.press(Action::SignHere);
    let _ = app.frame();
    let s = app.spend.as_ref().expect("the spend");
    assert_eq!(s.error, None);
    assert_eq!(
        s.spend.signed_here.len(),
        2,
        "both seeds loaded since signed"
    );
    assert_eq!(s.signers.len(), 2, "the review shows both signatures");
}

/// What the state keeps of what the Inbox gives, as a frame asks for it.
fn kept_found(app: &crate::Faraday) -> crate::inbox::Found {
    let c = crate::testkit::compute();
    app.inbox
        .found(&app.session, |items| app.found_in(&c, items))
}

/// [`kept_found`]'s fresh counterpart.
fn found_fresh(app: &crate::Faraday) -> String {
    let c = crate::testkit::compute();
    format!("{:?}", app.found_in(&c, &app.inbox))
}

/// The first file on the test stick of wallet `id` that reads as a
/// wallet.
fn wallet_file(id: &str) -> (String, Vec<u8>) {
    let c = crate::testkit::compute();
    testkit::files()
        .expect("the test stick's files")
        .into_iter()
        .find(|(n, b)| {
            n.starts_with(&format!("{id}-"))
                && crate::Item::new(&c, n, b.clone()).kind == crate::wallet::FileKind::Wallet
        })
        .expect("a wallet file")
}

/// What the Inbox gives as the state keeps it is what it gives worked out
/// again, and it is worked out again only when the files or the session
/// changed: each input of `found_in` changed in turn, and the kept result
/// follows (`inbox::InboxFiles`).
#[test]
fn what_the_inbox_gives_follows_the_files_and_the_session() {
    let c = crate::testkit::compute();
    use crate::Item;
    let kits = testkit::kits();
    let (one, two) = (&kits[0], &kits[1]);
    let (n1, b1) = wallet_file(one.id);
    let (n2, b2) = wallet_file(two.id);
    let mut app = testkit::started();
    let _ = app.session.set_network(testkit::NET);
    let mut last = format!("{:?}", kept_found(&app));
    let mut step = |app: &mut crate::Faraday, what: &str, changes: bool| {
        let kept = format!("{:?}", kept_found(app));
        assert_eq!(
            kept,
            found_fresh(app),
            "{what}: the kept result is the fresh one"
        );
        assert_eq!(
            kept != last,
            changes,
            "{what}: the Inbox's result changed: {changes}"
        );
        let rebuilds = app.inbox.rebuilds();
        let _ = kept_found(app);
        assert_eq!(
            app.inbox.rebuilds(),
            rebuilds,
            "{what}: asked again, not worked out again"
        );
        last = kept;
    };
    step(&mut app, "nothing in the Inbox", false);

    // The files: added, renamed, replaced, removed.
    app.inbox.push(Item::new(&c, &n1, b1.clone()));
    step(&mut app, "a wallet file added", true);
    assert_eq!(kept_found(&app).wallets.len(), 1);
    app.inbox[0].name = "renamed-wallet.txt".to_string();
    step(&mut app, "the file renamed", true);
    assert_eq!(kept_found(&app).wallets[0].name, "Renamed");
    app.inbox[0] = Item::new(&c, &n2, b2.clone());
    step(&mut app, "the file replaced by another wallet's", true);
    app.inbox.push(Item::new(&c, &n1, b1.clone()));
    step(&mut app, "the first wallet's file added again", true);
    assert_eq!(kept_found(&app).wallets.len(), 2);
    app.inbox.remove(0);
    step(&mut app, "a file removed", true);
    assert_eq!(kept_found(&app).wallets.len(), 1);

    // The session's wallets: loaded, renamed, its policy changed.
    app.session
        .add_wallet(&c, one.name, &one.descriptor, "test", Origin::Named)
        .expect("the wallet loads");
    step(&mut app, "the file's wallet loaded", true);
    assert!(kept_found(&app).wallets.iter().all(|w| w.loaded));
    app.session.wallets[0].name = "Kept under another name".to_string();
    step(&mut app, "a loaded wallet renamed", false);
    app.inbox.remove(0);
    step(&mut app, "the file removed, the loaded wallet stays", true);
    app.session.wallets[0].name = "Another name".to_string();
    step(&mut app, "the loaded wallet renamed", true);
    assert_eq!(kept_found(&app).wallets[0].name, "Another name");
    app.session.wallets[0].set_policy(read_wallet(&c, &two.descriptor).expect("a test wallet"));
    step(&mut app, "the loaded wallet's policy changed", true);
    app.session.wallets.clear();
    step(&mut app, "the loaded wallet gone", true);
    assert!(kept_found(&app).wallets.is_empty());

    // The session's keys: a seed in a file is loaded once the key is; a
    // key's label is its seed's name.
    let (word, l) = &testkit::TEST_SEEDS[0];
    app.inbox.push(Item::new(
        &c,
        "words.txt",
        format!("{}\n", testkit::test_words(word)).into_bytes(),
    ));
    step(&mut app, "a seed file added", true);
    assert!(!kept_found(&app).seeds[0].loaded);
    app.session
        .add_words(&c, &testkit::test_words(word), l, None)
        .expect("the seed loads");
    step(&mut app, "the seed loaded", true);
    assert!(kept_found(&app).seeds[0].loaded);
    let (word, l) = &testkit::TEST_SEEDS[1];
    app.session
        .add_words(&c, &testkit::test_words(word), l, None)
        .expect("the seed loads");
    step(&mut app, "another key loaded", true);
    assert_eq!(kept_found(&app).seeds.len(), 2);
    app.session.keys[1].label = "Another label".to_string();
    step(&mut app, "a key relabelled", true);
    assert_eq!(kept_found(&app).seeds[1].label, "Another label");
    app.session.keys.remove(0);
    step(&mut app, "a key gone", true);
    assert!(!kept_found(&app).seeds[0].loaded);
}
