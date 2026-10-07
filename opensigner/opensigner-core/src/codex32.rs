//! The codex32 string entry and the set of shares it gathers
//! (`docs/PLANNING.md` §16.109 rules 2 and 6).
//!
//! One engine: the field with `ms1` already in it, the key mask that
//! dims every character which cannot continue a string, the checksum
//! line, and the shares typed so far. Load reads it; the scanner fills
//! it; "Type it back" reads it too, which is why it is a struct and
//! not a handful of fields of the Load wizard.
//!
//! Beside it is [`Codex32Plan`] (§16.109 rules 4 and 5): the plan a
//! person chooses, the randomness the split is fed, the strings it
//! makes and the type-back over each of them. It is a field of
//! [`OpenSigner`](crate::OpenSigner) rather than of either flow,
//! because Add a key › Create Codex32 shares and Backup › Codex32 ask
//! the same questions over a different seed. It lives in this file
//! because the type-back is a [`Codex32Entry`], and a second file
//! would put one flow's state in two places.
//!
//! Everything here is a fixed-size array zeroized on drop: the string
//! being typed is the seed once it is finished, and so is every share.
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text.

use osk_bip::codex32::{
    self, Codex32, Error, MAX_SEED_BYTES, MAX_STRING_LEN, MAX_THRESHOLD, SEED_LENGTHS,
};
use osk_crypto::{Zeroize, ZeroizeOnDrop};
use osk_entropy::Strength;
use osk_ui::widgets::keyboard::{self, KeyMask, KeyboardKind};

use crate::create::Source;
use crate::ids::{self, Id};

/// What every codex32 string starts with, which the field holds before
/// the first key is pressed and which backspace never eats.
pub const PREFIX: &str = "ms1";

/// Strings one set holds at once: BIP 93's largest threshold, which is
/// as many as `recover` ever takes.
pub const MAX_STRINGS: usize = MAX_THRESHOLD as usize;

/// What ✓ did with the string in the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Submitted {
    /// The string is the secret, typed whole or rebuilt from the shares
    /// that were needed: the flow goes on to the key.
    Secret,
    /// A share that belongs with the ones already in.
    Accepted,
    /// A share that does not, with the codec's own reason.
    Refused(Error),
}

/// A codex32 string being typed, and the shares it has gathered.
pub struct Codex32Entry {
    /// The string as ASCII, `ms1` first.
    buf: [u8; MAX_STRING_LEN],
    len: u8,
    /// The shares typed so far, in the order they were taken.
    shares: [Codex32; MAX_STRINGS],
    count: u8,
    /// The secret, once a string carried it or `recover` gave it.
    secret: Codex32,
    has_secret: bool,
    /// Why the last string was refused, for the result that says so.
    refusal: Option<Error>,
}

impl Zeroize for Codex32Entry {
    fn zeroize(&mut self) {
        self.buf.zeroize();
        self.buf[..PREFIX.len()].copy_from_slice(PREFIX.as_bytes());
        self.len = PREFIX.len() as u8;
        self.shares.zeroize();
        self.count = 0;
        self.secret.zeroize();
        self.has_secret = false;
        self.refusal = None;
    }
}

impl Drop for Codex32Entry {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Codex32Entry {}

impl Default for Codex32Entry {
    fn default() -> Self {
        Self::new()
    }
}

impl Codex32Entry {
    /// An empty entry: `ms1` in the field and no shares.
    pub fn new() -> Self {
        let mut buf = [0u8; MAX_STRING_LEN];
        buf[..PREFIX.len()].copy_from_slice(PREFIX.as_bytes());
        Codex32Entry {
            buf,
            len: PREFIX.len() as u8,
            shares: core::array::from_fn(|_| Codex32::default()),
            secret: Codex32::default(),
            count: 0,
            has_secret: false,
            refusal: None,
        }
    }

    /// The string in the field, `ms1` included.
    pub fn typed(&self) -> &str {
        core::str::from_utf8(&self.buf[..usize::from(self.len)]).unwrap_or(PREFIX)
    }

    /// Appends one character. Anything outside the alphabet, and
    /// anything the mask dims, is dropped; a full string takes no more.
    pub fn type_char(&mut self, c: char) -> bool {
        let c = c.to_ascii_lowercase();
        if !c.is_ascii_alphanumeric()
            || usize::from(self.len) >= MAX_STRING_LEN
            || keyboard::key_bit(KeyboardKind::Codex32, c) & self.keys() == 0
        {
            return false;
        }
        self.buf[usize::from(self.len)] = c as u8;
        self.len += 1;
        true
    }

    /// Deletes the last character typed. `ms1` stays: it is the one
    /// beginning every codex32 string has.
    pub fn backspace(&mut self) {
        if usize::from(self.len) > PREFIX.len() {
            self.len -= 1;
            self.buf[usize::from(self.len)] = 0;
        }
    }

    /// Empties the field, leaving the shares already in.
    pub fn clear_string(&mut self) {
        self.buf.zeroize();
        self.buf[..PREFIX.len()].copy_from_slice(PREFIX.as_bytes());
        self.len = PREFIX.len() as u8;
        self.refusal = None;
    }

    /// Fills the field with `text`, as a scan does. `false`, and the
    /// field untouched, for text the field cannot hold.
    pub fn fill(&mut self, text: &str) -> bool {
        if text.len() > MAX_STRING_LEN || !text.is_ascii() {
            return false;
        }
        let lower = text.as_bytes();
        if lower.len() < PREFIX.len() || !text[..PREFIX.len()].eq_ignore_ascii_case(PREFIX) {
            return false;
        }
        self.clear_string();
        for &b in &lower[PREFIX.len()..] {
            if !self.type_char(char::from(b)) {
                self.clear_string();
                return false;
            }
        }
        true
    }

    /// The keys that can still continue the string: the mask
    /// [`crate::verify::address_keys`] is for an address.
    pub fn keys(&self) -> KeyMask {
        codex32_keys(self.typed())
    }

    /// Whether the whole string parses, which is what makes ✓ live.
    pub fn accepts(&self) -> bool {
        Codex32::parse(self.typed()).is_ok()
    }

    /// The codec's reason once the string is a length BIP 93 allows and
    /// still not a string. A string still being typed is not yet wrong,
    /// so it has no reason.
    pub fn error(&self) -> Option<Error> {
        match Codex32::parse(self.typed()) {
            Ok(_) | Err(Error::BadLength) => None,
            Err(e) => Some(e),
        }
    }

    /// Takes the string in the field: the secret, a share that belongs
    /// with the set, or a refusal with the codec's reason. `None` where
    /// the string does not parse at all, which is where ✓ is dead.
    pub fn submit(&mut self) -> Option<Submitted> {
        let string = Codex32::parse(self.typed()).ok()?;
        self.refusal = None;
        // Threshold 0 is a secret that was never split, and the index
        // `s` is the secret of a set: either is the seed itself.
        if string.is_secret() {
            self.secret = string;
            self.has_secret = true;
            return Some(Submitted::Secret);
        }
        if let Err(e) = self.admits(&string) {
            self.refusal = Some(e);
            return Some(Submitted::Refused(e));
        }
        let n = usize::from(self.count);
        self.shares[n] = string;
        self.count += 1;
        Some(Submitted::Accepted)
    }

    /// Whether `string` belongs with the shares already in: the checks
    /// [`Codex32::recover`] makes over a whole set, made one string at a
    /// time so that the one that does not fit is the one refused.
    fn admits(&self, string: &Codex32) -> Result<(), Error> {
        if usize::from(self.count) >= MAX_STRINGS {
            return Err(Error::WrongShareCount);
        }
        for held in &self.shares[..usize::from(self.count)] {
            if held.identifier() != string.identifier() {
                return Err(Error::IdentifierMismatch);
            }
            if held.threshold() != string.threshold() {
                return Err(Error::ThresholdMismatch);
            }
            if held.payload().len() != string.payload().len() {
                return Err(Error::LengthMismatch);
            }
            if held.share_index() == string.share_index() {
                return Err(Error::DuplicateIndex);
            }
        }
        Ok(())
    }

    /// Shares in hand, and how many the set needs.
    pub fn progress(&self) -> (u8, u8) {
        (self.count, self.threshold())
    }

    /// The threshold the shares state, or 0 before any is in.
    fn threshold(&self) -> u8 {
        self.shares[..usize::from(self.count)]
            .first()
            .map_or(0, Codex32::threshold)
    }

    /// The 1-based number of the string being typed.
    pub fn string_number(&self) -> usize {
        usize::from(self.count) + 1
    }

    /// Why the last string was refused.
    pub fn refusal(&self) -> Option<Error> {
        self.refusal
    }

    /// Whether the secret is in hand, either typed or recovered from
    /// the shares that were needed.
    pub fn enough(&mut self) -> bool {
        if self.has_secret {
            return true;
        }
        let threshold = self.threshold();
        if threshold == 0 || usize::from(self.count) != usize::from(threshold) {
            return false;
        }
        match Codex32::recover(&self.shares[..usize::from(self.count)]) {
            Ok(secret) => {
                self.secret = secret;
                self.has_secret = true;
                true
            }
            Err(e) => {
                self.refusal = Some(e);
                false
            }
        }
    }

    /// Runs `f` on the master seed the secret carries, which exists only
    /// for the duration of the call.
    pub fn with_seed<R>(&self, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        if !self.has_secret {
            return None;
        }
        let seed = self.secret.to_seed().ok()?;
        let out = seed.expose().bytes();
        Some(f(out))
    }
}

/// The keys the codex32 keyboard still offers after `typed`: those that
/// can continue a string, and no others (`docs/DESIGN.md` §4.3, the rule
/// [`crate::verify::address_keys`] follows for an address).
///
/// After `ms1` comes the threshold, which is `0` or 2 to 9; then four
/// characters of the identifier, which are any of the alphabet; then the
/// share index, which is `s` where the threshold is `0` and any
/// character otherwise; then the payload, until the string is as long as
/// BIP 93's longest.
pub fn codex32_keys(typed: &str) -> KeyMask {
    let n = typed.len();
    if n < PREFIX.len() || n >= MAX_STRING_LEN {
        return 0;
    }
    let mut mask = 0;
    for c in ('a'..='z').chain('0'..='9') {
        if continues(typed, c) {
            mask |= keyboard::key_bit(KeyboardKind::Codex32, c);
        }
    }
    mask
}

/// Whether `c` can follow `typed`.
fn continues(typed: &str, c: char) -> bool {
    let position = typed.len() - PREFIX.len();
    match position {
        // The threshold: 0 for a secret that was never split, else 2 to 9.
        0 => c == '0' || ('2'..='9').contains(&c),
        // The identifier, four characters of the alphabet.
        1..=4 => in_alphabet(c),
        // The share index. Threshold 0 has only the secret's own index.
        5 => {
            if typed.as_bytes()[PREFIX.len()] == b'0' {
                c == 's'
            } else {
                in_alphabet(c)
            }
        }
        _ => in_alphabet(c),
    }
}

/// Whether `c` is a bech32 character, which is every lower-case letter
/// but `b`, `i` and `o` and every digit but `1`.
fn in_alphabet(c: char) -> bool {
    const CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    CHARSET.contains(c)
}

// ---------------------------------------------------------------------
// The split this device writes (`docs/PLANNING.md` §16.109 rules 4 and 5)
// ---------------------------------------------------------------------

/// Strings one plan can write: BIP 93's own bound on a set.
pub const MAX_STRINGS_MADE: usize = codex32::MAX_SHARES as usize;

/// Random shares one split can need: one fewer than the largest
/// threshold, which is what BIP 93's "Generating Shares" asks for.
pub const MAX_RANDOM: usize = MAX_THRESHOLD as usize - 1;

/// Bytes one run of a source's own screens makes at most, which is the
/// largest strength Create a key offers.
const GATHER_MAX: usize = 32;

/// Where the plan is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// "Split into shares?"
    Split,
    /// "How many shares?"
    Count,
    /// "How many must be present?"
    Threshold,
    /// "Random shares from?", on the Backup path only.
    Source,
    /// The chosen source's own screens, once per run.
    Gather,
    /// One string on a Secret screen.
    Shown,
    /// Typing that string back.
    TypeBack,
    /// The caution behind "Skip".
    TypeSkip,
    /// "Strings made", on the Backup path only.
    Result,
}

/// What a tap on the plan asks the caller to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Nothing outside the plan changed.
    Stay,
    /// Open the entropy screens for the run now due.
    Gather,
    /// Every random byte is in: make the strings.
    Split,
    /// The strings have all been shown; the flow goes on.
    Done,
}

/// Shares a split has before anything is chosen, and how many of them
/// must be present.
const DEFAULT_SHARES: u8 = 3;
const DEFAULT_THRESHOLD: u8 = 2;

/// The plan, the randomness and the strings.
pub struct Codex32Plan {
    step: Step,
    /// The master seed being written.
    seed: [u8; MAX_SEED_BYTES],
    seed_len: u8,
    /// Whether the checked row of "Split into shares?" is "Yes".
    split: bool,
    shares: u8,
    threshold: u8,
    /// Where the random shares come from.
    source: Source,
    /// Whether this flow asks for the source: Backup does, Create does
    /// not, since the key was just made from one (§16.92).
    ask_source: bool,
    /// Whether the flow ends in a Result, which Backup's does.
    result: bool,
    /// Whether the key being backed up has words, which the Result
    /// states as a fact of the backup.
    words: bool,
    /// The bytes gathered for the random shares, one share's after
    /// another's.
    randoms: [u8; MAX_RANDOM * MAX_SEED_BYTES],
    /// Bytes gathered so far, and the runs they took.
    got: u16,
    runs: u8,
    /// Runs this plan needs, and the bytes they come to.
    need_runs: u8,
    need: u16,
    /// The strings, in index order: the secret alone, or the `n` shares.
    strings: [Codex32; MAX_STRINGS_MADE],
    total: u8,
    /// The string on screen, 0-based.
    show: u8,
    /// The string being typed back.
    entry: Codex32Entry,
    /// Whether the last ✓ typed something other than the string shown.
    mismatch: bool,
    /// Whether every string so far was typed back.
    verified: bool,
    identifier: [u8; 4],
}

impl Zeroize for Codex32Plan {
    fn zeroize(&mut self) {
        self.seed.zeroize();
        self.seed_len = 0;
        self.split = false;
        self.shares = DEFAULT_SHARES;
        self.threshold = DEFAULT_THRESHOLD;
        self.words = false;
        self.randoms.zeroize();
        self.got = 0;
        self.runs = 0;
        self.need_runs = 0;
        self.need = 0;
        for string in &mut self.strings {
            string.zeroize();
        }
        self.total = 0;
        self.show = 0;
        self.entry.zeroize();
        self.mismatch = false;
        self.verified = false;
        self.identifier = [0; 4];
    }
}

impl Drop for Codex32Plan {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Codex32Plan {}

impl Default for Codex32Plan {
    fn default() -> Self {
        Self::new()
    }
}

impl Codex32Plan {
    /// An empty plan.
    pub fn new() -> Self {
        Codex32Plan {
            step: Step::Split,
            seed: [0; MAX_SEED_BYTES],
            seed_len: 0,
            split: false,
            shares: DEFAULT_SHARES,
            threshold: DEFAULT_THRESHOLD,
            source: Source::Dice,
            ask_source: false,
            result: false,
            words: false,
            randoms: [0; MAX_RANDOM * MAX_SEED_BYTES],
            got: 0,
            runs: 0,
            need_runs: 0,
            need: 0,
            strings: core::array::from_fn(|_| Codex32::default()),
            total: 0,
            show: 0,
            entry: Codex32Entry::new(),
            mismatch: false,
            verified: false,
            identifier: [0; 4],
        }
    }

    /// Begins the plan for Add a key › Create Codex32 shares: the random
    /// shares come from the source the key was just made from.
    pub fn start_create(&mut self, seed: &[u8], source: Source, identifier: [u8; 4]) -> bool {
        if !self.load(seed, identifier) {
            return false;
        }
        self.source = source;
        self.ask_source = false;
        self.result = false;
        self.words = false;
        true
    }

    /// Begins the plan for Backup › Codex32, which asks for the source
    /// itself and ends in a Result.
    pub fn start_backup(
        &mut self,
        seed: &[u8],
        source: Source,
        identifier: [u8; 4],
        words: bool,
    ) -> bool {
        if !self.load(seed, identifier) {
            return false;
        }
        self.source = source;
        self.ask_source = true;
        self.result = true;
        self.words = words;
        true
    }

    fn load(&mut self, seed: &[u8], identifier: [u8; 4]) -> bool {
        self.zeroize();
        if !SEED_LENGTHS.contains(&seed.len()) {
            return false;
        }
        self.seed[..seed.len()].copy_from_slice(seed);
        self.seed_len = seed.len() as u8;
        self.identifier = identifier;
        self.step = Step::Split;
        true
    }

    /// Whether a plan is loaded with a seed and running.
    pub fn is_running(&self) -> bool {
        self.seed_len > 0
    }

    /// The step.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Whether the checked row of "Split into shares?" is "Yes".
    pub fn splits(&self) -> bool {
        self.split
    }

    /// Shares the split has, and how many must be present. `(0, 0)`
    /// where the seed is written as one string.
    pub fn plan(&self) -> (usize, usize) {
        if !self.split {
            return (0, 0);
        }
        (usize::from(self.threshold), usize::from(self.shares))
    }

    /// Shares the Choice has checked.
    pub fn shares(&self) -> usize {
        usize::from(self.shares)
    }

    /// Shares of them that must be present, as the Choice has it.
    pub fn threshold(&self) -> usize {
        usize::from(self.threshold)
    }

    /// Where the random shares come from.
    pub fn source(&self) -> Source {
        self.source
    }

    /// Whether this flow asks which source the random shares come from.
    pub fn asks_source(&self) -> bool {
        self.ask_source
    }

    /// Whether this flow ends in a Result of its own.
    pub fn has_result(&self) -> bool {
        self.result
    }

    /// Whether the key this backup is of has words, which the Result
    /// states.
    pub fn has_words(&self) -> bool {
        self.words
    }

    /// The seed's length in bytes.
    pub fn seed_len(&self) -> usize {
        usize::from(self.seed_len)
    }

    /// Strings this plan made.
    pub fn total(&self) -> usize {
        usize::from(self.total)
    }

    /// The string on screen, 0-based.
    pub fn show(&self) -> usize {
        usize::from(self.show)
    }

    /// The string on screen, as the characters a person copies.
    pub fn string(&self) -> Option<codex32::Codex32Ascii> {
        (self.show < self.total).then(|| self.strings[self.show()].encode())
    }

    /// The entry the string is typed back into.
    pub fn entry(&self) -> &Codex32Entry {
        &self.entry
    }

    /// Whether what was typed back is not the string that was shown.
    pub fn mismatch(&self) -> bool {
        self.mismatch
    }

    /// Whether every string was typed back.
    pub fn verified(&self) -> bool {
        self.verified
    }

    /// The identifier every string of this set carries.
    pub fn identifier(&self) -> [u8; 4] {
        self.identifier
    }

    /// Runs of the source still to come, and how many are in.
    pub fn randoms(&self) -> (usize, usize) {
        (usize::from(self.runs), usize::from(self.need_runs))
    }

    /// What the run now due is called: the 1-based run and how many
    /// there are.
    pub fn gather_label(&self) -> (u8, u8) {
        (self.runs.saturating_add(1), self.need_runs)
    }

    /// The bytes the run now due makes, which is what the source's own
    /// screens are opened at.
    pub fn gather_strength(&self) -> Strength {
        Strength::for_bytes(self.run_bytes()).unwrap_or(Strength::Bits256)
    }

    /// Bytes the run now due makes: the rest of the random share being
    /// gathered, up to what one run of a source can carry.
    fn run_bytes(&self) -> usize {
        let len = self.seed_len();
        if len == 0 {
            return GATHER_MAX;
        }
        let into = usize::from(self.got) % len;
        (len - into).min(GATHER_MAX)
    }

    // ----- taps -----

    /// Handles a tap.
    pub fn tap(&mut self, id: Id) -> Next {
        match self.step {
            Step::Split => {
                if id == ids::CODEX32_SPLIT_NO {
                    self.split = false;
                } else if id == ids::CODEX32_SPLIT_YES {
                    self.split = true;
                } else if id == ids::CODEX32_SPLIT_CONTINUE {
                    if !self.split {
                        return self.begin_random();
                    }
                    self.step = Step::Count;
                }
                Next::Stay
            }
            Step::Count => {
                if let Some(i) = ids::index_in(id, ids::CODEX32_COUNT_BASE, MAX_STRINGS_MADE) {
                    let n = i as u8 + 1;
                    if n >= 2 {
                        self.shares = n;
                        self.threshold = self.threshold.min(n).min(MAX_THRESHOLD);
                    }
                } else if id == ids::CODEX32_COUNT_CONTINUE {
                    self.step = Step::Threshold;
                }
                Next::Stay
            }
            Step::Threshold => {
                if let Some(i) = ids::index_in(id, ids::CODEX32_THRESHOLD_BASE, MAX_STRINGS_MADE) {
                    let n = i as u8 + 1;
                    if (2..=MAX_THRESHOLD.min(self.shares)).contains(&n) {
                        self.threshold = n;
                    }
                } else if id == ids::CODEX32_THRESHOLD_CONTINUE {
                    return self.begin_random();
                }
                Next::Stay
            }
            Step::Source => {
                if let Some(i) = ids::index_in(
                    id,
                    ids::CODEX32_SOURCE_BASE,
                    crate::create::SOURCE_ROWS.len(),
                ) {
                    self.source = crate::create::SOURCE_ROWS[i];
                } else if id == ids::CODEX32_SOURCE_CONTINUE {
                    return self.gather_or_split();
                }
                Next::Stay
            }
            // The gatherer's screens are the Create wizard's own.
            Step::Gather => Next::Stay,
            Step::Shown => {
                if id == ids::CREATE_CONTINUE {
                    self.entry.clear_string();
                    self.mismatch = false;
                    self.step = Step::TypeBack;
                } else if id == ids::QUIZ_SKIP {
                    self.step = Step::TypeSkip;
                }
                Next::Stay
            }
            Step::TypeBack => Next::Stay,
            Step::TypeSkip => {
                if id == ids::QUIZ_SKIP_CANCEL {
                    self.step = Step::Shown;
                } else if id == ids::QUIZ_SKIP_CONFIRM {
                    self.verified = false;
                    return self.next_string();
                }
                Next::Stay
            }
            Step::Result => {
                if id == ids::QUIZ_DONE {
                    return Next::Done;
                }
                Next::Stay
            }
        }
    }

    /// The type-back keyboard.
    pub fn key(&mut self, input: osk_ui::widgets::keyboard::KeyInput) -> Next {
        use osk_ui::widgets::keyboard::KeyInput;
        if self.step != Step::TypeBack {
            return Next::Stay;
        }
        match input {
            KeyInput::Char(c) => {
                self.entry.type_char(c);
                self.mismatch = false;
                Next::Stay
            }
            KeyInput::Backspace => {
                self.entry.backspace();
                self.mismatch = false;
                Next::Stay
            }
            KeyInput::Done => {
                if !self.entry.accepts() {
                    return Next::Stay;
                }
                let same = self
                    .string()
                    .is_some_and(|shown| shown.as_str() == self.entry.typed());
                if !same {
                    // The field stays: a person who typed one character
                    // wrong fixes that character.
                    self.mismatch = true;
                    return Next::Stay;
                }
                self.mismatch = false;
                self.next_string()
            }
            KeyInput::Shift | KeyInput::Symbols => Next::Stay,
        }
    }

    /// The plan is settled: how many runs of the source it needs, and
    /// the first of them.
    fn begin_random(&mut self) -> Next {
        // A seed written as one string is fed no randomness, so there
        // is no source to ask about.
        if self.ask_source && self.split {
            self.step = Step::Source;
            return Next::Stay;
        }
        self.gather_or_split()
    }

    fn gather_or_split(&mut self) -> Next {
        let len = self.seed_len();
        let randoms = if self.split {
            usize::from(self.threshold.saturating_sub(1))
        } else {
            0
        };
        self.need = (randoms * len) as u16;
        self.need_runs = (randoms * len.div_ceil(GATHER_MAX)) as u8;
        self.got = 0;
        self.runs = 0;
        if self.need_runs == 0 {
            return Next::Split;
        }
        self.step = Step::Gather;
        Next::Gather
    }

    /// Keeps one gathered run. Returns whether another is due or the
    /// strings can be made.
    pub fn take_random(&mut self, bytes: &[u8]) -> Next {
        if bytes.len() != self.run_bytes() || self.runs >= self.need_runs {
            return Next::Stay;
        }
        let at = usize::from(self.got);
        self.randoms[at..at + bytes.len()].copy_from_slice(bytes);
        self.got += bytes.len() as u16;
        self.runs += 1;
        if self.runs < self.need_runs {
            self.step = Step::Gather;
            Next::Gather
        } else {
            Next::Split
        }
    }

    /// Writes the strings: the secret alone where the seed is not
    /// split, else the `n` shares of a `k`-of-`n` set.
    ///
    /// Every random character the codec writes comes from the bytes
    /// gathered on the screens, packed into 5-bit groups by BIP 93's own
    /// rule for a payload, and the codec is never handed more than they
    /// hold.
    pub fn split(&mut self) -> bool {
        let len = self.seed_len();
        if len == 0 {
            return false;
        }
        let threshold = if self.split { self.threshold } else { 0 };
        let Ok(secret) = Codex32::from_seed(threshold, self.identifier, &self.seed[..len]) else {
            self.zeroize();
            return false;
        };
        if !self.split {
            self.strings[0] = secret;
            self.total = 1;
            return self.made();
        }
        // BIP 93's "Generating Shares": k - 1 random shares, each a
        // payload of ceil(bitlength / 5) characters. The bytes gathered
        // are exactly one seed's worth per random share, and they are
        // written into that payload the way the secret's own is.
        let symbols = codex32::symbols_for(len);
        let randoms = usize::from(self.threshold) - 1;
        let expected = randoms * symbols;
        let mut fresh = [0u8; MAX_RANDOM * codex32::MAX_STRING_LEN];
        let mut written = 0usize;
        for r in 0..randoms {
            let at = r * len;
            written += codex32::seed_symbols(
                &self.randoms[at..at + len],
                &mut fresh[written..written + symbols],
            );
        }
        debug_assert_eq!(usize::from(self.got), randoms * len, "the bytes gathered");
        debug_assert_eq!(written, expected, "the characters the split is handed");
        if usize::from(self.got) != randoms * len || written != expected {
            fresh.zeroize();
            self.zeroize();
            return false;
        }
        let made = secret.split(self.threshold, self.shares, &fresh[..expected]);
        fresh.zeroize();
        let Ok(set) = made else {
            self.zeroize();
            return false;
        };
        let n = set.len().min(MAX_STRINGS_MADE);
        for (slot, string) in self.strings.iter_mut().zip(set) {
            *slot = string;
        }
        self.total = n as u8;
        self.made()
    }

    /// The strings are written: the first of them, and the randomness
    /// forgotten.
    fn made(&mut self) -> bool {
        self.show = 0;
        self.entry.zeroize();
        self.mismatch = false;
        self.verified = true;
        self.randoms.zeroize();
        self.got = 0;
        self.runs = 0;
        self.step = Step::Shown;
        true
    }

    /// This string is done, whether typed back or skipped: the next
    /// string, or the end.
    fn next_string(&mut self) -> Next {
        self.entry.zeroize();
        self.mismatch = false;
        if usize::from(self.show) + 1 < self.total() {
            self.show += 1;
            self.step = Step::Shown;
            return Next::Stay;
        }
        if self.result {
            self.step = Step::Result;
            return Next::Stay;
        }
        // Back from Confirm comes to the last string, so the plan stays
        // where it was rather than running past its own end.
        self.step = Step::Shown;
        Next::Done
    }

    /// Goes one step back. `false` means leaving the plan, which the
    /// flow that opened it answers for.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            Step::Split => return false,
            Step::Count => Step::Split,
            Step::Threshold => Step::Count,
            // Leaving the source or the gathering drops every byte
            // gathered so far, as a SLIP-39 split's does.
            Step::Gather if self.ask_source => {
                self.drop_randoms();
                Step::Source
            }
            Step::Source | Step::Gather => {
                self.drop_randoms();
                self.plan_step()
            }
            // A string goes back to the one before it; the first goes
            // back to the plan, and the strings are forgotten.
            Step::Shown if self.show > 0 => {
                self.show -= 1;
                self.entry.zeroize();
                self.mismatch = false;
                Step::Shown
            }
            Step::Shown => {
                for string in &mut self.strings {
                    string.zeroize();
                }
                self.total = 0;
                self.show = 0;
                self.entry.zeroize();
                self.mismatch = false;
                if self.ask_source && self.split {
                    Step::Source
                } else {
                    self.plan_step()
                }
            }
            Step::TypeBack => {
                self.entry.zeroize();
                self.mismatch = false;
                Step::Shown
            }
            Step::TypeSkip => Step::Shown,
            Step::Result => return false,
        };
        true
    }

    fn drop_randoms(&mut self) {
        self.randoms.zeroize();
        self.got = 0;
        self.runs = 0;
    }

    /// The last Choice the plan asked: the threshold of a split, or the
    /// split question itself where the seed is written as one string.
    fn plan_step(&self) -> Step {
        if self.split {
            Step::Threshold
        } else {
            Step::Split
        }
    }
}
