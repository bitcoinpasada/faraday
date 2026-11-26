//! A wallet whose key waits for its backup is not offered in Sign a
//! message: Sign is refused and says why, and no file, QR code or sheet
//! with the address is made. A signed message's file made before the key
//! was held stays in Files, and is neither written nor shown as a code
//! until the key is backed up.

use faraday_core::address_check::AddressPress;
use faraday_core::catalog::Go;
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::in_progress::Going;
use faraday_core::scan_read::Way;
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction;
use faraday_core::work::Work;
use faraday_core::{
    Action, Ask, Code, Faraday, Screen, StickInfo, StorageCommand, StorageEvent, testkit,
};
use faraday_psbt::message::{Format, Signed};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// A key of the test words with its single-key wallet.
fn app() -> Faraday {
    with_the_key(testkit::started())
}

fn with_the_key(mut app: Faraday) -> Faraday {
    let c = testkit::compute();
    app.session = testkit::session();
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.session
        .add_wallet_confirmed(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    app
}

fn hold(app: &mut Faraday) {
    let fp = app.session.keys.last().unwrap().fingerprint;
    testkit::mark_made_here(app, fp);
}

fn back_up(app: &mut Faraday) {
    let fp = app.session.keys.last().unwrap().fingerprint;
    testkit::release_made(app, fp);
}

/// A signature of the wallet's first address, made beside the flow: what the
/// flow would hold had it signed.
fn signature(app: &Faraday) -> Signed {
    app.session
        .sign_message(&testkit::compute(), 0, 0, "hello", Format::Bip322)
        .expect("signs")
}

/// Sign a message open on the held wallet, chosen by hand.
fn held_flow() -> Faraday {
    let mut app = app();
    hold(&mut app);
    app.press(Action::SignMessage);
    app.press(Action::MWallet(0));
    app
}

/// The refusal said, and nothing made: no file and no code.
fn assert_refused(app: &Faraday) {
    let m = testkit::message(app).expect("open");
    assert_eq!(m.error.as_deref(), Some(EN.message_waits_backup));
    assert!(m.out_name.is_none());
    assert!(app.outbox.is_empty(), "no file");
    assert!(app.qr().is_none(), "no code");
}

#[test]
fn a_held_wallet_is_not_offered() {
    let mut app = app();
    assert_eq!(app.tile_need(Go::SignMessage), None);
    hold(&mut app);
    assert_eq!(
        app.tile_need(Go::SignMessage),
        Some(EN.message_waits_backup)
    );
    app.press(Action::SignMessage);
    assert_eq!(testkit::message(&app).unwrap().wallet, None);
    app.press(Action::SignMessageWith(0));
    assert_eq!(testkit::message(&app).unwrap().wallet, None);
}

#[test]
fn sign_is_refused() {
    let mut app = held_flow();
    let before = faraday_core::work::snapshot();
    app.press(Action::MSign);
    assert_eq!(
        faraday_core::work::since(&before).of(Work::Sign),
        0,
        "nothing was signed"
    );
    assert!(testkit::message(&app).unwrap().signed.is_none());
    assert_refused(&app);
}

#[test]
fn the_address_checks_sign_button_opens_nothing_for_a_held_key() {
    let mut app = app();
    let address = signature(&app).address;
    hold(&mut app);
    app.press(Action::ScanFor(Ask::Address));
    app.event(Event::Scanned {
        bytes: address.as_bytes().to_vec(),
    });
    let _ = app.frame();
    let way = app
        .address_check_state()
        .and_then(|s| s.ways().iter().position(|w| matches!(w, Way::SignAddress)))
        .expect("the result's way to Sign a message");
    app.press(Action::AddressCheck(AddressPress::Way(way as u8)));
    assert!(testkit::message(&app).is_none());
    assert_ne!(app.screen, Screen::Message);
    assert_eq!(app.toast_text(), Some(EN.message_waits_backup));
}

#[test]
fn the_file_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::MOut);
    assert_refused(&app);
}

#[test]
fn the_code_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::MQr);
    assert_refused(&app);
}

#[test]
fn the_code_and_its_sheet_are_not_built() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    let why = app
        .code_view(&testkit::compute(), Code::Message)
        .err()
        .expect("no code");
    assert_eq!(why.as_str(), EN.message_waits_backup);
}

#[test]
fn check_it_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::CheckMessageMade);
    assert!(!testkit::message_made_kept(&app));
    assert_eq!(app.screen, Screen::Message);
    assert_refused(&app);
}

#[test]
fn a_key_held_after_signing_drops_what_was_made() {
    let mut app = app();
    app.press(Action::SignMessage);
    app.press(Action::MSign);
    app.press(Action::CheckMessageMade);
    assert!(testkit::message_made_kept(&app));
    hold(&mut app);
    assert!(!testkit::message_made_kept(&app));
    let m = testkit::message(&app).unwrap();
    assert!(m.signed.is_none());
    assert!(m.out_name.is_none() && m.out_sha.is_none());
    assert!(!m.done[faraday_core::mstep::SIGN as usize]);
}

#[test]
fn a_picked_address_of_a_held_key_opens_nothing() {
    let mut app = app();
    testkit::message_on_address(&mut app, "bc1qpicked");
    assert!(testkit::message(&app).unwrap().at.is_some());
    app.press(Action::MDone);
    hold(&mut app);
    testkit::message_on_address(&mut app, "bc1qpicked");
    assert!(testkit::message(&app).is_none());
}

#[test]
fn a_flow_left_half_way_signs_once_the_key_is_backed_up() {
    let mut app = held_flow();
    app.press(Action::MSign);
    assert!(testkit::message(&app).unwrap().signed.is_none());
    back_up(&mut app);
    app.press(Action::MType);
    assert_eq!(app.tile_need(Go::SignMessage), None);
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert!(m.signed.is_some());
    assert!(m.error.is_none());
    assert_eq!(app.outbox.len(), 1);
}

/// Sign a message signed with the key not held, its file For the stick and
/// ticked, on a screen wide enough for Files' rows.
fn signed_file() -> (Faraday, String) {
    signed_file_in(app())
}

fn signed_file_in(mut app: Faraday) -> (Faraday, String) {
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "hello");
    app.press(Action::MSign);
    let name = app.outbox.first().expect("the file").name.to_string();
    assert!(app.visit().out.contains(name.as_str()));
    (app, name)
}

fn row_says_it_waits(app: &mut Faraday) -> bool {
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.drawn_text_boxes()
        .iter()
        .any(|(t, _)| t == EN.message_waits_backup)
}

/// Show as QR on Files' first file: whether a code came, or the refusal.
fn code_shown(app: &mut Faraday) -> bool {
    app.press(Action::QrOutbox(0));
    let shown = app.qr().is_some();
    if !shown {
        assert_eq!(app.toast_text(), Some(EN.message_waits_backup));
    }
    shown
}

/// The app after a lock and the restart, over what the lock kept.
fn restarted(app: Faraday) -> Faraday {
    restarted_over(app, |_| {})
}

/// [`restarted`], with the kept record of the signed files put through
/// `edit`.
fn restarted_over(mut app: Faraday, edit: impl FnOnce(&mut String)) -> Faraday {
    app.press(Action::Lock);
    let mut saved = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, kept, .. } = c {
            saved = Some((outbox, kept));
        }
    }
    let (outbox, mut kept) = saved.expect("the lock kept the boxes");
    let (_, bytes) = kept
        .iter_mut()
        .find(|(n, _)| &**n == "signed-files")
        .expect("the signed files' record");
    let mut record = String::from_utf8(bytes.to_vec()).expect("text");
    edit(&mut record);
    *bytes = record.into_bytes().into();
    let mut next = testkit::started();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox,
        kept,
    });
    while next.poll_storage().is_some() {}
    next
}

fn stick_in(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("one"),
        label: faraday_text::Text::of("FIRST"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
}

/// A stick visit's Write with everything Select all takes, and `name`
/// ticked besides when `tick`: the names the write sends. `waits` says
/// that Select all is to leave `name` out.
fn visit_writes(app: &mut Faraday, name: &str, tick: bool, waits: bool) -> Vec<String> {
    stick_in(app);
    // The stick arrives with what it can take ticked: untick all, then
    // Select all.
    // With only some ticked the first press ticks the rest.
    for _ in 0..2 {
        if !app.visit().out.is_empty() {
            app.press(Action::VisitOutAll);
        }
    }
    assert!(app.visit().out.is_empty(), "all unticked");
    app.press(Action::VisitOutAll);
    assert_eq!(app.visit().out.contains(name), !waits, "Select all");
    if tick {
        testkit::visit_tick(app, name);
    }
    while app.poll_storage().is_some() {}
    app.press(Action::VisitWrite);
    let mut names = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { name, .. } = c {
            names.push(name.to_string());
        }
    }
    names
}

/// The signed file `name` of an app whose key is now held: it stays in
/// Files, unticked, its row says why, it is no code, and a stick visit
/// writes the other file and not it, a tick left standing included. Backed
/// up, it is written, and it is a code when it is small enough for one
/// (`a_code`).
fn held_back_until_backed_up(mut app: Faraday, name: &str, a_code: bool) {
    let fp = app.session.keys.last().unwrap().fingerprint;
    assert!(app.outbox.iter().any(|i| i.name == name), "it stays");
    assert!(!app.visit().out.contains(name), "unticked");
    assert!(row_says_it_waits(&mut app));
    app.press(Action::VisitOut(0));
    assert!(!app.visit().out.contains(name), "no tick");
    assert!(!code_shown(&mut app));

    testkit::outbox_file(&mut app, "other.txt", b"other");
    // The lock and the restart: no key is loaded, and the file is held.
    let mut next = restarted(app);
    assert_eq!(next.session.keys.all().count(), 0);
    assert_held_without_the_key(&mut next, name);

    // The write answered and the stick out, the same words loaded again:
    // a key typed from its words counts as backed up.
    next.storage(StorageEvent::Written {
        stick: faraday_text::Text::of("one"),
        name: faraday_text::Text::of("other.txt"),
        wrote_as: faraday_text::Text::of("other.txt"),
    });
    while next.poll_storage().is_some() {}
    next.storage(StorageEvent::Sticks(Vec::new()));
    while next.poll_storage().is_some() {}
    load_the_key(&mut next);
    assert_eq!(next.session.keys.last().unwrap().fingerprint, fp);
    // The refusal's toast says the same words: its time runs out first.
    next.event(Event::Tick {
        now_ms: 10 * faraday_draw::tokens::TOAST_SHOW_MS,
    });
    assert_eq!(next.toast_text(), None);
    next.press(Action::Nav(Screen::Files));
    assert!(!row_says_it_waits(&mut next));
    if a_code {
        assert!(code_shown(&mut next));
        next.press(Action::Cancel);
    } else {
        // Too large for a code: nothing says it waits.
        next.press(Action::QrOutbox(0));
        assert!(next.qr().is_none());
        assert_eq!(next.toast_text(), None);
    }
    // A process that has had a key loaded locks before it writes: the next
    // one has none, and the hold is gone from what the lock kept.
    let mut last = restarted(next);
    assert!(!row_says_it_waits(&mut last));
    let written = visit_writes(&mut last, name, true, false);
    assert!(written.iter().any(|n| n == name), "{written:?}");
}

fn load_the_key(app: &mut Faraday) {
    let c = testkit::compute();
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
}

/// With no key loaded: the held file `name`, first in Files, says it waits,
/// takes no tick, is no code, and a stick visit writes `other.txt` and not
/// it.
fn assert_held_without_the_key(app: &mut Faraday, name: &str) {
    assert!(row_says_it_waits(app));
    assert!(!app.visit().out.contains(name), "unticked");
    app.press(Action::VisitOut(0));
    assert!(!app.visit().out.contains(name), "no tick");
    assert!(!code_shown(app));
    let written = visit_writes(app, name, true, true);
    assert!(written.iter().any(|n| n == "other.txt"), "{written:?}");
    assert!(!written.iter().any(|n| n == name), "{written:?}");
}

/// An open vault's record of the key's words, its backup pending, holds the
/// key only while it is loaded: Remove every key leaves the file held.
#[test]
#[allow(clippy::disallowed_methods)] // a test vault's own record of the words
fn a_held_file_stays_held_after_remove_every_key() {
    use faraday_vault::records::{self, field, kind};
    const PHRASE: &str = "correct horse battery staple";
    let c = testkit::compute();
    let file = crate::common::vault_of("Main", PHRASE, 0x41, Vec::new());
    let mut app = crate::common::process(vec![("main.kdbx".into(), file)], Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(VaultAction::Open(0)));
    testkit::vault_speed(&mut app, Some(180));
    crate::common::unlock(&mut app, PHRASE);
    let (mut app, name) = signed_file_in(with_the_key(app));
    let id = app.vaults().open[0].id;
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    testkit::vault_edit(&mut app, id, |s| {
        s.insert(
            id,
            faraday_vault::Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&m))
                .with(field::KEY_FLAGS, &[records::BACKUP_PENDING]),
        )
    });
    app.event(Event::Tick { now_ms: 1 });
    assert!(row_says_it_waits(&mut app));
    testkit::outbox_file(&mut app, "other.txt", b"other");
    app.press(Action::RemoveKeys);
    app.press(Action::RemoveKeysHold);
    assert_eq!(app.session.keys.all().count(), 0);
    assert!(row_says_it_waits(&mut app));
    app.press(Action::VisitOut(0));
    assert!(!app.visit().out.contains(name.as_str()), "no tick");
    assert!(!code_shown(&mut app));
    // Select all leaves it. This process has had a key, so Write asks for
    // a lock first and sends nothing.
    let written = visit_writes(&mut app, &name, true, true);
    assert!(!written.contains(&name), "{written:?}");
    // The lock is confirmed. In the next process the stick comes in, and
    // nothing writes the held file.
    let mut next = restarted(app);
    stick_in(&mut next);
    while let Some(c) = next.poll_storage() {
        if let StorageCommand::Write { name: wrote, .. } = c {
            assert!(*wrote != *name, "the held file was written");
        }
    }
    next.storage(StorageEvent::Sticks(Vec::new()));
    while next.poll_storage().is_some() {}
    assert_held_without_the_key(&mut next, &name);
}

/// A key with the same fingerprint and another id cannot be made here, so
/// the id the lock kept is changed instead: the key loaded after the
/// restart is then not the one that signed, and the file stays held. The
/// same words, over the record as it was kept, release it for good.
#[test]
fn only_the_key_that_signed_releases_a_held_file() {
    for another in [true, false] {
        let (mut app, name) = signed_file();
        hold(&mut app);
        let signer = app.session.keys.last().unwrap();
        let (fp, id) = (signer.fingerprint, signer.id);
        let id = faraday_crypto::hex::encode_string(&id.0);
        let mut next = restarted_over(app, |record| {
            assert!(record.contains(&*id), "the id is kept");
            if another {
                *record = record.replace(&*id, &"5a".repeat(32));
            }
        });
        assert!(row_says_it_waits(&mut next), "no key loaded");
        load_the_key(&mut next);
        // Brings the holds up to date with the key now loaded.
        testkit::mark_made_here(&mut next, faraday_bip::keys::Fingerprint([0; 4]));
        assert_eq!(next.session.keys.last().unwrap().fingerprint, fp);
        assert_eq!(row_says_it_waits(&mut next), another, "{name}");
        // The key gone again: what the holds were brought to stays.
        next.press(Action::RemoveKeys);
        next.press(Action::RemoveKeysHold);
        assert_eq!(next.session.keys.all().count(), 0);
        assert_eq!(row_says_it_waits(&mut next), another);
    }
}

/// A picture made of the signature while its key is away is recorded with the
/// key that signed: held once that key is back and waits for its backup, it
/// stays held in the next process.
#[test]
fn a_picture_made_with_the_signing_key_away_is_held_back() {
    let (mut app, text) = signed_file();
    app.press(Action::RemoveKeys);
    app.press(Action::RemoveKeysHold);
    assert_eq!(app.session.keys.all().count(), 0);
    app.press(Action::Resume(Going::Message));
    app.press(Action::MQr);
    app.press(Action::QrPng);
    if app.qr().is_some() {
        app.press(Action::Cancel);
    }
    let picture = app
        .outbox
        .iter()
        .map(|i| i.name.to_string())
        .find(|n| n.ends_with(".png"))
        .expect("the picture");
    load_the_key(&mut app);
    hold(&mut app);
    // The text file goes, so the first row is the picture's.
    app.press(Action::Nav(Screen::Files));
    assert_eq!(app.outbox[0].name.as_str(), text);
    app.press(Action::OutboxRemove(0));
    if app.sheet.is_some() {
        app.press(Action::OutboxDeleteHold);
    }
    assert_eq!(app.outbox.len(), 1);
    testkit::outbox_file(&mut app, "other.txt", b"other");
    let mut next = restarted(app);
    assert_eq!(next.session.keys.all().count(), 0);
    assert_held_without_the_key(&mut next, &picture);
}

/// A signature outlives its wallet, and so does the record of who signed: a
/// picture made of it after the wallet was removed, while the key waits for
/// its backup, is not ticked and stays held in the next process.
#[test]
fn a_picture_made_after_the_wallet_was_removed_is_held_back() {
    let (mut app, text) = signed_file();
    app.press(Action::RemoveWallet(0));
    assert!(app.session.wallets.is_empty());
    hold(&mut app);
    app.press(Action::Resume(Going::Message));
    app.press(Action::MQr);
    app.press(Action::QrPng);
    if app.qr().is_some() {
        app.press(Action::Cancel);
    }
    let picture = app
        .outbox
        .iter()
        .map(|i| i.name.to_string())
        .find(|n| n.ends_with(".png"))
        .expect("the picture");
    assert!(!app.visit().out.contains(picture.as_str()), "unticked");
    // The text file goes, so the first row is the picture's.
    app.press(Action::Nav(Screen::Files));
    assert_eq!(app.outbox[0].name.as_str(), text);
    app.press(Action::OutboxRemove(0));
    if app.sheet.is_some() {
        app.press(Action::OutboxDeleteHold);
    }
    assert_eq!(app.outbox.len(), 1);
    testkit::outbox_file(&mut app, "other.txt", b"other");
    let mut next = restarted(app);
    assert_eq!(next.session.keys.all().count(), 0);
    assert_held_without_the_key(&mut next, &picture);
}

/// A key held while the signed message's code is on show: the signature is
/// dropped, its code goes with it, and no picture of it is made.
#[test]
fn a_code_on_show_goes_with_a_dropped_signature() {
    let (mut app, text) = signed_file();
    app.press(Action::MQr);
    let _ = app.frame();
    assert!(app.offers(Action::QrPng));
    hold(&mut app);
    assert!(testkit::message(&app).unwrap().signed.is_none());
    assert!(app.qr().is_none(), "no code");
    assert!(app.sheet.is_none(), "no sheet");
    app.press(Action::QrPng);
    assert_eq!(app.outbox.len(), 1, "no picture");
    assert_eq!(app.outbox[0].name.as_str(), text);
    // After a press, which draws anew.
    let _ = app.frame();
    assert!(!app.offers(Action::QrPng));
}

#[test]
fn a_file_made_before_the_hold_is_held_back() {
    let (mut app, name) = signed_file();
    hold(&mut app);
    held_back_until_backed_up(app, &name, true);
}

/// The signed message's picture, made by `presses` before the hold, is held
/// back as the text file is.
fn picture_is_held_back(presses: &[Action]) {
    let (mut app, text) = signed_file();
    for press in presses {
        app.press(*press);
    }
    if app.qr().is_some() {
        app.press(Action::Cancel);
    }
    let picture = app
        .outbox
        .iter()
        .map(|i| i.name.to_string())
        .find(|n| n.ends_with(".png"))
        .expect("the picture");
    // The text file goes, so the first row is the picture's.
    app.press(Action::Nav(Screen::Files));
    assert_eq!(app.outbox[0].name.as_str(), text);
    app.press(Action::OutboxRemove(0));
    if app.sheet.is_some() {
        app.press(Action::OutboxDeleteHold);
    }
    assert_eq!(app.outbox.len(), 1);
    // More than a file shown as a code may hold.
    assert!(app.outbox[0].bytes.len() > 256 * 1024);
    hold(&mut app);
    held_back_until_backed_up(app, &picture, false);
}

#[test]
fn a_picture_for_the_stick_made_before_the_hold_is_held_back() {
    picture_is_held_back(&[Action::CodePng(Code::Message)]);
}

#[test]
fn a_picture_from_the_code_sheet_made_before_the_hold_is_held_back() {
    picture_is_held_back(&[Action::MQr, Action::QrPng]);
}

#[test]
fn a_file_of_a_flow_ended_with_done_is_held_back() {
    let (mut app, name) = signed_file();
    app.press(Action::MDone);
    assert!(testkit::message(&app).is_none());
    hold(&mut app);
    held_back_until_backed_up(app, &name, true);
}

#[test]
fn a_file_signed_before_the_number_changed_is_held_back() {
    let (mut app, name) = signed_file();
    app.press(Action::MIndex(1));
    assert!(testkit::message(&app).unwrap().signed.is_none());
    hold(&mut app);
    held_back_until_backed_up(app, &name, true);
}

#[test]
fn a_renamed_signed_file_is_held_back_still() {
    let (mut app, name) = signed_file();
    app.press(Action::Nav(Screen::Files));
    app.press(Action::File(FilePress::Rename(Row::Out(0))));
    app.event(Event::Key(Key::Char('x')));
    app.press(Action::File(FilePress::RenameGo));
    let renamed = app.outbox[0].name.to_string();
    assert_ne!(renamed, name);
    hold(&mut app);
    assert!(!code_shown(&mut app));
}

#[test]
fn another_file_under_the_name_is_not_held_back() {
    let (mut app, name) = signed_file();
    app.press(Action::OutboxRemove(0));
    if app.sheet.is_some() {
        app.press(Action::OutboxDeleteHold);
    }
    assert!(app.outbox.is_empty());
    testkit::outbox_file(&mut app, &name, b"other");
    hold(&mut app);
    assert!(!row_says_it_waits(&mut app));
    assert!(code_shown(&mut app));
}
