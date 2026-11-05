//! A backup flow's copy round and an item the person moved into one of its
//! vaults (`docs/VAULT.md` §7.2).

use crate::backup_flow_wallet::{
    PHRASE, PHRASE_2, PHRASE_3, backup_shape, bring_in_all, create_with_vaults,
    flow_whole_in_vault, key_in, restart, sticks, two_of_three_ready,
};
use crate::common::RecordsOf as _;
use crate::common::*;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::Thing;
use faraday_core::plan::Item;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen, cstep};
use faraday_vault::records::kind;

/// What the test moves: a seed's record, or a passphrase record.
#[derive(Clone, Copy, PartialEq)]
enum Moved {
    Seed,
    Passphrase,
}

fn held(app: &Faraday, o: usize, what: Moved) -> Vec<Vec<u8>> {
    use faraday_vault::records::field;
    match what {
        Moved::Seed => contents(app, o)
            .of(kind::KEY)
            .filter_map(|(_, r)| {
                faraday_keys::vault::key_record_id(faraday_keys::vault::RecordView::of(r))
            })
            .map(|id| id.to_vec())
            .collect(),
        Moved::Passphrase => contents(app, o)
            .of(kind::PASSPHRASE)
            .map(|(_, r)| r.field(field::PASSPHRASE).unwrap_or_default().to_vec())
            .collect(),
    }
}

/// A Copy interrupted by a locked vault, `what` moved from Vault 2 into
/// added Vault 1, then Copy again (after a lock of everything, if `lock`):
/// what was moved is still in Vault 1.
fn moved_then_copy(what: Moved, lock: bool) {
    use faraday_vault::Record;
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    let mut app = two_of_three_ready(&mut shell, false);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    // Vaults 1 and 3 take the description; Vault 2 holds seed 2 and is
    // not touched by the plan.
    flow_whole_in_vault(&mut shell, &mut app, 0);
    flow_whole_in_vault(&mut shell, &mut app, 2);
    shell.flow(&mut app, F::Continue);
    open_vault(&mut shell, &mut app, 0, PHRASE);
    open_vault(&mut shell, &mut app, 1, PHRASE_2);
    // Vault 3 is still locked: Copy says to unlock it first.
    shell.flow(&mut app, F::CopyKey);
    let mut place = 0;
    if what == Moved::Passphrase {
        // A passphrase record kept alone, which the plan does not keep in
        // Vault 1, lands in Vault 2.
        let fp = {
            let wf = app.flow().unwrap().wallet.as_ref().unwrap();
            let name = &wf.shape.seeds[0].name;
            (0..4)
                .map(|k| u8::from_str_radix(&name[2 * k..2 * k + 2], 16).unwrap())
                .collect::<Vec<u8>>()
        };
        let id = app.vaults().open[1].id;
        place = contents(&app, 1)
            .iter()
            .filter(|r| [kind::KEY, kind::PASSPHRASE, kind::CODEX32_SHARE].contains(&r.kind))
            .count();
        testkit::vault_edit(&mut app, id, |s| {
            s.insert(
                id,
                Record::new(kind::PASSPHRASE)
                    .with(field::PASSPHRASE_KEY, &fp)
                    .with(field::PASSPHRASE, b"moved apart"),
            )
        });
    }
    let before = held(&app, 0, what);
    // The person moves it into Vault 1 meanwhile.
    testkit::vault_current(&mut app, 1);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(faraday_core::vault::VaultAction::Category(0)));
    app.press(Action::Vault(faraday_core::vault::VaultAction::Item(place)));
    app.press(Action::Vault(faraday_core::vault::VaultAction::MoveTo));
    app.press(Action::Vault(faraday_core::vault::VaultAction::CopyPick(0)));
    app.press(Action::Vault(faraday_core::vault::VaultAction::CopyGo));
    let moved: Vec<_> = held(&app, 0, what)
        .into_iter()
        .filter(|x| !before.contains(x))
        .collect();
    assert!(!moved.is_empty(), "nothing arrived in Vault 1");
    app.press(Action::Nav(Screen::BackupFlow));
    let mut app = if lock {
        let mut app = shell.lock(&mut app);
        open_vault(&mut shell, &mut app, 0, PHRASE);
        open_vault(&mut shell, &mut app, 1, PHRASE_2);
        app
    } else {
        app
    };
    open_vault(&mut shell, &mut app, 2, PHRASE_3);
    shell.flow(&mut app, F::CopyKey);
    let after = held(&app, 0, what);
    for x in &moved {
        assert!(after.contains(x), "a moved item was taken out of Vault 1");
    }
}

#[test]
fn a_seed_moved_into_an_added_vault_between_two_presses_of_copy_is_not_taken_out_again() {
    moved_then_copy(Moved::Seed, false);
}

#[test]
fn a_seed_moved_into_an_added_vault_stays_after_a_lock_between_two_presses_of_copy() {
    moved_then_copy(Moved::Seed, true);
}

#[test]
fn a_passphrase_record_moved_into_an_added_vault_stays_after_a_lock_between_two_presses_of_copy() {
    moved_then_copy(Moved::Passphrase, true);
}

/// A passphrase record moved into Vault 1 after the checklist saved it,
/// then the lock and a change of the plan that changes Vault 1: the round
/// leaves the moved record in Vault 1.
#[test]
fn a_record_moved_into_a_saved_vault_before_the_lock_stays_when_a_later_plan_changes_it() {
    use faraday_vault::Record;
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    let mut app = two_of_three_ready(&mut shell, false);
    // A passphrase record, which the plan does not keep, in Vault 2.
    let fp = app.session.keys.info(0).unwrap().fingerprint.0;
    let id = app.vaults().open[1].id;
    let place = contents(&app, 1)
        .iter()
        .filter(|r| [kind::KEY, kind::PASSPHRASE, kind::CODEX32_SHARE].contains(&r.kind))
        .count();
    testkit::vault_edit(&mut app, id, |s| {
        s.insert(
            id,
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &fp)
                .with(field::PASSPHRASE, b"moved apart"),
        )
    });
    let before = held(&app, 0, Moved::Passphrase);
    // The person moves it into Vault 1.
    testkit::vault_current(&mut app, 1);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(faraday_core::vault::VaultAction::Category(0)));
    app.press(Action::Vault(faraday_core::vault::VaultAction::Item(place)));
    app.press(Action::Vault(faraday_core::vault::VaultAction::MoveTo));
    app.press(Action::Vault(faraday_core::vault::VaultAction::CopyPick(0)));
    app.press(Action::Vault(faraday_core::vault::VaultAction::CopyGo));
    let moved: Vec<_> = held(&app, 0, Moved::Passphrase)
        .into_iter()
        .filter(|x| !before.contains(x))
        .collect();
    assert!(!moved.is_empty(), "nothing arrived in Vault 1");
    app.press(Action::Nav(Screen::Backup));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_whole_in_vault(&mut shell, &mut app, 0);
    shell.flow(&mut app, F::Continue);
    open_vault(&mut shell, &mut app, 0, PHRASE);
    open_vault(&mut shell, &mut app, 1, PHRASE_2);
    open_vault(&mut shell, &mut app, 2, PHRASE_3);
    shell.flow(&mut app, F::CopyKey);
    let after = held(&app, 0, Moved::Passphrase);
    for x in &moved {
        assert!(
            after.contains(x),
            "the moved record was taken out of Vault 1"
        );
    }
}

/// A wallet's Share 2 in Vault 1 saved by Backup: the plan's split changed
/// (fewer keys left off a sheet) and saved again replaces the old Share 2,
/// and the vault holds one.
#[test]
fn a_changed_plans_split_saved_again_replaces_the_old_share_2() {
    use faraday_vault::records::field;
    let c = testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let _ = app.create().unwrap().built.expect("wallet made");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    let vaults = {
        let shape = backup_shape(&app);
        app.backup().unwrap().answers.vault_count(&shape)
    };
    for v in 1..vaults {
        vault_off(&mut app, v);
    }
    description_out_of(&mut app, 0);
    no_paper_description(&mut app);
    no_description_files(&mut app);
    into_vault(&mut app, 0, Thing::Share(1));
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    let shares = |app: &Faraday| -> Vec<(String, u8)> {
        contents(app, 0)
            .of(kind::WALLET_SHARE)
            .map(|(_, r)| {
                (
                    r.text(field::SHARE).unwrap_or("").to_string(),
                    r.field(field::SHARE_OF).unwrap_or(&[0])[0],
                )
            })
            .collect()
    };
    let own = shares(&app);
    assert_eq!(own.len(), 1);
    // The plan's split changed: its old Share 2 goes, the new one is kept.
    app.press(Action::BOmit(0));
    app.press(Action::BVaultSave(0));
    let resplit = shares(&app);
    assert_eq!(resplit.len(), 1, "{resplit:?}");
    assert!(!resplit.contains(&own[0]), "the old Share 2 went");
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
}
