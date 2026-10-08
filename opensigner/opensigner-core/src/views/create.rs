//! The Create wizard's screens (`docs/DESIGN.md` §5): three Choice
//! steps, the entropy as a Pad (dice, coins) or an Entry (hex), the
//! sanity check and the quiz's cautions as Results, the math as a Record
//! of secret rows with a Secret screen behind each of them, the Words
//! screen, and then the finishing steps shared with Load
//! (`views/finish.rs`).
//!
//! Reads [`CreateWizard`] through its non-secret accessors; the entries,
//! the entropy and the words reach a screen only as what that screen
//! draws (`docs/PLANNING.md` §4.6).

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_shell_api::SecureHardware;
use osk_ui::components::{self, Entries};
use osk_ui::geom::SizeClass;
use osk_ui::layout::Node;
use osk_ui::screens::{
    self, Action, Chrome, Entry, Field, Item, Pad, PadKind, Record, Result, Row, Scanner, Secret,
    Value, WordsSoFar,
};
use osk_ui::widgets::keyboard::{self, ALL_KEYS, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use osk_entropy::DiceProcedure;

use crate::create::{CreateWizard, Step};
use crate::load::EntryList;
use crate::strings::Strings;
use crate::views::{load, quiz as quiz_view, words};
use crate::{OpenSigner, ids, strings, text};
use osk_entropy::{MIX_SOURCES, SOURCE_ROWS, Source};

/// The title of a screen the wizard is drawing, with the part it is
/// making in front of it while a Seed XOR split runs the wizard once
/// per random part (`docs/PLANNING.md` §16.92). The scheme is one
/// title, `Part 1 of 3 \u{00b7} Roll 12 of 50`, on every source's
/// screens, as a typed part's entry already reads `Part 1 \u{00b7}
/// Word 3 of 12`.
fn titled(w: &CreateWizard, s: &Strings, title: impl Into<String>) -> String {
    let title = title.into();
    match w.part_of() {
        Some((at, of)) => alloc::format!(
            "{} \u{00b7} {title}",
            strings::fill(
                s.xor_part_of,
                &[&alloc::format!("{}", at + 1), &alloc::format!("{of}")],
            )
        ),
        None => title,
    }
}

impl OpenSigner {
    pub(crate) fn view_create(&self, w: &CreateWizard) -> Node {
        match w.step() {
            Step::Source => {
                self.with_chrome(Some(ids::BACK), |c| source_step(c, w, self.strings()))
            }
            Step::Count => self.with_chrome(Some(ids::BACK), |c| count_step(c, w, self.strings())),
            Step::Language => {
                self.with_chrome(Some(ids::BACK), |c| language_step(c, w, self.strings()))
            }
            Step::Procedure => {
                self.with_chrome(Some(ids::BACK), |c| procedure_step(c, w, self.strings()))
            }
            Step::Entropy => self.with_chrome(Some(ids::BACK), |c| self.entropy_step(c, w)),
            Step::Camera => self.with_chrome(Some(ids::BACK), |c| self.camera_step(c, w)),
            Step::Device => self.with_chrome(Some(ids::BACK), |c| self.device_step(c, w)),
            Step::MixChoose => {
                self.with_chrome(Some(ids::BACK), |c| mix_step(c, w, self.strings()))
            }
            Step::MixResult => {
                self.with_chrome(Some(ids::BACK), |c| mixed_step(c, w, self.strings()))
            }
            Step::XorPart => self.with_chrome(Some(ids::BACK), |c| self.xor_part_step(c, w)),
            Step::Sanity => {
                self.with_chrome(Some(ids::BACK), |c| sanity_step(c, w, self.strings()))
            }
            Step::Math => self.with_chrome(Some(ids::BACK), |c| math_step(c, w, self.strings())),
            Step::MathEntropy | Step::MathBits | Step::MathWord => {
                self.with_chrome(Some(ids::BACK), |c| self.math_secret(c, w))
            }
            Step::Words => self.with_chrome(Some(ids::BACK), |c| self.create_words_step(c, w)),
            // The first run has no way past the quiz: a key nobody has
            // checked a backup of is what §7.1 exists to prevent.
            Step::QuizStart => self.with_chrome(Some(ids::BACK), |c| {
                quiz_view::quiz_start(c, w.helper(), self.first_run_done(), self.strings())
            }),
            Step::Quiz => self.with_chrome(Some(ids::BACK), |c| match w.quiz() {
                Some(q) if q.wrong_slot().is_some() => quiz_view::quiz_wrong(c, q, self.strings()),
                Some(q) => {
                    quiz_view::quiz_question(c, q, EntryList::Bip39(w.language()), self.strings())
                }
                None => Node::column(),
            }),
            Step::QuizSkip => self.with_chrome(Some(ids::BACK), |c| skip_step(c, self.strings())),
            other => self.view_finish(w.finish(), other.finish().expect("a finishing step")),
        }
    }

    /// §5 Entry then §5 Result: one Seed XOR part, typed word by word
    /// on the Load wizard's own entry screen, and then the fingerprint
    /// that part comes to with the two ways on — one more part, or the
    /// combination (`docs/PLANNING.md` §8.2 item 14).
    ///
    /// The fingerprint is the part's own: loading that part as a key
    /// gives it, which is how a person tells one part from another
    /// without putting the key back together.
    fn xor_part_step(&self, c: &Chrome<'_>, w: &CreateWizard) -> Node {
        let s = self.strings();
        let n = alloc::format!("{}", w.xor_parts() + 1);
        let part = strings::fill1(s.xor_part, &n);
        if w.entry().step() == crate::load::Step::Words {
            return self.words_step(c, w.entry(), Some(&part));
        }
        let ok = w.entry().checksum_ok();
        let fingerprint = w
            .xor_print()
            .map_or_else(|| String::from(s.value_none), text::fingerprint_hex);
        let rows = if ok {
            vec![components::Record::fingerprint(s.confirm_key, fingerprint)]
        } else {
            vec![components::Record::text(
                s.load_words_row,
                alloc::format!("{}", w.entry().count()),
                Tone::Danger,
            )]
        };
        let mut actions = Vec::new();
        if w.xor_room() {
            actions.push(Action::when(ids::XOR_ADD_ANOTHER, s.xor_add_another, ok));
        }
        actions.push(Action::when(
            ids::XOR_COMBINE,
            s.xor_combine,
            ok && w.xor_ready(),
        ));
        screens::result(
            c,
            Result {
                caption: None,
                title: &part,
                icon: if ok { Icon::Success } else { Icon::Error },
                tone: if ok { Tone::Success } else { Tone::Danger },
                result: if ok {
                    s.xor_part_result
                } else {
                    s.load_checksum_failed
                },
                rows,
                actions,
            },
        )
    }

    /// §5 Pad, "Roll 23 of 50": the run so far masked in groups of five
    /// with the newest showing for half a second, the progress line and
    /// its bar, and the pad, all one bottom group. §5 Entry for hex,
    /// where the digits are typed into a field over the hex keyboard.
    fn entropy_step(&self, c: &Chrome<'_>, w: &CreateWizard) -> Node {
        let s = self.strings();
        if w.active() == Source::Hex {
            return self.hex_step(c, w);
        }
        // §4.3: "✓ dimmed until the input is acceptable", which for a
        // run of entropy is the target the count asks for — the same
        // state the Continue under the pad is in.
        let done = w.entry_ready();
        let cards = w.active() == Source::Cards;
        let (template, kind) = match w.active() {
            Source::Coins => (s.create_coin_title, PadKind::Coin { done }),
            Source::Cards => (s.create_cards_title, PadKind::Cards { done }),
            _ => (s.create_dice_title, PadKind::Dice { done }),
        };
        let n = w.entry_len();
        let need = w.entry_needed();
        let title = titled(
            w,
            s,
            strings::fill(
                template,
                &[
                    &alloc::format!("{}", (n + 1).min(need)),
                    &alloc::format!("{need}"),
                ],
            ),
        );
        let values: Vec<String> = if cards {
            card_strip(w)
        } else {
            w.entry_chars().map(String::from).collect()
        };
        // §4.3: direct selection shows each word as its six rolls
        // complete it, on the masked panel the dice passphrase uses.
        let direct = w.active() == Source::Dice && w.procedure().direct();
        let so_far: Vec<String> = if direct {
            w.dice_word_indices()
                .map(|i| text::shown_word(EntryList::Bip39(w.language()), i))
                .collect()
        } else {
            Vec::new()
        };
        let width = text::word_width(EntryList::Bip39(w.language()));
        let panel = !so_far.is_empty()
            && screens::pad_words_panel_fits(
                c,
                PadKind::Dice { done },
                false,
                true,
                true,
                width,
                w.dice_words_needed(),
            );
        let entries = Entries {
            values: &values,
            // The pad screen has no eye and no surface to hold: the run
            // shows one entry at a time, as it is entered (§4.3).
            revealed: false,
            flash: w.entry_fresh(self.now_ms),
        };
        let count = strings::fill(
            s.create_entry_count,
            &[
                &alloc::format!("{n}"),
                &alloc::format!("{need}"),
                &alloc::format!("{:.0}", w.entry_bits()),
            ],
        );
        let fraction = if need == 0 {
            0.0
        } else {
            (n as f32 / need as f32).min(1.0)
        };
        screens::pad(
            c,
            Pad {
                words: panel.then(|| WordsSoFar {
                    panel: ids::CREATE_REVEAL,
                    words: &so_far,
                    total: w.dice_words_needed(),
                    revealed: self.revealed(ids::CREATE_REVEAL),
                    width,
                }),
                eye: panel.then(|| (ids::SECRET_EYE, self.reveal_ring())),
                title,
                id: ids::CREATE_PAD,
                kind,
                // §4.3's field row: the card pad states which deck
                // the draws come from, because a 24-word key needs more
                // cards than a deck holds and the tracker starts again
                // when the second one is shuffled.
                field: Field::Nothing,
                caption: if cards {
                    Some(strings::fill1(
                        s.create_card_deck,
                        &alloc::format!("{}", w.cards().deck()),
                    ))
                } else {
                    None
                },
                progress: Some((count, fraction)),
                entries: Some(entries),
                // §4.3's reserved caption line: what the deck tracker
                // refused.
                error: if cards && w.card_repeat() {
                    Some(String::from(s.create_already_drawn))
                } else if direct && w.rerolled() {
                    Some(String::from(s.create_rerolled))
                } else {
                    None
                },
                action: Some(Action::when(
                    ids::CREATE_CONTINUE,
                    s.action_continue,
                    w.entry_ready(),
                )),
            },
        )
    }

    /// §5 Entry, "Hex entropy": the digits typed, masked one by one
    /// half a second after each (§4.10), over the hex keyboard, with the
    /// reserved caption line under the field.
    fn hex_step(&self, c: &Chrome<'_>, w: &CreateWizard) -> Node {
        let s = self.strings();
        let n = w.entry_len();
        let mut value: String = core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
        match w.entry_visible_last(self.now_ms) {
            Some(ch) => value.push(ch.to_ascii_uppercase()),
            None if n > 0 => value.push('\u{2022}'),
            None => {}
        }
        let error = w
            .entry_too_short()
            .then(|| String::from(s.create_too_short));
        let title = titled(w, s, s.create_hex_title);
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: &title,
                value,
                mono: true,
                above: screens::Above::Nothing,
                words: None,
                eye: None,
                keyboard: (ids::CREATE_PAD, KeyboardKind::Hex),
                // §4.3: ✓ is dead until the input is acceptable.
                enabled: ALL_KEYS
                    | if w.entry_ready() {
                        0
                    } else {
                        keyboard::DONE_DISABLED
                    },
                error,
            },
        )
    }

    /// §5 Scanner turned into a viewfinder: the camera's frames with
    /// the count inside the square and the shutter under it. A frame
    /// with too little variation is refused, and the state line inside
    /// the square says so, which is where this screen's caption is
    /// (§4.9).
    fn camera_step(&self, c: &Chrome<'_>, w: &CreateWizard) -> Node {
        let s = self.strings();
        let n = w.entry_len();
        let need = w.entry_needed();
        let refused = w
            .frame_stats()
            .is_some_and(|f| f.distinct < osk_entropy::CAMERA_MIN_DISTINCT);
        let state = if refused {
            String::from(s.create_too_little_variation)
        } else {
            strings::fill(
                s.create_camera_state,
                &[
                    &alloc::format!("{}", (n + 1).min(need.max(1))),
                    &alloc::format!("{need}"),
                ],
            )
        };
        let fraction = if need == 0 {
            0.0
        } else {
            (n as f32 / need as f32).min(1.0)
        };
        let title = titled(w, s, s.create_camera_title);
        screens::scanner(
            c,
            Scanner {
                title: &title,
                state: &state,
                parts: Some(fraction),
                ways: Vec::new(),
                preview: self.camera_preview(),
                action: Some(Action::new(ids::CREATE_SHUTTER, s.create_camera_shutter)),
            },
        )
    }

    /// §5 Result, "Device randomness": what produced the bytes, and the
    /// one thing worth saying about them. The argument is in Learn; a
    /// working screen carries the fact and the caution and nothing else.
    fn device_step(&self, c: &Chrome<'_>, w: &CreateWizard) -> Node {
        let s = self.strings();
        let generator = match self.secure() {
            SecureHardware::Tee => s.create_device_tee,
            SecureHardware::StrongBox => s.create_device_strongbox,
            SecureHardware::None => s.create_device_os,
        };
        let title = titled(w, s, s.create_device_title);
        screens::result(
            c,
            Result {
                caption: None,
                title: &title,
                icon: Icon::Warning,
                tone: Tone::Caution,
                result: s.create_device_result,
                rows: vec![
                    components::Record::text(s.create_device_row, generator, Tone::Text),
                    // What the bytes rest on is a fact about them, and
                    // a fact is a row: the tone says how it is read.
                    components::Record::text(
                        s.create_trust_row,
                        s.create_trust_device,
                        Tone::Caution,
                    ),
                ],
                actions: vec![Action::when(
                    ids::CREATE_CONTINUE,
                    s.action_continue,
                    w.device_ready(),
                )],
            },
        )
    }

    /// §5 Words: the panel, the pager where the class pages, the eye,
    /// "Numbers" and Continue.
    fn create_words_step(&self, c: &Chrome<'_>, w: &CreateWizard) -> Node {
        let s = self.strings();
        let Some(m) = w.mnemonic() else {
            return Node::column();
        };
        words::words_screen(
            c,
            words::WordsScreen {
                title: String::from(s.words_title),
                list: EntryList::Bip39(w.language()),
                indices: m.indices(),
                revealed: self.revealed(ids::CREATE_REVEAL),
                rows: words::Rows::Words {
                    numbers: w.numbers(),
                },
                page: w.page(),
                remaining: self.reveal_ring(),
                extra: None,
                action: Action::new(ids::CREATE_CONTINUE, s.action_continue),
            },
            s,
        )
    }

    /// §5 Secret: one of the math's three values, whole, on its own
    /// screen (§2.8).
    fn math_secret(&self, c: &Chrome<'_>, w: &CreateWizard) -> Node {
        let s = self.strings();
        let Some(m) = w.mnemonic() else {
            return Node::column();
        };
        let revealed = self.revealed(ids::CREATE_REVEAL);
        let (title, value) = match w.step() {
            Step::MathEntropy => (
                String::from(s.create_math_entropy),
                text::hex(m.entropy().expose().as_bytes()),
            ),
            Step::MathBits => (String::from(s.create_bits_row), checksum_bits(w)),
            _ => (
                strings::fill1(s.create_math_word, "1"),
                text::shown_word(EntryList::Bip39(w.language()), m.indices()[0]),
            ),
        };
        // The screen owns the mask: a masked panel keeps the string's
        // rectangle and draws one eye-slash in it (§4.10), so the value
        // is handed over whole in both states.
        screens::secret(
            c,
            Secret {
                rows: Vec::new(),
                action: None,
                title: &title,
                value: Value::Text(&value),
                revealed,
                panel: ids::CREATE_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                secondary: None,
                pager: None,
            },
        )
    }
}

/// §5 Choice, "Create from?": one row per source, the ones this build
/// cannot take dimmed with their reason (§4.11), and the one that has
/// something to weigh carrying it at its trailing edge (§4.8).
fn source_step(c: &Chrome<'_>, w: &CreateWizard, s: &Strings) -> Node {
    let items = SOURCE_ROWS
        .iter()
        .enumerate()
        .map(|(i, source)| {
            let id = ids::at(ids::CREATE_SOURCE_BASE, i);
            let label = source_label(i, s);
            if !w.available(*source) {
                Item::dimmed(label, source_reason(*source, s))
            } else if *source == Source::Device {
                Item::cautioned(id, label, s.create_trusts_device, *source == w.source())
            } else {
                Item::chosen(id, label, *source == w.source())
            }
        })
        .collect();
    screens::choice(
        c,
        s.create_source_title,
        items,
        Action::new(ids::CREATE_SOURCE_CONTINUE, s.action_continue),
    )
}

/// The label of a dice procedure, in `DiceProcedure::ALL` order.
fn procedure_label(procedure: DiceProcedure, s: &Strings) -> &'static str {
    match procedure {
        DiceProcedure::Hashed => s.dice_procedure_hashed,
        DiceProcedure::SixAsZero => s.dice_procedure_six_as_zero,
        DiceProcedure::Words => s.dice_procedure_words,
    }
}

/// §5 Choice, "Which procedure?": one row per published dice procedure,
/// the hashed one checked first, each with the rolls it takes at the
/// chosen length under its name (`docs/PLANNING.md` §16.115).
fn procedure_step(c: &Chrome<'_>, w: &CreateWizard, s: &Strings) -> Node {
    let items = DiceProcedure::ALL
        .iter()
        .enumerate()
        .map(|(i, procedure)| {
            let label = procedure_label(*procedure, s);
            if !w.procedure_available(*procedure) {
                return Item::dimmed(label, s.reason_no_words);
            }
            let rolls = alloc::format!("{}", w.procedure_rolls(*procedure));
            let under = strings::fill1(s.dice_procedure_rolls, &rolls);
            Item::key(
                ids::at(ids::CREATE_PROCEDURE_BASE, i),
                label,
                under,
                *procedure == w.chosen_procedure(),
            )
        })
        .collect();
    let title = titled(w, s, s.dice_procedure_title);
    screens::choice(
        c,
        &title,
        items,
        Action::new(ids::CREATE_PROCEDURE_CONTINUE, s.action_continue),
    )
}

/// §5 Choice, "How many words?": every BIP-39 count, in the order the
/// Load wizard lists them, or a SLIP-39 share's 20 and 33. A codex32
/// key has no words, so the same step asks "How long a seed?" over the
/// five lengths an entropy source makes (`docs/PLANNING.md` §16.109
/// rule 4).
fn count_step(c: &Chrome<'_>, w: &CreateWizard, s: &Strings) -> Node {
    if w.is_codex32() {
        let chosen = w.strength();
        let items = w
            .strengths()
            .iter()
            .enumerate()
            .map(|(i, bits)| {
                Item::chosen(
                    ids::at(ids::CREATE_COUNT_BASE, i),
                    strings::fill1(s.codex32_length_row, &alloc::format!("{}", bits.bits())),
                    *bits == chosen,
                )
            })
            .collect();
        return screens::choice(
            c,
            s.codex32_length_title,
            items,
            Action::new(ids::CREATE_COUNT_CONTINUE, s.action_continue),
        );
    }
    let items = w
        .counts()
        .iter()
        .enumerate()
        .map(|(i, n)| {
            Item::chosen(
                ids::at(ids::CREATE_COUNT_BASE, i),
                alloc::format!("{n}"),
                *n == w.count(),
            )
        })
        .collect();
    screens::choice(
        c,
        s.create_count_title,
        items,
        Action::new(ids::CREATE_COUNT_CONTINUE, s.action_continue),
    )
}

/// §5 Choice, "Which language?": one list, English first, the wordlists
/// this build does not carry dimmed with their reason. The same list the
/// Load wizard shows, under Create's own ids.
fn language_step(c: &Chrome<'_>, w: &CreateWizard, s: &Strings) -> Node {
    screens::choice(
        c,
        s.create_language_title,
        load::language_items(w.language(), ids::CREATE_LANG_BASE),
        Action::new(ids::CREATE_LANG_CONTINUE, s.action_continue),
    )
}

/// The label of source row `i`, in [`SOURCE_ROWS`] order.
pub(crate) fn source_label(i: usize, s: &Strings) -> &'static str {
    match i {
        0 => s.create_source_dice,
        1 => s.create_source_coins,
        2 => s.create_source_hex,
        3 => s.create_source_cards,
        4 => s.create_source_camera,
        5 => s.create_source_mix,
        _ => s.create_source_device,
    }
}

/// Why a source is dimmed, in two or three words (§4.11).
pub(crate) fn source_reason(source: Source, s: &Strings) -> &'static str {
    match source {
        // Tier D runs in a browser, where the generator, the memory and
        // the page all belong to something else.
        Source::Device => s.reason_browser,
        _ => s.reason_needs_camera,
    }
}

/// The label of a mix row, in [`MIX_SOURCES`] order.
fn mix_label(source: Source, s: &Strings) -> &'static str {
    match source {
        Source::Coins => s.create_source_coins,
        Source::Cards => s.create_source_cards,
        Source::Camera => s.create_source_camera,
        Source::Device => s.create_source_device,
        _ => s.create_source_dice,
    }
}

/// §5 Menu, "Mix which sources?": one toggle row per source, and
/// Continue under them. §4.2's Choice takes one row; a mix takes
/// several, so the rows are §4.1's toggles and the Continue is dimmed
/// until two are on.
fn mix_step(c: &Chrome<'_>, w: &CreateWizard, s: &Strings) -> Node {
    let rows = MIX_SOURCES
        .iter()
        .enumerate()
        .map(|(i, source)| {
            if w.available(*source) {
                Row::Toggle {
                    id: ids::at(ids::CREATE_MIX_BASE, i),
                    label: String::from(mix_label(*source, s)),
                    on: w.mix_chosen(i),
                    reason: None,
                }
            } else {
                Row::Dimmed {
                    icon: Some(Icon::Info),
                    label: String::from(mix_label(*source, s)),
                    reason: Some(String::from(source_reason(*source, s))),
                }
            }
        })
        .collect();
    let title = titled(w, s, s.create_mix_title);
    screens::menu(
        c,
        &title,
        None,
        rows,
        vec![Action::when(
            ids::CREATE_MIX_CONTINUE,
            s.action_continue,
            w.mix_count() >= osk_entropy::MIN_MIX,
        )],
    )
}

/// §5 Result, "Mixed": one reference row per source, holding the
/// SHA-256 of what that source put in. Writing them down before the
/// words appear is what makes the combination checkable afterwards: the
/// key is SHA-256 over exactly these, in exactly this order.
fn mixed_step(c: &Chrome<'_>, w: &CreateWizard, s: &Strings) -> Node {
    let rows = w
        .mix_taken()
        .zip(w.mix_commitments())
        .enumerate()
        .map(|(i, (source, commitment))| {
            components::Record::reference(
                ids::at(ids::CREATE_MIX_BASE, i),
                mix_label(source, s),
                text::hex(commitment),
            )
        })
        .collect();
    let title = titled(w, s, s.create_mixed_title);
    screens::result(
        c,
        Result {
            caption: None,
            title: &title,
            icon: Icon::Success,
            tone: Tone::Success,
            result: s.create_mixed_result,
            rows,
            actions: vec![Action::new(ids::CREATE_CONTINUE, s.action_continue)],
        },
    )
}

/// The cards drawn, newest last, as `A♠ 7♦ …`, with the half-typed card
/// at the end so that a rank or a suit on its own is visible as it is
/// tapped.
fn card_strip(w: &CreateWizard) -> Vec<String> {
    let mut out: Vec<String> = w
        .cards()
        .cards()
        .iter()
        .map(|&c| {
            let (rank, suit) = osk_entropy::card_parts(c);
            let mut label = String::new();
            label.push(osk_entropy::RANKS[usize::from(rank)]);
            label.push(osk_entropy::SUITS[usize::from(suit)]);
            label
        })
        .collect();
    let half: String = w
        .card_rank()
        .map(|r| osk_entropy::RANKS[usize::from(r)])
        .into_iter()
        .chain(w.card_suit().map(|u| osk_entropy::SUITS[usize::from(u)]))
        .collect();
    if !half.is_empty() {
        out.push(half);
    }
    out
}

/// The plural name of a dice face, so the longest run reads as what was
/// rolled — "8 sixes" — rather than as an arithmetic expression (plain
/// language; the review of 2026-09-09, §3.3).
pub(crate) fn face_name(face: u8, s: &Strings) -> &'static str {
    match face {
        1 => s.create_face_1,
        2 => s.create_face_2,
        3 => s.create_face_3,
        4 => s.create_face_4,
        5 => s.create_face_5,
        _ => s.create_face_6,
    }
}

/// §5 Result: what the entries look like, and the facts that say it. The
/// statistics are the record; what a chi-square is belongs in Learn, not
/// on a working screen (§2.1).
fn sanity_step(c: &Chrome<'_>, w: &CreateWizard, s: &Strings) -> Node {
    let warnings = w.warnings();
    let bits = alloc::format!("{:.0}", w.entry_bits());
    let n = w.entry_len();
    let count = alloc::format!("{n}");
    // A 268 dp panel gives the value column one line, and the count is
    // already the row above, so there the verdict is the word alone.
    let short = c.class() == SizeClass::Small;
    let verdict = |ok: bool| match (ok, short) {
        (true, true) => String::from(s.create_stat_normal_short),
        (false, true) => String::from(s.create_stat_high_short),
        (true, false) => strings::fill1(s.create_stat_normal, &count),
        (false, false) => strings::fill1(s.create_stat_high, &count),
    };
    let tone = |ok: bool| if ok { Tone::Text } else { Tone::Caution };
    let mut rows = Vec::new();
    match w.active() {
        // Direct selection keeps only 1–4 in a word's five faces and
        // reads the sixth roll as a coin, so the two streams are
        // counted apart, as EntropyLab's own analysis counts them.
        Source::Dice if w.procedure().direct() => {
            let d = w.dice().stats();
            let ((faces, chi_square), (heads, tails)) = w.dice().word_stats();
            rows.push(components::Record::text(
                s.create_rolls_row,
                count.clone(),
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.create_words_row,
                alloc::format!("{}", w.dice().word_count()),
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.create_bits_row,
                bits,
                Tone::Text,
            ));
            let face_count = alloc::format!("{}", faces.iter().sum::<u32>());
            rows.push(components::Record::text(
                s.create_chi_square,
                alloc::format!(
                    "{chi_square:.1} \u{00b7} {}",
                    match (!warnings.skewed, short) {
                        (true, true) => String::from(s.create_stat_normal_short),
                        (false, true) => String::from(s.create_stat_high_short),
                        (true, false) => strings::fill1(s.create_stat_normal, &face_count),
                        (false, false) => strings::fill1(s.create_stat_high, &face_count),
                    }
                ),
                tone(!warnings.skewed),
            ));
            rows.push(components::Record::text(
                s.create_heads,
                alloc::format!("{heads}"),
                tone(!warnings.skewed),
            ));
            rows.push(components::Record::text(
                s.create_tails,
                alloc::format!("{tails}"),
                tone(!warnings.skewed),
            ));
            rows.push(components::Record::text(
                s.create_longest_run,
                strings::fill(
                    s.create_run_face,
                    &[
                        &alloc::format!("{}", d.longest_run),
                        face_name(d.run_face, s),
                    ],
                ),
                tone(!warnings.long_run),
            ));
        }
        Source::Dice => {
            let d = w.dice().stats();
            rows.push(components::Record::text(
                s.create_rolls_row,
                count.clone(),
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.create_bits_row,
                bits,
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.create_chi_square,
                alloc::format!("{:.1} \u{00b7} {}", d.chi_square, verdict(!warnings.skewed)),
                tone(!warnings.skewed),
            ));
            rows.push(components::Record::text(
                s.create_longest_run,
                strings::fill(
                    s.create_run_face,
                    &[
                        &alloc::format!("{}", d.longest_run),
                        face_name(d.run_face, s),
                    ],
                ),
                tone(!warnings.long_run),
            ));
            if warnings.sequential {
                rows.push(components::Record::text(
                    s.create_order_row,
                    s.create_order_counting,
                    Tone::Caution,
                ));
            }
        }
        Source::Coins => {
            let f = w.coins().stats();
            rows.push(components::Record::text(
                s.create_flips_row,
                count,
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.create_bits_row,
                bits,
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.create_heads,
                alloc::format!("{}", f.heads),
                tone(!warnings.skewed),
            ));
            rows.push(components::Record::text(
                s.create_tails,
                alloc::format!("{}", f.tails),
                tone(!warnings.skewed),
            ));
            rows.push(components::Record::text(
                s.create_longest_run,
                alloc::format!("{}", f.longest_run),
                tone(!warnings.long_run),
            ));
        }
        Source::Cards => {
            let d = w.cards().stats();
            rows.push(components::Record::text(
                s.create_cards_row,
                count,
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.create_bits_row,
                bits,
                Tone::Text,
            ));
            for (i, n) in d.suits.iter().enumerate() {
                let mut suit = String::new();
                suit.push(osk_entropy::SUITS[i]);
                rows.push(components::Record::text(
                    suit,
                    alloc::format!("{n}"),
                    Tone::Text,
                ));
            }
        }
        // The camera's numbers are facts about the last picture, not a
        // verdict on it: the one judgement this source makes is the
        // refusal of a frame with too little variation, and that
        // happens before the frame is ever counted.
        Source::Camera => {
            rows.push(components::Record::text(
                s.create_frames_row,
                count,
                Tone::Text,
            ));
            if let Some(f) = w.frame_stats() {
                rows.push(components::Record::text(
                    s.create_frame_size_row,
                    alloc::format!("{}", f.pixels),
                    Tone::Text,
                ));
                rows.push(components::Record::text(
                    s.create_distinct_row,
                    alloc::format!("{}", f.distinct),
                    Tone::Text,
                ));
                rows.push(components::Record::text(
                    s.create_mean_row,
                    alloc::format!("{:.1}", f.mean),
                    Tone::Text,
                ));
                rows.push(components::Record::text(
                    s.create_variance_row,
                    alloc::format!("{:.1}", f.variance),
                    Tone::Text,
                ));
            }
        }
        // A Seed XOR key never reaches this screen: its parts are typed
        // as words and the key goes straight to the finishing steps.
        _ => {
            rows.push(components::Record::text(
                s.create_digits_row,
                count,
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.create_bits_row,
                bits,
                Tone::Text,
            ));
        }
    }
    // §4.11: a row that opens a screen is a value row in the table's
    // flow, so the way into the math looks like every other row here.
    // A split is not making a key, so it does not offer the math.
    if w.part_of().is_none() {
        rows.push(components::Record::value(
            ids::CREATE_MATH,
            s.create_math_row,
            String::new(),
        ));
    }
    let (icon, tone, result) = match (w.active(), warnings.any_caution()) {
        // No statistic checks typed entropy, a shuffled deck or a
        // picture: the key is as good as the source it came from, and
        // the screen says neither more nor less.
        (Source::Dice | Source::Coins, true) => {
            (Icon::Warning, Tone::Caution, s.create_sanity_uneven)
        }
        (Source::Dice | Source::Coins, false) => {
            (Icon::Success, Tone::Success, s.create_sanity_random)
        }
        _ => (Icon::Info, Tone::Muted, s.create_sanity_unchecked),
    };
    let title = titled(w, s, s.create_sanity_title);
    screens::result(
        c,
        Result {
            caption: None,
            title: &title,
            icon,
            tone,
            result,
            rows,
            actions: vec![
                Action::new(ids::CREATE_AGAIN, again_label(w.active(), s)),
                Action::new(ids::CREATE_CONTINUE, s.action_continue),
            ],
        },
    )
}

/// The verb the "again" action uses, which is the verb of the source.
fn again_label(source: Source, s: &Strings) -> &'static str {
    match source {
        Source::Cards => s.create_sanity_again_draw,
        Source::Camera => s.create_sanity_again_take,
        _ => s.create_sanity_again,
    }
}

/// §5 Record, "The math": the entropy, the checksum and the first word,
/// each a row that opens the value on its own Secret screen (§2.8).
fn math_step(c: &Chrome<'_>, w: &CreateWizard, s: &Strings) -> Node {
    let Some(m) = w.mnemonic() else {
        return Node::column();
    };
    let (_, cs_bits) = m.checksum_bits();
    let mut rows = Vec::new();
    // §16.115: with more than one published procedure on offer, the
    // math names which one these words came out of.
    if w.source() == Source::Dice {
        rows.push(components::Record::text(
            s.dice_procedure_row,
            String::from(procedure_label(w.procedure(), s)),
            Tone::Text,
        ));
    }
    rows.extend([
        components::Record::value(ids::CREATE_MATH_ENTROPY, s.create_math_entropy, masked()),
        components::Record::text(
            s.create_math_checksum,
            strings::fill1(s.create_math_bits, &alloc::format!("{cs_bits}")),
            Tone::Text,
        ),
        components::Record::value(ids::CREATE_MATH_BITS, s.create_bits_row, masked()),
        components::Record::value(
            ids::CREATE_MATH_WORD,
            strings::fill1(s.create_math_word, "1"),
            masked(),
        ),
    ]);
    screens::record(
        c,
        Record {
            pager: None,
            title: s.create_math_title,
            key: None,
            network: components::Network::Mainnet,
            rows,
            warnings: vec![],
            action: Some(Action::new(ids::CREATE_CONTINUE, s.action_done)),
        },
    )
}

/// §5 Result: what skipping the quiz leaves behind. The safe way on
/// takes the accent (§4.13).
pub(crate) fn skip_step(c: &Chrome<'_>, s: &Strings) -> Node {
    screens::result(
        c,
        Result {
            caption: None,
            title: s.quiz_start_title,
            icon: Icon::Warning,
            tone: Tone::Caution,
            result: s.quiz_failed_title,
            rows: vec![components::Record::text(
                s.quiz_row,
                s.quiz_skipped,
                Tone::Caution,
            )],
            actions: vec![
                Action::new(ids::QUIZ_SKIP_CONFIRM, s.quiz_skip_confirm),
                Action::new(ids::QUIZ_SKIP_CANCEL, s.quiz_skip_cancel),
            ],
        },
    )
}

/// The checksum's bits as the ones and zeros they are.
fn checksum_bits(w: &CreateWizard) -> String {
    let Some(m) = w.mnemonic() else {
        return String::new();
    };
    let (checksum, bits) = m.checksum_bits();
    (0..bits)
        .rev()
        .map(|b| if (checksum >> b) & 1 == 1 { '1' } else { '0' })
        .collect()
}

/// The value of a row that stands for a secret: bullets, never the head
/// and tail of a key (§4.10).
fn masked() -> String {
    core::iter::repeat_n('\u{2022}', osk_ui::tokens::ELIDE_HEAD).collect()
}
