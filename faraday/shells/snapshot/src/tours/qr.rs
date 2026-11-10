//! The qr tour (`docs/QR.md` §7).

use crate::tour::{Tour, not_now_load, scroll_to, test_key_1, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Code, Screen, StorageEvent};
use faraday_crypto::work::Compute;

/// Costs a vault's making and its unlock (Argon2id each). The codes of
/// public values and of an entry's TOTP secret: the wallet card with its
/// pressable receive address and that address's code; Explore's
/// extended key's code; the Lightning node key with its Show as QR
/// and PNG; a recovery sheet's page and its code; and the test vault's
/// entry with its TOTP Show as QR, and the Secret sheet it opens.
pub(crate) fn qr_tour(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    // The wallets' files first, while no key is held to stop the copy;
    // then test key 1 alone: the pages that read a key, no wallet beside.
    t.copy_wallets();
    t.press(Action::Entry(None));
    t.type_key(0);
    // Explore's extended key at its path.
    t.press(Action::Explore);
    // Its page is the default tour's (explore); here the code alone.
    t.press(Action::ShowCode(Code::ExploreXpub));
    t.shot("qr-explore-xpub")?;
    t.press(Action::Cancel);
    // The Lightning node key, its id with Show as QR and PNG.
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Lightning);
    t.press(Action::LKey(test_key_1(t)?));
    t.tick();
    scroll_to(t, Action::LOut);
    t.shot("qr-lightning")?;
    t.press(Action::ShowCode(Code::NodeId));
    t.shot("qr-lightning-code")?;
    t.press(Action::Cancel);
    t.load_kit()?;
    // The wallet card, its first receive address pressable.
    t.press(Action::OpenWallet(0));
    scroll_to(t, Action::ShowCode(Code::Receive(0, 0)));
    t.shot("qr-wallet-card")?;
    t.press(Action::ShowCode(Code::Receive(0, 0)));
    t.shot("qr-receive")?;
    t.press(Action::Cancel);
    // A vault with a recovery sheet beside the test vault's entry.
    let vault = sheet_vault()?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![("vault.kdbx".to_string(), vault)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASE);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    not_now_load(t);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(3)));
    t.press(Action::Vault(V::Item(0)));
    scroll_to(t, Action::CodePng(Code::VaultSheet));
    t.shot("qr-sheet")?;
    t.press(Action::ShowCode(Code::VaultSheet));
    t.shot("qr-sheet-code")?;
    t.press(Action::Cancel);
    t.press(Action::Vault(V::Category(2)));
    t.press(Action::Vault(V::Item(0)));
    scroll_to(t, Action::Vault(V::TotpQr));
    t.shot("qr-entry")?;
    t.press(Action::Vault(V::TotpQr));
    t.shot("qr-entry-secret")?;
    t.press(Action::Cancel);
    // A GPG key's paperkey: one static code, marked Secret, no choices.
    t.press(Action::Vault(V::Category(
        faraday_core::backup_flow::Kind::Gpg.category(),
    )));
    t.press(Action::Vault(V::Item(0)));
    scroll_to(t, Action::Vault(V::PaperkeyQr));
    t.shot("qr-paperkey")?;
    t.press(Action::Vault(V::PaperkeyQr));
    t.shot("qr-paperkey-secret")?;
    t.press(Action::Cancel);
    Ok(())
}

/// Costs a vault's making (an Argon2id run). The test vault's passphrase
/// over a vault holding an entry with a TOTP secret and a recovery sheet
/// of the Savings wallet.
fn sheet_vault() -> Result<Vec<u8>, String> {
    use faraday_keys::vault::gpg_field;
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let savings = testkit::savings();
    let sheet = faraday_vault::layouts::sheet_payload(savings.as_bytes(), b"Savings", b"")
        .ok_or("no sheet payload")?;
    let at = 1_780_000_000u32.to_le_bytes();
    let records = vec![
        Record::new(kind::ENTRY)
            .with(field::TITLE, b"Email")
            .with(field::USERNAME, b"you@example.com")
            .with(field::PASSWORD, b"correct-horse-battery-staple")
            .with(
                field::TOTP,
                b"otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP",
            ),
        Record::new(kind::SHEET).with(field::SHEET, &sheet),
        Record::new(kind::GPG)
            .with(gpg_field::PRIMARY, &[1; 32])
            .with(gpg_field::CREATED, &at)
            .with(gpg_field::SUBKEY, &[2; 32])
            .with(gpg_field::SUBKEY_CREATED, &at)
            .with(gpg_field::USER_ID, b"Faraday test"),
    ];
    testkit::kdbx_vault_at(
        "Test vault",
        testkit::VAULT_PASSPHRASE,
        0x51,
        64 * 1024,
        records,
    )
}
