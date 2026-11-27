//! The motion store: what a component's motion needs between frames (04 §8.1).

use faraday_draw::geom::Rect;
use faraday_port::values::Anchor;

/// Owned by the runtime, lent to each frame's `Ui`: entries keyed by (page anchor, component
/// anchor).
pub struct MotionStore {
    entries: Vec<(ComponentId, Motion)>,
}

impl MotionStore {
    /// The runtime's only: the one store, empty.
    #[expect(
        clippy::new_without_default,
        reason = "only the runtime makes the motion store (01 R1)"
    )]
    pub fn new() -> MotionStore {
        MotionStore {
            entries: Vec::new(),
        }
    }

    /// The entry of a component, if a frame kept one.
    pub fn get(&self, id: &ComponentId) -> Option<&Motion> {
        self.entries.iter().find(|(k, _)| k == id).map(|(_, m)| m)
    }

    /// Keeps `motion` for a component, replacing its entry.
    pub fn set(&mut self, id: ComponentId, motion: Motion) {
        match self.entries.iter_mut().find(|(k, _)| *k == id) {
            Some(e) => e.1 = motion,
            None => self.entries.push((id, motion)),
        }
    }

    /// Drops a component's entry.
    pub fn remove(&mut self, id: &ComponentId) {
        self.entries.retain(|(k, _)| k != id);
    }

    /// How many entries are kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entry is kept.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Numbers, rectangles and times only (K9).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Motion {
    Region {
        offset: f32,
        velocity: f32,
        stretch: f32,
        extent: f32,
        bar_shown_ms: u64,
    },
    /// The session pane, the guide's pane.
    Glide { from: Rect, to: Rect, began_ms: u64 },
    /// A sheet, a menu, the dice sheet, the snackbar.
    Shown { at: f32, began_ms: u64 },
    /// Hold-to-confirm's fill.
    Hold { began_ms: u64 },
    /// A segmented control's fill.
    Slide { from: u16, to: u16, began_ms: u64 },
    /// A page turn, a step's fade through.
    Fade { kind: Turn, began_ms: u64 },
    /// A last-frame measure of 04 §7.2's table.
    Laid { measure: f32 },
}

/// What fades: a page turn, or a step's fade through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Turn {
    Page,
    Step,
}

/// A component's key in the store: the page's anchor, the component's anchor.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ComponentId(pub Anchor, pub Anchor);
