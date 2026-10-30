//! The pictures of `docs/guide/walk/spend.md`.

use super::{kit_of, last_wallet, seedqr};
use crate::tour::Tour;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// `bytes`, a PSBT, held up to the camera now open as animated UR
/// parts until the camera closes or shows its read card (Home's Scan
/// shows the PSBT's card, `docs/SCANNING.md` §5.4); a shot named `mid`
/// at the second part.
pub(super) fn show_psbt(t: &mut Tour, bytes: &[u8], mid: Option<&str>) -> Result<(), String> {
    crate::tours::site_tour::camera(t);
    let mut enc = faraday_codec::ur::Encoder::psbt(bytes, 120).map_err(|e| format!("{e:?}"))?;
    let mut parts = 0;
    let reading = |t: &Tour| t.app.scan().is_some_and(|s| s.card.is_none());
    while reading(t) && parts < 80 {
        t.app.event(Event::Scanned {
            bytes: enc.next_part().into_bytes(),
        });
        parts += 1;
        if parts == 2
            && let Some(name) = mid
        {
            t.app.storage(StorageEvent::QrSeen {
                width: 640,
                height: 480,
                corners: [(220, 120), (430, 130), (420, 340), (210, 330)],
                module_tenths: 60,
                read: true,
            });
            t.shot(name)?;
            // The screen reference's camera with a code found.
            if name == "6-3-scanning" {
                t.shot_after("qr-and-scan-scan", 0)?;
            }
        }
    }
    t.pump();
    if reading(t) {
        return Err(format!("the camera still waits after {parts} parts"));
    }
    Ok(())
}

/// Costs what spending costs. §6.2 to §6.7: the card's Spend from this
/// wallet; Load the wallet in
/// Sparrow; Write the payment in Sparrow; the PSBT scanned as animated
/// codes; the review and its table; Who signs with both keys here; Sign
/// it; Send it, with the signed PSBT and the finished transaction as
/// codes.
pub(super) fn spend_by_qr(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::{FamilyAction as F, page};
    use faraday_core::wallet::step;
    let w = last_wallet(t)?;
    let (psbt, _) = faraday_core::testkit::spend_of(&kit_of(t, w, 2_140))?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot_marking("6-2-card", Action::Family(F::SpendFrom(w)))?;
    t.press(Action::Family(F::SpendFrom(w)));
    t.shot_marking("6-2-sparrow", Action::Family(F::Next(page::CHECK)))?;
    t.press(Action::Family(F::Next(page::CHECK)));
    t.shot_marking("6-3-write", Action::ScanFor(faraday_core::Ask::SpendPsbt))?;
    t.press(Action::ScanFor(faraday_core::Ask::SpendPsbt));
    show_psbt(t, &psbt.to_bytes(), Some("6-3-scanning"))?;
    t.shot_marking("6-4-transaction", Action::ToggleTable)?;
    t.press(Action::ToggleTable);
    t.shot_marking("6-4-table", Action::ToggleTable)?;
    t.press(Action::ToggleTable);
    t.press(Action::StepNext(step::TRANSACTION));
    if t.app.spend().and_then(|s| s.open) == Some(step::CHECK) {
        t.shot_marking("spend-check", Action::StepNext(step::CHECK))?;
        t.press(Action::StepNext(step::CHECK));
    }
    t.shot_marking("6-5-signers", Action::StepNext(step::SIGNERS))?;
    t.press(Action::StepNext(step::SIGNERS));
    t.shot_marking("6-5-sign", Action::SignHere)?;
    t.press(Action::SignHere);
    t.shot("spend-signed")?;
    if t.app.spend().and_then(|s| s.open) == Some(step::SIGN) {
        t.press(Action::StepNext(step::SIGN));
    }
    if t.app.spend().and_then(|s| s.open) == Some(step::COLLECT) {
        t.shot_marking("spend-collect", Action::StepNext(step::COLLECT))?;
        t.press(Action::StepNext(step::COLLECT));
    }
    t.shot_marking("6-7-send", Action::QrSigned)?;
    t.press(Action::QrSigned);
    t.shot_marking("6-7-qr-signed", Action::Cancel)?;
    t.shot("qr-and-scan-qr")?;
    t.shot("files-and-sticks-qr")?;
    t.press(Action::Cancel);
    t.press(Action::Family(F::QrTx));
    t.shot_marking("6-7-qr-tx", Action::Cancel)?;
    t.press(Action::Cancel);
    crate::tour::scroll_to(t, Action::DecodeFinished);
    t.shot("check-and-decode-nonce")?;
    t.press(Action::StepNext(step::FINISH));
    t.shot("6-7-away")
}

/// Costs what reading a PSBT costs. Sign a transaction on a PSBT of the
/// guide's wallet; §6.8: one whose fee is 0.012 BTC, and the
/// test kit's Savings PSBT, a wallet not loaded here: each scanned from
/// Home, which files it while a spend is under way, then started from
/// Files.
pub(super) fn spend_warnings(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = last_wallet(t)?;
    let fee = faraday_core::testkit::unsigned(&kit_of(t, w, 1_200_000))?.to_bytes();
    let other = std::fs::read(t.kit.join("savings-unsigned.psbt")).map_err(|e| e.to_string())?;
    // Sign a transaction on a PSBT with nothing to warn of.
    let plain = faraday_core::testkit::unsigned(&kit_of(t, w, 2_000))?.to_bytes();
    for (bytes, name) in [
        (plain, "sign-transaction"),
        (fee, "6-8-fee"),
        (other, "6-8-other-wallet"),
    ] {
        t.press(Action::Nav(Screen::Home));
        t.press(Action::Scan);
        show_psbt(t, &bytes, None)?;
        // The read card's Done: the PSBT is in Files, opened from there.
        t.press(Action::Cancel);
        let at = t
            .app
            .inbox()
            .iter()
            .rposition(|i| i.bytes == bytes)
            .ok_or("the scanned PSBT is not in Files")?;
        t.press(Action::StartSpend(at));
        t.shot(name)?;
    }
    Ok(())
}

/// Where the guide's stick keeps its files.
fn stick_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("faraday-guide-stick")
}

/// The stick Sparrow wrote the PSBT to, beside the wallet's public
/// description from its backup.
fn guide_stick(files: &[(String, Vec<u8>)]) -> Result<StickInfo, String> {
    let dir = stick_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (name, bytes) in files {
        std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    }
    Ok(StickInfo {
        id: dir.display().to_string(),
        label: "TRANSFER".into(),
        boot: false,
        serial: Some(0x3000),
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
        vaults: Vec::new(),
    })
}

/// Costs a PSBT made. §6.3 on a stick: the stick
/// plugged in with keys loaded, and
/// Lock and use stick; the visit, its files copied in (the wallet
/// description that `restore_wallet` restores, and the PSBT).
pub(super) fn stick_in(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = last_wallet(t)?;
    let kit = kit_of(t, w, 1_850);
    let psbt = faraday_core::testkit::unsigned(&kit)?;
    let mut files: Vec<(String, Vec<u8>)> =
        faraday_core::testkit::public_files(kit.name, &kit.descriptor)?
            .into_iter()
            .filter(|(n, _)| n.ends_with("-descriptor.txt"))
            .collect();
    files.push(("payment.psbt".into(), psbt.to_bytes()));
    let stick = guide_stick(&files)?;
    // The PSBTs scanned so far, deleted from Files: the stick's is the
    // one the card offers.
    faraday_core::testkit::inbox_retain(&mut t.app, |i| !i.name.starts_with("scanned-"));
    t.app.storage(StorageEvent::Sticks(vec![stick.clone()]));
    t.pump();
    t.shot_marking("6-3-stick-attached", Action::Lock)?;
    t.shot("lock-and-power-stick-attached")?;
    t.press(Action::Lock);
    if let Some((inbox, outbox, kept)) = t.boxes.clone() {
        t.app.storage(StorageEvent::Restored {
            inbox,
            outbox,
            kept,
        });
    }
    t.app.storage(StorageEvent::Sticks(vec![stick]));
    t.pump();
    t.shot_marking("6-3-visit", Action::VisitCopy)?;
    t.press(Action::VisitCopy);
    t.shot("spend-stick-copied")?;
    t.press(Action::Nav(Screen::Files));
    t.shot("files-and-sticks-files")?;
    t.shot("files-both")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    Ok(())
}

/// Costs a mnemonic's checksum. §6.3 to §6.7 from the stick, with one
/// key here: the card's Sign the PSBT from the stick; the review; Who
/// signs, short a key; its Scan a key, test key 2's SeedQR; Sign it;
/// Send it, the finished transaction decoded, and writing to a stick.
pub(super) fn spend_from_stick(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::FamilyAction as F;
    use faraday_core::wallet::step;
    let w = last_wallet(t)?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot_marking("6-3-card-stick", Action::Family(F::SpendFrom(w)))?;
    t.press(Action::Family(F::SpendFrom(w)));
    t.shot_marking(
        "spend-stick-transaction",
        Action::StepNext(step::TRANSACTION),
    )?;
    t.press(Action::StepNext(step::TRANSACTION));
    if t.app.spend().and_then(|s| s.open) == Some(step::CHECK) {
        t.shot_marking("spend-stick-check", Action::StepNext(step::CHECK))?;
        t.press(Action::StepNext(step::CHECK));
    }
    t.shot_marking("6-5-signers-short", Action::ScanKey(w))?;
    t.press(Action::ScanKey(w));
    crate::tours::site_tour::camera(t);
    t.shot("6-6-scan-key")?;
    let zebra = faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0);
    t.app.event(Event::Scanned {
        bytes: seedqr(c, &zebra)?,
    });
    t.pump();
    t.shot_after("6-6-signers-two", 0)?;
    t.press(Action::StepNext(step::SIGNERS));
    t.press(Action::SignHere);
    t.shot("spend-stick-signed")?;
    for k in [step::SIGN, step::COLLECT] {
        if t.app.spend().and_then(|s| s.open) == Some(k) {
            t.press(Action::StepNext(k));
        }
    }
    // Primary, the summary panel's action, is not drawn on this page.
    t.shot("6-7-stick-send")?;
    // Decode it, under the nonce check: the finished transaction taken
    // apart, then back to the spend.
    t.press(Action::DecodeFinished);
    if t.app.screen != Screen::Decode {
        return Err("Decode it opened no transaction".into());
    }
    t.shot("decode")?;
    t.shot("check-and-decode-decode")?;
    t.press(Action::Back);
    t.press(Action::Primary);
    t.shot("6-7-write-out")?;
    // The stick's files, gone with the tour.
    let _ = std::fs::remove_dir_all(stick_dir());
    Ok(())
}
