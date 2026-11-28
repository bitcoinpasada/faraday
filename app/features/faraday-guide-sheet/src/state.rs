use faraday_guide::{GuidePage, guide_groups, guide_page, how};
use faraday_kernel::AtLock;
use faraday_text::Text;

use crate::{Place, Room};

/// The most characters Find takes.
pub(crate) const FIND_MAX: usize = 40;

/// What the guide keeps: where it stands and what is typed in Find. Nothing here is a secret,
/// so a lock leaves it (the next build starts over anyway).
#[derive(AtLock)]
pub struct State {
    /// The guide is open as the sheet.
    #[lock(public)]
    sheet: bool,
    /// The guide is open as the supporting pane beside the page.
    #[lock(public)]
    pane: bool,
    /// The key the guide was opened at, followed with the page.
    #[lock(public)]
    key: Text,
    /// The place of the page on show; empty while the contents is.
    #[lock(public)]
    shown: Text,
    #[lock(public)]
    contents: bool,
    /// The page on show is a glossary word opened from another page: Back returns.
    #[lock(public)]
    word: bool,
    /// The Cryptography tab is on show.
    #[lock(public)]
    how: bool,
    /// The section the next frame scrolls to.
    #[lock(public)]
    to: Option<Text>,
    /// The key the pane last followed; empty while the pane is not open.
    #[lock(public)]
    followed: Text,
    #[lock(public)]
    find: Text,
    #[lock(public)]
    find_focus: bool,
    /// The page on show's name among the How blocks' screens, where it has blocks.
    #[lock(public)]
    how_screen: Option<Text>,
    #[lock(public)]
    release: Text,
    /// The window's room as the parent last told it.
    #[lock(public)]
    room: Room,
}

/// The place of a section key.
pub(crate) fn place_of(key: &str) -> &str {
    key.split('.').next().unwrap_or(key)
}

impl State {
    pub(crate) fn new() -> State {
        State {
            sheet: false,
            pane: false,
            key: Text::default(),
            shown: Text::default(),
            contents: false,
            word: false,
            how: false,
            to: None,
            followed: Text::default(),
            find: Text::default(),
            find_focus: false,
            how_screen: None,
            release: Text::default(),
            room: Room {
                extra_large: false,
                compact: false,
                held: false,
                place_free: true,
            },
        }
    }

    /// The guide is open, as the sheet or the pane.
    #[must_use]
    pub fn open(&self) -> bool {
        self.sheet || self.pane
    }

    /// The guide is open as the sheet.
    #[must_use]
    pub fn sheet(&self) -> bool {
        self.sheet
    }

    /// The guide is open as the pane, and the window and the page still leave it its place.
    #[must_use]
    pub fn pane_shown(&self) -> bool {
        self.pane && self.room.pane()
    }

    pub(crate) fn pane_open(&self) -> bool {
        self.pane
    }

    pub(crate) fn how_on(&self) -> bool {
        self.how && self.how_offered()
    }

    pub(crate) fn contents(&self) -> bool {
        self.contents
    }

    pub(crate) fn find(&self) -> &str {
        self.find.as_str()
    }

    pub(crate) fn find_text(&self) -> &Text {
        &self.find
    }

    pub(crate) fn find_focus(&self) -> bool {
        self.find_focus
    }

    pub(crate) fn room(&self) -> Room {
        self.room
    }

    pub(crate) fn to(&self) -> Option<&str> {
        self.to.as_ref().map(Text::as_str)
    }

    pub(crate) fn release(&self) -> &str {
        self.release.as_str()
    }

    pub(crate) fn how_screen(&self) -> Option<&str> {
        self.how_screen.as_ref().map(Text::as_str)
    }

    /// The How blocks of the page on show, as `faraday_guide::how` finds them.
    pub(crate) fn how_blocks(&self) -> Vec<&'static faraday_guide::Block> {
        self.how_screen().map(how::for_screen).unwrap_or_default()
    }

    /// Whether the page on show has How blocks: the tab is offered then.
    pub(crate) fn how_offered(&self) -> bool {
        self.how_screen().is_some_and(how::offered)
    }

    /// The page on show.
    pub(crate) fn shown_page(&self) -> Option<&'static GuidePage> {
        if self.contents || self.shown.is_empty() {
            return None;
        }
        guide_page(self.shown.as_str())
    }

    /// The page of the place the guide was opened at, if one is written.
    pub(crate) fn here(&self) -> Option<&'static GuidePage> {
        guide_page(place_of(self.key.as_str()))
    }

    /// Takes what the parent says of the page and the window.
    pub(crate) fn place(&mut self, place: &Place) {
        self.room = place.room;
        self.release = place.release.clone();
        self.how_screen = place.how_screen.clone();
    }

    /// The guide opens at `key` as the sheet, from the beginning.
    pub(crate) fn open_at(&mut self, key: &str) {
        self.find.clear();
        self.find_focus = false;
        self.how = false;
        self.show_key(key);
        self.pane = false;
        self.followed.clear();
        self.sheet = true;
    }

    /// Shows the page of `key`, scrolled to its section; the contents where the place has no
    /// page.
    pub(crate) fn show_key(&mut self, key: &str) {
        self.key = Text::of(key);
        self.contents = guide_page(place_of(key)).is_none();
        self.shown = Text::of(place_of(key));
        self.word = false;
        self.to = (!self.contents).then(|| Text::of(key));
    }

    /// Shows the definition at `key` in the glossary: Back returns to the page it was pressed on.
    pub(crate) fn show_word(&mut self, key: &str) {
        self.how = false;
        self.contents = false;
        self.shown = Text::of(place_of(key));
        self.word = self.shown.as_str() != place_of(self.key.as_str());
        self.to = Some(Text::of(key));
    }

    /// Whether Back goes one step inside the guide before it closes.
    pub(crate) fn inside(&self) -> bool {
        if self.how {
            false
        } else if self.contents {
            self.here().is_some()
        } else {
            !self.shown.is_empty() && self.shown.as_str() != place_of(self.key.as_str())
        }
    }

    /// Back inside the guide: from a page opened from the contents to the contents, from the
    /// contents to the place's page. False where Back closes the guide.
    pub(crate) fn step_back(&mut self) -> bool {
        if !self.inside() {
            return false;
        }
        if self.contents || self.word {
            let key = self.key.clone();
            self.show_key(key.as_str());
        } else {
            self.contents = true;
        }
        true
    }

    pub(crate) fn tab_guide(&mut self) {
        self.how = false;
        if !self.contents {
            let key = self.key.clone();
            self.to = (self.shown.as_str() == place_of(key.as_str())).then_some(key);
        }
    }

    pub(crate) fn tab_how(&mut self) {
        self.how = true;
    }

    pub(crate) fn show_contents(&mut self) {
        self.how = false;
        self.word = false;
        self.contents = true;
        self.find.clear();
        self.find_focus = false;
    }

    pub(crate) fn show_here(&mut self) {
        let key = self.key.clone();
        self.how = false;
        self.show_key(key.as_str());
    }

    /// The page in place `n` of the contents' groups.
    pub(crate) fn show_page(&mut self, n: u32) -> bool {
        let Some(page) = guide_groups()
            .iter()
            .flat_map(|g| g.pages.iter())
            .nth(n as usize)
        else {
            return false;
        };
        self.how = false;
        self.word = false;
        self.contents = false;
        self.shown = Text::of(page.place.as_str());
        self.to = Some(Text::of(page.place.as_str()));
        true
    }

    /// The screen offers no blocks any more: the first tab is on show.
    pub(crate) fn drop_how_if_gone(&mut self) -> bool {
        if self.how && !self.how_offered() {
            self.how = false;
            return true;
        }
        false
    }

    pub(crate) fn set_find_focus(&mut self, on: bool) {
        self.find_focus = on;
    }

    pub(crate) fn clear_find(&mut self) {
        self.find.clear();
    }

    /// A character typed on the contents' Find: none that is a control character, at most
    /// `FIND_MAX`. True when taken.
    pub(crate) fn type_char(&mut self, c: char) -> bool {
        if c.is_control() || self.find.as_str().chars().count() >= FIND_MAX {
            return false;
        }
        self.find.push(c);
        true
    }

    pub(crate) fn erase(&mut self) -> bool {
        if self.find.as_str().is_empty() {
            return false;
        }
        self.find.pop();
        true
    }

    pub(crate) fn close_sheet(&mut self) {
        self.sheet = false;
    }

    pub(crate) fn close_pane(&mut self) {
        self.pane = false;
        self.followed.clear();
    }

    /// The open sheet becomes the pane, on the page it shows; `now` is the page's key, which
    /// the pane follows from here.
    pub(crate) fn move_to_pane(&mut self, now: &str) {
        self.sheet = false;
        self.pane = true;
        self.followed = Text::of(now);
    }

    pub(crate) fn move_to_sheet(&mut self) {
        self.close_pane();
        self.sheet = true;
    }

    /// The page's key is `now`: where it is not the one the pane last followed, the pane shows
    /// its page.
    pub(crate) fn follow(&mut self, now: &str) -> bool {
        if self.followed.as_str() == now {
            return false;
        }
        self.followed = Text::of(now);
        self.show_key(now);
        true
    }
}
