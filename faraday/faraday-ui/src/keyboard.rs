//! Faraday's on-screen keyboard: the key layouts
//! (`faraday_draw::widgets::keyboard`) drawn as Faraday's buttons, across the
//! foot of a small panel or a strip of the wide layout. A press is the
//! key a physical keyboard would send. The desktop app types with it on
//! the stick's touch panel.
//!
//! Moved here from `faraday-core`'s `compact.rs`.

use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::{self, KeyInput, KeyboardKind, Modifiers};

use crate::pal::*;
use crate::{Style, Ui};
use faraday_strings::EN;

/// A press on the keyboard, or on a bar's keyboard button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OskPress {
    /// A character key.
    Char(char),
    /// Backspace.
    Back,
    /// Done: Enter.
    Done,
    /// Shift.
    Shift,
    /// The symbol layer.
    Symbols,
    /// Bring the keyboard up.
    Show,
    /// Put it away.
    Hide,
}

/// What a press types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typed {
    /// A character.
    Char(char),
    /// Backspace.
    Backspace,
    /// Enter.
    Enter,
}

/// One row of keys, design units.
pub const KEY_ROW: f32 = 32.0;

/// The widest the keys grow in the wide layout, design units: past it
/// they sit centred in the strip.
const KEYS_MAX_W: f32 = 720.0;

/// The keyboard's own state: Shift and the symbol layer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Keys {
    /// Shift is on: the next letter is a capital.
    pub shift: bool,
    /// The symbol layer is up.
    pub symbols: bool,
}

impl Keys {
    /// A press: what it types, if anything. A letter turns Shift off.
    /// Show and Hide are the caller's.
    pub fn press(&mut self, p: OskPress) -> Option<Typed> {
        match p {
            OskPress::Char(c) => {
                if c.is_alphabetic() {
                    self.shift = false;
                }
                Some(Typed::Char(c))
            }
            OskPress::Back => Some(Typed::Backspace),
            OskPress::Done => Some(Typed::Enter),
            OskPress::Shift => {
                self.shift = !self.shift;
                None
            }
            OskPress::Symbols => {
                self.symbols = !self.symbols;
                None
            }
            OskPress::Show | OskPress::Hide => None,
        }
    }

    /// Shift off and the letters up, as a keyboard comes up fresh.
    pub fn reset(&mut self) {
        *self = Keys::default();
    }
}

/// The keyboard's height, design units.
pub fn height(kind: KeyboardKind) -> f32 {
    keyboard::rows(kind) as f32 * row(kind) + 8.0
}

/// One row of `kind`'s keys, design units: [`KEY_ROW`], or the least
/// the keyboard asks for ([`keyboard::min_row`]).
fn row(kind: KeyboardKind) -> f32 {
    KEY_ROW.max(keyboard::min_row(kind))
}

/// The keyboard of `kind`, across the panel from `x`, `w` wide, from
/// `y` down; on a wide layout (`compact` false) its keys no wider than
/// [`KEYS_MAX_W`], centred. Each key presses as `act` of its
/// [`OskPress`].
#[allow(clippy::too_many_arguments)]
pub fn keyboard<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    w: f32,
    y: f32,
    kind: KeyboardKind,
    keys: Keys,
    compact: bool,
    act: impl Fn(OskPress) -> A,
) {
    let rows = keyboard::rows(kind) as f32;
    ui.fill(x, y, w, rows * row(kind) + 8.0, 0.0, SIDEBAR);
    ui.fill(x, y, w, 1.0, 0.0, LINE);
    let kw = if compact { w } else { w.min(KEYS_MAX_W) };
    let kx = x + (w - kw) / 2.0;
    let area = faraday_draw::geom::Rect::new(
        (kx + 4.0) as i32,
        (y + 6.0) as i32,
        (kw - 4.0) as i32,
        (rows * row(kind)) as i32,
    );
    let ctx = faraday_draw::layout::LayoutCtx::new(
        faraday_draw::geom::Scale::new(160),
        faraday_draw::geom::SizeClass::Small,
    );
    let mods = Modifiers {
        shift: keys.shift,
        symbols: keys.symbols,
    };
    for cap in keyboard::keys(kind, area, &ctx, keyboard::ALL_KEYS, None, mods) {
        let r = cap.rect;
        let (kx, ky, kw, kh) = (r.x as f32, r.y as f32, r.w as f32 - 3.0, r.h as f32 - 3.0);
        let (label, style, press) = match cap.input {
            KeyInput::Char(' ') => (String::new(), Style::Secondary, OskPress::Char(' ')),
            KeyInput::Char(c) => (c.to_string(), Style::Secondary, OskPress::Char(c)),
            KeyInput::Backspace => (String::new(), Style::Secondary, OskPress::Back),
            KeyInput::Done => (EN.compact_done.to_string(), Style::Primary, OskPress::Done),
            // Drawn as an icon below: Faraday's fonts have no "⇧".
            KeyInput::Shift => (
                String::new(),
                if keys.shift {
                    Style::Primary
                } else {
                    Style::Secondary
                },
                OskPress::Shift,
            ),
            KeyInput::Symbols => (
                if keys.symbols { "abc" } else { "#+=" }.to_string(),
                Style::Secondary,
                OskPress::Symbols,
            ),
        };
        let style = if cap.enabled { style } else { Style::Disabled };
        ui.button(kx, ky, Some(kw), kh, &label, style, act(press));
        match cap.input {
            KeyInput::Backspace => ui.icon(
                kx + kw / 2.0 - 9.0,
                ky + kh / 2.0 - 9.0,
                18.0,
                Icon::Delete,
                12.0,
                TEXT,
            ),
            KeyInput::Char(' ') => ui.icon(
                kx + kw / 2.0 - 9.0,
                ky + kh / 2.0 - 9.0,
                18.0,
                Icon::Space,
                12.0,
                MUTED,
            ),
            KeyInput::Shift => ui.icon(
                kx + kw / 2.0 - 9.0,
                ky + kh / 2.0 - 9.0,
                18.0,
                Icon::ChevronUp,
                12.0,
                if keys.shift { ON_ACCENT } else { TEXT },
            ),
            _ => {}
        }
    }
}
