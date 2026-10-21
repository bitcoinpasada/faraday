//! The plan tour.

use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::testkit;
use faraday_core::{Action, bstep, qstep};
use faraday_crypto::work::Compute;
use crate::tour::{Tour, bput, scroll_to, shares_on_paper, vault_stick};

/// Costs what `shares_on_paper`, `add_wallet` and `vault_stick` cost.
/// The backup's plan: for Spending (one key, test key 1 here), the
/// presets, the seeds and the places with the map beside them; for
/// Savings (2-of-3, one seed here), the seeds and the places, with the
/// whole sheet and then with its own share, the page What is a share?
/// opens, and on a small panel the map's own page; then the checklist
/// part done, and the blank template written beside the screens; then a
/// 3-of-5's shares on Places and on the checklist.
pub(crate) fn plan_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    let find = |t: &Tour, name: &str| {
        t.app
            .session
            .wallets
            .iter()
            .position(|w| w.name == name)
            .ok_or(format!("no {name}"))
    };
    let spending = find(t, "Spending")?;
    let savings = find(t, "Savings")?;
    // The board (`docs/BACKUP-FLOW.md` §17): a one-key wallet on Paper
    // and vault, its board, Vault 1 held with the places that take it
    // edged, and The plan.
    t.press(Action::Backup(spending));
    t.shot("plan-board-read-only-one-key")?;
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.shot("plan-board-one-key")?;
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Vault(0)))));
    t.shot("plan-board-vault-held-one-key")?;
    t.press(Action::Board(BP::LetGo));
    t.press(Action::BQ(qstep::PLAN));
    t.shot("plan-plan-one-key")?;
    scroll_to(t, Action::BChecklist);
    t.shot("plan-plan-one-key-foot")?;
    // Savings (2-of-3), no seed here: its board holds the vaults alone;
    // each place with a share on paper; What is a share? opens.
    t.press(Action::Backup(savings));
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.shot("plan-board-two-of-three")?;
    shares_on_paper(c, t);
    t.shot("plan-board-shares-two-of-three")?;
    t.press(Action::LearnShares);
    t.shot("plan-learn-shares")?;
    t.press(Action::Cancel);
    t.press(Action::BQ(qstep::PLAN));
    t.shot("plan-map-two-of-three")?;
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("plan-checklist-part-done")?;
    t.press(Action::BStep(bstep::ENVELOPE));
    t.shot("plan-envelopes")?;
    // A 3-of-5's shares: up to two keys left off each, on the board and
    // on the checklist's Shares card.
    let five = t
        .app
        .session
        .add_wallet(
            c,
            "Five",
            &testkit::three_of_five(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    t.press(Action::Backup(five));
    t.backup_as(faraday_core::plan::Preset::Paper);
    shares_on_paper(c, t);
    t.press(Action::BOmit(2));
    t.shot("plan-board-shares-three-of-five")?;
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("plan-shares-three-of-five")?;
    // A vault per seed: Spending's one vault and its checklist item; then
    // Savings with test key 2 typed in too, two seeds here and a vault
    // each: the board, Vault 1's stick at Place 1 too, The plan and the
    // second vault's item.
    t.press(Action::Backup(spending));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::VAULT));
    t.shot("plan-checklist-vault-one-key")?;
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::Backup(savings));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.shot("plan-board-vaults-two-of-three")?;
    if vault_stick(c, t, 0, 0).is_none() {
        bput(t, Thing::Vault(0), Zone::Card(0));
    }
    t.shot("plan-board-vault-linked-two-of-three")?;
    t.press(Action::BQ(qstep::PLAN));
    t.shot("plan-map-vaults-two-of-three")?;
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::VAULT + 1));
    t.shot("plan-checklist-vaults-two-of-three")?;
    for item in t
        .app
        .outbox
        .iter()
        .filter(|i| i.name.starts_with("blank-template"))
    {
        let path = t.out.join(format!("outbox-{}", item.name));
        std::fs::write(&path, &item.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}
