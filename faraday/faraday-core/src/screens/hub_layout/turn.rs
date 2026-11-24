//! A change of page: what it does under the Motion setting, and how long
//! it takes at the Speed (`docs/MOTION.md` §3.5). `fade.rs` draws it.

use crate::stick::settings_choice::{MotionSpeed, MotionStyle};
use faraday_draw::tokens;

/// How long a change of a flow's step takes to fade through: the old
/// card out over the first `motion::STEP_FADE_OUT` of its eased progress
/// to the page's surface, the new one in over the rest, and the frame
/// outside the card's place cross-fading over all of it
/// (`docs/FLOW-CONTAINER.md` §3); Material's medium 2. Not yet timed on
/// the Pi (F238): the owner's late-flow session times it.
pub(crate) const STEP_MS: u64 = tokens::MOTION_MEDIUM2;

/// What a change of page does under the motion settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    /// The two frames blend, eased.
    Crossfade,
    /// The old page goes out to the surface and the new one comes in.
    Through,
}

/// What a change of page does in `style` and how long it takes at
/// `speed`, in ms; `None` for a change shown at once.
pub(crate) fn page_turn(style: MotionStyle, speed: MotionSpeed) -> Option<(Kind, u64)> {
    let kind = match style {
        MotionStyle::Smooth => Kind::Through,
        MotionStyle::Crossfade => Kind::Crossfade,
        MotionStyle::Off => return None,
    };
    Some((kind, speed.ms()))
}
