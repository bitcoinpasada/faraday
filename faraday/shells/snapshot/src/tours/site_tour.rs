//! The site tour: the user guide's and the website tour's pictures, on
//! the network the tour runs on (Mainnet, from the public test seeds).

use super::default::pass_quiz;
use crate::tour::{Tour, bput, laptop, scroll_to, type_text, type_words};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, Sheet, StickInfo, StorageEvent, bstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, TouchPhase};

/// Costs what the parts it runs cost. Home with nothing loaded; the
/// vault read from the boot stick at power-on; a new key from dice; a
/// SeedQR scanned, and a wallet's missing key scanned; a 2-of-3 made and
/// its backup board edited; a backup's checklist and its sticks written;
/// a spend signed and shown as a code; a stick visit, Files and a vault;
/// and a wallet restored.
pub(crate) fn site_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.app.devices = laptop();
    t.pump();
    t.shot("home")?;
    boot(t)?;
    fresh(t);
    keygen(t)?;
    fresh(t);
    scan(c, t)?;
    fresh(t);
    let w = wallet(t)?;
    board(t, w)?;
    board_drag(t)?;
    board_sheets(t)?;
    fresh(t);
    checklist(t)?;
    fresh(t);
    spend(t)?;
    sticks_and_vaults(t)?;
    fresh(t);
    restore(t)
}

/// A fresh session, as Lock leaves one: the next process on the same
/// display, nothing loaded.
fn fresh(t: &mut Tour) {
    t.press(Action::Lock);
    if t.app.holds_secret() || t.app.sheet.is_some() {
        t.restart();
    }
    t.sticks.clear();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    t.pump();
}

/// Costs a vault's unlock. Power-on with the boot stick in: Read from
/// the stick with its files ticked, the vault's passphrase typed, its
/// Load sheet, and Wallets once Load is pressed.
fn boot(t: &mut Tour) -> Result<(), String> {
    use faraday_core::boot_import::ImportAction as I;
    let mut stick = t.stick();
    stick.boot = true;
    stick.label = "FARADAY".into();
    // What a person keeps on the stick they boot from: the vault, and a
    // transaction to sign. The wallets come from the vault.
    stick
        .files
        .retain(|(n, _)| ["vault.ofv", "savings-unsigned.psbt"].contains(&n.as_str()));
    t.app.storage(StorageEvent::Sticks(vec![stick]));
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    t.pump();
    if t.app.sheet != Some(Sheet::Import) {
        return Err("the boot stick's files did not come up for the import".into());
    }
    t.shot("boot-read")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.press(Action::Import(I::Go));
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.shot("boot-unlock")?;
    t.press(Action::Import(I::Submit));
    for _ in 0..20 {
        if t.app.sheet == Some(Sheet::UnlockLoad) {
            break;
        }
        t.tick();
        t.app.storage(StorageEvent::Memory {
            available_mib: 15_000,
        });
    }
    if t.app.sheet != Some(Sheet::UnlockLoad) {
        return Err(format!(
            "the vault's passphrase did not open its Load sheet: {:?} {:?} {:?} open {} wallets {} keys {}",
            t.app.sheet,
            t.app.screen,
            t.app.vaults().unlock_error,
            t.app.vaults().open.len(),
            t.app.session.wallets.len(),
            t.app.session.keys.len()
        ));
    }
    t.shot("boot-load")?;
    t.press(Action::Vault(V::LoadTicked));
    if t.app.screen != Screen::Wallets {
        return Err("Load did not land on Wallets".into());
    }
    t.shot_after("boot-landed", 0)
}

/// New key from 99 dice rolls: Randomness, the rolls card at 0, 30 and
/// 60 rolls, the randomness check, the Key card locked in, the words
/// shown, the quiz asking and passed.
fn keygen(t: &mut Tour) -> Result<(), String> {
    use faraday_core::keygen::Way;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    // Start opens closed on Make a new key, Length open; a length and a
    // way each move on by themselves (F103).
    t.press(Action::KWords(24));
    t.shot("keygen-source")?;
    t.press(Action::KGroup(1));
    t.press(Action::KWay(Way::DiceHashed.index()));
    t.shot("keygen-rolls-0")?;
    // Rolls a die would give: a fixed run from a small generator.
    let mut x: u32 = 0x2545_f491;
    for i in 1..=99 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        t.press(Action::KRoll((x % 6) as u8 + 1));
        if i == 30 || i == 60 {
            t.shot(&format!("keygen-rolls-{i}"))?;
        }
    }
    scroll_to(t, Action::KNext);
    t.shot("keygen-check")?;
    t.press(Action::KNext);
    t.press(Action::KLock);
    t.shot("keygen-key-open")?;
    t.press(Action::KShow);
    t.shot("keygen-words-open")?;
    t.press(Action::KShow);
    t.press(Action::KNext);
    t.shot("keygen-quiz-ask")?;
    pass_quiz(t);
    t.shot("keygen-quiz-passed")?;
    t.press(Action::KAdd);
    Ok(())
}

/// Costs a mnemonic's checksum. Test key `n`'s SeedQR digits, as a
/// camera reads them off the code.
fn seedqr(c: &Compute, n: usize) -> Result<Vec<u8>, String> {
    let words = testkit::test_words(testkit::TEST_SEEDS[n].0);
    let m = faraday_bip::bip39::Mnemonic::parse(c, faraday_bip::bip39::Language::English, &words)
        .map_err(|e| format!("{e:?}"))?;
    Ok(faraday_codec::seedqr::to_digits(&m)
        .expose()
        .as_bytes()
        .to_vec())
}

/// A camera's frame with nothing to read in it.
pub(crate) fn camera(t: &mut Tour) {
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: (0..640u32 * 480).map(|i| ((i % 640) / 3) as u8).collect(),
        chroma: None,
    });
}

fn savings(t: &Tour) -> Result<usize, String> {
    t.app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Savings")
        .ok_or_else(|| "Savings is not loaded".to_string())
}

/// Costs a mnemonic's checksum. A SeedQR held up to Add a key's camera,
/// read and put back; then Savings' Scan a key with test key 1 loaded,
/// test key 2's SeedQR read into it.
fn scan(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    t.press(Action::Entry(None));
    t.press(Action::ScanSeed);
    camera(t);
    t.shot("scan-seedqr")?;
    t.app.event(Event::Scanned {
        bytes: seedqr(c, 1)?,
    });
    t.pump();
    if t.app.sheet != Some(Sheet::SeedRead) {
        return Err("the scanned SeedQR did not ask how it loads".into());
    }
    t.shot("scan-seedqr-read")?;
    t.press(Action::Cancel);
    let s = savings(t)?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(s));
    t.press(Action::ScanKey(s));
    camera(t);
    t.shot("scan-key-camera")?;
    t.app.event(Event::Scanned {
        bytes: seedqr(c, 1)?,
    });
    t.pump();
    t.shot_after("scan-key-added", 0)
}

/// Make a new wallet: a 2-of-3 over test keys 1 to 3, typed in. Kind,
/// Keys, Check and the wallet's card. Returns the wallet.
fn wallet(t: &mut Tour) -> Result<usize, String> {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    for n in 0..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .ok_or("no multisig")? as u8;
    // Multisig moves on to Quorum; Kind, opened again, shows it ticked,
    // and Multisig, pressed again, moves on; No moves on to Keys (F103).
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CStep(cstep::KIND));
    t.shot("wallet-kind")?;
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.shot("wallet-keys")?;
    t.press(Action::CNext(cstep::KEYS));
    t.shot("wallet-check")?;
    let w = t
        .app
        .create()
        .and_then(|c| c.built)
        .ok_or("the wallet was not made")?;
    t.press(Action::OpenWallet(w));
    t.shot("wallet-card")?;
    Ok(w)
}

/// The 2-of-3's backup board: the standard way, read-only, then after
/// Customize backup.
fn board(t: &mut Tour, w: usize) -> Result<(), String> {
    t.press(Action::Backup(w));
    t.shot("board-standard")?;
    t.press(Action::Board(BP::Customize));
    t.shot("board-customize")
}

/// A finger on the panel at `at`, as `phase`.
fn touch(t: &mut Tour, (x, y): (u16, u16), phase: TouchPhase) {
    t.app.event(Event::Touch { x, y, phase });
    t.pump();
}

/// Seed 2 dragged from the list onto Place 2's paper by a finger, as a
/// person drags it, after its words were taken off there: picked up,
/// held over the paper with the boxes that take it edged, and dropped.
fn board_drag(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Board(BP::Off(Zone::Paper(1), Thing::Seed(1))));
    let _ = t.app.frame();
    let chip = Action::Board(BP::Chip(Chip::List(Thing::Seed(1))));
    let from = t
        .app
        .where_offered(chip)
        .ok_or("Seed 2's chip is not on screen")?;
    let to = t
        .app
        .where_offered(Action::Board(BP::Zone(Zone::Paper(1))))
        .ok_or("Place 2's paper is not on screen")?;
    touch(t, from, TouchPhase::Down);
    // Past the slop, a little way out of the list.
    let step =
        |a: u16, b: u16, k: i32| (i32::from(a) + (i32::from(b) - i32::from(a)) * k / 4) as u16;
    touch(
        t,
        (step(from.0, to.0, 1), step(from.1, to.1, 1)),
        TouchPhase::Move,
    );
    t.shot("board-drag-1")?;
    for k in 2..=4 {
        touch(
            t,
            (step(from.0, to.0, k), step(from.1, to.1, k)),
            TouchPhase::Move,
        );
    }
    t.shot("board-drag-2")?;
    touch(t, to, TouchPhase::Up);
    t.shot("board-drag-3")
}

/// A placed seed's ⋮ menu; its SeedQR pill before and after a press; a
/// new vault on a stick at Place 2; the partial descriptions' sheet; the
/// sheets over a drop that lets Place 3 see the balance, and one that
/// lets Place 1 spend.
fn board_sheets(t: &mut Tour) -> Result<(), String> {
    let seed1 = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    t.press(Action::Board(BP::Menu(seed1)));
    t.shot("board-menu")?;
    t.press(Action::Board(BP::MenuClose));
    t.press(Action::Board(BP::Chip(seed1)));
    t.shot("board-seedqr-off")?;
    t.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1)));
    t.shot("board-seedqr-on")?;
    t.press(Action::Board(BP::LetGo));
    // + new vault: Vault 1, at no place until it is put at one.
    t.press(Action::Board(BP::AddVault));
    let v = 0;
    bput(t, Thing::Vault(v), Zone::Card(1));
    t.shot("board-vault")?;
    // Partial descriptions added: the sheet; then taken off again.
    t.press(Action::Board(BP::Partials));
    if t.app.sheet != Some(Sheet::BoardPartials) {
        return Err(format!(
            "partial descriptions asked nothing: {:?}",
            t.app.sheet
        ));
    }
    t.shot("board-partial-sheet")?;
    t.press(Action::Board(BP::PartialsKeep));
    t.press(Action::Board(BP::Partials));
    // Place 3's whole description off, Share 1 dropped there beside
    // Seed 3: the sheet asks.
    t.press(Action::Board(BP::Off(Zone::Paper(2), Thing::Whole)));
    drop_asked(t, Thing::Share(0), Zone::Paper(2));
    if t.app.sheet != Some(Sheet::BoardSees) {
        return Err("the share at Place 3 did not ask".into());
    }
    t.shot("board-sees-sheet")?;
    t.press(Action::Board(BP::SeesKeep));
    // Seed 2 dropped beside Seed 1 at Place 1: enough to spend there.
    drop_asked(t, Thing::Seed(1), Zone::Paper(0));
    if t.app.sheet != Some(Sheet::BoardSees) {
        return Err("a second seed at Place 1 did not ask".into());
    }
    t.shot("board-spend-sheet")?;
    t.press(Action::Board(BP::SeesUndo));
    Ok(())
}

/// `th` placed at `z` by taps, with the sheet a drop may ask left open
/// (`Tour::press` answers it).
fn drop_asked(t: &mut Tour, th: Thing, z: Zone) {
    t.app.press(Action::Board(BP::Chip(Chip::List(th))));
    t.app.press(Action::Board(BP::Zone(z)));
    t.pump();
}

/// Costs what a vault's making and unlock cost. A single-key wallet made
/// here and backed up on Paper and vault: the checklist, the copy by
/// hand checked, the vault made and saved into, and each stick written,
/// with Insert Stick 2 and a stick being written on the way.
fn checklist(t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::plan::Preset;
    let kind = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::NativeSegwit)
        .ok_or("no native SegWit")? as u8;
    t.press(Action::CreateWallet);
    // A kind moves on by itself (F103).
    t.press(Action::CKind(kind));
    crate::tour::device_key(t, 0);
    let w = t
        .app
        .create()
        .and_then(|c| c.built)
        .ok_or("the wallet was not made")?;
    t.press(Action::BackupFirst(w));
    t.backup_as(Preset::PaperVault);
    t.press(Action::BChecklist);
    t.shot("checklist")?;
    t.press(Action::BNext(bstep::BLANK));
    // The copy typed back, five words in, then the rest.
    t.press(Action::BCheck);
    type_words(t, Some(5))?;
    t.shot("check-copy")?;
    type_rest(t, 5)?;
    // An item done moves on by itself: to the vault's, here.
    if t.app.backup().and_then(|b| b.open) != Some(bstep::VAULT) {
        t.press(Action::BStep(bstep::VAULT));
    }
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "site phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "site phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() && t.app.screen == Screen::Backup {
            break;
        }
        t.tick();
    }
    t.press(Action::BVaultSave(0));
    if t.app.backup().and_then(|b| b.open) == Some(bstep::VAULT) {
        t.press(Action::BNext(bstep::VAULT));
    }
    if t.app.backup().and_then(|b| b.open) != Some(bstep::ENVELOPE) {
        t.press(Action::BStep(bstep::ENVELOPE));
    }
    t.press(Action::BNext(bstep::ENVELOPE));
    write_sticks(t)
}

/// The open copy item's seed typed back from word `from` on, a space
/// after each.
fn type_rest(t: &mut Tour, from: usize) -> Result<(), String> {
    let b = t.app.backup().ok_or("no backup")?;
    let words = t
        .app
        .session
        .keys
        .reveal(b.key, faraday_keys::Reveal::CopyByHand)
        .to_string();
    for w in words.split_whitespace().skip(from) {
        type_text(t, &format!("{w} "));
    }
    Ok(())
}

fn empty_stick(id: &str, serial: u32) -> StickInfo {
    StickInfo {
        serial: Some(serial),
        id: id.into(),
        label: "KEYSTICK".into(),
        boot: false,
        files: Vec::new(),
    }
}

/// Write to sticks: the one lock, then each stick by place as the flow
/// asks. The flow asking for the first stick it writes (Stick 2, the
/// boot stick being Stick 1), that stick while its files are written,
/// and the end.
fn write_sticks(t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::steps::FlowAction as F;
    t.press(Action::Flow(F::WalletLock));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.pump();
    for n in 0..6 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.written.is_none()));
        let Some(slot) = slot else { break };
        if n == 0 {
            t.shot("sticks-insert")?;
        }
        let stick = match t.app.flow().and_then(|f| f.slots[slot].bound.clone()) {
            Some(b) if b.label == "TESTSTICK" => t.stick(),
            Some(b) => empty_stick("b", b.serial.unwrap_or(2)),
            None => empty_stick(&format!("c{slot}"), 3 + slot as u32),
        };
        t.app.storage(StorageEvent::Sticks(vec![stick]));
        t.pump();
        for k in 0..2 {
            let hold = t
                .app
                .flow()
                .and_then(|f| f.live.reading.as_ref())
                .is_some_and(|r| r.hold);
            t.app
                .press(Action::Flow(if hold { F::Hold } else { F::Tap }));
            if n == 0 && k == 0 {
                t.shot_unanswered("sticks-writing", 0)?;
            }
            t.pump();
        }
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
    }
    for _ in 0..5 {
        t.tick();
    }
    t.shot("sticks-written")
}

/// Savings' PSBT signed with test key 1: Transaction, Check, Sign, four
/// frames of the signed PSBT's animated code; then, locked, the Spend
/// tab's map.
fn spend(t: &mut Tour) -> Result<(), String> {
    t.sticks(true);
    t.press(Action::VisitInAll);
    if let Some(i) = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "savings-unsigned.psbt")
    {
        t.press(Action::VisitIn(i));
    }
    t.press(Action::VisitCopy);
    t.sticks(false);
    t.load_kit()?;
    let psbt = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .ok_or("no psbt")?;
    t.press(Action::StartSpend(psbt));
    t.shot("spend-transaction")?;
    t.press(Action::StepNext(2));
    t.shot("spend-check")?;
    t.press(Action::StepNext(1));
    t.press(Action::StepNext(4));
    t.shot("spend-sign")?;
    t.press(Action::SignHere);
    t.press(Action::QrSigned);
    for k in 1..=4 {
        t.shot(&format!("spend-qr-{k}"))?;
    }
    t.press(Action::Cancel);
    Ok(())
}

/// With a key held: a stick in and the sheet to lock and use it; the
/// visit after the lock, the Outbox written and its receipt; Files;
/// Create a vault; the test vault's entries.
fn sticks_and_vaults(t: &mut Tour) -> Result<(), String> {
    t.sticks(true);
    t.shot("stick-lock")?;
    t.press(Action::Lock);
    if let Some((inbox, outbox, kept)) = t.boxes.clone() {
        t.app.storage(StorageEvent::Restored {
            inbox,
            outbox,
            kept,
        });
    }
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    t.shot("stick-visit")?;
    t.press(Action::VisitWrite);
    // The receipt, under For the stick on Files.
    t.press(Action::Nav(Screen::Files));
    t.shot("stick-written")?;
    t.press(Action::Nav(Screen::Visit));
    let vi = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "vault.ofv")
        .ok_or("the kit has no vault.ofv")?;
    t.press(Action::VisitInAll);
    t.press(Action::VisitIn(vi));
    t.press(Action::VisitCopy);
    t.sticks(false);
    t.press(Action::Nav(Screen::Files));
    t.shot("files")?;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    t.shot("vault-create")?;
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    crate::tour::not_now_load(t);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(2)));
    t.shot("vault-entries")?;
    // Locked: the Spend tab's step-by-step spend from a backup.
    fresh(t);
    t.press(Action::Nav(Screen::Family));
    t.shot("spend-map")
}

/// Restore a wallet: where it comes from; then Savings from its
/// description, test key 1 matched, a seed of no key refused, and Done.
fn restore(t: &mut Tour) -> Result<(), String> {
    use faraday_core::rstep;
    use faraday_core::seeds::SeedsAction as S;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::RestoreWallet);
    t.press(Action::RNext(rstep::KIND));
    t.shot("restore-sources")?;
    fresh(t);
    t.copy_wallets();
    t.sticks(false);
    t.press(Action::RestoreWallet);
    // A kind and No each move on by themselves (F103).
    t.press(Action::RKind(4));
    let i = t
        .app
        .inbox()
        .iter()
        .position(|it| it.name == "savings-wallet.txt")
        .ok_or("no savings-wallet.txt in Files")?;
    t.press(Action::RUse(i));
    t.press(Action::RVaults(false));
    t.press(Action::RSlotWords);
    t.type_key(0);
    t.shot("restore-keys")?;
    t.press(Action::RSlotWords);
    type_text(
        t,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    t.press(Action::EntryAdd);
    t.shot("restore-refused")?;
    let w = t
        .app
        .restore()
        .and_then(|r| r.wallet)
        .ok_or("the description made no wallet")?;
    let empty: Vec<u8> = t
        .app
        .session
        .slots(&t.app.session.wallets[w])
        .iter()
        .enumerate()
        .filter(|(_, s)| s.held_by.is_none())
        .map(|(k, _)| k as u8)
        .collect();
    for k in empty {
        t.press(Action::RAbsent(k));
    }
    t.press(Action::Seeds(S::Make));
    t.press(Action::RNext(rstep::CHECK));
    t.shot("restore-done")
}
