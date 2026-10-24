//! The codex32 tour: Make codex32 shares (`docs/CODEX32.md` §6, §7) from
//! New key's Start card through every card, rolled on the booklet's dice
//! sheet to BIP 93's vector 2 (`NAME`, 2 of 3): the scheme, the
//! identifier, the dice (twenty faces, a die's colour changed) and a
//! character part way on them, the entry part way with an equal pair struck, the small
//! panel's tiles, a translation worksheet, a cell's working, the review's
//! tabs, a string typed back with a box that differs, its Codex32QR
//! code, Strings made with Check on another device, and Lock in; then
//! on Add a key's codex32 form, Check a string's worksheet, Make the rest
//! from vector 2's A and C, and Add a share from vector 3's A, C and E.

use crate::tour::Tour;
use faraday_core::codex32::{C32, Cell, DIE_SIZES, OskPress, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, Screen};
use faraday_shell_api::App;

/// Vector 2's initial shares A and C (`tools/vectors/bip93/vectors.json`).
const SHARES: [&str; 2] = [
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
];
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
/// A die's place in `DIE_SIZES`, by its faces.
fn die(faces: u8) -> u8 {
    DIE_SIZES.iter().position(|&n| n == faces).unwrap_or(0) as u8
}

fn c(t: &mut Tour, p: C32) {
    t.press(Action::Codex32(p));
}

fn key(t: &mut Tour, ch: char) {
    c(t, C32::Key(OskPress::Char(ch)));
}

/// Rolls character `ch` on the booklet's sheet: each die's bit 0 as 5
/// then 2, 1 as 2 then 5.
fn roll(t: &mut Tour, ch: u8) {
    let v = CHARSET.iter().position(|&x| x == ch).unwrap_or(0) as u8;
    let bits: Vec<u8> = (0..5).map(|d| v >> (4 - d) & 1).collect();
    for &b in &bits {
        key(t, if b == 0 { '5' } else { '2' });
    }
    for &b in &bits {
        key(t, if b == 0 { '2' } else { '5' });
    }
}

/// The dice: twenty faces, and die 1 black (seven steps from red); on
/// the entry, part of a character on them, both rolls of every die and
/// die 3's pair equal and struck. Then taken back, and six faces again.
fn twenty_faced(t: &mut Tour) -> Result<(), String> {
    c(t, C32::Faces(die(20)));
    for _ in 0..7 {
        c(t, C32::Colour(0));
    }
    t.shot("codex32-entropy-dice")?;
    c(t, C32::Next);
    t.shot("codex32-enter-first")?;
    let faces = [17, 3, 20, 9, 12, 4, 15, 20, 11, 19];
    for f in faces {
        c(t, C32::Face(f));
    }
    t.shot("codex32-enter-d20")?;
    // The five tracks, and the pad of twenty faces under the sheets.
    for _ in 0..4 {
        crate::tour::scroll(t, 120);
    }
    t.shot("codex32-enter-d20-tracks")?;
    for _ in 0..6 {
        crate::tour::scroll(t, 240);
    }
    t.shot("codex32-enter-d20-pad")?;
    for _ in 0..10 {
        crate::tour::scroll(t, -240);
    }
    for _ in faces {
        c(t, C32::Key(OskPress::Back));
    }
    c(t, C32::Faces(die(6)));
    Ok(())
}

/// Make codex32 shares, 2 of 3 named `name`, on the booklet's dice
/// sheet with share A's first characters rolled: the worksheet as the
/// themes tour draws it. Costs nothing but presses.
pub(crate) fn worksheet(t: &mut Tour) {
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::MakeCodex32.index()));
    let two = SPLITS
        .iter()
        .position(|s| *s == Split::TwoOfThree)
        .unwrap_or(0);
    c(t, C32::Split(two as u8));
    c(t, C32::Next);
    for ch in "name".chars() {
        key(t, ch);
    }
    c(t, C32::Next);
    let dice = WAYS.iter().position(|w| *w == Way::Dice).unwrap_or(0);
    c(t, C32::Way(dice as u8));
    c(t, C32::Next);
    for &ch in &SHARES[0].as_bytes()[9..35] {
        roll(t, ch);
    }
}

pub(crate) fn codex32_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.shot("codex32-start")?;
    t.press(Action::KStart(Start::MakeCodex32.index()));
    let other = SPLITS.iter().position(|s| *s == Split::Other).unwrap_or(2);
    c(t, C32::Split(other as u8));
    c(t, C32::K(3));
    t.shot("codex32-scheme-other")?;
    let two = SPLITS
        .iter()
        .position(|s| *s == Split::TwoOfThree)
        .unwrap_or(0);
    c(t, C32::Split(two as u8));
    t.shot("codex32-scheme")?;
    c(t, C32::Next);
    for ch in "na".chars() {
        key(t, ch);
    }
    t.shot("codex32-identifier")?;
    for ch in "me".chars() {
        key(t, ch);
    }
    c(t, C32::Next);
    let dice = WAYS.iter().position(|w| *w == Way::Dice).unwrap_or(0);
    c(t, C32::Way(dice as u8));
    t.shot("codex32-entropy")?;
    twenty_faced(t)?;
    let a = SHARES[0].as_bytes();
    for &ch in &a[9..15] {
        roll(t, ch);
    }
    // Part way through a character: three first values, die 1's pair,
    // and die 2's pair equal, struck.
    for f in ["3", "5", "1", "2", "6", "4", "5"] {
        key(t, f.chars().next().unwrap_or('1'));
    }
    t.shot("codex32-enter-dice")?;
    c(t, C32::View(1));
    t.shot("codex32-enter-share-d")?;
    c(t, C32::View(0));
    for p in 0..3 {
        c(t, C32::Panel(p));
        t.shot(&format!("codex32-enter-panel-{p}"))?;
    }
    c(t, C32::Panel(0));
    for _ in 0..7 {
        c(t, C32::Key(OskPress::Back));
    }
    for &ch in &a[15..35] {
        roll(t, ch);
    }
    t.shot("codex32-enter-share-a-done")?;
    c(t, C32::Cell(Cell::Given { sheet: 0, b: 40 }));
    t.shot("codex32-cell-checksum")?;
    c(
        t,
        C32::Cell(Cell::Lookup {
            sheet: 0,
            j: 3,
            i: 2,
        }),
    );
    t.shot("codex32-cell-lookup")?;
    c(t, C32::Cell(Cell::Given { sheet: 0, b: 12 }));
    t.shot("codex32-cell-dice")?;
    c(t, C32::CellClose);
    for &ch in &SHARES[1].as_bytes()[9..35] {
        roll(t, ch);
    }
    t.shot("codex32-enter-done")?;
    c(t, C32::Next);
    for tab in 0..6 {
        c(t, C32::Tab(tab));
        t.shot(&format!("codex32-sheets-{tab}"))?;
    }
    checksum_end(t)?;
    c(t, C32::Next);
    t.shot("codex32-string")?;
    let mut typed: Vec<char> = SHARES[0][3..].chars().collect();
    typed[13] = 'q';
    for ch in typed {
        key(t, ch);
    }
    t.shot("codex32-string-differs")?;
    c(t, C32::Qr);
    t.shot("codex32-string-qr")?;
    c(t, C32::Qr);
    c(t, C32::Skip);
    c(t, C32::Skip);
    c(t, C32::Skip);
    c(t, C32::Skip);
    c(t, C32::Skip);
    c(t, C32::Skip);
    t.shot("codex32-made")?;
    c(t, C32::Check);
    t.shot("codex32-made-check")?;
    c(t, C32::Lock);
    t.shot("codex32-locked")?;
    c(t, C32::Next);
    typed_in(t)
}

/// Vector 3's shares A, C and E (k = 3).
const CASH: [&str; 3] = [
    "ms13casha320zyxwvutsrqpnmlkjhgfedca2a8d0zehn8a0t",
    "ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr",
    "ms13casheekgpemxzshcrmqhaydlp6yhms3ws7320xyxsar9",
];

/// Add a key's codex32 form, open and empty.
fn form(t: &mut Tour) {
    let at = faraday_core::forms::Form::ALL
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Codex32)
        .unwrap_or(2);
    t.press(Action::Entry(None));
    t.press(Action::EntryForm(at as u8));
}

/// The small panel's keyboard put away, so the form under it is seen;
/// nothing where the window has none up.
fn hide_keyboard(t: &mut Tour) {
    t.press(Action::Osk(faraday_core::OskPress::Hide));
}

/// One string typed into the codex32 form and Enter.
fn string_in(t: &mut Tour, s: &str) {
    crate::tour::type_text(t, s);
    t.app
        .event(faraday_shell_api::Event::Key(faraday_shell_api::Key::Enter));
    t.pump();
}

/// Check a string's worksheet (§8.3), Make the rest (§8.1) and Add a
/// share (§8.2) from strings typed into Add a key.
fn typed_in(t: &mut Tour) -> Result<(), String> {
    use faraday_core::forms::C32Press;
    form(t);
    t.press(Action::Codex32Form(C32Press::Check));
    let mut bad = SHARES[0].as_bytes().to_vec();
    bad[19] = if bad[19] == b'q' { b'p' } else { b'q' };
    string_in(t, &String::from_utf8_lossy(&bad));
    hide_keyboard(t);
    t.shot("codex32-check-final-row")?;
    t.press(Action::Codex32Form(C32Press::Sheet));
    hide_keyboard(t);
    t.shot("codex32-check-worksheet")?;
    t.press(Action::Codex32Form(C32Press::Check));
    string_in(t, SHARES[0]);
    t.press(Action::Codex32Form(C32Press::Rest));
    hide_keyboard(t);
    t.shot("codex32-form-make-the-rest")?;
    string_in(t, SHARES[1]);
    t.shot("codex32-rest-scheme")?;
    c(t, C32::N(1));
    c(t, C32::Next);
    t.shot("codex32-rest-sheets")?;
    c(t, C32::Next);
    t.shot("codex32-rest-string")?;
    form(t);
    string_in(t, CASH[0]);
    t.press(Action::Codex32Form(C32Press::AddShare));
    string_in(t, CASH[1]);
    string_in(t, CASH[2]);
    t.shot("codex32-add-index")?;
    key(t, 'f');
    c(t, C32::Next);
    t.shot("codex32-add-sheets")?;
    c(t, C32::Next);
    t.shot("codex32-add-string")?;
    c(t, C32::Skip);
    c(t, C32::Skip);
    t.shot("codex32-add-made")
}

/// Share A's checksum worksheet dragged to its foot and right: the pink
/// checksum cells and the final row (`docs/CODEX32.md` §7.5).
fn checksum_end(t: &mut Tour) -> Result<(), String> {
    c(t, C32::Tab(2));
    pan_to_end(t);
    crate::tour::scroll(t, 360);
    t.shot("codex32-sheets-checksum-end")
}

/// The checksum worksheet shown dragged up and to the left, a finger's
/// drag at a time from near its top left corner, until its foot and its
/// right edge are in view.
fn pan_to_end(t: &mut Tour) {
    use faraday_shell_api::{Event, TouchPhase};
    let _ = t.app.frame();
    let cell = Action::Codex32(C32::Cell(Cell::Hrp { sheet: 0, i: 0 }));
    let Some((x, y, w, h)) = t.app.hit_box(cell) else {
        return;
    };
    let (x, y) = (x + w / 2, y + h / 2);
    for _ in 0..12 {
        let (dx, dy) = (x.min(300), y.min(200));
        for (px, py, phase) in [
            (x, y, TouchPhase::Down),
            (x - dx / 2, y - dy / 2, TouchPhase::Move),
            (x - dx, y - dy, TouchPhase::Move),
            (x - dx, y - dy, TouchPhase::Up),
        ] {
            t.app.event(Event::Touch {
                x: px,
                y: py,
                phase,
            });
        }
        let _ = t.app.frame();
    }
}
