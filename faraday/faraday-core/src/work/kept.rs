//! What the app keeps worked out follows what it was worked out from: a
//! change to a vault's records, a plan's answers or a wallet's policy is
//! what the next frame shows and the next press reads (`docs/SPEED.md`
//! §3.3).

use osk_shell_api::App;

use super::budget::{heavy, unlock_test_vault};
use crate::wallet::{Origin, Wallet, read_wallet};
use crate::{Action, Screen, testkit};

/// A wallet's shape, as the session names it.
fn shape_of(descriptor: &str) -> String {
    crate::vault_screens::policy_shape(read_wallet(descriptor).expect("a test wallet"))
}

#[test]
fn a_wallet_record_changed_in_an_open_vault_shows_as_it_now_is() {
    use faraday_vault::records::field;
    let (mut app, _) = heavy();
    unlock_test_vault(&mut app);
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    let v = app.vaults.current;
    let row = app
        .vault_rows(v)
        .into_iter()
        .find(|r| !r.seed)
        .expect("the test vault keeps a wallet");
    let other = testkit::kits()
        .into_iter()
        .find(|k| shape_of(&k.descriptor) != row.detail)
        .expect("a test wallet of another shape");
    app.vaults.open[v].contents.records[row.record].set(field::WALLET, other.descriptor.as_bytes());
    let _ = app.frame();
    let now = app
        .vault_rows(v)
        .into_iter()
        .find(|r| r.record == row.record)
        .expect("the record is listed");
    assert_eq!(
        now.detail,
        shape_of(&other.descriptor),
        "the vault's list shows the wallet the record now holds"
    );
}

#[test]
fn the_checklist_follows_a_plan_changed_since_the_last_frame() {
    use crate::plan::{Answers, Preset};
    let (mut app, w) = heavy();
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, Preset::PaperVault);
    app.press(Action::BChecklist);
    let _ = app.frame();
    let shape = app.plan_shape(w);
    let before = app.backup_items();
    let mut changed = false;
    for preset in Preset::ALL {
        let answers = Answers::preset(&shape, preset);
        app.backup.as_mut().expect("the backup is open").answers = answers.clone();
        let _ = app.frame();
        let want: Vec<u8> = crate::plan::checklist(&shape, &answers)
            .into_iter()
            .map(crate::bstep::of)
            .collect();
        assert_eq!(
            app.backup_items(),
            want,
            "{preset:?}: the checklist is the one the plan as it is makes"
        );
        changed |= want != before;
    }
    assert!(changed, "a preset made another checklist");
}

#[test]
fn a_wallets_address_follows_its_network_and_its_policy() {
    let kits = testkit::kits();
    let (one, two) = (&kits[0], &kits[1]);
    let mut w = Wallet::new(
        one.name.to_string(),
        read_wallet(&one.descriptor).expect("a test wallet"),
        String::new(),
        Origin::Named,
    );
    let fresh = |d: &str, net: osk_bip::keys::Network| {
        read_wallet(d)
            .expect("a test wallet")
            .address_at(net, false, 0)
            .expect("an address")
            .to_string()
    };
    for net in [testkit::NET, osk_bip::keys::Network::Regtest, testkit::NET] {
        assert_eq!(w.address_on(net, false, 0), fresh(&one.descriptor, net));
    }
    w.set_policy(read_wallet(&two.descriptor).expect("a test wallet"));
    assert_eq!(
        w.address_on(testkit::NET, false, 0),
        fresh(&two.descriptor, testkit::NET),
        "the address is the new policy's"
    );
}

#[test]
fn a_vault_holds_a_seed_typed_from_any_list_and_never_another_seed() {
    use osk_bip::bip39::{Language, Mnemonic};
    let (mut app, _) = heavy();
    unlock_test_vault(&mut app);
    let v = app.vaults.current;
    let words = |i: usize| {
        Mnemonic::parse(
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
            .add_mnemonic(&m, "", "", None)
            .expect("the seed loads");
        let k = app.session.keys.len() - 1;
        let _ = app.frame();
        assert_eq!(
            app.vault_holds_seed(v, k, false),
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
    use crate::vault_screens::{key_fingerprint, vault_has_key};
    use crate::wallet::fp_text;
    use faraday_vault::records::kind;
    use osk_bip::bip39::{Language, Mnemonic};
    let (mut app, _) = heavy();
    unlock_test_vault(&mut app);
    let v = app.vaults.current;
    let words = |i: usize| {
        Mnemonic::parse(
            Language::English,
            &testkit::test_words(testkit::TEST_SEEDS[i].0),
        )
        .expect("a test seed")
    };
    let kept = app
        .session
        .add_mnemonic(&words(0), "", "", None)
        .expect("key 1");
    let other = app
        .session
        .add_mnemonic(&words(1), "", "", None)
        .expect("key 2");
    let (k1, k2) = (app.session.keys.len() - 2, app.session.keys.len() - 1);
    assert!(
        vault_has_key(&app, v, &app.session.keys[k1]),
        "the vault keeps key 1"
    );
    assert!(!vault_has_key(&app, v, &app.session.keys[k2]));
    // The vault's record of key 1 now looks up as key 2's fingerprint.
    let record_fp = app.vaults.open[v]
        .contents
        .of(kind::KEY)
        .find_map(|(_, r)| key_fingerprint(&app, r))
        .expect("the vault's key record");
    assert_eq!(record_fp, fp_text(kept));
    for e in app.vaults.fingerprints.borrow_mut().iter_mut() {
        if e.1 == record_fp {
            e.1 = fp_text(other);
        }
    }
    assert!(
        !vault_has_key(&app, v, &app.session.keys[k2]),
        "a fingerprint alone does not make another seed kept"
    );
}
