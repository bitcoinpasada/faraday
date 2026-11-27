//! The display's and the machine's facts the kit may name (P4).

/// The name the app gives a place it draws; the kernel's view orders and the kit's motion store
/// key by it (section 04, K10).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Anchor {
    pub scope: &'static str,
    pub n: u32,
}

/// Which way a panel shows the app.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Wider than tall.
    #[default]
    Landscape,
    /// Taller than wide.
    Portrait,
}

/// A point in whole pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// What the removable medium is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Medium {
    #[default]
    Stick,
    SdCard,
    Desktop,
}

/// Faraday Demo's preview of a device's size: the size shown, in dp; `None` is the window's
/// own size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preview {
    pub chosen: Option<(u16, u16)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Display {
    pub width: u16,
    pub height: u16,
    pub dpi: u16,
    pub orientation: Orientation,
    /// The screen may be turned; its native orientation.
    pub rotation_offered: Option<Orientation>,
    /// The picture goes to an external screen.
    pub external: bool,
    /// Faraday Demo's preview of a device's size, and the size chosen.
    pub preview: Option<Preview>,
}

/// How soon the next tick wants to come (02 §3.3): the kit's frame output says it, and the
/// port's frame carries it to the adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    Rest,
    Moving,
}
