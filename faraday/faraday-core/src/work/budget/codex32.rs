//! Make codex32 shares' entry does per press only the work of the
//! columns its character completes (`docs/CODEX32.md` §13.2), on a
//! desktop and on the Pi's panel: a pad press that completes a character
//! works one column of the share's checksum worksheet and one of each
//! translation worksheet; the press that completes the last box of the
//! set works the checksums' columns, the set, each share made's checksum
//! check (the tour's, §5.7), its key's fingerprint and
//! the first string's Codex32QR code; the frames after either, and a
//! cell's working opened, work out nothing.

use faraday_shell_api::{App, EntropyBytes, Event};

use super::{DESKTOP, PI, Panel, show};
use crate::codex32::{C32, Cell, OskPress, SPLITS, Split, WAYS, Way};
use crate::keygen::Start;
use crate::work::{Counts, KINDS, Work, since, snapshot};
use crate::{Action, Faraday, Screen};

/// A press and the frames after it.
fn press_and_draw(app: &mut Faraday, p: C32) -> Counts {
    let before = snapshot();
    app.press(Action::Codex32(p));
    for _ in 0..3 {
        let _ = app.frame();
    }
    since(&before)
}

/// The frames alone, nothing pressed.
fn frames(app: &mut Faraday) -> Counts {
    let before = snapshot();
    for _ in 0..3 {
        let _ = app.frame();
    }
    since(&before)
}

/// Every kind but those listed is none; those listed at most their count.
fn within(what: &str, work: &Counts, table: &[(Work, u64)]) {
    for k in KINDS {
        let most = table.iter().find(|t| t.0 == k).map_or(0, |t| t.1);
        assert!(
            work.of(k) <= most,
            "{what}: {k:?} {} (at most {most})",
            work.of(k)
        );
    }
}

/// The ten faces of a character rolled 5 then 2 on every die (bit 0).
const Q: [char; 10] = ['5', '5', '5', '5', '5', '2', '2', '2', '2', '2'];

fn entry_on(panel: Panel) -> Faraday {
    let mut app = Faraday::new();
    show(&mut app, panel);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStep(crate::keygen::kstep::START));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let two = SPLITS
        .iter()
        .position(|s| *s == Split::TwoOfThree)
        .unwrap_or(0);
    app.press(Action::Codex32(C32::Split(two as u8)));
    app.press(Action::Codex32(C32::Next));
    for ch in "test".chars() {
        app.press(Action::Codex32(C32::Key(OskPress::Char(ch))));
    }
    app.press(Action::Codex32(C32::Next));
    let dice = WAYS.iter().position(|w| *w == Way::Dice).unwrap_or(0);
    app.press(Action::Codex32(C32::Way(dice as u8)));
    app.press(Action::Codex32(C32::Next));
    let _ = app.frame();
    app
}

fn the_entry_works_a_column_a_press_on(panel: Panel) {
    let mut app = entry_on(panel);
    // Two translation worksheets at 2 of 3: share D and the secret.
    let column = 1 + 2;
    for (n, &f) in Q.iter().enumerate() {
        let work = press_and_draw(&mut app, C32::Key(OskPress::Char(f)));
        let most = if n + 1 == Q.len() { column } else { 0 };
        within("a face", &work, &[(Work::Codex32, most)]);
    }
    for _ in 1..26 + 25 {
        for f in Q {
            app.press(Action::Codex32(C32::Key(OskPress::Char(f))));
        }
    }
    for &f in &Q[..9] {
        app.press(Action::Codex32(C32::Key(OskPress::Char(f))));
    }
    let _ = app.frame();
    // What one string's checksum check notes: share D's, worked once for
    // the tour of the shares made (`docs/CODEX32.md` §5.7).
    let check = {
        let c = crate::testkit::compute();
        let before = snapshot();
        let d = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
        assert!(faraday_keys::StringCheck::of(&c, d).is_some());
        since(&before).of(Work::Codex32)
    };
    // Box 35 of share C: its column, its 13 checksum columns in its
    // worksheet and in both translation worksheets, the checksum itself
    // and the set, and share D's checksum check; the key's fingerprint;
    // the first string's code.
    let last = press_and_draw(&mut app, C32::Key(OskPress::Char(Q[9])));
    within(
        "the set's last character",
        &last,
        &[
            (Work::Codex32, column * 14 + 2 + check),
            (Work::MasterKey, 1),
            (Work::QrEncode, 1),
        ],
    );
    let m = app.keygen().and_then(|k| k.codex32.as_ref());
    assert!(m.is_some_and(|m| m.strings().is_some()), "the set is made");
    within("frames after", &frames(&mut app), &[]);
    // The Working area's Next, through the last checksum box and on into
    // the tour of the shares made: each step and its frames, nothing.
    let toured = |app: &Faraday| {
        app.keygen()
            .and_then(|k| k.codex32.as_ref())
            .is_some_and(|m| m.tour_shown().is_some_and(|s| s.0 > 3))
    };
    for _ in 0..80 {
        if toured(&app) {
            break;
        }
        within("a step", &press_and_draw(&mut app, C32::Step(1)), &[]);
    }
    assert!(toured(&app), "the tour is reached");
    within(
        "a tour step back",
        &press_and_draw(&mut app, C32::Step(-1)),
        &[],
    );
    let open = press_and_draw(
        &mut app,
        C32::Cell(Cell::Sum {
            sheet: 1,
            j: 16,
            i: 3,
        }),
    );
    within("a cell's working opened", &open, &[]);
}

#[test]
fn the_codex32_entry_works_a_column_a_press() {
    the_entry_works_a_column_a_press_on(DESKTOP);
}

#[test]
fn on_the_pis_panel_the_codex32_entry_works_a_column_a_press() {
    the_entry_works_a_column_a_press_on(PI);
}
