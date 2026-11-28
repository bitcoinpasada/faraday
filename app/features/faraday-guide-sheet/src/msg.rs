use faraday_kernel::{FrameSheet, Message, Public};
use faraday_present::Pressed;
use faraday_text::Text;

/// What the window gives the guide, passed by the parent in the open and follow messages: the
/// feature has no window class, no held flag and no word of what a page keeps for its own.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct Room {
    /// The window is extra-large (1600 dp and over).
    pub extra_large: bool,
    /// The small panel's layout.
    pub compact: bool,
    /// The Starter holds the guide as a page under its own top app bar.
    pub held: bool,
    /// The page on show leaves the supporting pane's place free.
    pub place_free: bool,
}

impl Room {
    /// Whether the window holds the guide as a pane beside the page: extra-large, not compact,
    /// not held.
    #[must_use]
    pub fn wide(&self) -> bool {
        self.extra_large && !self.compact && !self.held
    }

    /// Whether the guide stands as the pane now: [`Room::wide`] and the place free.
    #[must_use]
    pub fn pane(&self) -> bool {
        self.wide() && self.place_free
    }
}

/// Where the page on show puts the guide. The parent maps its own page to it, since a feature
/// never names another feature's pages (A5).
#[derive(Public)]
pub struct Place {
    /// The section key: `<place>` or `<place>.<part>` of `faraday_guide`'s pages.
    pub key: Text,
    /// The page's name in the How blocks' screens (`faraday_guide::how`), where it has any.
    pub how_screen: Option<Text>,
    /// The build's release, put before each Code entry of a How block.
    pub release: Text,
    pub room: Room,
}

#[derive(Message)]
pub enum Msg {
    Press(Press),
    /// The Guide button: opens the guide at the page on show, or puts the pane away.
    Open(Place),
    /// A glossary word (its place in `faraday_guide::glossary`) from outside the guide: opens
    /// it at the word, or moves an open guide to it.
    OpenWord(Place, u32),
    /// The page on show or the window changed: an open guide follows it.
    Follow(Place),
    /// A key typed on the contents' Find.
    Typed(char),
    /// Backspace in Find.
    Erase,
    /// Escape: clears a Find with something typed, else puts the pane away when no sheet is
    /// over the page.
    Escape,
    /// Cancel (Back, the mouse's back button): a step back inside the guide, else it closes.
    Cancel,
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

/// What the feature tells its parent.
#[derive(Message)]
pub enum Out {
    /// A frame sheet: the guide as the sheet comes up (`Some`) or goes (`None`).
    Front(Option<FrameSheet>),
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    TabGuide,
    TabHow,
    Contents,
    /// The place's own page, first in the contents.
    Here,
    /// The page in this place of the contents' groups.
    Page(u32),
    /// The glossary word in this place.
    Word(u32),
    /// The contents' Find is pressed: the typing goes to it.
    Focus,
    /// The button that empties Find.
    ClearFind,
    /// Done on the sheet, the pane's close button, and the press beside the sheet.
    Close,
}
