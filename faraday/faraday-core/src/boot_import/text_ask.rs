//! A picture on a stick whose code holds 16 or 32 bytes of text that also
//! read as a CompactSeedQR is not loaded as a key until the person says so:
//! one sheet for each, in turn, naming the file (`docs/SCANNING.md` §1).

use super::BootImportArea;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Sheet};
use faraday_draw::widgets::Icon;
use std::collections::VecDeque;
use zeroize::Zeroizing;

/// What the sheet answers: **Load as a key** or **Leave as a file**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSeedAnswer {
    /// Load the code as a key.
    Load,
    /// Leave the picture a file; nothing is loaded.
    Leave,
}

/// One code waiting for its answer.
pub(super) struct Ask {
    /// The picture's name.
    pub(super) name: String,
    /// The code's bytes, wiped when answered.
    pub(super) code: Zeroizing<Vec<u8>>,
    /// The Import file the picture is, or `None` at a stick visit.
    pub(super) file: Option<usize>,
}

/// The codes waiting for an answer, and the sheet to return to.
#[derive(Default)]
pub(crate) struct TextAsks {
    pub(super) queue: VecDeque<Ask>,
    pub(super) back: Option<Sheet>,
}

impl BootImportArea {
    /// Whether a picture of the import waits for its answer.
    pub(crate) fn text_waits_for_import(&self) -> bool {
        self.text.queue.iter().any(|a| a.file.is_some())
    }

    /// The name of the picture the sheet asks about.
    pub(crate) fn text_front(&self) -> Option<&str> {
        self.text.queue.front().map(|a| a.name.as_str())
    }

    /// Forgets every code waiting (the import is over).
    pub(crate) fn text_forget(&mut self) {
        self.text.queue.clear();
        self.text.back = None;
    }
}

/// The sheet: the file's name, **Load as a key** and **Leave as a file**.
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(name) = app.import.text_front() else {
        return;
    };
    let title = fill(EN.text_seed_title, &[name]);
    let head = (Icon::File.into(), Role::OnSurface.paint(), title.as_str());
    crate::compact::dialog(
        ui,
        (w, h, faraday_draw::tokens::DIALOG_MAX),
        head,
        &mut |_ui, _x, _y, _iw| 0.0,
        &[
            (
                EN.text_seed_leave,
                Style::Secondary,
                Action::TextSeed(TextSeedAnswer::Leave),
            ),
            (
                EN.text_seed_load,
                Style::Secondary,
                Action::TextSeed(TextSeedAnswer::Load),
            ),
        ],
    );
}
