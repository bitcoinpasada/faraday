//! The screens of a key kept on the device (`docs/DESIGN.md` §5): the
//! Hold that keeps it, the Pad that opens it, the Pad that sets a duress
//! PIN, and the Result once it is gone. Forgetting it is the key's own
//! Forget (`views/detail.rs`), which says the stored copy goes too.
//!
//! Nothing on any of them is a paragraph: what keeping a key means is
//! a Learn page (§4.12), and the Hold's own table is the whole of what
//! §4.13 lets the screen say.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Field, Hold, Pad, PadKind};
use osk_ui::widgets::{Icon, Tone};

use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    /// §5 Hold, "Keep on this device": which keys — every loaded key
    /// the device can keep, since it keeps all or none (§16.66) — and
    /// what will open them afterwards.
    pub(crate) fn view_keep(&self, _key: usize) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows: Vec<components::Record> = self
                .keys
                .iter()
                .filter(|k| k.is_keepable())
                .map(|k| {
                    components::Record::fingerprint(
                        s.sign_key_row,
                        text::fingerprint_hex(k.fingerprint),
                    )
                })
                .collect();
            rows.push(components::Record::text(
                s.keep_unlocked_by,
                s.keep_unlocked_by_value,
                Tone::Text,
            ));
            // No caution about the boot: a device whose boot was not
            // verified never reaches this screen, because the app
            // refuses to run on it at all.
            screens::hold(
                c,
                Hold {
                    warnings: Vec::new(),
                    title: s.keep_title,
                    rows,
                    sign_with: None,
                    then_with: None,
                    id: ids::KEEP_HOLD,
                    label: s.keep_hold,
                    danger: false,
                    enabled: self.keys.iter().any(|k| k.is_keepable()),
                    // The offer that follows adding a key says how to
                    // decline it; the row from the key menu was asked
                    // for, and the chevron is the way back (§16.65).
                    secondary: self
                        .keep_offer
                        .then(|| Action::new(ids::KEEP_NOT_NOW, s.keep_not_now)),
                },
            )
        })
    }

    /// §5 Pad, the stored key: the lock screen's own title, the dots
    /// above the pad and the reserved caption line that carries a wrong
    /// PIN with the attempts left. It is the one PIN pad a person sees
    /// (§16.63), and on a device that keeps a key it is the first screen
    /// and has nowhere to go back to (§16.64).
    pub(crate) fn view_stored_key(&self) -> Node {
        let s = self.strings();
        let error = self
            .kept_attempts_left()
            .map(|left| strings::fill1(s.keep_wrong, &alloc::format!("{left}")));
        self.pad_screen(
            None,
            s.lock_title,
            ids::KEEP_PIN_KEYBOARD,
            self.keep.pin_len(),
            self.keep.pin_complete(),
            // One Back here asks for a second: the caption line carries
            // that while it can (§16.73).
            self.leave_notice().or(error),
        )
    }

    /// §5 Pad, "Duress PIN", typed and then repeated. A repeat that did
    /// not match restarts the entry and says so on the caption line.
    pub(crate) fn view_duress_pin(&self) -> Node {
        let s = self.strings();
        let title = if self.keep.repeat {
            s.pin_repeat_title
        } else {
            s.keep_duress_row
        };
        let error = self.keep.mismatch.then(|| String::from(s.pin_mismatch));
        self.pad_screen(
            Some(ids::BACK),
            title,
            ids::KEEP_DURESS_KEYBOARD,
            self.keep.pin_len(),
            self.keep.pin_complete(),
            error,
        )
    }

    /// §5 Result, "Stored key removed": what eight wrong PINs left.
    pub(crate) fn view_kept_removed(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title: s.keep_stored_row,
                    icon: Icon::Success,
                    tone: Tone::Success,
                    result: s.keep_removed_title,
                    rows: Vec::new(),
                    actions: vec![Action::new(ids::KEEP_REMOVED_DONE, s.action_done)],
                },
            )
        })
    }

    /// The one Pad both PIN screens are (§4.3): the dots directly above
    /// the pad, the reserved caption line, and nothing else.
    fn pad_screen(
        &self,
        back: Option<ids::Id>,
        title: &'static str,
        keyboard: ids::Id,
        typed: usize,
        done: bool,
        error: Option<String>,
    ) -> Node {
        self.with_chrome(back, |c| {
            screens::pad(
                c,
                Pad {
                    words: None,
                    eye: None,
                    title: String::from(title),
                    id: keyboard,
                    kind: PadKind::Pin {
                        scramble: self.session.scramble_seed(),
                        done,
                    },
                    field: Field::Dots(typed),
                    progress: None,
                    entries: None,
                    caption: None,
                    error: error.clone(),
                    action: None,
                },
            )
        })
    }
}
