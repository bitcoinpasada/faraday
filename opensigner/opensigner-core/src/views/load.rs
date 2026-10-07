//! The Load wizard's screens (`docs/DESIGN.md` §5): three Choice steps,
//! the word Entry, the checksum Result, and then the finishing steps
//! shared with Create (`views/finish.rs`).
//!
//! Reads [`LoadWizard`] through its non-secret accessors: the prefix,
//! candidate indices, the checked count and language, fingerprints.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::bip39::Language;
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Candidates, Chrome, Entry, Item, Result, WordsSoFar};
use osk_ui::widgets::keyboard;
use osk_ui::widgets::{Icon, Tone};

use crate::load::{LoadWizard, ShareRefusal, Source, Step};
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    pub(crate) fn view_load(&self, w: &LoadWizard) -> Node {
        match w.step() {
            Step::Source => self.with_chrome(Some(ids::BACK), |c| self.source_step(c, w)),
            Step::Count => self.with_chrome(Some(ids::BACK), |c| self.count_step(c, w)),
            Step::Language => self.with_chrome(Some(ids::BACK), |c| self.language_step(c, w)),
            Step::Words if w.is_codex32() => {
                self.with_chrome(Some(ids::BACK), |c| self.codex32_entry_step(c, w))
            }
            Step::Words if w.is_hex() => {
                self.with_chrome(Some(ids::BACK), |c| self.load_hex_step(c, w))
            }
            Step::Words => self.with_chrome(Some(ids::BACK), |c| self.words_step(c, w, None)),
            Step::Checksum => self.with_chrome(Some(ids::BACK), |c| self.checksum_step(c, w)),
            other => self.view_finish(w.finish(), other.finish().expect("a finishing step")),
        }
    }

    /// §5 Choice, "Load from?": one row per source, the ones this build
    /// cannot take dimmed with their reason (§4.11).
    fn source_step(&self, c: &Chrome<'_>, w: &LoadWizard) -> Node {
        let s = self.strings();
        let source = w.source();
        let scan = if self.has_camera {
            Item::chosen(
                ids::LOAD_SOURCE_SCAN,
                s.load_source_scan,
                source == Source::SeedCode,
            )
        } else {
            Item::dimmed(s.load_source_scan, s.reason_needs_camera)
        };
        // A backup arrives the same way a seed code does, so it is
        // offered where the camera is.
        let backup = if self.has_camera {
            Item::chosen(
                ids::LOAD_SOURCE_BACKUP,
                s.load_source_backup,
                source == Source::Backup,
            )
        } else {
            Item::dimmed(s.load_source_backup, s.reason_needs_camera)
        };
        // §16.125 rule 1: the rows most used first, the default checked
        // and at the top.
        screens::choice(
            c,
            s.load_source_title,
            vec![
                Item::chosen(
                    ids::LOAD_SOURCE_TYPE,
                    s.load_source_type,
                    source == Source::Type,
                ),
                scan,
                backup,
                // A SLIP-39 share is typed: no standard writes one as a
                // QR (§16.107).
                Item::chosen(
                    ids::LOAD_SOURCE_SLIP39,
                    s.load_source_slip39,
                    source == Source::Slip39,
                ),
                // Combining Seed XOR parts is a load: what comes out is
                // a key that already existed.
                Item::chosen(
                    ids::LOAD_SOURCE_XOR,
                    s.load_source_xor,
                    source == Source::SeedXor,
                ),
                // A codex32 string is typed on the bech32 keyboard; a
                // scanned one reaches the same entry (§16.109 rule 3).
                Item::chosen(
                    ids::LOAD_SOURCE_CODEX32,
                    s.load_source_codex32,
                    source == Source::Codex32,
                ),
                // A word by its number on the list, and a key by the
                // entropy its words encode (§16.125 rules 2 and 3).
                Item::chosen(
                    ids::LOAD_SOURCE_NUMBERS,
                    s.load_source_numbers,
                    source == Source::Numbers,
                ),
                Item::chosen(
                    ids::LOAD_SOURCE_HEX,
                    s.load_source_hex,
                    source == Source::Hex,
                ),
                Item::dimmed(s.load_source_element, s.reason_needs_tier_b),
            ],
            Action::new(ids::LOAD_SOURCE_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "How many words?": the counts this source offers, one
    /// per row — five for a key's words, two for a SLIP-39 share.
    fn count_step(&self, c: &Chrome<'_>, w: &LoadWizard) -> Node {
        let s = self.strings();
        // Rising order on the screen; the ids keep the order the wizard
        // lists them in, which is the order every flow taps them in.
        let listed = w.counts();
        let mut counts: Vec<u8> = listed.to_vec();
        counts.sort_unstable();
        let rows = counts
            .iter()
            .map(|n| {
                let i = listed.iter().position(|c| c == n).expect("a listed count");
                Item::chosen(
                    ids::at(ids::LOAD_COUNT_BASE, i),
                    alloc::format!("{n}"),
                    *n == w.count(),
                )
            })
            .collect();
        screens::choice(
            c,
            s.load_count_title,
            rows,
            Action::new(ids::LOAD_COUNT_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Which language?": one list, English first, the
    /// wordlists this build does not carry dimmed with their reason.
    fn language_step(&self, c: &Chrome<'_>, w: &LoadWizard) -> Node {
        let s = self.strings();
        screens::choice(
            c,
            s.load_language_title,
            language_items(w.language(), ids::LOAD_LANG_BASE),
            Action::new(ids::LOAD_LANG_CONTINUE, s.action_continue),
        )
    }

    /// §5 Entry, "Word 7 of 12": the field, the candidates, the reserved
    /// error line and the BIP-39 keyboard, with the masked words so far
    /// above them where the class has the room (§4.3).
    ///
    /// §4.3: "The panel is drawn only once a word has been accepted (an
    /// empty panel says nothing), at the fixed geometry of the word count,
    /// and while it is drawn the app bar carries the eye, as on every
    /// screen with a words panel." So the panel and the eye arrive
    /// together with the first accepted word, the panel reveals under a
    /// finger like every other secret panel (§4.10), and accepting the
    /// next word masks it again.
    ///
    /// `prefix` names the run the entry belongs to where it is one of
    /// several — "Part 1", when a key is being combined from Seed XOR
    /// parts — and the title becomes "Part 1 \u{00b7} Word 7 of 12".
    pub(crate) fn words_step(&self, c: &Chrome<'_>, w: &LoadWizard, prefix: Option<&str>) -> Node {
        let s = self.strings();
        let title = strings::fill(
            s.load_word_title,
            &[
                &alloc::format!("{}", w.position() + 1),
                &alloc::format!("{}", w.count()),
            ],
        );
        let title = match prefix {
            Some(p) => alloc::format!("{p} \u{00b7} {title}"),
            None => title,
        };
        let title = match w.is_slip39() {
            true => {
                let share = strings::fill1(
                    s.load_share_word_title,
                    &alloc::format!("{}", w.share_number()),
                );
                alloc::format!("{share} \u{00b7} {title}")
            }
            false => title,
        };
        let lang = w.list();
        let candidates: Vec<String> = w.candidates().map(|i| text::shown_word(lang, i)).collect();
        // §4.3: the lone candidate is outlined where a tap accepts.
        let accepting = w.accepting();
        // §4.3: with more candidates than the strip holds, its last cell
        // pages rather than carrying a word.
        // §16.45: a Chinese list's words are one character, so its strip
        // is ten cells a row and two rows on every class.
        let one_char = lang.max_display_chars() == 1;
        let more = w.more_candidates(osk_ui::tokens::candidates(c.class(), one_char));
        // §16.43: a Chinese reading without its tone is incomplete, not
        // wrong, so the strip is empty and the error line stays quiet.
        let error = (candidates.is_empty() && !w.prefix().is_empty() && !w.awaiting_tone())
            .then(|| String::from(s.load_word_no_match));
        let so_far: Vec<String> = w
            .committed_indices()
            .map(|i| text::shown_word(lang, i))
            .collect();
        // §4.3: the panel and the eye arrive together with the first
        // accepted word, on a class that has the room for this list's
        // panel.
        let panel = !so_far.is_empty()
            && screens::words_panel_fits(
                c,
                screens::EntryRoom {
                    keyboard: w.keyboard(),
                    candidates: Some(one_char),
                },
                text::word_width(lang),
                usize::from(w.count()),
            );
        // §16.125 rule 2: on the word-numbers entry the field holds the
        // digits typed, and the strip carries the word they name.
        let numbers = w.is_numbers();
        screens::entry(
            c,
            Entry {
                title: &title,
                value: match numbers {
                    true => w.prefix().iter().collect(),
                    false => text::shown_prefix(lang, w.prefix()),
                },
                // §16.126: a word being spelled is read character by
                // character, whether the field holds the letters or the
                // digits of its number.
                mono: true,
                above: Above::Nothing,
                candidates: Some(Candidates {
                    first: ids::LOAD_CANDIDATES,
                    second: ids::LOAD_CANDIDATES_2,
                    words: candidates,
                    accepting,
                    more,
                    one_char,
                    // §16.50: the selected candidate, which the next tap
                    // on it accepts.
                    selected: w.selected_cell(),
                }),
                words: panel.then(|| WordsSoFar {
                    panel: ids::LOAD_WORDS_PANEL,
                    words: &so_far,
                    total: usize::from(w.count()),
                    revealed: self.revealed(ids::LOAD_WORDS_PANEL),
                    width: text::word_width(lang),
                }),
                eye: panel.then(|| (ids::SECRET_EYE, self.reveal_ring())),
                keyboard: (ids::LOAD_KEYBOARD, w.keyboard()),
                // §16.118: a wordlist keyboard has no ✓, so nothing on
                // it turns on what a candidate is left.
                enabled: w.enabled_keys(),
                error,
            },
        )
    }

    /// §5 Entry, "Hex entropy": the key's entropy typed on the hex
    /// keyboard, the digits masked one by one half a second after each
    /// (§4.10), ✓ dead until the count's digits are in, and ✓ taking
    /// them (`docs/PLANNING.md` §16.125 rule 3). The Create wizard's own
    /// hex step is drawn the same way.
    fn load_hex_step(&self, c: &Chrome<'_>, w: &LoadWizard) -> Node {
        let s = self.strings();
        let n = w.hex_len();
        let mut value: String = core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
        match w.hex_visible_last(self.now_ms) {
            Some(ch) => value.push(ch.to_ascii_uppercase()),
            None if n > 0 => value.push('\u{2022}'),
            None => {}
        }
        screens::entry(
            c,
            Entry {
                title: s.create_hex_title,
                value,
                mono: true,
                above: Above::Nothing,
                candidates: None,
                words: None,
                eye: None,
                keyboard: (ids::LOAD_KEYBOARD, keyboard::KeyboardKind::Hex),
                // §4.3: ✓ is dead until the input is acceptable.
                enabled: keyboard::ALL_KEYS
                    | match w.hex_ready() {
                        true => 0,
                        false => keyboard::DONE_DISABLED,
                    },
                error: None,
            },
        )
    }

    /// §5 Result: what the checksum says, and the facts it says it
    /// about. A checksum is arithmetic over the words; what it does and
    /// does not prove is a Learn page, not a line here (§2.1).
    ///
    /// The wizard is also Explore's word entry, and there the action is
    /// what it does — "Explore" — because the words become a workspace
    /// rather than a key (§4.13: the label is the verb).
    fn checksum_step(&self, c: &Chrome<'_>, w: &LoadWizard) -> Node {
        if w.is_codex32() {
            return self.codex32_share_result(c, w);
        }
        if w.is_slip39() {
            return self.share_step(c, w);
        }
        let s = self.strings();
        let count = alloc::format!("{}", w.count());
        let action = if self.explore_entry {
            s.home_explore
        } else {
            s.action_continue
        };
        if w.checksum_ok() {
            let language = text::language_name(w.language());
            let rows = vec![
                components::Record::text(s.load_words_row, count, Tone::Text),
                // A code does not name its language, and the language is
                // part of the key, so scanned words keep it one tap from
                // the result. Typed words chose it three steps ago.
                if w.scanned() {
                    components::Record::value(ids::LOAD_LANG_OTHER, s.load_language_row, language)
                } else {
                    components::Record::text(s.load_language_row, language, Tone::Text)
                },
            ];
            return screens::result(
                c,
                Result {
                    caption: None,
                    title: s.load_checksum_title,
                    icon: Icon::Success,
                    tone: Tone::Success,
                    result: s.load_checksum_valid,
                    rows,
                    actions: vec![Action::new(ids::LOAD_CONTINUE, action)],
                },
            );
        }
        let suspect = w.suspects().next();
        let mut rows = vec![components::Record::text(
            s.load_words_row,
            count,
            Tone::Text,
        )];
        if let Some(p) = suspect {
            rows.push(components::Record::text(
                s.load_suspect_label,
                strings::fill1(s.load_suspect_word, &alloc::format!("{}", p + 1)),
                Tone::Caution,
            ));
        }
        let mut actions = Vec::new();
        if let Some(p) = suspect {
            actions.push(Action::new(
                ids::at(ids::LOAD_FIX_BASE, p),
                strings::fill1(s.load_fix_word, &alloc::format!("{}", p + 1)),
            ));
        }
        actions.push(Action::new(ids::LOAD_START_OVER, s.action_start_over));
        screens::result(
            c,
            Result {
                caption: None,
                title: s.load_checksum_title,
                icon: Icon::Warning,
                tone: Tone::Caution,
                result: s.load_checksum_failed,
                rows,
                actions,
            },
        )
    }

    /// §5 Entry, "Codex32": the string typed on the bech32 keyboard,
    /// `ms1` already in the field, the characters in groups of four, and
    /// the whole string above the group as the address entry shows an
    /// address (§4.3). ✓ is live only once the whole string parses, and
    /// the line under the field is the codec's own reason once the
    /// string is a length BIP 93 allows (§16.109 rule 2).
    fn codex32_entry_step(&self, c: &Chrome<'_>, w: &LoadWizard) -> Node {
        let s = self.strings();
        let entry = w.codex32();
        let (shares, _) = entry.progress();
        let title = if shares == 0 {
            String::from(s.load_codex32_title)
        } else {
            strings::fill1(
                s.load_codex32_share_title,
                &alloc::format!("{}", entry.string_number()),
            )
        };
        let accepts = entry.accepts();
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: &title,
                value: text::grouped(entry.typed()),
                mono: true,
                above: Above::Whole {
                    id: ids::LOAD_CODEX32_TYPED,
                    text: String::from(entry.typed()),
                },
                words: None,
                eye: None,
                keyboard: (ids::LOAD_KEYBOARD, keyboard::KeyboardKind::Codex32),
                enabled: entry.keys() | if accepts { 0 } else { keyboard::DONE_DISABLED },
                error: entry.error().map(|e| alloc::format!("{e}")),
            },
        )
    }

    /// §5 Result, after a codex32 share: how many strings the set has
    /// and needs, or the codec's own reason for refusing one.
    fn codex32_share_result(&self, c: &Chrome<'_>, w: &LoadWizard) -> Node {
        let s = self.strings();
        let entry = w.codex32();
        if let Some(reason) = entry.refusal() {
            let reason = alloc::format!("{reason}");
            return screens::result(
                c,
                Result {
                    caption: None,
                    title: s.load_share_refused_title,
                    icon: Icon::Warning,
                    tone: Tone::Caution,
                    result: &reason,
                    rows: Vec::new(),
                    actions: vec![Action::new(ids::LOAD_START_OVER, s.action_start_over)],
                },
            );
        }
        let (held, needed) = entry.progress();
        let enough = held >= needed;
        screens::result(
            c,
            Result {
                caption: None,
                title: s.load_share_accepted_title,
                icon: Icon::Success,
                tone: Tone::Success,
                result: if enough {
                    s.load_share_enough
                } else {
                    s.load_share_more
                },
                rows: vec![components::Record::text(
                    s.load_codex32_shares,
                    strings::fill(
                        s.load_codex32_shares_value,
                        &[&alloc::format!("{held}"), &alloc::format!("{needed}")],
                    ),
                    Tone::Text,
                )],
                actions: vec![Action::new(ids::LOAD_CONTINUE, s.action_continue)],
            },
        )
    }

    /// §5 Result, after a share: what the set now has and needs, or why
    /// the share was refused. A refusal states the codec's own reason,
    /// which names the share that does not fit.
    fn share_step(&self, c: &Chrome<'_>, w: &LoadWizard) -> Node {
        let s = self.strings();
        if let Some(refusal) = w.share_error() {
            let reason = match refusal {
                ShareRefusal::Codec(e) => alloc::format!("{e}"),
                ShareRefusal::TooMany => String::from(s.load_share_too_many),
            };
            return screens::result(
                c,
                Result {
                    caption: None,
                    title: s.load_share_refused_title,
                    icon: Icon::Warning,
                    tone: Tone::Caution,
                    result: &reason,
                    rows: vec![components::Record::text(
                        s.load_words_row,
                        alloc::format!("{}", w.count()),
                        Tone::Text,
                    )],
                    actions: vec![Action::new(ids::LOAD_START_OVER, s.action_start_over)],
                },
            );
        }
        let (complete, needed) = w.share_group_progress();
        let mut rows = vec![components::Record::text(
            s.load_share_groups,
            strings::fill(
                s.load_share_groups_value,
                &[&alloc::format!("{complete}"), &alloc::format!("{needed}")],
            ),
            Tone::Text,
        )];
        rows.extend(w.share_groups().map(|(number, held, need)| {
            components::Record::text(
                strings::fill1(s.load_share_group, &alloc::format!("{number}")),
                strings::fill(
                    s.load_share_group_value,
                    &[&alloc::format!("{held}"), &alloc::format!("{need}")],
                ),
                Tone::Text,
            )
        }));
        screens::result(
            c,
            Result {
                caption: None,
                title: s.load_share_accepted_title,
                icon: Icon::Success,
                tone: Tone::Success,
                result: if w.shares_enough() {
                    s.load_share_enough
                } else {
                    s.load_share_more
                },
                rows,
                actions: vec![Action::new(ids::LOAD_CONTINUE, s.action_continue)],
            },
        )
    }
}

/// One row per wordlist, English first. All ten ship, and each has a
/// keyboard that types it, so no row is dead. Shared with the Create
/// wizard, which lists the same wordlists under its own ids.
pub(crate) fn language_items(selected: Language, base: u32) -> Vec<Item> {
    Language::ALL
        .iter()
        .enumerate()
        .map(|(i, lang)| {
            let name = text::language_name(*lang);
            Item::chosen(ids::at(base, i), name, *lang == selected)
        })
        .collect()
}
