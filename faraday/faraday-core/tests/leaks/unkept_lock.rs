//! A lock whose save failed: the old app is dropped and a fresh one is built
//! in the same process over what it carried. The test seed loaded before the
//! lock must be in no live block afterwards, and none freed unwiped.

use crate::common::{lock, type_text};
use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish};
use faraday_core::{Action, testkit};
use faraday_shell_api::App;

#[test]
fn a_seed_loaded_before_a_lock_that_could_not_save_leaves_no_copy_in_the_fresh_app() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.session = faraday_core::wallet::Session::on(testkit::NET);
    app.press(Action::Entry(None));
    type_text(&mut app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    lock(&mut app);
    let carried = app.carry_unkept_lock();
    drop(app);
    let mut fresh = testkit::new_on_test_net();
    fresh.start_unrestarted(carried);
    let _ = fresh.frame();
    assert!(fresh.session.keys.is_empty(), "the fresh app holds a key");
    finish("seed loaded, lock unkept, fresh app", armed, true);
}
