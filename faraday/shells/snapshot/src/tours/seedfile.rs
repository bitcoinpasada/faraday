//! The seedfile tour.

use crate::tour::{Tour, bmap, bshape, scroll, scroll_to, seed_files_at, type_text};
use faraday_core::board::{BoardPress as B, Chip, Thing, Zone};
use faraday_core::plan::{At, What};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent, bstep};
use faraday_crypto::work::Compute;

/// Costs a map of the plan (through `seed_files_at`). The backup's
/// checklist offering a seed into a vault or as a file: with no vault
/// open, the step and the secret sheet; with the test vault open, a
/// seed it holds and one it does not, and the sheet; then the seed's
/// SeedQR (kept on paper through the board's form toggles, as the
/// presets keep none), Standard and Compact, put in the Outbox past the
/// warning and written beside the screens.
pub(crate) fn seedfile_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    t.press(Action::Entry(None));
    t.type_key(1);
    let second = t
        .app
        .session
        .keys
        .all()
        .enumerate()
        .position(|(i, _)| testkit::has_test_words(&t.app.session.keys, i, 1))
        .ok_or("test key 2 was not added")?;
    t.press(Action::Backup(0));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    seed_files_at(c, t, 0);
    seedqr_on_paper(c, t)?;
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::FILES));
    scroll_to(t, Action::BFile);
    t.shot("seedfile-no-vault")?;
    t.press(Action::BFile);
    t.shot("seedfile-sheet-no-vault")?;
    // On a small panel the sheet scrolls: its middle.
    scroll(t, 240);
    t.shot("seedfile-sheet-no-vault-down")?;
    t.press(Action::Cancel);
    // The test vault, unlocked: it holds test key 1, not test key 2.
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    let inbox: Vec<(String, Vec<u8>)> = t
        .app
        .inbox()
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
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    crate::tour::not_now_load(t);
    if t.app.vaults().open.len() != 1 {
        return Err("the test vault did not open".into());
    }
    t.press(Action::Nav(Screen::Backup));
    t.press(Action::BStep(bstep::VAULT));
    t.shot("seedfile-in-vault")?;
    t.press(Action::BKey(second));
    t.shot("seedfile-vault")?;
    t.press(Action::BStep(bstep::FILES));
    t.press(Action::BFile);
    t.press(Action::SecretForm(1));
    t.shot("seedfile-sheet-seedqr")?;
    scroll(t, 240);
    t.shot("seedfile-sheet-seedqr-down")?;
    t.press(Action::SecretUnprotected);
    t.shot("seedfile-plain")?;
    t.press(Action::SecretHold);
    t.press(Action::BCompact(true));
    t.press(Action::BFile);
    t.press(Action::SecretForm(1));
    t.press(Action::SecretUnprotected);
    t.press(Action::SecretHold);
    let pictures: Vec<_> = t
        .app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".png"))
        .collect();
    if pictures.len() != 2 {
        return Err(format!("{} SeedQR pictures in the Outbox", pictures.len()));
    }
    for item in pictures {
        let path = t.out.join(format!("outbox-{}", item.name));
        std::fs::write(&path, &item.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

/// Costs a map of the plan (through `bmap`). The presets keep no SeedQR,
/// and the sheet offers a seed's SeedQR picture only where the plan keeps
/// it on paper: each seed's words chip on paper takes the SeedQR form, by
/// the chip's own toggle.
fn seedqr_on_paper(c: &Compute, t: &mut Tour) -> Result<(), String> {
    for i in bshape(t).map(|sh| sh.here()).unwrap_or_default() {
        let place = (0..t.app.board_plan().map_or(0, |(_, a)| a.places)).find(|&p| {
            bmap(c, t)
                .iter()
                .any(|s| s.at == At::Place(p) && s.holds.iter().any(|(w, _)| *w == What::Words(i)))
        });
        let Some(p) = place else {
            return Err(format!("seed {i} has no words on paper"));
        };
        t.press(Action::Board(B::Chip(Chip::Placed(
            Zone::Paper(p),
            Thing::Seed(i),
        ))));
        t.press(Action::Board(B::Form(Zone::Paper(p), Thing::Seed(i), 1)));
        t.press(Action::Board(B::LetGo));
    }
    Ok(())
}
