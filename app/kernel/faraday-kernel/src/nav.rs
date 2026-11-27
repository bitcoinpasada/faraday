//! The one navigation every frame uses (06 §4.3).

use crate::message::Public;

/// Back, Forward, the page a flow was begun from and the resume on a tab.
// Spec: 09 §4.1 gives `Nav` as `/* 06 §4.3 */`; the fields are the plainest that hold its rules.
pub struct Nav<P: Copy + Eq> {
    shown: P,
    back: Vec<P>,
    forward: Vec<P>,
    begun: Vec<(P, P)>,
}

impl<P: Copy + Eq> Nav<P> {
    pub fn new(_home: P) -> Self {
        todo!()
    }

    /// The page left, for `Notice::Left`.
    pub fn open(&mut self, _p: P) -> Option<P> {
        todo!()
    }

    pub fn back(&mut self) -> Option<P> {
        todo!()
    }

    pub fn forward(&mut self) -> Option<P> {
        todo!()
    }

    pub fn shown(&self) -> P {
        todo!()
    }

    pub fn begun_from(&mut self, _flow: P, _from: P) {
        todo!()
    }

    /// The Starter's `Page::Demo` entries.
    pub fn drop_where(&mut self, _f: fn(&P) -> bool) {
        todo!()
    }
}

/// The frame sheets by kind (06 §4.4); the stack's order is `Ord`.
#[derive(Public, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameSheet {
    Lock,
    LockAsk,
    IdleWarn,
    NewInput,
    Devices,
    Power,
    Guide,
    Restart,
    Uninstall,
    LetThrough,
    How,
}

/// Which child holds the camera, recorded by each composite from its children's outs.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct Holder(pub u8);

pub struct Frame<P: Copy + Eq> {
    nav: Nav<P>,
    sheets: Vec<FrameSheet>,
    camera: Option<Holder>,
}
