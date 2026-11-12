//! The backup hold (`docs/BACKUP-FLOW.md` §10.2) on a passphrase made in
//! Create and kept in a vault: the flow ends Done and the key signs.

use crate::backup_flow_wallet::{
    PHRASE, PHRASE_2, bring_in_all, create_with_vaults, key_in, restart, save_vaults, step, sticks,
    write_sticks,
};
use crate::common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::Thing;
use faraday_core::testkit;
use faraday_core::{Action, Screen, bstep, cstep};

#[test]
fn a_passphrase_made_in_create_and_kept_in_the_vault_is_written_and_signs() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let bare = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, bare));
    app.press(Action::CPassOpen(0));
    type_text(&mut app, "kept");
    app.press(Action::CPassField(1));
    type_text(&mut app, "kept");
    app.press(Action::CPassLock);
    let fp = app.session.keys.last().unwrap().fingerprint;
    assert_ne!(fp.0, bare);
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    no_paper_passphrases(&mut app);
    no_paper_description(&mut app);
    into_vault(&mut app, 0, Thing::Pass(0));
    assert!(app.backup().unwrap().answers.pass_in_vault(0));
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    assert!(
        !app.key_held(&c, fp),
        "the key signs once its stick is written"
    );
}

#[test]
fn a_chart_edit_moving_a_written_sticks_vault_is_refused_and_the_key_stays_held() {
    use crate::backup_flow_wallet::load_first_vault;
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::ChartAction as C;
    use faraday_core::plan::{self, At, What};
    let c = testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 2);
    bring_in_all(&mut shell, &mut app, &["S1", "S2"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    app.press(Action::Nav(Screen::Create));
    let fp = crate::backup_flow_wallet::new_key(&mut app);
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    let w = app.create().unwrap().built.expect("wallet made");
    let name = app.session.wallets[w].name.clone();
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    second_place(&mut app);
    no_words(&mut app);
    no_paper_description(&mut app);
    description_file_at(&mut app, 1);
    // Both vaults on one stick at Place 1, the seed in each.
    let shape = app.plan_shape_now(app.backup().unwrap().wallet);
    testkit::change_backup_answers(&mut app, |a| {
        let mut m = plan::map(&c, &shape, a);
        for v in 0..2 {
            for p in 0..plan::places_of(&m) {
                m = plan::without(&m, At::Place(p), What::VaultStick(v));
            }
        }
        m = plan::with_vault_stick(&m, 0, 0);
        m = plan::with_added(&m, At::Stick(0, 0), What::VaultStick(1));
        m = plan::with_added(&m, At::Vault(1), What::Seed(0));
        a.set_map(m);
    });
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    let_seed_files_out(&mut app);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert!(app.flow().unwrap().slots[0].written.is_some());
    load_first_vault(&mut app, PHRASE);
    let w = app
        .session
        .wallets
        .iter()
        .position(|x| x.name == name)
        .expect("the wallet loaded from its vault");
    let fp = faraday_bip::keys::Fingerprint(fp);
    assert!(app.key_held(&c, fp), "held before the edit");
    let before = app.flow().unwrap().wallet.as_ref().unwrap().answers.clone();
    app.press(Action::Backup(w));
    let press = Press::Loaded(w);
    // Vault 2's stick moves to Place 2: Place 1 keeps a stick, but not the
    // vaults it was written with.
    app.press(Action::Chart(C::Edit(
        press,
        plan::Edit::Move(At::Place(0), What::VaultStick(1), At::Place(1)),
    )));
    app.press(Action::Chart(C::Confirm(press)));
    expect(&mut app, "A stick already written stays on the plan");
    let f = app.flow().unwrap();
    assert_eq!(f.wallet.as_ref().unwrap().answers, before, "the plan stays");
    assert_ne!(step(&app), Step::Done);
    assert!(app.key_held(&c, fp), "the key stays held");
}

#[test]
fn words_kept_without_a_passphrase_by_a_flow_hold_only_the_passphrase_still_written_nowhere() {
    use crate::backup_hold_passphrases::{
        back_up_on_paper, pass_in_slot, toast_names, vault_says_pending,
    };
    use faraday_bip::keys::Fingerprint;
    let c = testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let bare = Fingerprint(key_in(&mut app, 1));
    app.press(Action::Nav(Screen::Create));
    // Two passphrases made here on the words: `two` waits, the flow keeps
    // the words of `kept` in its vault, its passphrase on paper.
    let two = pass_in_slot(&mut app, bare, "two");
    let kept = pass_in_slot(&mut app, bare, "kept");
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    no_paper_description(&mut app);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    save_vaults(&mut app);
    testkit::type_passphrases(&mut app);
    if app.backup().unwrap().open != Some(bstep::ENVELOPE) {
        app.press(Action::BStep(bstep::ENVELOPE));
    }
    app.press(Action::BNext(bstep::ENVELOPE));
    let_seed_files_out(&mut app);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(step(&app), Step::Done);
    // The vault the flow wrote, unlocked after the restart: its record of
    // the words says pending, for `two`.
    crate::backup_flow_wallet::load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(vault_says_pending(&app, 0));
    assert!(
        !app.key_held(&c, bare),
        "the words the flow wrote are not held again"
    );
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    bip39_passphrase(&mut app, "kept");
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.last().unwrap().fingerprint, kept);
    assert!(!app.key_held(&c, kept), "the flow wrote its copy");
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    bip39_passphrase(&mut app, "two");
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.last().unwrap().fingerprint, two);
    assert!(app.key_held(&c, two), "its passphrase is written nowhere");
    app.press(Action::CreateWallet);
    ckind(&mut app, 0);
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CSlotHere(0, two.0));
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, two));
    assert!(app.held_keys_now(&c).is_empty(), "each key has a copy");
    assert!(!vault_says_pending(&app, 0), "a reload holds none of them");
}
