//! The backup starts with a plan and goes on to a checklist of only what
//! the plan needs. Each item is done by what it does: the template or the
//! sheet in the Outbox, the copy by hand matched, the seed and the wallet
//! in the vault; the envelopes alone by a press. A place's name is kept
//! in the vault, where the next backup of the wallet finds the plan, and
//! in no file. The blank template has a line per place and no name.

use faraday_core::plan::Item;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, BStage, Faraday, Screen, StorageEvent, bstep, qrow, qstep};
use faraday_vault::records::{field, kind};
use osk_bip::bip39::{Language, Mnemonic};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

const NAME: &str = "Harbour cellar";

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
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
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = app.frame();
    app
}

fn vault_file() -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap()
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn unlock(app: &mut Faraday) {
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
}

/// Test key 2 typed in and a one-key wallet made from it; returns the
/// wallet's index.
fn one_key(app: &mut Faraday) -> usize {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().master.fingerprint();
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    app.session.wallets.len() - 1
}

/// Presses `action`, which the screen on show must offer somewhere down
/// its column.
fn press_offered(app: &mut Faraday, action: Action) {
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(action) {
            app.press(action);
            return;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    panic!("{action:?} is not offered");
}

fn done(app: &Faraday, item: Item) -> bool {
    app.backup_item_done(item)
}

#[test]
fn the_checklist_marks_each_item_done_by_what_it_does() {
    let mut app = device(vec![vault_file()]);
    let w = one_key(&mut app);
    unlock(&mut app);
    app.press(Action::Backup(w));
    assert_eq!(app.backup.as_ref().unwrap().q, Some(qstep::PRESET));
    press_offered(&mut app, Action::BPreset(1));
    press_offered(&mut app, Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    assert_eq!(
        app.backup_items(),
        vec![
            bstep::BLANK,
            bstep::COPY,
            bstep::VAULT,
            bstep::WALLET,
            bstep::SHEETS,
            bstep::ENVELOPE
        ],
        "only what paper and a vault need"
    );
    for item in [
        Item::Templates,
        Item::Copy(0),
        Item::SeedsVault,
        Item::WalletVault,
        Item::Sheets,
        Item::Envelopes,
    ] {
        assert!(!done(&app, item), "{item:?} done before anything");
    }
    // The template, in the Outbox.
    press_offered(&mut app, Action::BOut(0));
    assert!(done(&app, Item::Templates));
    // The copy, typed back.
    press_offered(&mut app, Action::BStep(bstep::COPY));
    app.press(Action::BReveal);
    app.storage(StorageEvent::Cameras(Vec::new()));
    press_offered(&mut app, Action::BCheck);
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let digits = osk_codec::seedqr::to_digits(&Mnemonic::parse(Language::English, &words).unwrap());
    type_text(
        &mut app,
        std::str::from_utf8(digits.expose().as_bytes()).unwrap(),
    );
    assert!(done(&app, Item::Copy(0)));
    assert!(!done(&app, Item::SeedsVault), "the copy is not the vault");
    // The seed and the wallet, into the vault.
    press_offered(&mut app, Action::BStep(bstep::VAULT));
    press_offered(&mut app, Action::BVault(false));
    assert!(done(&app, Item::SeedsVault));
    press_offered(&mut app, Action::BStep(bstep::WALLET));
    press_offered(&mut app, Action::Vault(V::SaveWallet(w)));
    assert!(done(&app, Item::WalletVault));
    // The sheet, in the Outbox.
    press_offered(&mut app, Action::BStep(bstep::SHEETS));
    press_offered(&mut app, Action::BOut(3));
    assert!(done(&app, Item::Sheets));
    // The envelopes, by a press; then the way to a stick.
    press_offered(&mut app, Action::BStep(bstep::ENVELOPE));
    press_offered(&mut app, Action::BNext(bstep::ENVELOPE));
    assert!(done(&app, Item::Envelopes));
    press_offered(&mut app, Action::WriteAsk);
}

#[test]
fn a_place_name_is_kept_in_the_vault_and_in_no_file() {
    let mut app = device(vec![vault_file()]);
    let w = one_key(&mut app);
    unlock(&mut app);
    app.press(Action::Backup(w));
    press_offered(&mut app, Action::BPreset(2));
    press_offered(&mut app, Action::BQ(qstep::PLACES));
    press_offered(&mut app, Action::BName(0));
    type_text(&mut app, NAME);
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.place_name(0), NAME);
    assert_eq!(app.place_name(1), "Place 2");
    // Every file the plan can make: text and pictures, the seed as a file.
    app.press(Action::BAnswer(
        qrow::WALLET,
        faraday_core::plan::wallet::FILES as u8,
    ));
    app.press(Action::BAnswer(
        qrow::FORM,
        faraday_core::plan::form::TEXT as u8,
    ));
    app.press(Action::BAnswer(
        qrow::SEEDS,
        faraday_core::plan::seeds::FILE as u8,
    ));
    press_offered(&mut app, Action::BChecklist);
    let plans: Vec<_> = app.vaults.open[0]
        .contents
        .of(kind::PLAN)
        .map(|(_, r)| r.clone())
        .collect();
    assert_eq!(plans.len(), 1, "one plan for the wallet");
    assert_eq!(plans[0].text(field::PLAN_PLACE), Some(NAME));
    for what in [0u8, 1, 3, 5, 7, 8] {
        app.press(Action::PublicOut(w, what));
    }
    app.press(Action::QrWallet(w));
    app.press(Action::Cancel);
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert!(app.outbox.len() >= 7, "{:?}", app.outbox.len());
    for item in &app.outbox {
        assert!(
            !item.bytes.windows(NAME.len()).any(|b| b == NAME.as_bytes()),
            "{} holds the place's name",
            item.name
        );
    }
    // Made again, the plan is replaced, not added to.
    app.press(Action::BPlan);
    app.press(Action::BAnswer(qrow::STICKS, 1));
    app.press(Action::BChecklist);
    assert_eq!(app.vaults.open[0].contents.of(kind::PLAN).count(), 1);
    // The next backup of the wallet starts from it.
    let answers = app.backup.as_ref().unwrap().answers.clone();
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::Backup(w));
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.answers, answers);
    assert_eq!(b.q, None, "not the presets");
    assert_eq!(app.place_name(0), NAME);
}

#[test]
fn without_a_vault_places_have_no_names() {
    let mut app = device(Vec::new());
    let w = one_key(&mut app);
    app.press(Action::Backup(w));
    app.press(Action::BQ(qstep::PLACES));
    let _ = app.frame();
    assert!(!app.offers(Action::BName(0)));
    app.press(Action::BName(0));
    type_text(&mut app, NAME);
    assert_eq!(app.place_name(0), "Place 1");
}

#[test]
fn the_blank_template_has_a_line_per_place_and_no_name() {
    let mut app = device(vec![vault_file()]);
    let w = one_key(&mut app);
    unlock(&mut app);
    app.press(Action::Backup(w));
    app.press(Action::BPreset(0));
    app.press(Action::BAnswer(qrow::PLACES, 3));
    app.press(Action::BName(1));
    type_text(&mut app, NAME);
    app.press(Action::BChecklist);
    app.press(Action::BOut(0));
    let pdf = &app
        .outbox
        .iter()
        .find(|i| i.name == "blank-template-24-words.pdf")
        .expect("the template")
        .bytes;
    let count = |s: &[u8]| pdf.windows(s.len()).filter(|w| *w == s).count();
    assert_eq!(count(b"(Place) Tj"), 3, "a line per place");
    assert_eq!(count(b"(holds) Tj"), 3);
    assert_eq!(count(NAME.as_bytes()), 0, "a place's name on the template");
}
