//! What to write's Clear the whole stick first (`docs/STICK-IDENTITY.md`
//! §4.2): zeros over every sector of the stick before it is made, for a
//! new stick (a whole write of a stick) and never with keep data or a Pi
//! card. Unticked at first; the time it adds is worked out from the
//! stick's size at a USB 2 stick's writing speed, so it errs long.

use crate::api::Mode;
use crate::decide::Target;
use crate::{Page, Starter};

/// A USB 2 stick's writing speed, in bytes a second, the clearing
/// pass's time is estimated at.
const BYTES_A_SECOND: u64 = 8_000_000;

/// The minutes clearing `bytes` takes at 8 MB/s, rounded up (a 32 GB
/// stick: 4000 s, 67 min).
pub fn minutes(bytes: u64) -> u64 {
    bytes.div_ceil(BYTES_A_SECOND).div_ceil(60)
}

impl Starter {
    /// Whether What to write offers Clear the whole stick first: a new
    /// stick, not keep data, not a card. A lookup.
    pub fn clear_first_offered(&self) -> bool {
        self.making
            .as_ref()
            .is_some_and(|m| m.target == Target::Stick && m.mode == Some(Mode::Whole))
    }

    /// Whether the write clears the stick first: offered and ticked. A
    /// lookup.
    pub fn clear_first_order(&self) -> bool {
        self.clear_first_offered() && self.making.as_ref().is_some_and(|m| m.clear_first)
    }

    /// The tick pressed on What to write.
    pub(crate) fn toggle_clear_first(&mut self) {
        if self.page != Page::Mode || !self.clear_first_offered() {
            return;
        }
        if let Some(m) = self.making.as_mut() {
            m.clear_first = !m.clear_first;
        }
    }
}
