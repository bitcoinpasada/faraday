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
    // One check: the check's inspection of each PSBT (2) and the Result's
    // of the Returned one against the wallets loaded (1); and no more
    // than the one check's parse of nothing, for both files were read
    // when they were chosen.
    assert_eq!(press.of(Work::PsbtInspect), 3, "one check");
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
