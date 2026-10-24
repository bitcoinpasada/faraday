//! The codex32form tour: Add a key's codex32 form (`docs/CODEX32.md`
//! §8.1, §8.3, §8.4, §8.5): the form as it opens, a share failing its
//! checksum, Suggest corrections ticked and a proposal in place of the
//! field, the re-entry from the paper, a string taken, the same string
//! again, a string at an index already in beside the one held, and Check
//! a string.

use crate::tour::{Tour, type_text};
use faraday_core::codex32_correct::Press;
use faraday_core::forms::{C32Press, Form};
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event, Key};

/// BIP 93's vector 2, two shares of one set.
const SHARE_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const SHARE_C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
/// Another set's share at index `a`, with the same header.
const OTHER_A: &str = "ms12nameax8gf2tvdw0s3jn54khce6mua7ly0vdlux4s0wh7";

/// `s` with box `n` (from 1) put to `to`.
fn with(s: &str, n: usize, to: u8) -> String {
    let mut b = s.as_bytes().to_vec();
    b[n - 1] = to;
    String::from_utf8(b).unwrap_or_default()
}

fn key(t: &mut Tour, k: Key) {
    t.app.event(Event::Key(k));
    t.pump();
}

fn clear(t: &mut Tour) {
    for _ in 0..SHARE_A.len() + 2 {
        t.app.event(Event::Key(Key::Backspace));
    }
    t.pump();
}

pub(crate) fn codex32_form_tour(t: &mut Tour) -> Result<(), String> {
    let codex32 = Form::ALL
        .iter()
        .position(|f| *f == Form::Codex32)
        .ok_or("no codex32 form")?;
    t.press(Action::Entry(None));
    t.press(Action::EntryForm(codex32 as u8));
    t.shot("form")?;
    // A share failing its checksum is named by its index.
    type_text(t, &with(SHARE_C, 30, b'q'));
    key(t, Key::Enter);
    t.shot("checksum-fails")?;
    clear(t);
    // Suggest corrections: `?` on the keyboard, the proposal in place of
    // the field.
    t.press(Action::Codex32Correct(Press::Tick));
    t.shot("tick-on")?;
    type_text(t, &with(&with(SHARE_A, 20, b'q'), 31, b'?'));
    key(t, Key::Enter);
    t.shot("proposal")?;
    t.press(Action::Codex32Correct(Press::Mistyped));
    t.shot("retype")?;
    for n in [20usize, 31] {
        key(t, Key::Char(char::from(SHARE_A.as_bytes()[n - 1])));
    }
    t.shot("taken")?;
    // The same string again; then another at index a.
    type_text(t, SHARE_A);
    key(t, Key::Enter);
    t.shot("already-in")?;
    type_text(t, OTHER_A);
    key(t, Key::Enter);
    if t.app.screen == Screen::Entry {
        t.shot("clash")?;
    }
    t.press(Action::Codex32Form(C32Press::Keep));
    // Check a string.
    t.press(Action::Codex32Form(C32Press::Check));
    type_text(t, SHARE_C);
    key(t, Key::Enter);
    t.shot("check-holds")?;
    clear(t);
    type_text(t, &with(SHARE_C, 25, b'q'));
    key(t, Key::Enter);
    t.shot("check-fails")
}
