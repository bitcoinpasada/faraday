//! What the strip's controls said in the frame last drawn: the host's
//! own wording, a lookup for drivers and tests. It holds control labels
//! and menu rows only, never text a person typed or a key's, so it is
//! kept apart from the code that handles those.

use faraday_text::Text;

/// The strip's texts, left to right, then the open menu's rows.
#[derive(Default)]
pub(crate) struct Labels(Vec<String>);

impl Labels {
    /// The texts a frame drew, taken whole.
    pub(crate) fn of(drawn: Vec<String>) -> Labels {
        Labels(drawn)
    }

    /// The texts, one [`Text`] each. A lookup: made when asked, not when
    /// a frame is drawn.
    pub(crate) fn texts(&self) -> Vec<Text> {
        self.0.iter().map(|s| Text::of(s)).collect()
    }
}
