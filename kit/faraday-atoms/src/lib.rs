//! The kit's atoms level: faraday-ui's lib.rs, controls, press, region, hold, mark.
//! Section 09 §3.2; `Ui` and the frame's output are 04 §7.1.

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

pub mod badge;
pub mod button;
pub mod check;
pub mod chip;
pub mod fab;
pub mod field;
pub mod focus;
pub mod frame_out;
pub mod frost;
pub mod hits;
pub mod hold;
pub mod icon_button;
pub mod line;
pub mod mark;
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

pub use button::Style;
pub use frame_out::{FrameOut, Laid, Placement};
pub use hits::Hit;
pub use motion::{ComponentId, Motion, MotionStore, Turn};
pub use offered::{Drawn, Offered};
pub use region::Scrolled;
pub use shown::{PlainRef, Shown};
pub use ui::{Pointer, Ui};

/// A press: copied into the frame's hit list, compared for hover, focus and the pressed layer.
/// Not `Public`: a typed character is a press (04 §2).
pub trait Pressed: Clone + PartialEq + Send + 'static {
    /// Given by `#[derive(Pressed)]` from the `#[hides]` marks (04 §11.1).
    fn hides(&self) -> bool;
}
