//! The boot tour.

use crate::tour::{
    Tour, boff, bput, description_file_at, device_key, keep_vault, test_key_1, type_text,
    type_words, vault_place, vault_stick,
};
use faraday_core::board::{Thing, Zone};
use faraday_core::explore::{Page, XAction};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// Costs what `vault_place`, `keep_vault`, `vault_off` and the rest it
/// calls cost. A fresh session at boot: the boot stick's files are
/// copied into memory and the sheet lists them, each ticked (F60);
/// pulling the stick leaves the sheet as it was. Import brings them in
/// and puts the vault's passphrase on the sheet. A wrong passphrase
/// stays; Later leaves Home leading with the vault's Unlock; the right
/// passphrase loads the vault's wallets and keys. Then a wallet made
/// here is saved into the vault.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn boot_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::Sheet;
    use faraday_core::boot_import::ImportAction as I;
    t.press(Action::Lock);
    let mut boot = t.stick();
    boot.boot = true;
    boot.label = "FARADAY".into();
    t.app.storage(StorageEvent::Sticks(vec![boot]));
    t.pump();
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    if t.app.screen != Screen::Home
        || t.app.sheet != Some(Sheet::Import)
        || t.app.inbox.iter().any(|i| i.name == "vault.ofv")
    {
        return Err("the boot stick's files did not come up for the import".into());
    }
    t.shot("boot-import-stick")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    if t.app.sheet != Some(Sheet::Import) || t.app.import_field().is_some() {
        return Err("pulling the boot stick did not leave the sheet's list".into());
    }
    let (w, h) = t.size();
    t.app.event(Event::Scroll {
        x: w / 2,
        y: h / 2,
        dy: i16::MAX,
    });
    t.shot("boot-import-end")?;
    t.press(Action::Import(I::Go));
    if t.app.sheet != Some(Sheet::Import) || t.app.import_field().is_none() {
        return Err("Import did not put the vault's passphrase on the sheet".into());
    }
    t.shot("boot-import")?;
    // A wrong passphrase stays on the sheet and says so. A tap on the
    // field first, which on a small panel brings the keyboard up.
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, "not the passphrase");
    t.press(Action::Import(I::Submit));
    t.tick();
    if t.app.sheet != Some(Sheet::Import) || t.app.vaults.unlock_error.is_none() {
        return Err("a wrong passphrase did not stay on the sheet".into());
    }
    t.shot("boot-import-wrong")?;
    // Later leaves Home leading with the vault's Unlock, which opens the
    // sheet again.
    t.press(Action::Import(I::Later));
    t.shot("home-import-waiting")?;
    t.press(faraday_core::boot_import::OPEN);
    if t.app.sheet != Some(Sheet::Import) {
        return Err("the import did not open again from Home".into());
    }
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.shot("boot-unlock")?;
    t.press(Action::Import(I::Submit));
    t.tick();
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    // Its Load sheet (F62), then Load ticked; none when everything it
    // holds is loaded already.
    if t.app.sheet == Some(Sheet::UnlockLoad) {
        t.shot("boot-load-sheet")?;
        t.press(Action::Vault(V::LoadTicked));
    }
    if t.app.vaults.open.len() != 1
        || t.app.import.is_some()
        || t.app.sheet.is_some()
        || t.app.screen != Screen::Wallets
    {
        return Err("unlocking on the import sheet did not import and land on Wallets".into());
    }
    if t.app.session.wallets.len() < 12 || t.app.session.keys.is_empty() {
        return Err(format!(
            "the import gave {} wallets and {} keys",
            t.app.session.wallets.len(),
            t.app.session.keys.len()
        ));
    }
    t.shot("boot-imported")?;
    t.press(Action::Nav(Screen::Files));
    t.shot("boot-loaded")?;
    // A 2-of-3 made here over test key 1 and two new keys, saved into
    // the open vault with the new keys.
    t.press(Action::CreateWallet);
    t.press(Action::CKind(4));
    t.press(Action::CNext(0));
    t.press(Action::CNext(1));
    // Test key 1, which the vault holds.
    let bacon = test_key_1(t)?;
    t.press(Action::CSlotHere(0, bacon));
    device_key(t, 1);
    device_key(t, 2);
    t.press(Action::CNext(faraday_core::cstep::KEYS));
    t.press(Action::CNext(faraday_core::cstep::CHECK));
    // After the check, the backup plan: Paper and vault, every seed
    // ticked into Vault 1, puts the new keys and the wallet into the open
    // vault from the checklist.
    let built = t.app.create.as_ref().and_then(|c| c.built).unwrap_or(0);
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    let here = t.app.plan_shape_now(built).here();
    let vaults = t
        .app
        .backup
        .as_ref()
        .map_or(0, |b| b.answers.vault_count(&t.app.plan_shape_now(built)));
    let p = vault_place(c, t, 0).unwrap_or(0);
    keep_vault(c, t, p, 0, &here);
    for v in 1..vaults {
        vault_off(c, t, v);
    }
    // The description into the vault, and as files too, for the public
    // files item below.
    description_into_vault(c, t, 0);
    description_file_at(c, t, 0);
    t.press(Action::BChecklist);
    // The items before the vault's wait for it (`docs/history/NEW-WALLET.md`
    // §14.1): each copy by hand checked in turn, then the vault.
    items_before(c, t, faraday_core::bstep::VAULT)?;
    if t.app.backup.as_ref().and_then(|b| b.open) != Some(faraday_core::bstep::VAULT) {
        t.press(Action::BStep(faraday_core::bstep::VAULT));
    }
    // Making the checklist saved the plan into the vault already.
    use faraday_vault::records::kind;
    let count = |t: &Tour, k: u8| t.app.vaults.records(t.app.vaults.open[0].id).of(k).count();
    let before = t.app.vaults.open[0].changes;
    let plans = count(t, kind::PLAN);
    let keys = count(t, kind::KEY);
    let wallets = count(t, kind::WALLET);
    t.shot("create-backup-vault")?;
    // One Save into the vault saves all the plan puts there
    // (`docs/history/NEW-WALLET.md` §14.1): the two new keys and the wallet;
    // test key 1 was there already. The plan record the checklist wrote
    // is kept as it is, one of it: nothing in it changed.
    t.press(Action::BVaultSave(0));
    t.shot("create-backup-vault-saved")?;
    let saved = t.app.vaults.open[0].changes - before;
    if saved != 3 {
        return Err(format!("the checklist saved {saved} records, not 3"));
    }
    if count(t, kind::KEY) != keys + 2 || count(t, kind::WALLET) != wallets + 1 {
        return Err("the vault's save did not add the two new keys and the wallet".into());
    }
    if plans == 0 || count(t, kind::PLAN) != plans {
        return Err(format!(
            "the vault holds {} plan records, {plans} before the save",
            count(t, kind::PLAN)
        ));
    }
    // Then the public files, to the Outbox.
    t.press(Action::BStep(faraday_core::bstep::PUBLIC));
    t.press(Action::PublicOut(built, 1));
    t.press(Action::PublicOut(built, 2));
    t.shot("create-backup-public")?;
    let outbox: Vec<_> = t.app.outbox.iter().map(|i| i.kind.exposure()).collect();
    if outbox.contains(&faraday_core::secrets::Exposure::Secret) {
        return Err("a secret reached the Outbox from Create".into());
    }
    println!("boot: imported from the boot stick, a new wallet saved into its vault");
    // BIP-85: a password from test key 1, into the open vault.
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Bip85);
    if t.app.bip85().is_none() {
        return Err("BIP-85 did not open".into());
    }
    let first = bacon;
    t.press(Action::PKey(first));
    t.press(Action::PApp(4));
    t.shot("bip85-app")?;
    t.press(Action::PNext);
    t.press(Action::PNext);
    t.press(Action::PShow);
    t.shot("bip85-value")?;
    t.press(Action::PVault);
    // Silent payments: test key 1's address with label 1.
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Silent);
    t.press(Action::SKey(first));
    t.press(Action::SLabel(1));
    t.shot("silent-address")?;
    // Explore: test key 1 at BIP-86's first address.
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("start-tools")?;
    t.press(Action::Explore);
    t.press(Action::XPreset(1));
    t.shot("explore")?;
    // Its account's addresses, and a search of them for a part (F76).
    t.press(Action::X(XAction::Page(Page::Addresses)));
    t.shot("explore-addresses")?;
    t.press(Action::X(XAction::Page(Page::Find)));
    type_text(t, "x9");
    t.press(Action::X(XAction::Search));
    let searching = |t: &Tour| {
        t.app
            .explore()
            .and_then(|e| e.search.as_ref())
            .is_some_and(|s| s.running())
    };
    for _ in 0..100 {
        if !searching(t) {
            break;
        }
        t.tick();
    }
    t.shot("explore-find")?;
    // Lightning: test key 1 as ldk-node reads it.
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Lightning);
    t.press(Action::LKey(first));
    t.shot("lightning")?;
    // Tools: a key in every spelling.
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Tools);
    t.press(Action::TTool(3));
    type_text(
        t,
        "xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V",
    );
    t.shot("tools-convert")?;
    // Restore begins with the kind of wallet (`docs/history/NEW-WALLET.md` §12).
    t.press(Action::Lock);
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("start-empty")?;
    t.press(Action::RestoreWallet);
    t.shot("restore-kind")?;
    t.press(Action::RNext(faraday_core::rstep::KIND));
    t.shot("restore-sources")?;
    Ok(())
}

/// Costs what `backup_waiting` costs. Does each checklist item before
/// `target` in the checklist's order, as a person must
/// (`docs/history/NEW-WALLET.md` §14.1): a copy by hand is checked by its words
/// typed back, the envelopes by their Continue.
pub(crate) fn items_before(c: &Compute, t: &mut Tour, target: u8) -> Result<(), String> {
    use faraday_core::bstep;
    while let Some(n) = t.app.backup_waiting(c) {
        if n == target {
            break;
        }
        if t.app.backup.as_ref().and_then(|b| b.open) != Some(n) {
            t.press(Action::BStep(n));
        }
        if bstep::is_copy(n) {
            t.press(Action::BCheck);
            type_words(t, None)?;
        }
        t.press(Action::BNext(n));
        if t.app.backup_waiting(c) == Some(n) {
            return Err(format!("checklist item {n} was not done in turn"));
        }
    }
    Ok(())
}

/// Costs a map of the plan (through `vault_stick`). Vault `v`'s stick
/// taken off every place.
pub(crate) fn vault_off(c: &Compute, t: &mut Tour, v: usize) {
    let places = t.app.board_plan().map_or(0, |(_, a)| a.places);
    for p in 0..places {
        while let Some(s) = vault_stick(c, t, p, v) {
            boff(t, Zone::Files(p, s), Thing::Vault(v));
        }
    }
}

/// Costs a map of the plan (through `vault_place` and `vault_stick`).
/// The whole description sealed in vault `v`, at the first place that
/// keeps its stick.
pub(crate) fn description_into_vault(c: &Compute, t: &mut Tour, v: usize) {
    if let Some(p) = vault_place(c, t, v)
        && let Some(s) = vault_stick(c, t, p, v)
    {
        bput(t, Thing::Whole, Zone::Vault(p, s, v));
    }
}
