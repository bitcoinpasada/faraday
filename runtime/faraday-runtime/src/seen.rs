//! What a check reads from the runtime (05 §9).

use faraday_atoms::{Anchor, ComponentId};
use faraday_draw::canvas::Ink;

use crate::root::Root;
use crate::runtime::Runtime;

impl<R: Root> Runtime<R> {
    /// What the last frame shows and keeps, and what the last event did.
    pub fn seen(&self) -> Seen<'_> {
        todo!()
    }

    /// Turn the ink record on (04 V21). The tour root's, the leak crate's and test support's
    /// only (clippy `disallowed-methods`).
    pub fn record_ink(&mut self, _on: bool) {
        todo!()
    }
}

pub struct Seen<'a> {
    /// The last frame's `FrameOut::ink`; `None` unless the record is on.
    pub ink: Option<&'a Ink>,
    /// Every image the runtime keeps: a sheet's frost, a fade's frame.
    pub kept: &'a [KeptImage<'a>],
    /// `FrameOut::page`, set by the screen template.
    pub page: Anchor,
    /// `FrameOut::sheets`, the lowest first.
    pub sheets: &'a [Anchor],
    /// For the last event.
    pub marks: &'a [Mark],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    LockBegun,
    Hidden,
    Hides,
    ModelDropped,
    ModelBuilt,
}

pub struct KeptImage<'a> {
    pub id: ComponentId,
    pub taken_at: u64,
    pub secret: bool,
    pub pixels: &'a [u8],
}
