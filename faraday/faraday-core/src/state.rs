//! The state of screens that belong to no area folder: seed entry, signing a
//! message and the stick visit.

use std::collections::BTreeSet;

use crate::wallet::Session;
use crate::{Column, Screen, VisitBarAt, Writing, flow, forms, secret_text, visit_taken};
use faraday_crypto::work::Compute;

/// Seed entry.
#[derive(Default)]
pub struct EntryState {
    /// The words as typed.
    pub typed: secret_text::SecretText,
    /// The slot fingerprint the key is meant for.
    pub wanted: Option<[u8; 4]>,
    /// The loaded wallet any of whose missing keys the words may give.
    pub wallet: Option<usize>,
    /// The label for the key.
    pub label: String,
    /// The last refusal.
    pub error: Option<String>,
    /// Where to go once the key is added.
    pub back: Option<Screen>,
    /// The BIP-39 passphrase, when one is typed.
    pub passphrase: secret_text::SecretText,
    /// Typing goes to the passphrase rather than the words.
    pub on_passphrase: bool,
    /// Which form of key is being typed.
    pub form: forms::Form,
    /// The BIP-39 list the words are typed from, by its place in
    /// `forms::LANGUAGES`.
    pub language_at: u8,
    /// The word entry, for a list typed on an on-screen
    /// keyboard ([`forms::on_screen`]); `None` for a Latin list, whose
    /// words are typed into `typed`.
    pub keys: Option<Box<crate::word_entry::WordEntry>>,
    /// The on-screen keyboard's shift is on.
    pub shift: bool,
    /// SLIP-39 shares, codex32 strings or Seed XOR parts collected.
    pub parts: forms::Parts,
    /// The lists other than English are shown to choose from.
    pub languages: bool,
    /// The chips for SLIP-39, codex32 and Seed XOR are shown: pressed
    /// directly, or set already by a Tools tile for one of those forms
    /// (`docs/history/SIMPLIFY.md` §2.4).
    pub other_forms: bool,
    /// How many words were chosen, 0 until one is: 12 boxes.
    pub word_count: u8,
    /// The status line's key: what the words typed make with the
    /// passphrase, the fingerprint or why there is none, public alone.
    /// Worked out by the first frame that asks for it and kept until the
    /// next event that can change the words, list or passphrase clears it
    /// ([`EntryState::probed`], [`EntryState::forget_probe`]).
    probe: std::cell::RefCell<Option<Result<faraday_bip::keys::Fingerprint, String>>>,
}

impl EntryState {
    /// The word counts a BIP-39 key can have.
    pub const COUNTS: [u8; 5] = [12, 15, 18, 21, 24];

    /// How many word boxes the choice asks for: 12 until another count
    /// is chosen.
    pub fn words_chosen(&self) -> usize {
        if Self::COUNTS.contains(&self.word_count) {
            usize::from(self.word_count)
        } else {
            12
        }
    }

    /// How many word boxes to show with `filled` words in: the count
    /// chosen, or all 24 once the words run past it.
    pub fn word_boxes(&self, filled: usize) -> usize {
        let chosen = self.words_chosen();
        if filled > chosen { 24 } else { chosen }
    }

    /// The BIP-39 list the words are typed from.
    pub fn language(&self) -> faraday_bip::bip39::Language {
        forms::LANGUAGES[usize::from(self.language_at) % forms::LANGUAGES.len()]
    }

    /// Costs a PBKDF2 and an elliptic-curve multiplication when nothing is
    /// kept, the first frame after an event; then it is a lookup until
    /// [`EntryState::forget_probe`]. The key words `m` make with the
    /// passphrase typed, or why they make none, for the status line. `m`
    /// must be the words typed now. Decisions (Add, Continue, Lock in) load
    /// the words themselves and never read this.
    pub(crate) fn probed(
        &self,
        c: &Compute,
        m: &faraday_bip::bip39::Mnemonic,
    ) -> Result<faraday_bip::keys::Fingerprint, String> {
        if let Some(r) = self.probe.borrow().as_ref() {
            return r.clone();
        }
        // Kept until an event forgets it: worked out once, by the frame
        // that asks first.
        let r = {
            Session::default()
                .add_mnemonic(c, m, &self.passphrase, "", None)
                .map_err(|e| e.text())
        };
        *self.probe.borrow_mut() = Some(r.clone());
        r
    }

    /// Forgets the status line's key, for an event that may have changed
    /// the words, list or passphrase.
    pub(crate) fn forget_probe(&self) {
        self.probe.borrow_mut().take();
    }

    /// The status line's key as kept, if it is: a lookup.
    pub(crate) fn probe_kept(&self) -> Option<Result<faraday_bip::keys::Fingerprint, String>> {
        self.probe.borrow().clone()
    }
}

/// Signing a message: the cards' state.
#[derive(Default)]
pub struct MessageState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; 4],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The wallet, by index into the session.
    pub wallet: Option<usize>,
    /// The receive address's index.
    pub index: u32,
    /// The message.
    pub text: String,
    /// Typing goes to the message.
    pub typing: bool,
    /// BIP-322 rather than BIP-137.
    pub bip322: bool,
    /// The signature, once made.
    pub signed: Option<faraday_psbt::message::Signed>,
    /// The last refusal.
    pub error: Option<String>,
}

/// The visit screen's choices.
#[derive(Default)]
pub struct VisitState {
    /// The stick shown, by index.
    pub stick: usize,
    /// Outbox files chosen to write, by name.
    pub out: BTreeSet<String>,
    /// Stick files chosen to copy, by name.
    pub inn: BTreeSet<String>,
    /// Stick files asked for and not yet answered, by name: a picture's
    /// codes are read off the shell's thread, one picture at a time.
    pub reading: BTreeSet<String>,
    /// The settings file's row as the person set it; `None` for its
    /// default (`Faraday::visit_settings_on`).
    pub settings: Option<bool>,
    /// What the last write or copy did, one line per file.
    pub log: Vec<(String, bool)>,
    /// Names from an "Import and load" copy, by name: loaded as keys the
    /// moment the stick is pulled, no further press needed.
    pub load_after: BTreeSet<String>,
    /// How far the Outbox list is scrolled, design units. The stick's
    /// files scroll by [`Faraday::list_offset`].
    pub out_offset: f32,
    /// Where each column's scrollbar was drawn, by [`VisitState::bar`].
    pub bars: [std::cell::Cell<Option<VisitBarAt>>; 2],
    /// Inbox files chosen to write, by name: a copy from one stick to
    /// another. One that may be a secret is chosen only on the secret
    /// sheet.
    pub from_inbox: BTreeSet<String>,
    /// Inbox files sent to be written and not yet answered, by name.
    pub writing_inbox: BTreeSet<String>,
    /// The write under way: the stick's label, and each file sent with
    /// the SHA-256 of its bytes. Its first answer starts a new receipt.
    pub writing: Option<Writing>,
    /// The files the write would write whose names are taken on the
    /// stick, while the Already on the stick sheet asks.
    pub taken: Vec<visit_taken::Taken>,
}

impl VisitState {
    /// Where `column`'s scrollbar was drawn, `None` when its list fits.
    pub fn bar(&self, column: Column) -> &std::cell::Cell<Option<VisitBarAt>> {
        &self.bars[match column {
            Column::Outbox => 0,
            Column::Stick => 1,
        }]
    }
}
