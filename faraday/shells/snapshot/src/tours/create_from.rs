//! The create_from tour.

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_crypto::work::Compute;

/// Costs what `add_words`, `key_text` and `add_wallet` cost. Keys from
/// a vault on Create's Keys card, and New wallet from this one
/// (`docs/history/NEW-WALLET.md` §16): a 2-of-3's Keys with the test vault
/// locked, then unlocked from there with test key 1 offered under its
/// name; a 2-of-3 over the vault's key, test key 2 here and test key
/// 3's xpub, its card with New wallet from this one, and Create filled
/// from it: Kind with the Account stepper, then Keys with key 3 waiting
/// for its xpub at account 1.
pub(crate) fn create_from_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    let (word, label) = testkit::TEST_SEEDS[1];
    t.app
        .session
        .add_words(c, &testkit::test_words(word), label, None)
        .map_err(|e| e.text())?;
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.press(Action::CNext(cstep::QUORUM));
    t.shot("create-keys-vault-locked")?;
    // A small panel's page, down to what the change put there.
    let down = |t: &mut Tour, action: Action, name: &str| -> Result<(), String> {
        if t.app.is_compact() {
            scroll_to(t, action);
            t.shot(name)?;
        }
        Ok(())
    };
    let unlock = Action::Vault(V::OpenFrom(0, Screen::Create));
    down(t, unlock, "create-keys-vault-locked-down")?;
    t.press(Action::Vault(V::OpenFrom(0, Screen::Create)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults.open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.screen != Screen::Create {
        return Err("unlocking did not come back to Create".into());
    }
    t.shot("create-keys-vault-open")?;
    let record = t.app.vaults.open[0]
        .contents
        .records
        .iter()
        .position(|r| r.kind == faraday_vault::records::kind::KEY)
        .ok_or("no key in the test vault")?;
    down(
        t,
        Action::CSlotVault(0, 0, record),
        "create-keys-vault-open-down",
    )?;
    // The vault's key and test key 3 worked out apart from the session.
    let mut apart = faraday_core::wallet::Session::default();
    for n in [0, 2] {
        let (word, label) = testkit::TEST_SEEDS[n];
        apart
            .add_words(c, &testkit::test_words(word), label, None)
            .map_err(|e| e.text())?;
    }
    let keys: Vec<String> = [
        apart.keys[0].master(),
        t.app.session.keys[0].master(),
        apart.keys[1].master(),
    ]
    .iter()
    .map(|m| NewKind::Multi.key_text(c, m))
    .collect::<Result<_, _>>()?;
    let i = t
        .app
        .session
        .add_wallet(
            c,
            "Shared",
            &NewKind::Multi.descriptor(2, &keys),
            "Test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    t.press(Action::OpenWallet(i));
    t.shot("wallet-new-from")?;
    down(t, Action::NewWalletFrom(i), "wallet-new-from-down")?;
    t.press(Action::NewWalletFrom(i));
    t.press(Action::CStep(cstep::KIND));
    t.shot("create-from-kind")?;
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.shot("create-from-keys")?;
    down(t, Action::CSlotClear(2), "create-from-keys-down")?;
    // A fresh single-key wallet: Check's Path, and Change back to Kind's
    // Account (`docs/history/NEW-WALLET.md` §17).
    t.press(Action::CreateOver);
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    let fp = t.app.session.keys[0].fingerprint().0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.shot("create-check-path")?;
    down(t, Action::CPathChange, "create-check-path-down")?;
    t.press(Action::CPathChange);
    t.press(Action::CAccount(1));
    t.shot("create-path-kind-account")
}
