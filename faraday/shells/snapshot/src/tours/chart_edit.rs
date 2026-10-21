//! The chart_edit tour.

use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_crypto::work::Compute;
use crate::tour::{Tour, scroll, type_text};

/// Costs what `key_text`, `add_wallet`, `of_now` and the rest it calls
/// cost. The chart's changes (`docs/history/NEW-WALLET.md` §9.5–§9.7), on a
/// one-key wallet with a vault open and the Paper and vault plan made
/// with it: the vault's stick moved to Place 1, asked with the check
/// before and after, and the chart once moved; the words at Place 2
/// destroyed, asked, a line made worse; a line marked lost and a place
/// marked exposed, with what follows; the vault copied to a stick at a
/// new place; and on a 2-of-3, the cosigner's holder typed and the card
/// with it.
pub(crate) fn chart_edit_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::glance::sheet::{ChartAction as C, Target};
    use faraday_core::glance::{Backup, Press};
    use faraday_core::plan::{At, Edit, Mark, What};
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Entry(None));
    t.type_key(0);
    let one = {
        let text = NewKind::NativeSegwit
            .key_text(c, t.app.session.keys[0].master())
            .map_err(|e| e.to_string())?;
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
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "chart phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "chart phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults.open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.vaults.open.is_empty() {
        return Err("the vault made for the chart tour did not open".to_string());
    }
    t.press(Action::Backup(one));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    t.press(Action::BVaultSave(0));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::AddKind(1)));
    t.press(Action::Vault(V::SaveWallet(one)));
    let press = Press::Loaded(one);
    t.press(Action::OpenWallet(one));
    // The card, or on a small panel the chart's own page.
    let shot_chart = |t: &mut Tour, w: usize, name: &str| -> Result<(), String> {
        t.press(Action::OpenWallet(w));
        if t.app.is_compact() {
            t.press(Action::Glance(true));
            t.shot(name)?;
            scroll(t, 500);
            t.shot(&format!("{name}-2"))?;
            t.press(Action::Glance(false));
        } else {
            t.shot(name)?;
            scroll(t, 400);
            t.shot(&format!("{name}-foot"))?;
        }
        Ok(())
    };
    // A move: the vault's stick to Place 1, asked, then made.
    let stick = Edit::Move(At::Place(1), What::VaultStick(0), At::Place(0));
    t.press(Action::Chart(C::Edit(press, stick)));
    t.shot("chart-move-ask")?;
    t.press(Action::Chart(C::Confirm(press)));
    shot_chart(t, one, "chart-moved")?;
    // The words at Place 1 destroyed: asked, a line worse.
    t.press(Action::Chart(C::Edit(
        press,
        Edit::Drop(At::Place(0), What::Words(0)),
    )));
    t.shot("chart-drop-ask")?;
    t.press(Action::Cancel);
    let node = |t: &Tour, at: At| -> Option<(u8, Vec<What>)> {
        let g = faraday_core::glance::of_now(c, &t.app, one)?;
        let Backup::Plan { nodes, .. } = g.backup else {
            return None;
        };
        nodes
            .iter()
            .position(|n| n.at == at)
            .map(|i| (i as u8, nodes[i].whats.clone()))
    };
    // The words at Place 2 marked lost: they are at Place 1 too.
    if let Some((n, whats)) = node(t, At::Place(1))
        && let Some(j) = whats.iter().position(|w| *w == What::Words(0))
    {
        t.press(Action::Chart(C::Mark(press, n, Some(j as u8), Mark::Lost)));
        t.shot("chart-lost")?;
        t.press(Action::Cancel);
    }
    // Place 1, with the words and the vault's stick, marked exposed.
    if let Some((n, _)) = node(t, At::Place(0)) {
        t.press(Action::Chart(C::Mark(press, n, None, Mark::Exposed)));
        t.shot("chart-exposed")?;
        t.press(Action::Cancel);
    }
    shot_chart(t, one, "chart-marked")?;
    // The vault copied to a stick at a new place.
    t.press(Action::Chart(C::Open(
        press,
        Target::CopyTo(What::VaultStick(0)),
    )));
    t.shot("chart-vault-copy-to")?;
    t.press(Action::Chart(C::Edit(
        press,
        Edit::Add(At::Place(2), What::VaultStick(0)),
    )));
    t.shot("chart-vault-copy-ask")?;
    t.press(Action::Chart(C::Confirm(press)));
    t.press(Action::Cancel);
    shot_chart(t, one, "chart-vault-copy")?;
    // A 2-of-3 with test key 3 a cosigner's: its holder named.
    t.press(Action::Entry(None));
    t.type_key(1);
    let mut elsewhere = faraday_core::wallet::Session::default();
    elsewhere
        .add_words(
            c,
            &testkit::test_words(testkit::TEST_SEEDS[2].0),
            "Test key 3",
            None,
        )
        .map_err(|e| e.text())?;
    let masters = [
        &t.app.session.keys[0].master(),
        &t.app.session.keys[1].master(),
        &elsewhere.keys[0].master(),
    ];
    let keys: Vec<String> = masters
        .iter()
        .map(|m| NewKind::Multi.key_text(c, m))
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
    t.press(Action::Backup(two));
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.press(Action::BChecklist);
    let press = Press::Loaded(two);
    t.press(Action::OpenWallet(two));
    t.press(Action::Chart(C::Open(press, Target::Holder(2))));
    type_text(t, "Alice's Coldcard");
    t.shot("chart-holder")?;
    t.press(Action::Chart(C::Save(press)));
    t.press(Action::Chart(C::KeyChecked(press, 2)));
    t.press(Action::Chart(C::Open(press, Target::Key(2))));
    t.shot("chart-holder-key")?;
    t.press(Action::Cancel);
    shot_chart(t, two, "chart-holder-card")?;
    Ok(())
}
