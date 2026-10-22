//! The public tour.

use crate::tour::{
    Tour, description_file_at, device_key, flow_at_make, flow_made, scroll, scroll_to, test_key_1,
    to_checklist, type_text,
};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent, bstep, qrow};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, Key};

/// Costs what `description_file_at` and `flow_at_make` cost. The
/// backup's copy item checking the copy: with no camera, the typed
/// check; with one, Scan my copy, a copy with a word wrong on the
/// sheet, and the match back on the step. Public files as codes and
/// pictures: a signed message, Silent payments, Create's keys and the
/// backup's public files item for a 2-of-3 made here, the QR sheet with
/// its PNG, the backup's public files item for Savings, and a GPG key
/// made in the test vault.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn public_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::Code;
    t.load_kit()?;
    // A signed message: For the stick as it is signed, as a code, as a
    // picture.
    t.press(Action::SignMessage);
    if let Some(m) = t.app.message.as_mut() {
        m.text = "Faraday signs this".to_string();
    }
    t.press(Action::MSign);
    t.press(Action::MStep(faraday_core::mstep::SIGN));
    scroll_to(t, Action::MQr);
    t.shot("public-message")?;
    t.press(Action::CodePng(Code::Message));
    // Silent payments: the record beside the address.
    t.press(Action::Silent);
    scroll_to(t, Action::CodePng(Code::SilentRecord));
    t.shot("public-silent")?;
    t.press(Action::SQr(false));
    t.shot("public-silent-sheet")?;
    t.press(Action::QrPng);
    t.press(Action::Cancel);
    t.press(Action::CreateWallet);
    t.press(Action::CKind(4));
    t.press(Action::CNext(0));
    t.press(Action::CNext(1));
    let bacon = test_key_1(t)?;
    t.press(Action::CSlotHere(0, bacon));
    device_key(t, 1);
    device_key(t, 2);
    scroll_to(t, Action::CodePng(Code::Key(0)));
    t.shot("public-keys")?;
    t.press(Action::CNext(faraday_core::cstep::KEYS));
    t.press(Action::CNext(faraday_core::cstep::CHECK));
    // The public files are the backup checklist's item; with every
    // software chosen and the description going as files, it offers
    // them all.
    let built = t.app.create.as_ref().and_then(|c| c.built).unwrap_or(0);
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVaultSoftware);
    for row in 0..5u8 {
        let on = t
            .app
            .backup
            .as_ref()
            .is_some_and(|b| b.answers.software[usize::from(row)]);
        if !on {
            t.press(Action::BAnswer(faraday_core::qrow::SOFTWARE, row));
        }
    }
    // The description goes as files too, at Place 1: the public files
    // item.
    description_file_at(c, t, 0);
    t.press(Action::BChecklist);
    t.press(Action::BStep(faraday_core::bstep::PUBLIC));
    // Found while it is offered: once done, the row says so instead.
    scroll_to(t, Action::PublicOut(built, 8));
    t.press(Action::PublicOut(built, 8));
    t.shot("public-create")?;
    t.press(Action::CodePng(Code::MultisigConfig(built)));
    t.press(Action::CodePng(Code::Bsms(built)));
    scroll_to(t, Action::CodePng(Code::WalletKeyBsms(built, 0)));
    t.shot("public-create-keys")?;
    t.press(Action::CodePng(Code::WalletKey(built, 0)));
    t.press(Action::ShowCode(Code::MultisigConfig(built)));
    t.shot("public-sheet")?;
    t.press(Action::Cancel);
    t.press(Action::Backup(built));
    to_checklist(
        t,
        faraday_core::plan::Preset::PaperVaultSoftware,
        &[(qrow::FORM, 1)],
    );
    // Opened as the wallet's chart opens it: the items before it wait
    // for the copies and the vaults, which this tour does not make.
    t.press(Action::Chart(faraday_core::glance::sheet::ChartAction::Do(
        faraday_core::glance::Press::Loaded(built),
        faraday_core::glance::sheet::Step::Item(bstep::PUBLIC),
    )));
    t.press(Action::PublicOut(built, 8));
    scroll_to(t, Action::PublicOut(built, 8));
    t.shot("public-backup")?;
    // The files few people need, under More options (F64): the button,
    // then the rows it shows.
    scroll_to(t, Action::PublicMore(built));
    t.shot("public-more")?;
    t.press(Action::PublicMore(built));
    scroll_to(t, Action::PublicOut(built, 10));
    t.shot("public-more-open")?;
    // A GPG key made in the test vault: its public key and revocation,
    // and a signature, each as a code and a picture.
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    let outbox: Vec<(String, Vec<u8>)> = t
        .app
        .outbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .collect();
    t.app.storage(StorageEvent::Restored {
        inbox: vec![
            vault,
            (
                "SHA256SUMS".into(),
                b"3d6f  faraday-0.1.0-linux-x86_64\n".to_vec(),
            ),
        ],
        outbox,
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.vaults.open.len() != 1 {
        return Err("the test vault did not open".into());
    }
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(4)));
    flow_at_make(c, t, faraday_core::backup_flow::Kind::Gpg);
    type_text(t, "Test Person");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "test@example.com");
    flow_made(t, faraday_core::backup_flow::Kind::Gpg)?;
    // On a small panel the key's page is opened from the list.
    t.press(Action::Vault(V::Item(0)));
    scroll_to(t, Action::CodePng(Code::GpgKey));
    t.shot("public-gpg")?;
    t.press(Action::CodePng(Code::GpgKey));
    t.press(Action::Vault(V::GpgSignPick));
    // Back to the top of the list of files.
    for _ in 0..10 {
        scroll(t, -600);
    }
    t.shot("public-gpg-sign")?;
    t.press(Action::ShowCode(Code::GpgKey));
    t.shot("public-gpg-sheet")?;
    t.press(Action::Cancel);
    // The pictures made, beside the screens.
    for item in t.app.outbox.iter().filter(|i| i.name.ends_with(".png")) {
        let path = t.out.join(format!("outbox-{}", item.name));
        std::fs::write(&path, &item.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}
