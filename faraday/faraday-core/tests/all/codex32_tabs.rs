//! Make codex32 shares' tabs (`docs/CODEX32.md` §6.4.7, §6.5): each tab
//! keeps where its page was scrolled to. A person who scrolls a tab,
//! looks at another and comes back finds the same place, on the Pi's
//! panel (the entry's Dice, Checksum and String) and on a desktop window
//! (the worksheets' review).

use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_ui::keyboard::OskPress;

/// A string drawn and the box of its ink, pixels (x, y, w, h).
type DrawnText = (String, (i32, i32, i32, i32));

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// BIP 93's vector 2, shares A and C (`tools/vectors/bip93/vectors.json`).
const SHARES: [&str; 2] = [
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
];

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
    // Every move made at once: what a frame shows is where it rests.
    app.press(Action::ReduceMotion(true));
    app
}

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
    app.settle();
}

fn key(app: &mut Faraday, ch: char) {
    app.press(Action::Codex32(C32::Key(OskPress::Char(ch))));
}

/// Make codex32 shares, 2 of 3 named "name", its entry open on `way`.
fn entry(app: &mut Faraday, way: Way) {
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStep(kstep::START));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let two = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(app, C32::Split(two as u8));
    c32(app, C32::Next);
    for ch in "name".chars() {
        key(app, ch);
    }
    c32(app, C32::Next);
    let w = WAYS.iter().position(|w| *w == way).unwrap();
    c32(app, C32::Way(w as u8));
    c32(app, C32::Next);
    assert_eq!(open(app), Some(Card::Enter));
}

fn open(app: &Faraday) -> Option<Card> {
    app.keygen()
        .and_then(|k| k.codex32.as_ref())
        .and_then(|m| m.open())
}

/// Character `ch` rolled on the booklet's sheet: each die's bit 0 as 4
/// then 3, 1 as 3 then 4.
fn roll(app: &mut Faraday, ch: u8) {
    let v = CHARSET.iter().position(|&x| x == ch).unwrap() as u8;
    let bits: Vec<u8> = (0..5).map(|d| v >> (4 - d) & 1).collect();
    for &b in &bits {
        key(app, if b == 0 { '4' } else { '3' });
    }
    for &b in &bits {
        key(app, if b == 0 { '3' } else { '4' });
    }
}

/// Where the page is scrolled to, and what it shows there.
fn place(app: &mut Faraday) -> (f32, Vec<DrawnText>) {
    let _ = app.frame();
    let y = app.keygen().map_or(0.0, |k| k.scroll.y);
    (y, app.drawn_text_boxes())
}

/// The page scrolled by the wheel at (`x`, `y`), then tab `away` shown
/// and tab `back` shown again: the same place shows as before `away`.
fn comes_back_to_the_same_place(
    app: &mut Faraday,
    (x, y, dy): (u16, u16, i16),
    away: C32,
    back: C32,
) {
    let (start, _) = place(app);
    for _ in 0..3 {
        app.event(Event::Scroll { x, y, dy });
        app.settle();
    }
    let (scrolled, seen) = place(app);
    assert!(
        (scrolled - start).abs() > 1.0,
        "the wheel scrolls the page: {start} then {scrolled}"
    );
    c32(app, away);
    let _ = app.frame();
    c32(app, back);
    let (again, shown) = place(app);
    assert_eq!(again, scrolled, "back on the tab, the page is where it was");
    assert_eq!(shown, seen, "back on the tab, the same place shows");
}

#[test]
fn on_the_pi_the_entrys_tabs_each_keep_their_place() {
    let mut app = on(480, 640, 286);
    entry(&mut app, Way::Dice);
    for &ch in &SHARES[0].as_bytes()[9..20] {
        roll(&mut app, ch);
    }
    // Each tab seen once, then Dice, scrolled up from the pad, then
    // Checksum and back.
    for p in [1, 2, 0] {
        c32(&mut app, C32::Panel(p));
    }
    comes_back_to_the_same_place(&mut app, (240, 300, -120), C32::Panel(1), C32::Panel(0));
    // Checksum, scrolled, then String and back.
    c32(&mut app, C32::Panel(1));
    comes_back_to_the_same_place(&mut app, (240, 100, -120), C32::Panel(2), C32::Panel(1));
    // And Dice still where it was left.
    let (dice_y, _) = {
        c32(&mut app, C32::Panel(0));
        place(&mut app)
    };
    c32(&mut app, C32::Panel(2));
    c32(&mut app, C32::Panel(0));
    assert_eq!(place(&mut app).0, dice_y);
}

#[test]
fn on_a_desktop_the_reviews_tabs_each_keep_their_place() {
    let mut app = on(1366, 768, 160);
    entry(&mut app, Way::Typed);
    for s in SHARES {
        for &ch in &s.as_bytes()[9..35] {
            key(&mut app, char::from(ch));
        }
    }
    c32(&mut app, C32::Next);
    assert_eq!(open(&app), Some(Card::Sheets));
    // Each tab seen once, so its mark is there before and after.
    for t in [1, 0] {
        c32(&mut app, C32::Tab(t));
    }
    comes_back_to_the_same_place(&mut app, (700, 500, 120), C32::Tab(1), C32::Tab(0));
}
