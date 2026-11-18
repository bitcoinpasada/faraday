//! The wallet_flow tour.

use crate::tour::{Tour, bmap, boff, type_text};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::plan::{self as fplan, At, What};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StickInfo, StorageEvent, plan::Item};
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
/// (Kind, Keys, Check, Back up) with Keys' row for a vault already on a
/// stick: its stick brought in, the vault unlocked, the key made; Back up's
/// plan; the checklist ends in Write to sticks; one lock, each stick by
/// place (the vault's own stick replaced), done.
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
        make: Default::default(),
        serial: Some(serial),
        id: faraday_text::Text::of(id),
        label: faraday_text::Text::of("KEYSTICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    };
    t.press(Action::CreateWallet);
    t.press(Action::CKind(0));
    t.press(Action::CBackupForm(
        faraday_core::create::BackupForm::Words.index(),
    ));
    t.shot("wallet-flow-1-create-keys-row")?;
    t.press(Action::CExistingVault(true));
    t.shot("wallet-flow-3-insert")?;
    // A figure's ⋮ menu; Cancel leaves it.
    t.press(Action::Flow(F::FigMenu(0)));
    t.shot("wallet-flow-3b-figure-menu")?;
    t.press(Action::Cancel);
    // The vault on the stick: the person picks which, then the stick's sheet.
    let first = t.stick();
    t.app.storage(StorageEvent::Sticks(vec![first.clone()]));
    t.pump();
    t.shot("wallet-flow-4-which-vault")?;
    // The stick's vault is picked from its files and read.
    let k = t
        .app
        .flow()
        .and_then(|f| f.live.reading.as_ref())
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "vault.kdbx"))
        .ok_or("no vault on the stick")?;
    t.press(Action::Flow(F::PickFile(0, k)));
    t.press(Action::Flow(F::Tap));
    t.pump();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.press(Action::Flow(F::Want(0)));
    t.shot("wallet-flow-4a-stick-sheet")?;
    t.press(Action::Cancel);
    // Bring in, collapsed to the sticks inserted; the vault is unlocked
    // from Vaults, and Create's Keys follows.
    t.press(Action::Flow(F::SticksOn));
    t.shot("wallet-flow-5-bring-in-inserted")?;
    t.press(Action::Flow(F::UnlockVault(0)));
    type_text(t, faraday_core::testkit::VAULT_PASSPHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..3 {
        t.tick();
    }
    t.shot("wallet-flow-6-create-keys")?;
    t.press(Action::Flow(F::Show(0)));
    t.shot("wallet-flow-6a-vaults-unlocked")?;
    t.press(Action::Nav(Screen::Create));
    t.press(Action::Entry(None));
    t.type_key(1);
    let fp = t.app.session.keys.last().ok_or("no key")?.fingerprint.0;
    t.press(Action::Nav(Screen::Create));
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    // Back up's plan holds the new vault the preset adds.
    t.shot("wallet-flow-7-back-up-vaults")?;
    // The key in it, none on paper; the vault is made from its item.
    words_off(c, t);
    t.press(Action::BChecklist);
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    type_text(t, "a vault phrase for the tour");
    t.press(Action::Vault(V::CFocus(0, true)));
    type_text(t, "a vault phrase for the tour");
    t.press(Action::Vault(V::CGo));
    for _ in 0..3 {
        t.tick();
    }
    type_text(t, "a vault phrase for the tour");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..3 {
        t.tick();
    }
    t.press(Action::BVaultSave(0));
    t.press(Action::BOut(3));
    t.press(Action::BNext(Item::Envelopes));
    t.shot("wallet-flow-8-write-to-sticks")?;
    t.press(Action::Flow(F::WalletLock));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: faraday_core::files_from(outbox),
        kept: faraday_core::kept_from(kept),
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
    for n in 0..6 {
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
        // The vault's name is taken on its own stick: replaced.
        if t.app.sheet == Some(faraday_core::Sheet::NameTaken) {
            t.press(Action::NameTakenOverwrite);
        }
        for _ in 0..3 {
            t.tick();
        }
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
    }
    t.shot("wallet-flow-11-done")?;
    // A new wallet's Keys card, with the making on it.
    t.press(Action::CreateWallet);
    t.press(Action::CKind(0));
    t.press(Action::CBackupForm(
        faraday_core::create::BackupForm::Words.index(),
    ));
    // The making on the card: some rolls in, then the words made. A roll
    // the entry refuses (a repeat) is tried again with the next face.
    let roll = |t: &mut Tour, n: usize| {
        if t.app.keygen().is_some_and(|k| !k.length_chosen) {
            t.press(Action::KWords(12));
        }
        for i in 0..n {
            let before = t.app.keygen().map_or(0, |k| k.dice.len());
            for f in 0..6usize {
                t.press(Action::KRoll(((i * 5 + f) % 6) as u8 + 1));
                if t.app.keygen().map_or(0, |k| k.dice.len()) > before {
                    break;
                }
            }
        }
    };
    roll(t, 20);
    crate::tour::scroll_to(t, Action::KWords(24));
    t.shot("wallet-flow-12-create-keys-rolls")?;
    let need = t.app.keygen().map_or(0, |k| k.dice_needed());
    let have = t.app.keygen().map_or(0, |k| k.dice.len());
    roll(t, need.saturating_sub(have));
    t.press(Action::KNext);
    crate::tour::scroll_to(t, Action::KShow);
    t.shot("wallet-flow-13-create-keys-words")?;
    // A multisig's Key 2: Scan its xpub first, New words beside it, which
    // opens the same making there.
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::Nav(faraday_core::Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(faraday_core::cstep::KIND));
    t.press(Action::CNext(faraday_core::cstep::QUORUM));
    t.press(Action::CSlotLater(0));
    crate::tour::scroll_to(t, Action::KeyOnCard(1));
    t.shot("wallet-flow-13b-create-key-2-new-words")?;
    t.press(Action::KeyOnCard(1));
    roll(t, 20);
    crate::tour::scroll_to(t, Action::KWords(24));
    t.shot("wallet-flow-13c-create-key-2-rolls")?;
    println!(
        "the backup flow of a single key in a new vault, from Keys with the row for a vault already on a stick to Done"
    );
    Ok(())
}
