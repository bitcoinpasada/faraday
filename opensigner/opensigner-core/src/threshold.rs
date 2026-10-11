//! The FROST dealer (`docs/PLANNING.md` §16.103, §16.104 rule 3), run
//! on this one device as the last part of Add a wallet.
//!
//! The wizard in `build.rs` gathers the kind, the counts and the `t`
//! loaded 24-word keys that are the chosen shares; what is here is what
//! the deal itself holds: the dealt set, the public group record, and
//! the words and quiz of each computed key on its way to Keys.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text. The views that draw it live in `views/build.rs` and receive
//! only what the screen shows.

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::NetworkKind;
use osk_bip::bitcoin::secp256k1::Secp256k1;
use osk_bip::frost::{self, Dealt, SecShare};
use osk_bip::keys::{Fingerprint, Network};
use osk_bip::threshold::ThresholdRecord;
use osk_crypto::{Zeroize, ZeroizeOnDrop};

use crate::ids::{self, Id};
use crate::load::EntryList;
use crate::quiz::{Quiz, QuizState};
use crate::sign::Save;

/// The most members a group made here has (§16.103: "n at most 5 in
/// this version").
pub const MAX_SHARES: usize = 5;

/// The member counts "How many keys?" lists, in the order the screen
/// lists them: three first, because three is the route the design is
/// for.
pub const COUNTS: [u8; 4] = [3, 2, 4, 5];

/// The deal, once the chosen keys are in.
pub struct Dealer {
    dealt: Dealt,
    record: ThresholdRecord,
    /// How many of the group's members were chosen from loaded keys.
    chosen: usize,
    /// Which computed member's words are on screen, as an index into the
    /// whole group.
    show_at: usize,
    page: u8,
    numbers: bool,
    quiz: Option<Quiz>,
    helper: bool,
    /// What became of "Save to file" on the record screen.
    save: Save,
}

impl Zeroize for Dealer {
    fn zeroize(&mut self) {
        self.quiz = None;
        self.page = 0;
        self.numbers = false;
    }
}

impl Drop for Dealer {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Dealer {}

impl Dealer {
    /// The dealer over `chosen`, whose identifiers are their positions
    /// in the choosing order, with the other `n - chosen.len()` members
    /// computed. `None` where the deal does not stand.
    pub fn deal(n: usize, t: usize, chosen: &[(u32, SecShare)], network: Network) -> Option<Self> {
        let secp = Secp256k1::verification_only();
        let dealt = frost::deal(&secp, n, t, chosen).ok()?;
        let kind = match network {
            Network::Mainnet => NetworkKind::Main,
            _ => NetworkKind::Test,
        };
        let record = ThresholdRecord::new(dealt.info.clone(), kind);
        Some(Dealer {
            dealt,
            record,
            chosen: chosen.len(),
            show_at: chosen.len(),
            page: 0,
            numbers: false,
            quiz: None,
            helper: false,
            save: Save::Idle,
        })
    }

    /// The group record.
    pub fn record(&self) -> &ThresholdRecord {
        &self.record
    }

    /// How many members the group has.
    pub fn n(&self) -> usize {
        self.record.n()
    }

    /// The first computed member, which is where the words start.
    pub fn first_computed(&self) -> usize {
        self.chosen
    }

    /// Which member's words are on screen.
    pub fn show_at(&self) -> usize {
        self.show_at
    }

    /// The word-grid page.
    pub fn page(&self) -> u8 {
        self.page
    }

    /// Whether the words carry their wordlist numbers.
    pub fn numbers(&self) -> bool {
        self.numbers
    }

    /// Whether the quiz's helper variant is on.
    pub fn helper(&self) -> bool {
        self.helper
    }

    /// The quiz, while running.
    pub fn quiz(&self) -> Option<&Quiz> {
        self.quiz.as_ref()
    }

    /// What became of "Save to file".
    pub fn save(&self) -> Save {
        self.save
    }

    /// Marks a write as out.
    pub fn mark_saving(&mut self) {
        self.save = Save::Waiting;
    }

    /// The shell answered.
    pub fn mark_saved(&mut self, written: bool) {
        self.save = if written { Save::Written } else { Save::Failed };
    }

    /// Fingerprint of member `i`, which is hash160 of its public share:
    /// the label a person copies beside the words.
    pub fn share_fingerprint(&self, i: usize) -> Option<Fingerprint> {
        self.record.share_fingerprint(i)
    }

    /// Member `i`'s 24 words. The mnemonic lives only as long as the
    /// caller holds it.
    pub fn words(&self, i: usize) -> Option<Mnemonic> {
        let share = self.dealt.shares.get(i)?;
        let mut bytes = share.secret_bytes();
        let m = Mnemonic::from_entropy(Language::English, &bytes).ok();
        bytes.zeroize();
        m
    }

    /// Toggles the wordlist numbers, which is the Words screen's own
    /// control.
    pub fn tap(&mut self, id: Id) {
        if id == ids::WORDS_NUMBERS {
            self.numbers = !self.numbers;
        }
    }

    /// The word-grid pager, which needs the page count the class holds.
    pub fn page_by(&mut self, back: bool, pages: usize) {
        if back {
            self.page = self.page.saturating_sub(1);
        } else if usize::from(self.page) + 1 < pages.max(1) {
            self.page += 1;
        }
    }

    /// Starts the quiz over the member on screen.
    pub fn start_quiz(&mut self, indices: &[u16], now_ms: u64) {
        let seed = Quiz::seed(now_ms, indices);
        self.quiz = Some(Quiz::new(
            indices,
            EntryList::Bip39(Language::English),
            self.helper,
            seed,
        ));
    }

    /// The helper toggle on "Start quiz".
    pub fn toggle_helper(&mut self) {
        self.helper = !self.helper;
    }

    /// Handles a tap inside the quiz over `indices`. Returns whether the
    /// quiz has just been passed.
    pub fn quiz_tap(&mut self, id: Id, indices: &[u16]) -> bool {
        let Some(q) = &mut self.quiz else {
            return false;
        };
        if let Some(slot) = ids::index_in(id, ids::QUIZ_CHOICE_BASE, crate::quiz::CHOICES) {
            q.pick(slot);
        } else if id == ids::QUIZ_CONTINUE {
            q.confirm(indices, EntryList::Bip39(Language::English));
        } else if id == ids::QUIZ_RETRY {
            q.retry(indices, EntryList::Bip39(Language::English));
        }
        q.state() == QuizState::Passed
    }

    /// This member is done, whether by the quiz or by skipping it.
    /// Returns whether another member's words follow; `false` means the
    /// record comes next.
    pub fn next_member(&mut self) -> bool {
        self.quiz = None;
        self.page = 0;
        self.numbers = false;
        if self.show_at + 1 < self.n() {
            self.show_at += 1;
            true
        } else {
            self.save = Save::Idle;
            false
        }
    }

    /// One member back, for the chevron on the Words screens. `false`
    /// where the one on screen is the first computed member.
    pub fn previous_member(&mut self) -> bool {
        if self.show_at > self.chosen {
            self.show_at -= 1;
            self.page = 0;
            true
        } else {
            false
        }
    }

    /// The last member, which is what Back from the record screen lands
    /// on.
    pub fn last_member(&mut self) {
        self.show_at = self.n().saturating_sub(1);
    }
}
