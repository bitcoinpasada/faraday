//! The codex32 dice (`docs/CODEX32.md` §5.1, §5.5, §6.4.2): dice of any
//! number of faces up to twenty, each die's colour, both rolls side by
//! side, and the booklet's sheet with its tracks, as a person rolling a
//! set's characters on New key would notice them. Rolled on twenty-faced
//! dice, a set is BIP 93's vector 2 as on six-faced ones.

use faraday_core::codex32::{C32, Card, DIE_SIZES, DieColour, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::Start;
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

/// Frames drawn, a tick before each, until a step's fade has run out: the
/// step that changed shows as the card.
fn through_fade(app: &mut Faraday) {
    let mut t = app.now_ms();
    for _ in 0..40 {
        t += 16;
        app.event(Event::Tick { now_ms: t });
        let _ = app.frame();
    }
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
    through_fade(app);
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
        assert!(
            texts.iter().filter(|t| t.as_str() == n).count() >= 5,
            "pad {n}"
        );
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

/// While a share's characters are rolled, the top app bar's title is the
/// step, "Share A · character 2 of 26", and it stays in view as the page
/// scrolls under the bar; the step is drawn once (F146). On a phone-sized window, a wide one and the Pi's
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
        }
    }
}

/// On the Pi's panel off mainnet the mode pill stands at the end of the
/// bar's first line, so the entry's step, too long for one line beside it,
/// wraps onto the second and is read whole there, with no way-back label
/// under it; the card keeps its name, Characters.
#[test]
fn on_the_pi_panel_off_mainnet_the_step_wraps_in_the_bar_beside_the_pill() {
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
    assert!(
        texts.iter().any(|t| t == "Share A") && texts.iter().any(|t| t == "· character 2 of 26"),
        "the bar reads the step whole on two lines: {texts:?}"
    );
    assert!(texts.iter().any(|t| t == "Testnet"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t == "Characters"),
        "the card keeps its name: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.starts_with("Back to")),
        "no way-back label under a name on two lines: {texts:?}"
    );
}

#[test]
fn twenty_faced_dice_are_padded_one_to_twenty() {
    let mut app = on(1366, 4000);
    app.press(Action::ReduceMotion(true));
    to_entry(&mut app, 20);
    let _ = app.frame();
    // The pad's keys 1 to 20.
    for n in 1..=20u8 {
        assert!(app.offers(Action::Codex32(C32::Face(n))), "key {n}");
    }
    assert!(!app.offers(Action::Codex32(C32::Face(21))));
    // Faces past six, pressed by hand, are rolls of the character.
    for f in [17, 3, 20, 9] {
        c32(&mut app, C32::Face(f));
    }
    assert_eq!(make(&app).seq_len(), 4);
}
