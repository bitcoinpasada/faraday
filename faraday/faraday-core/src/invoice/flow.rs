//! Decode a Lightning invoice as the flow's one card on a window that is
//! not compact (`docs/FLOW-CONTAINER.md` §2): Input, then Result. A small
//! panel draws the page.

use super::InvoicePress;
use crate::flow::{Contained, Row};
use crate::strings::EN;
use crate::trail::{StepFlow, Steps};
use crate::{Action, Faraday, Screen};
use faraday_text::Text;

/// The step with the field.
pub(crate) const INPUT: u8 = 0;
/// The step that shows what the text read as.
pub(crate) const RESULT: u8 = 1;
/// How many steps there are.
pub(crate) const COUNT: usize = 2;

/// The longest typed text a row of Your choices names, characters.
const CUT: usize = 32;

/// The card titles, by step.
pub(crate) const TITLES: [&str; COUNT] = [EN.tools_card_input, EN.tools_card_result];

/// Whether the tool is drawn as the flow: the one place every site asks,
/// so the screen, Back, the keys and the scroll agree.
pub(crate) fn contained(app: &Faraday) -> bool {
    app.screen == Screen::Invoice && !app.compact && app.tools.invoice.state().is_some()
}

/// Whether typed text goes into the field now: the page always, the flow
/// on its Input step.
pub(crate) fn typing_here(app: &Faraday) -> bool {
    app.tools
        .invoice
        .state()
        .is_some_and(|s| !contained(app) || s.open == INPUT)
}

/// Opens a step; the Result waits for a reading.
pub(crate) fn open_step(app: &mut Faraday, k: u8) {
    let ready = app
        .tools
        .invoice
        .state()
        .is_some_and(|s| s.read().is_some());
    if k == INPUT || (k == RESULT && ready) {
        app.tools.invoice.show(k);
    }
}

/// The steps, plugged into Back, Forward and Your choices.
pub(crate) struct InvoiceSteps;

/// The one [`InvoiceSteps`].
pub(crate) static INVOICE_STEPS: InvoiceSteps = InvoiceSteps;

impl StepFlow for InvoiceSteps {
    fn steps(&self, app: &Faraday) -> Option<Steps> {
        let s = app.tools.invoice.state()?;
        let mut steps = Steps::new((0..COUNT as u8).collect(), Some(s.open));
        steps.answered = (0..COUNT as u8).filter(|_| s.read().is_some()).collect();
        Some(steps)
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::Invoice(InvoicePress::Step(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        TITLES[usize::from(k.min(RESULT))]
    }

    fn contained(&self) -> Option<&dyn Contained> {
        Some(self)
    }
}

impl Contained for InvoiceSteps {
    fn rows(&self, app: &Faraday) -> Vec<Row> {
        let Some(s) = app.tools.invoice.state() else {
            return Vec::new();
        };
        let all = s.text();
        let said = &all[..all.char_indices().nth(CUT).map_or(all.len(), |(i, _)| i)];
        // The Result is not an answer, so it has no row; the step count
        // stays.
        (0..RESULT)
            .map(|k| Row {
                label: Text::of(TITLES[usize::from(k)]),
                value: (!said.is_empty()).then(|| Text::of(said)),
                mono: true,
                opens: k,
                current: s.open == k,
                mark: None,
            })
            .collect()
    }

    fn hide_shown(&self, _: &mut Faraday) {
        // A Lightning invoice is public: no step shows a secret.
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }
}
