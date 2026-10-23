//! The spend tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_shell_api::{App, Event};

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn spend_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::{FamilyAction as F, page};
    let fam = |a: F| Action::Family(a);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Family));
    t.shot("spend-map")?;
    t.press(fam(F::More(page::MAP)));
    t.shot("spend-map-more")?;
    t.press(fam(F::Next(page::MAP)));
    t.shot("spend-stick")?;
    t.press(fam(F::Next(page::SAFE)));
    t.press(fam(F::Unsure));
    t.shot("spend-holding-unsure")?;
    t.press(fam(F::Unsure));
    // The vault and the transaction from a stick: the visit, and back.
    t.press(Action::Nav(Screen::Family));
    t.sticks(true);
    t.press(Action::VisitInAll);
    for name in ["vault.ofv", "savings-unsigned.psbt"] {
        if let Some(i) = t.stick().files.iter().position(|(n, _)| n == name) {
            t.press(Action::VisitIn(i));
        }
    }
    t.press(Action::VisitCopy);
    t.shot("spend-visit")?;
    t.sticks(false);
    if t.app.screen != Screen::Family {
        return Err("pulling the stick did not return to the Spend tab".into());
    }
    t.press(fam(F::Pick(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.shot("spend-vault-passphrase")?;
    t.press(fam(F::Unlock));
    t.tick();
    t.tick();
    t.shot("spend-vault-list")?;
    t.press(Action::Vault(V::LoadChosen(0)));
    t.shot("spend-vault-opened")?;
    let savings = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Savings")
        .ok_or("the vault did not load Savings")?;
    // The PSBT in Files chose Savings: What do you have? is done.
    if t.app.family_wallet() != Some(savings) {
        return Err("the PSBT in Files did not choose Savings".into());
    }
    t.shot("spend-check")?;
    t.press(fam(F::Next(page::CHECK)));
    t.shot("spend-write")?;
    t.press(fam(F::Next(page::WRITE)));
    t.shot("spend-bring")?;
    let at = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .ok_or("the PSBT is not in Files")?;
    t.press(fam(F::UsePsbt(at)));
    t.shot("spend-transaction")?;
    // Check is already closed: "Yes, my money shows" on the Spend tab's
    // own check page marked this wallet checked this power-on.
    t.press(Action::StepNext(faraday_core::wallet::step::TRANSACTION));
    t.shot("spend-signers")?;
    // Who signs' Scan a key, and its Type the words instead.
    t.press(Action::ScanKey(savings));
    t.press(Action::EntryWallet(savings));
    t.type_key(1);
    t.shot("spend-signers-two")?;
    t.press(Action::StepNext(faraday_core::wallet::step::SIGNERS));
    t.press(Action::SignHere);
    t.shot("spend-signed")?;
    t.press(Action::StepNext(faraday_core::wallet::step::SIGN));
    t.shot("spend-collect")?;
    t.press(Action::StepNext(faraday_core::wallet::step::COLLECT));
    t.shot("spend-send")?;
    t.press(Action::StepNext(faraday_core::wallet::step::FINISH));
    t.shot("spend-away")?;
    println!("spend: vault unlocked on the tab, Savings signed by two keys and finished");
    // A vanity address for test key 1, on its account number.
    use faraday_core::vanity::{VanityAction as Va, vstep};
    t.press(Action::Vanity(Va::Open));
    let first = t.app.session.keys[0].fingerprint.0;
    t.press(Action::Vanity(Va::Key(first)));
    t.shot("vanity-dial")?;
    t.press(Action::Vanity(Va::Next(vstep::DIAL)));
    t.press(Action::Vanity(Va::Next(vstep::SCRIPT)));
    type_text(t, "qq");
    t.shot("vanity-prefix")?;
    t.press(Action::Vanity(Va::Next(vstep::PREFIX)));
    t.press(Action::Vanity(Va::Start));
    // Ticks a tenth of a second apart, as a shell sends them: well inside
    // the idle lock.
    for _ in 0..4000 {
        t.now += 100;
        t.app.event(Event::Tick { now_ms: t.now });
        t.pump();
        if t.app.vanity().is_some_and(|v| v.grind.find.is_some()) {
            break;
        }
    }
    if t.app.vanity().is_none_or(|v| v.grind.find.is_none()) {
        let v = t.app.vanity();
        return Err(format!(
            "the vanity search found nothing: running {:?} tried {:?} prefix {:?} error {:?} screen {:?}",
            v.map(|v| v.running),
            v.map(|v| v.grind.tested),
            v.map(|v| v.grind.prefix.as_str().to_string()),
            v.and_then(|v| v.error.clone()),
            t.app.screen
        ));
    }
    t.shot("vanity-found")?;
    println!("vanity: test key 1's account dial found an address beginning tb1qqq");
    Ok(())
}
