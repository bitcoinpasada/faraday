//! The keyboard: evdev key codes to [`osk_shell_api::Key`].
//!
//! One US layout, in one table. The core's keyboards need the BIP-39
//! letters, the digits, space and hyphen for a wallet name, and the
//! printable set a passphrase is typed from; the table carries the whole
//! of a US keyboard's printable range, so every one of those arrives
//! whichever screen asks for it. Enter, Backspace, Escape, Tab and the
//! four arrows are the rest; every other key is dropped.
//!
//! There is no layout file and no XKB: a device with no package manager
//! has one layout, and a person who needs another types the words with
//! the on-screen keyboard, which is the input every screen is designed
//! for. The physical keyboard is an accelerator (`docs/UX.md` §2).
//!
//! Caps Lock is ignored — only Shift changes a letter's case — and the
//! kernel's auto-repeat (`EV_KEY` value 2) repeats Backspace, Tab and
//! the arrows and nothing else, which is what the desktop shell does
//! with a held key: holding Down walks a list, and holding a letter
//! down should not type it fifty times into a seed word.
//!
//! A key coming up is reported as well as a key going down, because a
//! hold is held with Enter the way it is held with a finger
//! (`docs/DESIGN.md` §4.15).

use osk_shell_api::Key;

use crate::evdev::{EV_KEY, RawEvent};

/// `KEY_ESC`.
const KEY_ESC: u16 = 1;
/// `KEY_BACKSPACE`.
const KEY_BACKSPACE: u16 = 14;
/// `KEY_DELETE`.
const KEY_DELETE: u16 = 111;
/// `KEY_TAB`.
const KEY_TAB: u16 = 15;
/// `KEY_ENTER`.
const KEY_ENTER: u16 = 28;
/// `KEY_LEFTSHIFT`.
const KEY_LEFTSHIFT: u16 = 42;
/// `KEY_RIGHTSHIFT`.
const KEY_RIGHTSHIFT: u16 = 54;
/// `KEY_KPENTER`, the keypad's own Enter.
const KEY_KPENTER: u16 = 96;

/// The keypad's digits, `KEY_KP0` to `KEY_KP9`, in the order 0 to 9.
/// Num Lock is not read: a PIN pad and a passphrase both want the digit
/// the key is printed with.
const KEYPAD_DIGITS: [u16; 10] = [82, 79, 80, 81, 75, 76, 77, 71, 72, 73];
/// `KEY_UP`.
const KEY_UP: u16 = 103;
/// `KEY_LEFT`.
const KEY_LEFT: u16 = 105;
/// `KEY_RIGHT`.
const KEY_RIGHT: u16 = 106;
/// `KEY_DOWN`.
const KEY_DOWN: u16 = 108;

/// The printable keys of a US layout: code, unshifted, shifted.
const PRINTABLE: &[(u16, char, char)] = &[
    (2, '1', '!'),
    (3, '2', '@'),
    (4, '3', '#'),
    (5, '4', '$'),
    (6, '5', '%'),
    (7, '6', '^'),
    (8, '7', '&'),
    (9, '8', '*'),
    (10, '9', '('),
    (11, '0', ')'),
    (12, '-', '_'),
    (13, '=', '+'),
    (16, 'q', 'Q'),
    (17, 'w', 'W'),
    (18, 'e', 'E'),
    (19, 'r', 'R'),
    (20, 't', 'T'),
    (21, 'y', 'Y'),
    (22, 'u', 'U'),
    (23, 'i', 'I'),
    (24, 'o', 'O'),
    (25, 'p', 'P'),
    (26, '[', '{'),
    (27, ']', '}'),
    (30, 'a', 'A'),
    (31, 's', 'S'),
    (32, 'd', 'D'),
    (33, 'f', 'F'),
    (34, 'g', 'G'),
    (35, 'h', 'H'),
    (36, 'j', 'J'),
    (37, 'k', 'K'),
    (38, 'l', 'L'),
    (39, ';', ':'),
    (40, '\'', '"'),
    (41, '`', '~'),
    (43, '\\', '|'),
    (44, 'z', 'Z'),
    (45, 'x', 'X'),
    (46, 'c', 'C'),
    (47, 'v', 'V'),
    (48, 'b', 'B'),
    (49, 'n', 'N'),
    (50, 'm', 'M'),
    (51, ',', '<'),
    (52, '.', '>'),
    (53, '/', '?'),
    (57, ' ', ' '),
];

/// The key a code stands for, with Shift held or not.
pub fn map(code: u16, shift: bool) -> Option<Key> {
    if let Some((_, plain, shifted)) = PRINTABLE.iter().find(|(c, _, _)| *c == code) {
        return Some(Key::Char(if shift { *shifted } else { *plain }));
    }
    if let Some(digit) = KEYPAD_DIGITS.iter().position(|c| *c == code) {
        return Some(Key::Char(char::from(b'0' + digit as u8)));
    }
    match code {
        KEY_ESC => Some(Key::Escape),
        // Delete as well: the fields have no cursor to delete after, and
        // with a field selected it empties it.
        KEY_BACKSPACE | KEY_DELETE => Some(Key::Backspace),
        KEY_TAB => Some(Key::Tab),
        KEY_ENTER | KEY_KPENTER => Some(Key::Enter),
        KEY_UP => Some(Key::Up),
        KEY_DOWN => Some(Key::Down),
        KEY_LEFT => Some(Key::Left),
        KEY_RIGHT => Some(Key::Right),
        _ => None,
    }
}

/// A key going down — a press, or the kernel repeating one — or coming
/// up again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stroke {
    /// The key is down.
    Down(Key),
    /// The key is up again.
    Up(Key),
}

/// Whether a key held down repeats: Backspace deletes character after
/// character, and Tab and the arrows walk a list while they are held.
fn repeats(key: Key) -> bool {
    matches!(
        key,
        Key::Backspace | Key::Tab | Key::BackTab | Key::Up | Key::Down | Key::Left | Key::Right
    )
}

/// One keyboard's state: which Shift keys are held.
#[derive(Debug, Default)]
pub struct Keyboard {
    left_shift: bool,
    right_shift: bool,
}

impl Keyboard {
    /// Feeds one raw event; returns the stroke it delivers.
    ///
    /// The kernel's `EV_KEY` values are 0 for a release, 1 for a press
    /// and 2 for auto-repeat.
    pub fn feed(&mut self, ev: RawEvent) -> Option<Stroke> {
        if ev.kind != EV_KEY {
            return None;
        }
        match ev.code {
            KEY_LEFTSHIFT => {
                self.left_shift = ev.value != 0;
                return None;
            }
            KEY_RIGHTSHIFT => {
                self.right_shift = ev.value != 0;
                return None;
            }
            _ => {}
        }
        let shift = self.left_shift || self.right_shift;
        Some(match ev.value {
            1 => Stroke::Down(self.shifted(ev.code, shift)?),
            2 => {
                let key = self.shifted(ev.code, shift)?;
                if !repeats(key) {
                    return None;
                }
                Stroke::Down(key)
            }
            // A key released while Shift is no longer held is the same
            // key: the release names what went down, so Shift's state is
            // read the same way and a mismatch costs nothing, since the
            // core acts on Enter and Space coming up and on nothing else.
            0 => Stroke::Up(self.shifted(ev.code, shift)?),
            _ => return None,
        })
    }

    /// The key a code stands for here: Shift+Tab is Back-Tab, and the
    /// rest is the table.
    fn shifted(&self, code: u16, shift: bool) -> Option<Key> {
        match map(code, shift)? {
            Key::Tab if shift => Some(Key::BackTab),
            other => Some(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: u16) -> RawEvent {
        RawEvent {
            time_ms: 0,
            kind: EV_KEY,
            code,
            value: 1,
        }
    }

    fn release(code: u16) -> RawEvent {
        RawEvent {
            time_ms: 0,
            kind: EV_KEY,
            code,
            value: 0,
        }
    }

    fn repeat(code: u16) -> RawEvent {
        RawEvent {
            time_ms: 0,
            kind: EV_KEY,
            code,
            value: 2,
        }
    }

    #[test]
    fn typing_on_a_us_keyboard_reaches_the_core() {
        let mut kb = Keyboard::default();
        assert_eq!(kb.feed(press(30)), Some(Stroke::Down(Key::Char('a'))));
        assert_eq!(kb.feed(release(30)), Some(Stroke::Up(Key::Char('a'))));

        assert_eq!(kb.feed(press(KEY_LEFTSHIFT)), None);
        assert_eq!(kb.feed(press(30)), Some(Stroke::Down(Key::Char('A'))));
        assert_eq!(kb.feed(press(3)), Some(Stroke::Down(Key::Char('@'))));
        assert_eq!(kb.feed(release(KEY_LEFTSHIFT)), None);
        assert_eq!(kb.feed(press(3)), Some(Stroke::Down(Key::Char('2'))));

        assert_eq!(kb.feed(press(12)), Some(Stroke::Down(Key::Char('-'))));
        assert_eq!(kb.feed(press(57)), Some(Stroke::Down(Key::Char(' '))));
        assert_eq!(kb.feed(press(KEY_ENTER)), Some(Stroke::Down(Key::Enter)));
        assert_eq!(
            kb.feed(press(KEY_BACKSPACE)),
            Some(Stroke::Down(Key::Backspace))
        );
        assert_eq!(
            kb.feed(press(KEY_DELETE)),
            Some(Stroke::Down(Key::Backspace))
        );
        assert_eq!(kb.feed(press(KEY_ESC)), Some(Stroke::Down(Key::Escape)));
        assert_eq!(kb.feed(press(KEY_TAB)), Some(Stroke::Down(Key::Tab)));
        assert_eq!(kb.feed(press(KEY_DOWN)), Some(Stroke::Down(Key::Down)));
        assert_eq!(kb.feed(press(KEY_RIGHT)), Some(Stroke::Down(Key::Right)));

        // The keypad types a PIN as the top row does.
        for (code, digit) in KEYPAD_DIGITS.iter().zip('0'..='9') {
            assert_eq!(kb.feed(press(*code)), Some(Stroke::Down(Key::Char(digit))));
        }
        assert_eq!(kb.feed(press(KEY_KPENTER)), Some(Stroke::Down(Key::Enter)));
    }

    #[test]
    fn a_held_letter_types_once_and_a_held_backspace_keeps_deleting() {
        let mut kb = Keyboard::default();
        assert_eq!(kb.feed(press(30)), Some(Stroke::Down(Key::Char('a'))));
        assert_eq!(kb.feed(repeat(30)), None);
        assert_eq!(kb.feed(repeat(30)), None);

        assert_eq!(
            kb.feed(press(KEY_BACKSPACE)),
            Some(Stroke::Down(Key::Backspace))
        );
        assert_eq!(
            kb.feed(repeat(KEY_BACKSPACE)),
            Some(Stroke::Down(Key::Backspace))
        );
        assert_eq!(
            kb.feed(repeat(KEY_BACKSPACE)),
            Some(Stroke::Down(Key::Backspace))
        );
    }

    /// Holding Down walks a list and holding Tab walks the focus, so the
    /// kernel's repeat goes through for Tab and the four arrows.
    #[test]
    fn a_held_tab_or_arrow_keeps_moving_the_focus() {
        let mut kb = Keyboard::default();
        for (code, key) in [
            (KEY_TAB, Key::Tab),
            (KEY_UP, Key::Up),
            (KEY_DOWN, Key::Down),
            (KEY_LEFT, Key::Left),
            (KEY_RIGHT, Key::Right),
        ] {
            assert_eq!(kb.feed(press(code)), Some(Stroke::Down(key)));
            assert_eq!(kb.feed(repeat(code)), Some(Stroke::Down(key)));
            assert_eq!(kb.feed(release(code)), Some(Stroke::Up(key)));
        }
    }

    /// Enter coming up ends a hold, so a release is reported as a
    /// release and not as another press, and Shift+Tab is its own key.
    #[test]
    fn enter_is_reported_going_down_and_coming_up_and_shift_tab_goes_back() {
        let mut kb = Keyboard::default();
        assert_eq!(kb.feed(press(KEY_ENTER)), Some(Stroke::Down(Key::Enter)));
        assert_eq!(kb.feed(release(KEY_ENTER)), Some(Stroke::Up(Key::Enter)));

        assert_eq!(kb.feed(press(KEY_TAB)), Some(Stroke::Down(Key::Tab)));
        kb.feed(press(KEY_LEFTSHIFT));
        assert_eq!(kb.feed(press(KEY_TAB)), Some(Stroke::Down(Key::BackTab)));
    }

    #[test]
    fn keys_the_app_has_no_use_for_are_dropped() {
        let mut kb = Keyboard::default();
        // KEY_F1, KEY_CAPSLOCK, KEY_LEFTCTRL.
        for code in [59, 58, 29] {
            assert_eq!(kb.feed(press(code)), None);
        }
        // Caps Lock does not change a letter's case.
        assert_eq!(kb.feed(press(30)), Some(Stroke::Down(Key::Char('a'))));
    }
}
