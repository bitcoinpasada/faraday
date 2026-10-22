//! The kept tour.

use crate::tour::{Tour, scroll_to, seed_files_at, to_checklist, type_text, type_words};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent, bstep};
use faraday_crypto::work::Compute;

/// Scrolls the checklist until the open item's Continue is on screen.
pub(crate) fn scroll_to_open_next(t: &mut Tour) {
    if let Some(n) = t.app.backup.as_ref().and_then(|b| b.open) {
        scroll_to(t, Action::BNext(n));
    }
}

/// Costs a map of the plan (through `seed_files_at`). Where each seed
/// is kept, on the backup's panel (the seeds page on a small panel):
/// Savings with test key 1 here, its words put in the Outbox
/// unprotected and no vault; then, with the test vault open, Spending,
/// whose one key and wallet the vault holds, its copy typed back and
/// matched.
pub(crate) fn kept_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    t.press(Action::Backup(0));
    t.backup_as(faraday_core::plan::Preset::Paper);
    seed_files_at(c, t, 0);
    t.press(Action::BChecklist);
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BFile);
    t.press(Action::SecretAck);
    t.press(Action::SecretUnprotected);
    scroll_to_open_next(t);
    t.shot("kept-two-of-three")?;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    let inbox: Vec<(String, Vec<u8>)> = t
        .app
        .inbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .chain(std::iter::once(vault))
        .collect();
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.vaults.open.len() != 1 {
        return Err("the test vault did not open".into());
    }
    t.press(Action::Backup(1));
    to_checklist(t, faraday_core::plan::Preset::PaperVault, &[]);
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.press(Action::BCheck);
    type_words(t, None)?;
    scroll_to_open_next(t);
    t.shot("kept-one-key")?;
    Ok(())
}
