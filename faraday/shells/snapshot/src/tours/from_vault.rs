//! The from_vault tour.

use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};
use crate::tour::{Tour, type_text};

/// Costs what `psbt_for`, `add_words` and `parse` cost. From the vault
/// to a spend (`docs/history/NEW-WALLET.md` §11): the boot stick's vault
/// unlocked after the pull lands on Wallets with a toast; a wallet's
/// card leads with Spend from this wallet, which opens the Spend tab on
/// Load the wallet in Sparrow with the shortcut; a 2-of-3 with one key
/// here, its PSBT in Files, reaches Signers with Scan a key and Collect
/// a signature; the card's Scan a key opens the key scanner, and a
/// missing key's SeedQR makes the wallet able to spend.
pub(crate) fn from_vault_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::boot_import::ImportAction as I;
    use faraday_core::family::{FamilyAction as F, page};
    t.press(Action::Lock);
    // The vault and the one PSBT Sparrow wrote, for Savings; no seed
    // beside them, so Savings has only the vault's key here.
    let mut boot = t.stick();
    boot.boot = true;
    boot.label = "FARADAY".into();
    boot.files
        .retain(|(n, _)| n == "vault.ofv" || n == "savings-unsigned.psbt");
    t.app.storage(StorageEvent::Sticks(vec![boot]));
    t.pump();
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.press(Action::Import(I::Go));
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Import(I::Submit));
    t.tick();
    if t.app.screen != Screen::Wallets {
        return Err(format!("the unlock landed on {:?}", t.app.screen));
    }
    t.shot_after("vault-landing", 0)?;
    let at = |t: &Tour, name: &str| {
        t.app
            .session
            .wallets
            .iter()
            .position(|w| w.name == name)
            .ok_or_else(|| format!("{name} is not loaded"))
    };
    let spending = at(t, "Spending")?;
    t.press(Action::OpenWallet(spending));
    t.shot("vault-card-spend")?;
    t.press(Action::Family(F::SpendFrom(spending)));
    t.shot("vault-spend-first")?;
    t.press(Action::Family(F::Next(page::CHECK)));
    t.shot("vault-spend-write")?;
    // A wallet of several keys with fewer here than sign, and its PSBT.
    let short = (0..t.app.session.wallets.len()).find(|&w| {
        let wl = &t.app.session.wallets[w];
        let here = t
            .app
            .session
            .slots(wl)
            .iter()
            .filter(|s| s.held_by.is_some())
            .count();
        here < faraday_core::wallet::needed(wl) && t.app.psbt_for(c, w).is_some()
    });
    let Some(short) = short else {
        return Err("no wallet short of keys has its PSBT in Files".into());
    };
    t.press(Action::OpenWallet(short));
    t.shot("vault-card-sign")?;
    t.press(Action::Family(F::SpendFrom(short)));
    t.press(Action::Step(faraday_core::wallet::step::SIGNERS));
    t.shot("vault-spend-signers")?;
    // The card's Scan a key: the key scanner, then a missing key's
    // SeedQR, which loads at once, and the card can spend.
    t.press(Action::OpenWallet(short));
    t.press(Action::ScanKey(short));
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: (0..640u32 * 480).map(|i| ((i % 640) / 3) as u8).collect(),
        chroma: None,
    });
    t.shot("wallet-scan-key")?;
    let missing: Vec<[u8; 4]> = {
        let wl = &t.app.session.wallets[short];
        t.app
            .session
            .slots(wl)
            .iter()
            .filter(|s| s.held_by.is_none())
            .filter_map(|s| s.fingerprint.map(|f| f.0))
            .collect()
    };
    let words = testkit::TEST_SEEDS
        .iter()
        .map(|(w, _)| testkit::test_words(w))
        .find(|w| {
            let mut s = testkit::session();
            s.add_words(c, w, "", None)
                .is_ok_and(|fp| missing.contains(&fp.0))
        })
        .ok_or("no test seed is a missing key")?;
    let m = faraday_bip::bip39::Mnemonic::parse(c, faraday_bip::bip39::Language::English, &words)
        .map_err(|e| format!("{e:?}"))?;
    let digits = faraday_codec::seedqr::to_digits(&m);
    t.app.event(Event::Scanned {
        bytes: digits.expose().as_bytes().to_vec(),
    });
    t.pump();
    if !t.app.can_spend(short) {
        return Err("the scanned key did not make the wallet able to spend".into());
    }
    t.shot("vault-card-can-spend")?;
    println!("Scan a key on a wallet's card loaded its missing key at once");
    Ok(())
}
