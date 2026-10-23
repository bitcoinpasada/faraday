//! The codex32 work set: a fresh set being made by hand, held where
//! every secret is (`docs/CODEX32.md` §13.1, `docs/KEYS.md` §3).
//!
//! The person's characters are handed in one by one as each is complete
//! ([`WorkSet::push`]); the rows of each initial share's checksum
//! worksheet, the checksums, the derived shares, the secret and the seed
//! are computed here and kept in step with the characters at every
//! change, so drawing reads them and computes nothing. Like a FROST
//! share or a BIP-85 child it is a new secret held inside this crate: it
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

use faraday_bip::codex32::{
    ChecksumWorksheet, Error, FreshSet, Scheme, TranslationWorksheet, fresh_set,
};
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

use faraday_bip::keys::Fingerprint;
use faraday_intake::wallet::Refusal;

use crate::{Held, Keys, Reveal};

/// A fresh codex32 set in the making: its scheme, the characters
/// entered for each initial share, those shares' checksum worksheets and,
/// once every character is in, the set.
pub struct WorkSet {
    scheme: Scheme,
    /// The data symbols entered, one list per initial share.
    entered: Vec<Zeroizing<Vec<u8>>>,
    /// One checksum worksheet per initial share, header first.
    sheets: Vec<ChecksumWorksheet>,
    /// The translation worksheets filling as the characters arrive, one
    /// per target of [`Self::targets`], in that order (none at threshold 0).
    live: Vec<TranslationWorksheet>,
    /// The made set, once every initial share is complete.
    made: Option<FreshSet>,
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
            entered,
            sheets,
            live,
            made: None,
        })
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
    /// order: each derived share, then `s` (the secret). Public. None at
    /// threshold 0. Cheap.
    pub fn targets(&self) -> Vec<u8> {
        Self::targets_of(&self.scheme)
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
        self.made = Some(fresh_set(c, &self.scheme, &data)?);
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
        TranslationWorksheet::of(c, &set.strings()[..k], target)
            .ok()
            .map(Held::of)
    }

    /// The strings to write, in index order (the initial shares, then
    /// the derived ones; the secret alone at threshold 0), as paper
    /// forms. `None` until the set is made or for another reason.
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
        self.add_seed(c, &seed, label)
    }
}
