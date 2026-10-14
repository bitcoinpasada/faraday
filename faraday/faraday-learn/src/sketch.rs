//! A firmware screen's sketch (`docs/DECISIONS.md` F18, `docs/STARTER.md`
//! §7.8): the shape every sketch is built from, and the marker a page's
//! paragraph names one by, so Faraday's own drawing
//! (`faraday_ui::sketch`) turns it into a mock-up of the firmware
//! screen it depicts, drawn to look like one: square corners, firmware
//! colours and a text-mode font.
//!
//! A maker page asks for one with a paragraph of its own, `::sketch
//! <id>`, right after the step it shows ([`is_marker`]). Faraday
//! Starter and Faraday Companion draw it there; the stick, the Pi and
//! Faraday Desktop never reach a page that has one. The sketches
//! themselves, and the lookup from an id or a marker to one
//! ([`crate`]'s sibling, `faraday-guides`), are not here: the words in
//! them are the firmware's own, read from the makers' simulators,
//! manuals and support articles, and not Faraday's.

/// The firmware colours a sketch is drawn in: the ones nearest the
/// family's real screen, as the makers' own pictures show it
/// (`docs/STARTER.md` §7.8). Every one is a text-mode screen's palette;
/// none is Faraday's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    /// Grey text on black, a blue bar on the row: classic text-mode
    /// setup, and the default where a maker's pictures show black or
    /// nothing.
    Black,
    /// Grey text on blue, a red bar on the row: setup drawn on blue.
    Blue,
    /// Black text on light grey, a blue title bar: a grey graphical
    /// setup.
    Grey,
    /// Black text on white, a blue title bar: a white graphical setup.
    White,
    /// Grey text on black, red bars: a black and red gaming setup.
    Red,
    /// Grey text on black, gold bars: a black setup with gold
    /// highlights.
    Gold,
}

/// Where the screen's menus are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// None: a boot menu, a message, a single page.
    None,
    /// A row of tabs across the top.
    Tabs,
    /// A list down the left side.
    Side,
}

/// What a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// A row near the one to find.
    Plain,
    /// A group's heading inside the list.
    Group,
    /// The row to find, left as it is shown.
    Target,
    /// The row to find, changed from its value to this one.
    Change(&'static str),
}

/// One row: a label and its value, as the firmware shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// The label.
    pub label: &'static str,
    /// The value; empty for a row that is only a label.
    pub value: &'static str,
    /// What the row is.
    pub mark: Mark,
}

/// A box over the screen, which is drawn dimmed behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dialog {
    /// Its title.
    pub title: &'static str,
    /// Its message; empty for none.
    pub text: &'static str,
    /// Choices listed in it, one marked.
    pub rows: &'static [Row],
    /// Its buttons.
    pub buttons: &'static [&'static str],
    /// The button to choose, when it has buttons.
    pub choose: usize,
}

/// One firmware screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sketch {
    /// What a page's marker names it by.
    pub id: &'static str,
    /// The colours it is drawn in.
    pub palette: Palette,
    /// The title line at the top; empty for none.
    pub title: &'static str,
    /// Where the menus are.
    pub nav: Nav,
    /// The tabs or the side list's items.
    pub items: &'static [&'static str],
    /// The one open.
    pub selected: usize,
    /// The page's heading or path under the menus; empty for none.
    pub heading: &'static str,
    /// A message above the rows; empty for none.
    pub text: &'static str,
    /// The rows near the one to find, in screen order.
    pub rows: &'static [Row],
    /// A box over the screen.
    pub dialog: Option<Dialog>,
    /// The key hints or buttons along the bottom.
    pub keys: &'static [&'static str],
}

/// A row with a label and a value, marked [`Mark::Plain`].
pub const fn row(label: &'static str, value: &'static str) -> Row {
    Row {
        label,
        value,
        mark: Mark::Plain,
    }
}

/// A group's heading row.
pub const fn group(label: &'static str) -> Row {
    Row {
        label,
        value: "",
        mark: Mark::Group,
    }
}

/// The row to find, left as it is shown.
pub const fn target(label: &'static str, value: &'static str) -> Row {
    Row {
        label,
        value,
        mark: Mark::Target,
    }
}

/// The row to find, changed from `from` to `to`.
pub const fn change(label: &'static str, from: &'static str, to: &'static str) -> Row {
    Row {
        label,
        value: from,
        mark: Mark::Change(to),
    }
}

/// A sketch with nothing in it but its id and palette, filled in by
/// whoever builds the data (`faraday-guides`).
pub const fn blank(id: &'static str, palette: Palette) -> Sketch {
    Sketch {
        id,
        palette,
        title: "",
        nav: Nav::None,
        items: &[],
        selected: 0,
        heading: "",
        text: "",
        rows: &[],
        dialog: None,
        keys: &[],
    }
}

/// The keys a key hint can begin with by name, as firmware prints them;
/// a function key, an arrow or a sign is told by its shape. Faraday's
/// drawing sets the key in a hint apart from the words after it.
pub const KEY_NAMES: &[&str] = &["Esc", "ESC", "Enter", "ENTER", "Tab", "Del", "Space"];

/// What a page's marker paragraph begins with.
pub const MARKER: &str = "::sketch ";

/// Whether `paragraph` is a marker: a paragraph of its own that begins
/// `::sketch`, which only the apps that draw sketches show, as the
/// sketch.
pub fn is_marker(paragraph: &str) -> bool {
    paragraph.trim_start().starts_with(MARKER.trim_end())
}
