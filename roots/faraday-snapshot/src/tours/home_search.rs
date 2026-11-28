//! The `home-search` tour: Home's search bar with nothing typed, focused, with a word typed, with
//! a word nothing has, and cleared.

use faraday_fixture_adapters::Scripted;
use faraday_port::Text;

use crate::runner::Tour;

pub fn run(t: &mut Tour) -> Result<(), String> {
    t.step("empty")?;
    // The Pi's Home has no search field: it has the lead and its places.
    if !has_search(t) {
        return Ok(());
    }
    t.press("faraday-home.Focus")?;
    t.step("focused")?;
    for (word, step) in [("send", "send"), ("zzzz", "none")] {
        t.input(&[Scripted::Text(Text::of(word))]);
        t.step(step)?;
        t.press("faraday-home.Clear")?;
    }
    t.step("cleared")
}

/// Whether the frame last taken has Home's search field.
fn has_search(t: &Tour) -> bool {
    t.look()
        .is_some_and(|l| l.hits.iter().any(|h| h.message == "faraday-home.Focus"))
}
