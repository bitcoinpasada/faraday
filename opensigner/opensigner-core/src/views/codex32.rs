//! The codex32 split's screens (`docs/DESIGN.md` §5): the plan as
//! Choices, the randomness as the chosen source's own screens, each
//! string on a Secret screen with the entry that types it back, and,
//! where a backup made them, the Result that states the plan.
//!
//! Reads [`Codex32Plan`] through its non-secret accessors; a string
//! reaches the screen only as the characters that screen draws.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_shell_api::SecureHardware;
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Chrome, Entry, Item, Result, Secret, Value};
use osk_ui::widgets::keyboard::{self, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::codex32::{Codex32Plan, MAX_STRINGS_MADE, Step};
use crate::create::{SOURCE_ROWS, Source};
use crate::views::create as create_view;
use crate::{OpenSigner, ids, strings};

use osk_bip::codex32::MAX_THRESHOLD;

impl OpenSigner {
    /// The screen the codex32 plan is on.
    pub(crate) fn view_codex32(&self) -> Node {
        let p = &self.codex32;
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| match p.step() {
            Step::Split => screens::choice(
                c,
                s.codex32_split_title,
                vec![
                    Item::chosen(ids::CODEX32_SPLIT_NO, s.codex32_split_no, !p.splits()),
                    Item::chosen(ids::CODEX32_SPLIT_YES, s.codex32_split_yes, p.splits()),
                ],
                Action::new(ids::CODEX32_SPLIT_CONTINUE, s.action_continue),
            ),
            Step::Count => counts(
                c,
                s.codex32_count_title,
                ids::CODEX32_COUNT_BASE,
                ids::CODEX32_COUNT_CONTINUE,
                2..=MAX_STRINGS_MADE,
                p.shares(),
                s,
            ),
            Step::Threshold => counts(
                c,
                s.codex32_threshold_title,
                ids::CODEX32_THRESHOLD_BASE,
                ids::CODEX32_THRESHOLD_CONTINUE,
                2..=p.shares().min(MAX_THRESHOLD as usize),
                p.threshold(),
                s,
            ),
            Step::Source => self.plan_source(c, p),
            // The gatherer draws the source's own Create screens.
            Step::Gather => Node::column(),
            Step::Shown => self.plan_string(c, p),
            Step::TypeBack => self.plan_type_back(c, p),
            Step::TypeSkip => create_view::skip_step(c, s),
            Step::Result => self.plan_result(c, p),
        })
    }

    /// §5 Choice, "Random shares from?": Create a key's own source rows,
    /// in the same order, with the same dimming and the same caution on
    /// "This device" (`docs/PLANNING.md` §16.92).
    fn plan_source(&self, c: &Chrome<'_>, p: &Codex32Plan) -> Node {
        let s = self.strings();
        let items = SOURCE_ROWS
            .iter()
            .enumerate()
            .map(|(i, source)| {
                let id = ids::at(ids::CODEX32_SOURCE_BASE, i);
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
            s.codex32_source_title,
            items,
            Action::new(ids::CODEX32_SOURCE_CONTINUE, s.action_continue),
        )
    }

    /// §5 Secret: one codex32 string, whole, in groups of four at the
    /// largest mono size that fits. Continue types it back; Skip goes on
    /// without, behind the caution the quiz's skip carries.
    fn plan_string(&self, c: &Chrome<'_>, p: &Codex32Plan) -> Node {
        let s = self.strings();
        let Some(string) = p.string() else {
            return Node::column();
        };
        let title = string_title(p, s);
        screens::secret(
            c,
            Secret {
                rows: Vec::new(),
                action: Some(Action::new(ids::CREATE_CONTINUE, s.action_continue)),
                secondary: Some(Action::new(ids::QUIZ_SKIP, s.codex32_skip)),
                title: &title,
                value: Value::Text(string.as_str()),
                revealed: self.revealed(ids::CREATE_REVEAL),
                panel: ids::CREATE_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                pager: None,
            },
        )
    }

    /// §5 Entry, "Type it back": the same field, mask and checksum line
    /// Load a key › Codex32 types into, over the string just shown. A
    /// string that parses but is not that one says so on the error line
    /// and stays in the field (`docs/PLANNING.md` §16.109 rule 4).
    fn plan_type_back(&self, c: &Chrome<'_>, p: &Codex32Plan) -> Node {
        let s = self.strings();
        let entry = p.entry();
        let title = alloc::format!("{} \u{00b7} {}", s.codex32_type_back, string_title(p, s));
        let error = if p.mismatch() {
            Some(String::from(s.codex32_not_same))
        } else {
            entry.error().map(|e| alloc::format!("{e}"))
        };
        let accepts = entry.accepts();
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: &title,
                value: crate::text::grouped(entry.typed()),
                mono: true,
                above: Above::Whole {
                    id: ids::CODEX32_TYPED,
                    text: String::from(entry.typed()),
                },
                words: None,
                eye: None,
                keyboard: (ids::CODEX32_KEYBOARD, KeyboardKind::Codex32),
                enabled: entry.keys() | if accepts { 0 } else { keyboard::DONE_DISABLED },
                error,
            },
        )
    }

    /// §5 Result, "Strings made": the plan the backup carries, where its
    /// randomness came from, and, for a key with words, what a codex32
    /// backup of it holds.
    fn plan_result(&self, c: &Chrome<'_>, p: &Codex32Plan) -> Node {
        let s = self.strings();
        let (threshold, shares) = p.plan();
        let plan = if threshold == 0 {
            String::from(s.codex32_split_none)
        } else {
            strings::fill(
                s.codex32_split_value,
                &[&alloc::format!("{threshold}"), &alloc::format!("{shares}")],
            )
        };
        let mut rows = vec![components::Record::text(
            s.codex32_split_row,
            plan,
            Tone::Text,
        )];
        // A seed written as one string is fed no randomness, so there is
        // no source to name.
        let device = p.source() == Source::Device;
        if threshold > 0 {
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
        }
        if p.has_words() {
            rows.push(components::Record::text(
                s.codex32_holds_row,
                s.codex32_holds_seed,
                Tone::Text,
            ));
        }
        screens::result(
            c,
            Result {
                caption: None,
                title: s.codex32_result_title,
                icon: Icon::Success,
                tone: Tone::Success,
                result: s.codex32_result,
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

/// A string's title: "Codex32" where the seed is written as one string,
/// "Share 2 of 5" where it is split.
fn string_title(p: &Codex32Plan, s: &crate::strings::Strings) -> String {
    if p.total() <= 1 {
        return String::from(s.codex32_string_title);
    }
    strings::fill(
        s.codex32_share_title,
        &[
            &alloc::format!("{}", p.show() + 1),
            &alloc::format!("{}", p.total()),
        ],
    )
}
