//! An address in Your choices (the pane, the phone's sheet) is drawn whole:
//! the small mono type, groups of four wrapped only between groups, the row
//! growing, every character there and nothing over the next row
//! (`docs/WORKSPACE.md` §18).

use faraday_core::testkit::{self, Plain};
use faraday_core::{Action, ChoicesPress, Faraday};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

const SIZES: [(u16, u16); 3] = [(1366, 768), (480, 640), (320, 480)];

/// A taproot-style address of `n` characters, every group of four unlike
/// the others.
fn address_of(n: usize) -> String {
    const SET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let mut out = String::from("bc1p");
    let mut k = 1usize;
    while out.len() < n {
        let v = k * 977 + 12345;
        for d in [v >> 15, v >> 10, v >> 5, v] {
            out.push(char::from(SET[d % 32]));
        }
        k += 1;
    }
    out.truncate(n);
    out
}

fn on(w: u16, h: u16, address: &str, keyboard: bool) -> Faraday {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: w,
        height: h,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.session = testkit::session();
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
    testkit::keep_published(&mut app);
    app.press(Action::SignMessage);
    testkit::message_on_address(&mut app, address);
    if keyboard {
        // A press on the message field brings the keyboard up.
        let _ = app.frame();
        let (x, y) = app
            .where_offered(Action::TypeHere(faraday_core::edit::Here::Message))
            .expect("the message field");
        for phase in [TouchPhase::Down, TouchPhase::Up] {
            app.event(Event::Touch { x, y, phase });
        }
        let _ = app.frame();
        assert!(
            app.keyboard_top().is_some(),
            "the tap brought the keyboard up"
        );
    }
    if w < 700 {
        app.press(Action::Choices(ChoicesPress::Open));
    }
    for _ in 0..30 {
        let _ = app.frame();
    }
    app
}

/// The groups of the address drawn, with their boxes, and the top of the
/// first text under them in the same column.
type Boxed = (String, (i32, i32, i32, i32));

fn pieces(app: &mut Faraday, whole: &str) -> (Vec<Boxed>, Option<i32>) {
    let _ = app.frame();
    let all = app.drawn_text_boxes().plain();
    // A group is four characters, or the address's short tail; a badge's
    // lone digit is not one.
    let is_group = |t: &str| {
        whole.contains(t) && (t.len() == 4 || (t.len() >= 2 && t.len() < 4 && whole.ends_with(t)))
    };
    let mut got: Vec<Boxed> = all
        .iter()
        .filter(|(t, _)| is_group(t))
        .map(|(t, b)| (t.clone(), (b.0, b.1, b.2, b.3)))
        .collect();
    // A line is the groups whose tops are within a few pixels.
    got.sort_by_key(|(_, b)| b.1);
    let mut line = 0;
    let mut keyed = Vec::new();
    for (k, (t, b)) in got.iter().enumerate() {
        if k > 0 && b.1 - got[k - 1].1.1 > 6 {
            line += 1;
        }
        keyed.push((line, b.0, t.clone(), *b));
    }
    keyed.sort_by_key(|k| (k.0, k.1));
    let got: Vec<_> = keyed.into_iter().map(|k| (k.2, k.3)).collect();
    let last = got.last().map_or(0, |(_, b)| b.1 + b.3);
    let below = all
        .iter()
        .filter(|(t, b)| !is_group(t) && b.1 >= last - 1)
        .map(|(_, b)| b.1)
        .min();
    (got, below)
}

#[test]
fn every_character_is_drawn_in_whole_groups_clear_of_the_next_row() {
    for (w, h) in SIZES {
        for n in [42, 90] {
            let a = address_of(n);
            let mut app = on(w, h, &a, false);
            let (got, below) = pieces(&mut app, &a);
            let joined: String = got.iter().map(|(t, _)| t.as_str()).collect();
            assert_eq!(joined, a, "{w}x{h}, {n} characters: every one drawn");
            for (t, _) in &got {
                assert!(
                    t.len() == 4 || a.ends_with(t.as_str()),
                    "{t}: a whole group"
                );
            }
            let bottom = got.iter().map(|(_, b)| b.1 + b.3).max().unwrap();
            if let Some(next) = below {
                assert!(
                    bottom <= next + 1,
                    "{w}x{h}: the next row starts at {next}, the address ends at {bottom}"
                );
            }
        }
    }
}

#[test]
fn with_the_keyboard_up_the_address_is_still_whole() {
    let a = address_of(90);
    let mut app = on(480, 640, &a, true);
    let (got, _) = pieces(&mut app, &a);
    let joined: String = got.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(joined, a);
    // Where the keyboard stays up with the sheet, the rows stand above it.
    let bottom = got.iter().map(|(_, b)| b.1 + b.3).max().unwrap();
    assert!(
        app.keyboard_top().is_none_or(|k| bottom <= k.into()),
        "above the keyboard"
    );
}
