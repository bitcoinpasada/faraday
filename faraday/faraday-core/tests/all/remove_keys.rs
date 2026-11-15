//! Remove every key while a key is kept nowhere else: the list of what is
//! lost, and a hold, not a press.

use crate::common::*;
use faraday_core::{Action, Sheet};
use faraday_shell_api::App;

#[test]
fn remove_every_key_with_a_key_kept_nowhere_lists_it_and_takes_a_hold() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    add_key(&mut app, faraday_core::testkit::TEST_SEEDS[0].0);
    assert!(app.holds_secret());
    app.press(Action::RemoveKeys);
    assert!(app.holds_secret(), "a press alone removes nothing");
    assert_eq!(app.sheet, Some(Sheet::RemoveKeys));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Hold to remove every key"),
        "{texts:?}"
    );
    app.press(Action::RemoveKeysHold);
    assert!(!app.holds_secret());
    assert_eq!(app.sheet, None);
}
