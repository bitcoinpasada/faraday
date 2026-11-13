//! The tours' check of what a frame leaves of a secret
//! (`docs/SECRET-LEAKS.md` §5). The test secrets are planted as text
//! forms; after every press the frame is drawn, and at each step that
//! follows Lock, a hide or leaving a page or a sheet that showed a secret,
//! the frame's text and every image the core keeps across frames (a
//! sheet's frosted copy of the page, the fade's two frames) must not hold
//! one it should not. The events are found by what the press did, not by
//! the screen it was on: Lock pressed or the process restarted; a control
//! of hiding pressed again on what it opened, or a control that only hides
//! pressed, while a secret was drawn; the screen changed, or a sheet closed
//! or gave way to another, while a secret was drawn or kept. What stays
//! rightly shown is not a hit: after a hide, what the frame showed before
//! the hidden thing was opened; after leaving a sheet, what the page under
//! it shows. A sheet opened over a page leaves nothing: the page is still
//! there under it.

use crate::tour::Tour;
use faraday_core::testkit;
use faraday_core::{Action, Screen, Sheet};
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

/// Whether `a` only hides (its name says "hide"), rather than showing and
/// hiding in turn.
fn only_hides(a: Action) -> bool {
    format!("{a:?}").to_lowercase().contains("hide")
}

/// What the check keeps from press to press, to tell a hide from an open.
#[derive(Default)]
pub(crate) struct Opened {
    /// The controls that show and hide in turn pressed to show, on the
    /// screen on show, each with the form the frame showed before it.
    toggles: Vec<(Action, Option<String>)>,
    /// The form the frame showed before the last press.
    last: Option<String>,
}

/// What the screen showed before a press.
pub(crate) struct Before {
    /// The planted form the last frame's text showed.
    form: Option<String>,
    /// The planted form an image the core keeps was made from.
    kept: Option<String>,
    /// The planted form the page under the open sheet showed, from its
    /// frosted copy.
    page: Option<String>,
    screen: Screen,
    sheet: Option<Sheet>,
}

impl Tour {
    /// The state before a press: the planted form the last frame showed,
    /// what the images kept were made from, and its screen and sheet.
    pub(crate) fn leak_before(&mut self, a: Action) -> Before {
        // A control of hiding is pressed on the screen as a person sees it:
        // drawn first, as a shell draws after every press.
        if hides(a) {
            let _ = self.app.frame();
        }
        let kept = self.app.kept_images();
        Before {
            form: self.app.shown_form(),
            kept: kept.iter().find_map(|k| k.form.clone()),
            page: kept
                .iter()
                .find(|k| k.what == "frost")
                .and_then(|k| k.form.clone()),
            screen: self.app.screen,
            sheet: self.app.sheet,
        }
    }

    fn leak_key(&self) -> String {
        format!("{:?}/{:?}", self.app.screen, self.app.sheet)
    }

    /// Whether press `a` hid a secret, and if so what may still be shown:
    /// the form the frame showed before the thing hidden was opened. A
    /// control that shows and hides in turn hides when pressed again on
    /// what it opened; pressed with no secret drawn, or first, it opens.
    /// A control that only hides hides what the press before it opened.
    fn hide_of(&mut self, a: Action, before: &Before) -> Option<Option<String>> {
        if !hides(a) {
            return None;
        }
        if only_hides(a) {
            return Some(self.opened.last.clone());
        }
        let at = self.opened.toggles.iter().position(|(t, _)| *t == a);
        match (at, &before.form) {
            (Some(i), Some(_)) => Some(self.opened.toggles.remove(i).1),
            (at, _) => {
                if let Some(i) = at {
                    self.opened.toggles.remove(i);
                }
                self.opened.toggles.push((a, before.form.clone()));
                None
            }
        }
    }

    /// After press `a`, drawn: on a step that follows Lock, a hide or
    /// leaving a page or a sheet that showed a secret, the frame's text and
    /// each kept image are looked at; a hit is noted for the end of the
    /// tour.
    pub(crate) fn leak_after(&mut self, a: Action, before: &Before, restarts: u32) {
        let lock = restarts != self.restarts || matches!(a, Action::Lock | Action::LockUnsealed);
        let hide = self.hide_of(a, before);
        self.opened.last = before.form.clone();
        if lock || self.app.screen != before.screen {
            self.opened.toggles.clear();
        }
        // A frame is drawn only where it can matter: the screen showed or
        // kept a secret, or the press locked. Drawing after every press
        // would move the app's own work, which a frame starts.
        let shown = before.form.is_some() || before.kept.is_some();
        if !lock && !shown {
            return;
        }
        let _ = self.app.frame();
        let asking = self.app.sheet == Some(Sheet::LockAsk);
        let left_page = self.app.screen != before.screen;
        let left_sheet = !left_page && before.sheet.is_some() && self.app.sheet != before.sheet;
        let (event, stays) = if lock && !asking {
            ("Lock", None)
        } else if let (Some(stays), Some(_)) = (hide, &before.form) {
            ("a hide", stays)
        } else if left_page {
            ("leaving a page", None)
        } else if left_sheet {
            ("leaving a sheet", before.page.clone())
        } else {
            return;
        };
        let at = format!(
            "{} step {:02} ({a:?}, after {event}) on {}",
            self.name,
            self.n + 1,
            self.leak_key()
        );
        let wrong = |form: Option<String>| form.filter(|f| stays.as_ref() != Some(f));
        if let Some(form) = wrong(self.app.shown_form()) {
            self.leaks
                .push(format!("{at}: the frame's text shows {form}"));
        }
        for k in self.app.kept_images() {
            if let Some(form) = wrong(k.form) {
                self.leaks.push(format!(
                    "{at}: the kept {} of {} shows {form}",
                    k.what, k.screen
                ));
            }
        }
    }
}
