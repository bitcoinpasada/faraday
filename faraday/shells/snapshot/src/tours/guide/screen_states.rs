//! The screen reference's pictures of states a walkthrough does not pass
//! through (`docs/guide/screens/`): the word list, the boot stick's
//! import, a new keyboard, the idle lock, a SeedQR read, partial
//! descriptions, a chart's sheet and Faraday Demo's Transfer.

use super::{DICE_FP, DICE_WORDS, files_in, key_session, kind_at, seedqr, single_kit};
use crate::tour::{Tour, type_text};
use faraday_core::create::NewKind;
use faraday_core::testkit;
use faraday_core::{Action, Screen, StickInfo, StorageEvent, cstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// Costs what the guide's wallet's PSBT, a SeedQR read and a backup's
/// board cost. Each state from a fresh session of its own.
pub(super) fn run(c: &Compute, t: &mut Tour) -> Result<(), String> {
    word_list(t)?;
    import(c, t)?;
    new_keyboard(t)?;
    idle(c, t)?;
    seedqr_read(c, t)?;
    partials(t)?;
    chart(t)?;
    transfer(t)
}

/// Tools' BIP-39 words, the dice key's third word marked.
fn word_list(t: &mut Tour) -> Result<(), String> {
    use faraday_core::wordlist::WordListAction as WL;
    key_session(t);
    let word = DICE_WORDS.split(' ').nth(2).ok_or("no third word")?;
    let at = faraday_bip::bip39::Language::English
        .index_of(word)
        .ok_or("not a BIP-39 word")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::WordList(WL::Open(0, Some(at))));
    t.shot("tools-word-list")?;
    t.press(Action::WordList(WL::Close));
    Ok(())
}

/// Costs the guide's wallet's PSBT. A fresh stick session, the boot
/// stick's first look with a vault, a wallet's description and a PSBT on
/// it: Read from the boot stick.
fn import(c: &Compute, t: &mut Tour) -> Result<(), String> {
    key_session(t);
    let kit = single_kit(c, t, 1_410)?;
    let psbt = testkit::unsigned(&kit)?.to_bytes();
    let vault = std::fs::read(t.kit.join("vault.ofv")).map_err(|e| format!("vault.ofv: {e}"))?;
    let mut boot: StickInfo = super::folder_stick(
        "boot",
        "FARADAY",
        &[
            ("new-wallet-1-descriptor.txt", kit.descriptor.into_bytes()),
            ("payment.psbt", psbt),
            ("vault.ofv", vault),
        ],
    )?;
    boot.boot = true;
    crate::tours::screens::stick_session(t);
    t.app.storage(StorageEvent::Sticks(vec![boot]));
    t.pump();
    if t.app.sheet != Some(faraday_core::Sheet::Import) {
        return Err("the boot stick's files did not come up for the import".into());
    }
    t.shot("prompts-import")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    Ok(())
}

/// A keyboard plugged in after the start, half its code typed on it.
fn new_keyboard(t: &mut Tour) -> Result<(), String> {
    crate::tours::screens::stick_session(t);
    t.press(Action::Nav(Screen::Home));
    let id = 7;
    t.app.storage(StorageEvent::NewInput {
        id,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
        touch: false,
    });
    t.pump();
    let code = t
        .app
        .new_inputs()
        .first()
        .and_then(|i| i.code.clone())
        .ok_or("the new keyboard has no code")?;
    for ch in code.chars().take(code.chars().count() / 2) {
        t.app.storage(StorageEvent::InputTyped { id, ch });
    }
    t.pump();
    t.shot("prompts-new-keyboard")?;
    t.press(Action::Cancel);
    Ok(())
}

/// Costs the guide's wallet's PSBT. A fresh stick session with the
/// guide's key typed and a signed PSBT For the stick, then no input: the
/// warning, and the lock after it with the file still waiting.
fn idle(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::{IDLE_LOCK_MIN, IDLE_WARN_MS};
    key_session(t);
    let kit = single_kit(c, t, 1_410)?;
    let psbt = testkit::unsigned(&kit)?.to_bytes();
    crate::tours::screens::stick_session(t);
    t.app.event(Event::Tick { now_ms: t.now });
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Entry(None));
    let _ = t.app.frame();
    type_text(t, DICE_WORDS);
    t.press(Action::EntryAdd);
    files_in(t, &[], &[("payment-signed.psbt", psbt)]);
    t.press(Action::Nav(Screen::Home));
    t.now += u64::from(IDLE_LOCK_MIN) * 60_000 - IDLE_WARN_MS + 20_000;
    t.app.event(Event::Tick { now_ms: t.now });
    t.pump();
    t.shot_after("lock-and-power-locking-soon", 0)?;
    t.now += IDLE_WARN_MS;
    t.app.event(Event::Tick { now_ms: t.now });
    t.pump();
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the idle lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.pump();
    if t.app.sheet != Some(faraday_core::Sheet::Locked) {
        return Err("the idle lock did not show Locked".into());
    }
    t.shot_after("lock-and-power-locked", 0)
}

/// Costs a mnemonic's checksum. Add a key's Scan a SeedQR, the guide's
/// key's SeedQR held up: SeedQR read.
fn seedqr_read(c: &Compute, t: &mut Tour) -> Result<(), String> {
    crate::tours::screens::stick_session(t);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Entry(None));
    t.press(Action::ScanSeed);
    t.app.event(Event::Scanned {
        bytes: seedqr(c, DICE_WORDS)?,
    });
    t.pump();
    if t.app.sheet != Some(faraday_core::Sheet::SeedRead) {
        return Err("the SeedQR did not open SeedQR read".into());
    }
    t.shot("qr-and-scan-seedqr-read")?;
    t.press(Action::Cancel);
    Ok(())
}

/// The guide's key and test keys 2 and 3 typed, a 2-of-3 made over them,
/// its board customised and partial descriptions added.
fn partials(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::BoardPress as B;
    key_session(t);
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    let multi = kind_at(NewKind::Multi)?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    if t.app.create().and_then(|c| c.open) == Some(cstep::KIND) {
        t.press(Action::CKind(multi));
    }
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    let w = super::last_wallet(t)?;
    t.press(Action::Backup(w));
    t.app.press(Action::Board(B::Customize));
    t.pump();
    t.app.press(Action::Board(B::Partials));
    t.pump();
    t.shot("back-up-partials")
}

/// The guide's single-key wallet, its backup planned the standard way,
/// and its card's chart: the sheet of Place 1.
fn chart(t: &mut Tour) -> Result<(), String> {
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::{ChartAction as C, Target};
    key_session(t);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(kind_at(NewKind::NativeSegwit)?));
    t.press(Action::CVaults(false));
    t.press(Action::CSlotHere(0, DICE_FP));
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    let w = super::last_wallet(t)?;
    t.press(Action::Backup(w));
    t.press(Action::BChecklist);
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.press(Action::Chart(C::Open(Press::Loaded(w), Target::Node(0))));
    t.shot("backups-chart")
}

/// Faraday Demo's Transfer, its Downloads folder listed.
fn transfer(t: &mut Tour) -> Result<(), String> {
    super::demo_session(t);
    t.app.storage(StorageEvent::HostFiles {
        dir: "~/Downloads".into(),
        files: vec![
            ("payment.psbt".into(), 2_342),
            ("new-wallet-1-descriptor.txt".into(), 178),
            ("IMG_0412.jpg".into(), 2_300_000),
        ],
        opens: true,
    });
    t.pump();
    t.press(Action::Nav(Screen::Transfer));
    if t.app.screen != Screen::Transfer {
        return Err("Transfer did not open in Faraday Demo".into());
    }
    t.shot("qr-and-scan-transfer")
}
