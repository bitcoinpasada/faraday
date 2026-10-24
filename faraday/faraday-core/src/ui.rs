//! Drawing in design units: Faraday's drawing (`faraday-ui`) with the
//! desktop app's [`Action`] as what a press does, and the few pieces of
//! drawing that only the desktop app has ([`CoreUi`]).

use crate::Action;

pub use faraday_ui::pal;
pub use faraday_ui::{
    BAR_GRAB, BarGeometry, Contrast, Disclosure, Glyph, Mode, NameSuffix, Paint, Preset, Role,
    Scrolled, Slot, Style, Theme, ThemeGroup, ThemeName, W, btc, grouped, short, thousands,
};

/// What is written or drawn on a control that is not available now:
/// on-surface at the disabled opacity (`docs/THEMES.md` §8).
pub fn off() -> Paint {
    Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTENT)
}

/// A card, tile or row at `(x, y, w, h, r)` filled in `fill`, with
/// the pressed state layer over it while `pressed` (`docs/THEMES.md` §8).
pub fn fill_pressed(ui: &mut Ui, at: (f32, f32, f32, f32, f32), fill: Role, pressed: bool) {
    let (x, y, w, h, r) = at;
    ui.fill(x, y, w, h, r, fill);
    if pressed {
        ui.wash(
            x,
            y,
            w,
            h,
            r,
            Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
        );
    }
}

/// One frame's drawing context, its presses the desktop app's actions.
pub type Ui<'a> = faraday_ui::Ui<'a, Action>;

/// Where the caret and the selection are in the field typing goes to.
pub type Edit = faraday_ui::Edit<Action>;

/// Where a line of typing drew its characters.
pub type FieldAt = faraday_ui::FieldAt<Action>;

/// What only the desktop app draws: a field that may hold a secret, the
/// passphrase field's empty look, and the slider, whose stops are
/// [`Action::Slide`].
pub trait CoreUi {
    /// [`faraday_ui::Ui::fit`] for text that may be a secret: every
    /// shortened try is a [`SecretText`](crate::secret_text::SecretText),
    /// wiped as it goes.
    fn fit_secret(&self, size: f32, w: W, s: &str, max: f32) -> crate::secret_text::SecretText;

    /// A slider over the whole numbers `lo..=hi`, at `value`. Returns the
    /// height used.
    #[allow(clippy::too_many_arguments)]
    fn slider(&mut self, x: f32, y: f32, w: f32, id: u8, lo: u8, hi: u8, value: u8) -> f32;

    /// An empty BIP-39 passphrase field's content: the **No passphrase**
    /// pill, and the caret after it while the field is focused.
    fn no_passphrase(&mut self, x: f32, y: f32, h: f32, focused: bool);
}

impl CoreUi for Ui<'_> {
    /// [`Ui::fit`] for text that may be a secret: every shortened try is
    /// a [`SecretText`](crate::secret_text::SecretText), wiped as it goes.
    fn fit_secret(&self, size: f32, w: W, s: &str, max: f32) -> crate::secret_text::SecretText {
        use crate::secret_text::SecretText;
        if self.measure(size, w, s) <= max {
            return SecretText::of(s);
        }
        let mut t = SecretText::of(s);
        while t.pop().is_some() {
            t.push('…');
            if self.measure(size, w, &t) <= max {
                return t;
            }
            t.pop();
        }
        SecretText::new()
    }

    /// A slider over the whole numbers `lo..=hi`, at `value`: a track
    /// with a stop for each number and a round thumb carrying the value,
    /// the two ends' numbers under it. Each stop's share of the track
    /// presses as [`Action::Slide`]`(id, n)`; a finger held on it and
    /// dragged along presses each stop it crosses (`Faraday::touch`), and
    /// the arrow keys step the slider last pressed. Returns the height
    /// used.
    #[allow(clippy::too_many_arguments)]
    fn slider(&mut self, x: f32, y: f32, w: f32, id: u8, lo: u8, hi: u8, value: u8) -> f32 {
        let (lo, hi) = (lo.min(hi), hi.max(lo));
        let value = value.clamp(lo, hi);
        // Room at each end for the thumb.
        let pad = 14.0;
        let track = (w - 2.0 * pad).max(1.0);
        let gap = if hi > lo {
            track / f32::from(hi - lo)
        } else {
            0.0
        };
        let at = |n: u8| x + pad + gap * f32::from(n - lo);
        let mid = y + 20.0;
        let held = matches!(self.pressed, Some(Action::Slide(i, _)) if i == id);
        self.fill(
            x + pad,
            mid - 2.0,
            track,
            4.0,
            2.0,
            Role::SurfaceContainerHighest,
        );
        let vx = at(value);
        self.fill(x + pad, mid - 2.0, vx - x - pad, 4.0, 2.0, Role::Primary);
        for n in lo..=hi {
            if n != value {
                let c = if n < value {
                    Role::Primary
                } else {
                    Role::OutlineVariant
                };
                self.dot(at(n), mid, 3.0, c);
            }
        }
        let r = if held { 15.0 } else { 13.0 };
        let bg = if held {
            Paint::from(self.col(Role::Primary).mix(self.col(Role::OnSurface), 0.12))
        } else {
            Role::Primary.paint()
        };
        self.dot(vx, mid, r, bg);
        let v = value.to_string();
        let vw = self.measure(12.0, W::S, &v);
        self.text_mid(
            vx - vw / 2.0,
            mid - 10.0,
            20.0,
            12.0,
            W::S,
            Role::OnPrimary,
            &v,
        );
        let (l, h) = (lo.to_string(), hi.to_string());
        self.text(
            x + pad - self.measure(11.0, W::R, &l) / 2.0,
            y + 38.0,
            11.0,
            W::R,
            Role::OnSurfaceVariant,
            &l,
        );
        self.text(
            x + pad + track - self.measure(11.0, W::R, &h) / 2.0,
            y + 38.0,
            11.0,
            W::R,
            Role::OnSurfaceVariant,
            &h,
        );
        // Each stop takes the presses nearest it, the ends out to the
        // slider's edges.
        for n in lo..=hi {
            let left = if n == lo { x } else { at(n) - gap / 2.0 };
            let right = if n == hi { x + w } else { at(n) + gap / 2.0 };
            self.hit_quiet(left, y, right - left, 40.0, Action::Slide(id, n));
        }
        56.0
    }

    /// An empty BIP-39 passphrase field's content
    /// (`docs/history/NEW-WALLET.md` §13.4): the **No passphrase** pill at the
    /// start of the box `h` tall at (x, y), and the caret after it while
    /// the field is focused.
    fn no_passphrase(&mut self, x: f32, y: f32, h: f32, focused: bool) {
        let ph = 24.0;
        let label = crate::strings::EN.passphrase_none;
        let pw = self.pill_in(
            x + 8.0,
            y + (h - ph) / 2.0,
            ph,
            label,
            Role::OnSurfaceVariant,
        );
        if focused {
            self.caret(x + 14.0 + pw, y + (h - 18.0) / 2.0, 18.0);
        }
    }
}
