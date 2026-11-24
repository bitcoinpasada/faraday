//! The Tools screen's flow: a tool with one way in drawn as the flow's
//! one card (`docs/FLOW-CONTAINER.md` §2), Input then Result. Only
//! Hashes is drawn so for now; every other tool, and every tool on a
//! small panel, draws as the page.

use super::Tool;
use crate::flow::{Contained, Row};
use crate::strings::EN;
use crate::trail::{StepFlow, Steps};
use crate::{Action, Faraday, Screen};
use faraday_text::Text;

/// The step with the field.
pub(crate) const INPUT: u8 = 0;
/// The step that shows what the tool worked out.
pub(crate) const RESULT: u8 = 1;
/// How many steps there are.
pub(crate) const COUNT: usize = 2;

/// The longest typed text a row of Your choices names, characters.
const CUT: usize = 32;

/// The card titles, by step.
pub(crate) const TITLES: [&str; COUNT] = [EN.tools_card_input, EN.tools_card_result];

/// Whether the tool open is drawn as the flow: the one place every site
/// asks, so the screen, Back, the keys and the scroll agree. A small
/// panel draws the page.
pub(crate) fn contained_tool(app: &Faraday) -> bool {
    app.screen == Screen::Tools
        && !app.compact
        && app.tools.state().is_some_and(|t| t.tool == Tool::Hashes)
}

/// Whether typed text goes into the field now: a tool drawn as the page
/// always, one drawn as the flow on its Input step.
pub(crate) fn typing_here(app: &Faraday) -> bool {
    app.tools
        .state()
        .is_some_and(|t| !contained_tool(app) || t.open == INPUT)
}

/// The tool's steps, plugged into Back, Forward and Your choices.
pub(crate) struct ToolSteps;

/// The one [`ToolSteps`].
pub(crate) static TOOL_STEPS: ToolSteps = ToolSteps;

impl StepFlow for ToolSteps {
    fn steps(&self, app: &Faraday) -> Option<Steps> {
        let t = app.tools.state()?;
        let mut steps = Steps::new((0..COUNT as u8).collect(), Some(t.open));
        steps.answered = (0..COUNT as u8).filter(|_| !t.typed.is_empty()).collect();
        Some(steps)
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::TStep(k)
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        TITLES[usize::from(k.min(RESULT))]
    }

    fn contained(&self) -> Option<&dyn Contained> {
        Some(self)
    }
}

impl Contained for ToolSteps {
    fn rows(&self, app: &Faraday) -> Vec<Row> {
        let Some(t) = app.tools.state() else {
            return Vec::new();
        };
        // What is typed, cut to the first characters.
        let all = t.typed.as_str();
        let said = &all[..all.char_indices().nth(CUT).map_or(all.len(), |(i, _)| i)];
        // The Result is not an answer: the other flows' pane shows it as
        // "Not chosen yet" (Convert), so it has no row here. The step
        // count stays.
        (0..RESULT)
            .map(|k| Row {
                label: Text::of(TITLES[usize::from(k)]),
                value: (k == INPUT && !said.is_empty()).then(|| Text::of(said)),
                mono: k == INPUT,
                opens: k,
                current: t.open == k,
                mark: None,
            })
            .collect()
    }

    fn hide_shown(&self, app: &mut Faraday) {
        // Typed text here is public: no step shows a secret.
        let _ = app;
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }
}
