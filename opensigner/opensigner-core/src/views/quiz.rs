//! The backup quiz's screens (`docs/DESIGN.md` §5), shared by the Create
//! wizard and the Backup flow: the start as a Menu with the helper
//! toggle, each question as a Choice with the run's progress above the
//! candidates, and a wrong answer as a Result.
//!
//! A wrong answer never names the right word, in either mode: the result
//! says the answer did not match and offers the words themselves, so the
//! whole backup is checked rather than one letter corrected.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_ui::components;
use osk_ui::geom::SizeClass;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Chrome, Item, Result, Row};
use osk_ui::widgets::{Icon, Tone};

use crate::load::EntryList;
use crate::quiz::Quiz;
use crate::strings::Strings;
use crate::{ids, strings, text};

/// The candidate words as shown, in slot order.
pub(crate) fn choice_labels(q: &Quiz, list: EntryList) -> Vec<String> {
    q.choices()
        .iter()
        .map(|&i| text::shown_word(list, i))
        .collect()
}

/// What a wrong answer says. The same words in both modes, because
/// neither may leak the right one.
pub(crate) fn wrong_message(q: &Quiz, s: &Strings) -> Option<String> {
    q.wrong_slot()?;
    Some(String::from(s.quiz_wrong_title))
}

/// The app bar's title on a quiz screen. Helper mode has no component of
/// its own in the app bar, so the title carries it, which is what an
/// auditor reads to see which mode is running (UX.md K1).
fn bar_title(base: &str, helper: bool, s: &Strings) -> String {
    if helper {
        strings::fill1(s.quiz_helper_title, base)
    } else {
        String::from(base)
    }
}

/// §5 Menu, "Verify backup": the helper toggle, and the actions that
/// start the quiz or, on the Create path, leave it. The toggle's state
/// is its value, so nothing under it explains the mode (§2.1).
pub(crate) fn quiz_start(c: &Chrome<'_>, helper: bool, skip: bool, s: &Strings) -> Node {
    let mut actions = Vec::new();
    if skip {
        actions.push(Action::new(ids::QUIZ_SKIP, s.quiz_skip));
    }
    actions.push(Action::new(ids::QUIZ_START, s.quiz_start));
    screens::menu(
        c,
        s.quiz_start_title,
        None,
        vec![Row::Toggle {
            id: ids::QUIZ_HELPER,
            label: String::from(s.quiz_helper_toggle),
            on: helper,
            reason: None,
        }],
        actions,
    )
}

/// §5 Choice, "Which is word 7?": the run's progress above the
/// candidates, none of them pre-checked, and the Continue that confirms
/// the one the tap checked (§2.7).
pub(crate) fn quiz_question(c: &Chrome<'_>, q: &Quiz, list: EntryList, s: &Strings) -> Node {
    let (done, total) = q.progress();
    let word = alloc::format!("{}", q.position() + 1);
    let count = strings::fill(
        s.words_page,
        &[&alloc::format!("{}", done + 1), &alloc::format!("{total}")],
    );
    let fraction = if total == 0 {
        0.0
    } else {
        done as f32 / total as f32
    };
    let picked = q.picked();
    let items = choice_labels(q, list)
        .into_iter()
        .enumerate()
        .map(|(i, word)| Item::chosen(ids::at(ids::QUIZ_CHOICE_BASE, i), word, picked == Some(i)))
        .collect();
    let action = Action::when(ids::QUIZ_CONTINUE, s.action_continue, picked.is_some());
    // §4.1: "the title carries progress where progress exists." Four
    // candidate rows and a Continue take the whole of a 268 dp panel, so
    // there the progress is in the title and the row above the rows goes;
    // the taller classes have the room for the bar §4.14 asks for.
    if c.class() == SizeClass::Small {
        let title = strings::fill(s.quiz_question_short, &[&word, &count]);
        return screens::choice(c, &bar_title(&title, q.helper(), s), items, action);
    }
    let question = strings::fill1(s.quiz_question, &word);
    screens::choice_with_progress(
        c,
        &bar_title(&question, q.helper(), s),
        (count, fraction),
        items,
        action,
    )
}

/// §5 Result, "Not a match": which question it was, the words as the
/// way to check the whole backup, and another go at the same word. The
/// right word is nowhere on it.
pub(crate) fn quiz_wrong(c: &Chrome<'_>, q: &Quiz, s: &Strings) -> Node {
    let (done, total) = q.progress();
    let rows = vec![components::Record::text(
        s.quiz_question_row,
        strings::fill(
            s.words_page,
            &[&alloc::format!("{}", done + 1), &alloc::format!("{total}")],
        ),
        Tone::Text,
    )];
    screens::result(
        c,
        Result {
            caption: None,
            title: &bar_title(s.quiz_start_title, q.helper(), s),
            icon: Icon::Warning,
            tone: Tone::Caution,
            result: s.quiz_wrong_title,
            rows,
            actions: vec![
                Action::new(ids::QUIZ_SHOW_WORDS, s.quiz_show_words),
                Action::new(ids::QUIZ_RETRY, s.quiz_retry),
            ],
        },
    )
}
