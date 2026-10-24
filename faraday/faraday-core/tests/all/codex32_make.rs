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
    let k = KeyGen::new(None, Screen::Entry, false);
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
