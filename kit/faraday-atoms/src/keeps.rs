//! What the runtime keeps between frames that is not a number (04 K16, V19): the frost under a
//! sheet. The kit is given the copies the runtime holds, read-only, and gives back the ones it
//! made this frame; it keeps none.

use faraday_port::values::Anchor;

use crate::Pressed;

/// A frosted page the runtime kept, lent to a frame: the one under `sheet`, made while `page`
/// was drawn.
#[derive(Clone, Copy)]
pub struct KeptFrost<'a> {
    pub sheet: Anchor,
    pub page: Anchor,
    pub pixels: &'a [u8],
}

/// A frosted page a frame made under `sheet` for the runtime to keep. The runtime moves the
/// pixels into a wiping buffer and refuses them when the frame showed a secret.
pub struct MadeFrost {
    pub sheet: Anchor,
    pub page: Anchor,
    pub pixels: Vec<u8>,
}

/// What a frame is given and gives back beyond its motion: grouped here so a child `Ui` carries
/// them as one.
pub(crate) struct Keeps<'f, A: Pressed> {
    /// Lent by the runtime.
    pub(crate) kept: &'f [KeptFrost<'f>],
    pub(crate) made: Vec<MadeFrost>,
    /// The presses of the holds drawn, and those of them that are full.
    pub(crate) holds: Vec<A>,
    pub(crate) held: Vec<A>,
}

impl<'f, A: Pressed> Keeps<'f, A> {
    pub(crate) fn new(kept: &'f [KeptFrost<'f>]) -> Self {
        Keeps {
            kept,
            made: Vec::new(),
            holds: Vec::new(),
            held: Vec::new(),
        }
    }
}
