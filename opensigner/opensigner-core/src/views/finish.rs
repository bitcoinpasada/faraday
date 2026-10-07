//! The steps every key wizard ends with (`docs/DESIGN.md` §5): the
//! passphrase offer and "Which key?" as Choice screens, passphrase entry
//! as an Entry screen, the confirmation as a Record, and the session PIN
//! typed twice on the Pad screen. The Load and Create wizards share
//! them; both read [`Finish`] through its non-secret accessors.

use alloc::string::String;
use alloc::vec;

use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Chrome, Entry, Field, Item, Pad, PadKind, Record};
use osk_ui::widgets::keyboard::{ALL_KEYS, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::finish::{Finish, FinishStep};
use crate::strings::Strings;
use crate::{OpenSigner, ids, text};

impl OpenSigner {
    /// The screen for `fstep`.
    pub(crate) fn view_finish(&self, f: &Finish, fstep: FinishStep) -> Node {
        self.with_chrome(Some(ids::BACK), |c| match fstep {
            FinishStep::PassphraseOffer => passphrase_offer_step(c, f, self.strings()),
            FinishStep::Passphrase => self.passphrase_step(c, f),
            FinishStep::PassphraseConfirm => which_key_step(c, f, self.strings()),
            FinishStep::Confirm => self.confirm_step(c, f),
            FinishStep::Pin => self.pin_step(c, f, false),
            FinishStep::PinConfirm => self.pin_step(c, f, true),
        })
    }

    /// §5 Result, "Key created": the fingerprint of the key the first
    /// run made, and the two ways on. "Check an address" is what proves
    /// the wallet against a coordinator, so it stands beside Done
    /// (UX.md §7.1, job A4).
    pub(crate) fn view_created(&self, key: usize) -> Node {
        let s = self.strings();
        let fingerprint = self.keys.get(key).map_or_else(
            || String::from(s.value_none),
            |k| text::fingerprint_hex(k.fingerprint),
        );
        self.with_chrome(Some(ids::BACK), |c| {
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title: s.created_title,
                    icon: Icon::Success,
                    tone: Tone::Success,
                    result: s.created_result,
                    rows: vec![components::Record::fingerprint(
                        s.confirm_key,
                        fingerprint.clone(),
                    )],
                    actions: vec![
                        Action::new(ids::CREATED_DONE, s.action_done),
                        Action::new(ids::CREATED_CHECK, s.wallet_check),
                    ],
                },
            )
        })
    }

    /// §5 Pad, "Set a PIN" and "Repeat the PIN": the dots in the field
    /// directly above the pad, the reserved caption line under the
    /// field, and the pad itself. A repeat that did not match restarts
    /// the entry and says so on that line.
    fn pin_step(&self, c: &Chrome<'_>, f: &Finish, confirm: bool) -> Node {
        let s = self.strings();
        screens::pad(
            c,
            Pad {
                words: None,
                eye: None,
                title: String::from(if confirm {
                    s.pin_repeat_title
                } else {
                    s.pin_set_title
                }),
                id: ids::LOAD_PIN_KEYBOARD,
                kind: PadKind::Pin {
                    scramble: self.pin_scramble(confirm),
                    done: f.pin_complete(),
                },
                field: Field::Dots(f.pin_len()),
                progress: None,
                entries: None,
                caption: None,
                error: (!confirm && f.pin_mismatch()).then(|| String::from(s.pin_mismatch)),
                action: None,
            },
        )
    }

    /// §5 Entry: the passphrase as one masked line over its keyboard.
    /// The last character typed stays visible for half a second.
    fn passphrase_step(&self, c: &Chrome<'_>, f: &Finish) -> Node {
        let n = f.passphrase_len();
        let mut masked: String = core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
        match f.passphrase_visible_char(self.now_ms) {
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

    /// §5 Record, "Add this key?": what the key is, one fact per row,
    /// and the tap that adds it. Adding a key is undone by forgetting
    /// it, so it is a tap and not a hold (§2.7).
    ///
    /// The tap is dead until the shell has answered `RequestEntropy`.
    /// A key added before it does is held in the clear until the answer
    /// arrives, and the window has no reason to exist: every shell
    /// answers in about a millisecond, so the button is live before a
    /// finger reaches it, and a shell that never answers is one this
    /// key should not be added to. §4.13 keeps the button's place and
    /// size either way, and nothing on the screen explains the wait.
    fn confirm_step(&self, c: &Chrome<'_>, f: &Finish) -> Node {
        let s = self.strings();
        let fingerprint = f
            .fingerprint()
            .map_or_else(|| String::from(s.value_none), text::fingerprint_hex);
        let rows = vec![
            components::Record::fingerprint(s.confirm_key, fingerprint),
            components::Record::text(s.confirm_network, self.network.name(), Tone::Text),
            components::Record::text(
                s.load_words_row,
                alloc::format!("{}", self.wizard_words()),
                Tone::Text,
            ),
            components::Record::text(
                s.detail_passphrase,
                if f.has_passphrase() {
                    s.value_yes
                } else {
                    s.value_no
                },
                Tone::Text,
            ),
        ];
        screens::record(
            c,
            Record {
                pager: None,
                title: s.confirm_title,
                key: None,
                network: text::network(self.network),
                rows,
                warnings: vec![],
                action: Some(Action::when(
                    ids::LOAD_HOLD,
                    s.confirm_add,
                    !self.session.entropy_pending(),
                )),
            },
        )
    }
}

/// §5 Choice, "Add a passphrase?": two rows and Continue. What a
/// passphrase costs is a Learn page, not a line on a working screen
/// (§2.1), so it is not here.
fn passphrase_offer_step(c: &Chrome<'_>, f: &Finish, s: &Strings) -> Node {
    screens::choice(
        c,
        s.passphrase_offer_title,
        vec![
            Item::chosen(ids::LOAD_SKIP, s.passphrase_none, !f.offer_add()),
            Item::chosen(ids::LOAD_ADD_PASSPHRASE, s.passphrase_add, f.offer_add()),
        ],
        Action::new(ids::LOAD_PASS_CONTINUE, s.action_continue),
    )
}

/// §5 Choice, "Which key?": the fingerprint without the passphrase and
/// the one with it, as two rows. The row the check is on is the key that
/// is added.
fn which_key_step(c: &Chrome<'_>, f: &Finish, s: &Strings) -> Node {
    let name = |fp: Option<osk_bip::keys::Fingerprint>| {
        fp.map_or_else(|| String::from(s.value_none), text::fingerprint_hex)
    };
    screens::choice(
        c,
        s.passphrase_which_title,
        vec![
            Item::key(
                ids::LOAD_WHICH_PLAIN,
                name(f.fingerprint_plain()),
                s.passphrase_without,
                !f.which_passphrase(),
            ),
            Item::key(
                ids::LOAD_WHICH_PASSPHRASE,
                name(f.fingerprint_with_passphrase()),
                s.passphrase_with,
                f.which_passphrase(),
            ),
        ],
        Action::new(ids::LOAD_WHICH_CONTINUE, s.action_continue),
    )
}
