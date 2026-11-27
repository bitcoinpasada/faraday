use faraday_kernel::AtLock;
use faraday_text::Text;

/// What Home keeps: the words typed in the bar search and the guide words they found. Nothing
/// here is a secret, so a lock leaves it (the next build starts over anyway).
#[derive(AtLock)]
pub struct State {
    /// The words typed in the search.
    #[lock(public)]
    find: Text,
    /// The typing goes to the search field.
    #[lock(public)]
    focused: bool,
    /// The places in the guide's glossary that hold every word typed, in its order.
    #[lock(public)]
    found: Vec<u32>,
}

impl State {
    pub(crate) fn new() -> State {
        State {
            find: Text::default(),
            focused: false,
            found: Vec::new(),
        }
    }

    /// The glossary words the typed words found: their places in `faraday_guide::glossary`.
    pub fn found(&self) -> &[u32] {
        &self.found
    }

    /// What is typed in the search.
    pub fn typed(&self) -> &str {
        self.find.as_str()
    }

    pub(crate) fn find(&self) -> &Text {
        &self.find
    }

    pub(crate) fn focused(&self) -> bool {
        self.focused
    }

    pub(crate) fn focus(&mut self) {
        self.focused = true;
    }

    pub(crate) fn type_char(&mut self, c: char) {
        self.find.push(c);
        self.focused = true;
    }

    pub(crate) fn erase(&mut self) {
        self.find.pop();
    }

    pub(crate) fn clear(&mut self) {
        self.find.clear();
        self.found.clear();
    }

    pub(crate) fn set_found(&mut self, found: Vec<u32>) {
        self.found = found;
    }
}
