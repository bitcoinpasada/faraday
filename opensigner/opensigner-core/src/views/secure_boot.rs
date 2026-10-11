//! The screen a Tier A device shows once at start, built from
//! `docs/DESIGN.md` §5: a Result stating what the board checks when it
//! comes up, and Continue as the one way on (security review
//! 2026-09-11, M6).

use alloc::vec;

use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action};
use osk_ui::widgets::{Icon, Tone};

use crate::{OpenSigner, ids};

impl OpenSigner {
    pub(crate) fn view_no_secure_boot(&self) -> Node {
        let s = self.strings();
        // No back chevron: the screen is what the device opens on, and
        // Continue is the only way anywhere.
        self.with_chrome(None, |c| {
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title: s.no_secure_boot_title,
                    icon: Icon::Warning,
                    tone: Tone::Caution,
                    result: s.no_secure_boot_result,
                    rows: vec![components::Record::text(
                        s.no_secure_boot_device_row,
                        s.no_secure_boot_device,
                        Tone::Caution,
                    )],
                    actions: vec![Action::new(
                        ids::NO_SECURE_BOOT_CONTINUE,
                        s.no_secure_boot_continue,
                    )],
                },
            )
        })
    }
}
