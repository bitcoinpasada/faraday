//! What a check reads from the runtime (05 §9).

use faraday_atoms::{Anchor, ComponentId, Hit};
use faraday_draw::canvas::Ink;

use crate::root::Root;
use crate::runtime::Runtime;

const NO_PAGE: Anchor = Anchor { scope: "", n: 0 };

impl<R: Root> Runtime<R> {
    /// What the last frame shows and keeps, and what the last event did.
    pub fn seen(&self) -> Seen<'_> {
        let out = self.frame.last();
        Seen {
            ink: out.and_then(|out| out.ink.as_ref()),
            kept: self
                .frame
                .frosts
                .iter()
                .map(|k| KeptImage {
                    id: ComponentId(k.page, k.sheet),
                    taken_at: k.taken_at,
                    secret: k.secret,
                    pixels: k.pixels.expose().as_slice(),
                })
                .collect(),
            page: out.map_or(NO_PAGE, |out| out.page),
            sheets: out.map_or(&[], |out| out.sheets.as_slice()),
            marks: &self.marks,
        }
    }

    /// The press targets the last frame drew, in drawing order: what the tour root's touch-target
    /// walk reads (07 §5.2). Read-only, like `seen`.
    pub fn hits(&self) -> &[Hit<R::Press>] {
        self.frame.last().map_or(&[], |out| out.hits.as_slice())
    }

    /// Turn the ink record on (04 V21). The tour root's, the leak crate's and test support's
    /// only (clippy `disallowed-methods`).
    pub fn record_ink(&mut self, on: bool) {
        self.frame.ink = on;
    }
}

pub struct Seen<'a> {
    /// The last frame's `FrameOut::ink`; `None` unless the record is on.
    pub ink: Option<&'a Ink>,
    /// Every image the runtime keeps: a sheet's frost, a fade's frame.
    pub kept: Vec<KeptImage<'a>>,
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
