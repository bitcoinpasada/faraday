//! The oldparts tour.

use crate::tour::Tour;
use faraday_core::{Action, Screen};
use faraday_crypto::work::Compute;

/// Costs a map of the plan (through `of_now`). Make new parts's kept old
/// Seed XOR parts (`docs/BACKUP-FLOW.md` §17.26) on Savings: a key split
/// into Seed XOR parts and placed, its set then replaced by another one
/// (as a second split does), so the plan's placed parts are kept old
/// parts: the board with the old set's group, and a kept part's sheet.
pub(crate) fn old_parts_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::StorageEvent;
    use faraday_core::create::NewKind;
    use faraday_core::glance::sheet::{ChartAction as C, Target};
    use faraday_core::glance::{Backup, Press};
    use faraday_core::plan::What;
    use faraday_core::testkit;
    use faraday_core::vault::VaultAction as V;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Entry(None));
    t.type_key(0);
    let savings = {
        let text = crate::tour::key_text_of(c, &t.app.session.keys, 0, NewKind::NativeSegwit)?;
        t.app
            .session
            .add_wallet_confirmed(
                c,
                "Savings",
                &format!("wpkh({text}/<0;1>/*)"),
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .map_err(|e| String::from(&*e.text()))?
    };
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        crate::tour::type_text(t, "old parts phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..30 {
        if !t.app.vaults().open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.vaults().open.is_empty() {
        return Err("the vault made for the oldparts tour did not open".to_string());
    }
    // The key split into three Seed XOR parts, the plan placing them.
    let first = t.app.session.keys.info(0).ok_or("no key")?.fingerprint.0;
    let set = faraday_vault::xor::XorFacts {
        parts: vec![first, [5, 6, 7, 8], [9, 10, 11, 12]],
    };
    if !t.app.session.keys.set_xor(0, Some(set)) {
        return Err("the key was not split".into());
    }
    let save = |t: &mut Tour| {
        t.press(Action::Backup(savings));
        t.press(Action::BChecklist);
        t.press(Action::BVaultSave(0));
        t.press(Action::Nav(Screen::VaultContents));
        t.press(Action::Vault(V::AddKind(1)));
        t.press(Action::Vault(V::SaveWallet(savings)));
        t.press(Action::OpenWallet(savings));
    };
    save(t);
    // Split again: a new set the plan did not place, so the placed parts
    // read as kept old parts once Back up saves the plan again.
    let fresh = faraday_vault::xor::XorFacts {
        parts: vec![[21, 22, 23, 24], [25, 26, 27, 28]],
    };
    if !t.app.session.keys.replace_xor(0, fresh) {
        return Err("the set was not replaced".into());
    }
    t.press(Action::Backup(savings));
    t.press(Action::BChecklist);
    t.press(Action::BVaultSave(0));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::SaveWallet(savings)));
    // The chart: the old set's group beside the places.
    t.press(Action::OpenWallet(savings));
    if t.app.is_compact() {
        t.press(Action::Glance(true));
    }
    crate::tour::scroll(t, 1000);
    t.shot("old-parts-board")?;
    crate::tour::scroll(t, 400);
    t.shot("old-parts-board-2")?;
    // A kept part's sheet from the chart.
    let g = faraday_core::glance::of_now(c, &t.app, savings).ok_or("no chart")?;
    let Backup::Plan { nodes, .. } = g.backup else {
        return Err("no plan on the chart".into());
    };
    let (n, j) = nodes
        .iter()
        .enumerate()
        .find_map(|(n, node)| {
            let j = node
                .whats
                .iter()
                .position(|w| matches!(w, What::XorKept(..)))?;
            Some((n as u8, j as u8))
        })
        .ok_or("no kept part on the chart")?;
    t.press(Action::Chart(C::Open(
        Press::Loaded(savings),
        Target::Line(n, j),
    )));
    t.shot("old-part-sheet")?;
    Ok(())
}
