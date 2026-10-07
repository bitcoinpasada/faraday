//! The lock screen (`docs/DESIGN.md` §5 Pad): the title, the dots field
//! directly above the pad, the reserved caption line that carries a
//! wrong PIN, and the pad. Nothing names the keys behind the lock:
//! whoever picks the device up would learn which keys are on it.

use alloc::string::String;

use osk_ui::layout::Node;
use osk_ui::screens::{self, Field, Pad, PadKind};

use crate::session::ATTEMPTS;
use crate::{OpenSigner, ids, strings};

impl OpenSigner {
    pub(crate) fn view_lock(&self) -> Node {
        let s = self.strings();
        let attempts = self.session.attempts_left();
        // "Wrong PIN · 4 left" on the one caption line every field
        // reserves; before the first attempt the line is empty and the
        // pad is in exactly the same place (§4.3). On a device that
        // keeps a key the count is the stored key's, as on its own pad
        // (§16.63).
        let error = self
            .kept_attempts_left()
            .map(|left| strings::fill1(s.keep_wrong, &alloc::format!("{left}")))
            .or_else(|| {
                (attempts < ATTEMPTS)
                    .then(|| strings::fill1(s.lock_wrong, &alloc::format!("{attempts}")))
            });
        self.with_chrome(None, |c| {
            screens::pad(
                c,
                Pad {
                    words: None,
                    eye: None,
                    title: String::from(s.lock_title),
                    id: ids::LOCK_KEYBOARD,
                    kind: PadKind::Pin {
                        scramble: self.session.scramble_seed(),
                        done: self.session.entry().is_complete(),
                    },
                    field: Field::Dots(self.session.entry().len()),
                    progress: None,
                    entries: None,
                    // One Back here asks for a second: the caption line
                    // carries that while it can (§16.73).
                    caption: None,
                    error: self.leave_notice().or_else(|| error.clone()),
                    action: None,
                },
            )
        })
    }
}
