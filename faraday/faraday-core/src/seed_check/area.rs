//! My seed makes my wallet's area state: the card open, what each card
//! holds and the Result worked out. Private to this folder; the rest of
//! the crate reads it through [`SeedArea`]'s accessors. The seed itself is
//! held by `faraday-keys` ([`faraday_keys::Keys::check_read`]), the typed
//! words here only until the passphrase card has read them; the passphrase
//! is taken again through [`faraday_keys::Keys::check_reread`].

use super::compare::Compare;
use super::{Account, Made, Source};
use crate::secret_text::SecretText;
use crate::{Screen, flow};
use faraday_bip::keys::{Fingerprint, ScriptType};
use faraday_text::Text;

/// The cards, in order.
pub mod step {
    /// The seed.
    pub const SEED: u8 = 0;
    /// Its passphrase.
    pub const PASSPHRASE: u8 = 1;
    /// The account.
    pub const ACCOUNT: u8 = 2;
    /// What the seed makes.
    pub const RESULT: u8 = 3;
    /// How many cards.
    pub const COUNT: usize = 4;
}

/// What the key read shows of itself, public values.
#[derive(Clone, Copy)]
pub(super) struct Facts {
    /// The key's fingerprint, the passphrase applied.
    pub(super) fingerprint: Fingerprint,
    /// Whether a passphrase was given.
    pub(super) with_pass: bool,
    /// How many words, where it is a BIP-39 key.
    pub(super) words: Option<usize>,
}

/// One visit to the flow.
pub(crate) struct SeedState {
    pub(super) open: Option<u8>,
    pub(super) scroll: flow::Scroll,
    /// Where the flow was opened from.
    pub(super) back: Screen,
    /// The Seed card shows the words form.
    pub(super) typing: bool,
    /// The words typed, kept from the Seed card until the passphrase has
    /// been chosen; empty for words read from a SeedQR.
    pub(super) words: SecretText,
    /// The Seed card took typed words (held here until read, then only by
    /// `faraday-keys`): the passphrase can be taken again.
    pub(super) typed: bool,
    /// SLIP-39 shares are held by the entry form (`App::entry`) until the
    /// passphrase has been chosen: the Seed card is done, the words are
    /// none, and the passphrase cannot be taken again without the shares.
    pub(super) shares: bool,
    /// Add a passphrase was pressed: the fields show.
    pub(super) adding: bool,
    pub(super) pass: SecretText,
    pub(super) pass2: SecretText,
    /// Which passphrase field typing goes to.
    pub(super) focus: Option<u8>,
    /// A line under the card that refused.
    pub(super) error: Option<Text>,
    /// Where the key is, once read.
    pub(super) source: Option<Source>,
    pub(super) facts: Option<Facts>,
    /// The path field is open on the Account card.
    pub(super) path_open: bool,
    pub(super) path: Text,
    pub(super) path_focus: bool,
    pub(super) script: ScriptType,
    /// The account picked: its name and path, for the closed card.
    pub(super) picked: Option<(Text, Text)>,
    /// The account's row, where it is one of the five.
    pub(super) account: Option<Account>,
    /// The Result of the key the check holds; every card that takes a key
    /// drops it before it stores what it took.
    pub(super) made: Option<Made>,
    /// The Result shows the plain xpub rather than SLIP-132's spelling.
    pub(super) plain: bool,
    /// Compare with the wallet app's, once opened.
    pub(super) compare: Option<Compare>,
}

impl SeedState {
    /// How many characters the passphrase fields hold.
    pub(crate) fn pass_len(&self) -> usize {
        self.pass.as_str().len() + self.pass2.as_str().len()
    }

    pub(super) fn new(back: Screen) -> SeedState {
        SeedState {
            open: Some(step::SEED),
            scroll: flow::Scroll::default(),
            back,
            typing: false,
            words: SecretText::new(),
            typed: false,
            shares: false,
            adding: false,
            pass: SecretText::new(),
            pass2: SecretText::new(),
            focus: None,
            error: None,
            source: None,
            facts: None,
            path_open: false,
            path: Text::default(),
            path_focus: false,
            script: ScriptType::NativeSegwit,
            picked: None,
            account: None,
            made: None,
            plain: false,
            compare: None,
        }
    }

    /// Whether the card is filled.
    pub(crate) fn done(&self, k: u8) -> bool {
        match k {
            step::SEED => self.typed || self.shares || self.source.is_some(),
            step::PASSPHRASE => self.facts.is_some(),
            step::ACCOUNT => self.picked.is_some(),
            _ => self.made.is_some(),
        }
    }

    /// Whether the key is one the session has loaded.
    pub(crate) fn is_loaded(&self) -> bool {
        matches!(self.source, Some(Source::Loaded(_)))
    }

    /// Whether the flow is Check a multisig setup's Check a seed against it:
    /// only the Seed and Passphrase cards are used, and the screen is
    /// that page's.
    pub(crate) fn for_check(&self) -> bool {
        self.back == Screen::MultisigCheck
    }

    /// The card open.
    pub(crate) fn open_card(&self) -> Option<u8> {
        self.open
    }

    /// The line under the card that refused.
    pub(crate) fn error(&self) -> Option<&Text> {
        self.error.as_ref()
    }

    /// Where the Result is, once worked out.
    pub(crate) fn made(&self) -> Option<&Made> {
        self.made.as_ref()
    }

    pub(super) fn compare(&self) -> Option<&Compare> {
        self.compare.as_ref()
    }

    pub(super) fn compare_mut(&mut self) -> Option<&mut Compare> {
        self.compare.as_mut()
    }

    /// What Compare with the wallet app's answered, once it has.
    pub(crate) fn answer(&self) -> Option<&super::compare::Answer> {
        self.compare.as_ref()?.answer.as_ref()
    }

    /// The field typing goes to, with what is typed in it.
    pub(crate) fn field(&self) -> Option<(super::SeedAction, &str)> {
        use super::SeedAction as A;
        if let Some(c) = self.compare.as_ref().filter(|c| c.focus)
            && self.open == Some(step::RESULT)
        {
            return Some((A::CompareField, c.text.as_str()));
        }
        if self.path_focus && self.open == Some(step::ACCOUNT) {
            return Some((A::PathField, self.path.as_str()));
        }
        match (self.focus?, self.open) {
            (0, Some(step::PASSPHRASE)) => Some((A::PassField(0), self.pass.as_str())),
            (_, Some(step::PASSPHRASE)) => Some((A::PassField(1), self.pass2.as_str())),
            _ => None,
        }
    }

    /// Whether the flow holds something typed that is secret: the words
    /// or either passphrase field.
    pub(crate) fn holds_secret(&self) -> bool {
        self.shares || !self.words.is_empty() || !self.pass.is_empty() || !self.pass2.is_empty()
    }

    /// Whether the Seed card shows the words form.
    pub(crate) fn typing_words(&self) -> bool {
        self.typing && self.open == Some(step::SEED)
    }
}

/// The flow's place in Tools.
#[derive(Default)]
pub struct SeedArea {
    state: Option<SeedState>,
}

impl SeedArea {
    /// The state, while the flow is open.
    pub(crate) fn state(&self) -> Option<&SeedState> {
        self.state.as_ref()
    }

    pub(super) fn state_mut(&mut self) -> Option<&mut SeedState> {
        self.state.as_mut()
    }

    pub(super) fn begin(&mut self, back: Screen) {
        self.state = Some(SeedState::new(back));
    }

    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    /// How far the column is scrolled, units, while it is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|x| x.scroll.y)
    }

    /// The column scrolled to `y` units, while it is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(x) = self.state.as_mut() {
            x.scroll.y = y;
        }
    }

    /// The flow is over: what it held is wiped.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }
}

// Words and passphrase typed.
impl crate::at_lock::AtLock for SeedArea {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds::typed(
            self.state()
                .is_some_and(crate::seed_check::SeedState::holds_secret),
        )
    }
    fn lock(&mut self) {
        self.clear();
    }
}
