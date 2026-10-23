//! The guide tour: the pictures in `docs/GUIDE.md` §2 to §5.

use crate::tour::{Tour, display, scroll_to, type_text};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::devices::DeviceCheck;
use faraday_core::keygen::{Way, kstep};
use faraday_core::{Action, Faraday, Medium, Screen, Sheet, StickInfo, StorageEvent, cstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, Key, TouchPhase};

/// EntropyLab's BitBox transcript for twelve words (`tools/reference/dice/
/// README.md`), the rolls the guide types: the last two are not taken.
const ROLLS: &str = "123411234122341233412344123415234126341231412342123413234124341235432141";

/// Costs what the parts it runs cost. A fresh stick session as a person
/// meets it: the stick it started from still in, Secure Boot off at
/// start, a laptop named for no maker; then Home, the power sheet,
/// Wallets, and New key made from the dice transcript, with this
/// device's randomness shown on the way, the words and the quiz; then a
/// single-key wallet from that key, the Lock sheet while it waits for
/// its backup, and the wallet backed up (§4); and a 2-of-3 over it and
/// two test keys (§5).
pub(crate) fn guide_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let size = t.size();
    t.app = Faraday::new();
    t.app.medium = Medium::Stick;
    t.app.devices = DeviceCheck::machine("flags\t: fpu vme", "Example", "Example 14", "10");
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    t.app.event(display(size, t.dpi));
    t.app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        serial: Some(0x1234_5678),
        files: Vec::new(),
    }]));
    t.pump();
    // An IP-KVM attached at start: named before anything else, then
    // unplugged and checked again.
    t.app.usb_devices(&[super::devices::kvm()]);
    t.pump();
    t.shot("guide-devices-kvm")?;
    t.app.usb_devices(&[]);
    t.press(Action::DevicesCheck);
    for _ in 0..3 {
        t.tick();
    }
    t.shot("guide-home")?;
    t.press(Action::PowerAsk);
    t.shot("guide-power-sheet")?;
    t.app.event(Event::Key(Key::Escape));
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("guide-wallets")?;
    t.go_on = false;
    t.press(Action::KeyGen(None));
    t.shot("guide-pull-sheet")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    if t.app.sheet == Some(Sheet::Devices) {
        t.shot("guide-devices-sheet")?;
        t.press(Action::DevicesUnderstood);
    }
    t.go_on = true;
    t.shot("guide-newkey-start")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-length")?;
    t.press(Action::KWords(12));
    t.shot("guide-newkey-randomness")?;
    new_key(t)?;
    single_key(t)?;
    board(t)?;
    checklist(c, t)?;
    multisig(t)?;
    multisig_board(t)?;
    scan_key(c, t)
}

/// New key from Randomness on: this device's generator chosen and its
/// entry card, then the dice transcript, the words and the quiz.
fn new_key(t: &mut Tour) -> Result<(), String> {
    t.press(Action::KGroup(2));
    t.press(Action::KWay(Way::Device.index()));
    scroll_to(t, Action::KNext);
    t.shot("guide-newkey-randomness-device")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-device-entry")?;
    t.press(Action::KStep(kstep::SOURCE));
    t.press(Action::KWay(Way::DiceWords.index()));
    t.shot("guide-newkey-randomness-dice")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-rolls-empty")?;
    // A 5 in a word's first five places is not taken.
    t.press(Action::KRoll(5));
    t.shot("guide-newkey-rolls-five")?;
    for (i, c) in ROLLS.bytes().enumerate() {
        t.press(Action::KRoll(c - b'0'));
        if i == 23 {
            t.shot("guide-newkey-rolls-some")?;
        }
    }
    scroll_to(t, Action::KNext);
    t.shot("guide-newkey-rolls-done")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-key")?;
    t.press(Action::KLock);
    t.shot("guide-newkey-locked")?;
    t.press(Action::KNext);
    t.shot("guide-newkey-words")?;
    t.press(Action::KShow);
    t.shot("guide-newkey-words-shown")?;
    t.press(Action::KShow);
    t.press(Action::KNext);
    t.shot("guide-newkey-quiz")?;
    let wrong = t
        .app
        .keygen()
        .and_then(|k| k.quiz.as_ref())
        .map(|q| (q.correct_slot() as u8 + 1) % 4)
        .ok_or("no quiz")?;
    t.press(Action::KQuiz(wrong));
    t.shot("guide-newkey-quiz-wrong")?;
    t.press(Action::KQuizRetry);
    super::default::pass_quiz(t);
    t.shot("guide-newkey-quiz-passed")?;
    t.press(Action::KAdd);
    t.shot("guide-newkey-loaded")
}

/// The dice key's words, as §3 makes them.
const DICE_WORDS: &str =
    "brand hobby ranch shoulder brass hockey ranch short brand hockey random torch";

/// §4.1: Make a new wallet, a single key from the dice key: Kind, the
/// vault question, Keys, Check, and its Back up step.
fn single_key(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.shot("guide-single-kind")?;
    t.press(Action::CNext(cstep::KIND));
    t.shot("guide-single-next")?;
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.shot("guide-single-keys")?;
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.shot("guide-single-keys-filled")?;
    t.press(Action::CNext(cstep::KEYS));
    t.shot("guide-single-check")?;
    // Lock while the wallet waits for its backup: the sheet asks.
    t.press(Action::LockAsk);
    t.shot("guide-lock-sheet")?;
    t.press(Action::Cancel);
    t.press(Action::CNext(cstep::CHECK));
    t.shot("guide-single-backup-step")?;
    t.press(Action::CBackup);
    t.shot("guide-board-standard")
}

/// A finger on the panel at `at`, as `phase`.
fn touch(t: &mut Tour, (x, y): (u16, u16), phase: TouchPhase) {
    t.app.event(Event::Touch { x, y, phase });
    t.pump();
}

/// `chip` dragged by a finger onto `zone`: a shot on the way, over the
/// zone, and once dropped.
fn drag(t: &mut Tour, chip: Chip, zone: Zone, name: &str) -> Result<(), String> {
    let _ = t.app.frame();
    let from = t
        .app
        .where_offered(Action::Board(BP::Chip(chip)))
        .ok_or("the chip is not on screen")?;
    let to = t
        .app
        .where_offered(Action::Board(BP::Zone(zone)))
        .ok_or("the zone is not on screen")?;
    touch(t, from, TouchPhase::Down);
    let step =
        |a: u16, b: u16, k: i32| (i32::from(a) + (i32::from(b) - i32::from(a)) * k / 4) as u16;
    for k in 1..=4 {
        touch(
            t,
            (step(from.0, to.0, k), step(from.1, to.1, k)),
            TouchPhase::Move,
        );
    }
    t.shot(&format!("{name}-over"))?;
    touch(t, to, TouchPhase::Up);
    t.shot(&format!("{name}-dropped"))
}

/// §4.2 to §4.5: the board after Customize backup; a new vault dragged
/// onto Place 1; the seed's ⋮ menu putting it in the vault;
/// the description put there too; the SeedQR at Place 1 turned on.
fn board(t: &mut Tour) -> Result<(), String> {
    t.app.press(Action::Board(BP::Customize));
    t.pump();
    t.shot("guide-board-customize")?;
    t.press(Action::Board(BP::AddVault));
    t.shot("guide-board-vault-added")?;
    drag(
        t,
        Chip::List(Thing::Vault(0)),
        Zone::Card(0),
        "guide-board-drag",
    )?;
    t.press(Action::Board(BP::Menu(Chip::List(Thing::Seed(0)))));
    t.shot("guide-board-menu")?;
    t.press(Action::Board(BP::MenuTo(Zone::Vault(0, 0, 0))));
    t.shot("guide-board-seed-in-vault")?;
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Whole))));
    t.press(Action::Board(BP::Zone(Zone::Vault(0, 0, 0))));
    let seed = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    t.press(Action::Board(BP::Chip(seed)));
    t.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1)));
    t.press(Action::Board(BP::LetGo));
    t.shot("guide-board-done")?;
    scroll_to(t, Action::BChecklist);
    t.shot("guide-board-plan")
}

/// Costs a mnemonic's checksum. §4.6 to §4.9: the checklist; the blank template; the copy by hand
/// and Check my copy; the vault made and saved into; the sticks written.
fn checklist(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::bstep;
    use faraday_core::vault::VaultAction as V;
    t.press(Action::BChecklist);
    t.shot("guide-checklist")?;
    t.press(Action::BNext(bstep::BLANK));
    t.shot("guide-checklist-copy")?;
    t.press(Action::BReveal);
    t.shot("guide-checklist-copy-shown")?;
    t.press(Action::BReveal);
    t.press(Action::BCheck);
    t.shot("guide-check-copy")?;
    crate::tour::type_words(t, Some(5))?;
    t.shot("guide-check-copy-some")?;
    for w in DICE_WORDS.split_whitespace().skip(5) {
        type_text(t, &format!("{w} "));
    }
    t.shot("guide-check-copy-words")?;
    // The SeedQR copy, scanned.
    t.press(Action::BScan);
    super::site_tour::camera(t);
    t.shot("guide-check-copy-scan")?;
    t.app.event(Event::Scanned {
        bytes: crate::tour::copy_digits(c, t, None)?,
    });
    t.pump();
    t.shot("guide-check-copy-done")?;
    t.press(Action::BStep(bstep::VAULT));
    t.shot("guide-checklist-vault")?;
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    t.shot("guide-vault-create")?;
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "guide vault phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "guide vault phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() && t.app.screen == Screen::Backup {
            break;
        }
        t.tick();
    }
    t.shot("guide-checklist-vault-made")?;
    t.press(Action::BVaultSave(0));
    t.shot("guide-checklist-vault-saved")?;
    t.press(Action::BNext(bstep::VAULT));
    t.shot("guide-checklist-next")?;
    if t.app.backup().and_then(|b| b.open) != Some(bstep::ENVELOPE) {
        t.press(Action::BStep(bstep::ENVELOPE));
    }
    t.shot("guide-checklist-envelope")?;
    t.press(Action::BNext(bstep::ENVELOPE));
    t.shot("guide-checklist-sticks")?;
    write_sticks(t)
}

/// The one lock, then each stick the plan names, as the flow asks.
fn write_sticks(t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::steps::FlowAction as F;
    t.press(Action::Flow(F::WalletLock));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.pump();
    t.shot("guide-sticks-after-lock")?;
    for n in 0..6 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.written.is_none()));
        let Some(slot) = slot else { break };
        t.shot(&format!("guide-sticks-insert-{n}"))?;
        let stick = StickInfo {
            serial: Some(0x2000 + slot as u32),
            id: format!("s{slot}"),
            label: "KEYSTICK".into(),
            boot: false,
            files: Vec::new(),
        };
        t.app.storage(StorageEvent::Sticks(vec![stick]));
        t.pump();
        t.shot(&format!("guide-sticks-read-{n}"))?;
        for _ in 0..2 {
            let hold = t
                .app
                .flow()
                .and_then(|f| f.live.reading.as_ref())
                .is_some_and(|r| r.hold);
            t.app
                .press(Action::Flow(if hold { F::Hold } else { F::Tap }));
            t.pump();
        }
        t.shot(&format!("guide-sticks-written-{n}"))?;
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
    }
    for _ in 0..5 {
        t.tick();
    }
    t.shot("guide-sticks-done")
}

/// §5.1 to §5.4: a fresh session; the dice key typed back from its
/// paper and test keys 2 and 3 typed; Make a new wallet, a 2-of-3:
/// Kind, Quorum, Keys, Check; the wallet card; its wallet QR.
fn multisig(t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    t.restart();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.pump();
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Entry(None));
    t.shot("guide-type-words")?;
    type_text(t, DICE_WORDS);
    t.shot("guide-type-words-filled")?;
    t.press(Action::EntryAdd);
    t.shot("guide-type-words-added")?;
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.shot("guide-multi-kind")?;
    t.press(Action::CNext(cstep::KIND));
    t.shot("guide-multi-quorum")?;
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.shot("guide-multi-keys")?;
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.shot("guide-multi-keys-filled")?;
    t.press(Action::CNext(cstep::KEYS));
    t.shot("guide-multi-check")?;
    scroll_to(t, Action::CNext(cstep::CHECK));
    t.shot("guide-multi-check-addresses")?;
    let w = t
        .app
        .create()
        .and_then(|c| c.built)
        .ok_or("the wallet was not made")?;
    t.press(Action::OpenWallet(w));
    t.shot("guide-multi-card")?;
    t.press(Action::QrWallet(w));
    t.shot("guide-multi-wallet-qr")?;
    t.press(Action::Cancel);
    Ok(())
}

/// §5.5: the 2-of-3's board, the standard way; a share at Place 3 that
/// lets it see the balance; a second seed at Place 1 that lets it spend.
fn multisig_board(t: &mut Tour) -> Result<(), String> {
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Backup(w));
    t.shot("guide-multi-board")?;
    t.app.press(Action::Board(BP::Customize));
    t.pump();
    t.press(Action::Board(BP::Off(Zone::Paper(2), Thing::Whole)));
    t.app
        .press(Action::Board(BP::Chip(Chip::List(Thing::Share(0)))));
    t.app.press(Action::Board(BP::Zone(Zone::Paper(2))));
    t.pump();
    t.shot("guide-multi-sees-sheet")?;
    t.app.press(Action::Board(BP::SeesUndo));
    t.app
        .press(Action::Board(BP::Chip(Chip::List(Thing::Seed(1)))));
    t.app.press(Action::Board(BP::Zone(Zone::Paper(0))));
    t.pump();
    t.shot("guide-multi-spend-sheet")?;
    t.app.press(Action::Board(BP::SeesUndo));
    t.pump();
    Ok(())
}

/// Costs a mnemonic's checksum. The SeedQR digits of `words`, as a
/// camera reads them off the code.
fn seedqr(c: &Compute, words: &str) -> Result<Vec<u8>, String> {
    let m = faraday_bip::bip39::Mnemonic::parse(c, faraday_bip::bip39::Language::English, words)
        .map_err(|e| format!("{e:?}"))?;
    Ok(faraday_codec::seedqr::to_digits(&m)
        .expose()
        .as_bytes()
        .to_vec())
}

/// Costs two mnemonics' checksums. §5.6: Remove every key; the 2-of-3's
/// card short of keys; Scan a key; test key 2's SeedQR, then the dice
/// key's.
fn scan_key(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Nav(Screen::Settings));
    t.shot("guide-settings-session")?;
    t.press(Action::RemoveKeys);
    t.shot("guide-keys-removed")?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot("guide-multi-card-short")?;
    t.press(Action::ScanKey(w));
    super::site_tour::camera(t);
    t.shot("guide-scan-key-camera")?;
    let zebra = faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0);
    t.app.event(Event::Scanned {
        bytes: seedqr(c, &zebra)?,
    });
    t.pump();
    t.shot_after("guide-scan-key-one", 0)?;
    t.app.event(Event::Scanned {
        bytes: seedqr(c, DICE_WORDS)?,
    });
    t.pump();
    t.shot_after("guide-scan-key-two", 0)?;
    t.shot("guide-multi-card-can-spend")
}
