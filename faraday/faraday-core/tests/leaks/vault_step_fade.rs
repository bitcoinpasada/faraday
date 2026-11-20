//! A step of Create a vault with a passphrase typed, left and fading
//! through, drawn again from the form at every step with no image kept
//! (`docs/MOTION.md` §3.5): the passphrase is not in any image, and Lock
//! halfway through the fade leaves no copy of it.

use crate::forms::{VAULT_PASSPHRASE, vault_passphrase_forms};
use crate::{arm, finish};
use faraday_core::vault::{VaultAction as V, vstep};
use faraday_core::{Action, Screen, testkit};
use faraday_shell_api::{App as _, Event};

#[test]
fn a_passphrase_typed_on_a_step_left_leaves_no_image_or_copy_after_a_lock_mid_fade() {
    let forms = vault_passphrase_forms();
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    app.press(Action::Vault(V::CFocus(0, false)));
    crate::common::type_text(&mut app, VAULT_PASSPHRASE);
    let _ = app.frame();
    let mut clock = 10_000;
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    for _ in 0..3 {
        clock += 16;
        app.event(Event::Tick { now_ms: clock });
        while app.poll_command().is_some() {}
        let _ = app.frame();
        assert!(app.kept_images().is_empty(), "an image of the step is kept");
    }
    crate::seed_load::lock(&mut app);
    finish("create vault step fade", armed, true);
}
