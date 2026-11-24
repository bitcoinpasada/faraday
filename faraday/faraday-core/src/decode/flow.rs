//! Decode a transaction's flow: the pick (the transactions held in Files,
//! Scan and Open Files) at the left and the transaction taken apart as the
//! flow's one card (`docs/FLOW-CONTAINER.md` §2). On a small panel the
//! page draws as before.

use crate::flow::{Contained, Pick, Row};
use crate::strings::EN;
use crate::trail::{StepFlow, Steps};
use crate::{Action, Faraday, Screen};
use faraday_text::Text;

/// The step that has no transaction yet: the pick.
pub(crate) const PICK: u8 = 0;
/// The step that shows the transaction taken apart.
pub(crate) const RESULT: u8 = 1;

/// The card titles, by step.
const TITLES: [&str; 2] = [EN.decode_card_transaction, EN.decode_card_result];

/// Whether Decode a transaction is drawn as the flow: the one place every
/// site asks. A small panel draws the page.
pub(crate) fn contained_decode(app: &Faraday) -> bool {
    app.screen == Screen::Decode && !app.compact
}

/// The step open: the result once a transaction is on show.
pub(crate) fn open_step(app: &Faraday) -> u8 {
    if app.decode().is_some() { RESULT } else { PICK }
}

/// Decode a transaction's steps, plugged into Back and Your choices. Only
/// the open step is in the order: Back leaves as the page's does.
pub(crate) struct DecodeSteps;

/// The one [`DecodeSteps`].
pub(crate) static DECODE_STEPS: DecodeSteps = DecodeSteps;

impl StepFlow for DecodeSteps {
    fn steps(&self, app: &Faraday) -> Option<Steps> {
        let k = open_step(app);
        Some(Steps::new(vec![k], Some(k)))
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        if k == PICK {
            Action::DecodeAnother
        } else {
            Action::Nav(Screen::Decode)
        }
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        TITLES[usize::from(k.min(RESULT))]
    }

    fn contained(&self) -> Option<&dyn Contained> {
        Some(self)
    }
}

impl Contained for DecodeSteps {
    fn rows(&self, app: &Faraday) -> Vec<Row> {
        let Some(d) = app.decode() else {
            return Vec::new();
        };
        vec![Row {
            label: Text::of(TITLES[usize::from(PICK)]),
            value: Some(Text::of(&d.decoded.source)),
            mono: false,
            opens: PICK,
            current: false,
            mark: None,
        }]
    }

    fn hide_shown(&self, _: &mut Faraday) {
        // The hex is public: no step shows a secret.
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }

    /// The only step in the order stands without a count.
    fn heads(&self, _: &Faraday) -> usize {
        1
    }

    fn pick(&self, app: &Faraday, _: u8) -> Option<Pick> {
        // Both steps: the pick stays at the left with the one shown marked.
        Some(Pick {
            question: EN.decode_choose_transaction,
            groups: super::screen::pick_groups(app),
        })
    }
}
