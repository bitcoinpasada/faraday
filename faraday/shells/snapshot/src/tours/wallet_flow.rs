//! The wallet_flow tour.

use crate::tour::{Tour, bmap, boff, keep_vault, type_text, vault_place};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::plan::{self as fplan, At, What};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StickInfo, StorageEvent, bstep};
use faraday_crypto::work::Compute;

/// Costs a map of the plan (through `bmap`). Every seed's words and
/// SeedQR taken off every place's paper.
pub(crate) fn words_off(c: &Compute, t: &mut Tour) {
    let places = t.app.board_plan().map_or(0, |(_, a)| a.places);
    for p in 0..places {
        for w in fplan::holds_at(&bmap(c, t), At::Place(p)) {
            if let What::Words(i) | What::SeedQr(i) = w
                && fplan::holds_at(&bmap(c, t), At::Place(p)).contains(&w)
            {
                boff(t, Zone::Paper(p), Thing::Seed(i));
            }
        }
    }
}

/// Costs a map of the plan. A wallet's backup flow: Make a new wallet
/// asks whether vaults are part of the backup and how many, brings in
/// each vault's stick, opens or makes the vaults, makes the key; Back
/// up's plan offers the vaults; the checklist ends in Write to sticks;
/// one lock, each stick by place, done. Then another wallet's two vaults,
/// one found on a stick and one on the boot stick, to the step's Open and
/// Create, and Create a vault.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn wallet_flow_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::steps::FlowAction as F;
    use faraday_core::cstep;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    let empty = |id: &str, serial: u32| StickInfo {
        serial: Some(serial),
        id: id.into(),
        label: "KEYSTICK".into(),
        boot: false,
        files: Vec::new(),
    };
    t.press(Action::CreateWallet);
    t.press(Action::CKind(0));
    t.shot("wallet-flow-1-create-yes-no")?;
    t.press(Action::CVaults(true));
    t.press(Action::CNext(cstep::WHERE));
    t.press(Action::CVaultN(2));
    t.shot("wallet-flow-2-create-count")?;
    t.press(Action::CNext(cstep::HOW_MANY));
    t.shot("wallet-flow-3-insert")?;
    // Guided's walk-through of Bring in the sticks, behind its tap on a
    // small panel.
    t.press(Action::Guided(true));
    t.press(Action::About(0));
    t.shot("wallet-flow-3a-insert-guided")?;
    t.press(Action::About(0));
    // Vault 1: a stick that holds a vault, and the person picks.
    let first = t.stick();
    t.app.storage(StorageEvent::Sticks(vec![first.clone()]));
    t.pump();
    t.shot("wallet-flow-4-which-vault")?;
    t.press(Action::Flow(F::PickBeside(0)));
    t.press(Action::Flow(F::Tap));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    // Vault 2: an empty stick, a new vault.
    t.app.storage(StorageEvent::Sticks(vec![empty("b", 2)]));
    t.pump();
    t.press(Action::Flow(F::Tap));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.shot("wallet-flow-5-vaults")?;
    for v in 0..2 {
        t.press(Action::Flow(F::CreateVault(v)));
        type_text(t, "a vault phrase for the tour");
        t.press(Action::Vault(V::CFocus(0, true)));
        type_text(t, "a vault phrase for the tour");
        t.press(Action::Vault(V::CGo));
        for _ in 0..3 {
            t.tick();
        }
    }
    // Back in Make a new wallet, on Keys, with the cards done.
    t.shot("wallet-flow-6-create-keys")?;
    t.press(Action::Entry(None));
    t.type_key(1);
    let fp = t.app.session.keys.last().ok_or("no key")?.fingerprint.0;
    t.press(Action::Nav(Screen::Create));
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    // Back up's plan offers the two open vaults.
    t.shot("wallet-flow-7-back-up-vaults")?;
    // The key in both vaults, none on paper.
    words_off(c, t);
    let p = vault_place(c, t, 1).unwrap_or(0);
    keep_vault(c, t, p, 1, &[0]);
    t.press(Action::BChecklist);
    t.press(Action::BVaultSave(0));
    t.press(Action::BVaultSave(1));
    t.press(Action::BOut(3));
    t.press(Action::BNext(bstep::ENVELOPE));
    t.shot("wallet-flow-8-write-to-sticks")?;
    t.press(Action::Flow(F::WalletLock));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.pump();
    t.shot("wallet-flow-9-write")?;
    // Change the plan in the flow: the board, drawn in the flow on the
    // flow's own plan (`docs/BACKUP-FLOW.md` §17.12 item 1), after the
    // lock. A wallet's plan opens on its board (§17.21 item 11); then a
    // vault's chip held.
    t.press(Action::Flow(F::ChangePlan));
    t.shot("wallet-flow-9a-plan-board")?;
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Vault(0)))));
    t.shot("wallet-flow-9b-plan-board-vault-held")?;
    t.press(Action::Cancel);
    if let Some(n) = t.app.flow().map(|f| f.step.n()) {
        t.press(Action::Flow(F::Open(n)));
    }
    // Each stick by place: the sticks read for the vaults, then the stick
    // of public files.
    for n in 0..3 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.written.is_none()));
        let Some(slot) = slot else { break };
        let stick = match t.app.flow().and_then(|f| f.slots[slot].bound.clone()) {
            Some(b) if b.label == "TESTSTICK" => t.stick(),
            Some(b) => empty("b", b.serial.unwrap_or(2)),
            None => empty("c", 3),
        };
        t.app.storage(StorageEvent::Sticks(vec![stick]));
        t.pump();
        if n == 0 {
            t.shot("wallet-flow-10-written")?;
        }
        for _ in 0..2 {
            let hold = t
                .app
                .flow()
                .and_then(|f| f.live.reading.as_ref())
                .is_some_and(|r| r.hold);
            t.press(Action::Flow(if hold { F::Hold } else { F::Tap }));
        }
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
    }
    t.shot("wallet-flow-11-done")?;
    // Another wallet, two vaults: Vault 1 found on the first stick, then
    // Use Boot stick takes the boot stick for Vault 2 in one press; with
    // every stick read, the step offers Open Vault 1 and Create Vault 2,
    // and Create's Pull sheet asks for the boot stick out first.
    let boot = StickInfo {
        serial: Some(9),
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        files: Vec::new(),
    };
    t.press(Action::Guided(false));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(0));
    t.press(Action::CVaults(true));
    t.press(Action::CNext(cstep::WHERE));
    t.press(Action::CVaultN(2));
    t.press(Action::CNext(cstep::HOW_MANY));
    let stick = t.stick();
    t.app.storage(StorageEvent::Sticks(vec![stick]));
    t.pump();
    t.press(Action::Flow(F::PickFile(0, 0)));
    t.press(Action::Flow(F::Tap));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.app.storage(StorageEvent::Sticks(vec![boot]));
    t.pump();
    t.shot("wallet-flow-12-boot-insert")?;
    let k = t
        .app
        .sticks()
        .iter()
        .position(|s| s.boot)
        .ok_or("no boot stick")?;
    t.press(Action::Flow(F::UseBoot(k)));
    t.pump();
    t.shot("wallet-flow-13-open-or-create")?;
    t.press(Action::Flow(F::CreateVault(1)));
    t.shot("wallet-flow-14-boot-pull")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.shot("wallet-flow-15-boot-create-vault")?;
    println!(
        "the backup flow of a single key in two vaults, from the question of vaults to Done; then two vaults, one found on a stick and one on the boot stick, to Open or Create and Create a vault"
    );
    Ok(())
}
