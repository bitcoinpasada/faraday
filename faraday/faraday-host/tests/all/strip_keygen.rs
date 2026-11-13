//! The strip's side of the keygen tests: what Faraday Demo's host offers there.

use faraday_core::keygen::Start;
use faraday_core::keygen::Way;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn added(app: &Faraday) -> Vec<[u8; 4]> {
    app.session.keys.all().map(|k| k.fingerprint.0).collect()
}

/// Make a new wallet, single key or a 2-of-3 multisig, with New key opened
/// for its first slot and carried to the entry card on coin flips.
fn slot_flow(multi: bool) -> Faraday {
    use faraday_core::create::NewKind;
    let mut app = faraday_core::testkit::started();
    tall(&mut app);
    app.press(Action::CreateWallet);
    let kind = if multi {
        NewKind::Multi
    } else {
        NewKind::NativeSegwit
    };
    let at = NewKind::ALL.iter().position(|k| *k == kind).unwrap() as u8;
    crate::common::ckind(&mut app, at);
    app.press(Action::CNext(faraday_core::cstep::KIND));
    if multi {
        app.press(Action::CNext(faraday_core::cstep::QUORUM));
    }
    app.press(Action::KeyGen(Some(0)));
    // Start: Make a new key pressed opens Length.
    app.press(Action::KStart(Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    app
}

/// Flips enough for twelve words.
fn flip_all(app: &mut Faraday) {
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
}

fn texts(app: &mut Faraday) -> String {
    app.drawn_texts().join(" ")
}

/// A tall display, so a whole card is drawn at once.
fn tall(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// The demo on a Create slot (F20, `docs/MODES.md` §7): the passphrase is
/// `faraday` once Use faraday is pressed, in both fields; Lock in adds the next
/// sample with it, saying so before and after.
#[test]
fn the_demo_locks_in_its_sample_with_faraday_when_a_passphrase_is_chosen() {
    use faraday_core::wallet::fp_text;
    let c = faraday_core::testkit::compute();
    let mut app = slot_flow(true);
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.session.set_demo(faraday_core::testkit::demo_samples());
    flip_all(&mut app);
    app.press(Action::KNext);
    let n = app.keygen().unwrap().handout_words();
    let (word, fp) = app.session.keys.next_handout(n).unwrap();
    assert!(crate::strips::strip(&mut app).contains(&format!(
        "Lock in adds sample {word} ×{n} ({}) in place of these words",
        &*fp_text(fp)
    )));
    let chip = Action::PassChip(faraday_core::demo::PassField::KeyGen(0));
    assert!(
        crate::strips::strip_offers(&mut app, chip),
        "the pair offers faraday"
    );
    app.press(chip);
    let k = app.keygen().unwrap();
    assert_eq!(k.passphrase.as_str(), "faraday");
    assert_eq!(k.passphrase2.as_str(), "faraday");
    app.press(Action::KLock);
    let with = faraday_core::wallet::Session::default()
        .keys
        .add_words_with(
            &c,
            &faraday_core::samples::repeated(word, n),
            "faraday",
            "",
            None,
        )
        .unwrap()
        .0;
    assert_eq!(added(&app), vec![with]);
    let c = app.create().unwrap();
    assert_eq!(c.slots[0], faraday_core::create::Source::Here(with));
    // Lock in went back to Create, which shows the sample's key.
    assert_eq!(app.screen, Screen::Create);
    let shown = texts(&mut app);
    assert!(shown.contains(&*fp_text(faraday_bip::keys::Fingerprint(with))));
}
