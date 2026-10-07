//! Tools › Dice passphrase (`docs/DESIGN.md` §5): two Choice steps, the
//! dice Pad with the words it has rolled on a masked panel above it, the
//! Secret the passphrase is, and the confirm that guards the rolls on the
//! way out.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::diceware::List;
use osk_ui::components::{self, Entries, secrets::WordWidth};
use osk_ui::layout::Node;
use osk_ui::screens::{
    self, Action, Chrome, Field, Pad, PadKind, Result, Secret, Value, WordsSoFar,
};
use osk_ui::widgets::{Icon, Tone};

use crate::dice::{DicePassphrase, Step, WORD_COUNTS};
use crate::strings::Strings;
use crate::views::create::face_name;
use crate::{OpenSigner, ids, strings};

/// How a list is named on its Choice row: its name, and under it what it
/// holds and what it costs to roll (§4.6's vocabulary — plain statements,
/// no comparatives).
fn list_name(list: List, s: &Strings) -> (&'static str, &'static str) {
    match list {
        List::Large => (s.dice_list_large, s.dice_list_large_detail),
        List::Short1 => (s.dice_list_short1, s.dice_list_short1_detail),
        List::Short2 => (s.dice_list_short2, s.dice_list_short2_detail),
    }
}

/// How wide a word of `list` is, which is what the panel pads its rows
/// to.
fn word_width(list: List) -> WordWidth {
    WordWidth {
        chars: list.max_chars(),
        sample: WordWidth::LATIN.sample,
    }
}

impl OpenSigner {
    pub(crate) fn view_dice(&self, d: &DicePassphrase) -> Node {
        self.with_chrome(Some(ids::BACK), |c| match d.step() {
            Step::List => self.dice_list_step(c, d),
            Step::Words => self.dice_words_step(c, d),
            Step::Rolls => self.dice_rolls_step(c, d),
            Step::Result => self.dice_result(c, d),
            Step::Discard => self.dice_discard(c, d),
        })
    }

    /// §5 Choice, "Which list?": the three EFF lists, the long one
    /// checked.
    fn dice_list_step(&self, c: &Chrome<'_>, d: &DicePassphrase) -> Node {
        let s = self.strings();
        let items = List::ALL
            .iter()
            .enumerate()
            .map(|(i, list)| {
                let (name, detail) = list_name(*list, s);
                screens::Item::valued(
                    ids::at(ids::DICE_LIST_BASE, i),
                    name,
                    detail,
                    *list == d.list(),
                )
            })
            .collect();
        screens::choice(
            c,
            s.dice_list_title,
            items,
            Action::new(ids::DICE_LIST_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "How many words?": the lengths, each with what it is
    /// worth in bits.
    fn dice_words_step(&self, c: &Chrome<'_>, d: &DicePassphrase) -> Node {
        let s = self.strings();
        let items = WORD_COUNTS
            .iter()
            .enumerate()
            .map(|(i, n)| {
                screens::Item::chosen(
                    ids::at(ids::DICE_WORDS_BASE, i),
                    strings::fill(
                        s.dice_words_bits,
                        &[&alloc::format!("{n}"), &alloc::format!("{}", d.bits_of(*n))],
                    ),
                    i == d.words_index(),
                )
            })
            .collect();
        screens::choice(
            c,
            s.dice_words_title,
            items,
            Action::new(ids::DICE_WORDS_CONTINUE, s.action_continue),
        )
    }

    /// §5 Pad, "Roll 23 of 30": the run so far masked in groups of five
    /// with the newest showing for half a second, the progress line and
    /// its bar, the pad, and above them the words the completed groups
    /// have named, on a masked panel with the app bar's eye (§4.10).
    fn dice_rolls_step(&self, c: &Chrome<'_>, d: &DicePassphrase) -> Node {
        let s = self.strings();
        let n = d.len();
        let need = d.needed();
        let title = strings::fill(
            s.dice_roll_title,
            &[
                &alloc::format!("{}", (n + 1).min(need)),
                &alloc::format!("{need}"),
            ],
        );
        let values: Vec<String> = d.entry_chars().map(String::from).collect();
        let count = strings::fill(
            s.dice_roll_count,
            &[
                &alloc::format!("{n}"),
                &alloc::format!("{need}"),
                &alloc::format!("{}", d.list().bits(n / d.list().dice_per_word())),
            ],
        );
        let fraction = if need == 0 {
            0.0
        } else {
            (n as f32 / need as f32).min(1.0)
        };
        let so_far: Vec<String> = d.words().into_iter().map(String::from).collect();
        // §4.3: the panel and the eye arrive together with the first
        // word, on a class that has the room for this list's panel.
        let panel = !so_far.is_empty()
            && screens::pad_words_panel_fits(
                c,
                PadKind::Dice { done: d.ready() },
                false,
                true,
                true,
                word_width(d.list()),
                d.word_count(),
            );
        screens::pad(
            c,
            Pad {
                title,
                id: ids::DICE_PAD,
                kind: PadKind::Dice { done: d.ready() },
                field: Field::Nothing,
                progress: Some((count, fraction)),
                entries: Some(Entries {
                    values: &values,
                    revealed: false,
                    flash: d.visible_last(self.now_ms).is_some(),
                }),
                words: panel.then(|| WordsSoFar {
                    panel: ids::DICE_PANEL,
                    words: &so_far,
                    total: d.word_count(),
                    revealed: self.revealed(ids::DICE_PANEL),
                    width: word_width(d.list()),
                }),
                eye: panel.then(|| (ids::SECRET_EYE, self.reveal_ring())),
                caption: None,
                error: None,
                action: Some(Action::when(
                    ids::DICE_CONTINUE,
                    s.action_continue,
                    d.ready(),
                )),
            },
        )
    }

    /// §5 Secret: the passphrase on the words panel, what it is worth
    /// under it, and Done. Nothing is stored; leaving zeroizes the rolls.
    fn dice_result(&self, c: &Chrome<'_>, d: &DicePassphrase) -> Node {
        let s = self.strings();
        let words: Vec<String> = d.words().into_iter().map(String::from).collect();
        let mut rows = vec![components::Record::text(
            s.dice_entropy_row,
            strings::fill1(s.dice_bits, &alloc::format!("{}", d.bits())),
            Tone::Text,
        )];
        let (long_run, sequential) = d.warnings();
        if long_run {
            let (run, face) = d.longest_run();
            rows.push(components::Record::text(
                s.create_longest_run,
                strings::fill(
                    s.create_run_face,
                    &[&alloc::format!("{run}"), face_name(face, s)],
                ),
                Tone::Caution,
            ));
        }
        if sequential {
            rows.push(components::Record::text(
                s.create_order_row,
                s.create_order_counting,
                Tone::Caution,
            ));
        }
        screens::secret(
            c,
            Secret {
                title: s.dice_result_title,
                value: Value::Words {
                    words: &words,
                    width: word_width(d.list()),
                },
                revealed: self.revealed(ids::DICE_PANEL),
                panel: ids::DICE_PANEL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                rows,
                action: Some(Action::new(ids::DICE_DONE, s.action_done)),
                secondary: None,
                pager: None,
            },
        )
    }

    /// §5 Result: what leaving with rolls entered costs, as one fact,
    /// and the two ways out. §4.13 gives the accent to the safe one.
    fn dice_discard(&self, c: &Chrome<'_>, d: &DicePassphrase) -> Node {
        let s = self.strings();
        screens::result(
            c,
            Result {
                caption: None,
                title: s.tools_dice_passphrase,
                icon: Icon::Warning,
                tone: Tone::Caution,
                result: s.dice_discard_title,
                rows: vec![components::Record::text(
                    s.create_rolls_row,
                    alloc::format!("{}", d.len()),
                    Tone::Text,
                )],
                actions: vec![
                    Action::new(ids::DICE_DISCARD, s.explore_discard),
                    Action::new(ids::DICE_KEEP, s.explore_keep),
                ],
            },
        )
    }
}
