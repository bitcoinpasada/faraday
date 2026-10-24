//! Make codex32 shares (`docs/CODEX32.md` §6): what a person typing or
//! rolling a set's characters on New key would notice, against BIP 93's
//! published vectors (`tools/vectors/bip93/vectors.json`). The sheets
//! fill as each character arrives, the strings are the published ones,
//! a string typed back with a box changed names that box, and Lock in
//! loads the key the strings carry.

use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_ui::keyboard::OskPress;

const BIP93: &str = include_str!("../../../../tools/vectors/bip93/vectors.json");
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

fn on(width: u16, height: u16, dpi: u16) -> Faraday {
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

/// Vector 2's set in index order (A, C, then the derived D), lowercase,
/// and its seed, as `tools/vectors/bip93/vectors.json` publishes them.
fn vector2() -> (Vec<String>, Vec<u8>) {
    let all = [
        "MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM",
        "MS12NAMECACDEFGHJKLMNPQRSTUVWXYZ023FTR2GDZMPY6PN",
        "MS12NAMEDLL4F8JLH4E5VDVULDLFXU2JHDNLSM97XVENRXEG",
    ];
    assert!(
        all.iter().all(|s| BIP93.contains(s)),
        "the published strings"
    );
    let hex = "d1808e096b35b209ca12132b264662a5";
    assert!(BIP93.contains(hex));
    let seed = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    (all.iter().map(|s| s.to_lowercase()).collect(), seed)
}

/// New key opened on its own, Make a new key's Codex32 chosen, its
/// cards open on Scheme.
fn opened(app: &mut Faraday) {
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    // Start opens closed on Make a new key; Change opens it.
    assert_eq!(app.keygen().unwrap().open, Some(kstep::LENGTH));
    app.press(Action::KStep(kstep::START));
    let _ = app.frame();
    assert!(
        app.offers(Action::KStart(Start::MakeCodex32.index())),
        "New key offers Codex32 under Make a new key"
    );
    // The choice moves on by itself.
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let k = app.keygen().unwrap();
    assert_eq!(k.open, Some(kstep::CODEX32));
    assert_eq!(k.codex32.as_ref().unwrap().open(), Some(Card::Scheme));
}

/// The scheme (2 of 3 at 48 characters, the defaults), `name` typed as
/// the identifier, and `way` chosen: the entry open.
fn to_entry(app: &mut Faraday, way: Way) {
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(app, C32::Split(split as u8));
    c32(app, C32::Next);
    for ch in "name".chars() {
        key(app, ch);
    }
    c32(app, C32::Next);
    let w = WAYS.iter().position(|w| *w == way).unwrap();
    c32(app, C32::Way(w as u8));
    c32(app, C32::Next);
    let m = app.keygen().unwrap().codex32.as_ref().unwrap();
    assert_eq!(m.open(), Some(Card::Enter), "{:?}", m.note());
}

/// The ten faces that roll character `ch` on the booklet's sheet: each
/// die's bit 0 as 4 then 3, 1 as 3 then 4.
fn faces(ch: u8) -> [u8; 10] {
    let v = CHARSET.iter().position(|&x| x == ch).unwrap() as u8;
    let mut out = [0u8; 10];
    for d in 0..5 {
        let bit = v >> (4 - d) & 1;
        (out[d], out[5 + d]) = if bit == 0 { (4, 3) } else { (3, 4) };
    }
    out
}

/// The flow's strings, once made.
fn strings(app: &Faraday) -> Vec<String> {
    let m = app.keygen().unwrap().codex32.as_ref().unwrap();
    m.strings()
        .map(|s| s.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn characters_typed_make_vector_2s_strings_and_lock_in_loads_its_key() {
    let (all, seed) = vector2();
    let mut app = on(1366, 768, 160);
    opened(&mut app);
    to_entry(&mut app, Way::Typed);
    for s in &all[..2] {
        for &ch in &s.as_bytes()[9..35] {
            key(&mut app, char::from(ch));
            let _ = app.frame();
        }
    }
    assert_eq!(strings(&app), all, "the published strings, in index order");
    // The worksheets, then each string: typed back the same.
    c32(&mut app, C32::Next);
    assert_eq!(
        app.keygen().unwrap().codex32.as_ref().unwrap().open(),
        Some(Card::Sheets)
    );
    let _ = app.frame();
    c32(&mut app, C32::Next);
    for s in &all {
        for ch in s[3..].chars() {
            key(&mut app, ch);
        }
        let _ = app.frame();
        c32(&mut app, C32::Next);
    }
    let m = app.keygen().unwrap().codex32.as_ref().unwrap();
    assert_eq!(m.open(), Some(Card::Made));
    let want = {
        let c = faraday_core::testkit::compute();
        faraday_keys::identity::fingerprint_of_seed(&c, &seed)
            .unwrap()
            .0
    };
    assert_eq!(m.fingerprint(), Some(want), "vector 2's key");
    let _ = app.frame();
    c32(&mut app, C32::Lock);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].fingerprint.0, want);
    assert!(
        !app.session.keys[0].has_words(),
        "a codex32 key has no words"
    );
    c32(&mut app, C32::Next);
    assert!(app.keygen().is_none(), "Done leaves the flow");
}

#[test]
fn rolled_dice_make_the_same_strings_and_an_equal_pair_is_rolled_again() {
    let (all, _) = vector2();
    let mut app = on(1366, 768, 160);
    opened(&mut app);
    to_entry(&mut app, Way::Dice);
    for (n, &ch) in all[0].as_bytes()[9..35].iter().enumerate() {
        let f = faces(ch);
        if n == 0 {
            // Die 2 comes out equal first: struck, and rolled again.
            for &x in &f[..5] {
                key(&mut app, char::from(b'0' + x));
            }
            key(&mut app, char::from(b'0' + f[5]));
            key(&mut app, char::from(b'0' + f[1]));
            let m = app.keygen().unwrap().codex32.as_ref().unwrap();
            assert_eq!(m.equal(), Some(1), "die 2 is rolled again");
            key(&mut app, char::from(b'0' + f[1]));
            key(&mut app, char::from(b'0' + f[6]));
            for &x in &f[7..] {
                key(&mut app, char::from(b'0' + x));
            }
            let m = app.keygen().unwrap().codex32.as_ref().unwrap();
            let made = m.made_of(0, 0).expect("box 10 is in");
            assert_eq!(made.rows().len(), 6, "five pairs and the struck one");
            assert!(made.rows().iter().any(|r| r.bit().is_none()));
            let _ = app.frame();
            continue;
        }
        for x in f {
            key(&mut app, char::from(b'0' + x));
        }
    }
    for &ch in &all[1].as_bytes()[9..35] {
        for x in faces(ch) {
            key(&mut app, char::from(b'0' + x));
        }
    }
    let _ = app.frame();
    assert_eq!(strings(&app), all);
}

#[test]
fn backspace_takes_back_a_roll_then_a_character() {
    let mut app = on(1366, 768, 160);
    opened(&mut app);
    to_entry(&mut app, Way::Dice);
    for x in faces(b'q') {
        key(&mut app, char::from(b'0' + x));
    }
    key(&mut app, '2');
    let front = |app: &Faraday| app.keygen().unwrap().codex32.as_ref().unwrap().front();
    assert_eq!(front(&app), Some((0, 1)));
    c32(&mut app, C32::Key(OskPress::Back));
    assert_eq!(
        front(&app),
        Some((0, 1)),
        "the roll of the next character goes"
    );
    assert!(
        app.keygen()
            .unwrap()
            .codex32
            .as_ref()
            .unwrap()
            .dice()
            .is_empty()
    );
    c32(&mut app, C32::Key(OskPress::Back));
    assert_eq!(
        front(&app),
        Some((0, 0)),
        "then box 10, its rolls back less the last"
    );
    assert_eq!(
        app.keygen().unwrap().codex32.as_ref().unwrap().dice().len(),
        9
    );
}

#[test]
fn a_string_typed_back_with_a_box_changed_names_that_box() {
    let (all, _) = vector2();
    let mut app = on(1366, 768, 160);
    opened(&mut app);
    to_entry(&mut app, Way::Typed);
    for s in &all[..2] {
        for &ch in &s.as_bytes()[9..35] {
            key(&mut app, char::from(ch));
        }
    }
    c32(&mut app, C32::Next);
    c32(&mut app, C32::Next);
    let mut typed: Vec<char> = all[0][3..].chars().collect();
    // Box 17 is typed[13].
    typed[13] = if typed[13] == 'q' { 'p' } else { 'q' };
    for ch in typed {
        key(&mut app, ch);
    }
    let m = app.keygen().unwrap().codex32.as_ref().unwrap();
    assert_eq!(m.typed_back().1, Some(17));
    c32(&mut app, C32::Next);
    let m = app.keygen().unwrap().codex32.as_ref().unwrap();
    assert_eq!(m.at(), 0, "a copy that differs does not go on");
    // Skip asks, then goes on.
    c32(&mut app, C32::Skip);
    assert_eq!(app.keygen().unwrap().codex32.as_ref().unwrap().at(), 0);
    c32(&mut app, C32::Skip);
    assert_eq!(app.keygen().unwrap().codex32.as_ref().unwrap().at(), 1);
}

#[test]
fn the_secret_alone_is_its_characters_with_their_checksum() {
    let s = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw".to_string();
    assert!(BIP93.contains(&s), "vector 1");
    let mut app = on(1366, 768, 160);
    opened(&mut app);
    let alone = SPLITS.iter().position(|s| *s == Split::Alone).unwrap();
    c32(&mut app, C32::Split(alone as u8));
    c32(&mut app, C32::Next);
    for ch in s[4..8].chars() {
        key(&mut app, ch);
    }
    c32(&mut app, C32::Next);
    c32(&mut app, C32::Way(1));
    c32(&mut app, C32::Next);
    for ch in s[9..35].chars() {
        key(&mut app, ch);
    }
    assert_eq!(strings(&app), vec![s]);
}

#[test]
fn a_multisig_slot_offers_no_codex32_to_make() {
    use faraday_core::create::NewKind;
    let mut app = faraday_core::testkit::started();
    app.press(Action::CreateWallet);
    let at = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CKind(at));
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::CNext(faraday_core::cstep::QUORUM));
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KStep(kstep::START));
    let _ = app.frame();
    assert!(app.offers(Action::KStart(Start::Make.index())));
    assert!(!app.offers(Action::KStart(Start::MakeCodex32.index())));
    assert!(app.offers(Action::KStart(Start::Codex32.index())));
}

/// Every card of the flow draws at the three size classes: the desktop,
/// a phone and the Pi's panel.
#[test]
fn every_card_draws_at_each_size_class() {
    let (all, _) = vector2();
    for (w, h, dpi) in [(1366, 768, 160), (1080, 2340, 420), (480, 640, 286)] {
        let mut app = on(w, h, dpi);
        opened(&mut app);
        let _ = app.frame();
        to_entry(&mut app, Way::Dice);
        let _ = app.frame();
        for &ch in &all[0].as_bytes()[9..20] {
            for x in faces(ch) {
                key(&mut app, char::from(b'0' + x));
            }
        }
        for p in 0..3 {
            c32(&mut app, C32::Panel(p));
            let _ = app.frame();
        }
        c32(
            &mut app,
            C32::Cell(faraday_core::codex32::Cell::Given { sheet: 0, b: 12 }),
        );
        let _ = app.frame();
        c32(&mut app, C32::CellClose);
        for s in &all[..2] {
            let from = if s == &all[0] { 20 } else { 9 };
            for &ch in &s.as_bytes()[from..35] {
                for x in faces(ch) {
                    key(&mut app, char::from(b'0' + x));
                }
            }
        }
        let _ = app.frame();
        c32(&mut app, C32::Next);
        let tabs = app.keygen().unwrap().codex32.as_ref().unwrap().tabs().len();
        assert_eq!(
            tabs,
            2 + 2 + 2,
            "Dice · A, C; Checksum · A, C; Share D; Secret"
        );
        for t in 0..tabs as u8 {
            c32(&mut app, C32::Tab(t));
            let _ = app.frame();
        }
        c32(&mut app, C32::Next);
        let _ = app.frame();
        c32(&mut app, C32::Qr);
        let _ = app.frame();
        assert!(
            app.keygen()
                .unwrap()
                .codex32
                .as_ref()
                .unwrap()
                .qr()
                .is_some()
        );
    }
}

/// One rolled character 'z' in the entry, on a device.
fn rolled_z(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = on(width, height, dpi);
    opened(&mut app);
    to_entry(&mut app, Way::Dice);
    for f in faces(b'z') {
        key(&mut app, char::from(b'0' + f));
    }
    let _ = app.frame();
    app
}

fn masked(app: &Faraday) -> bool {
    app.keygen().unwrap().codex32.as_ref().unwrap().masked()
}

#[test]
fn the_sheets_are_shown_while_the_flow_is_open_and_the_eye_masks_them() {
    let mut app = rolled_z(1366, 768, 160);
    assert!(!masked(&app), "unmasked while the flow is open");
    assert!(app.offers(Action::Codex32(C32::Eye)), "the eye is offered");
    assert!(app.drawn_texts().iter().any(|t| t == "z"), "the character");
    c32(&mut app, C32::Eye);
    let texts = app.drawn_texts();
    assert!(masked(&app));
    assert!(!texts.iter().any(|t| t == "z"), "no character while masked");
    assert!(texts.iter().any(|t| t == "\u{2022}"), "dots stand for them");
    c32(&mut app, C32::Eye);
    assert!(!masked(&app));
    assert!(app.drawn_texts().iter().any(|t| t == "z"));
}

#[test]
fn going_back_past_the_entry_masks_the_sheets_until_the_eye_shows_them() {
    let mut app = rolled_z(1366, 768, 160);
    c32(&mut app, C32::Card(Card::Entropy.index()));
    assert!(masked(&app), "back past the first entry card");
    c32(&mut app, C32::Card(Card::Enter.index()));
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t == "z"),
        "still masked when reopened"
    );
    c32(&mut app, C32::Eye);
    assert!(app.drawn_texts().iter().any(|t| t == "z"));
}

#[test]
fn leaving_new_key_forgets_the_sheets() {
    let mut app = rolled_z(1366, 768, 160);
    app.press(Action::Nav(Screen::Home));
    assert!(app.keygen().is_none(), "nothing is kept to show again");
}

#[test]
fn faraday_desktop_never_masks_and_has_no_eye() {
    let mut app = on(1366, 768, 160);
    app.online = true;
    opened(&mut app);
    to_entry(&mut app, Way::Dice);
    for f in faces(b'z') {
        key(&mut app, char::from(b'0' + f));
    }
    let _ = app.frame();
    assert!(!app.offers(Action::Codex32(C32::Eye)));
    c32(&mut app, C32::Card(Card::Entropy.index()));
    c32(&mut app, C32::Card(Card::Enter.index()));
    assert!(!masked(&app));
    assert!(app.drawn_texts().iter().any(|t| t == "z"));
}

#[test]
fn the_pi_panel_shows_the_context_above_the_pad_and_the_cell_has_its_own_page() {
    let mut app = rolled_z(480, 640, 286);
    let boxes = app.drawn_text_boxes();
    let find = |name: &str| {
        boxes
            .iter()
            .find(|(t, _)| t == name)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("{name} is drawn"))
    };
    let die = find("Die 5");
    let (_, key_y, _, _) = app
        .hit_box(Action::Codex32(C32::Key(OskPress::Char('4'))))
        .expect("the pad is drawn");
    assert!(
        die.1 + die.3 <= i32::from(key_y),
        "the dice rows end above the pad, not under it"
    );
    assert!(die.1 + die.3 <= 640, "and within the panel");
    // A checksum cell opens its working as a page: Back closes it.
    c32(&mut app, C32::Panel(1));
    let _ = app.frame();
    c32(
        &mut app,
        C32::Cell(faraday_core::codex32::Cell::Given { sheet: 0, b: 10 }),
    );
    let _ = app.frame();
    assert!(app.offers(Action::Codex32(C32::CellClose)), "its own Back");
    assert!(app.offers(Action::Codex32(C32::CellStep(1))));
    c32(&mut app, C32::CellStep(1));
    c32(&mut app, C32::CellClose);
    assert!(
        app.keygen()
            .unwrap()
            .codex32
            .as_ref()
            .unwrap()
            .cell()
            .is_none()
    );
}

#[test]
fn start_offers_making_codex32_only_while_it_is_switched_on() {
    use faraday_core::keygen::KeyGen;
    let k = KeyGen::new(None, Screen::Entry);
    assert!(k.starts_with(true).contains(&Start::MakeCodex32));
    let off = k.starts_with(false);
    assert!(!off.contains(&Start::MakeCodex32), "not offered when off");
    assert!(off.contains(&Start::Codex32), "typing codex32 stays");
    assert!(off.contains(&Start::Make) && off.contains(&Start::Words));
    assert_eq!(
        k.starts(),
        k.starts_with(faraday_core::codex32::MAKE_OFFERED)
    );
}

#[test]
fn the_translation_symbols_are_drawn_as_the_booklets_glyphs() {
    use faraday_bip::codex32::{SYMBOL_GLYPHS, SYMBOL_NAMES};
    let (all, _) = vector2();
    let mut app = on(1366, 2400, 160);
    opened(&mut app);
    to_entry(&mut app, Way::Typed);
    // Every character but the last: the sheets are still filling.
    for s in &all[..2] {
        for &ch in &s.as_bytes()[9..35] {
            key(&mut app, char::from(ch));
        }
    }
    c32(&mut app, C32::Key(OskPress::Back));
    // The first translation symbol's working, as its cell opens it.
    c32(
        &mut app,
        C32::Cell(faraday_core::codex32::Cell::Symbol { sheet: 0, i: 0 }),
    );
    let texts = app.drawn_texts();
    let glyphs: Vec<char> = SYMBOL_GLYPHS[1..]
        .iter()
        .copied()
        .filter(|c| !c.is_ascii())
        .collect();
    assert!(
        texts.iter().any(|t| t.chars().any(|c| glyphs.contains(&c))),
        "a symbol drawn as its glyph: {texts:?}"
    );
    for name in SYMBOL_NAMES.iter().filter(|n| n.len() > 3) {
        assert!(
            !texts
                .iter()
                .any(|t| t.split_whitespace().any(|w| w == *name)),
            "{name} drawn as its name: {texts:?}"
        );
    }
}

/// Whether pixel boxes `a` and `b` (x, y, w, h) share any column.
fn cols_meet(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2
}

fn boxed(b: (u16, u16, u16, u16)) -> (i32, i32, i32, i32) {
    (
        i32::from(b.0),
        i32::from(b.1),
        i32::from(b.2),
        i32::from(b.3),
    )
}

#[test]
fn a_cells_working_and_its_buttons_stay_inside_its_sheet_beside_the_sidebar() {
    use faraday_core::codex32::Cell;
    use faraday_shell_api::{Event, TouchPhase};
    let mut app = rolled_z(1280, 800, 160);
    c32(&mut app, C32::Cell(Cell::Given { sheet: 0, b: 10 }));
    let boxes = app.drawn_text_boxes();
    let prev = boxed(app.hit_box(Action::Codex32(C32::CellStep(-1))).unwrap());
    let next = boxed(app.hit_box(Action::Codex32(C32::CellStep(1))).unwrap());
    let done = boxed(app.hit_box(Action::Codex32(C32::CellClose)).unwrap());
    // Previous, Next and Done in one row, Done last, none over another.
    assert_eq!((prev.1, next.1), (done.1, done.1), "one row");
    assert!(prev.0 + prev.2 <= next.0 && next.0 + next.2 <= done.0);
    let (left, right) = (prev.0, done.0 + done.2);
    // Every line of the working lies between the sheet's buttons' ends:
    // its dice lines ("Die 1 · 5 then 2 · lower · 0") and its bits, not
    // the page's own die names under the sheet.
    let working: Vec<_> = boxes
        .iter()
        .filter(|(t, b)| {
            b.1 < done.1 && ((t.starts_with("Die ") && t.contains(" · ")) || t.contains("bits"))
        })
        .collect();
    assert!(!working.is_empty(), "{boxes:?}");
    for (t, b) in working {
        assert!(
            b.0 >= left && b.0 + b.2 <= right,
            "{t:?} {b:?} outside {left}..{right}"
        );
    }
    // A press inside the sheet's edge, just right of Done, is on the
    // sheet: it reaches nothing under it.
    let x = (done.0 + done.2 + 8) as u16;
    let y = (done.1 + done.3 / 2) as u16;
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
    assert_eq!(
        app.keygen().unwrap().codex32.as_ref().unwrap().cell(),
        Some(Cell::Given { sheet: 0, b: 10 }),
        "the sheet's own margin is the sheet's"
    );
}

#[test]
fn the_eye_stands_clear_of_every_row_of_the_flow() {
    // Beside the sidebar, the eye is in the page's margin: no text of the
    // flow's column, which scrolls under the top, shares its columns
    // below the top row of the network and the ?. The input panel pinned
    // at the window's foot (`docs/LAYOUT.md` §16.3) never scrolls under
    // it: its rows, from the rolls' heads down, are not the column's.
    let mut app = rolled_z(1280, 800, 160);
    let eye = boxed(app.hit_box(Action::Codex32(C32::Eye)).expect("the eye"));
    let boxes = app.drawn_text_boxes();
    let panel_top = boxes
        .iter()
        .find(|(t, _)| t == "First roll")
        .map_or(i32::MAX, |(_, b)| b.1 - 48);
    for (t, b) in boxes.into_iter().filter(|(_, b)| b.1 < panel_top) {
        assert!(
            !cols_meet(b, eye) || b.1 + b.3 <= eye.1,
            "{t:?} {b:?} in the eye's column {eye:?}"
        );
    }
    // On a small panel the eye is on the bar, which the page never runs
    // under: only the bar's own lines meet it.
    let mut app = rolled_z(480, 640, 286);
    let eye = boxed(app.hit_box(Action::Codex32(C32::Eye)).expect("the eye"));
    for (t, b) in app.drawn_text_boxes() {
        let meets = cols_meet(b, eye) && b.1 < eye.1 + eye.3 && eye.1 < b.1 + b.3;
        assert!(
            !meets || b.1 + b.3 <= eye.1 + eye.3,
            "{t:?} {b:?} under the eye {eye:?}"
        );
    }
}

/// Share A of vector 2, its first `n` data characters typed, on a
/// window `width` × `height` at 160 dpi (tall enough that the pad
/// and the sheet are both on it), a frame drawn after each as the shells
/// draw one after every event, and the worksheet's view let come to
/// rest.
fn typed_a(width: u16, height: u16, n: usize) -> Faraday {
    typed_a_in(None, width, height, n)
}

/// As [`typed_a`], drawn in `theme` from the start where one is given.
fn typed_a_in(theme: Option<faraday_ui::Theme>, width: u16, height: u16, n: usize) -> Faraday {
    let mut app = on(width, height, 160);
    if let Some(theme) = theme {
        app.press(Action::Theme(theme));
    }
    opened(&mut app);
    to_entry(&mut app, Way::Typed);
    let _ = app.frame();
    type_a(&mut app, 0..n);
    frames(&mut app, 40);
    app
}

/// Share A of vector 2's data characters `range` typed, a frame after
/// each.
fn type_a(app: &mut Faraday, range: std::ops::Range<usize>) {
    let (all, _) = vector2();
    for &ch in &all[0].as_bytes()[9 + range.start..9 + range.end] {
        key(app, char::from(ch));
        let _ = app.frame();
    }
}

/// `n` frames 16 ms apart, a tick before each, as a shell draws them.
fn frames(app: &mut Faraday, n: usize) {
    let mut t = app.now_ms();
    for _ in 0..n {
        t += 16;
        app.event(Event::Tick { now_ms: t });
        let _ = app.frame();
    }
}

/// Vector 2's shares A and C typed in `theme` on a window `width` ×
/// `height` at 160 dpi: the entry done, Share C's string and worksheet
/// on it.
pub(crate) fn both_typed_in(theme: faraday_ui::Theme, width: u16, height: u16) -> Faraday {
    let (all, _) = vector2();
    let mut app = typed_a_in(Some(theme), width, height, 26);
    for &ch in &all[1].as_bytes()[9..35] {
        key(&mut app, char::from(ch));
        let _ = app.frame();
    }
    app
}

fn pan(app: &Faraday) -> faraday_core::codex32::SheetPan {
    app.keygen().unwrap().codex32.as_ref().unwrap().pan()
}

#[test]
fn every_box_of_the_string_carries_its_number() {
    // The small panel's String tile: the share's boxes alone.
    let mut app = rolled_z(480, 640, 286);
    c32(&mut app, C32::Panel(2));
    let texts = app.drawn_texts();
    for n in 1..=48 {
        assert!(
            texts.iter().any(|t| *t == n.to_string()),
            "box {n} is numbered: {texts:?}"
        );
    }
}

#[test]
fn the_checksum_worksheet_pans_by_a_drag_the_wheel_and_the_arrow_keys() {
    use faraday_core::codex32::Cell;
    use faraday_shell_api::{Key, TouchPhase};
    let mut app = typed_a(1280, 1600, 1);
    assert_eq!(pan(&app), Default::default(), "it opens at its top left");
    // A cell of the residue's row, near the sheet's top left corner.
    let cell = Action::Codex32(C32::Cell(Cell::Hrp { sheet: 0, i: 0 }));
    let (x, y, w, h) = app.hit_box(cell).expect("the residue's first cell");
    let (x, y) = (x + w / 2, y + h / 2);
    // Held and dragged up and to the left: the sheet follows both ways,
    // and the cell held is not pressed.
    let touch = |app: &mut Faraday, x: u16, y: u16, phase| {
        app.event(Event::Touch { x, y, phase });
    };
    touch(&mut app, x, y, TouchPhase::Down);
    touch(&mut app, x - 60, y - 60, TouchPhase::Move);
    touch(&mut app, x - 120, y - 120, TouchPhase::Move);
    touch(&mut app, x - 120, y - 120, TouchPhase::Up);
    let _ = app.frame();
    let dragged = pan(&app);
    assert!(dragged.x > 0.0 && dragged.y > 0.0, "{dragged:?}");
    assert!(
        app.keygen()
            .unwrap()
            .codex32
            .as_ref()
            .unwrap()
            .cell()
            .is_none(),
        "a drag opens no cell's working"
    );
    // A trackpad scrolls it down; the wheel with Shift held, across.
    app.event(Event::Scroll { x, y, dy: 40 });
    let _ = app.frame();
    assert!(pan(&app).y > dragged.y, "scrolled down");
    let before = pan(&app);
    app.event(Event::Shift { held: true });
    app.event(Event::Wheel { x, y, dy: 48 });
    app.event(Event::Shift { held: false });
    let _ = app.frame();
    assert!(pan(&app).x > before.x, "Shift and the wheel pan across");
    assert_eq!(pan(&app).y, before.y);
    // The arrow keys, once the pointer is over it.
    let before = pan(&app);
    app.event(Event::Key(Key::Right));
    let _ = app.frame();
    assert!(pan(&app).x > before.x, "Right pans across");
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Left));
    let _ = app.frame();
    assert!(pan(&app).x < before.x, "Left pans back");
}

#[test]
fn the_worksheet_keeps_the_character_being_entered_in_view() {
    use faraday_core::codex32::Cell;
    // Twenty characters in: box 29 is brought down at the end of the
    // seventh `=` row, below the view's first height.
    let app = typed_a(1280, 1600, 20);
    let row = Action::Codex32(C32::Cell(Cell::Sum {
        sheet: 0,
        j: 6,
        i: 12,
    }));
    assert!(pan(&app).y > 0.0, "the sheet scrolled down to the front");
    assert!(
        app.hit_box(row).is_some(),
        "the front's row is in view, its cells pressable"
    );
}

#[test]
fn the_worksheet_follows_the_front_over_several_frames() {
    // Twelve characters in and the view at rest; eight more bring box 29
    // down below the view, and the view moves to it over several frames,
    // every one of them part of the way.
    let mut app = typed_a(1280, 1600, 12);
    let from = pan(&app).y;
    type_a(&mut app, 12..20);
    let mut ys = vec![pan(&app).y];
    let mut t = app.now_ms();
    for _ in 0..40 {
        t += 16;
        app.event(Event::Tick { now_ms: t });
        let _ = app.frame();
        ys.push(pan(&app).y);
    }
    let to = *ys.last().unwrap();
    assert!(to > from, "it moved down to the front: {ys:?}");
    assert_eq!(ys[0], from, "nothing jumps at the key");
    let between = ys.iter().filter(|&&y| y > from && y < to).count();
    assert!(between >= 5, "several frames on the way: {ys:?}");
    assert!(ys.windows(2).all(|w| w[1] >= w[0]), "one way: {ys:?}");

    // Under Reduce motion the view is at the same place at once.
    let mut reduced = typed_a(1280, 1600, 12);
    reduced.press(Action::ReduceMotion(true));
    type_a(&mut reduced, 12..20);
    assert_eq!(pan(&reduced).y, to);
}

#[test]
fn the_eye_names_what_a_press_does_when_the_pointer_rests_on_it() {
    use faraday_core::strings::EN;
    let mut app = rolled_z(1366, 768, 160);
    let count = |app: &mut Faraday, s: &str| app.drawn_texts().iter().filter(|t| *t == s).count();
    let (x, y) = app.where_offered(Action::Codex32(C32::Eye)).unwrap();
    let (show, hide) = (count(&mut app, EN.tip_show), count(&mut app, EN.tip_hide));
    let now = app.now_ms();
    app.event(Event::Hover { x, y });
    app.event(Event::Tick { now_ms: now + 600 });
    assert_eq!(
        count(&mut app, EN.tip_hide),
        hide + 1,
        "while the sheets show, a press hides them"
    );
    assert_eq!(count(&mut app, EN.tip_show), show);
    app.event(Event::HoverEnd);
    c32(&mut app, C32::Eye);
    let (x, y) = app.where_offered(Action::Codex32(C32::Eye)).unwrap();
    let (show, hide) = (count(&mut app, EN.tip_show), count(&mut app, EN.tip_hide));
    app.event(Event::Hover { x, y });
    app.event(Event::Tick { now_ms: now + 1200 });
    assert_eq!(
        count(&mut app, EN.tip_show),
        show + 1,
        "while masked, a press shows them"
    );
    assert_eq!(count(&mut app, EN.tip_hide), hide);
}

/// The page's own scroll on New key, design units.
fn page_y(app: &Faraday) -> f32 {
    app.keygen().unwrap().scroll.y
}

/// The residue's first cell on the checksum worksheet shown.
const HRP: Action = Action::Codex32(C32::Cell(faraday_core::codex32::Cell::Hrp {
    sheet: 0,
    i: 0,
}));

/// Box 34 where the worksheet brings it down, near the sheet's foot.
const BOX_34: Action = Action::Codex32(C32::Cell(faraday_core::codex32::Cell::Given {
    sheet: 0,
    b: 34,
}));

#[test]
fn on_large_windows_the_checksum_worksheet_is_drawn_whole_and_a_drag_moves_nothing() {
    use faraday_shell_api::TouchPhase;
    for (w, h) in [(1920, 1080), (2560, 1440)] {
        // Twenty-five characters in: box 34 is the last one written.
        let mut app = typed_a(w, h, 25);
        assert_eq!(pan(&app), Default::default(), "{w}x{h}: no view to pan");
        assert!(page_y(&app) > 0.0, "{w}x{h}: the page went to the work");
        let at = app.hit_box(BOX_34).expect("box 34 is on screen");
        assert!(u32::from(at.1) + u32::from(at.3) <= u32::from(h), "{at:?}");
        // Dragged across, as a view of the sheet would pan: nothing moves.
        let (x, y) = (at.0 + at.2 / 2, at.1 + at.3 / 2);
        for (x, phase) in [
            (x, TouchPhase::Down),
            (x - 60, TouchPhase::Move),
            (x - 120, TouchPhase::Move),
            (x - 120, TouchPhase::Up),
        ] {
            app.event(Event::Touch { x, y, phase });
        }
        frames(&mut app, 20);
        assert_eq!(pan(&app), Default::default());
        assert_eq!(app.hit_box(BOX_34), Some(at), "{w}x{h}: the sheet stayed");
    }
}

#[test]
fn a_larger_window_draws_the_whole_worksheet_at_a_larger_cell() {
    let side = |w, h| typed_a(w, h, 1).hit_box(HRP).expect("the residue's cell").2;
    let (at_1920, at_2560) = (side(1920, 1080), side(2560, 1440));
    assert!(at_2560 > at_1920, "{at_1920} then {at_2560}");
}

#[test]
fn at_1280_by_800_the_worksheet_is_in_a_view_that_follows_the_work() {
    let app = typed_a(1280, 800, 25);
    assert!(pan(&app).y > 0.0, "the view panned down to the front");
    assert!(app.hit_box(BOX_34).is_some(), "box 34 is in the view");
}

/// The residue's cell's side, the page scrolled down by a wheel over
/// its column's top until the cell stands clear of the input panel at
/// the window's foot (`docs/LAYOUT.md` §16.3).
fn residue_side(app: &mut Faraday) -> u16 {
    for _ in 0..40 {
        if let Some(at) = app.hit_box(HRP) {
            return at.2;
        }
        let (x, y) = (600, 120);
        app.event(Event::Hover { x, y });
        app.event(Event::Scroll { x, y, dy: 60 });
        app.event(Event::ScrollEnd { x, y });
        app.settle();
        let _ = app.frame();
    }
    panic!("the residue's cell is not on screen");
}

#[test]
fn the_worksheets_cell_follows_the_window_as_it_is_resized() {
    let display = |app: &mut Faraday, width, height| {
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
        let _ = app.frame();
        residue_side(app)
    };
    let mut app = typed_a(1280, 800, 1);
    let small = residue_side(&mut app);
    let large = residue_side(&mut typed_a(1920, 1080, 1));
    assert_ne!(small, large);
    assert_eq!(display(&mut app, 1920, 1080), large, "made larger");
    assert_eq!(display(&mut app, 1280, 800), small, "and smaller again");
}

#[test]
fn the_whole_sheet_brings_the_page_to_the_work_over_several_frames() {
    // Ten characters in, the page at rest; fifteen more take the front
    // down the sheet, and the page follows over several frames, at once
    // under Reduce motion.
    let mut app = typed_a(1920, 1080, 10);
    let from = page_y(&app);
    type_a(&mut app, 10..25);
    let mut ys = vec![page_y(&app)];
    let mut t = app.now_ms();
    for _ in 0..40 {
        t += 16;
        app.event(Event::Tick { now_ms: t });
        let _ = app.frame();
        ys.push(page_y(&app));
    }
    let to = *ys.last().unwrap();
    assert!(to > from, "the page moved down to the front: {ys:?}");
    let between = ys.iter().filter(|&&y| y > from && y < to).count();
    assert!(between >= 3, "several frames on the way: {ys:?}");
    assert_eq!(pan(&app), Default::default());

    let mut reduced = typed_a(1920, 1080, 10);
    reduced.press(Action::ReduceMotion(true));
    type_a(&mut reduced, 10..25);
    assert_eq!(page_y(&reduced), to);
}

#[test]
fn at_1920_the_strings_boxes_run_twenty_four_a_row() {
    let mut app = typed_a(1920, 1080, 1);
    // A row of numbered boxes: a line on which every one of `boxes` is
    // drawn, left to right.
    let texts = app.drawn_text_boxes();
    let row_of = |boxes: std::ops::RangeInclusive<usize>| {
        let mid = |r: &(i32, i32, i32, i32)| r.1 + r.3 / 2;
        let last = boxes.end().to_string();
        texts.iter().filter(|(t, _)| *t == last).find_map(|(_, r)| {
            let y = mid(r);
            let xs: Option<Vec<i32>> = boxes
                .clone()
                .map(|n| {
                    texts
                        .iter()
                        .find(|(t, b)| *t == n.to_string() && (mid(b) - y).abs() <= 2)
                        .map(|(_, b)| b.0 + b.2 / 2)
                })
                .collect();
            xs.filter(|xs| xs.windows(2).all(|w| w[0] < w[1]))
                .map(|xs| (y, xs[0]))
        })
    };
    let (one, x1) = row_of(1..=24).expect("boxes 1 to 24 on one row");
    let (two, x25) = row_of(25..=48).expect("boxes 25 to 48 on the next");
    assert!(two > one);
    assert!(
        (x25 - x1).abs() <= 2,
        "the second row starts under the first"
    );
}

/// A finger put down at (`x`, `y`) pixels and dragged by `dy` pixels, then
/// lifted, and a few frames drawn.
fn drag_from(app: &mut Faraday, (x, y): (u16, u16), dy: i32) {
    use faraday_shell_api::TouchPhase;
    let touch = |k: i32, phase| Event::Touch {
        x,
        y: (i32::from(y) + dy * k / 3).clamp(0, 4000) as u16,
        phase,
    };
    app.event(touch(0, TouchPhase::Down));
    app.event(touch(1, TouchPhase::Move));
    app.event(touch(2, TouchPhase::Move));
    app.event(touch(3, TouchPhase::Move));
    app.event(touch(3, TouchPhase::Up));
    frames(app, 3);
}

/// The middle of the topmost of the string's boxes in view: somewhere on
/// the worksheet's view to put a finger.
fn top_of_the_sheet(app: &Faraday) -> (u16, u16) {
    (1..=48)
        .filter_map(|b| {
            app.hit_box(Action::Codex32(C32::Cell(
                faraday_core::codex32::Cell::Given { sheet: 0, b },
            )))
        })
        .min_by_key(|b| b.1)
        .map(|(x, y, w, h)| (x + w / 2, y + h / 2))
        .expect("a box in view")
}

/// The character of Share A at `from` entered and the frames after it
/// drawn one at a time: where `place` stood after the key and after each
/// frame.
fn enter_then_watch(app: &mut Faraday, from: usize, place: impl Fn(&Faraday) -> f32) -> Vec<f32> {
    type_a(app, from..from + 1);
    let mut ys = vec![place(app)];
    let mut t = app.now_ms();
    for _ in 0..40 {
        t += 16;
        app.event(Event::Tick { now_ms: t });
        let _ = app.frame();
        ys.push(place(app));
    }
    ys
}

#[test]
fn an_entry_brings_a_dragged_view_back_to_the_cell_over_several_frames() {
    // 1280x800: the view pans. Twenty-two characters in, the view is at
    // the front; the person drags the sheet away, and the next character
    // sends the view back to the front over several frames.
    let mut app = typed_a(1280, 800, 22);
    let front = pan(&app);
    assert!(front.y > 0.0);
    let at = top_of_the_sheet(&app);
    drag_from(&mut app, at, 300);
    let away = pan(&app);
    assert!(away.y < front.y, "dragged up the sheet: {away:?} {front:?}");
    let ys = enter_then_watch(&mut app, 22, |a| pan(a).y);
    assert_eq!(ys[0], away.y, "nothing jumps at the key: {ys:?}");
    let to = *ys.last().unwrap();
    assert!(to > away.y + 20.0, "back toward the front: {ys:?}");
    let between = ys.iter().filter(|&&y| y > away.y && y < to).count();
    assert!(between >= 5, "several frames on the way: {ys:?}");

    // Under Reduce motion the view is back at once.
    let mut reduced = typed_a(1280, 800, 22);
    reduced.press(Action::ReduceMotion(true));
    let at = top_of_the_sheet(&reduced);
    drag_from(&mut reduced, at, 300);
    assert!(pan(&reduced).y < front.y);
    type_a(&mut reduced, 22..23);
    assert!(pan(&reduced).y > away.y + 20.0, "at once");
    assert!(!pan(&reduced).moving());
}

#[test]
fn an_entry_brings_a_scrolled_page_back_to_a_whole_sheets_cell() {
    // 1920x1080: the sheet is whole and the page follows it. Scrolled away
    // by a drag, the next character brings the page back over several
    // frames, at once under Reduce motion.
    let mut app = typed_a(1920, 1080, 22);
    let front = page_y(&app);
    assert!(front > 0.0);
    drag_from(&mut app, (900, 300), 700);
    let away = page_y(&app);
    assert!(away < front, "the page was dragged away: {away} {front}");
    let ys = enter_then_watch(&mut app, 22, page_y);
    let to = *ys.last().unwrap();
    assert!(to > away + 20.0, "the page came back: front {front} {ys:?}");
    assert_eq!(ys[0], away, "nothing jumps at the key: {ys:?}");
    let between = ys.iter().filter(|&&y| y > away && y < to).count();
    assert!(between >= 3, "several frames on the way: {ys:?}");

    let mut reduced = typed_a(1920, 1080, 22);
    reduced.press(Action::ReduceMotion(true));
    drag_from(&mut reduced, (900, 300), 700);
    assert!(page_y(&reduced) < front);
    type_a(&mut reduced, 22..23);
    assert!(page_y(&reduced) > away + 20.0, "at once");
}

/// The top edge of the pinned input panel: the topmost of the codex32
/// keyboard's keys in view.
fn panel_top(app: &Faraday) -> u16 {
    "qwertyuiopasdfghjklzxcvbnm023456789"
        .chars()
        .filter_map(|c| app.hit_box(Action::Codex32(C32::Key(OskPress::Char(c)))))
        .map(|b| b.1)
        .min()
        .expect("the keyboard is on screen")
}

#[test]
fn at_1280_by_800_the_page_brings_the_worksheet_above_the_pinned_panel() {
    // Opening the Enter card, and then each entry, leave the worksheet's
    // front cell above the panel at the window's foot.
    for n in [0usize, 1, 22] {
        let app = typed_a(1280, 800, n);
        let top = panel_top(&app);
        // Opened, the sheet's first cell; then the last box written.
        let front = if n == 0 {
            HRP
        } else {
            Action::Codex32(C32::Cell(faraday_core::codex32::Cell::Given {
                sheet: 0,
                b: (9 + n) as u8,
            }))
        };
        let at = app
            .hit_box(front)
            .unwrap_or_else(|| panic!("{n}: the last cell is in view"));
        assert!(
            at.1 + at.3 <= top,
            "{n}: cell {at:?} above the panel at {top}"
        );
    }
}
