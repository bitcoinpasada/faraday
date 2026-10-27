//! The codex32 work set: a fresh set being made by hand, held where
//! every secret is (`docs/CODEX32.md` §13.1, `docs/KEYS.md` §3).
//!
//! The person's characters are handed in one by one as each is complete
//! ([`WorkSet::push`]); the rows of each initial share's checksum
//! worksheet, the checksums, the derived shares, the secret and the seed
//! are computed here and kept in step with the characters at every
//! change, so drawing reads them and computes nothing. Like a BIP-85
//! child it is a new secret held inside this crate: it
//! leaves only as a paper form (`Reveal::PaperForm`), or as a key through
//! [`Keys::add_codex32`], and everything it holds is wiped when it is
//! dropped.
//!
//! **Kept in step.** `entered[s]` is how many data characters initial
//! share `s` has. Its worksheet holds the header and exactly those
//! characters, and, once all of them are in, the checksum. Each live
//! translation worksheet (one per derived share and one towards `s`,
//! at a threshold of 2 or more) holds, for input `s`, the same header,
//! characters and checksum as that worksheet. The made set exists
//! exactly when every initial share is complete. A push or a pop updates
//! them all together, and nothing else changes them.
//!
//! A set made of `k` strings typed in ([`WorkSet::given`]: Make the rest
//! and Add a share, `docs/CODEX32.md` §8.1, §8.2) is made whole at once
//! and never changes: its inputs are the strings given, every worksheet
//! is filled from them, and a push or a pop is refused.
//! [`StringCheck`] is one string's checksum worksheet for Check a string
//! (§8.3).

use faraday_bip::codex32::{
    ChecksumWorksheet, Codex32, Error, FreshSet, INDEX_ORDER, Length, Scheme, TranslationWorksheet,
    fresh_set,
};
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;
use faraday_vault::codex32::{SetFacts, State};
use zeroize::Zeroizing;

use faraday_bip::keys::Fingerprint;
use faraday_intake::wallet::Refusal;

use crate::{Held, Keys, Reveal};

/// A fresh codex32 set in the making: its scheme, the characters
/// entered for each initial share, those shares' checksum worksheets and,
/// once every character is in, the set.
pub struct WorkSet {
    scheme: Scheme,
    /// The share indices of the inputs: the initial shares of a fresh
    /// set, or the strings a set made of strings was given.
    inputs: Vec<u8>,
    /// The share indices the translation worksheets make, in order.
    targets: Vec<u8>,
    /// The data symbols entered, one list per input.
    entered: Vec<Zeroizing<Vec<u8>>>,
    /// One checksum worksheet per input, header first.
    sheets: Vec<ChecksumWorksheet>,
    /// The translation worksheets filling as the characters arrive, one
    /// per target of [`Self::targets`], in that order (none at threshold 0).
    live: Vec<TranslationWorksheet>,
    /// The made set, once every input is complete.
    made: Option<Made>,
    /// The strings a set made of strings was given (`docs/CODEX32.md`
    /// §8.1, §8.2); none for a fresh set.
    given: Vec<Codex32>,
}

/// A made set: the strings to write, in index order, and the secret.
enum Made {
    /// A fresh set: every string, the initial shares first.
    Fresh(FreshSet),
    /// A set made of strings given: the strings made from them, and the
    /// secret they carry.
    Given {
        /// The strings made, in the order of the targets.
        strings: Vec<Codex32>,
        /// The secret.
        secret: Codex32,
    },
}

impl Made {
    /// The strings to write.
    fn strings(&self) -> &[Codex32] {
        match self {
            Made::Fresh(f) => f.strings(),
            Made::Given { strings, .. } => strings,
        }
    }

    /// The secret the set carries.
    fn secret(&self) -> &Codex32 {
        match self {
            Made::Fresh(f) => f.secret(),
            Made::Given { secret, .. } => secret,
        }
    }
}

/// What a set made of strings makes (`docs/CODEX32.md` §8.1, §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Making {
    /// Make the rest: the set's strings up to `n` in all, at the first
    /// indices in BIP 93's order that none of the strings given has, and
    /// the secret's sheet.
    Rest(u8),
    /// As [`Making::Rest`], the strings to write being the whole set, the
    /// strings given among them, in index order: Practice's sample in place
    /// of a set entered (`docs/CODEX32.md` §12).
    All(u8),
    /// Add a share: the one string at this index.
    Add(u8),
}

/// A string read again from its own text, through the one door in.
fn again(s: &Codex32) -> Result<Codex32, Error> {
    Codex32::parse(s.encode().as_str())
}

impl WorkSet {
    /// Costs the header's columns of each initial share's worksheet and
    /// of each live translation worksheet. A work set for `scheme`,
    /// nothing entered.
    pub fn new(c: &Compute, scheme: Scheme) -> Result<WorkSet, Error> {
        let mut sheets = Vec::new();
        let mut entered = Vec::new();
        for &index in scheme.initial_indices() {
            let mut sheet = ChecksumWorksheet::new(scheme.length().chars())?;
            for symbol in scheme.header(index)? {
                sheet.push(c, symbol)?;
            }
            sheets.push(sheet);
            entered.push(Zeroizing::new(Vec::new()));
        }
        let mut live = Vec::new();
        let initial = scheme.initial_indices();
        for target in Self::targets_of(&scheme) {
            let mut sheet = TranslationWorksheet::new(scheme.length().chars(), initial, target)?;
            for (i, &index) in initial.iter().enumerate() {
                for symbol in scheme.header(index)? {
                    sheet.push(c, i, symbol)?;
                }
            }
            live.push(sheet);
        }
        Ok(WorkSet {
            scheme,
            inputs: initial.to_vec(),
            targets: Self::targets_of(&scheme),
            entered,
            sheets,
            live,
            made: None,
            given: Vec::new(),
        })
    }

    /// Costs a checksum worksheet of each string given, a translation
    /// worksheet (an interpolation of `k` strings) towards each target,
    /// and an interpolation of `k` strings for each string made and for
    /// the secret. The set exactly `k` strings of one set make
    /// (`docs/CODEX32.md` §8.1 Make the rest, §8.2 Add a share), the
    /// secret counting as one: every worksheet filled whole, the strings
    /// made at the indices `making` names. Refused where the strings are
    /// not `k` of one set at a length a fresh set has, or an index asked
    /// for is the secret's, one given, or not a share index.
    pub fn given(c: &Compute, strings: &[Codex32], making: Making) -> Result<WorkSet, Error> {
        let first = strings.first().ok_or(Error::WrongShareCount)?;
        let k = first.threshold();
        if k < 2 || strings.len() != usize::from(k) {
            return Err(Error::WrongShareCount);
        }
        let length = Length::of_chars(first.encode().as_str().len()).ok_or(Error::BadLength)?;
        let inputs: Vec<u8> = strings.iter().map(Codex32::share_index).collect();
        let (made_at, n): (Vec<u8>, u8) = match making {
            Making::Rest(n) | Making::All(n) => {
                let want = usize::from(n).saturating_sub(usize::from(k));
                let at: Vec<u8> = INDEX_ORDER
                    .iter()
                    .filter(|i| !inputs.contains(i))
                    .take(want)
                    .copied()
                    .collect();
                if want == 0 || at.len() != want {
                    return Err(Error::BadSplit);
                }
                (at, n)
            }
            Making::Add(i) => {
                let i = i.to_ascii_lowercase();
                if inputs.contains(&i) || !INDEX_ORDER.contains(&i) {
                    return Err(Error::BadShareIndex);
                }
                (vec![i], (k + 1).min(faraday_bip::codex32::MAX_SHARES))
            }
        };
        let scheme = Scheme::new(k, n, first.identifier(), length)?;
        let given = strings.iter().map(again).collect::<Result<Vec<_>, _>>()?;
        let secret = match given.iter().find(|s| s.is_secret()) {
            Some(s) => again(s)?,
            None => Codex32::recover(&given)?,
        };
        let mut made = made_at
            .iter()
            .map(|&i| Codex32::derive_share(&given, i))
            .collect::<Result<Vec<_>, _>>()?;
        if matches!(making, Making::All(_)) {
            for s in &given {
                made.push(again(s)?);
            }
            made.sort_by_key(|s| INDEX_ORDER.iter().position(|&i| i == s.share_index()));
        }
        let mut targets = made_at;
        if !matches!(making, Making::Add(_)) && !inputs.contains(&b's') {
            targets.push(b's');
        }
        let live = targets
            .iter()
            .map(|&t| TranslationWorksheet::of(c, &given, t))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WorkSet {
            scheme,
            sheets: given.iter().map(|s| ChecksumWorksheet::of(c, s)).collect(),
            entered: given
                .iter()
                .map(|s| Zeroizing::new(s.payload().to_vec()))
                .collect(),
            inputs,
            targets,
            live,
            made: Some(Made::Given {
                strings: made,
                secret,
            }),
            given,
        })
    }

    /// Whether the set was made of strings given rather than entered.
    /// Public.
    pub fn is_given(&self) -> bool {
        !self.given.is_empty()
    }

    /// The share indices of the inputs, in the order their worksheets
    /// are kept: the initial shares of a fresh set (`s` alone at
    /// threshold 0), or the strings given. Public. Cheap.
    pub fn inputs(&self) -> &[u8] {
        &self.inputs
    }

    /// The share indices the live translation worksheets make: each
    /// derived share in index order, then `s`; none at threshold 0.
    fn targets_of(scheme: &Scheme) -> Vec<u8> {
        if scheme.threshold() == 0 {
            return Vec::new();
        }
        let mut out = scheme.derived_indices().to_vec();
        out.push(b's');
        out
    }

    /// The share indices of the live translation worksheets, in their
    /// order: each share made, then `s` (the secret) where its sheet is
    /// kept. Public. None at threshold 0. Cheap.
    pub fn targets(&self) -> Vec<u8> {
        self.targets.clone()
    }

    /// The share indices of every string of the set known here: the
    /// inputs, then the strings made, each once. Public. Cheap.
    pub fn indices(&self) -> Vec<u8> {
        let mut out = self.inputs.clone();
        for s in self.made.iter().flat_map(|m| m.strings()) {
            let i = s.share_index();
            if !out.contains(&i) {
                out.push(i);
            }
        }
        out
    }

    /// The scheme. Public.
    pub fn scheme(&self) -> &Scheme {
        &self.scheme
    }

    /// How many data characters initial share `share` (its place among
    /// the initial shares) has. Public (a count). Zero for no such share.
    pub fn entered(&self, share: usize) -> usize {
        self.entered.get(share).map_or(0, |e| e.len())
    }

    /// Whether every initial share has all its characters and the set is
    /// made. A lookup.
    pub fn is_complete(&self) -> bool {
        self.made.is_some()
    }

    /// Costs a worksheet column and that column of each live translation
    /// worksheet, and at the last character the checksum's columns (in
    /// every sheet) and, once every initial share is complete, the
    /// interpolation of the whole set. Hands initial share `share` its
    /// next character, a symbol `0..32`. Refused past the last one.
    pub fn push(&mut self, c: &Compute, share: usize, symbol: u8) -> Result<(), Error> {
        if self.is_given() {
            return Err(Error::BadLength);
        }
        let data_len = self.scheme.length().data_len();
        let (Some(have), Some(sheet)) = (self.entered.get_mut(share), self.sheets.get_mut(share))
        else {
            return Err(Error::WrongShareCount);
        };
        if have.len() >= data_len {
            return Err(Error::BadLength);
        }
        sheet.push(c, symbol)?;
        have.push(symbol);
        for live in &mut self.live {
            live.push(c, share, symbol)?;
        }
        if have.len() == data_len {
            sheet.finish(c)?;
            let chars = sheet.chars();
            let first = chars - sheet.checksum_len() + 1;
            for b in first..=chars {
                let v = sheet.symbol(b).ok_or(Error::BadLength)?;
                for live in &mut self.live {
                    live.push(c, share, v)?;
                }
            }
            self.make(c)?;
        }
        Ok(())
    }

    /// Takes back the last character of initial share `share`, and the
    /// checksum and the made set where they depended on it. Cheap.
    pub fn pop(&mut self, share: usize) {
        if self.is_given() {
            return;
        }
        let data_len = self.scheme.length().data_len();
        let (Some(have), Some(sheet)) = (self.entered.get_mut(share), self.sheets.get_mut(share))
        else {
            return;
        };
        if have.is_empty() {
            return;
        }
        let mut back = 1;
        if have.len() == data_len {
            for _ in 0..sheet.checksum_len() {
                sheet.pop();
            }
            back += sheet.checksum_len();
        }
        sheet.pop();
        have.pop();
        for live in &mut self.live {
            for _ in 0..back {
                live.pop(share);
            }
        }
        self.made = None;
    }

    /// Costs `fresh_set`. Makes the set once every initial share is
    /// complete.
    fn make(&mut self, c: &Compute) -> Result<(), Error> {
        let data_len = self.scheme.length().data_len();
        if self.entered.iter().any(|e| e.len() != data_len) {
            return Ok(());
        }
        let data: Vec<&[u8]> = self.entered.iter().map(|e| e.as_slice()).collect();
        self.made = Some(Made::Fresh(fresh_set(c, &self.scheme, &data)?));
        Ok(())
    }

    /// The checksum worksheet of initial share `share`, lent to drawing
    /// as a paper form. `None` for no such share or any reason but
    /// `Reveal::PaperForm`.
    pub fn worksheet(&self, share: usize, why: Reveal) -> Option<&ChecksumWorksheet> {
        (why == Reveal::PaperForm)
            .then(|| self.sheets.get(share))
            .flatten()
    }

    /// The live translation worksheet towards the `n`-th of
    /// [`Self::targets`], filled as far as the initial shares are
    /// entered, lent to drawing as a paper form. A lookup. `None` for no
    /// such sheet or any reason but `Reveal::PaperForm`.
    pub fn live_translation(&self, n: usize, why: Reveal) -> Option<&TranslationWorksheet> {
        (why == Reveal::PaperForm)
            .then(|| self.live.get(n))
            .flatten()
    }

    /// Costs a translation worksheet of `k` strings. The worksheet that
    /// makes the string at share index `target` (or `s`) from the initial
    /// shares, filled whole, as a paper form. `None` until the set is
    /// made, at threshold 0, or for another reason.
    pub fn translation(
        &self,
        c: &Compute,
        target: u8,
        why: Reveal,
    ) -> Option<Held<TranslationWorksheet>> {
        if why != Reveal::PaperForm || self.scheme.threshold() == 0 {
            return None;
        }
        let set = self.made.as_ref()?;
        let k = usize::from(self.scheme.threshold());
        let inputs = match set {
            Made::Fresh(f) => f.strings().get(..k)?,
            Made::Given { .. } => &self.given,
        };
        TranslationWorksheet::of(c, inputs, target)
            .ok()
            .map(Held::of)
    }

    /// The strings to write, in index order (the initial shares, then
    /// the derived ones; the secret alone at threshold 0; of a set made
    /// of strings, only the strings made), as paper forms. `None` until the set is made or for another reason.
    pub fn strings(&self, why: Reveal) -> Option<Held<Vec<Zeroizing<String>>>> {
        if why != Reveal::PaperForm {
            return None;
        }
        let set = self.made.as_ref()?;
        Some(Held::of(
            set.strings()
                .iter()
                .map(|s| Zeroizing::new(s.encode().as_str().to_string()))
                .collect(),
        ))
    }

    /// The master seed the set carries, once made.
    fn seed(&self) -> Option<Zeroizing<Vec<u8>>> {
        let seed: Secret<_> = self.made.as_ref()?.secret().to_seed().ok()?;
        Some(Zeroizing::new(seed.expose().bytes().to_vec()))
    }

    /// Costs an HMAC-SHA512 and an elliptic-curve multiplication. The
    /// fingerprint of the key the set makes, for the identifier's check
    /// before the key is loaded. Public. `None` until the set is made.
    pub fn fingerprint(&self, c: &Compute) -> Option<Fingerprint> {
        crate::identity::fingerprint_of_seed(c, &self.seed()?)
    }
}

/// The bech32 alphabet in symbol order (BIP 173).
const BECH32: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// One string's checksum worksheet in the verification form, for Check a
/// string (`docs/CODEX32.md` §8.3): every box as typed, its checksum
/// holding or not, the final row worked out from them. Lent to drawing
/// as a paper form; wiped on drop with the worksheet's own cells.
pub struct StringCheck {
    sheet: ChecksumWorksheet,
}

impl StringCheck {
    /// Costs a checksum worksheet of one string. The worksheet of `text`,
    /// `ms1` and then bech32 characters of one case at a length a codex32
    /// string has, whether its checksum holds or not. `None` for anything
    /// else.
    pub fn of(c: &Compute, text: &str) -> Option<StringCheck> {
        let rest = text
            .strip_prefix("ms1")
            .or_else(|| text.strip_prefix("MS1"))?;
        let lower = rest.bytes().all(|b| !b.is_ascii_uppercase());
        let upper = rest.bytes().all(|b| !b.is_ascii_lowercase());
        if !(lower || upper) {
            return None;
        }
        let mut sheet = ChecksumWorksheet::new(rest.len() + 3).ok()?;
        for b in rest.bytes() {
            let v = BECH32.iter().position(|&x| x == b.to_ascii_lowercase())?;
            sheet.push(c, v as u8).ok()?;
        }
        Some(StringCheck { sheet })
    }

    /// The worksheet, as a paper form. `None` for any reason but
    /// `Reveal::PaperForm`.
    pub fn worksheet(&self, why: Reveal) -> Option<&ChecksumWorksheet> {
        (why == Reveal::PaperForm).then_some(&self.sheet)
    }
}

impl Keys {
    /// Costs what `add_seed` costs. Loads the key the finished set
    /// `work` carries, its seed bytes kept as for any key loaded from a
    /// master seed. A refusal while the set is not complete.
    pub fn add_codex32(
        &mut self,
        c: &Compute,
        work: &WorkSet,
        label: &str,
    ) -> Result<faraday_bip::keys::Fingerprint, Refusal> {
        let seed = work.seed().ok_or_else(|| Refusal::Words(String::new()))?;
        let fp = self.add_seed(c, &seed, label)?;
        // The set made is the key's, its plain facts kept with it
        // (`docs/CODEX32.md` §9.4): every string made or given, none
        // placed yet.
        let scheme = work.scheme();
        let strings = work
            .indices()
            .into_iter()
            .map(|i| (i, State::Made))
            .collect();
        self.set_codex32(
            self.len() - 1,
            Some(SetFacts {
                threshold: scheme.threshold(),
                identifier: scheme.identifier(),
                length: u8::try_from(scheme.length().chars()).unwrap_or(0),
                unknown: 0,
                strings,
            }),
        );
        // A set of strings typed in keeps them and those made, so they go
        // to a stick or a vault untyped (§9.5); the secret is never kept.
        if work.is_given() {
            let texts: Vec<Zeroizing<String>> = work
                .given
                .iter()
                .chain(work.made.iter().flat_map(Made::strings))
                .map(|s| Zeroizing::new(s.encode().as_str().to_string()))
                .collect();
            let refs: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();
            self.keep_codex32(self.len() - 1, &refs);
        }
        Ok(fp)
    }

    /// Costs a parse of the words and a PBKDF2 for a key with words. The
    /// seed key `i`'s codex32 strings carry: the 64-byte BIP-39 seed of a
    /// key with words (its passphrase applied), or the master seed a key
    /// was loaded from.
    fn codex32_seed_of(&self, c: &Compute, i: usize) -> Option<Zeroizing<Vec<u8>>> {
        let s = self.secrets(i)?;
        match (s.words, s.seed) {
            (Some(words), _) => {
                let m = faraday_bip::bip39::Mnemonic::parse(c, s.language, words).ok()?;
                let seed = crate::identity::seed_of(c, &m, s.passphrase.unwrap_or(""))?;
                Some(Zeroizing::new(seed.expose().to_vec()))
            }
            (None, Some(bytes)) => Some(Zeroizing::new(bytes.to_vec())),
            (None, None) => None,
        }
    }

    /// Costs an interpolation of the strings given (BIP 93's recovery), an
    /// HMAC-SHA512 and an elliptic-curve multiplication. Whether `strings`
    /// rebuild key `i` (`docs/CODEX32.md` §9.4, Check my copy): with the
    /// secret among them, or `k` shares of one set, the seed they carry
    /// makes key `i`'s fingerprint. `None` while they cannot rebuild a
    /// seed (fewer than `k`, or not of one set).
    pub fn codex32_rebuilds(&self, c: &Compute, i: usize, strings: &[Codex32]) -> Option<bool> {
        let fp = self.info(i)?.fingerprint;
        let k = usize::from(strings.first()?.threshold()).max(1);
        let parts: &[Codex32] = match strings.iter().position(Codex32::is_secret) {
            Some(s) => &strings[s..=s],
            None => strings.get(..k)?,
        };
        let seed = crate::operations::codex32_seed(parts).ok()?;
        Some(crate::identity::fingerprint_of_seed(c, &seed) == Some(fp))
    }

    /// Costs an interpolation of `k` strings (BIP 93's derivation) and a
    /// check that they rebuild key `i`. The string at share index `index`
    /// of the set `strings` belong to (`docs/CODEX32.md` §9.7, Add a share
    /// and Replace a lost share), from `k` of its strings, the secret
    /// counting as one, which must rebuild key `i`. The new string is a
    /// paper form. Refused where the strings are fewer than `k`, are not
    /// of one set, or are not of key `i`'s seed.
    ///
    /// The key's seed never stands in for the secret: the secret's
    /// padding bits are not in the seed (BIP 93 ignores them when reading
    /// it), so a secret made from the seed may differ from the set's own
    /// and would derive a string of another set, one that mixed with the
    /// old strings rebuilds nothing.
    pub fn codex32_derive(
        &self,
        c: &Compute,
        i: usize,
        strings: &[Codex32],
        index: u8,
        why: Reveal,
    ) -> Result<Held<Zeroizing<String>>, crate::Paper> {
        use crate::Paper;
        if why != Reveal::PaperForm {
            return Err(Paper::Failed);
        }
        let k = usize::from(strings.first().ok_or(Paper::Failed)?.threshold());
        if k < 2 || self.codex32_rebuilds(c, i, strings) != Some(true) {
            return Err(Paper::Failed);
        }
        let refused = |e: Error| Paper::Refused(e.to_string());
        let have = strings
            .iter()
            .take(k)
            .map(|x| Codex32::parse(x.encode().as_str()).map_err(refused))
            .collect::<Result<Vec<Codex32>, Paper>>()?;
        let made = Codex32::derive_share(&have, index).map_err(refused)?;
        Ok(Held::of(Zeroizing::new(made.encode().as_str().to_string())))
    }

    /// Costs a parse of the words and a PBKDF2 for a key with words, and a
    /// split. Key `i`'s seed as a new codex32 set with identifier
    /// `identifier` (`docs/CODEX32.md` §9.7, Rotate the set): `n` strings,
    /// any `k` enough, the other shares spread from the fresh `draw`. The
    /// strings are paper forms, in index order.
    pub fn codex32_new_set(
        &self,
        c: &Compute,
        i: usize,
        (k, n): (u8, u8),
        identifier: [u8; 4],
        draw: &[u8; 32],
        why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, crate::Paper> {
        use crate::Paper;
        if why != Reveal::PaperForm || k < 2 {
            return Err(Paper::Failed);
        }
        let seed = self.codex32_seed_of(c, i).ok_or(Paper::NoWords)?;
        let refused = |e: Error| Paper::Refused(e.to_string());
        let need = usize::from(k - 1) * faraday_bip::codex32::symbols_for(seed.len());
        let fresh = crate::operations::spread(draw, b"codex32 rotation", need);
        let shares = Codex32::from_seed(k, identifier, &seed)
            .and_then(|secret| secret.split(k, n, &fresh))
            .map_err(refused)?;
        Ok(Held::of(
            shares
                .iter()
                .map(|s| Zeroizing::new(s.encode().as_str().to_string()))
                .collect(),
        ))
    }
}
