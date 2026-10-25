//! The codex32 dice (`docs/CODEX32.md` §5.1, §5.5, §6.4.2): dice of any
//! number of faces up to twenty, each die's colour, both rolls side by
//! side, and the booklet's sheet with its tracks, as a person rolling a
//! set's characters on New key would notice them. Rolled on twenty-faced
//! dice, a set is BIP 93's vector 2 as on six-faced ones.

use faraday_core::codex32::WalkAct;
use faraday_core::codex32::{C32, Card, DIE_SIZES, DieColour, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::{Start, kstep};
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
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
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
    app.press(Action::KStep(kstep::START));
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

/// The app on Faraday Desktop (the roller offered) or the stick, Reduce
/// motion on, on the entry by dice of `faces` faces.
fn desktop(offered: bool, faces: u8) -> Faraday {
    let mut app = on(1366, 4000);
    app.set_roller_offered(offered);
    if offered {
        // The roller is Demo's alone (`docs/DEMO.md` §14).
        faraday_core::testkit::into_demo(&mut app);
    }
    app.press(Action::ReduceMotion(true));
    to_entry(&mut app, faces);
    app
}

#[test]
fn the_roller_and_its_dice_are_in_a_demo_only_box_on_faraday_desktop_alone() {
    let mut app = desktop(true, 6);
    let texts = app.drawn_texts();
    for want in ["Demo only", "Rolls d6", "Or let Faraday roll", "Your dice"] {
        assert!(texts.iter().any(|t| t == want), "{want}");
    }
    assert!(app.offers(Action::Codex32(C32::Walk(WalkAct::Next))));
    // Beside the person's own entry: the box stands right of the pad.
    let boxes = app.drawn_text_boxes();
    let at = |s: &str| boxes.iter().find(|(t, _)| t == s).map(|b| b.1).unwrap();
    assert!(at("Demo only").0 > at("Your dice").0 + at("Your dice").2);
    let mut stick = desktop(false, 6);
    let texts = stick.drawn_texts();
    assert!(
        !texts
            .iter()
            .any(|t| t == "Demo only" || t == "Or let Faraday roll")
    );
    assert!(!stick.offers(Action::Codex32(C32::Walk(WalkAct::Next))));
}

#[test]
fn twenty_faced_dice_are_rolled_and_padded_one_to_twenty() {
    let mut app = desktop(true, 20);
    assert!(app.drawn_texts().iter().any(|t| t == "Rolls d20"));
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
    app.press(Action::KStep(kstep::START));
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
    app.press(Action::KStep(kstep::START));
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
        "First roll",
        "Second roll",
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
