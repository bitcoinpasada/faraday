//! The pictures of `docs/guide/walk/files-and-sticks.md`, and those of
//! the screen reference's Files and Stick visit
//! (`docs/guide/screens/files.md`, `docs/guide/screens/stick-visit.md`)
//! that show the same states.

use super::{DICE_FP, DICE_WORDS, files_in, folder_stick, fresh_session, key_session, single_kit};
use crate::tour::{Tour, not_now_load, type_text};
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// Costs what the guide's wallet's PSBT and a vault's unlock cost. A
/// stick brought in with nothing loaded, and the visit's writes, ⋮
/// menu, deletes and zeroing; a key's words made into a wallet from
/// Files; a write asked for while a key is loaded; a child seed's file
/// on its way out with a vault open.
pub(super) fn run(c: &Compute, t: &mut Tour) -> Result<(), String> {
    key_session(t);
    let kit = single_kit(c, t, 1_410)?;
    let psbt = testkit::unsigned(&kit)?.to_bytes();
    let vault = std::fs::read(t.kit.join("vault.fvlt")).map_err(|e| format!("vault.fvlt: {e}"))?;
    make_it_a_wallet(t)?;
    write_asks_to_lock(t, &psbt)?;
    this_is_a_secret(t, &vault)?;
    visit(t, &kit.descriptor, &psbt, &vault)
}

/// A fresh session: a stick with the wallet's description, a PSBT and a
/// photo too large to read, its two files ticked and imported; the signed PSBT For the
/// stick and its ⋮; a stick holding a vault and that PSBT, the visit,
/// Already on the stick, the receipt; Delete files and the free space
/// zeroed.
fn visit(t: &mut Tour, descriptor: &str, psbt: &[u8], vault: &[u8]) -> Result<(), String> {
    fresh_session(t);
    let transfer = folder_stick(
        "transfer",
        "TRANSFER",
        &[
            ("IMG_0412.jpg", vec![0xff; 4096]),
            (
                "new-wallet-1-descriptor.txt",
                descriptor.as_bytes().to_vec(),
            ),
            ("payment.psbt", psbt.to_vec()),
        ],
    )?;
    // The photo as a camera writes it, too large to be read.
    let mut transfer = transfer;
    if let Some(photo) = transfer.files.iter_mut().find(|(n, _)| n == "IMG_0412.jpg") {
        photo.1 = 24_600_000;
    }
    t.app.storage(StorageEvent::Sticks(vec![transfer]));
    t.pump();
    t.press(Action::Nav(Screen::Visit));
    t.shot_marking("files-and-sticks-visit", Action::VisitCopy)?;
    t.press(Action::VisitCopy);
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    let signed = [("payment-signed.psbt", psbt.to_vec())];
    files_in(t, &[], &signed);
    t.press(Action::Nav(Screen::Files));
    t.press(Action::File(FilePress::Menu(Row::Out(0))));
    t.shot("files-and-sticks-menu")?;
    t.shot("files-menu")?;
    t.press(Action::File(FilePress::Close));

    let keystick = folder_stick(
        "keystick",
        "KEYSTICK",
        &[
            ("payment-signed.psbt", psbt.to_vec()),
            ("vault.fvlt", vault.to_vec()),
        ],
    )?;
    t.app.storage(StorageEvent::Sticks(vec![keystick.clone()]));
    t.pump();
    t.press(Action::Nav(Screen::Visit));
    t.shot("stick-visit")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    files_in(t, &[], &[signed[0].clone(), ("vault.fvlt", vault.to_vec())]);
    t.app.storage(StorageEvent::Sticks(vec![keystick.clone()]));
    t.pump();
    t.press(Action::Nav(Screen::Visit));
    t.press(Action::VisitWrite);
    t.shot_marking("stick-visit-already", Action::VisitReplace)?;
    t.press(Action::VisitReplace);
    t.press(Action::Nav(Screen::Files));
    t.shot("files-and-sticks-receipt")?;

    t.press(Action::Nav(Screen::Visit));
    t.press(Action::DeleteFiles);
    t.press(Action::DeletePick(0));
    t.shot("files-and-sticks-delete")?;
    t.shot("stick-visit-delete")?;
    t.press(Action::DeleteWhat(1));
    t.press(Action::Vault(V::HoldWipe));
    if t.app.sheet != Some(faraday_core::Sheet::Zeroing) {
        return Err("zeroing the free space did not start".into());
    }
    // The first step answered, then two minutes on, 40% done.
    for next in [2, 403] {
        t.app.storage(StorageEvent::Zeroing {
            stick: keystick.id.clone(),
            next,
            total: 1_002,
            done: false,
        });
        t.now += 120_000;
        t.app.event(Event::Tick { now_ms: t.now });
        t.pump();
    }
    t.shot("stick-visit-zeroing")?;
    t.app.storage(StorageEvent::Zeroing {
        stick: keystick.id,
        next: 1_002,
        total: 1_002,
        done: true,
    });
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    Ok(())
}

/// A fresh session: the key's words copied in from a stick, ticked by
/// hand, then Files' Make it a wallet on it.
fn make_it_a_wallet(t: &mut Tour) -> Result<(), String> {
    fresh_session(t);
    let paper = folder_stick(
        "words",
        "TRANSFER",
        &[("new-key-1-words.txt", DICE_WORDS.as_bytes().to_vec())],
    )?;
    t.app.storage(StorageEvent::Sticks(vec![paper]));
    t.pump();
    t.press(Action::Nav(Screen::Visit));
    t.press(Action::VisitIn(0));
    t.press(Action::VisitCopy);
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.press(Action::Nav(Screen::Files));
    t.press(Action::PotentialOpen(DICE_FP));
    t.shot("files-make-it-a-wallet")?;
    t.press(Action::Cancel);
    Ok(())
}

/// A fresh stick session with the guide's key typed in and the signed
/// PSBT For the stick: a stick put in and Not now, then the visit's
/// Write, which asks for a lock first.
fn write_asks_to_lock(t: &mut Tour, psbt: &[u8]) -> Result<(), String> {
    key_session(t);
    files_in(t, &[], &[("payment-signed.psbt", psbt.to_vec())]);
    let stick = folder_stick("blank", "KEYSTICK", &[])?;
    t.app.storage(StorageEvent::Sticks(vec![stick]));
    t.pump();
    if t.app.sheet.is_some() {
        t.press(Action::Cancel);
    }
    t.press(Action::Nav(Screen::Visit));
    t.press(Action::VisitWrite);
    t.shot("stick-visit-write-to-a-stick")?;
    t.press(Action::Cancel);
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    Ok(())
}

/// Costs a vault's unlock. A fresh stick session with the guide's key
/// typed in and the test kit's vault unlocked: a BIP-85 child seed made
/// and sent out, This is a secret offering the open vault.
fn this_is_a_secret(t: &mut Tour, vault: &[u8]) -> Result<(), String> {
    key_session(t);
    files_in(t, &[("vault.fvlt", vault.to_vec())], &[]);
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    not_now_load(t);
    if t.app.vaults().open.is_empty() {
        return Err("the test vault did not open".into());
    }
    t.press(Action::Bip85);
    t.press(Action::PKey(DICE_FP));
    t.press(Action::PNext);
    t.press(Action::PNext);
    t.press(Action::POut);
    t.shot("files-this-is-a-secret")
}
