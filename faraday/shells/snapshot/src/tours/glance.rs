//! The glance tour.

use crate::tour::{Tour, bmap, bput, scroll, type_text, vault_place, vault_stick};
use faraday_core::board::{Thing, Zone};
use faraday_core::plan::{At, What};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// Costs a map of the plan. Seed `i`'s passphrase sealed with it in
/// each vault that holds it.
pub(crate) fn pass_into_vault(c: &Compute, t: &mut Tour, i: usize) {
    let vaults: Vec<usize> = bmap(c, t)
        .iter()
        .filter_map(|s| match s.at {
            At::Vault(v) if s.holds.iter().any(|(w, _)| *w == What::Seed(i)) => Some(v),
            _ => None,
        })
        .collect();
    for v in vaults {
        if let Some(p) = vault_place(c, t, v)
            && let Some(s) = vault_stick(c, t, p, v)
        {
            bput(t, Thing::Pass(i), Zone::Vault(p, s, v));
        }
    }
}

/// Costs what `key_text`, `add_wallet`, `add_words_with` and the rest
/// it calls cost. The Wallets card at a glance: a one-key wallet with
/// no plan, then with Paper and vault; a 2-of-3 with test keys 1 and 2
/// here, key 2 with a passphrase, and test key 3 a cosigner, with Paper
/// and vault and key 2's passphrase in its vault too. On a small panel,
/// each card's At a glance row and the chart's page. The sheets a press
/// on the chart opens (`docs/history/NEW-WALLET.md` §9): the wallet, a key
/// here, a cosigner's key, a place, a vault, a line inside a place;
/// Where it is; and in the open vault's view, a place's.
pub(crate) fn glance_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::{ChartAction as C, Target};
    let shot_sheet = |t: &mut Tour, press: Press, target: Target, name: &str| {
        t.press(Action::Chart(C::Open(press, target)));
        let shot = t.shot(name);
        t.press(Action::Cancel);
        shot
    };
    t.press(Action::Entry(None));
    t.type_key(0);
    let one = {
        let text = crate::tour::key_text_of(c, &t.app.session.keys, 0, NewKind::NativeSegwit)?;
        t.app
            .session
            .add_wallet(
                c,
                "Spending",
                &format!("wpkh({text}/<0;1>/*)"),
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .map_err(|e| e.text())?
    };
    let shot_card = |t: &mut Tour, w: usize, name: &str| -> Result<(), String> {
        t.press(Action::OpenWallet(w));
        t.shot(name)?;
        if t.app.is_compact() {
            t.press(Action::Glance(true));
            t.shot(&format!("{name}-page"))?;
            for k in 1..=3 {
                scroll(t, 500);
                t.shot(&format!("{name}-page-{k}"))?;
            }
            t.press(Action::Glance(false));
        } else {
            scroll(t, 400);
            t.shot(&format!("{name}-foot"))?;
        }
        Ok(())
    };
    shot_card(t, one, "glance-no-plan")?;
    t.press(Action::Backup(one));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    shot_card(t, one, "glance-one-key")?;
    let press = Press::Loaded(one);
    t.press(Action::OpenWallet(one));
    shot_sheet(t, press, Target::Wallet, "glance-sheet-wallet")?;
    shot_sheet(t, press, Target::Key(0), "glance-sheet-key")?;
    shot_sheet(t, press, Target::Node(0), "glance-sheet-place")?;
    shot_sheet(t, press, Target::Node(2), "glance-sheet-vault")?;
    shot_sheet(t, press, Target::Line(0, 0), "glance-sheet-words")?;
    // Test key 2 with a passphrase, test key 3 a cosigner's.
    t.app
        .session
        .keys
        .add_words_with(
            c,
            &testkit::test_words(testkit::TEST_SEEDS[1].0),
            "glance",
            "Test key 2",
            None,
        )
        .map_err(|e| e.text())?;
    let mut elsewhere = faraday_core::wallet::Session::default();
    elsewhere
        .keys
        .add_words(
            c,
            &testkit::test_words(testkit::TEST_SEEDS[2].0),
            "Test key 3",
            None,
        )
        .map_err(|e| e.text())?;
    let keys: Vec<String> = [
        (&t.app.session.keys, 0),
        (&t.app.session.keys, 1),
        (&elsewhere.keys, 0),
    ]
    .iter()
    .map(|(k, i)| crate::tour::key_text_of(c, k, *i, NewKind::Multi))
    .collect::<Result<_, _>>()?;
    let two = t
        .app
        .session
        .add_wallet(
            c,
            "Savings",
            &NewKind::Multi.descriptor(2, &keys),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    shot_card(t, two, "glance-two-of-three-no-plan")?;
    t.press(Action::Backup(two));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    // Key 2's passphrase into its vault too.
    pass_into_vault(c, t, 1);
    t.press(Action::BChecklist);
    shot_card(t, two, "glance-two-of-three")?;
    let press = Press::Loaded(two);
    t.press(Action::OpenWallet(two));
    shot_sheet(t, press, Target::Key(2), "glance-sheet-cosigner")?;
    if !t.app.is_compact() {
        t.press(Action::Chart(C::Where(press, 1)));
        t.shot("glance-where")?;
    }
    // The open vault's view (§7.2): a vault made and open, the one-key
    // wallet's plan made with it, its seed and the wallet saved into it;
    // the wallet's chart from the backup up.
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "glance phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "glance phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.vaults().open.is_empty() {
        return Err("the vault made for the glance tour did not open".to_string());
    }
    t.press(Action::Backup(one));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    t.press(Action::BVaultSave(0));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::AddKind(1)));
    t.press(Action::Vault(V::SaveWallet(one)));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(1)));
    t.press(Action::Vault(V::Item(0)));
    let shot_item = |t: &mut Tour, name: &str| -> Result<(), String> {
        t.shot(name)?;
        if t.app.is_compact() {
            for k in 1..=3 {
                scroll(t, 500);
                t.shot(&format!("{name}-{k}"))?;
            }
        } else {
            // The item's own region, right of the list.
            let _ = t.app.frame();
            let (w, h) = t.size();
            t.app.event(Event::Scroll {
                x: w - w / 5,
                y: h / 2,
                dy: 600,
            });
            t.shot(&format!("{name}-foot"))?;
        }
        Ok(())
    };
    shot_item(t, "glance-vault")?;
    // The 2-of-3's plan made with the vault open, and the wallet saved.
    t.press(Action::Backup(two));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    // Key 2's passphrase into its vault too.
    pass_into_vault(c, t, 1);
    t.press(Action::BChecklist);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::AddKind(1)));
    t.press(Action::Vault(V::SaveWallet(two)));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(1)));
    t.press(Action::Vault(V::Item(1)));
    shot_item(t, "glance-vault-two-of-three")?;
    if let Some(r) = t.app.vault_selected_index() {
        t.press(Action::Vault(V::Item(1)));
        shot_sheet(
            t,
            Press::Vault(t.app.vaults().current, r),
            Target::Node(0),
            "glance-vault-sheet-place",
        )?;
    }
    Ok(())
}
