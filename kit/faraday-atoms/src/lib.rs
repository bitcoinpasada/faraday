//! The kit's atoms level: faraday-ui's lib.rs, controls, press, region, hold, mark.
//! Section 09 §3.2; `Ui` and the frame's output are 04 §7.1.

// The derive names `::faraday_organisms::Pressed`, which a feature's crate has; here it is this crate.
#[cfg(test)]
extern crate self as faraday_organisms;
#[cfg(test)]
mod derive_tests;

pub mod badge;
pub mod button;
pub mod check;
pub mod chip;
pub mod dimmed;
pub mod fab;
pub mod field;
pub mod focus;
pub mod frame_out;
pub mod frost;
pub mod hits;
pub mod hold;
pub mod icon_button;
pub mod keeps;
pub mod line;
pub mod mark;
pub mod measured;
pub mod motion;
pub mod offered;
pub mod pill;
pub mod progress;
pub mod region;
pub mod rule;
pub mod segmented;
pub mod shadow;
pub mod shown;
pub mod stepper;
pub mod switch;
pub mod tag;
pub mod ui;

pub use faraday_atoms_derive::Pressed;
pub use faraday_draw::icons::Icon;
pub use faraday_port::values::Anchor;
pub use faraday_tokens::{TextSize, Theme};

pub use button::{Look, Style};
pub use focus::step as focus_step;
pub use frame_out::{FrameOut, Laid, Placement};
pub use hits::{Hit, Plain, PressMark, Ring, Shape, Unpressed, hit_at};
pub use icon_button::TipAnchor;
pub use keeps::{KeptFrost, KeptRows, MadeFrost, MadeRows};
pub use motion::{ComponentId, Motion, MotionStore, Pan, Turn};
pub use offered::{Drawn, Offered};
pub use region::Scrolled;
pub use shown::{PlainRef, Shown, btc, grouped, short, thousands};
pub use ui::{Glyph, Pointer, Ui, W};

/// A press: copied into the frame's hit list, compared for hover, focus and the pressed layer.
/// Not `Public`: a typed character is a press (04 §2).
pub trait Pressed: Clone + PartialEq + Send + 'static {
    /// Given by `#[derive(Pressed)]` from the `#[hides]` marks (04 §11.1).
    fn hides(&self) -> bool;
    /// The variant's name with its crate, such as "faraday-power.Away", where a variant holds
    /// another press the one it holds (07 §5.1). Given by `#[derive(Pressed)]`.
    fn kind(&self) -> &'static str {
        "Press"
    }
}

/// What `#[derive(Pressed)]` writes to name a press by the variant that holds it: a variant
/// holding one value that is itself a press names that press's kind, any other names itself.
pub mod kind_probe {
    use super::Pressed;

    pub struct Probe<'a, T>(pub &'a T);

    /// Chosen for a value that is a press.
    pub trait Deeper {
        fn kind_of(&self, own: &'static str) -> &'static str;
    }

    impl<T: Pressed> Deeper for Probe<'_, T> {
        fn kind_of(&self, _own: &'static str) -> &'static str {
            self.0.kind()
        }
    }

    /// Chosen for any other value, by one more step of auto-reference.
    pub trait Leaf {
        fn kind_of(&self, own: &'static str) -> &'static str;
    }

    impl<T> Leaf for &Probe<'_, T> {
        fn kind_of(&self, own: &'static str) -> &'static str {
            own
        }
    }
}
