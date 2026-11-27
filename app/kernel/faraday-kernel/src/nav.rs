//! The one navigation every frame uses (06 §4.3).

use crate::message::Public;

/// Back, Forward, the page a flow was begun from and the resume on a tab.
pub struct Nav<P: Copy + Eq> {
    shown: P,
    back: Vec<P>,
    forward: Vec<P>,
    begun: Vec<(P, P)>,
}

impl<P: Copy + Eq> Nav<P> {
    pub fn new(home: P) -> Self {
        Nav {
            shown: home,
            back: Vec::new(),
            forward: Vec::new(),
            begun: Vec::new(),
        }
    }

    /// The page left, for `Notice::Left`; none when `p` is already shown. What Forward would
    /// reopen is forgotten.
    pub fn open(&mut self, p: P) -> Option<P> {
        if p == self.shown {
            return None;
        }
        let left = self.shown;
        self.back.push(left);
        self.forward.clear();
        self.shown = p;
        Some(left)
    }

    /// Goes back one page; the page left, or none at the first page.
    pub fn back(&mut self) -> Option<P> {
        let to = self.back.pop()?;
        let left = self.shown;
        self.forward.push(left);
        self.shown = to;
        Some(left)
    }

    /// Reopens the page Back left; the page left, or none when Back left none.
    pub fn forward(&mut self) -> Option<P> {
        let to = self.forward.pop()?;
        let left = self.shown;
        self.back.push(left);
        self.shown = to;
        Some(left)
    }

    pub fn shown(&self) -> P {
        self.shown
    }

    /// Whether Back has a page to go to.
    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    /// Whether Forward has a page to reopen.
    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// The page behind the shown one: where Back goes.
    pub fn behind(&self) -> Option<P> {
        self.back.last().copied()
    }

    /// Records that `flow` was begun from `from`.
    pub fn begun_from(&mut self, flow: P, from: P) {
        self.begun.retain(|(f, _)| *f != flow);
        self.begun.push((flow, from));
    }

    /// The Starter's `Page::Demo` entries.
    pub fn drop_where(&mut self, f: fn(&P) -> bool) {
        self.back.retain(|p| !f(p));
        self.forward.retain(|p| !f(p));
        self.begun.retain(|(flow, from)| !f(flow) && !f(from));
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

impl<P: Copy + Eq> Frame<P> {
    /// A frame showing `home`, with no sheet and no camera holder.
    pub fn new(home: P) -> Self {
        Frame {
            nav: Nav::new(home),
            sheets: Vec::new(),
            camera: None,
        }
    }

    pub fn nav(&self) -> &Nav<P> {
        &self.nav
    }

    pub fn nav_mut(&mut self) -> &mut Nav<P> {
        &mut self.nav
    }

    /// The sheets on show, the top one first (the order of `FrameSheet`).
    pub fn sheets(&self) -> &[FrameSheet] {
        &self.sheets
    }

    /// `Out::Front`: a sheet comes up in its place of the stack, or, with `None`, the top one
    /// is put away.
    pub fn front(&mut self, sheet: Option<FrameSheet>) {
        match sheet {
            Some(sheet) => {
                if !self.sheets.contains(&sheet) {
                    self.sheets.push(sheet);
                    self.sheets.sort();
                }
            }
            None => {
                if !self.sheets.is_empty() {
                    self.sheets.remove(0);
                }
            }
        }
    }

    pub fn camera(&self) -> Option<Holder> {
        self.camera
    }

    pub fn hold_camera(&mut self, holder: Option<Holder>) {
        self.camera = holder;
    }
}
