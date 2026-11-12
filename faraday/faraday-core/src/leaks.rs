//! Test hooks for the tours' check of what a frame leaves of a secret
//! (`docs/SECRET-LEAKS.md` §5): the forms of the secrets a test planted,
//! found in the text a frame draws, and what each image the core keeps
//! across frames was made from. Nothing here changes what is drawn or
//! kept; it reads the ink a frame records and notes it. Compiled only
//! with `testkit`.

use std::cell::RefCell;

use faraday_draw::Canvas;

use crate::ui::Ui;

thread_local! {
    /// The planted forms, each a name and its text, as normalised by
    /// [`words_of`]; of this thread alone, so tests side by side do not
    /// see each other's.
    static FORMS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    /// What the last frame that made a new frosted page found drawn when
    /// the page was done, before the sheet over it: `Some(form)` once a
    /// frame made one.
    static PAGE: RefCell<Option<Option<String>>> = const { RefCell::new(None) };
}

/// Plants `forms`, each a name for the report and the text of one form of
/// a test secret, for this thread: they replace those planted before. A
/// form is looked for in what a frame draws, numbers left out as a grid
/// of numbered words has them between the words.
pub fn plant(forms: &[(&str, &str)]) {
    FORMS.with(|f| {
        *f.borrow_mut() = forms
            .iter()
            .map(|(name, text)| ((*name).to_string(), words_of(std::iter::once(*text))))
            .filter(|(_, t)| !t.is_empty())
            .collect();
    });
}

/// Whether any form is planted on this thread.
pub(crate) fn planted() -> bool {
    FORMS.with(|f| !f.borrow().is_empty())
}

/// The words of `texts` in the order drawn, a token that is only digits
/// and punctuation left out, joined by single spaces.
fn words_of<'a>(texts: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for token in texts.flat_map(str::split_whitespace) {
        if token
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_punctuation())
        {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(token);
    }
    out
}

/// The name of the first planted form found in `texts`, the strings one
/// frame drew; `None` when none is.
pub fn find_form<S: AsRef<str>>(texts: &[S]) -> Option<String> {
    if !planted() {
        return None;
    }
    let drawn = words_of(texts.iter().map(AsRef::as_ref));
    FORMS.with(|f| {
        f.borrow()
            .iter()
            .find(|(_, text)| drawn.contains(text.as_str()))
            .map(|(name, _)| name.clone())
    })
}

/// The form found in what `c` recorded as drawn.
pub(crate) fn find_in_ink(c: &Canvas) -> Option<String> {
    let texts: Vec<&str> = c
        .ink()
        .iter()
        .filter_map(|i| match &i.kind {
            faraday_draw::canvas::InkKind::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    find_form(&texts)
}

/// Before a frame is drawn: turns ink recording on when a form is
/// planted and it is off. Whether it was turned on, for [`end`].
pub(crate) fn begin(c: Option<&mut Canvas>) -> bool {
    match c {
        Some(c) if planted() && !c.recording_ink() => {
            c.record_ink(true);
            true
        }
        _ => false,
    }
}

/// After a frame is drawn: the form it showed, and recording off again
/// if [`begin`] turned it on.
pub(crate) fn end(c: Option<&mut Canvas>, turned_on: bool) -> Option<String> {
    let c = c?;
    let found = find_in_ink(c);
    if turned_on {
        c.record_ink(false);
    }
    found
}

/// The page under a sheet is drawn and about to be frosted: notes what it
/// drew, when this frame makes the frosted copy.
pub(crate) fn page_drawn(ui: &Ui<'_>) {
    if planted() && ui.frost.is_none() && ui.c.recording_ink() {
        let found = find_in_ink(ui.c);
        PAGE.with(|p| *p.borrow_mut() = Some(found));
    }
}

/// What the last page made for a frosted copy showed, once.
pub(crate) fn take_page() -> Option<Option<String>> {
    PAGE.with(|p| p.borrow_mut().take())
}

/// An image the core keeps across frames, and what drew it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptImage {
    /// Which: the frosted copy of the page under a sheet (`frost`), the
    /// frame a fade starts from (`fade from`) or the new one kept aside
    /// (`fade after`).
    pub what: &'static str,
    /// The screen and sheet it was made from, as written for a report.
    pub screen: String,
    /// The planted form its screen showed when it was made, if any.
    pub form: Option<String>,
}

/// What a frame showed: its screen and the form found in its text.
#[derive(Debug, Clone, Default)]
pub(crate) struct Seen {
    pub(crate) screen: String,
    pub(crate) form: Option<String>,
}

/// The notes kept with the images of a fade.
#[derive(Default)]
pub(crate) struct Marks {
    /// The last frame drawn.
    pub(crate) frame: Option<Seen>,
    /// The frame the fade under way starts from, as it was drawn.
    pub(crate) from: Option<Seen>,
    /// The frame kept aside as the new one.
    pub(crate) after: Option<Seen>,
    /// The page the frosted copy was made of.
    pub(crate) frost: Option<Seen>,
    /// The frame took the frosted copy it had, not a new one.
    pub(crate) frost_reused: bool,
}
