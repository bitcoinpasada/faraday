//! The Bitcoin wallet's part of the guided backup flow, later cases
//! (`docs/BACKUP-FLOW.md` §15, §17): the plan by place, the round taking
//! out only what the flow put in, and what the lock leaves to change.

use crate::backup_flow_wallet::{
    PHRASE, PHRASE_2, backup_places, backup_shape, bring_in_all, create_with_vaults, flow_board,
    flow_into_vault, flow_keep, flow_pass_in_vault, flow_place_of, flow_plan, flow_put,
    flow_whole_file_at, flow_whole_in_vault, kdbx, key_in, load_first_vault, multi_index, names,
    new_key, new_key_here, no_visit, offered_down, restart, save_vaults, seed_words_file,
    single_key, single_key_ready, single_key_with, step, sticks, two_of_three_with_setup,
    vault_place, vault_with_key, write_sticks,
};
use crate::common::RecordsOf as _;
use crate::common::*;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::backup_flow::{BackupFlow, Step, Target};
use faraday_core::board::{BoardPress, Thing, Zone};
use faraday_core::create::NewKind;
use faraday_core::plan::{self, Answers, At, Item, Preset, Row, What, form};
use faraday_core::testkit;
use faraday_core::testkit::KindText;
use faraday_core::{Action, BStage, Faraday, PlanPress, Screen, bstep, cstep, qrow};
use faraday_shell_api::{App, Event, Key};
use faraday_vault::records::kind;

// ---- the review of the plan by place (`docs/BACKUP-FLOW.md` §15) ----

/// The guided flow's own plan card, Back up's cards, edits the map the
/// cards made: what they put elsewhere stays.
#[test]
fn the_flows_plan_card_edits_the_map_the_cards_made() {
    let c = faraday_core::testkit::compute();
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    // The seed out of Vault 1, on its sheet.
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    let a = &wf.answers;
    assert!(a.map.is_some(), "the map stays the plan");
    let m = plan::place_map(&c, &wf.shape, a);
    let at = |at: At| -> Vec<What> {
        m.iter()
            .filter(|s| s.at == at)
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .collect()
    };
    assert!(a.vault_seeds(&wf.shape, 0).is_empty(), "no seed in a vault");
    assert!(
        at(At::Place(1))
            .iter()
            .any(|w| matches!(w, What::WalletFile(_))),
        "the description's files stay at Place 2"
    );
    assert!(
        !m.iter()
            .any(|s| s.holds.iter().any(|(w, _)| matches!(w, What::Words(_)))),
        "no words came back"
    );
}

/// A key kept only as a file at a place is held until that place's stick
/// of files is written, and released once it is.
#[test]
fn a_seed_kept_as_a_file_at_a_place_is_released_once_that_stick_is_written() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    let fp = new_key_here(&mut app);
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    vault_off(&mut app, 0);
    no_words(&mut app);
    files_at(&mut app, 1, &[0]);
    no_paper_description(&mut app);
    app.press(Action::BChecklist);
    app.press(Action::BNext(bstep::ENVELOPE));
    let fp = faraday_bip::keys::Fingerprint(fp);
    assert!(app.key_held(&c, fp), "held for Place 2's stick of files");
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(
        let_seed_files_out(&mut app),
        0,
        "held for at the drop: none at the lock"
    );
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(app.flow().unwrap().slots[0].target, Target::PlaceFiles(1));
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(step(&app), Step::Done);
    assert!(!app.key_held(&c, fp), "released once its stick is written");
}

/// Each seed's file at a place of its own, all three let out: each
/// place's stick of files is written with that place's seed file alone.
#[test]
fn each_places_stick_of_files_takes_only_that_places_seed_file() {
    let c = faraday_core::testkit::compute();
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, multi_index());
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    no_words(&mut app);
    no_paper_description(&mut app);
    for p in 0..3 {
        files_at(&mut app, p, &[p]);
    }
    let seed_names: Vec<String> = backup_shape(&app)
        .seeds
        .iter()
        .map(|s| s.name.to_lowercase())
        .collect();
    app.press(Action::BChecklist);
    // The files are made at the lock and written with their sticks
    // (`docs/BACKUP-FLOW.md` §17.12 item 6).
    assert_eq!(
        app.backup_items_now(&c),
        vec![bstep::ENVELOPE, bstep::STICKS]
    );
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(
        let_seed_files_out(&mut app),
        0,
        "held for at the drop: none at the lock"
    );
    assert_eq!(app.outbox.iter().filter(|i| i.secret).count(), 3);
    let mut app = restart(&mut shell, &mut app);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        vec![
            Target::PlaceFiles(0),
            Target::PlaceFiles(1),
            Target::PlaceFiles(2)
        ]
    );
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    for (p, d) in ["S1", "S2", "S3"].iter().enumerate() {
        let words: Vec<String> = names(&shell, d)
            .into_iter()
            .filter(|n| n.starts_with("seed-") && n.ends_with(".txt"))
            .collect();
        assert_eq!(words.len(), 1, "{d}: {words:?}");
        assert!(words[0].contains(&seed_names[p]), "{d}: {words:?}");
    }
    no_visit(&shell);
}

/// A share kept in a vault (`docs/BACKUP-FLOW.md` §15.6): Save into the
/// vault puts it there as a type 13 record, the item is done only then,
/// the sealed vault holds it after the lock, and Restore with that vault
/// open and another share in the Inbox rebuilds the wallet.
#[test]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn a_share_kept_in_a_vault_is_saved_sealed_and_rebuilds_with_another() {
    let c = testkit::compute();
    use faraday_vault::records::field;
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
    let w = app.create().unwrap().built.expect("wallet made");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    // Nothing on paper; Vault 1 alone, holding Share 2.
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
    let p = vault_place(&app, 0);
    let s = vault_stick(&app, p, 0).expect("Vault 1's stick");
    put(&mut app, Thing::Share(1), Zone::Vault(p, s, 0));
    no_paper_description(&mut app);
    {
        let shape = backup_shape(&app);
        let a = &app.backup().unwrap().answers;
        assert_eq!(a.vault_shares(&c, &shape, 0), vec![1], "Share 2 in Vault 1");
    }
    let omit = app.backup().unwrap().answers.omit;
    let (m, n) = faraday_core::wallet::Session::quorum(&app.session.wallets[w]);
    let split = faraday_core::wallet_backup::split_plan(n, m, omit);
    let texts: Vec<String> = split
        .iter()
        .map(|row| faraday_core::wallet_backup::split_share(&app.session.wallets[w], row).unwrap())
        .collect();
    let share = |j: usize| texts[j].clone();
    let made = app.session.wallets[w].policy().to_descriptor_checksummed();
    let stem = faraday_core::file_stem(&app.session.wallets[w].name);
    app.press(Action::BChecklist);
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
    assert!(!app.backup_item_done_now(&c, Item::Vault(0)));
    app.press(Action::BVaultSave(0));
    let shares = |app: &Faraday| -> Vec<(String, Vec<u8>)> {
        contents(app, 0)
            .of(kind::WALLET_SHARE)
            .map(|(_, r)| {
                (
                    r.text(field::SHARE).unwrap_or("").to_string(),
                    r.field(field::SHARE_NUMBER).unwrap_or(&[]).to_vec(),
                )
            })
            .collect()
    };
    assert_eq!(shares(&app), vec![(share(1), vec![2])]);
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    // Done only while the vault holds it; saved again, kept once.
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| {
        s.remove_where(id, |r| r.kind() == kind::WALLET_SHARE)
    });
    assert!(
        !app.backup_item_done_now(&c, Item::Vault(0)),
        "the share is part of the item"
    );
    app.press(Action::BVaultSave(0));
    app.press(Action::BVaultSave(0));
    assert_eq!(shares(&app).len(), 1);
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
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
    let sealed = kdbx(&shell, "S1").expect("the vault's copy");
    let c = kdbx_records(&sealed, PHRASE);
    assert_eq!(c.of(kind::WALLET_SHARE).count(), 1, "sealed with the share");
    // Restore: that vault open, Share 1 in the Inbox.
    let name = format!("{stem}-share-1-of-{n}.txt");
    let mut r = process(
        vec![
            ("vault-1.kdbx".into(), sealed),
            (name.clone(), share(0).into_bytes()),
        ],
        Vec::new(),
        Vec::new(),
    );
    keep_time(&mut r);
    r.press(Action::Nav(Screen::Vaults));
    r.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    unlock(&mut r, PHRASE);
    assert_eq!(r.vaults().open.len(), 1);
    r.press(Action::RestoreWallet);
    let i = r.inbox().iter().position(|f| f.name == name).unwrap();
    r.press(Action::RShare(i));
    r.press(Action::RRebuild);
    let rw = r
        .restore()
        .and_then(|x| x.wallet)
        .unwrap_or_else(|| panic!("{:?}", r.restore().and_then(|x| x.error.clone())));
    assert_eq!(
        r.session.wallets[rw].policy().to_descriptor_checksummed(),
        made,
        "the same wallet, the same checksum"
    );
}

/// A share put into a vault after the lock, from Change the plan, changes
/// what the vault holds (`docs/BACKUP-FLOW.md` §10.2, §15.6): the vault
/// goes back to be filled and written, and the flow does not finish with
/// the share missing from its sealed copy.
#[test]
fn a_share_put_into_a_vault_after_the_lock_starts_the_round_and_is_sealed() {
    let c = testkit::compute();
    use faraday_vault::records::field;
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
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    // Vault 1 alone, with its seed and the whole description; nothing on
    // paper.
    no_words(&mut app);
    let vaults = {
        let shape = backup_shape(&app);
        app.backup().unwrap().answers.vault_count(&shape)
    };
    for v in 1..vaults {
        vault_off(&mut app, v);
    }
    no_paper_description(&mut app);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    // Change the plan: in shares, Share 2 in Vault 1.
    shell.flow(&mut app, F::ChangePlan);
    let in_vault = |app: &Faraday| {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        wf.answers.vault_shares(&c, &wf.shape, 0)
    };
    // The vault's shares only (§17.8): Share 2 sealed in Vault 1, the
    // description in shares from then on.
    flow_into_vault(&mut shell, &mut app, 0, Thing::Share(1));
    assert_eq!(in_vault(&app), vec![1], "Share 2 in Vault 1");
    assert_eq!(app.flow().unwrap().slots.len(), 1, "Vault 1's stick alone");
    assert!(app.flow().unwrap().adding, "a share added is a round");
    assert_eq!(step(&app), Step::Vaults);
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    shell.flow(&mut app, F::CopyKey);
    let numbers: Vec<Vec<u8>> = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::WALLET_SHARE)
        .map(|(_, r)| r.field(field::SHARE_NUMBER).unwrap_or(&[]).to_vec())
        .collect();
    assert_eq!(numbers, vec![vec![2]], "{:?}", app.toast_text());
    shell.flow(&mut app, F::MadeContinue);
    assert_eq!(step(&app), Step::Lock);
    shell.flow(&mut app, F::LockWrite);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = kdbx(&shell, "S1").expect("Vault 1 on S1");
    let c = kdbx_records(&sealed, PHRASE);
    assert_eq!(
        c.of(kind::WALLET_SHARE).count(),
        1,
        "the sealed copy holds Share 2"
    );
    no_visit(&shell);
}

// ---- the second review (`docs/BACKUP-FLOW.md` §15.6, §15.7) ----

/// A single key whose Vault 1 is on one stick, at Place 2, with the
/// description's files at Place 2 too: made, saved, stopped on the
/// checklist's Write to sticks.
fn single_key_one_stick(shell: &mut Shell) -> Faraday {
    sticks(shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(shell, &mut app, &["S1"]);
    create_vault(shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    // Over two places, as a plan saved before one place per key
    // (`docs/BACKUP-FLOW.md` §17.19).
    preset_two_places(&mut app, 2);
    no_words(&mut app);
    no_paper_description(&mut app);
    description_file_at(&mut app, 1);
    assert_eq!(vault_place(&app, 0), 1, "Vault 1's stick at Place 2 alone");
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    app
}

/// The row sheet's Done while a stick is written changes nothing of the
/// plan, and the write counts for the stick it was for.
#[test]
fn done_on_a_row_sheet_during_a_write_changes_nothing_and_the_write_counts_for_its_stick() {
    let mut shell = Shell::new();
    let mut app = single_key_one_stick(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let targets = |app: &Faraday| -> Vec<Target> {
        app.flow().unwrap().slots.iter().map(|s| s.target).collect()
    };
    assert_eq!(targets(&app), vec![Target::Place(1), Target::PlaceFiles(1)]);
    shell.flow(&mut app, F::ChangePlan);
    // Vault 1's chip held to go to Place 1, then S1 in: its write starts.
    flow_board(
        &mut shell,
        &mut app,
        BoardPress::Chip(faraday_core::board::Chip::List(Thing::Vault(0))),
    );
    shell.hold_writes = true;
    shell.plug(&mut app, "S1");
    assert!(
        app.flow().unwrap().live.writing.is_some(),
        "the write is under way"
    );
    let before = app.flow().unwrap().wallet.as_ref().unwrap().answers.clone();
    shell.press(&mut app, Action::Board(BoardPress::Zone(Zone::Card(0))));
    assert_eq!(
        app.flow().unwrap().wallet.as_ref().unwrap().answers,
        before,
        "the plan does not change under a write"
    );
    assert_eq!(targets(&app), vec![Target::Place(1), Target::PlaceFiles(1)]);
    shell.release_writes(&mut app);
    let f = app.flow().unwrap();
    assert!(
        f.slots[0].written.is_some(),
        "S1 counts for Vault 1 at Place 2"
    );
    assert!(f.slots[1].written.is_none());
}

/// Two wallets of one name, each with its Share 2 in one vault: both
/// shares stay, and each wallet's vault item sees only its own.
#[test]
fn two_wallets_of_one_name_keep_each_its_own_share_in_one_vault() {
    let c = testkit::compute();
    use faraday_vault::records::field;
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
    let a = app.create().unwrap().built.expect("wallet made");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    // Share 2 in Vault 1, as each of the two wallets plans it.
    let share_two_in_vault = |app: &mut Faraday| {
        no_words(app);
        let vaults = {
            let shape = backup_shape(app);
            app.backup().unwrap().answers.vault_count(&shape)
        };
        for v in 1..vaults {
            vault_off(app, v);
        }
        description_out_of(app, 0);
        no_paper_description(app);
        no_description_files(app);
        into_vault(app, 0, Thing::Share(1));
        app.press(Action::BChecklist);
        app.press(Action::BVaultSave(0));
    };
    share_two_in_vault(&mut app);
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    // Another wallet, of the same name.
    let name = app.session.wallets[a].name.clone();
    let keys: Vec<String> = (0..3)
        .map(|k| {
            NewKind::TapMulti
                .key_text(&c, app.session.keys.master_for_tests(k))
                .unwrap()
        })
        .collect();
    let b = app
        .session
        .add_wallet(
            &c,
            &name,
            &NewKind::TapMulti.descriptor(2, &keys),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_ne!(b, a);
    app.press(Action::Backup(b));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    share_two_in_vault(&mut app);
    assert!(
        app.backup_item_done_now(&c, Item::Vault(0)),
        "B's share saved"
    );
    let shares = |app: &Faraday| -> Vec<(String, Vec<u8>)> {
        contents(app, 0)
            .of(kind::WALLET_SHARE)
            .map(|(_, r)| {
                (
                    r.text(field::SHARE_WALLET).unwrap_or("").to_string(),
                    r.field(field::SHARE_NUMBER).unwrap_or(&[]).to_vec(),
                )
            })
            .collect()
    };
    let both = shares(&app);
    assert_eq!(both.len(), 2, "both Share 2s stay");
    assert!(both.iter().all(|(_, n)| n == &vec![2]));
    assert_ne!(both[0].0, both[1].0, "each told by its wallet");
    // A's share taken out: B's item is still done, A's is not.
    let a_sum = app.session.wallets[a].checksum().to_string();
    let id = app.vaults().open[0].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| {
        s.remove_where(id, |r| {
            r.kind() == kind::WALLET_SHARE
                && r.public_text(field::SHARE_WALLET)
                    .and_then(|t| faraday_core::wallet::read_wallet(&c, t).ok())
                    .is_some_and(|p| p.checksum() == a_sum)
        })
    });
    assert!(
        app.backup_item_done_now(&c, Item::Vault(0)),
        "B sees its own share"
    );
    app.press(Action::Backup(a));
    assert!(
        !app.backup_item_done_now(&c, Item::Vault(0)),
        "A does not take B's"
    );
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    assert_eq!(shares(&app).len(), 2);
}

/// "With its passphrase" unticked after the lock: a round starts, and
/// the sealed copy on S1 keeps the key without its passphrase.
#[test]
fn a_passphrase_taken_out_of_a_vault_after_the_lock_starts_a_round_and_leaves_it() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    bip39_passphrase(&mut app, "kept apart");
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint.0;
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    // The key in Vault 1 with its passphrase; nothing else.
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
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    let p = flow_place_of(&app, 0);
    let _ = p;
    flow_pass_in_vault(&mut shell, &mut app, 0, 0);
    assert!(app.flow().unwrap().adding, "{:?}", app.toast_text());
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    shell.flow(&mut app, F::CopyKey);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::LockWrite);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = kdbx(&shell, "S1").expect("Vault 1 on S1");
    let c = kdbx_records(&sealed, PHRASE);
    let keys: Vec<bool> = c
        .of(kind::KEY)
        .map(|(_, r)| {
            r.field(faraday_vault::records::field::KEY_PASSPHRASE)
                .is_some()
        })
        .collect();
    assert_eq!(keys, vec![false], "the key, without its passphrase");
}

/// After the lock, Back up's own cards edit the flow's plan, with its
/// refusals: an allowed edit is the flow's, a refused one changes
/// neither.
#[test]
fn back_ups_cards_after_the_lock_edit_the_flows_plan_with_its_refusals() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    let (mut app, w, fp) = single_key(&mut shell, true);
    let name = app.session.wallets[w].name.clone();
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert!(app.flow().unwrap().made);
    // A vault unlocked after the lock: the wallet and its key load from
    // it, the key still held and no receive address shown for it; the
    // plan stays the flow's (§17.8).
    load_first_vault(&mut app, PHRASE);
    let w = app
        .session
        .wallets
        .iter()
        .position(|x| x.name == name)
        .expect("the wallet loaded from its vault");
    assert!(
        app.key_held(&c, faraday_bip::keys::Fingerprint(fp)),
        "held, from the vault"
    );
    assert!(
        app.receive_held(&c, &app.session.wallets[w]),
        "no receive address"
    );
    app.press(Action::Backup(w));
    assert!(app.backup().is_some());
    let slots = app.flow().unwrap().slots.len();
    // Vault 1's stick at Place 1 off: a vault's stick, allowed, the flow's.
    let s = vault_stick(&app, 0, 0).expect("Vault 1's stick at Place 1");
    take_off(&mut app, Zone::Files(0, s), Thing::Vault(0));
    assert_eq!(
        app.flow().unwrap().slots.len(),
        slots - 1,
        "{:?}",
        app.toast_text()
    );
    let flow_answers = app.flow().unwrap().wallet.as_ref().unwrap().answers.clone();
    assert_eq!(app.backup().unwrap().answers, flow_answers);
    // Its last stick off: the key made here would be kept on no stick.
    let s = vault_stick(&app, 1, 0).expect("Vault 1's stick at Place 2");
    take_off(&mut app, Zone::Files(1, s), Thing::Vault(0));
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    assert_eq!(app.backup().unwrap().answers, flow_answers);
    // Files stay as made, a vault open or not.
    put(&mut app, Thing::Whole, Zone::NewStick(0));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert_eq!(
        app.flow().unwrap().wallet.as_ref().unwrap().answers,
        flow_answers
    );
}

/// The flow's own slider of keys left off each share sets the flow's
/// plan, and the vault that keeps a share goes back to take the share it
/// implies (§17.7).
#[test]
fn the_flows_slider_sets_its_keys_left_off_and_the_vault_takes_the_share_it_implies() {
    use faraday_vault::records::field;
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
    let w = app.create().unwrap().built.expect("wallet made");
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
    into_vault(&mut app, 0, Thing::Share(1));
    no_paper_description(&mut app);
    assert_eq!(app.backup().unwrap().answers.omit, 1);
    let (m, n) = faraday_core::wallet::Session::quorum(&app.session.wallets[w]);
    let none_off = faraday_core::wallet_backup::split_share(
        &app.session.wallets[w],
        &faraday_core::wallet_backup::split_plan(n, m, 0)[1],
    )
    .unwrap();
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    // Change the plan opens the board (§19.21 item 11).
    shell.flow(&mut app, F::ChangePlan);
    // The keys left off set as the checklist's slider sets them; the
    // board has no stepper (`docs/BACKUP-FLOW.md` §17.19).
    let omit = Action::Flow(F::Plan(PlanPress::Omit(0)));
    shell.press(&mut app, omit);
    assert_eq!(app.flow().unwrap().wallet.as_ref().unwrap().answers.omit, 0);
    assert!(app.flow().unwrap().adding, "Vault 1's share is another now");
    unlock(&mut app, PHRASE);
    shell.flow(&mut app, F::CopyKey);
    let texts: Vec<String> = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::WALLET_SHARE)
        .map(|(_, r)| r.text(field::SHARE).unwrap_or("").to_string())
        .collect();
    assert_eq!(texts, vec![none_off], "Share 2 with no key left off");
}

/// Place names typed on the flow's Places card before the lock are in
/// each vault's plan record; after the lock no name field is drawn, and
/// the flow's kept state holds no name (§17.7).
#[test]
fn names_typed_before_the_lock_are_in_each_vaults_plan_and_never_in_the_flow() {
    use faraday_vault::records::field;
    const NAME: &str = "Harbour cellar";
    let mut shell = Shell::new();
    let mut app = two_of_three_with_setup(&mut shell, 2);
    no_words(&mut app);
    // Two vaults, as set up.
    vault_off(&mut app, 2);
    let i = app.backup_flows().len() - 1;
    shell.flow(&mut app, F::Show(i));
    // Change the plan opens the board (§19.21 item 11).
    shell.flow(&mut app, F::ChangePlan);
    assert!(offered_down(&mut app, Action::BName(0)), "a name field");
    app.press(Action::BName(0));
    type_text(&mut app, NAME);
    app.event(Event::Key(Key::Enter));
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    for o in 0..2 {
        let names: Vec<String> = app
            .vaults()
            .store()
            .contents_for_tests(app.vaults().open[o].id)
            .of(kind::PLAN)
            .flat_map(|(_, r)| {
                r.fields
                    .iter()
                    .filter(|f| f.number == field::PLAN_PLACE)
                    .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(names.iter().any(|n| n == NAME), "vault {o}: {names:?}");
    }
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let kept = faraday_core::backup_flow::encode(app.backup_flows());
    assert!(
        !kept.windows(NAME.len()).any(|w| w == NAME.as_bytes()),
        "the flow keeps no name"
    );
    // Change the plan opens the board (§19.21 item 11).
    shell.flow(&mut app, F::ChangePlan);
    assert!(
        !offered_down(&mut app, Action::BName(0)),
        "no name field after the lock"
    );
}

/// The flow's plan card has one way on: whatever card is open, one
/// Continue on the screen, at the small panel's size and the desktop's.
#[test]
fn the_flows_plan_card_has_one_continue_at_both_sizes() {
    let c = faraday_core::testkit::compute();
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let display = |app: &mut Faraday, w: u16, h: u16, dpi: u16| {
        app.event(Event::Display(faraday_shell_api::DisplayInfo {
            width: w,
            height: h,
            dpi,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: faraday_shell_api::SecureHardware::None,
            boot: faraday_shell_api::BootState::Unknown,
            memory_mib: None,
        }));
    };
    display(&mut app, 480, 640, 286);
    shell.flow(&mut app, F::ChangePlan);
    for (w, h, dpi) in [(480u16, 640u16, 286u16), (1280, 800, 160)] {
        display(&mut app, w, h, dpi);
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        let cards = faraday_core::plan_cards(&c, &wf.shape, &wf.answers);
        for q in cards {
            if app.flow().unwrap().live.plan_q != Some(q) {
                flow_plan(&mut shell, &mut app, PlanPress::Q(q));
            }
            let _ = app.frame();
            let n = app
                .drawn_texts()
                .iter()
                .filter(|t| t.as_str() == "Continue")
                .count();
            assert!(n <= 1, "{w}x{h}, card {q}: {n} Continue");
        }
    }
}

/// One wallet, one checksum (F35): a 2-of-3 made here, split into shares
/// and rebuilt from a quorum of share files, has the descriptor and the
/// checksum it was made with.
#[test]
fn a_wallet_rebuilt_from_its_share_files_has_the_checksum_it_was_made_with() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, multi_index());
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let w = app.create().unwrap().built.expect("wallet made");
    let wallet = &app.session.wallets[w];
    let made = wallet.policy().to_descriptor_checksummed();
    let (m, n) = faraday_core::wallet::Session::quorum(wallet);
    let split = faraday_core::wallet_backup::split_plan(n, m, 1);
    let files: Vec<(String, Vec<u8>)> = (0..2)
        .map(|j| {
            (
                format!("w-share-{}-of-{n}.txt", j + 1),
                faraday_core::wallet_backup::split_share(wallet, &split[j])
                    .unwrap()
                    .into_bytes(),
            )
        })
        .collect();
    let mut r = process(files.clone(), Vec::new(), Vec::new());
    keep_time(&mut r);
    r.press(Action::RestoreWallet);
    for (name, _) in &files {
        let i = r.inbox().iter().position(|f| &f.name == name).unwrap();
        r.press(Action::RShare(i));
    }
    r.press(Action::RRebuild);
    let rw = r.restore().and_then(|x| x.wallet).expect("rebuilt");
    assert_eq!(
        r.session.wallets[rw].policy().to_descriptor_checksummed(),
        made
    );
}

/// A wallet written with `'` reads with the checksum of its `h` form.
#[test]
fn a_wallet_written_with_apostrophes_has_the_checksum_of_its_h_form() {
    let c = faraday_core::testkit::compute();
    let h = testkit::savings().split('#').next().unwrap().to_string();
    let apostrophe = apostrophes(&h);
    assert_ne!(h, apostrophe);
    let a = faraday_core::wallet::read_wallet(&c, &apostrophe).unwrap();
    let b = faraday_core::wallet::read_wallet(&c, &h).unwrap();
    assert_eq!(a.checksum(), b.checksum());
    assert_eq!(a.to_descriptor_checksummed(), b.to_descriptor_checksummed());
}

/// `text` with each hardened step of its keys' origins written `'`.
fn apostrophes(text: &str) -> String {
    let text = text.split('#').next().unwrap_or(text);
    let mut out = String::new();
    let mut origin = false;
    for c in text.chars() {
        match c {
            '[' => origin = true,
            ']' => origin = false,
            'h' if origin => {
                out.push('\'');
                continue;
            }
            _ => {}
        }
        out.push(c);
    }
    out
}

/// A vault whose wallet (type 3) and plan (type 11) were kept with `'`
/// opens with the wallet loaded and its plan attached.
#[test]
fn a_vault_kept_with_apostrophes_opens_with_its_plan_attached() {
    let c = testkit::compute();
    use faraday_vault::Record;
    use faraday_vault::records::field;
    let desc = apostrophes(&testkit::savings());
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    let w = app
        .session
        .add_wallet(
            &c,
            "Savings",
            &desc,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let shape = app.plan_shape_now(w);
    let mut answers = Answers::preset(&shape, Preset::Paper);
    answers.set_places(&shape, 4);
    let text = answers.to_text();
    let records = vec![
        Record::new(kind::WALLET)
            .with(field::WALLET, desc.as_bytes())
            .with(field::WALLET_NAME, b"Savings"),
        Record::new(kind::PLAN)
            .with(field::PLAN_WALLET, desc.as_bytes())
            .with(field::PLAN_ANSWERS, text.as_bytes()),
    ];
    let file = vault_of("Savings", PHRASE, 9, records);
    let mut next = process(vec![("old.kdbx".into(), file)], Vec::new(), Vec::new());
    keep_time(&mut next);
    next.press(Action::Nav(Screen::Vaults));
    next.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    unlock(&mut next, PHRASE);
    let loaded = next
        .session
        .wallets
        .iter()
        .position(|x| x.checksum() == app.session.wallets[w].checksum());
    let loaded = loaded.unwrap_or_else(|| {
        next.press(Action::Vault(faraday_core::vault::VaultAction::LoadChosen(
            0,
        )));
        next.session.wallets.len().saturating_sub(1)
    });
    assert_eq!(
        next.session.wallets[loaded]
            .policy()
            .to_descriptor_checksummed(),
        app.session.wallets[w].policy().to_descriptor_checksummed()
    );
    next.press(Action::Backup(loaded));
    let b = next.backup().unwrap();
    assert_eq!(b.q, None, "the plan is attached");
    assert_eq!(b.answers.places, 4);
}

// ---- the round takes out only what the flow put in (§17) ----

/// After the lock, taking Key 1 out of the existing vault that held it
/// before the flow is refused, and the sealed copy still holds it.
#[test]
fn a_key_an_existing_vault_held_before_the_flow_is_never_taken_out() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let main = vault_with_key("Main", PHRASE, 0x41, 1);
    shell.disk_mut("S1").files = vec![("main.kdbx".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "main.kdbx"));
    if let Some(k) = k {
        shell.flow(&mut app, F::PickFile(0, k));
    }
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert_eq!(step(&app), Step::Vaults);
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Create, "{:?}", unlock_error(&app));
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    no_paper_description(&mut app);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    assert_eq!(contents(&app, 0).of(kind::KEY).count(), 1);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("Vault 1 held this before; take it out from the vault's contents")
    );
    {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        assert!(
            wf.answers.vault_seeds(&wf.shape, 0).contains(&0),
            "the plan stays"
        );
    }
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = kdbx(&shell, "S1").expect("Vault 1 on S1");
    let c = kdbx_records(&sealed, PHRASE);
    assert_eq!(c.of(kind::KEY).count(), 1, "Key 1 stays");
}

/// Two wallets that share a key in one vault: taking the key out for one
/// of them after the lock is refused, and the key stays.
#[test]
fn a_key_another_wallet_in_the_vault_uses_is_never_taken_out() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    let (mut app, a, _) = single_key(&mut shell, false);
    // Another wallet of the same key, saved into the same vault.
    let k = app.session.keys.len() - 1;
    let text = NewKind::Taproot
        .key_text(&c, app.session.keys.master_for_tests(k))
        .unwrap();
    let b = app
        .session
        .add_wallet(
            &c,
            "Other",
            &format!("tr({text}/<0;1>/*)"),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_ne!(a, b);
    app.press(Action::Backup(b));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    no_paper_description(&mut app);
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    assert_eq!(contents(&app, 0).of(kind::WALLET).count(), 2);
    // Back to the first wallet's checklist, and its lock.
    app.press(Action::Backup(a));
    app.press(Action::BChecklist);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("Another wallet in Vault 1 uses this key; take it out from the vault's contents")
    );
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    assert!(
        wf.answers.vault_seeds(&wf.shape, 0).contains(&0),
        "the key stays"
    );
}

// ---- the final review (§17) ----

/// A key held for its sticks never ends on none: after the lock, a seed's
/// file is not added (it is made before the lock), the seed stays in its
/// vault, and the key is released only once the sealed copies hold it.
#[test]
fn a_held_seed_is_never_left_on_no_stick_after_the_lock() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    flow_keep(&mut shell, &mut app, 1, Row::File, &[0]);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        assert!(wf.answers.vault_seeds(&wf.shape, 0).contains(&0));
        assert!(
            wf.answers.seeds_as_files(&c, &wf.shape).is_empty(),
            "no seed file on the plan"
        );
    }
    let fp = faraday_bip::keys::Fingerprint(fp);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    for d in ["S1", "S2"] {
        let sealed = kdbx(&shell, d).expect("Vault 1's copy");
        let c = kdbx_records(&sealed, PHRASE);
        assert_eq!(c.of(kind::KEY).count(), 1, "{d} holds the key");
    }
    assert!(
        !app.key_held(&c, fp),
        "released once its copies are written"
    );
}

/// A vault that holds any record of another wallet keeps its key records:
/// a hidden wallet on the same words with a passphrase kept apart is one
/// the flow cannot tell from the words alone.
#[test]
fn a_key_is_never_taken_out_of_a_vault_another_wallets_record_is_in() {
    let c = testkit::compute();
    use faraday_vault::Record;
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_with(&mut shell, false, true);
    // The hidden wallet: the same words with a passphrase; its
    // description in Vault 1.
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    bip39_passphrase(&mut app, "hidden");
    app.press(Action::EntryAdd);
    let k = app.session.keys.len() - 1;
    let text = NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(k))
        .unwrap();
    let hidden = format!("wpkh({text}/<0;1>/*)");
    {
        let id = app.vaults().open[0].id;
        faraday_core::testkit::vault_edit(&mut app, id, |s| {
            s.insert(
                id,
                Record::new(kind::WALLET)
                    .with(field::WALLET, hidden.as_bytes())
                    .with(field::WALLET_NAME, b"Hidden"),
            )
        });
    }
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    // Words at Place 1, then the seed out of Vault 1.
    flow_keep(&mut shell, &mut app, 0, Row::Words, &[0]);
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("Another wallet in Vault 1 uses this key; take it out from the vault's contents")
    );
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    assert!(
        wf.answers.vault_seeds(&wf.shape, 0).contains(&0),
        "the key stays"
    );
}

/// After the lock, Keys left off each share does not change while a share
/// is on paper or in a file: those were made with it as it is.
#[test]
fn keys_left_off_each_share_stay_after_the_lock_while_a_share_is_in_a_file() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
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
    // Share 2 in Vault 1 and as a file, at the place of Vault 1's stick.
    let p = vault_place(&app, 0);
    description_out_of(&mut app, 0);
    no_paper_description(&mut app);
    into_vault(&mut app, 0, Thing::Share(1));
    match files_stick(&app, p) {
        Some(s) => put(&mut app, Thing::Share(1), Zone::Files(p, s)),
        None => put(&mut app, Thing::Share(1), Zone::NewStick(p)),
    }
    no_paper_description(&mut app);
    assert_eq!(app.backup().unwrap().answers.omit, 1);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    // The keys left off, on the flow's board's stepper.
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::Plan(PlanPress::Omit(0)));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert_eq!(app.flow().unwrap().wallet.as_ref().unwrap().answers.omit, 1);
}

// ---- the re-check: after the lock, each stick's files from the plan ----

/// A 2-of-3 with no vault: the three keys loaded, Paper only, nothing on
/// paper, at the checklist.
fn two_of_three_paper(shell: &mut Shell, n: usize) -> Faraday {
    sticks(shell, n);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, multi_index());
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    no_words(&mut app);
    no_paper_description(&mut app);
    app
}

/// A seed file taken off a place's stick of files after the lock leaves
/// that stick's list: the stick is written with what the plan keeps
/// there, and nothing stale holds the flow.
#[test]
fn a_seed_file_taken_off_after_the_lock_is_refused_and_its_stick_keeps_its_files() {
    let mut shell = Shell::new();
    let mut app = two_of_three_paper(&mut shell, 2);
    files_at(&mut app, 0, &[0, 1]);
    files_at(&mut app, 1, &[2]);
    let names: Vec<String> = backup_shape(&app)
        .seeds
        .iter()
        .map(|s| s.name.to_lowercase())
        .collect();
    app.press(Action::BChecklist);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(
        let_seed_files_out(&mut app),
        0,
        "held for at the drop: none at the lock"
    );
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    // Files stay as made after the lock (§17.8): refused, and Place 1's
    // stick keeps both files.
    flow_keep(&mut shell, &mut app, 0, Row::File, &[1]);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let f = app.flow().unwrap();
    assert_eq!(f.slots[0].target, Target::PlaceFiles(0));
    assert_eq!(f.slots[0].files.len(), 2, "{:?}", f.slots[0].files);
    write_sticks(&mut shell, &mut app, &["S1", "S2"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let words = |d: &str| -> Vec<String> {
        names_on(&shell, d)
            .into_iter()
            .filter(|n| seed_words_file(n))
            .collect()
    };
    assert_eq!(words("S1").len(), 2);
    assert!(words("S1").iter().any(|n| n.contains(&names[0])));
    assert!(words("S1").iter().any(|n| n.contains(&names[1])));
    assert!(words("S2")[0].contains(&names[2]));
}

fn names_on(shell: &Shell, d: &str) -> Vec<String> {
    shell.disk(d).files.iter().map(|f| f.0.clone()).collect()
}

/// Types back the copy of the key in Create's slot 0 on the checklist's
/// copy item.
fn type_back_the_copy(app: &mut Faraday) {
    let c = testkit::compute();
    let k = app.session.keys.len() - 1;
    let words = app
        .session
        .keys
        .reveal(k, faraday_keys::Reveal::Words)
        .to_string();
    if app.backup().unwrap().open != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BCheck);
    faraday_core::testkit::type_words(app, &words);
    assert!(
        app.backup_item_done_now(&c, Item::Copy(0)),
        "the copy is checked"
    );
}

/// A key made here, in Vault 1, as a file at Place 2 and in words at
/// Place 1: after the lock, with the file off the plan, emptying Vault 1
/// leaves it on no stick and is refused.
#[test]
fn a_held_key_is_not_left_on_a_stale_file_after_the_lock() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    app.press(Action::Nav(Screen::Create));
    let fp = new_key(&mut app);
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_paper_description(&mut app);
    for p in 1..backup_places(&app) {
        words_off_at(&mut app, p);
    }
    files_at(&mut app, 1, &[0]);
    app.press(Action::BChecklist);
    app.press(Action::BNext(bstep::BLANK));
    type_back_the_copy(&mut app);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(
        let_seed_files_out(&mut app),
        0,
        "held for at the drop: none at the lock"
    );
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    let s = (0..plan::sticks_at(&backup_map(&app), 1))
        .find(|&s| plan::holds_at(&backup_map(&app), At::Stick(1, s)).contains(&What::SeedFile(0)))
        .expect("the seed's file at Place 2");
    flow_board(
        &mut shell,
        &mut app,
        BoardPress::Off(Zone::Files(1, s), Thing::Seed(0)),
    );
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let p = flow_place_of(&app, 0);
    // The file stays as made, so Vault 1 may be emptied: the key stays
    // held until the stick of files that holds it is written.
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    let f = app.flow().unwrap();
    let wf = f.wallet.as_ref().unwrap();
    assert!(!wf.answers.vault_seeds(&wf.shape, 0).contains(&0));
    assert!(
        f.slots.iter().any(
            |s| s.target == Target::PlaceFiles(1) && s.files.iter().any(|n| seed_words_file(n))
        ),
        "kept on the stick of files"
    );
    let held = &app.flow().unwrap().wallet.as_ref().unwrap().held;
    let fp = faraday_core::wallet::fp_text(faraday_bip::keys::Fingerprint(fp));
    assert!(
        held.iter().any(|h| h.eq_ignore_ascii_case(&fp)),
        "held until that stick is written"
    );
}

/// A 2-of-3 in shares, Share 2 in Vault 1 and as a file at the place of
/// Vault 1's stick, saved, locked and restarted: (app, that place).
fn shares_locked(shell: &mut Shell) -> (Faraday, usize) {
    sticks(shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 1);
    bring_in_all(shell, &mut app, &["S1"]);
    create_vault(shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
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
    let p = vault_place(&app, 0);
    description_out_of(&mut app, 0);
    no_paper_description(&mut app);
    into_vault(&mut app, 0, Thing::Share(1));
    match files_stick(&app, p) {
        Some(s) => put(&mut app, Thing::Share(1), Zone::Files(p, s)),
        None => put(&mut app, Thing::Share(1), Zone::NewStick(p)),
    }
    no_paper_description(&mut app);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    (restart(shell, &mut app), p)
}

/// The sticks of the flow on screen in its order, as disks: the stick
/// read for Vault 1 is S1, the others new sticks after it.
fn disks_in_order(app: &Faraday) -> Vec<&'static str> {
    let mut fresh = ["S2", "S3"].into_iter();
    app.flow()
        .unwrap()
        .slots
        .iter()
        .map(|s| {
            if matches!(s.target, Target::Place(_) | Target::PlaceVault(..)) {
                "S1"
            } else {
                fresh.next().unwrap_or("S3")
            }
        })
        .collect()
}

/// After the lock, a share moved and "As a file" ticked at another place:
/// each stick of files is written with the shares' files the plan puts
/// there, and the vault takes the share now planned.
#[test]
fn shares_files_follow_the_plan_after_the_lock() {
    let mut shell = Shell::new();
    let (mut app, p) = shares_locked(&mut shell);
    let q = (0..3).find(|&q| q != p).unwrap();
    let files_at = |app: &Faraday, at: usize| -> Vec<String> {
        app.flow()
            .unwrap()
            .slots
            .iter()
            .find(|s| s.target == Target::PlaceFiles(at))
            .map(|s| s.files.clone())
            .unwrap_or_default()
    };
    let before = files_at(&app, p);
    assert!(
        before.iter().any(|n| n.contains("-share-2-of-3")),
        "{before:?}"
    );
    // After the lock the shares' files stay as made (§17.8): moving a
    // share, or a file at another place, is refused.
    shell.flow(&mut app, F::ChangePlan);
    let q_files = files_stick(&app, q).map_or(Zone::NewStick(q), |s| Zone::Files(q, s));
    for (t, z) in [(Thing::Share(2), Zone::Paper(p)), (Thing::Whole, q_files)] {
        flow_put(&mut shell, &mut app, t, z);
        let press = (t, z);
        assert_eq!(
            app.toast_text(),
            Some(
                "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
            ),
            "{press:?}"
        );
    }
    assert_eq!(files_at(&app, p), before);
    assert!(
        app.flow()
            .unwrap()
            .slots
            .iter()
            .all(|s| s.target != Target::PlaceFiles(q))
    );
    // The sticks are written with the files made.
    let order = disks_in_order(&app);
    write_sticks(&mut shell, &mut app, &order);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let k = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .position(|s| s.target == Target::PlaceFiles(p))
        .unwrap();
    let on = names_on(&shell, order[k]);
    assert!(on.iter().any(|n| n.contains("-share-2-of-3")), "{on:?}");
}

/// After the lock, with a share in a file, Whole and back cannot change
/// the split: whole or in shares is fixed, as the keys left off are.
#[test]
fn whole_or_in_shares_is_fixed_after_the_lock_while_a_share_is_in_a_file() {
    let mut shell = Shell::new();
    let (mut app, _) = shares_locked(&mut shell);
    shell.flow(&mut app, F::ChangePlan);
    // The share's file taken off: refused, so the split stays.
    let (p, s) = (0..board_places(&app))
        .find_map(|p| {
            (0..plan::sticks_at(&backup_map(&app), p)).find_map(|s| {
                plan::holds_at(&backup_map(&app), At::Stick(p, s))
                    .iter()
                    .any(|w| matches!(w, What::ShareFile(..)))
                    .then_some((p, s))
            })
        })
        .expect("a share's file");
    flow_board(
        &mut shell,
        &mut app,
        BoardPress::Off(Zone::Files(p, s), Thing::Share(1)),
    );
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let a = &app.flow().unwrap().wallet.as_ref().unwrap().answers;
    assert!(a.split);
    assert_eq!(a.omit, 1);
}

/// A vault that keeps another wallet's bare record of the words: each
/// round adds this wallet's record with its passphrase once, not again.
#[test]
fn the_round_adds_a_planned_key_record_once_beside_another_of_its_words() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let main = vault_with_key("Main", PHRASE, 0x41, 1);
    shell.disk_mut("S1").files = vec![("main.kdbx".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "main.kdbx"));
    if let Some(k) = k {
        shell.flow(&mut app, F::PickFile(0, k));
    }
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    unlock(&mut app, PHRASE);
    // The same words with a passphrase: this wallet's key.
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    bip39_passphrase(&mut app, "kept here");
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint.0;
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    no_paper_passphrases(&mut app);
    no_paper_description(&mut app);
    description_out_of(&mut app, 0);
    into_vault(&mut app, 0, Thing::Pass(0));
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    let keys = |app: &Faraday| -> (usize, usize) {
        let all: Vec<bool> = app
            .vaults()
            .store()
            .contents_for_tests(app.vaults().open[0].id)
            .of(kind::KEY)
            .map(|(_, r)| r.field(field::KEY_PASSPHRASE).is_some())
            .collect();
        (all.len(), all.iter().filter(|&&p| p).count())
    };
    assert_eq!(keys(&app), (2, 1), "the bare record and this wallet's");
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    // Two rounds: the description into Vault 1, then out of it.
    for _ in 0..2 {
        shell.flow(&mut app, F::ChangePlan);
        let p = flow_place_of(&app, 0);
        let _ = p;
        flow_whole_in_vault(&mut shell, &mut app, 0);
        assert!(app.flow().unwrap().adding, "{:?}", app.toast_text());
        if app.vaults().open.is_empty() {
            unlock(&mut app, PHRASE);
        }
        shell.flow(&mut app, F::CopyKey);
        assert_eq!(keys(&app), (2, 1), "{:?}", app.toast_text());
        shell.flow(&mut app, F::MadeContinue);
    }
}

// ---- after the lock only vaults change (§17.8) ----

/// A refused edit after the lock loads nothing: no wallet, no receive
/// address; "As a file" at a place with no vault open is one.
#[test]
fn a_file_added_after_the_lock_is_refused_and_loads_no_wallet() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert!(app.session.wallets.is_empty());
    shell.flow(&mut app, F::ChangePlan);
    flow_whole_file_at(&mut shell, &mut app, 0);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert!(app.session.wallets.is_empty(), "no wallet is loaded");
    assert!(
        app.flow()
            .unwrap()
            .slots
            .iter()
            .all(|s| s.target != Target::PlaceFiles(0))
    );
}

/// The Done card's unlock loads the vault's wallet and key, the key no
/// longer held once every stick holding it is written.
#[test]
fn the_done_landing_loads_the_vaults_keys_released() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    shell.flow(&mut app, F::UnlockDone(0));
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    unlock(&mut app, PHRASE);
    let fp = faraday_bip::keys::Fingerprint(fp);
    assert!(
        app.session.keys.all().any(|k| k.fingerprint == fp),
        "the key loaded from the vault"
    );
    assert!(!app.session.wallets.is_empty(), "its wallet too");
    assert!(!app.key_held(&c, fp), "released");
}

/// An edit after the lock that only moves a vault's sticks leaves every
/// stick of files' list as it was made, the account key files with it.
#[test]
fn a_stick_of_files_keeps_its_list_after_a_vault_edit() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let files = |app: &Faraday| -> Vec<String> {
        app.flow()
            .unwrap()
            .slots
            .iter()
            .find(|s| s.target == Target::PlaceFiles(1))
            .map(|s| s.files.clone())
            .unwrap()
    };
    let before = files(&app);
    assert!(!before.is_empty());
    shell.flow(&mut app, F::ChangePlan);
    flow_board(&mut shell, &mut app, BoardPress::AddPlace);
    flow_put(&mut shell, &mut app, Thing::Vault(0), Zone::Card(2));
    assert!(
        app.flow()
            .unwrap()
            .slots
            .iter()
            .any(|s| s.target == Target::Place(2)),
        "Vault 1's stick at Place 3"
    );
    assert_eq!(files(&app), before, "the list stays as made");
}

/// A written stick of files is not sent back after the lock; with the
/// vault unlocked again, Back up's change of its files makes it unwritten,
/// to be written again, while a vault's stick that did not change stays
/// written.
#[test]
fn a_written_stick_of_files_is_never_sent_back_after_the_lock() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    let w = app.create().unwrap().built.unwrap();
    let name = app.session.wallets[w].name.clone();
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    // Over two places, as a plan saved before one place per key
    // (`docs/BACKUP-FLOW.md` §17.19).
    preset_two_places(&mut app, 2);
    // Vault 1 at both places, the description in it and as files at
    // Place 1, nothing else.
    no_words(&mut app);
    stick_also_at(&mut app, 0, 0);
    no_paper_description(&mut app);
    description_file_at(&mut app, 0);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        vec![Target::Place(0), Target::PlaceFiles(0), Target::Place(1)]
    );
    write_sticks(&mut shell, &mut app, &["S1", "S2"]);
    assert!(
        app.flow().unwrap().slots[1].written.is_some(),
        "the files written"
    );
    // Locked: its files stay.
    shell.flow(&mut app, F::ChangePlan);
    flow_whole_file_at(&mut shell, &mut app, 0);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert!(app.flow().unwrap().slots[1].written.is_some());
    // A vault unlocked again changes nothing of it: the text form on
    // Back up, the checklist and Lock and write are refused, and every
    // stick keeps its written mark.
    load_first_vault(&mut app, PHRASE);
    let w = app
        .session
        .wallets
        .iter()
        .position(|x| x.name == name)
        .expect("the wallet, from the vault");
    app.press(Action::Backup(w));
    let before: Vec<bool> = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .map(|s| s.written.is_some())
        .collect();
    app.press(Action::BAnswer(qrow::FORM, form::TEXT as u8));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    app.press(Action::BChecklist);
    assert_eq!(
        app.toast_text(),
        Some("This backup is past its lock; finish the writes, then Back up again")
    );
    shell.flow(&mut app, F::WalletLock);
    let after: Vec<bool> = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .map(|s| s.written.is_some())
        .collect();
    assert_eq!(after, before, "written marks unchanged");
    assert!(app.flow().unwrap().slots[1].written.is_some());
}

/// A vault that held the bare words before the backup: this wallet's key
/// with its passphrase, taken out of its passphrase after the lock, leaves
/// the sealed copy with the earlier bare record and no record with the
/// passphrase.
#[test]
fn a_passphrase_taken_out_leaves_the_earlier_bare_record_alone() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let main = vault_with_key("Main", PHRASE, 0x41, 1);
    shell.disk_mut("S1").files = vec![("main.kdbx".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "main.kdbx"));
    if let Some(k) = k {
        shell.flow(&mut app, F::PickFile(0, k));
    }
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    unlock(&mut app, PHRASE);
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    bip39_passphrase(&mut app, "kept here");
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint.0;
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    no_paper_passphrases(&mut app);
    no_paper_description(&mut app);
    into_vault(&mut app, 0, Thing::Pass(0));
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    let p = flow_place_of(&app, 0);
    let _ = p;
    flow_pass_in_vault(&mut shell, &mut app, 0, 0);
    assert!(app.flow().unwrap().adding, "{:?}", app.toast_text());
    unlock(&mut app, PHRASE);
    shell.flow(&mut app, F::CopyKey);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::LockWrite);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = kdbx(&shell, "S1").expect("Vault 1 on S1");
    let c = kdbx_records(&sealed, PHRASE);
    let keys: Vec<bool> = c
        .of(kind::KEY)
        .map(|(_, r)| r.field(field::KEY_PASSPHRASE).is_some())
        .collect();
    assert_eq!(keys, vec![false], "the earlier bare record alone");
}

// ---- after the lock, files and paper never change in that backup ----

/// F1: with Vault 1 unlocked after the lock, "As a file" and words by hand
/// on the flow's plan card are refused, and no stick is added.
#[test]
fn files_and_words_stay_as_made_after_the_lock_with_a_vault_unlocked() {
    let c = faraday_core::testkit::compute();
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    load_first_vault(&mut app, PHRASE);
    assert!(!app.vaults().open.is_empty(), "Vault 1 is unlocked");
    let slots = app.flow().unwrap().slots.len();
    shell.flow(&mut app, F::ChangePlan);
    flow_whole_file_at(&mut shell, &mut app, 0);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert_eq!(app.flow().unwrap().slots.len(), slots, "no new stick");
    // Words by hand at Place 1: refused too.
    flow_keep(&mut shell, &mut app, 0, Row::Words, &[0]);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    assert_eq!(wf.answers.row_seeds(&c, &wf.shape, 0, Row::Words), None);
}

/// After the lock, another vault holding the same key and wallet,
/// unlocked, with a chart edit, the checklist and Lock and write: the
/// flow's plan, its vaults and its written marks stay as they were.
#[test]
fn another_open_vault_never_replaces_a_backup_past_its_lock() {
    let c = testkit::compute();
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::ChartAction as C;
    use faraday_vault::Record;
    use faraday_vault::records::{self, field};
    let mut shell = Shell::new();
    let (mut app, w, _) = single_key_ready(&mut shell);
    let desc = app.session.wallets[w].policy().to_descriptor_checksummed();
    let name = app.session.wallets[w].name.clone();
    shell.flow(&mut app, F::WalletLock);
    // A second vault, of another passphrase, with the same key and wallet.
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    let key_records = vec![
        Record::new(kind::KEY).with(field::KEY, &records::words_payload(&m)),
        Record::new(kind::WALLET)
            .with(field::WALLET, desc.as_bytes())
            .with(field::WALLET_NAME, name.as_bytes()),
    ];
    let second = vault_of("Second", PHRASE_2, 0x77, key_records);
    shell.boxes.0.push(("second.kdbx".into(), second));
    let mut app = restart(&mut shell, &mut app);
    let flow_before = app.flow().unwrap().clone();
    // The second vault unlocked.
    app.press(Action::Nav(Screen::Vaults));
    for n in 0..4 {
        if !app.vaults().open.is_empty() {
            break;
        }
        app.press(Action::Vault(faraday_core::vault::VaultAction::Open(n)));
        faraday_core::testkit::vault_speed(&mut app, Some(180));
        unlock(&mut app, PHRASE_2);
    }
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Vault(faraday_core::vault::VaultAction::LoadChosen(
        0,
    )));
    let w = app
        .session
        .wallets
        .iter()
        .position(|x| x.name == name)
        .expect("the wallet, from the second vault");
    app.press(Action::Backup(w));
    // A chart edit: the description's files at Place 1.
    let press = Press::Loaded(w);
    app.press(Action::Chart(C::Edit(
        press,
        plan::Edit::Add(At::Place(0), What::WalletFile(plan::form::QR)),
    )));
    app.press(Action::Chart(C::Confirm(press)));
    app.press(Action::BChecklist);
    shell.flow(&mut app, F::WalletLock);
    let f = app.flow().unwrap();
    let wf = f.wallet.as_ref().unwrap();
    assert_eq!(
        wf.answers,
        flow_before.wallet.as_ref().unwrap().answers,
        "the plan stays"
    );
    let salts = |f: &BackupFlow| f.plan.vaults.iter().map(|v| v.salt).collect::<Vec<_>>();
    assert_eq!(salts(f), salts(&flow_before), "its vaults stay");
    let marks = |f: &BackupFlow| {
        f.slots
            .iter()
            .map(|s| (s.target, s.written.is_some()))
            .collect::<Vec<_>>()
    };
    assert_eq!(marks(f), marks(&flow_before), "its written marks stay");
}

/// Once a backup is done, Back up again on the wallet is a new backup: a
/// file added at a place is made by its checklist and written by its Lock
/// and write.
#[test]
fn back_up_again_after_done_adds_a_file_and_writes_it() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    let mut d = Disk::empty("S4", "KEYSTICK");
    d.serial = Some(4);
    shell.add(d);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    shell.flow(&mut app, F::UnlockDone(0));
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BPlan);
    description_file_at(&mut app, 0);
    app.press(Action::BChecklist);
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert!(targets.contains(&Target::PlaceFiles(0)), "{targets:?}");
    let mut app = restart(&mut shell, &mut app);
    let k = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .position(|s| s.target == Target::PlaceFiles(0))
        .unwrap();
    let mut order = vec!["S1", "S2", "S3"];
    order.insert(k, "S4");
    write_sticks(&mut shell, &mut app, &order);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    assert!(
        names(&shell, "S4")
            .iter()
            .any(|n| n.ends_with("-descriptor.txt")),
        "{:?}",
        names(&shell, "S4")
    );
}

/// A single key with a passphrase and two vaults set up before it: the
/// seed's words in Vault 1, its passphrase alone in Vault 2
/// (`docs/BACKUP-FLOW.md` §17.12 item 4). Vault 1's item saves the key
/// record of the words alone; Vault 2's a passphrase record naming the
/// key, and nothing else of it.
#[test]
fn a_passphrase_alone_in_a_vault_is_saved_as_a_passphrase_record() {
    let c = testkit::compute();
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 2);
    bring_in_all(&mut shell, &mut app, &["S1", "S2"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words("summer"));
    bip39_passphrase(&mut app, "TREZOR");
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp.0));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    no_paper_passphrases(&mut app);
    no_paper_description(&mut app);
    // Vault 1 at Place 1 with the words; Vault 2 at Place 2 with the
    // passphrase, dropped there as the board drops it.
    let shape = app.plan_shape_now(app.backup().unwrap().wallet);
    {
        let a = faraday_core::testkit::backup_answers_mut(&mut app);
        let mut m = plan::map(&c, &shape, a);
        for v in 0..2 {
            for w in [What::Seed(0), What::SeedPassphrase(0)] {
                m = plan::without(&m, At::Vault(v), w);
            }
            for p in 0..plan::places_of(&m) {
                m = plan::without(&m, At::Place(p), What::VaultStick(v));
            }
        }
        m = plan::with_vault_stick(&m, 0, 0);
        m = plan::with_vault_stick(&m, 1, 1);
        m = plan::with_added(&m, At::Vault(0), What::Seed(0));
        m = plan::with_added(&m, At::Vault(1), What::Passphrase(0));
        a.set_map(m);
    }
    let m = plan::map(&c, &shape, &app.backup().unwrap().answers);
    let held = plan::holds_at(&m, At::Vault(0));
    assert!(held.contains(&What::Seed(0)), "{held:?}");
    assert!(!held.contains(&What::SeedPassphrase(0)), "{held:?}");
    let held = plan::holds_at(&m, At::Vault(1));
    assert!(held.contains(&What::PassphraseSealed(0)), "{held:?}");
    assert!(!held.contains(&What::Seed(0)), "{held:?}");
    assert_eq!(app.backup_kept_nowhere(&c), None);
    app.press(Action::BChecklist);
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
    save_vaults(&mut app);
    assert_eq!(app.vaults().open.len(), 2);
    let with = |k: u8| {
        app.vaults()
            .open
            .iter()
            .find(|o| app.vaults().store().contents_for_tests(o.id).of(k).count() > 0)
            .map(|o| app.vaults().store().contents_for_tests(o.id).to_vec())
            .expect("a vault with it")
    };
    let words = with(kind::KEY);
    let keys: Vec<_> = words.of(kind::KEY).map(|(_, r)| r.clone()).collect();
    assert_eq!(keys.len(), 1);
    assert!(
        keys[0].field(field::KEY_PASSPHRASE).is_none(),
        "the words alone"
    );
    assert_eq!(words.of(kind::PASSPHRASE).count(), 0);
    let apart = with(kind::PASSPHRASE);
    assert_eq!(apart.of(kind::KEY).count(), 0, "no seed in Vault 2");
    let (_, r) = apart.of(kind::PASSPHRASE).next().expect("the record");
    assert_eq!(r.field(field::PASSPHRASE_KEY), Some(&fp.0[..]));
    assert_eq!(r.text(field::PASSPHRASE), Some("TREZOR"));
}

/// A drawn string and its box in pixels.
type TextBox = (String, (i32, i32, i32, i32));

/// Whether the row `row` of the sticks card is ticked with the label of
/// the stick written: the label drawn on its line.
fn stick_row_written(boxes: &[TextBox], row: &str, label: &str) -> bool {
    let (_, (_, y, _, _)) = boxes
        .iter()
        .find(|(t, _)| t == row)
        .unwrap_or_else(|| panic!("no row {row:?} in {boxes:?}"));
    boxes
        .iter()
        .any(|(t, (_, by, _, _))| t == label && (by - y).abs() < 8)
}

#[test]
fn a_sticks_files_written_show_on_the_sticks_card_on_the_next_frame() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    // Back to the checklist, its sticks card open: the first frame ticks
    // each vault's stick with the stick it was written to.
    shell.flow(&mut app, F::UnlockDone(0));
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BNext(bstep::ENVELOPE));
    app.press(Action::BStep(bstep::STICKS));
    app.settle();
    let boxes = app.drawn_text_boxes();
    for row in ["Vault 1 (Place 1)", "Vault 1 (Place 2)"] {
        assert!(
            stick_row_written(&boxes, row, "KEYSTICK"),
            "{row}: {boxes:?}"
        );
    }
    // The copy on Place 1's stick deleted on the stick: the very next
    // frame shows that stick not written, the other still written, and
    // the lock that writes it again.
    let name = shell.disk("S1").files[0].0.clone();
    shell.plug(&mut app, "S1");
    app.storage(faraday_core::StorageEvent::Wiped {
        stick: "S1".into(),
        name,
    });
    shell.pump(&mut app);
    app.press(Action::Cancel);
    app.settle();
    let boxes = app.drawn_text_boxes();
    assert!(
        !stick_row_written(&boxes, "Vault 1 (Place 1)", "KEYSTICK"),
        "{boxes:?}"
    );
    assert!(
        stick_row_written(&boxes, "Vault 1 (Place 2)", "KEYSTICK"),
        "{boxes:?}"
    );
    assert!(app.offers(Action::Flow(F::WalletLock)), "Lock and write");
}

/// The vault question, in Make a new wallet and in Restore, says under it
/// that vaults already on sticks are brought in there, not later
/// (`docs/BACKUP-FLOW.md` §17.21 item 5).
#[test]
fn the_vault_question_says_vaults_on_sticks_are_brought_in_here() {
    const LINE: &str = "Vaults already on sticks are brought in here, not later.";
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    create_single_key(&mut app);
    if app.create().unwrap().open != Some(cstep::WHERE) {
        app.press(Action::CStep(cstep::WHERE));
    }
    assert!(has(&mut app, "Will you use vaults as part of this backup?"));
    assert!(has(&mut app, LINE), "Make a new wallet");
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.press(Action::RestoreWallet);
    if app.restore().unwrap().open != Some(faraday_core::rstep::WHERE) {
        app.press(Action::RStep(faraday_core::rstep::WHERE));
    }
    assert!(has(&mut app, "Will you use vaults as part of this backup?"));
    assert!(has(&mut app, LINE), "Restore");
}

/// Change the plan, from a wallet flow's cards, opens the flow's own
/// board, as it was left (`docs/BACKUP-FLOW.md` §17.21 item 11).
#[test]
fn change_the_plan_from_a_wallet_flow_opens_its_board() {
    let c = faraday_core::testkit::compute();
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let before = {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        plan::map(&c, &wf.shape, &wf.answers)
    };
    shell.flow(&mut app, F::ChangePlan);
    assert_eq!(
        app.flow().unwrap().live.plan_q,
        Some(faraday_core::qstep::BOARD)
    );
    let (shape, a) = app.board_plan().expect("the board is on screen");
    assert_eq!(plan::map(&c, &shape, &a), before, "as it was left");
    let _ = app.frame();
    assert!(app.offers(Action::Board(BoardPress::Zone(Zone::Paper(0)))));
}

/// NEW-WALLET §17 (F55, F98): with the vaults set up before the keys, Check's
/// Path Change takes the wallet back and the remade wallet, at another
/// account, is the one the backup is for.
#[test]
fn a_wallet_taken_back_from_check_after_the_vault_setup_backs_up_as_remade() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    let first = app.create().unwrap().built.expect("wallet made");
    let first_sum = app.session.wallets[first].checksum().to_string();
    let wallets = app.session.wallets.len();
    let _ = app.frame();
    assert!(
        app.offers(Action::CPathChange),
        "Check's Path offers Change"
    );
    app.press(Action::CPathChange);
    assert_eq!(
        app.session.wallets.len(),
        wallets - 1,
        "the wallet is taken back"
    );
    app.press(Action::CAccount(1));
    app.press(Action::CNext(cstep::KEYS));
    let w = app.create().unwrap().built.expect("wallet made again");
    assert_eq!(app.session.wallets.len(), wallets, "one wallet, not two");
    let sum = app.session.wallets[w].checksum().to_string();
    assert_ne!(sum, first_sum, "another account, another wallet");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    assert_eq!(app.screen, Screen::Backup);
    let tied: Vec<&str> = app
        .backup_flows()
        .iter()
        .filter_map(|f| f.subject.as_ref().map(|s| s.id.as_str()))
        .collect();
    assert!(
        tied.contains(&sum.as_str()),
        "the backup is the remade wallet's"
    );
    assert!(
        !tied.contains(&first_sum.as_str()),
        "nothing is tied to the one taken back"
    );
}
