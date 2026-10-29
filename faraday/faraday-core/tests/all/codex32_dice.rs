//! The codex32 dice (`docs/CODEX32.md` §5.1, §5.5, §6.4.2): dice of any
//! number of faces up to twenty, each die's colour, both rolls side by
//! side, and the booklet's sheet with its tracks, as a person rolling a
//! set's characters on New key would notice them. Rolled on twenty-faced
//! dice, a set is BIP 93's vector 2 as on six-faced ones.

use faraday_core::codex32::WalkAct;
use faraday_core::codex32::{C32, Card, DIE_SIZES, DieColour, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::Start;
use faraday_core::testkit::five::Reach;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_ui::keyboard::OskPress;

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Vector 2's set in index order, lowercase
/// (`tools/vectors/bip93/vectors.json`).
const VECTOR2: [&str; 3] = [
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
    "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg",
];

fn on(width: u16, height: u16) -> Faraday {
    on_at(width, height, 160)
}

/// The app on a display of `width` × `height` pixels at `dpi`.
fn on_at(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    app
}

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
}

fn key(app: &mut Faraday, ch: char) {
    c32(app, C32::Key(OskPress::Char(ch)));
}

fn make(app: &Faraday) -> &faraday_core::codex32::Codex32Make {
    app.keygen().unwrap().codex32.as_ref().unwrap()
}

/// The die of `faces` faces, as the Entropy card offers it.
fn die(faces: u8) -> C32 {
    C32::Faces(DIE_SIZES.iter().position(|&n| n == faces).unwrap() as u8)
}

/// New key, Codex32, 2 of 3, `name`, Roll dice on dice of `faces` faces:
/// the entry open.
fn to_entry(app: &mut Faraday, faces: u8) {
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(app, C32::Split(split as u8));
    c32(app, C32::Next);
    for ch in "name".chars() {
        key(app, ch);
    }
    c32(app, C32::Next);
    let w = WAYS.iter().position(|w| *w == Way::Dice).unwrap();
    c32(app, C32::Way(w as u8));
    c32(app, die(faces));
    c32(app, C32::Next);
    assert_eq!(make(app).open(), Some(Card::Enter));
}

/// The ten faces that roll character `ch`: each die's bit 0 as `hi` then
/// `lo`, 1 as `lo` then `hi`.
fn faces(ch: u8, lo: u8, hi: u8) -> [u8; 10] {
    let v = CHARSET.iter().position(|&x| x == ch).unwrap() as u8;
    let mut out = [0u8; 10];
    for d in 0..5 {
        let bit = v >> (4 - d) & 1;
        (out[d], out[5 + d]) = if bit == 0 { (hi, lo) } else { (lo, hi) };
    }
    out
}

fn strings(app: &Faraday) -> Vec<String> {
    make(app)
        .strings()
        .map(|s| s.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn twenty_faced_dice_make_vector_2s_strings() {
    let mut app = on(1366, 768);
    to_entry(&mut app, 20);
    assert_eq!(make(&app).dice_choice().faces, 20);
    for s in &VECTOR2[..2] {
        for &ch in &s.as_bytes()[9..35] {
            // Faces past six, pressed on the pad's keys.
            for f in faces(ch, 7, 19) {
                c32(&mut app, C32::Face(f));
            }
        }
    }
    let _ = app.frame();
    assert_eq!(strings(&app), VECTOR2);
}

#[test]
fn every_die_size_reads_the_same_character() {
    for &n in &DIE_SIZES {
        let mut app = on(1366, 768);
        to_entry(&mut app, n);
        for f in faces(b'k', 1, n) {
            c32(&mut app, C32::Face(f));
        }
        let m = make(&app);
        assert_eq!(
            m.made_of(0, 0).map(|c| c.value()),
            Some(22),
            "d{n}: k is 22"
        );
        assert_eq!(m.made_of(0, 0).map(|c| c.faces()), Some(n));
    }
}

#[test]
fn on_the_keyboard_a_face_of_two_digits_is_typed_as_two_digits() {
    // Tall enough for the sheets and the pad in view.
    let mut app = on(1366, 4000);
    to_entry(&mut app, 20);
    // 1 7 is seventeen; 1 then Space is one; 2 5 is two, then five; 9
    // is nine at once, as no face of a twenty-faced die starts with it.
    for ch in ['1', '7', '1', ' ', '2', '5', '9'] {
        key(&mut app, ch);
    }
    let d = make(&app).dice();
    let first: Vec<Option<u8>> = (0..5).map(|i| d.first(i)).collect();
    assert_eq!(first, [Some(17), Some(1), Some(2), Some(5), Some(9)]);
    // A tens digit waits, shown; Backspace drops it and takes back
    // nothing else; Enter takes it as a face of one digit.
    key(&mut app, '2');
    assert_eq!(make(&app).pending(), Some(2));
    assert!(
        app.drawn_texts().iter().any(|t| t == "Face 2…"),
        "the waiting digit is shown"
    );
    c32(&mut app, C32::Key(OskPress::Back));
    assert_eq!(make(&app).pending(), None);
    assert_eq!(make(&app).dice().len(), 5);
    key(&mut app, '1');
    c32(&mut app, C32::Key(OskPress::Done));
    assert_eq!(make(&app).dice().second(0), Some(1));
    assert_eq!(make(&app).open(), Some(Card::Enter), "Enter took the digit");
}

#[test]
fn faces_past_the_dice_are_refused() {
    let mut app = on(1366, 768);
    to_entry(&mut app, 8);
    key(&mut app, '9');
    key(&mut app, '0');
    c32(&mut app, C32::Face(9));
    assert!(make(&app).dice().is_empty());
    key(&mut app, '8');
    assert_eq!(make(&app).dice().first(0), Some(8));
}

#[test]
fn the_dice_change_between_characters_and_come_back_with_their_rolls() {
    let mut app = on(1366, 768);
    to_entry(&mut app, 20);
    // Other dice in the middle of a character: its rolls are cleared,
    // and the note says so.
    c32(&mut app, C32::Face(17));
    c32(&mut app, C32::Face(19));
    c32(&mut app, die(6));
    assert_eq!(make(&app).dice_choice().faces, 6);
    assert!(make(&app).dice().is_empty());
    assert_eq!(
        make(&app).note(),
        Some("Dice changed: this character's rolls are cleared")
    );
    c32(&mut app, die(20));
    for &f in &faces(b'q', 3, 17) {
        c32(&mut app, C32::Face(f));
    }
    assert!(make(&app).made_of(0, 0).is_some());
    c32(&mut app, die(6));
    assert_eq!(make(&app).dice_choice().faces, 6);
    for ch in "4444433333".chars() {
        key(&mut app, ch);
    }
    assert!(make(&app).made_of(0, 1).is_some());
    // Backspace through the six-faced character into the twenty-faced
    // one brings its rolls back on its own dice.
    for _ in 0..11 {
        c32(&mut app, C32::Key(OskPress::Back));
    }
    let m = make(&app);
    assert_eq!(m.dice_choice().faces, 20);
    assert_eq!(m.dice().len(), 9);
    assert_eq!(m.dice().first(0), Some(17));
}

/// The app on Faraday Demo (the roller offered) or the stick, Reduce
/// motion on, on the entry by dice of `faces` faces.
fn desktop(offered: bool, faces: u8) -> Faraday {
    let mut app = on(1366, 4000);
    app.set_roller_offered(offered);
    if offered {
        // The roller is Practice's alone (`docs/DEMO.md` §14).
        faraday_core::testkit::into_demo(&mut app);
    }
    app.press(Action::ReduceMotion(true));
    to_entry(&mut app, faces);
    app
}

#[test]
fn the_computers_dice_are_offered_in_the_host_strip_on_faraday_desktop_alone() {
    let mut app = desktop(true, 6);
    // The person's own entry in the app; the computer's walk outside it.
    assert!(app.drawn_texts().iter().any(|t| t == "Your dice"));
    assert!(!app.drawn_texts().iter().any(|t| t == "Demo only"));
    let s = crate::common::strip(&mut app);
    for want in ["Rolls d6", "Previous", "Next", "Play"] {
        assert!(s.iter().any(|t| t == want), "{want}: {s:?}");
    }
    let mut stick = desktop(false, 6);
    let s = crate::common::strip(&mut stick);
    assert!(!s.iter().any(|t| t == "Next" || t == "Rolls d6"), "{s:?}");
}

#[test]
fn twenty_faced_dice_are_rolled_and_padded_one_to_twenty() {
    let mut app = desktop(true, 20);
    assert!(
        crate::common::strip(&mut app)
            .iter()
            .any(|t| t == "Rolls d20")
    );
    // The pad's keys 1 to 20.
    for n in 1..=20u8 {
        assert!(app.offers(Action::Codex32(C32::Face(n))), "key {n}");
    }
    assert!(!app.offers(Action::Codex32(C32::Face(21))));
    // A whole share rolled: every face 1 to 20, some past six.
    c32(&mut app, C32::Walk(WalkAct::Reach(Reach::Share)));
    c32(&mut app, C32::Walk(WalkAct::Speed(3)));
    c32(&mut app, C32::Walk(WalkAct::Play));
    let mut t = app.now_ms();
    for i in 0..4000u32 {
        let b = (i as u8).wrapping_mul(53).wrapping_add(7);
        app.event(Event::Entropy(EntropyBytes::new([b; 32])));
        t += 16;
        app.event(Event::Tick { now_ms: t });
        if !make(&app).walk().playing() {
            break;
        }
    }
    let m = make(&app);
    let mut seen = Vec::new();
    for n in 0..26 {
        let made = m.made_of(0, n).expect("the share is rolled");
        assert_eq!(made.faces(), 20);
        for r in made.rows() {
            seen.push(r.first);
            seen.push(r.second);
        }
    }
    assert!(seen.iter().all(|f| (1..=20).contains(f)));
    assert!(seen.iter().any(|&f| f > 6), "faces past six come up");
}

#[test]
fn each_die_takes_a_named_colour_and_the_dice_are_kept_for_the_session() {
    let mut app = on(1366, 768);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    c32(&mut app, C32::Next);
    for ch in "name".chars() {
        key(&mut app, ch);
    }
    c32(&mut app, C32::Next);
    let texts = app.drawn_texts();
    // Five distinct colours to start, each named with its die.
    for want in [
        "Die 1 · Red",
        "Die 2 · Blue",
        "Die 3 · Green",
        "Die 4 · Yellow",
        "Die 5 · Purple",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want}");
    }
    assert!(app.offers(Action::Codex32(C32::Colour(2))));
    c32(&mut app, C32::Colour(2));
    c32(&mut app, die(12));
    assert!(app.drawn_texts().iter().any(|t| t == "Die 3 · Blue"));
    let chosen = make(&app).dice_choice();
    assert_eq!(chosen.colours[2], DieColour::Blue);
    // A new Make codex32 shares this session starts on them.
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    assert_eq!(make(&app).dice_choice(), chosen);
    assert_eq!(make(&app).dice_choice().faces, 12);
}

#[test]
fn the_sheet_shows_both_rolls_the_tracks_and_a_struck_pair() {
    let mut app = on(1366, 2400);
    to_entry(&mut app, 20);
    for f in [17, 3, 20, 9, 12, 4, 15, 20] {
        c32(&mut app, C32::Face(f));
    }
    let texts = app.drawn_texts();
    for want in [
        "Die 1",
        "lower · 0",
        "higher · 1",
        "Die tracks",
        "Die pads",
        "Die pads (d7+)",
        "Free space",
        "Die 3 · 20 then 20 · equal · roll both again",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want}");
    }
    // Every pad of the booklet's track, 1 to 20, on each of the five.
    for n in 1..=20 {
        let n = n.to_string();
        assert!(texts.iter().filter(|t| **t == n).count() >= 5, "pad {n}");
    }
}

#[test]
fn a_ten_faced_die_numbered_0_to_9_enters_its_0_as_10() {
    let mut app = on(1366, 4000);
    to_entry(&mut app, 10);
    assert!(app.drawn_texts().iter().any(|t| t == "0 is 10"));
    key(&mut app, '0');
    key(&mut app, '7');
    let d = make(&app).dice();
    assert_eq!((d.first(0), d.first(1)), (Some(10), Some(7)));
}

/// Faraday at `width` × `height` pixels and `dpi`.
fn on_dpi(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = on(width, height);
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// WCAG's relative luminance of an sRGB colour.
fn luminance([r, g, b]: [u8; 3]) -> f64 {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// WCAG's contrast ratio of two colours.
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// The frame's pixel at (x, y).
fn px(rgba: &[u8], width: usize, (x, y): (i32, i32)) -> [u8; 3] {
    let i = (y as usize * width + x as usize) * 4;
    [rgba[i], rgba[i + 1], rgba[i + 2]]
}

/// Of the pixels within `r` of (x, y), the one farthest in contrast from
/// `bg`: the middle of a thin line, whatever its smoothing.
fn inkiest(rgba: &[u8], width: usize, (x, y): (i32, i32), r: i32, bg: [u8; 3]) -> [u8; 3] {
    let mut best = bg;
    for dy in -r..=r {
        for dx in -r..=r {
            let p = px(rgba, width, (x + dx, y + dy));
            if contrast(p, bg) > contrast(best, bg) {
                best = p;
            }
        }
    }
    best
}

/// A drawn box: x, y, width, height in pixels.
type Box4 = (i32, i32, i32, i32);

/// The drawn box of the tree's leaf `s`, on the row of leaves Q, P, Z.
fn leaf_box(boxes: &[(String, Box4)], s: &str) -> Box4 {
    // Letters' ink starts at their own heights: a row is within a
    // letter's height.
    let near = |a: &Box4, y: i32| (a.1 - y).abs() <= a.3 / 2;
    let on_row = |t: &str, y: i32| boxes.iter().any(|(u, b)| u == t && near(b, y));
    let row = boxes
        .iter()
        .find(|(t, b)| t == "Q" && on_row("P", b.1) && on_row("Z", b.1))
        .expect("the tree's leaves")
        .1
        .1;
    boxes
        .iter()
        .find(|(t, b)| t == s && near(b, row))
        .unwrap_or_else(|| panic!("leaf {s}"))
        .1
}

#[test]
fn the_trees_path_stands_apart_from_its_lines_and_the_sheet_in_every_theme() {
    use faraday_core::ui::Theme;
    // Two pixels to the dp, a tall window: the whole tree above the panel.
    let mut app = on_dpi(2560, 2600, 320);
    to_entry(&mut app, 6);
    // Each die's first roll, then die 1's second: 1 then 2, higher, so
    // the path's first branch goes right.
    for f in [1, 1, 1, 1, 1, 2] {
        c32(&mut app, C32::Face(f));
    }
    for theme in Theme::all() {
        app.press(Action::Theme(theme));
        let boxes = app.drawn_text_boxes();
        let (q, l) = (leaf_box(&boxes, "Q"), leaf_box(&boxes, "L"));
        let frame = app.frame();
        let (w, rgba) = (usize::from(frame.width), frame.rgba.to_vec());
        let (qx, lx) = (q.0 + q.2 / 2, l.0 + l.2 / 2);
        let lw = f64::from(lx - qx) / 31.0;
        // A point `leaves` leaf widths from the tree's left end.
        let at = |leaves: f64| qx + (lw * (leaves - 0.5)).round() as i32;
        // Just over the leaves, between two of them: the sheet.
        let bg = px(&rgba, w, (at(16.0), q.1 - q.3));
        // Up a column from the leaves to its first ink. Between the
        // middle two leaves it is the root; a quarter and three quarters
        // across, the ends of die 1's two branches: no other line crosses
        // those columns.
        let first_ink = |x: i32| {
            (q.1 - 1200..q.1 - q.3)
                .rev()
                .find(|&y| contrast(px(&rgba, w, (x, y)), bg) > 1.5)
                .expect("ink up the column")
        };
        let root = (at(16.0), first_ink(at(16.0)));
        let right = (at(24.0), first_ink(at(24.0)));
        let left = (at(8.0), first_ink(at(8.0)));
        let mid = |a: (i32, i32), b: (i32, i32)| ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
        let path = inkiest(&rgba, w, mid(root, right), 2, bg);
        let line = inkiest(&rgba, w, mid(root, left), 2, bg);
        let sheet = px(&rgba, w, (root.0, (root.1 + right.1) / 2));
        let name = format!("{theme:?}");
        assert!(
            contrast(path, line) >= 3.0,
            "{name}: path {path:?} on line {line:?}"
        );
        assert!(
            contrast(path, sheet) >= 3.0,
            "{name}: path {path:?} on sheet {sheet:?}"
        );
        assert_ne!(line, sheet, "{name}: the other branch is drawn");
    }
}

/// The leaves standing out of the sheet: each leaf whose letter has a
/// fill round it, by its letter.
fn filled_leaves(app: &mut Faraday) -> Vec<String> {
    const LEAVES: &str = "QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L";
    let boxes = app.drawn_text_boxes();
    let frame = app.frame();
    let (w, rgba) = (usize::from(frame.width), frame.rgba.to_vec());
    let beside = |s: &str| {
        let b = leaf_box(&boxes, s);
        px(&rgba, w, (b.0 - 1, b.1 + b.3 / 2))
    };
    let leaves: Vec<String> = LEAVES.chars().map(|c| c.to_string()).collect();
    // The sheet: what stands beside most of the leaves.
    let sheet = leaves
        .iter()
        .map(|s| beside(s))
        .max_by_key(|c| leaves.iter().filter(|s| beside(s) == *c).count())
        .expect("leaves");
    leaves
        .into_iter()
        .filter(|s| contrast(beside(s), sheet) >= 3.0)
        .collect()
}

#[test]
fn the_leaf_the_step_through_reaches_is_filled_in_the_paths_colour() {
    let mut app = on_dpi(2560, 2600, 320);
    app.set_roller_offered(true);
    faraday_core::testkit::into_demo(&mut app);
    to_entry(&mut app, 6);
    assert!(filled_leaves(&mut app).is_empty(), "no leaf reached yet");
    // Next to the character's own step: its five bits read down the tree.
    let shown = |app: &Faraday| make(app).walk().shown().map(|(_, _, s)| s);
    for _ in 0..40 {
        c32(&mut app, C32::Walk(WalkAct::Next));
        if shown(&app) == Some(faraday_core::codex32::Step::Char) {
            break;
        }
    }
    assert_eq!(shown(&app), Some(faraday_core::codex32::Step::Char));
    assert_eq!(filled_leaves(&mut app).len(), 1, "the one leaf reached");
}

/// While a share's characters are rolled, the top app bar's title is the
/// step, "Share A · character 2 of 26", and it stays in view as the page
/// scrolls under the bar; the open card reads Characters, so the step is
/// drawn once (F146). On a phone-sized window, a wide one and the Pi's
/// 2.8-inch panel (480 × 640 pixels at 286 dpi), where the bar has room
/// for its short form, "Share A · 2 of 26", and shows it whole.
#[test]
fn the_step_being_rolled_is_the_top_app_bars_title_while_the_page_scrolls() {
    for (w, h, dpi) in [(480, 640, 160), (1366, 768, 160), (480, 640, 286)] {
        let mut app = on_at(w, h, dpi);
        to_entry(&mut app, 6);
        for f in faces(b'q', 1, 2) {
            c32(&mut app, C32::Face(f));
        }
        let step = if dpi == 160 {
            "Share A · character 2 of 26"
        } else {
            "Share A · 2 of 26"
        };
        for dy in [0, 300, 300] {
            app.event(Event::Scroll {
                x: w / 2,
                y: h / 2,
                dy,
            });
            app.settle();
            let _ = app.frame();
            let texts = app.drawn_text_boxes();
            let at: Vec<_> = texts.iter().filter(|(t, _)| t == step).collect();
            assert_eq!(at.len(), 1, "{step} drawn once at {w} × {h}: {at:?}");
            // In the bar: above its foot, 64 dp.
            let (_, (_, y, _, bh)) = at[0];
            assert!(
                y + bh <= 64 * i32::from(dpi) / 160,
                "{step} in the bar at {w} × {h}: {:?}",
                at[0]
            );
            // On the panel the page opens scrolled to the dice, the card's
            // head under the bar.
            if dy == 0 && dpi == 160 {
                assert!(
                    texts.iter().any(|(t, _)| t == "Characters"),
                    "the open card reads Characters at {w} × {h}"
                );
            }
        }
    }
}

/// On the Pi's panel off mainnet, where the mode pill leaves the bar no
/// room for the step even in its short form, the open card reads it in
/// place of Characters, so where the entry stands is still on screen;
/// the way back, with no room for "Back to Entropy", reads the step's
/// name as far as it fits (F146).
#[test]
fn on_the_pi_panel_off_mainnet_the_card_reads_the_step_the_bar_has_no_room_for() {
    let mut app = on_at(480, 640, 286);
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    to_entry(&mut app, 6);
    for f in faces(b'q', 1, 2) {
        c32(&mut app, C32::Face(f));
    }
    for _ in 0..4 {
        app.event(Event::Scroll {
            x: 240,
            y: 320,
            dy: -30_000,
        });
    }
    app.settle();
    let _ = app.frame();
    let texts = app.drawn_texts();
    // The card's title wraps where the panel is narrow: its lines joined.
    let read = texts.join(" ");
    let step = "Share A · character 2 of 26";
    assert_eq!(
        read.matches(step).count(),
        1,
        "{step} drawn once: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == "Share A · 2 of 26"),
        "the bar has no short form beside it: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.starts_with("Entr")) && !texts.iter().any(|t| t.starts_with("Back")),
        "the way back is named after Entropy: {texts:?}"
    );
}
