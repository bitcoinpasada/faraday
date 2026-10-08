//! The SLIP-39 split's screens (`docs/DESIGN.md` §5): the plan as
//! Choices, the randomness as the chosen source's own screens, each
//! share on the Words screen with the quiz behind it, and, where a
//! backup made them, the Result that states the plan.
//!
//! Reads [`SharePlan`] through its non-secret accessors; a share's words
//! reach the screen only as the words that screen draws.

use alloc::string::String;
use alloc::vec;

use osk_shell_api::SecureHardware;
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Chrome, Entry, Item, Result};
use osk_ui::widgets::keyboard::{ALL_KEYS, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::load::EntryList;
use crate::shares::{MAX_GROUPS, SharePlan, Step};
use crate::views::{create as create_view, quiz as quiz_view, words};
use crate::{OpenSigner, ids, strings};
use osk_entropy::{SOURCE_ROWS, Source};

impl OpenSigner {
    /// The screen the split is on.
    pub(crate) fn view_shares(&self) -> Node {
        let p = &self.shares;
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| match p.step() {
            Step::PassphraseOffer => screens::choice(
                c,
                s.passphrase_offer_title,
                vec![
                    Item::chosen(ids::LOAD_SKIP, s.passphrase_none, !p.offer_add()),
                    Item::chosen(ids::LOAD_ADD_PASSPHRASE, s.passphrase_add, p.offer_add()),
                ],
                Action::new(ids::LOAD_PASS_CONTINUE, s.action_continue),
            ),
            Step::Passphrase => self.share_passphrase(c, p),
            Step::Groups => counts(
                c,
                s.share_groups_title,
                ids::SHARE_GROUPS_BASE,
                ids::SHARE_GROUPS_CONTINUE,
                1..=MAX_GROUPS,
                p.groups(),
                s,
            ),
            Step::GroupThreshold => counts(
                c,
                s.share_quorum_title,
                ids::SHARE_QUORUM_BASE,
                ids::SHARE_QUORUM_CONTINUE,
                2..=p.groups(),
                p.group_threshold(),
                s,
            ),
            Step::Count => {
                let title = group_title(p, s.share_count_title, s);
                counts(
                    c,
                    &title,
                    ids::SHARE_COUNT_BASE,
                    ids::SHARE_COUNT_CONTINUE,
                    1..=MAX_GROUPS,
                    p.count(p.at()),
                    s,
                )
            }
            Step::Threshold => {
                let title = group_title(p, s.share_threshold_title, s);
                counts(
                    c,
                    &title,
                    ids::SHARE_THRESHOLD_BASE,
                    ids::SHARE_THRESHOLD_CONTINUE,
                    1..=p.count(p.at()),
                    p.threshold(p.at()),
                    s,
                )
            }
            Step::Source => self.share_source(c, p),
            // The gatherer draws the source's own Create screens.
            Step::Gather => Node::column(),
            Step::Words => self.share_words(c, p),
            Step::QuizStart => quiz_view::quiz_start(c, p.helper(), true, s),
            Step::Quiz => match p.quiz() {
                Some(q) if q.wrong_slot().is_some() => quiz_view::quiz_wrong(c, q, s),
                Some(q) => quiz_view::quiz_question(c, q, EntryList::Slip39, s),
                None => Node::column(),
            },
            Step::QuizSkip => create_view::skip_step(c, s),
            Step::Result => self.share_result(c, p),
        })
    }

    /// §5 Entry: the passphrase the shares are written under, as one
    /// masked line over its keyboard. Nothing checks it — every
    /// passphrase gives some wallet — so there is no comparison behind
    /// it (`docs/PLANNING.md` §16.107 rule 3).
    fn share_passphrase(&self, c: &Chrome<'_>, p: &SharePlan) -> Node {
        let n = p.passphrase_len();
        let mut masked: String = core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
        match p.passphrase_visible_char(self.now_ms) {
            Some(ch) => masked.push(ch),
            None if n > 0 => masked.push('\u{2022}'),
            None => {}
        }
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: self.strings().passphrase_title,
                value: masked,
                mono: true,
                above: screens::Above::Nothing,
                words: None,
                eye: None,
                keyboard: (ids::LOAD_PASS_KEYBOARD, KeyboardKind::Passphrase),
                enabled: ALL_KEYS,
                error: None,
            },
        )
    }

    /// §5 Choice, "Random shares from?": Create a key's own source rows,
    /// in the same order, with the same dimming and the same caution on
    /// "This device" (`docs/PLANNING.md` §16.92).
    fn share_source(&self, c: &Chrome<'_>, p: &SharePlan) -> Node {
        let s = self.strings();
        let items = SOURCE_ROWS
            .iter()
            .enumerate()
            .map(|(i, source)| {
                let id = ids::at(ids::SHARE_SOURCE_BASE, i);
                let label = create_view::source_label(i, s);
                if !self.source_available(*source) {
                    Item::dimmed(label, create_view::source_reason(*source, s))
                } else if *source == Source::Device {
                    Item::cautioned(id, label, s.create_trusts_device, *source == p.source())
                } else {
                    Item::chosen(id, label, *source == p.source())
                }
            })
            .collect();
        screens::choice(
            c,
            s.share_source_title,
            items,
            Action::new(ids::SHARE_SOURCE_CONTINUE, s.action_continue),
        )
    }

    /// §5 Words: one share, which is a secret like any other, titled by
    /// where it stands in the plan.
    fn share_words(&self, c: &Chrome<'_>, p: &SharePlan) -> Node {
        let s = self.strings();
        let Some(indices) = p.words() else {
            return Node::column();
        };
        words::words_screen(
            c,
            words::WordsScreen {
                title: share_title(p, s),
                list: EntryList::Slip39,
                indices,
                revealed: self.revealed(ids::CREATE_REVEAL),
                rows: words::Rows::Words {
                    numbers: p.numbers(),
                },
                page: p.page(),
                remaining: self.reveal_ring(),
                extra: None,
                action: Action::new(ids::CREATE_CONTINUE, s.action_continue),
            },
            s,
        )
    }

    /// §5 Result, "Shares made": the plan the backup carries — how many
    /// groups must be present and what each group holds — and where its
    /// randomness came from.
    fn share_result(&self, c: &Chrome<'_>, p: &SharePlan) -> Node {
        let s = self.strings();
        let device = p.source() == Source::Device;
        let source = if device {
            alloc::format!(
                "{} \u{00b7} {}",
                s.create_source_device,
                match self.secure() {
                    SecureHardware::Tee => s.create_device_tee,
                    SecureHardware::StrongBox => s.create_device_strongbox,
                    SecureHardware::None => s.create_device_os,
                }
            )
        } else {
            String::from(create_view::source_label(
                SOURCE_ROWS
                    .iter()
                    .position(|r| *r == p.source())
                    .unwrap_or(0),
                s,
            ))
        };
        let mut rows = vec![components::Record::text(
            s.load_share_groups,
            strings::fill(
                s.share_groups_value,
                &[
                    &alloc::format!("{}", p.group_threshold()),
                    &alloc::format!("{}", p.groups()),
                ],
            ),
            Tone::Text,
        )];
        rows.extend((0..p.groups()).map(|g| {
            components::Record::text(
                strings::fill1(s.load_share_group, &alloc::format!("{}", g + 1)),
                strings::fill(
                    s.load_share_group_value,
                    &[
                        &alloc::format!("{}", p.threshold(g)),
                        &alloc::format!("{}", p.count(g)),
                    ],
                ),
                Tone::Text,
            )
        }));
        rows.push(components::Record::text(
            s.share_source_row,
            source,
            Tone::Text,
        ));
        if device {
            rows.push(components::Record::text(
                s.create_trust_row,
                s.create_trust_device,
                Tone::Caution,
            ));
        }
        screens::result(
            c,
            Result {
                caption: None,
                title: s.slip39_result_title,
                icon: Icon::Success,
                tone: Tone::Success,
                result: s.slip39_result,
                rows,
                actions: vec![Action::new(ids::QUIZ_DONE, s.action_done)],
            },
        )
    }
}

/// §5 Choice over a run of counts: one row per number, the chosen one
/// checked. The list scrolls where it is taller than the space (§4.2).
fn counts(
    c: &Chrome<'_>,
    title: &str,
    base: u32,
    cont: ids::Id,
    range: core::ops::RangeInclusive<usize>,
    chosen: usize,
    s: &crate::strings::Strings,
) -> Node {
    let items = range
        .map(|n| Item::chosen(ids::at(base, n - 1), alloc::format!("{n}"), n == chosen))
        .collect();
    screens::choice(c, title, items, Action::new(cont, s.action_continue))
}

/// A per-group Choice's title: the group in front of the question where
/// the backup has more than one.
fn group_title(p: &SharePlan, question: &str, s: &crate::strings::Strings) -> String {
    if p.groups() <= 1 {
        return String::from(question);
    }
    alloc::format!(
        "{} \u{00b7} {question}",
        strings::fill(
            s.share_group_of,
            &[
                &alloc::format!("{}", p.at() + 1),
                &alloc::format!("{}", p.groups()),
            ],
        )
    )
}

/// A share's title: "Group 1 · Share 2 of 3", or "Share 2 of 3" where
/// the backup has one group.
fn share_title(p: &SharePlan, s: &crate::strings::Strings) -> String {
    let (group, at, of) = p.share_label();
    let share = strings::fill(
        s.share_words_title,
        &[&alloc::format!("{at}"), &alloc::format!("{of}")],
    );
    if group == 0 {
        return share;
    }
    alloc::format!(
        "{} \u{00b7} {share}",
        strings::fill1(s.share_group, &alloc::format!("{group}"))
    )
}
