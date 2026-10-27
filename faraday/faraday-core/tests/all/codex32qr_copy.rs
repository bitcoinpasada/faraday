//! A codex32 string's Codex32QR drawn by hand from the backup's checklist
//! (`docs/CODEX32.md` §9.5, `docs/BACKUP-FLOW.md` §17.24), as a person
//! copying it would notice: the Templates item names the Codex32QR sheet;
//! a string of the key's set, typed from its paper, shows its Codex32QR
//! as a grid marked block by block (a partial block at the edge, rows on
//! request); and a copy drawn on the sheet's grid, scanned back, is
//! checked against the string.

use faraday_bip::codex32::Codex32;
use faraday_core::board::{Thing, Zone};
use faraday_core::bstep;
use faraday_core::codex32_backup::{Found, Press, printed};
use faraday_core::plan::{self, At, What};
use faraday_core::{Action, Faraday, ScanPurpose, StorageEvent, testkit};
use faraday_keys::texts::Spec;
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};
use faraday_ui::keyboard::OskPress;

use crate::common::form;

/// A 16-byte master seed, not one of the public vectors (a public seed
/// would put the session in Practice), so its strings are 48 characters.
const SEED: [u8; 16] = [
    0x5a, 0x17, 0x93, 0x0c, 0xe2, 0x41, 0xb8, 0x6d, 0x24, 0xf1, 0x0e, 0x9b, 0x73, 0xc6, 0x38, 0xad,
];

/// [`SEED`]'s 2 of 3 set, identifier `cash`, by BIP 93's arithmetic.
fn set() -> Vec<String> {
    let s = Codex32::from_seed(2, *b"cash", &SEED).unwrap();
    s.split(2, 3, &[0x37; 64])
        .unwrap()
        .iter()
        .map(|x| x.encode().as_str().to_string())
        .collect()
}

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// [`SEED`]'s key loaded from two strings of its set, a single-key
/// wallet over it, Back up open on its standard way (one string per
/// place, on paper), customised so the first string's place keeps it only
/// as its Codex32QR drawn. Returns the app and the first string's index.
fn drawn_plan() -> (Faraday, usize) {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
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
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    for s in &set()[..2] {
        typed(&mut app, s);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    assert!(!app.session.keys.demo(), "a seed of its own");
    let c = testkit::compute();
    let text = app
        .session
        .keys
        .key_text(
            &c,
            0,
            &Spec::Single(faraday_bip::keys::ScriptType::NativeSegwit, 0),
        )
        .unwrap()
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            "shares-wallet.txt".into(),
            format!("wpkh({text}/<0;1>/*)").into_bytes(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Backup(0));
    let c = testkit::compute();
    let (shape, a) = app.board_plan().expect("the board");
    let m = plan::map(&c, &shape, &a);
    let held = plan::holds_at(&m, At::Place(0));
    let j = held
        .iter()
        .find_map(|w| match w {
            What::Codex32Share(0, j) => Some(*j),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a string at the first place: {held:?}"));
    // Drawn as its Codex32QR, not written.
    form(&mut app, Zone::Paper(0), Thing::String(0, j), 1);
    form(&mut app, Zone::Paper(0), Thing::String(0, j), 0);
    let (shape, a) = app.board_plan().unwrap();
    let held = plan::holds_at(&plan::map(&c, &shape, &a), At::Place(0));
    assert!(held.contains(&What::Codex32QrPaper(0, j)), "{held:?}");
    assert!(!held.contains(&What::Codex32Share(0, j)), "{held:?}");
    (app, j)
}

/// The string of [`set`] at index `j`.
fn string_at(j: usize) -> String {
    let index = plan::codex32::CHARSET[j];
    set()
        .into_iter()
        .find(|s| s.as_bytes()[8] == index)
        .unwrap()
}

/// Every text the last frame drew, one string, wrapped lines joined.
fn texts(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.drawn_texts().join(" ")
}

/// The checklist made, the blank templates marked printed, the codex32
/// item open on string `j`, typed from its paper.
fn typed_back(app: &mut Faraday, j: usize) {
    app.press(Action::BChecklist);
    if app.backup().unwrap().open == Some(bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    let n = bstep::of(plan::Item::Codex32(0));
    if app.backup().unwrap().open != Some(n) {
        app.press(Action::BStep(n));
    }
    app.press(Action::BC32(Press::Open(u8::try_from(j).unwrap())));
    assert!(
        app.backup().unwrap().c32.as_ref().unwrap().qr.is_none(),
        "nothing to draw until it is typed"
    );
    for ch in string_at(j)[3..].chars() {
        app.press(Action::BC32(Press::Key(OskPress::Char(ch))));
    }
}

/// `action`'s box once it is on screen, scrolling down to it.
fn seen(app: &mut Faraday, action: Action) -> (u16, u16, u16, u16) {
    for _ in 0..30 {
        let _ = app.frame();
        if let Some(b) = app.hit_box(action) {
            return b;
        }
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 120,
        });
    }
    panic!("{action:?} not drawn");
}

/// Whether `text` is drawn, scrolling down to it.
fn text_seen(app: &mut Faraday, text: &str) -> bool {
    for _ in 0..30 {
        if texts(app).contains(text) {
            return true;
        }
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 120,
        });
    }
    false
}

#[test]
fn the_templates_item_names_the_codex32qr_sheet() {
    let (mut app, _) = drawn_plan();
    let c = testkit::compute();
    let (shape, a) = app.board_plan().unwrap();
    let mapped = plan::Mapped::of(&c, &shape, &a);
    use plan::codex32::PaperCopy;
    let sheets = plan::codex32::templates_of(&a, &mapped);
    assert!(
        sheets.contains(&(48, PaperCopy::Codex32Qr, 1)),
        "{sheets:?}"
    );
    assert!(
        sheets
            .iter()
            .any(|&(l, p, _)| l == 48 && p == PaperCopy::String),
        "{sheets:?}"
    );
    app.press(Action::BChecklist);
    if app.backup().unwrap().open != Some(bstep::BLANK) {
        app.press(Action::BStep(bstep::BLANK));
    }
    assert!(text_seen(
        &mut app,
        "1 from Blank templates, codex32, 48 characters, Codex32QR"
    ));
}

#[test]
fn a_drawn_string_shows_its_grid_marked_by_blocks_and_rows() {
    let (mut app, j) = drawn_plan();
    typed_back(&mut app, j);
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    assert!(matches!(st.found, Some(Found::Holds(_))), "{:?}", st.found);
    assert!(st.qr.is_some(), "its Codex32QR, to draw");
    for line in [
        "Sheet · Faraday Starter, Blank templates, codex32, 48 characters, Codex32QR",
        "Grey · printed on the sheet",
        "Block A1",
    ] {
        assert!(text_seen(&mut app, line), "{line}");
    }
    // Scan my copy is offered.
    seen(&mut app, Action::BC32(Press::ScanCopy));
    // Blocks of five from the top left: A1 is 5 × 5, A6 at the right
    // edge 4 × 5, F6 in the corner 4 × 4.
    // B1, the second row's first block (0 and 1 are also Previous and
    // Next while A1 is marked).
    let a1 = seen(&mut app, Action::BC32(Press::BlockAt(6)));
    let a6 = seen(&mut app, Action::BC32(Press::BlockAt(5)));
    let f6 = seen(&mut app, Action::BC32(Press::BlockAt(35)));
    let cell = f32::from(a1.2) / 5.0;
    assert!((f32::from(a1.3) / cell - 5.0).abs() < 0.3, "{a1:?}");
    assert!((f32::from(a6.2) / cell - 4.0).abs() < 0.3, "{a6:?}");
    assert!((f32::from(a6.3) / cell - 5.0).abs() < 0.3, "{a6:?}");
    assert!((f32::from(f6.2) / cell - 4.0).abs() < 0.3, "{f6:?}");
    assert!((f32::from(f6.3) / cell - 4.0).abs() < 0.3, "{f6:?}");
    // A press on a block marks it.
    app.press(Action::BC32(Press::BlockAt(7)));
    assert!(text_seen(&mut app, "Block B2"));
    // Rows: each row a press across the whole grid.
    app.press(Action::BC32(Press::Rows(true)));
    let r5 = seen(&mut app, Action::BC32(Press::BlockAt(5)));
    let r28 = seen(&mut app, Action::BC32(Press::BlockAt(28)));
    assert!((f32::from(r5.3) / cell - 1.0).abs() < 0.3, "{r5:?}");
    assert!((f32::from(r5.2) / cell - 29.0).abs() < 0.5, "{r5:?}");
    assert_eq!(r5.0, r28.0);
    assert!(text_seen(&mut app, "row 0"));
}

/// The luma of a Codex32QR copied onto the Starter's sheet: the squares
/// the sheet prints, and every square the device's grid draws black,
/// four pixels a square in a white quiet zone of four squares.
fn drawn_copy(m: &faraday_codec::qr::QrMatrix) -> (usize, usize, Vec<u8>) {
    let n = m.size();
    let (scale, quiet) = (4, 4);
    let side = (n + 2 * quiet) * scale;
    let mut px = vec![255u8; side * side];
    for y in 0..n {
        for x in 0..n {
            // The grid draws grey only squares the code has dark.
            if printed(x, y) {
                assert!(m.module(x, y), "({x}, {y}) printed but light");
            }
            if !(printed(x, y) || m.module(x, y)) {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    px[((y + quiet) * scale + dy) * side + (x + quiet) * scale + dx] = 0;
                }
            }
        }
    }
    (side, side, px)
}

#[test]
fn a_copy_drawn_on_the_sheet_scanned_back_is_checked() {
    let (mut app, j) = drawn_plan();
    typed_back(&mut app, j);
    let c = testkit::compute();
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    let (w, h, px) = drawn_copy(st.qr.as_ref().unwrap());
    let found = faraday_codec::decode_luma(&c, w, h, &px);
    assert_eq!(found.len(), 1, "the drawn copy reads");
    assert_eq!(
        String::from_utf8_lossy(&found[0].bytes).to_lowercase(),
        string_at(j)
    );
    // The camera opens on the copy.
    seen(&mut app, Action::BC32(Press::ScanCopy));
    app.press(Action::BC32(Press::ScanCopy));
    assert_eq!(
        app.scan().map(|s| s.purpose),
        Some(ScanPurpose::CheckCodex32)
    );
    // Another string of the set is not this copy: the box is named.
    let other = set()
        .into_iter()
        .find(|s| *s != string_at(j))
        .unwrap()
        .to_uppercase();
    app.event(Event::Scanned {
        bytes: other.into_bytes(),
    });
    let note = app.scan().and_then(|s| s.note.clone()).unwrap_or_default();
    assert!(note.starts_with("Box 9 differs"), "{note}");
    // Not a codex32 string at all.
    app.event(Event::Scanned {
        bytes: b"hello".to_vec(),
    });
    let note = app.scan().and_then(|s| s.note.clone()).unwrap_or_default();
    assert_eq!(note, "Not a codex32 string");
    // The drawn copy holds: checked, and the camera closes.
    app.event(Event::Scanned {
        bytes: found[0].bytes.to_vec(),
    });
    assert!(app.scan().is_none(), "the camera closes");
    assert!(app.backup().unwrap().c32_checked.contains(&(0, j)));
    assert!(matches!(
        app.backup().unwrap().c32.as_ref().unwrap().found,
        Some(Found::Holds(_))
    ));
    // Nothing scanned was loaded.
    assert_eq!(app.session.keys.len(), 1);
}
