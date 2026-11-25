//! A wallet's plan, its backup record and its date notes are its own: two
//! wallets of one descriptor checksum keep them apart in the vault, on
//! Back up, on the chart and on the plan card (`docs/WALLETS.md` §4).

use crate::chart_edit::{chart, clock_kept, desktop, fresh_vault, node_of, nodes};
use faraday_core::backup_flow::BackupFlow;
use faraday_core::glance::Backup;
use faraday_core::plan::{Answers, At, Preset};
use faraday_core::testkit::{self, WALLET_ID_FIRST as FIRST, WALLET_ID_TWIN as TWIN};
use faraday_core::wallet::Origin;
use faraday_core::{Action, Faraday};
use faraday_shell_api::App;

/// FIRST (wallet 0) and TWIN (wallet 1) loaded, the clock kept, a vault
/// open.
fn twins() -> Faraday {
    let c = testkit::compute();
    let mut app = desktop();
    for (name, d) in [("First", FIRST), ("Twin", TWIN)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "kit", Origin::Named)
            .expect("the wallet reads");
    }
    clock_kept(&mut app);
    fresh_vault(&mut app);
    app
}

/// Wallet `w` planned and its plan saved into the open vault.
fn kept_plan(app: &mut Faraday, w: usize) {
    app.press(Action::Backup(w));
    testkit::backup_as(app, Preset::PaperVault);
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
}

/// The dates the chart of wallet `w` shows for what its places hold.
fn dates(app: &Faraday, w: usize) -> Vec<String> {
    nodes(&chart(app, w))
        .iter()
        .flat_map(|n| {
            n.checked
                .iter()
                .filter_map(|c| c.as_deref().map(String::from))
        })
        .collect()
}

/// A copy of the first thing Place 1 of wallet `w` keeps dated, on Back up.
fn date_a_copy(app: &mut Faraday, w: usize) {
    let (_, node) = node_of(app, w, "Place 1");
    let At::Place(p) = node.at else {
        panic!("Place 1 is no place: {node:?}");
    };
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    testkit::backup_date_copy(app, p, node.whats[0]);
    app.press(Action::OpenWallet(w));
}

/// The date a copy was checked is noted in its wallet's plan record and
/// not in its twin's.
#[test]
fn a_date_note_enters_the_plan_record_of_its_own_wallet_alone() {
    let mut app = twins();
    kept_plan(&mut app, 0);
    kept_plan(&mut app, 1);
    assert!(dates(&app, 0).is_empty() && dates(&app, 1).is_empty());
    date_a_copy(&mut app, 0);
    assert!(!dates(&app, 0).is_empty(), "the first's own record has it");
    assert!(dates(&app, 1).is_empty(), "the twin's record took the note");
    date_a_copy(&mut app, 1);
    assert!(!dates(&app, 1).is_empty(), "the twin's own record has it");
}

/// Home's glance of a wallet shows its own plan and backup lines: the
/// twin of a planned wallet has none, and a locked vault's summary does
/// not speak for it.
#[test]
fn the_glance_of_a_twin_shows_no_plan_of_the_other_wallet() {
    let mut app = twins();
    kept_plan(&mut app, 0);
    assert!(matches!(chart(&app, 0).backup, Backup::Plan { .. }));
    assert!(matches!(chart(&app, 1).backup, Backup::None), "the plan");
    let c = testkit::compute();
    let all = app.backups_now(&c);
    let twin = app.session.wallets[1].id().to_hex().to_string();
    let e = all.iter().find(|e| *e.id == *twin).expect("an entry");
    assert!(e.lines.is_none(), "the twin has backup lines");
    testkit::vault_lock_one(&mut app, 0);
    assert!(
        matches!(chart(&app, 1).backup, Backup::None),
        "the locked vault's summary was said for the twin"
    );
}

/// A record saved with no id belongs to the one wallet of its checksum,
/// and to none where two are loaded; it is kept either way.
#[test]
fn a_record_with_no_id_is_adopted_by_the_one_wallet_of_its_checksum_only() {
    let c = testkit::compute();
    let mut app = desktop();
    app.session
        .add_wallet_confirmed(&c, "First", FIRST, "kit", Origin::Named)
        .unwrap();
    let sum = app.session.wallets[0].checksum().to_string();
    let record = format!("backup\t{sum}\tfirst\t1\n");
    testkit::restore_backups(&mut app, record.as_bytes());
    let all = app.backups_now(&c);
    assert_eq!(all.len(), 1);
    assert!(all[0].lines.is_some(), "the one wallet takes the record");

    let mut app = desktop();
    for (name, d) in [("First", FIRST), ("Twin", TWIN)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "kit", Origin::Named)
            .unwrap();
    }
    testkit::restore_backups(&mut app, record.as_bytes());
    let all = app.backups_now(&c);
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|e| e.lines.is_none()), "a wallet took it");
    let kept = String::from_utf8_lossy(&testkit::backups_record(&app)).to_string();
    assert!(kept.contains(&format!("backup\t{sum}\tfirst")), "{kept:?}");
}

/// An unmade flow of wallet `id` for `app`'s wallet `w`.
fn unmade_flow(app: &Faraday, w: usize, id: faraday_bip::policy::WalletId) -> BackupFlow {
    let c = testkit::compute();
    let shape = app.plan_shape_now(w);
    let base = if shape.multi() { shape.seeds.len() } else { 2 };
    let answers = Answers::preset_at(&shape, Preset::PaperVault, base);
    let mut f = BackupFlow::new_wallet(&c, shape, answers, "W");
    let wf = f.wallet.as_mut().unwrap();
    wf.checksum = Some(faraday_text::Text::of(app.session.wallets[w].checksum()));
    wf.id = Some(id);
    f.live.plan_q = Some(faraday_core::qstep::PLAN);
    f
}

/// The plan card of a flow takes the place names typed on Back up only
/// when Back up is open on the flow's own wallet.
#[test]
fn the_plan_card_of_a_flow_takes_the_names_of_its_own_wallets_backup_alone() {
    let mut app = twins();
    app.press(Action::Backup(0));
    app.press(Action::BName(0));
    crate::common::type_text(&mut app, "Attic");
    assert_eq!(&*app.place_name(0), "Attic");
    let (first, twin) = (app.session.wallets[0].id(), app.session.wallets[1].id());
    let of_first = unmade_flow(&app, 0, first);
    let of_twin = unmade_flow(&app, 1, twin);
    let a = testkit::flow_push(&mut app, of_first);
    let b = testkit::flow_push(&mut app, of_twin);
    let shown = |app: &mut Faraday, i: usize| {
        app.press(Action::Flow(
            faraday_core::backup_flow::steps::FlowAction::Show(i),
        ));
        let _ = app.frame();
        app.drawn_texts().iter().any(|t| t.contains("Attic"))
    };
    assert!(shown(&mut app, a), "the first's flow shows its own names");
    assert!(
        !shown(&mut app, b),
        "the twin's flow shows the first's names"
    );
}

/// Taking a wallet out ends only its own unmade flow: its twin's flow,
/// of the same checksum, stays.
#[test]
fn taking_a_wallet_out_ends_its_own_unmade_flow_and_not_its_twins() {
    let mut app = desktop();
    let c = testkit::compute();
    for (name, d) in [("First", FIRST), ("Twin", TWIN)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "kit", Origin::Named)
            .unwrap();
    }
    let first = app.session.wallets[0].id();
    let of_first = unmade_flow(&app, 0, first);
    testkit::flow_push(&mut app, of_first);
    app.press(Action::Backup(1));
    app.press(Action::RemoveWallet(1));
    assert_eq!(app.backup_flows().len(), 1, "the first's flow ended");
}
