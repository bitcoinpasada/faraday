//! The Backup flow's screens (`docs/DESIGN.md` §5): the quiz start as a
//! Menu, the questions and their cautions as the quiz's own screens, the
//! result as a Result, the words on the Words screen, and each seed code
//! on a Secret screen.
//!
//! A seed code is the whole key, so it is a Secret and not a QR: masked
//! until a finger is on the panel or the app bar's eye is running
//! (§4.10).

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::bip39::Mnemonic;
use osk_bip::slip39;
use osk_codec::qr::{Ecc, Payload, QrMatrix};
use osk_codec::seedqr;
use osk_shell_api::SecureHardware;
use osk_ui::components;
use osk_ui::geom::SizeClass;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Chrome, Entry, Item, Pager, Qr, Result, Secret, Value};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use osk_bip::bip39::Language;

use crate::backup::{BackupFlow, BackupStep};
use crate::create::{SOURCE_ROWS, Source};
use crate::load::{EntryList, LoadedKey};
use crate::sign::Save;
use crate::views::{create as create_view, quiz as quiz_view, words};
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    pub(crate) fn view_backup(&self, b: &BackupFlow) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            // The encrypted backup is the one part of the flow that
            // needs no words: a key with none is sealed as its master
            // seed (`docs/PLANNING.md` §16.112 rule 1), and its three
            // screens read nothing out of a mnemonic.
            if matches!(
                b.step(),
                BackupStep::Passphrase
                    | BackupStep::PassphraseRepeat
                    | BackupStep::Encrypted
                    | BackupStep::EncryptedQr
            ) {
                return self.encrypted_screen(c, b);
            }
            match self.backup_words(b, |m| self.backup_screen(c, b, m)) {
                Some(node) => node,
                None => screens::menu(c, s.backup_title, None, Vec::new(), Vec::new()),
            }
        })
    }

    /// The three screens of the encrypted backup, which read no words.
    fn encrypted_screen(&self, c: &Chrome<'_>, b: &BackupFlow) -> Node {
        match b.step() {
            BackupStep::PassphraseRepeat => self.backup_pass(c, b, true),
            BackupStep::Encrypted => self.encrypted_result(c, b),
            BackupStep::EncryptedQr => self.encrypted_qr(c, b),
            _ => self.backup_pass(c, b, false),
        }
    }

    /// The flow's screen for the key whose words `m` are open for the
    /// duration of this call.
    fn backup_screen(&self, c: &Chrome<'_>, b: &BackupFlow, m: &Mnemonic) -> Node {
        let s = self.strings();
        let lang = m.language();
        match b.step() {
            BackupStep::QuizStart => quiz_view::quiz_start(c, b.helper(), false, s),
            BackupStep::Quiz => match b.quiz() {
                Some(q) if q.wrong_slot().is_some() => quiz_view::quiz_wrong(c, q, s),
                Some(q) => quiz_view::quiz_question(c, q, EntryList::Bip39(lang), s),
                None => Node::column(),
            },
            BackupStep::QuizResult => {
                let (done, total) = b.quiz().map_or((0, 0), |q| q.progress());
                let questions = strings::fill(
                    s.words_page,
                    &[&alloc::format!("{done}"), &alloc::format!("{total}")],
                );
                if b.passed() {
                    return screens::result(
                        c,
                        Result {
                            caption: None,
                            title: s.quiz_start_title,
                            icon: Icon::Success,
                            tone: Tone::Success,
                            result: s.quiz_passed_title,
                            rows: vec![
                                components::Record::text(
                                    s.load_words_row,
                                    alloc::format!("{total}"),
                                    Tone::Text,
                                ),
                                components::Record::text(
                                    s.quiz_questions_row,
                                    questions,
                                    Tone::Text,
                                ),
                            ],
                            actions: vec![Action::new(ids::QUIZ_DONE, s.action_done)],
                        },
                    );
                }
                screens::result(
                    c,
                    Result {
                        caption: None,
                        title: s.quiz_start_title,
                        icon: Icon::Warning,
                        tone: Tone::Caution,
                        result: s.quiz_failed_title,
                        rows: vec![components::Record::text(
                            s.quiz_questions_row,
                            questions,
                            Tone::Caution,
                        )],
                        actions: vec![
                            Action::new(ids::QUIZ_DONE, s.action_done),
                            Action::new(ids::QUIZ_AGAIN, s.quiz_again),
                        ],
                    },
                )
            }
            BackupStep::Words => words::words_screen(
                c,
                words::WordsScreen {
                    title: String::from(s.words_title),
                    list: EntryList::Bip39(lang),
                    indices: m.indices(),
                    revealed: self.revealed(ids::CREATE_REVEAL),
                    rows: words::Rows::Words {
                        numbers: b.numbers(),
                    },
                    page: b.page(),
                    remaining: self.reveal_ring(),
                    extra: None,
                    action: Action::new(ids::QUIZ_DONE, s.action_done),
                },
                s,
            ),
            BackupStep::GridChoice => self.grid_choice(c, b, m),
            BackupStep::Grid => self.grid_screen(c, b, m),
            BackupStep::Steel => words::words_screen(
                c,
                words::WordsScreen {
                    title: String::from(s.backup_steel),
                    list: EntryList::Bip39(lang),
                    indices: m.indices(),
                    revealed: self.revealed(ids::CREATE_REVEAL),
                    rows: words::Rows::Steel,
                    page: b.page(),
                    remaining: self.reveal_ring(),
                    // §8.2 item 6: the blank template is a file, and a
                    // file is written where there is somewhere to write
                    // it — the desktop window.
                    extra: (c.class() == SizeClass::Wide)
                        .then(|| Action::new(ids::BACKUP_STEEL_TEMPLATE, s.backup_print_template)),
                    action: Action::new(ids::QUIZ_DONE, s.action_done),
                },
                s,
            ),
            BackupStep::XorCount => self.xor_count(c, b),
            BackupStep::XorSource => self.xor_source(c, b),
            // The gatherer draws its own screens (`build_screen`).
            BackupStep::XorGather => Node::column(),
            BackupStep::XorPart => self.xor_part(c, b, lang),
            BackupStep::XorResult => self.xor_result(c, b),
            // The plan draws its own screens (`build_screen`).
            BackupStep::Shares | BackupStep::Codex32 => Node::column(),
            BackupStep::SeedQr => self.seedqr_secret(c, m, false),
            BackupStep::CompactSeedQr => self.seedqr_secret(c, m, true),
            BackupStep::Passphrase => self.backup_pass(c, b, false),
            BackupStep::PassphraseRepeat => self.backup_pass(c, b, true),
            BackupStep::Encrypted => self.encrypted_result(c, b),
            BackupStep::EncryptedQr => self.encrypted_qr(c, b),
        }
    }

    /// §5 Entry, "Backup passphrase" and "Repeat it": one masked line
    /// over the passphrase keyboard, with the reserved error line under
    /// the field. The last character typed stays visible for half a
    /// second, as it does everywhere a passphrase is typed.
    ///
    /// §4.3: ✓ is dead until what is typed can be used, which here is
    /// [`crate::pass_entry::MIN_PASSPHRASE`] characters. What makes a
    /// good passphrase, and that losing it loses the backup, is a Learn
    /// page and not a line on this screen (§2.1).
    fn backup_pass(&self, c: &Chrome<'_>, b: &BackupFlow, repeat: bool) -> Node {
        let s = self.strings();
        let n = b.pass_len();
        let mut masked: String = core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
        match b.pass_visible_char(self.now_ms) {
            Some(ch) => masked.push(ch),
            None if n > 0 => masked.push('\u{2022}'),
            None => {}
        }
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: if repeat {
                    s.backup_pass_repeat
                } else {
                    s.backup_pass_title
                },
                value: masked,
                mono: true,
                above: screens::Above::Nothing,
                words: None,
                eye: None,
                keyboard: (ids::BACKUP_PASS_KEYBOARD, KeyboardKind::Passphrase),
                enabled: if b.pass_ready() {
                    ALL_KEYS
                } else {
                    DONE_DISABLED
                },
                error: b
                    .pass_mismatch()
                    .then(|| String::from(s.backup_pass_mismatch)),
            },
        )
    }

    /// §5 Result, "Encrypted backup": which key, what is inside, how
    /// many bytes, and the two ways off the device.
    fn encrypted_result(&self, c: &Chrome<'_>, b: &BackupFlow) -> Node {
        let s = self.strings();
        let key = self.keys.get(b.key());
        let fingerprint = self.backup_fingerprint(b);
        // A passphrase key's words are its parent's: the passphrase is
        // never in a backup, so the row says what the bytes hold.
        let inside = if key.is_some_and(|k| k.has_passphrase) {
            s.backup_inside_parent
        } else if key.is_some_and(|k| !k.has_mnemonic()) {
            s.backup_inside_seed
        } else {
            s.backup_inside_words
        };
        let mut rows = vec![
            components::Record::fingerprint(s.confirm_key, fingerprint),
            components::Record::text(s.backup_inside_row, inside, Tone::Text),
            components::Record::text(
                s.row_bytes,
                alloc::format!("{}", b.backup().len()),
                Tone::Text,
            ),
            // §16.112: a file states its own Argon2id cost, and so does
            // the screen that made it, because that cost is what opening
            // it elsewhere will need.
            self.memory_record(),
        ];
        // §16.134: what it is sealed with and what reads it, so that an
        // heir holding the file knows what to look for.
        rows.extend(self.encryption_records(b.backup(), self.form_is_kdbx()));
        match b.save() {
            Save::Idle => {}
            Save::Waiting => rows.push(components::Record::text(
                s.sign_file_row,
                String::from(s.sign_waiting),
                Tone::Muted,
            )),
            Save::Written => rows.push(components::Record::text(
                s.sign_file_row,
                strings::fill1(s.sign_saved_as, &self.backup_file_name()),
                Tone::Text,
            )),
            Save::Failed => rows.push(components::Record::text(
                s.sign_file_row,
                String::from(s.sign_not_saved),
                Tone::Caution,
            )),
        }
        // The backup is ciphertext, which is public by construction:
        // §4.10's rule keeps the words out of the clipboard, not this.
        rows.extend(self.copy_record());
        let first = if b.save() == Save::Written {
            Action::new(ids::QUIZ_DONE, s.action_done)
        } else {
            Action::new(ids::BACKUP_SAVE, s.action_save_file)
        };
        // A KDBX database is kilobytes and the apps that read one read
        // files, so the form that has no QR does not offer one.
        let mut actions = vec![first];
        if !self.form_is_kdbx() {
            actions.push(Action::new(ids::BACKUP_SHOW_QR, s.action_show_qr));
        }
        screens::result(
            c,
            Result {
                caption: self.notice().map(String::from),
                title: s.backup_encrypted,
                icon: Icon::Success,
                tone: Tone::Success,
                result: s.backup_encrypted_result,
                rows,
                actions,
            },
        )
    }

    /// §5 QR, "Encrypted backup": the whole backup in one code. The
    /// bytes are ciphertext and fit one code on every class, so the
    /// screen has no Animated toggle and no hold over it.
    fn encrypted_qr(&self, c: &Chrome<'_>, b: &BackupFlow) -> Node {
        let s = self.strings();
        let fingerprint = self.backup_fingerprint(b);
        let matrix = osk_codec::qr::encode(Payload::Bytes(b.backup()), Ecc::Low)
            .ok()
            .map(Rc::new);
        let Some(matrix) = matrix else {
            return screens::result(
                c,
                Result {
                    caption: None,
                    title: s.backup_encrypted,
                    icon: Icon::Error,
                    tone: Tone::Danger,
                    result: s.export_too_long,
                    rows: Vec::new(),
                    actions: vec![Action::new(ids::QUIZ_DONE, s.action_done)],
                },
            );
        };
        screens::qr(
            c,
            Qr {
                title: s.backup_encrypted,
                matrix,
                // §4.9: the label under the square says what the code
                // carries, never the title again — here, which key.
                label: &fingerprint,
                toggle: None,
                animated: false,
                forced: None,
                progress: None,
                save: self.png_row(),
                caption: self.notice().map(String::from),
            },
        )
    }

    /// §5 Secret: the seed code as the secret it is — the panel, the
    /// hold and the eye, and nothing under it (§2.1).
    fn seedqr_secret(&self, c: &Chrome<'_>, m: &Mnemonic, compact: bool) -> Node {
        let s = self.strings();
        let matrix = if compact {
            seedqr::encode_compact(m)
        } else {
            seedqr::encode_seedqr(m)
        };
        let title = if compact {
            s.compact_title
        } else {
            s.seedqr_title
        };
        let Ok(matrix) = matrix else {
            return screens::result(
                c,
                Result {
                    caption: None,
                    title,
                    icon: Icon::Error,
                    tone: Tone::Danger,
                    result: s.export_too_long,
                    rows: Vec::new(),
                    actions: vec![Action::new(ids::QUIZ_DONE, s.action_done)],
                },
            );
        };
        screens::secret(
            c,
            Secret {
                rows: Vec::new(),
                action: None,
                title,
                value: Value::Code(Rc::new(matrix)),
                revealed: self.revealed(ids::CREATE_REVEAL),
                panel: ids::CREATE_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                secondary: None,
                pager: None,
            },
        )
    }

    /// §5 Choice, "Which code?": the two seed codes a grid can draw,
    /// each with the square it comes to on this key's word count.
    fn grid_choice(&self, c: &Chrome<'_>, b: &BackupFlow, m: &Mnemonic) -> Node {
        let s = self.strings();
        let side = |matrix: core::result::Result<QrMatrix, _>| {
            matrix.map(|q| {
                let n = q.size();
                alloc::format!("{n} \u{00d7} {n}")
            })
        };
        let mut rows = Vec::new();
        let mut row = Item::chosen(ids::BACKUP_GRID_SEEDQR, s.seedqr_title, !b.compact());
        row.subtitle = side(seedqr::encode_seedqr(m)).ok();
        rows.push(row);
        let mut row = Item::chosen(ids::BACKUP_GRID_COMPACT, s.compact_title, b.compact());
        row.subtitle = side(seedqr::encode_compact(m)).ok();
        rows.push(row);
        screens::choice(
            c,
            s.backup_grid_title,
            rows,
            Action::new(ids::BACKUP_GRID_CONTINUE, s.action_continue),
        )
    }

    /// §5 Secret: the seed code as a grid of squares to copy by hand,
    /// with the pager over its quadrants in the bottom slot on a class
    /// that pages it (`docs/PLANNING.md` §8.2 item 5a).
    fn grid_screen(&self, c: &Chrome<'_>, b: &BackupFlow, m: &Mnemonic) -> Node {
        let s = self.strings();
        let compact = b.compact();
        let matrix = if compact {
            seedqr::encode_compact(m)
        } else {
            seedqr::encode_seedqr(m)
        };
        let title = if compact {
            s.compact_title
        } else {
            s.seedqr_title
        };
        let Ok(matrix) = matrix else {
            return screens::result(
                c,
                Result {
                    caption: None,
                    title,
                    icon: Icon::Error,
                    tone: Tone::Danger,
                    result: s.export_too_long,
                    rows: Vec::new(),
                    actions: vec![Action::new(ids::QUIZ_DONE, s.action_done)],
                },
            );
        };
        let class = c.class();
        let pages = components::qr_grid_pages(class);
        let page = usize::from(b.grid_page()).min(pages - 1);
        let (origin, span) = components::qr_grid_region(class, matrix.size(), page);
        let quadrants = [
            s.grid_top_left,
            s.grid_top_right,
            s.grid_bottom_left,
            s.grid_bottom_right,
        ];
        screens::secret(
            c,
            Secret {
                rows: Vec::new(),
                action: None,
                secondary: None,
                pager: (pages > 1).then(|| Pager {
                    prev: ids::WORDS_PREV,
                    next: ids::WORDS_NEXT,
                    label: String::from(quadrants[page]),
                    at_start: page == 0,
                    at_end: page + 1 >= pages,
                }),
                title,
                value: Value::Grid {
                    matrix: Rc::new(matrix),
                    origin,
                    span,
                },
                revealed: self.revealed(ids::CREATE_REVEAL),
                panel: ids::CREATE_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
            },
        )
    }

    /// §5 Choice, "How many parts?": 2, 3 or 4.
    fn xor_count(&self, c: &Chrome<'_>, b: &BackupFlow) -> Node {
        let s = self.strings();
        let rows = crate::backup::XOR_COUNTS
            .iter()
            .enumerate()
            .map(|(i, n)| {
                Item::chosen(
                    ids::at(ids::XOR_COUNT_BASE, i),
                    alloc::format!("{n}"),
                    usize::from(*n) == b.part_count(),
                )
            })
            .collect();
        screens::choice(
            c,
            s.xor_count_title,
            rows,
            Action::new(ids::XOR_COUNT_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Random parts from?": the rows of Create a key's own
    /// source choice, in the same order, with the same dimming and the
    /// same caution on "This device" (`docs/PLANNING.md` §16.92).
    fn xor_source(&self, c: &Chrome<'_>, b: &BackupFlow) -> Node {
        let s = self.strings();
        let items = SOURCE_ROWS
            .iter()
            .enumerate()
            .map(|(i, source)| {
                let id = ids::at(ids::XOR_SOURCE_BASE, i);
                let label = create_view::source_label(i, s);
                if !self.source_available(*source) {
                    Item::dimmed(label, create_view::source_reason(*source, s))
                } else if *source == Source::Device {
                    Item::cautioned(id, label, s.create_trusts_device, *source == b.xor_source())
                } else {
                    Item::chosen(id, label, *source == b.xor_source())
                }
            })
            .collect();
        screens::choice(
            c,
            s.xor_source_title,
            items,
            Action::new(ids::XOR_SOURCE_CONTINUE, s.action_continue),
        )
    }

    /// §5 Words: one Seed XOR part, which is a key of its own and so a
    /// secret like any other.
    fn xor_part(&self, c: &Chrome<'_>, b: &BackupFlow, lang: Language) -> Node {
        let s = self.strings();
        let at = b.part_at();
        let title = strings::fill(
            s.xor_part_of,
            &[
                &alloc::format!("{}", at + 1),
                &alloc::format!("{}", b.part_count()),
            ],
        );
        let Some(m) = b
            .part(at)
            .and_then(|bytes| Mnemonic::from_entropy(lang, bytes).ok())
        else {
            return screens::menu(c, s.xor_split, None, Vec::new(), Vec::new());
        };
        words::words_screen(
            c,
            words::WordsScreen {
                title,
                list: EntryList::Bip39(lang),
                indices: m.indices(),
                revealed: self.revealed(ids::CREATE_REVEAL),
                rows: words::Rows::Words {
                    numbers: b.numbers(),
                },
                page: b.page(),
                remaining: self.reveal_ring(),
                extra: None,
                action: Action::new(ids::CREATE_CONTINUE, s.action_continue),
            },
            s,
        )
    }

    /// §5 Result, "Split": one row per part, each naming the
    /// fingerprint a key loaded from that part has. That fingerprint is
    /// how a person checks a part later without putting the key back
    /// together.
    fn xor_result(&self, c: &Chrome<'_>, b: &BackupFlow) -> Node {
        let s = self.strings();
        // Where the random parts came from, before the parts
        // themselves: what a split rests on is a fact about it.
        let device = b.xor_source() == Source::Device;
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
                    .position(|r| *r == b.xor_source())
                    .unwrap_or(0),
                s,
            ))
        };
        let mut rows = vec![components::Record::text(
            s.xor_source_row,
            source,
            Tone::Text,
        )];
        if device {
            rows.push(components::Record::text(
                s.create_trust_row,
                s.create_trust_device,
                Tone::Caution,
            ));
        }
        rows.extend((0..b.part_count()).map(|i| {
            let label = strings::fill1(s.xor_part, &alloc::format!("{}", i + 1));
            let value = b
                .part_fingerprint(i)
                .map_or_else(|| String::from(s.value_none), text::fingerprint_hex);
            components::Record::fingerprint(label, value)
        }));
        screens::result(
            c,
            Result {
                caption: None,
                title: s.xor_split,
                icon: Icon::Success,
                tone: Tone::Success,
                result: s.xor_split_result,
                rows,
                actions: vec![Action::new(ids::QUIZ_DONE, s.action_done)],
            },
        )
    }

    /// §5 Menu, "Backup": one row per way to record the key. A key whose
    /// words are sealed away dims the rows that need them and says why
    /// (§4.11), rather than replacing the screen with a sentence.
    pub(crate) fn view_backup_menu(&self, key: usize) -> Node {
        let s = self.strings();
        let words = self.keys.get(key).is_some_and(|k| k.has_mnemonic());
        let verified = self.keys.get(key).is_some_and(|k| k.backup_verified);
        let row = |id, icon, label: &str| {
            if words {
                screens::Row::Menu {
                    id,
                    icon: Some(icon),
                    label: String::from(label),
                    value: None,
                    tone: Tone::Text,
                }
            } else {
                screens::Row::Dimmed {
                    icon: Some(icon),
                    label: String::from(label),
                    reason: Some(String::from(s.backup_no_words)),
                }
            }
        };
        // §5: "verified" is a neutral state, so the value keeps the body
        // tone and only the label is muted (§4.8 leaves "not verified" in
        // the caution tone).
        let (state, tone) = if verified {
            (s.detail_backup_verified, Tone::Text)
        } else {
            (s.detail_backup_unverified, Tone::Caution)
        };
        let verify = if words {
            screens::Row::Menu {
                id: ids::BACKUP_VERIFY,
                icon: Some(Icon::Checklist),
                label: String::from(s.backup_verify),
                value: Some(String::from(state)),
                tone,
            }
        } else {
            screens::Row::Dimmed {
                icon: Some(Icon::Checklist),
                label: String::from(s.backup_verify),
                reason: Some(String::from(s.backup_no_words)),
            }
        };
        let codex32 = screens::Row::Menu {
            id: ids::BACKUP_CODEX32,
            icon: Some(Icon::Numbers),
            label: String::from(s.backup_codex32),
            value: None,
            tone: Tone::Text,
        };
        if !words {
            // §16.107 rule 5 and §16.109 rule 1: a key with no words is
            // its seed, and the forms it can be written in are the ones
            // that take a seed. Codex32 takes any of them; SLIP-39 takes
            // the two master-secret lengths and no others.
            let seed = self.keys.get(key).map_or(0, LoadedKey::seed_len);
            // §16.112 rule 1: the container takes a master seed, so a
            // key with no words has an encrypted backup too.
            let mut rows = vec![
                screens::Row::Menu {
                    id: ids::BACKUP_ENCRYPTED,
                    icon: Some(Icon::Vault),
                    label: String::from(s.backup_encrypted),
                    value: None,
                    tone: Tone::Text,
                },
                codex32,
            ];
            if matches!(
                seed,
                slip39::MIN_STRENGTH_BYTES | slip39::MAX_STRENGTH_BYTES
            ) {
                rows.push(screens::Row::Menu {
                    id: ids::BACKUP_SLIP39,
                    icon: Some(Icon::Scissors),
                    label: String::from(s.backup_slip39),
                    value: None,
                    tone: Tone::Text,
                });
            }
            return self.with_chrome(Some(ids::BACK), |c| {
                screens::menu(c, s.backup_title, None, rows, Vec::new())
            });
        }
        let rows = vec![
            row(ids::BACKUP_WORDS, Icon::List, s.backup_words),
            row(ids::BACKUP_SEEDQR, Icon::Qr, s.backup_seedqr),
            row(ids::BACKUP_COMPACT, Icon::Compress, s.backup_compact),
            row(ids::BACKUP_ENCRYPTED, Icon::Vault, s.backup_encrypted),
            row(ids::BACKUP_GRID, Icon::Grid, s.backup_grid),
            row(ids::BACKUP_STEEL, Icon::Numbers, s.backup_steel),
            row(ids::BACKUP_XOR, Icon::Scissors, s.xor_split),
            // §16.109 rule 5: a codex32 backup takes the seed, which
            // every key has, so the row is live on a key with words too.
            codex32,
            verify,
        ];
        self.with_chrome(Some(ids::BACK), |c| {
            // §4.12: what the quiz asks and what its outcome means is a
            // Learn page, not a paragraph over the rows.
            screens::menu(c, s.backup_title, None, rows, Vec::new())
        })
    }
}
