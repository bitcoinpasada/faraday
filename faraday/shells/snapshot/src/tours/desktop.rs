//! The desktop tours.

use crate::tour::{Tour, type_text};
use faraday_core::samples::Pick;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};

/// The desktop app's stick pills: the default two, then a wallet's backup
/// with five vaults.
pub(crate) fn pills_tour(t: &mut Tour) -> Result<(), String> {
    t.app.shell_sticks = true;
    t.app.online = true;
    let stick = |name: &str| StickInfo {
        id: name.into(),
        label: name.into(),
        boot: name == "BOOTSTICK",
        serial: None,
        files: Vec::new(),
    };
    t.app.storage(StorageEvent::Sticks(vec![
        stick("BOOTSTICK"),
        stick("STICK-2"),
    ]));
    t.press(Action::Nav(Screen::Home));
    t.shot("pills-default")?;
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .unwrap_or(0) as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CVaults(true));
    t.press(Action::CNext(faraday_core::cstep::WHERE));
    t.press(Action::CVaultN(5));
    t.press(Action::CNext(faraday_core::cstep::HOW_MANY));
    t.shot("pills-five-vaults")
}

/// The demo's dimmed actions (`docs/DEMO.md` §7): BIP-85's Value card
/// and the Lightning node key, on a sample.
pub(crate) fn demo_tour(t: &mut Tour) -> Result<(), String> {
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::on(testkit::NET);
    t.app
        .session
        .set_demo(faraday_core::testkit::demo_samples());
    t.press(Action::Entry(None));
    type_text(t, &faraday_core::testkit::test_words("bacon"));
    t.press(Action::EntryAdd);
    let fp = t
        .app
        .session
        .keys
        .first()
        .ok_or("bacon was not loaded")?
        .fingerprint
        .0;
    t.press(Action::Bip85);
    t.press(Action::PKey(fp));
    t.press(Action::PNext);
    t.press(Action::PNext);
    t.shot("demo-bip85-value")?;
    t.press(Action::Lightning);
    t.press(Action::LKey(fp));
    t.shot("demo-lightning")
}

/// The mode pill and sheet (`docs/MODES.md`
/// §3): Home and the sheet in Demo and on Testnet, then Settings' Mode
/// row on Mainnet, where no pill shows.
pub(crate) fn modes_tour(t: &mut Tour) -> Result<(), String> {
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::on(testkit::NET);
    t.app.session.set_demo(&faraday_samples::SAMPLES);
    t.app.samples = Some(&faraday_samples::SAMPLES);
    t.app.set_modes(faraday_core::DESKTOP_MODES);
    t.press(Action::Nav(Screen::Home));
    t.shot("modes-demo-home")?;
    t.press(Action::ModeSheet);
    t.shot("modes-demo-sheet")?;
    t.press(Action::Cancel);
    // Where a seed is typed, online, in Demo and on testnet: one line
    // under the field.
    t.press(Action::Entry(None));
    // Use a sample: the chip in the empty grid, pressed, and the share step
    // of a set with its chip for the second share.
    t.shot("modes-demo-add-key-sample")?;
    t.press(Action::UseSample);
    t.shot("modes-demo-add-key-filled")?;
    t.press(Action::EntryClear);
    t.press(Action::EntryForm(1));
    t.press(Action::UseSample);
    t.press(Action::UseSample);
    t.press(Action::EntryPart);
    t.shot("modes-demo-add-key-share")?;
    t.press(Action::Entry(None));
    type_text(t, "abandon abandon abandon");
    t.shot("modes-demo-add-key")?;
    crate::tour::scroll(t, 700);
    t.shot("modes-demo-add-key-foot")?;
    demo_passphrases(t)?;
    // Load samples: the sheet, Savings ticked, loaded (its toast), and the
    // sheet again with Savings and its three keys ticked and dimmed.
    t.press(Action::Nav(Screen::Home));
    t.press(Action::ScanSeed);
    t.shot("modes-demo-scan-sample")?;
    t.press(Action::Cancel);
    t.press(Action::Samples);
    t.shot("modes-demo-samples-sheet")?;
    t.press(Action::SampleTick(Pick::Wallet(0)));
    t.shot("modes-demo-samples-ticked")?;
    t.press(Action::SamplesLoad);
    t.shot("modes-demo-samples-loaded")?;
    t.press(Action::Samples);
    t.shot("modes-demo-samples-after")?;
    crate::tour::scroll(t, 700);
    t.shot("modes-demo-samples-after-foot")?;
    t.press(Action::Cancel);
    t.app.session = faraday_core::wallet::Session::on(testkit::NET);
    t.app.set_modes(faraday_core::DESKTOP_MODES);
    t.press(Action::Nav(Screen::Home));
    t.shot("modes-testnet-home")?;
    t.press(Action::ModeSheet);
    t.shot("modes-testnet-sheet")?;
    t.press(Action::Cancel);
    t.press(Action::Entry(None));
    t.shot("modes-testnet-add-key")?;
    testnet_passphrases(t)?;
    t.press(Action::Nav(Screen::Tools));
    t.shot("modes-testnet-tools")?;
    t.app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Mainnet);
    t.app.set_modes(faraday_core::STICK_MODES);
    t.press(Action::Nav(Screen::Settings));
    t.shot("modes-mainnet-settings")
}

/// Passphrases in Demo (`docs/MODES.md` §6.2): each empty field offers the
/// faraday chip, which fills it; Add a key's, the sample vault's unlock
/// and Create a vault's pair are shown before and after it.
fn demo_passphrases(t: &mut Tour) -> Result<(), String> {
    use faraday_core::demo::PassField as P;
    t.press(Action::Nav(Screen::Home));
    t.press(Action::Entry(None));
    t.press(Action::UseSample);
    t.shot("modes-demo-add-key-passphrase-empty")?;
    t.press(Action::PassChip(P::Entry));
    t.shot("modes-demo-add-key-passphrase")?;
    crate::tour::scroll(t, 700);
    t.shot("modes-demo-add-key-passphrase-foot")?;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.shot("modes-demo-vaults-empty")?;
    t.press(Action::UseSample);
    t.press(Action::Vault(V::Open(0)));
    t.shot("modes-demo-unlock")?;
    t.press(Action::PassChip(P::Unlock));
    t.shot("modes-demo-unlock-chip")?;
    t.press(Action::Vault(V::Create));
    t.shot("modes-demo-create-vault")?;
    t.press(Action::PassChip(P::Phrase(0, false)));
    t.press(Action::PassChip(P::Phrase(0, true)));
    t.shot("modes-demo-create-vault-chips")?;
    let rolls = "123456123456123456123456123456";
    t.press(Action::Vault(V::Dice(0)));
    type_text(t, rolls);
    t.press(Action::Vault(V::DiceUse));
    crate::tour::scroll(t, 300);
    t.shot("modes-demo-create-vault-dice")?;
    Ok(())
}

/// The same fields on Testnet: typed, masked and measured as before.
fn testnet_passphrases(t: &mut Tour) -> Result<(), String> {
    t.press(Action::EntryPassphrase);
    type_text(t, "ab");
    t.shot("modes-testnet-add-key-passphrase")?;
    let vault = faraday_samples::SAMPLES.inputs.vault.to_vec();
    t.app.storage(StorageEvent::Restored {
        inbox: vec![("vault.ofv".to_string(), vault)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, "ab");
    t.shot("modes-testnet-unlock")?;
    t.press(Action::Vault(V::Create));
    t.press(Action::Vault(V::CFocus(0, false)));
    type_text(t, "ab");
    t.press(Action::Vault(V::CFocus(0, true)));
    type_text(t, "ab");
    t.shot("modes-testnet-create-vault")?;
    Ok(())
}
