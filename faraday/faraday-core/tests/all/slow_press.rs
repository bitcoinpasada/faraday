//! A press that stretches a key or makes many pictures is drawn first and
//! does its work on the tick after (`docs/SPEED.md` §9): a person's tap
//! or Enter does none of that work itself, the frame after it shows the
//! press (and, for files or a node key, what is being made), and the tick
//! after that frame does the work. Every other press does no such work
//! at all, which the guard in every test holds (`Faraday::press_in`).

use faraday_core::testkit;
use faraday_core::work::{Counts, Work, since, snapshot};
use faraday_core::{Action, Faraday, Screen, qrow};
use faraday_shell_api::{App, Event, Key, TouchPhase};
use faraday_strings::EN;

/// A person's tap on `action`, the page scrolled until it is drawn: the
/// work the tap itself did.
fn tap(app: &mut Faraday, action: Action) -> Counts {
    let mut at = None;
    for _ in 0..80 {
        app.settle();
        let _ = app.frame();
        at = app.where_offered(action);
        if at.is_some() {
            break;
        }
        app.event(Event::Scroll {
            x: 900,
            y: 400,
            dy: 60,
        });
    }
    let (x, y) = at.unwrap_or_else(|| panic!("{action:?} is never on screen"));
    let before = snapshot();
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    since(&before)
}

/// The frame that shows what was pressed, then the tick after it: the
/// work the tick did.
fn frame_then_tick(app: &mut Faraday) -> Counts {
    let _ = app.frame();
    let before = snapshot();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    let done = since(&before);
    let _ = app.frame();
    done
}

/// Neither a key stretched nor more than one picture made.
fn light(c: &Counts) -> bool {
    c.of(Work::Kdf) == 0 && c.of(Work::Picture) <= 1
}

#[test]
fn the_checklists_pictures_are_made_after_the_frame_that_shows_the_press() {
    let c = faraday_core::testkit::compute();
    // A 2-of-15 whose descriptor needs a picture per part.
    let keys: Vec<String> = (0..5)
        .flat_map(|a| (0..3).map(move |n| testkit::key(n, &format!("m/48'/1'/{a}'/2'"))))
        .map(|k| format!("{k}/<0;1>/*"))
        .collect();
    let descriptor = format!("wsh(sortedmulti(2,{}))", keys.join(","));
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    let w = app
        .session
        .add_wallet(
            &c,
            "Big",
            &descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("a wallet");
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVaultSoftware);
    let answers = &app.backup.as_ref().unwrap().answers;
    if !answers.software[faraday_core::plan::software::NOT_SURE] {
        app.press(Action::BAnswer(
            qrow::SOFTWARE,
            faraday_core::plan::software::NOT_SURE as u8,
        ));
    }
    // The descriptor as a picture and as text.
    for row in [faraday_core::plan::form::QR, faraday_core::plan::form::TEXT] {
        if !app.backup.as_ref().unwrap().answers.form[row] {
            app.press(Action::BAnswer(qrow::FORM, row as u8));
        }
    }
    // Make the checklist is on The plan card: a person opens it first.
    if app
        .backup
        .as_ref()
        .is_some_and(|b| b.q != Some(faraday_core::qstep::PLAN))
    {
        app.press(Action::BQ(faraday_core::qstep::PLAN));
    }
    let outbox = app.outbox.len();
    let pressed = tap(&mut app, Action::BChecklist);
    assert!(light(&pressed), "the tap made the files: {pressed:?}");
    assert_eq!(app.outbox.len(), outbox, "nothing made before the frame");
    let _ = app.frame();
    assert_eq!(
        app.toast_text(),
        Some(EN.working_files),
        "the frame drawn first says the files are being made"
    );
    let done = frame_then_tick(&mut app);
    assert!(
        done.of(Work::Picture) > 1,
        "the tick after made the pictures: {done:?}"
    );
    assert!(
        app.outbox.len() > outbox,
        "the files are For the stick: {:?}",
        app.outbox
            .iter()
            .map(|i| i.name.clone())
            .collect::<Vec<_>>()
    );
    assert_ne!(
        app.toast_text(),
        Some(EN.working_files),
        "the working toast goes once they are made"
    );
}

#[test]
fn a_key_added_by_a_tap_is_stretched_after_the_frame_that_shows_the_press() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let pressed = tap(&mut app, Action::EntryAdd);
    assert!(light(&pressed), "the tap stretched the key: {pressed:?}");
    assert!(
        app.session.keys.is_empty(),
        "nothing loaded before the frame"
    );
    let done = frame_then_tick(&mut app);
    assert!(
        done.of(Work::Kdf) > 0,
        "the tick after stretched it: {done:?}"
    );
    assert_eq!(app.session.keys.len(), 1, "the key is loaded");
}

#[test]
fn an_aezeed_is_stretched_after_the_frame_that_shows_enter() {
    const CIPHER_SEED: &str = "above judge emerge veteran reform crunch system all snap please \
        shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
    let mut app = testkit::started();
    app.press(Action::Lightning);
    assert_eq!(app.screen, Screen::Lightning);
    app.press(Action::LAezeed);
    for c in CIPHER_SEED
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
    {
        app.event(Event::Key(Key::Char(c)));
    }
    let before = snapshot();
    app.event(Event::Key(Key::Enter));
    let pressed = since(&before);
    assert!(light(&pressed), "Enter stretched the aezeed: {pressed:?}");
    assert!(app.lightning.as_ref().unwrap().node.is_none());
    let _ = app.frame();
    assert_eq!(
        app.toast_text(),
        Some(EN.working_node_key),
        "the frame drawn first says so"
    );
    let done = frame_then_tick(&mut app);
    assert!(
        done.of(Work::Kdf) > 0,
        "the tick after stretched it: {done:?}"
    );
    assert!(
        app.lightning.as_ref().unwrap().node.is_some(),
        "the node key"
    );
}
