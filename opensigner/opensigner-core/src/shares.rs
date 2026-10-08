//! The SLIP-39 split this device makes (`docs/PLANNING.md` §16.107 rules
//! 4 and 5): the plan a person chooses, the randomness the split is fed,
//! the shares it deals, and the quiz over each of them.
//!
//! One state holds all of it, because Add a key › Create SLIP-39 shares
//! and Backup › SLIP-39 shares ask the same questions in the same order
//! over a different master secret: the first over the entropy just
//! gathered, the second over a loaded key's. It is a field of
//! [`OpenSigner`](crate::OpenSigner) rather than of either flow, so the
//! shares exist once and are zeroized when the flow that made them is
//! left.
//!
//! Everything here is inline and fixed-size: the master secret, the
//! passphrase the shares are written under, the random values and the
//! shares themselves. This file is listed in `tools/lint-secrets.sh` and
//! may hold no heap text; the views in `views/shares.rs` receive only
//! what the screen shows.

use osk_bip::slip39::{self, GroupSpec, Share};
use osk_crypto::{Zeroize, ZeroizeOnDrop};

use crate::ids::{self, Id};
use crate::load::EntryList;
use crate::quiz::{Quiz, QuizState};
use osk_entropy::Source;

/// Groups a backup may have, and shares a group may have.
pub const MAX_GROUPS: usize = slip39::MAX_COUNT;

/// Shares one backup can carry: the format's own bound, sixteen groups
/// of sixteen.
pub const MAX_SHARES: usize = MAX_GROUPS * slip39::MAX_COUNT;

/// Random values one split can need: one fewer than the group threshold
/// at the group level, and one fewer than its own threshold in each
/// group.
pub const MAX_RANDOM: usize = (MAX_GROUPS - 1) + MAX_GROUPS * (slip39::MAX_COUNT - 1);

/// Longest passphrase the shares are written under, which is BIP-39's
/// bound and SLIP-39's.
pub const MAX_PASSPHRASE_BYTES: usize = osk_bip::bip39::MAX_PASSPHRASE_BYTES;

/// Every backup this device writes is extendable, so its master secret's
/// encryption does not salt with the identifier.
pub const EXTENDABLE: bool = true;

/// The iteration exponent every backup this device writes carries.
pub const ITERATION_EXPONENT: u8 = 1;

/// Milliseconds a typed passphrase character stays visible.
pub const MASK_MS: u64 = crate::finish::MASK_MS;

/// Where the split is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// "Add a passphrase?", on the Backup path only: Create asks it in
    /// the wizard's own finishing steps, before the plan.
    PassphraseOffer,
    /// Typing that passphrase.
    Passphrase,
    /// "How many groups?"
    Groups,
    /// "How many groups must be present?"
    GroupThreshold,
    /// "How many shares?", for the group at [`SharePlan::at`].
    Count,
    /// "How many must be present?", for the same group.
    Threshold,
    /// "Random shares from?", on the Backup path only.
    Source,
    /// The chosen source's own screens, once per random value.
    Gather,
    /// One share's words.
    Words,
    /// The helper toggle and "Start quiz", for that share.
    QuizStart,
    /// The quiz over that share.
    Quiz,
    /// The caution behind "Skip quiz".
    QuizSkip,
    /// "Shares made", on the Backup path only.
    Result,
}

/// What a tap on the plan asks the caller to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Nothing outside the plan changed.
    Stay,
    /// Open the entropy screens for the random value now due.
    Gather,
    /// Every random value is in: make the split.
    Split,
    /// The shares have all been shown; the flow goes on.
    Done,
}

/// The plan, the randomness and the shares.
pub struct SharePlan {
    step: Step,
    /// The master secret being split.
    secret: [u8; slip39::MAX_STRENGTH_BYTES],
    secret_len: u8,
    /// The passphrase the shares carry the secret under.
    passphrase: [u8; MAX_PASSPHRASE_BYTES],
    passphrase_len: u16,
    typed_at: u64,
    /// "Add a passphrase?": whether the checked row is "Add".
    offer_add: bool,
    groups: u8,
    group_threshold: u8,
    counts: [u8; MAX_GROUPS],
    thresholds: [u8; MAX_GROUPS],
    /// The group whose counts are being asked for, or shown.
    at: u8,
    /// Where the random values come from.
    source: Source,
    /// Whether this flow asks for the source: Backup does, Create does
    /// not, since the key was just made from one (§16.92).
    ask_source: bool,
    /// Whether the flow ends in a Result, which Backup's does.
    result: bool,
    randoms: [[u8; slip39::MAX_STRENGTH_BYTES]; MAX_RANDOM],
    /// Random values gathered so far.
    got: u8,
    /// Random values this plan needs.
    need: u8,
    /// The shares, group by group and member by member.
    shares: [Share; MAX_SHARES],
    total: u16,
    /// The share on screen, 0-based.
    show: u16,
    page: u8,
    numbers: bool,
    quiz: Option<Quiz>,
    helper: bool,
    /// Whether every share's quiz has been passed so far.
    verified: bool,
    identifier: u16,
}

impl Zeroize for SharePlan {
    fn zeroize(&mut self) {
        self.secret.zeroize();
        self.secret_len = 0;
        self.passphrase.zeroize();
        self.passphrase_len = 0;
        self.typed_at = 0;
        self.offer_add = false;
        self.groups = 1;
        self.group_threshold = 1;
        self.counts = [DEFAULT_COUNT; MAX_GROUPS];
        self.thresholds = [DEFAULT_THRESHOLD; MAX_GROUPS];
        self.at = 0;
        self.randoms.zeroize();
        self.got = 0;
        self.need = 0;
        for share in &mut self.shares {
            share.zeroize();
        }
        self.total = 0;
        self.show = 0;
        self.page = 0;
        self.numbers = false;
        self.quiz = None;
        self.helper = false;
        self.verified = false;
        self.identifier = 0;
    }
}

impl Drop for SharePlan {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SharePlan {}

impl Default for SharePlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Shares a group has before anything is chosen.
const DEFAULT_COUNT: u8 = 3;
/// Shares of a group that must be present, before anything is chosen.
const DEFAULT_THRESHOLD: u8 = 2;

impl SharePlan {
    /// An empty plan.
    pub fn new() -> Self {
        SharePlan {
            step: Step::Groups,
            secret: [0; slip39::MAX_STRENGTH_BYTES],
            secret_len: 0,
            passphrase: [0; MAX_PASSPHRASE_BYTES],
            passphrase_len: 0,
            typed_at: 0,
            offer_add: false,
            groups: 1,
            group_threshold: 1,
            counts: [DEFAULT_COUNT; MAX_GROUPS],
            thresholds: [DEFAULT_THRESHOLD; MAX_GROUPS],
            at: 0,
            source: Source::Dice,
            ask_source: false,
            result: false,
            randoms: [[0; slip39::MAX_STRENGTH_BYTES]; MAX_RANDOM],
            got: 0,
            need: 0,
            shares: core::array::from_fn(|_| Share::default()),
            total: 0,
            show: 0,
            page: 0,
            numbers: false,
            quiz: None,
            helper: false,
            verified: false,
            identifier: 0,
        }
    }

    /// Begins the plan for Add a key › Create SLIP-39 shares: the
    /// passphrase was already typed in the wizard's finishing steps, and
    /// the random values come from the source the key was made from.
    pub fn start_create(&mut self, secret: &[u8], passphrase: &[u8], source: Source) -> bool {
        if !self.load(secret, passphrase) {
            return false;
        }
        self.source = source;
        self.ask_source = false;
        self.result = false;
        self.step = Step::Groups;
        true
    }

    /// Begins the plan for Backup › SLIP-39 shares, which asks for the
    /// passphrase and the source itself.
    pub fn start_backup(&mut self, secret: &[u8], source: Source) -> bool {
        if !self.load(secret, b"") {
            return false;
        }
        self.source = source;
        self.ask_source = true;
        self.result = true;
        self.step = Step::PassphraseOffer;
        true
    }

    fn load(&mut self, secret: &[u8], passphrase: &[u8]) -> bool {
        self.zeroize();
        if slip39::SecretBytes::new(secret).is_err() || passphrase.len() > MAX_PASSPHRASE_BYTES {
            return false;
        }
        self.secret[..secret.len()].copy_from_slice(secret);
        self.secret_len = secret.len() as u8;
        self.passphrase[..passphrase.len()].copy_from_slice(passphrase);
        self.passphrase_len = passphrase.len() as u16;
        true
    }

    /// Whether a split is loaded with a secret and running.
    pub fn is_running(&self) -> bool {
        self.secret_len > 0
    }

    /// The step.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Groups the plan has.
    pub fn groups(&self) -> usize {
        usize::from(self.groups)
    }

    /// Groups that must be present.
    pub fn group_threshold(&self) -> usize {
        usize::from(self.group_threshold)
    }

    /// Shares group `i` has.
    pub fn count(&self, i: usize) -> usize {
        usize::from(self.counts[i.min(MAX_GROUPS - 1)])
    }

    /// Shares of group `i` that must be present.
    pub fn threshold(&self, i: usize) -> usize {
        usize::from(self.thresholds[i.min(MAX_GROUPS - 1)])
    }

    /// The group whose counts are being asked for, 0-based.
    pub fn at(&self) -> usize {
        usize::from(self.at)
    }

    /// Where the random values come from.
    pub fn source(&self) -> Source {
        self.source
    }

    /// Whether this flow asks which source the random values come from.
    pub fn asks_source(&self) -> bool {
        self.ask_source
    }

    /// Whether this flow ends in a Result of its own.
    pub fn has_result(&self) -> bool {
        self.result
    }

    /// The shares this plan asks the split for.
    pub fn total(&self) -> usize {
        usize::from(self.total)
    }

    /// The share on screen, 0-based.
    pub fn show(&self) -> usize {
        usize::from(self.show)
    }

    /// The word-grid page.
    pub fn page(&self) -> u8 {
        self.page
    }

    /// Whether the words carry their place in the list.
    pub fn numbers(&self) -> bool {
        self.numbers
    }

    /// Whether the quiz's helper variant is on.
    pub fn helper(&self) -> bool {
        self.helper
    }

    /// The quiz, while one is running.
    pub fn quiz(&self) -> Option<&Quiz> {
        self.quiz.as_ref()
    }

    /// Whether every share's quiz has been passed.
    pub fn verified(&self) -> bool {
        self.verified
    }

    /// The identifier every share of this backup states.
    pub fn identifier(&self) -> u16 {
        self.identifier
    }

    /// Bytes of each random value, which is the master secret's length.
    pub fn value_len(&self) -> usize {
        usize::from(self.secret_len)
    }

    /// Random values this plan needs, and how many are in.
    pub fn randoms(&self) -> (usize, usize) {
        (usize::from(self.got), usize::from(self.need))
    }

    /// What the random value now due is called: the 1-based group it
    /// belongs to (0 at the group level), which value it is there, and
    /// how many that level needs.
    pub fn gather_label(&self) -> (u8, u8, u8) {
        self.label_of(self.got)
    }

    fn label_of(&self, index: u8) -> (u8, u8, u8) {
        let mut left = index;
        let level = self.group_threshold.saturating_sub(1);
        if left < level {
            return (0, left, level);
        }
        left -= level;
        for g in 0..self.groups {
            let level = self.thresholds[usize::from(g)].saturating_sub(1);
            if left < level {
                return (g + 1, left, level);
            }
            left -= level;
        }
        (0, 0, 0)
    }

    /// The share on screen: its 1-based group (0 where the backup has
    /// one group), its 1-based place in that group, and the shares that
    /// group has.
    pub fn share_label(&self) -> (u8, u8, u8) {
        let Some(share) = self.shares.get(self.show()) else {
            return (0, 1, 1);
        };
        let group = share.group_index();
        let of = self.counts[usize::from(group).min(MAX_GROUPS - 1)];
        let group = if self.groups > 1 { group + 1 } else { 0 };
        (group, share.member_index() + 1, of)
    }

    /// The words of the share on screen.
    pub fn words(&self) -> Option<&[u16]> {
        (self.total > 0 && self.show < self.total).then(|| self.shares[self.show()].indices())
    }

    /// Runs `f` over every share this split made, in group order then
    /// member order.
    pub fn each_share<R>(&self, mut f: impl FnMut(&Share) -> R) {
        for share in &self.shares[..self.total()] {
            f(share);
        }
    }

    // ----- the passphrase (the Backup path's own steps) -----

    /// Whether the offer's checked row is "Add a passphrase".
    pub fn offer_add(&self) -> bool {
        self.offer_add
    }

    /// Characters typed so far.
    pub fn passphrase_len(&self) -> usize {
        usize::from(self.passphrase_len)
    }

    /// The last typed character while it is still unmasked at `now_ms`.
    pub fn passphrase_visible_char(&self, now_ms: u64) -> Option<char> {
        if self.passphrase_len == 0 || now_ms >= self.mask_deadline()? {
            return None;
        }
        Some(self.passphrase[self.passphrase_len() - 1] as char)
    }

    /// When that character masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        (self.step == Step::Passphrase && self.passphrase_len > 0)
            .then_some(self.typed_at + MASK_MS)
    }

    /// The passphrase keyboard. Returns what to do next.
    pub fn key(&mut self, input: osk_ui::widgets::keyboard::KeyInput, now_ms: u64) -> Next {
        use osk_ui::widgets::keyboard::KeyInput;
        if self.step != Step::Passphrase {
            return Next::Stay;
        }
        match input {
            KeyInput::Char(c) => {
                let n = self.passphrase_len();
                if c.is_ascii() && (0x20..=0x7E).contains(&(c as u32)) && n < MAX_PASSPHRASE_BYTES {
                    self.passphrase[n] = c as u8;
                    self.passphrase_len += 1;
                    self.typed_at = now_ms;
                }
                Next::Stay
            }
            KeyInput::Backspace => {
                if self.passphrase_len > 0 {
                    self.passphrase_len -= 1;
                    self.passphrase[self.passphrase_len()] = 0;
                }
                Next::Stay
            }
            // Nothing checks a SLIP-39 passphrase, so there is nothing
            // to confirm: ✓ goes on to the plan.
            KeyInput::Done => {
                self.step = Step::Groups;
                Next::Stay
            }
            KeyInput::Shift | KeyInput::Symbols => Next::Stay,
        }
    }

    // ----- taps -----

    /// Handles a tap at `now_ms`.
    pub fn tap(&mut self, id: Id, now_ms: u64, per_page: usize) -> Next {
        match self.step {
            Step::PassphraseOffer => {
                if id == ids::LOAD_SKIP {
                    self.offer_add = false;
                } else if id == ids::LOAD_ADD_PASSPHRASE {
                    self.offer_add = true;
                } else if id == ids::LOAD_PASS_CONTINUE {
                    self.step = if self.offer_add {
                        Step::Passphrase
                    } else {
                        Step::Groups
                    };
                }
                Next::Stay
            }
            Step::Passphrase => Next::Stay,
            Step::Groups => {
                if let Some(i) = ids::index_in(id, ids::SHARE_GROUPS_BASE, MAX_GROUPS) {
                    self.groups = i as u8 + 1;
                    // The codec allows a group threshold of 1 only where
                    // there is one group.
                    self.group_threshold = if self.groups > 1 { 2 } else { 1 };
                } else if id == ids::SHARE_GROUPS_CONTINUE {
                    self.at = 0;
                    self.step = if self.groups > 1 {
                        Step::GroupThreshold
                    } else {
                        Step::Count
                    };
                }
                Next::Stay
            }
            Step::GroupThreshold => {
                if let Some(i) = ids::index_in(id, ids::SHARE_QUORUM_BASE, MAX_GROUPS) {
                    let n = i as u8 + 1;
                    if (2..=self.groups).contains(&n) {
                        self.group_threshold = n;
                    }
                } else if id == ids::SHARE_QUORUM_CONTINUE {
                    self.at = 0;
                    self.step = Step::Count;
                }
                Next::Stay
            }
            Step::Count => {
                if let Some(i) = ids::index_in(id, ids::SHARE_COUNT_BASE, MAX_GROUPS) {
                    let n = i as u8 + 1;
                    self.counts[self.at()] = n;
                    // A group of one share is a group whose threshold is
                    // 1, which is the only threshold the codec allows it.
                    self.thresholds[self.at()] = if n == 1 { 1 } else { DEFAULT_THRESHOLD.min(n) };
                } else if id == ids::SHARE_COUNT_CONTINUE {
                    if self.counts[self.at()] == 1 {
                        return self.next_group();
                    }
                    self.step = Step::Threshold;
                }
                Next::Stay
            }
            Step::Threshold => {
                if let Some(i) = ids::index_in(id, ids::SHARE_THRESHOLD_BASE, MAX_GROUPS) {
                    let n = i as u8 + 1;
                    if n <= self.counts[self.at()] {
                        self.thresholds[self.at()] = n;
                    }
                } else if id == ids::SHARE_THRESHOLD_CONTINUE {
                    return self.next_group();
                }
                Next::Stay
            }
            Step::Source => {
                if let Some(i) =
                    ids::index_in(id, ids::SHARE_SOURCE_BASE, osk_entropy::SOURCE_ROWS.len())
                {
                    self.source = osk_entropy::SOURCE_ROWS[i];
                } else if id == ids::SHARE_SOURCE_CONTINUE {
                    return self.begin_random();
                }
                Next::Stay
            }
            // The gatherer's screens are the Create wizard's own.
            Step::Gather => Next::Stay,
            Step::Words => {
                if id == ids::WORDS_NUMBERS {
                    self.numbers = !self.numbers;
                } else if id == ids::WORDS_PREV {
                    self.page = self.page.saturating_sub(1);
                } else if id == ids::WORDS_NEXT {
                    let words = self.words().map_or(0, <[u16]>::len);
                    let pages = words.div_ceil(per_page.max(1)).max(1) as u8;
                    if u16::from(self.page) + 1 < u16::from(pages) {
                        self.page += 1;
                    }
                } else if id == ids::CREATE_CONTINUE {
                    self.page = 0;
                    self.step = Step::QuizStart;
                }
                Next::Stay
            }
            Step::QuizStart => {
                if id == ids::QUIZ_HELPER {
                    self.helper = !self.helper;
                } else if id == ids::QUIZ_START {
                    if let Some(words) = self.words() {
                        let seed = Quiz::seed(now_ms, words);
                        self.quiz = Some(Quiz::new(words, EntryList::Slip39, self.helper, seed));
                        self.step = Step::Quiz;
                    }
                } else if id == ids::QUIZ_SKIP {
                    self.step = Step::QuizSkip;
                }
                Next::Stay
            }
            Step::QuizSkip => {
                if id == ids::QUIZ_SKIP_CANCEL {
                    self.step = Step::QuizStart;
                } else if id == ids::QUIZ_SKIP_CONFIRM {
                    self.verified = false;
                    self.quiz = None;
                    return self.next_share();
                }
                Next::Stay
            }
            Step::Quiz => self.tap_quiz(id),
            Step::Result => {
                if id == ids::QUIZ_DONE {
                    return Next::Done;
                }
                Next::Stay
            }
        }
    }

    fn tap_quiz(&mut self, id: Id) -> Next {
        // A wrong answer never names the word; it offers the share's
        // words instead, as the backup quiz does.
        if id == ids::QUIZ_SHOW_WORDS {
            return Next::Stay;
        }
        let Some(words) = (self.total > 0 && self.show < self.total)
            .then(|| self.shares[usize::from(self.show)].indices())
        else {
            return Next::Stay;
        };
        let Some(q) = &mut self.quiz else {
            return Next::Stay;
        };
        if let Some(slot) = ids::index_in(id, ids::QUIZ_CHOICE_BASE, crate::quiz::CHOICES) {
            q.pick(slot);
        } else if id == ids::QUIZ_CONTINUE {
            q.confirm(words, EntryList::Slip39);
        } else if id == ids::QUIZ_RETRY {
            q.retry(words, EntryList::Slip39);
        }
        if q.state() == QuizState::Passed {
            self.quiz = None;
            return self.next_share();
        }
        Next::Stay
    }

    /// The group on screen is settled: the next group's counts, or the
    /// end of the plan.
    fn next_group(&mut self) -> Next {
        if usize::from(self.at) + 1 < self.groups() {
            self.at += 1;
            self.step = Step::Count;
            return Next::Stay;
        }
        if self.ask_source {
            self.step = Step::Source;
            return Next::Stay;
        }
        self.begin_random()
    }

    /// The plan is settled: how many random values it needs, and the
    /// first of them.
    fn begin_random(&mut self) -> Next {
        let mut need = usize::from(self.group_threshold.saturating_sub(1));
        for g in 0..self.groups() {
            need += usize::from(self.thresholds[g].saturating_sub(1));
        }
        self.need = need.min(MAX_RANDOM) as u8;
        self.got = 0;
        if self.need == 0 {
            return Next::Split;
        }
        self.step = Step::Gather;
        Next::Gather
    }

    /// Keeps one gathered random value. Returns whether another is due
    /// ([`Next::Gather`]) or the split can be made.
    pub fn take_random(&mut self, bytes: &[u8]) -> Next {
        if bytes.len() != self.value_len() || self.got >= self.need {
            return Next::Stay;
        }
        let at = usize::from(self.got);
        self.randoms[at][..bytes.len()].copy_from_slice(bytes);
        self.got += 1;
        if self.got < self.need {
            self.step = Step::Gather;
            Next::Gather
        } else {
            Next::Split
        }
    }

    /// Makes the split, under `identifier`, and shows the first share.
    ///
    /// Every random byte the codec draws comes from the values gathered
    /// on the screens, one value per draw, in the order the codec asks
    /// for them; it is never asked for more than they hold.
    pub fn split(&mut self, identifier: u16) -> bool {
        let groups = self.groups();
        let len = self.value_len();
        if len == 0 || groups == 0 {
            return false;
        }
        let mut specs = [GroupSpec {
            threshold: 1,
            count: 1,
        }; MAX_GROUPS];
        let mut total = 0usize;
        for (g, spec) in specs.iter_mut().enumerate().take(groups) {
            spec.threshold = self.thresholds[g];
            spec.count = self.counts[g];
            total += usize::from(self.counts[g]);
        }
        if total > MAX_SHARES {
            return false;
        }
        let got = usize::from(self.got);
        // The gathered values and what the codec draws have to agree
        // exactly: a draw the plan did not foresee would fall back on
        // zeros, which is a share with no randomness in it.
        let expected: usize = (0..groups)
            .map(|g| self.thresholds[g])
            .chain(core::iter::once(self.group_threshold))
            .map(|t| match t {
                0 | 1 => 0,
                t => (usize::from(t) - 2) * len + len - slip39::DIGEST_LENGTH,
            })
            .sum();
        let mut taken = 0usize;
        let mut drawn = 0usize;
        let mut over = false;
        let made = {
            let SharePlan {
                secret,
                secret_len,
                passphrase,
                passphrase_len,
                randoms,
                shares,
                group_threshold,
                ..
            } = self;
            let mut random = |out: &mut [u8]| {
                if taken >= got || out.len() > len {
                    over = true;
                    out.fill(0);
                    return;
                }
                out.copy_from_slice(&randoms[taken][..out.len()]);
                drawn += out.len();
                taken += 1;
            };
            slip39::split(
                &secret[..usize::from(*secret_len)],
                &passphrase[..usize::from(*passphrase_len)],
                identifier,
                EXTENDABLE,
                ITERATION_EXPONENT,
                *group_threshold,
                &specs[..groups],
                &mut random,
                &mut shares[..],
            )
        };
        debug_assert!(!over, "the split drew more randomness than was gathered");
        debug_assert_eq!(taken, got, "every gathered value is drawn exactly once");
        debug_assert_eq!(drawn, expected, "the split drew the bytes the plan counted");
        if over || taken != got || drawn != expected {
            self.zeroize();
            return false;
        }
        let Ok(n) = made else {
            self.zeroize();
            return false;
        };
        self.identifier = identifier;
        self.total = n as u16;
        self.show = 0;
        self.page = 0;
        self.numbers = false;
        self.quiz = None;
        self.verified = true;
        self.randoms.zeroize();
        self.got = 0;
        self.step = Step::Words;
        true
    }

    /// This share is done, whether by the quiz or by skipping it: the
    /// next share, or the end.
    fn next_share(&mut self) -> Next {
        self.page = 0;
        self.numbers = false;
        if usize::from(self.show) + 1 < self.total() {
            self.show += 1;
            self.step = Step::Words;
            return Next::Stay;
        }
        if self.result {
            self.step = Step::Result;
            return Next::Stay;
        }
        // Back from Confirm comes to the last share, so the plan stays
        // where it was rather than running past its own end.
        self.step = Step::Words;
        Next::Done
    }

    /// Goes one step back. `false` means leaving the plan, which the
    /// flow that opened it answers for.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            Step::PassphraseOffer => return false,
            Step::Passphrase => {
                self.passphrase.zeroize();
                self.passphrase_len = 0;
                self.typed_at = 0;
                self.offer_add = false;
                Step::PassphraseOffer
            }
            Step::Groups if self.ask_source => Step::PassphraseOffer,
            Step::Groups => return false,
            Step::GroupThreshold => Step::Groups,
            Step::Count if self.at > 0 => {
                self.at -= 1;
                self.threshold_step()
            }
            Step::Count if self.groups > 1 => Step::GroupThreshold,
            Step::Count => Step::Groups,
            Step::Threshold => Step::Count,
            // Leaving the source or the gathering drops every value
            // gathered so far, as a Seed XOR split's does.
            Step::Gather if self.ask_source => {
                self.randoms.zeroize();
                self.got = 0;
                Step::Source
            }
            Step::Source | Step::Gather => {
                self.randoms.zeroize();
                self.got = 0;
                self.at = self.groups.saturating_sub(1);
                self.threshold_step()
            }
            // A share goes back to the one before it; the first goes
            // back to the plan, and the shares are forgotten.
            Step::Words if self.show > 0 => {
                self.show -= 1;
                self.page = 0;
                self.numbers = false;
                Step::Words
            }
            Step::Words => {
                for share in &mut self.shares {
                    share.zeroize();
                }
                self.total = 0;
                self.show = 0;
                self.page = 0;
                self.at = self.groups.saturating_sub(1);
                if self.ask_source {
                    Step::Source
                } else {
                    self.threshold_step()
                }
            }
            Step::QuizStart => Step::Words,
            Step::Quiz => {
                self.quiz = None;
                Step::QuizStart
            }
            Step::QuizSkip => Step::QuizStart,
            Step::Result => return false,
        };
        true
    }

    /// The last Choice the plan asked for the group at [`at`](Self::at):
    /// its threshold, or its count where a group of one share has no
    /// threshold to choose.
    fn threshold_step(&self) -> Step {
        if self.counts[self.at()] == 1 {
            Step::Count
        } else {
            Step::Threshold
        }
    }
}
