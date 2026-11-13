//! The transfer tour.

use crate::tour::Tour;
use faraday_core::testkit;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event};

/// The desktop as the device's QR link, in Practice.
pub(crate) fn transfer_tour(t: &mut Tour) -> Result<(), String> {
    let downloads = t.out.join("downloads");
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let put = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let path = downloads.join(name);
        if !path.exists() {
            std::fs::write(&path, bytes).map_err(|e| format!("{name}: {e}"))?;
        }
        // Written apart in the order given, so the last is the newest.
        std::thread::sleep(std::time::Duration::from_millis(20));
        Ok(())
    };
    let spending = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no spending kit")?;
    put("photo-2026-10-01.jpg", &vec![0x5a; 300 * 1024])?;
    put("notes-backup.tar", &vec![0x33; 100 * 1024])?;
    put("spending-wallet.txt", spending.descriptor.as_bytes())?;
    let psbt = std::fs::read(t.kit.join("savings-unsigned.psbt")).map_err(|e| e.to_string())?;
    put("savings-unsigned.psbt", &psbt)?;
    // The online app starts in Practice, on testnet.
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::on(testkit::net());
    t.app
        .session
        .set_demo(faraday_core::testkit::demo_samples());
    let host = faraday_storage::Host {
        print: t.out.clone(),
        downloads: downloads.clone(),
        opens: true,
    };
    t.host = Some(host);
    if let Some(files) = t.host_files() {
        t.app.storage(files);
    }
    t.press(Action::Nav(Screen::Transfer));
    if t.app.screen != Screen::Transfer {
        return Err("Transfer did not open in the online app".into());
    }
    t.shot("transfer")?;
    send_text(t)?;
    let at = |t: &Tour, name: &str| {
        t.app
            .transfer()
            .files
            .iter()
            .position(|(n, _)| n == name)
            .ok_or(format!("{name} is not listed"))
    };
    let i = at(t, "savings-unsigned.psbt")?;
    t.press(Action::TransferSend(i));
    if t.app.qr().is_none() {
        return Err("the PSBT was not shown as codes".into());
    }
    t.shot("transfer-send-psbt")?;
    t.press(Action::Cancel);
    let i = at(t, "photo-2026-10-01.jpg")?;
    t.press(Action::TransferSend(i));
    t.shot("transfer-refused")?;
    // Receive: a frame, then what a device's Files sends as codes.
    t.press(Action::TransferReceive);
    let frame: Vec<u8> = (0..640u32 * 480)
        .map(|i| ((i % 640) / 3 + (i / 640) / 4) as u8)
        .collect();
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: frame,
        chroma: None,
    });
    let envelope = |name: &str, bytes: &[u8]| -> Result<Vec<String>, String> {
        let env = faraday_qr::envelope::pack(name, bytes, "file").map_err(|e| format!("{e:?}"))?;
        faraday_qr::bbqr::encode('J', &env, 160)
            .map(|parts| parts.iter().map(|p| String::from(&**p)).collect())
            .ok_or_else(|| "too large".to_string())
    };
    for part in envelope("spending-wallet.txt", spending.descriptor.as_bytes())? {
        t.app.event(Event::Scanned {
            bytes: part.into_bytes(),
        });
    }
    t.pump();
    t.shot("transfer-received")?;
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    for part in envelope("seed-words.txt", words.as_bytes())? {
        t.app.event(Event::Scanned {
            bytes: part.into_bytes(),
        });
    }
    t.pump();
    t.shot("transfer-received-secret")?;
    t.press(Action::Cancel);
    t.shot("transfer-saved")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("transfer-home")?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("transfer-wallets")
}

/// Send text: a block header pasted and shown, then a seed's words refused.
fn send_text(t: &mut Tour) -> Result<(), String> {
    // Send text: a block header pasted, then a seed's words refused.
    let paste = |t: &mut Tour, text: &str| {
        t.app.event(Event::Clipboard {
            kind: faraday_shell_api::FileKind::Text,
            text: faraday_text::Text::of(text),
        });
    };
    paste(
        t,
        "0000000000000000000320283a032748d0f8be1b7bcf3a2c5a41eb4bce29b7b4",
    );
    t.shot("transfer-text")?;
    t.press(Action::TransferText(
        faraday_core::transfer::TextPress::Show,
    ));
    if t.app.qr().is_none() {
        return Err("the text was not shown as a code".into());
    }
    t.shot("transfer-text-code")?;
    t.press(Action::Cancel);
    paste(
        t,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    t.shot("transfer-text-refused")?;
    t.press(Action::TransferText(
        faraday_core::transfer::TextPress::Clear,
    ));
    Ok(())
}
