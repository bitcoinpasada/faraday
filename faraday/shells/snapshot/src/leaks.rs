//! The tours' check of what a frame leaves of a secret
//! (`docs/SECRET-LEAKS.md` §5). The test secrets are planted as text
//! forms; after every press the frame is drawn, and at each step that
//! follows Lock, a hide or leaving a page that showed a secret, the frame's
//! text and every image the core keeps across frames (a sheet's frosted
//! copy of the page, the fade's two frames) must not hold one. The events
//! are found by what the press did, not by the screen it was on: Lock
//! pressed or the process restarted; a control of hiding pressed while a
//! secret was drawn; the screen or sheet changed while a secret was drawn.

use crate::tour::Tour;
use faraday_core::testkit;
use faraday_core::{Action, Sheet};
use faraday_shell_api::App;

/// Words of an action's name that hide what a screen shows.
const HIDES: [&str; 5] = ["hide", "eye", "reveal", "cover", "conceal"];

/// The published 24-word BIP-39 vector.
const VECTOR: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
abandon abandon art";

/// The codex32 strings of BIP-93's vectors the tours use.
const CODEX32: [&str; 3] = [
    "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw",
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
];

/// Plants the test secrets' text forms for this thread.
pub(crate) fn plant() {
    let phrases: Vec<String> = testkit::TEST_SEEDS
        .iter()
        .map(|(word, _)| testkit::test_words(word))
        .collect();
    let mut forms: Vec<(String, &str)> = Vec::new();
    for (i, p) in phrases.iter().enumerate() {
        forms.push((format!("the phrase of test key {}", i + 1), p));
    }
    forms.push(("the BIP-39 vector's phrase".into(), VECTOR));
    for c in CODEX32 {
        forms.push(("a codex32 string".into(), c));
    }
    // The test vault's items: a password, a TOTP secret as Base32, a note.
    forms.push((
        "a vault item's password".into(),
        "correct-horse-battery-staple",
    ));
    forms.push(("a TOTP secret as Base32".into(), "JBSWY3DPEHPK3PXP"));
    forms.push(("a vault note".into(), "Recovery codes"));
    let forms: Vec<(&str, &str)> = forms.iter().map(|(n, t)| (n.as_str(), *t)).collect();
    faraday_core::leaks::plant(&forms);
}

/// Whether `a` is a control of hiding, by the words of its name.
fn hides(a: Action) -> bool {
    let name = format!("{a:?}").to_lowercase();
    HIDES.iter().any(|w| name.contains(w))
}

/// What the screen showed before a press.
pub(crate) struct Before {
    form: Option<String>,
    key: String,
}

impl Tour {
    /// The state before a press: the planted form the last frame showed,
    /// and its screen and sheet.
    pub(crate) fn leak_before(&mut self, a: Action) -> Before {
        // A control of hiding is pressed on the screen as a person sees it:
        // drawn first, as a shell draws after every press.
        if hides(a) {
            let _ = self.app.frame();
        }
        Before {
            form: self.app.shown_form(),
            key: self.leak_key(),
        }
    }

    fn leak_key(&self) -> String {
        format!("{:?}/{:?}", self.app.screen, self.app.sheet)
    }

    /// After press `a`, drawn: on a step that follows Lock, a hide or
    /// leaving a page that showed a secret, the frame's text and each kept
    /// image are looked at; a hit is noted for the end of the tour.
    pub(crate) fn leak_after(&mut self, a: Action, before: &Before, restarts: u32) {
        let lock = restarts != self.restarts || matches!(a, Action::Lock | Action::LockUnsealed);
        // A frame is drawn only where it can matter: the screen showed a
        // secret, or the press locked. Drawing after every press would
        // move the app's own work, which a frame starts.
        if !lock && before.form.is_none() {
            return;
        }
        let _ = self.app.frame();
        let asking = self.app.sheet == Some(Sheet::LockAsk);
        let shown = before.form.is_some();
        let event = if lock && !asking {
            "Lock"
        } else if shown && hides(a) {
            "a hide"
        } else if shown && self.leak_key() != before.key {
            "leaving a page"
        } else {
            return;
        };
        let at = format!(
            "{} step {:02} ({a:?}, after {event}) on {}",
            self.name,
            self.n + 1,
            self.leak_key()
        );
        if let Some(form) = self.app.shown_form() {
            self.leaks
                .push(format!("{at}: the frame's text shows {form}"));
        }
        for k in self.app.kept_images() {
            if let Some(form) = k.form {
                self.leaks.push(format!(
                    "{at}: the kept {} of {} shows {form}",
                    k.what, k.screen
                ));
            }
        }
    }
}
