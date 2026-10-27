//! The stick that carries a wallet's public files keeps no secret file
//! (`docs/BACKUP-FLOW.md` §10.2): a plan accepted at the lock can be
//! written, every stick of it, and the key it holds is let go.

use crate::backup_flow_wallet::{PHRASE, load_first_vault, restart, step, write_sticks};
use crate::common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::{Thing, Zone};
use faraday_core::plan::{self, At, What};
use faraday_core::{Action, Faraday, Screen, bstep, cstep};

/// `n` empty sticks of different serials.
fn sticks(shell: &mut Shell, n: usize) {
    for k in 1..=n {
        let mut d = Disk::empty(&format!("S{k}"), "KEYSTICK");
        d.serial = Some(k as u32);
        shell.add(d);
    }
}

/// A key made by New key for slot 0 of the Create open.
fn new_key_here(app: &mut Faraday) -> [u8; 4] {
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 3 + i / 7) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    let fp = app.session.keys.last().unwrap().fingerprint.0;
    app.press(Action::KNext);
    fp
}

/// A single key with Vault 1 set up before it on S1, then made here and
/// backed up on paper and in the vault.
fn single_key_in_a_vault(shell: &mut Shell) -> (Faraday, [u8; 4]) {
    sticks(shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(1));
    app.press(Action::CNext(cstep::HOW_MANY));
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    create_vault(shell, &mut app, 0, PHRASE);
    app.press(Action::Nav(Screen::Create));
    let fp = new_key_here(&mut app);
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    (app, fp)
}

/// Saves into the open vaults what the plan puts in each.
fn save_vaults(app: &mut Faraday) {
    let c = faraday_core::testkit::compute();
    let vaults: Vec<usize> = app
        .backup_items_now(&c)
        .into_iter()
        .filter_map(|n| match bstep::item(n) {
            Some(faraday_core::plan::Item::Vault(v)) => Some(v),
            _ => None,
        })
        .collect();
    for v in vaults {
        app.press(Action::BVaultSave(v as u8));
    }
}

/// The seed's file beside the vault on the stick of the place that keeps
/// the wallet, with the sheet on paper: the public files go on a stick
/// that keeps no secret file, every stick is written, and the key is let
/// go.
#[test]
fn a_seed_file_beside_the_wallets_vault_leaves_the_public_files_another_stick() {
    let c = faraday_core::testkit::compute();
    let mut shell = Shell::new();
    let (mut app, fp) = single_key_in_a_vault(&mut shell);
    no_words(&mut app);
    let p = (0..board_places(&app))
        .find(|&p| vault_stick(&app, p, 0).is_some())
        .expect("the vault's stick has a place");
    let s = vault_stick(&app, p, 0).unwrap();
    put(&mut app, Thing::Seed(0), Zone::Files(p, s));
    let on_stick = backup_held(&app, At::Stick(p, s));
    assert!(
        on_stick.contains(&What::VaultStick(0)) && on_stick.contains(&What::SeedFile(0)),
        "the seed's file beside the vault: {on_stick:?}"
    );
    // The plan, written as a custom map, names the wallet at that place.
    {
        let shape = app.plan_shape_now(app.backup().unwrap().wallet);
        let a = faraday_core::testkit::backup_answers_mut(&mut app);
        let m = plan::map(&c, &shape, a);
        a.set_map(plan::with_added(&m, At::Place(p), What::Wallet));
    }
    assert!(
        backup_held(&app, At::Place(p)).contains(&What::Wallet),
        "the place keeps the wallet"
    );
    assert!(
        backup_held(&app, At::Place(p)).contains(&What::Sheet),
        "the sheet is on paper there"
    );
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BOut(3));
    app.press(Action::BNext(bstep::ENVELOPE));
    let open: Vec<u8> = app
        .backup_items_now(&c)
        .into_iter()
        .filter(|&n| n != bstep::STICKS)
        .filter(|&n| !bstep::item(n).is_some_and(|it| app.backup_item_done_now(&c, it)))
        .collect();
    assert!(open.is_empty(), "every other item is done: {open:?}");
    let fp = faraday_bip::keys::Fingerprint(fp);
    assert!(app.key_held(&c, fp), "held until its sticks are written");
    shell.flow(&mut app, F::WalletLock);
    assert_eq!(app.toast_text(), None);
    assert_eq!(let_seed_files_out(&mut app), 1, "the seed's file let out");
    let mut app = restart(&mut shell, &mut app);
    assert!(
        app.backup_flows()[0].waiting(),
        "the key waits for its sticks"
    );
    let n = app.flow().unwrap().slots.len();
    let disks: Vec<String> = (1..=n).map(|k| format!("S{k}")).collect();
    let disks: Vec<&str> = disks.iter().map(String::as_str).collect();
    write_sticks(&mut shell, &mut app, &disks);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    // No stick carries both the sheet and the seed's file.
    for d in &disks {
        let names: Vec<String> = shell.disk(d).files.iter().map(|f| f.0.clone()).collect();
        let seed = names
            .iter()
            .any(|n| n.starts_with("seed-") && n.ends_with(".txt"));
        let sheet = names.iter().any(|n| n.ends_with("-backup.pdf"));
        assert!(!(seed && sheet), "{d}: {names:?}");
    }
    // The next power-on, its key loaded from the vault: the gate holds it
    // no more.
    assert!(!app.backup_flows()[0].waiting());
    let mut next = shell.lock(&mut app);
    load_first_vault(&mut next, PHRASE);
    assert!(
        next.session.keys.all().any(|k| k.fingerprint == fp),
        "the key is loaded"
    );
    assert!(!next.key_held(&c, fp), "let go once every stick is written");
}
