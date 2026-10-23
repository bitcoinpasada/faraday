//! The codex32 tour: Make codex32 shares (`docs/CODEX32.md` §6, §7) from
//! New key's Start card through every card, rolled on the booklet's dice
//! sheet to BIP 93's vector 2 (`NAME`, 2 of 3): the scheme, the
//! identifier, the entry part way with an equal pair struck, the small
//! panel's tiles, a translation worksheet, a cell's working, the review's
//! tabs, a string typed back with a box that differs, its Codex32QR
//! code, Strings made with Check on another device, and Lock in.

use crate::tour::Tour;
use faraday_core::codex32::{C32, Cell, OskPress, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::Start;
use faraday_core::{Action, Screen};

/// Vector 2's initial shares A and C (`tools/vectors/bip93/vectors.json`).
const SHARES: [&str; 2] = [
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
];
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

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

pub(crate) fn codex32_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStart(Start::MakeCodex32.index()));
    t.shot("codex32-start")?;
    t.press(Action::KNext);
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
    c(t, C32::Next);
    t.shot("codex32-enter-first")?;
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
    t.shot("codex32-locked")
}
