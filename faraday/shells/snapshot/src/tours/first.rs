//! The first tour.

use crate::tour::{Tour, device_key, scroll, scroll_to, type_text, type_words, vault_place};
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent, bstep};
use faraday_crypto::work::Compute;

/// Costs what `vault_place`, `backup_items_now` and
/// `backup_item_done_now` cost. The backup before spending
/// (`docs/history/NEW-WALLET.md` §14): a single-key wallet made here, its card
/// with Finish the backup first and the receive warning; Places with
/// Vault 1's stick unticked; the vault item with no vault open, with
/// one made and open, and saved into; the templates marked printed, Write
/// to sticks (the lock, each stick by place, Unlock Vault 1); the end of
/// the backup with the checks strip and the chart.
pub(crate) fn first_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::plan::Preset;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    let kind = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::NativeSegwit)
        .ok_or("no native SegWit")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(kind));
    t.press(Action::CNext(faraday_core::cstep::KIND));
    device_key(t, 0);
    let w = t
        .app
        .create
        .as_ref()
        .and_then(|c| c.built)
        .ok_or("the wallet was not made")?;
    t.press(Action::OpenWallet(w));
    t.shot("first-card-held")?;
    // The plan: Paper and vault; Vault 1's stick unticked on Places.
    t.press(Action::BackupFirst(w));
    t.backup_as(Preset::PaperVault);
    // The board, Vault 1 on the place that keeps its stick.
    vault_place(c, t, 0).ok_or("Vault 1's stick has no place")?;
    t.shot("first-place-vault-board")?;
    t.press(Action::BChecklist);
    // The blank templates, printed from the Starter and marked done.
    t.press(Action::BNext(bstep::BLANK));
    // The copy checked, typed back.
    t.press(Action::BCheck);
    type_words(t, None)?;
    t.press(Action::BStep(bstep::VAULT));
    scroll_to(t, Action::Vault(V::CreateFrom(Screen::Backup)));
    t.shot("first-vault-make")?;
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "first phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "first phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults.open.is_empty() && t.app.screen == Screen::Backup {
            break;
        }
        t.tick();
    }
    if t.app.vaults.open.is_empty() {
        return Err("the vault made for the first tour did not open".to_string());
    }
    scroll_to(t, Action::BVaultSave(0));
    t.shot("first-vault-save")?;
    t.press(Action::BVaultSave(0));
    scroll_to(t, Action::BNext(bstep::VAULT));
    t.shot("first-vault-saved")?;
    t.press(Action::BNext(bstep::VAULT));
    if t.app.backup.as_ref().and_then(|b| b.open) != Some(bstep::ENVELOPE) {
        t.press(Action::BStep(bstep::ENVELOPE));
    }
    t.press(Action::BNext(bstep::ENVELOPE));
    write_sticks(t)?;
    t.shot("first-sticks-written")?;
    t.press(Action::Flow(F::UnlockDone(0)));
    type_text(t, "first phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        t.tick();
    }
    let done = t
        .app
        .backup_items_now(c)
        .into_iter()
        .filter_map(bstep::item)
        .all(|it| t.app.backup_item_done_now(c, it));
    if !done {
        return Err("the first tour's checklist is not done".to_string());
    }
    t.shot("first-done")?;
    if t.app.is_compact() {
        // The done card's top: the checks strip, then the chart.
        scroll(t, 420);
        t.shot("first-done-strip")?;
    }
    scroll_to(t, Action::BPlan);
    t.shot("first-done-foot")?;
    Ok(())
}

/// Write to sticks: the one lock, then each stick by place, as the flow
/// asks, until every stick of the plan is written.
fn write_sticks(t: &mut Tour) -> Result<(), String> {
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
    t.pump();
    for _ in 0..6 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.written.is_none()));
        let Some(slot) = slot else { break };
        let stick = match t.app.flow().and_then(|f| f.slots[slot].bound.clone()) {
            Some(b) if b.label == "TESTSTICK" => t.stick(),
            Some(b) => empty_stick("b", b.serial.unwrap_or(2)),
            None => empty_stick(&format!("c{slot}"), 3 + slot as u32),
        };
        t.app.storage(StorageEvent::Sticks(vec![stick]));
        t.pump();
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
    for _ in 0..5 {
        t.tick();
    }
    Ok(())
}

fn empty_stick(id: &str, serial: u32) -> faraday_core::StickInfo {
    faraday_core::StickInfo {
        serial: Some(serial),
        id: id.into(),
        label: "KEYSTICK".into(),
        boot: false,
        files: Vec::new(),
    }
}
