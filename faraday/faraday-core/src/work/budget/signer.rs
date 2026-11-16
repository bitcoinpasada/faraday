//! Check a signer does its costly work on the press that completes the
//! second card and nowhere else (`docs/SIGNER-CHECK.md` §8.2): the press
//! counts one check, and drawing the Result card again, a pointer move
//! and a scroll work out nothing.

use crate::work::{Work, since, snapshot};
use crate::{
    Action, Faraday, StorageEvent, decode::check::SignerPress, decode::check::Slot, testkit,
};
use faraday_shell_api::{App, Event};

fn vector(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../tools/vectors/psbt/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Files holds two PSBTs of one transaction; the flow is open on its
/// Sent card with nothing chosen yet.
fn open_unchosen() -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: crate::files_from(vec![
            (
                faraday_text::Text::of("sent.psbt"),
                vector("signer-core-sent.psbt"),
            ),
            (
                faraday_text::Text::of("signed.psbt"),
                vector("signer-core-signed.psbt"),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let tile = crate::catalog::TILES
        .iter()
        .position(|t| t.go == crate::catalog::Go::SignerCheck)
        .expect("the tile") as u8;
    app.press(Action::Nav(crate::Screen::Catalog));
    app.press(Action::Catalog(tile));
    // The default pair was chosen; choose again from the start.
    app.press(Action::SignerCheck(SignerPress::Card(0)));
    let _ = app.frame();
    app
}

#[test]
fn the_press_completing_the_second_card_counts_one_check_and_drawing_counts_none() {
    let mut app = open_unchosen();
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    let _ = app.frame();
    let before = snapshot();
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
    let press = since(&before);
    // One check: the check's inspection of each PSBT (2) and the
    // comparison's of each against the wallets loaded (2, the Returned
    // one also what the facts read); and no more than the one check's
    // parse of nothing, for both files were read when they were chosen.
    assert_eq!(press.of(Work::PsbtInspect), 4, "one check");
    assert_eq!(press.of(Work::Sign), 0);
    assert!(app.signer_check_kept());

    // The Result drawn again, a pointer move and a scroll: nothing.
    let _ = app.frame();
    let before = snapshot();
    let _ = app.frame();
    app.event(Event::Hover { x: 40, y: 200 });
    let _ = app.frame();
    app.event(Event::Wheel {
        x: 40,
        y: 200,
        dy: 48,
    });
    let _ = app.frame();
    app.event(Event::Wheel {
        x: 40,
        y: 200,
        dy: -48,
    });
    let _ = app.frame();
    let drawn = since(&before);
    for k in crate::work::KINDS {
        if crate::work::PIXELS.contains(&k) {
            continue;
        }
        assert_eq!(drawn.of(k), 0, "{k:?} while drawing the Result");
    }
}

#[test]
fn the_press_loading_a_seed_counts_one_check() {
    use faraday_shell_api::Key;
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: crate::files_from(vec![
            (
                faraday_text::Text::of("sent.psbt"),
                vector("demo-regtest.psbt"),
            ),
            (
                faraday_text::Text::of("random.psbt"),
                vector("signer-random-nonce.psbt"),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let tile = crate::catalog::TILES
        .iter()
        .position(|t| t.go == crate::catalog::Go::SignerCheck)
        .expect("the tile") as u8;
    app.press(Action::Nav(crate::Screen::Catalog));
    app.press(Action::Catalog(tile));
    app.press(Action::SignerCheck(SignerPress::Seed(0)));
    app.press(Action::SignerCheck(SignerPress::SeedWords));
    let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                 abandon abandon about";
    for c in words.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let _ = app.frame();
    let before = snapshot();
    app.press(Action::EntryAdd);
    let press = since(&before);
    assert_eq!(press.of(Work::PsbtInspect), 4, "one check");
    assert_eq!(app.session.keys.len(), 1);
}

mod compare {
    //! The two files' comparison does its costly work on the press that
    //! completes the second card and nowhere else (`docs/COMPARE.md` §6.2):
    //! the press counts its inspections, and drawing the Result card again, a pointer move and a
    //! scroll work out nothing.

    use crate::decode::check::{SignerPress, Slot};
    use crate::work::{Work, since, snapshot};
    use crate::{Action, Faraday, StorageEvent, testkit};
    use faraday_shell_api::{App, Event};

    fn vector(name: &str) -> Vec<u8> {
        let path = format!(
            "{}/../../tools/vectors/psbt/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    /// Files holds the Core pair; the flow is open on its First card with
    /// nothing chosen yet.
    fn open_unchosen() -> Faraday {
        let mut app = testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: crate::files_from(vec![
                (
                    faraday_text::Text::of("original.psbt"),
                    vector("bump-core-original.psbt"),
                ),
                (
                    faraday_text::Text::of("replacement.psbt"),
                    vector("bump-core-replacement.psbt"),
                ),
            ]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        let tile = crate::catalog::TILES
            .iter()
            .position(|t| t.go == crate::catalog::Go::SignerCheck)
            .expect("the tile") as u8;
        app.press(Action::Nav(crate::Screen::Catalog));
        app.press(Action::Catalog(tile));
        let _ = app.frame();
        app
    }

    #[test]
    fn the_press_completing_the_second_card_counts_its_inspections_and_drawing_counts_none() {
        let mut app = open_unchosen();
        app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
        let _ = app.frame();
        let before = snapshot();
        app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
        let press = since(&before);
        assert!(press.of(Work::PsbtInspect) > 0, "the pair is inspected");
        assert_eq!(press.of(Work::Sign), 0);
        assert!(app.signer_check().is_some_and(|s| s.shown.is_some()));

        // The Result drawn again, a pointer move and a scroll: nothing.
        let _ = app.frame();
        let before = snapshot();
        let _ = app.frame();
        app.event(Event::Hover { x: 40, y: 200 });
        let _ = app.frame();
        app.event(Event::Wheel {
            x: 40,
            y: 200,
            dy: 48,
        });
        let _ = app.frame();
        app.event(Event::Wheel {
            x: 40,
            y: 200,
            dy: -48,
        });
        let _ = app.frame();
        let drawn = since(&before);
        for k in crate::work::KINDS {
            if crate::work::PIXELS.contains(&k) {
                continue;
            }
            assert_eq!(drawn.of(k), 0, "{k:?} while drawing the Result");
        }
    }
}
