//! The two screens that replace the app instead of being part of it,
//! both built from `docs/DESIGN.md` §5: a Result with Exit as the one
//! way on and no way back (§4.14 Terminal state). One is the self-test
//! naming the check that did not reproduce; the other is the refusal a
//! device whose boot was not verified gets.

use alloc::vec;

use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action};
use osk_ui::widgets::{Icon, Tone};

use crate::{OpenSigner, ids};

impl OpenSigner {
    pub(crate) fn view_selftest_failed(&self, check: &str) -> Node {
        let s = self.strings();
        // No back chevron: a build whose vectors do not reproduce has
        // nowhere to go but out.
        self.with_chrome(None, |c| {
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title: s.selftest_title,
                    icon: Icon::Error,
                    tone: Tone::Danger,
                    result: s.selftest_failed_title,
                    rows: vec![components::Record::text(
                        s.selftest_check_row,
                        check,
                        Tone::Danger,
                    )],
                    actions: vec![Action::new(ids::SELFTEST_EXIT, s.selftest_exit)],
                },
            )
        })
    }

    /// The refusal on a device whose platform did not verify the system
    /// it is running: the state, what it means, and what makes the
    /// device a signer again. Exit is the only control.
    pub(crate) fn view_boot_refused(&self) -> Node {
        let s = self.strings();
        self.with_chrome(None, |c| {
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title: s.settings_boot,
                    icon: Icon::Error,
                    tone: Tone::Danger,
                    result: s.boot_refused_title,
                    rows: vec![
                        components::Record::text(s.settings_boot, s.value_no, Tone::Danger),
                        components::Record::text(
                            s.boot_refused_system,
                            s.boot_refused_system_value,
                            Tone::Text,
                        ),
                        components::Record::text(
                            s.boot_refused_fix,
                            s.boot_refused_fix_value,
                            Tone::Text,
                        ),
                    ],
                    actions: vec![Action::new(ids::BOOT_EXIT, s.boot_refused_exit)],
                },
            )
        })
    }
}
