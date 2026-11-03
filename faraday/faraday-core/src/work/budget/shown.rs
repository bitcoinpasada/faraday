//! A secret the person chose to show is worked out once, when the press
//! that shows it is made, and held until it is hidden (F195): the
//! backup's words with their SeedQR numbers, the copy page's SeedQR and
//! the BIP-85 child. The press costs what the work is, once; the frames
//! after it work out nothing; hiding it drops what was held.

use super::walk;
use crate::plan::{self, At, Tag, What};
use crate::work::{Counts, KINDS, PIXELS, Work, since, snapshot};
use crate::{Action, Faraday, bstep, testkit};
use faraday_shell_api::App;

/// The heavy session on the backup's first copy page, the seed hidden.
fn copy_page() -> Faraday {
    let c = testkit::compute();
    let (mut app, w) = walk(&c, &mut |_, _| {});
    app.press(Action::Backup(w));
    // The first seed's copy takes the SeedQR as well as the words.
    let shape = app.plan_shape_now(w);
    let answers = testkit::backup_answers_mut(&mut app);
    let mut map = plan::stored_map(&shape, answers);
    for spot in &mut map {
        if spot.at == At::Place(0) && spot.holds.iter().any(|(h, _)| *h == What::Words(0)) {
            spot.holds.push((What::SeedQr(0), Tag::Secret));
        }
    }
    answers.set_map(map);
    app.press(Action::BStep(bstep::COPY));
    assert_eq!(app.backup().and_then(|b| b.open), Some(bstep::COPY));
    app.settle();
    let _ = app.frame();
    app
}

/// A press, then a frame: what it works out.
fn press_work(app: &mut Faraday, a: Action) -> Counts {
    let before = snapshot();
    app.press(a);
    let _ = app.frame();
    since(&before)
}

/// Two frames drawn again, nothing changed: no counted work at all.
fn frames_work_out_nothing(app: &mut Faraday) {
    app.settle();
    let _ = app.frame();
    let before = snapshot();
    let _ = app.frame();
    let _ = app.frame();
    let again = since(&before);
    for k in KINDS {
        assert_eq!(again.of(k), 0, "the frames did {k:?} {}", again.of(k));
    }
}

/// `work` is the press's: only the kinds named, as often as named (the
/// fade and the frosted page aside).
fn costs(work: &Counts, named: &[(Work, u64)]) {
    for k in KINDS {
        if PIXELS.contains(&k) {
            continue;
        }
        let want = named.iter().find(|(n, _)| *n == k).map_or(0, |(_, n)| *n);
        assert_eq!(work.of(k), want, "the press did {k:?} {}", work.of(k));
    }
}

#[test]
fn the_backups_words_and_their_seedqr_are_worked_out_once_when_shown() {
    let mut app = copy_page();
    let work = press_work(&mut app, Action::BReveal);
    assert!(app.backup().is_some_and(|b| b.reveal));
    assert!(app.backup.shown_number(0).is_some(), "the numbers are held");
    assert!(app.backup.shown_qr(false).is_some(), "the SeedQR is held");
    costs(&work, &[(Work::WordsParse, 1), (Work::QrEncode, 1)]);
    frames_work_out_nothing(&mut app);
    // Compact is another grid: made once by its press.
    let work = press_work(&mut app, Action::BCompact(true));
    assert!(app.backup.shown_qr(true).is_some());
    costs(&work, &[(Work::WordsParse, 1), (Work::QrEncode, 1)]);
    frames_work_out_nothing(&mut app);
    // Hidden, it is gone.
    let _ = press_work(&mut app, Action::BReveal);
    assert!(!app.backup().is_some_and(|b| b.reveal));
    assert!(app.backup.shown_number(0).is_none());
    assert!(app.backup.shown_qr(true).is_none());
    assert!(!app.backup.shown_held());
}

#[test]
fn the_backups_seed_is_dropped_when_the_page_is_left() {
    let mut app = copy_page();
    let _ = press_work(&mut app, Action::BReveal);
    assert!(app.backup.shown_held());
    app.press(Action::Nav(crate::Screen::Home));
    assert!(!app.backup.shown_held());
}

#[test]
fn a_bip85_child_is_derived_once_when_shown() {
    let c = testkit::compute();
    let (mut app, _) = walk(&c, &mut |_, _| {});
    app.press(Action::Bip85);
    app.press(Action::PKey(app.session.keys[0].fingerprint.0));
    app.press(Action::PStep(crate::bip85::pstep::RESULT));
    app.settle();
    let _ = app.frame();
    assert!(app.bip85.held().is_none());
    let work = press_work(&mut app, Action::PShow);
    assert!(app.bip85().is_some_and(|b| b.shown));
    assert!(app.bip85.held().is_some(), "the child is held");
    costs(&work, &[(Work::KeyDerive, 1)]);
    frames_work_out_nothing(&mut app);
    let _ = press_work(&mut app, Action::PShow);
    assert!(app.bip85.held().is_none());
}
