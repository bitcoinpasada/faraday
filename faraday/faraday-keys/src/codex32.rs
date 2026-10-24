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
    ChecksumWorksheet, Codex32, Error, FreshSet, Scheme, TranslationWorksheet, fresh_set,
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
        let fp = self.add_seed(c, &seed, label)?;
        // The set made is the key's, its plain facts kept with it
        // (`docs/CODEX32.md` §9.4): every string made, none placed yet.
        let scheme = work.scheme();
        let strings = scheme
            .initial_indices()
            .iter()
            .chain(scheme.derived_indices())
            .map(|&i| (i, State::Made))
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
