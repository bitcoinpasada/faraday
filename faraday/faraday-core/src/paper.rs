//! Other paper forms of a key held here, beside its words:
//! a Seed XOR split of its words
//! into parts that are each BIP-39 words, every part needed; and codex32
//! shares of its BIP-32 seed (BIP 93), any `k` of `n` enough. The
//! arithmetic is `faraday-entropy`'s and `faraday-bip`'s; the randomness is fresh
//! from the system. The parts are shown to copy by hand and never
//! written to a file; only the seed itself goes into a vault or, past
//! the secret sheet, a file (`crate::secrets`).

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::bip39::Mnemonic;
use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use faraday_bip::codex32::{self, Codex32};
use faraday_crypto::work::Compute;
use faraday_entropy::SeedXor;
use zeroize::Zeroizing;

use crate::Faraday;

/// A key's other paper form, on screen.
pub enum PaperForm {
    /// Seed XOR parts, each a line of words; every one is needed.
    Xor(Vec<Zeroizing<String>>),
    /// codex32 shares; any `k` of them make the seed.
    Codex32 {
        /// How many make the seed.
        k: u8,
        /// The strings.
        strings: Vec<Zeroizing<String>>,
    },
}

/// `len` bytes spread from one fresh draw, for `what`.
fn spread(draw: &[u8; 32], what: &[u8], len: usize) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::with_capacity(len + 32));
    let mut i = 0u32;
    while out.len() < len {
        let mut e = sha256::Hash::engine();
        e.input(draw);
        e.input(what);
        e.input(&i.to_le_bytes());
        out.extend_from_slice(&sha256::Hash::from_engine(e).to_byte_array());
        i += 1;
    }
    out.truncate(len);
    out
}

impl Faraday {
    /// Costs what `parse_words` costs. The mnemonic of the key the backup
    /// shows, and its passphrase.
    fn paper_key(&self, c: &Compute) -> Option<(Mnemonic, Zeroizing<String>, [u8; 4])> {
        let b = self.backup.as_ref()?;
        let k = self.session.keys.get(b.key)?;
        let m = crate::wallet::parse_words(c, k.language, k.words.as_ref()?).ok()?;
        let p = Zeroizing::new(
            k.passphrase
                .as_deref()
                .map_or("", |p| p.as_str())
                .to_string(),
        );
        Some((m, p, k.fingerprint().0))
    }

    /// Splits the key's words into `parts` Seed XOR parts.
    pub(crate) fn paper_xor(&mut self, parts: u8) {
        let c = self.compute();
        // Fresh parts are a new secret: none in the demo.
        if !self.makes_secrets() {
            return;
        }
        let Some((m, _, _)) = self.paper_key(&c) else {
            return self.toast(EN.paper_this_key_has_no);
        };
        let Some(draw) = self.fresh(b"seed xor") else {
            return self.toast(EN.paper_no_randomness);
        };
        let lang = m.language();
        let entropy = m.entropy();
        let seed = entropy.expose().as_bytes();
        let mut xor = SeedXor::new();
        let mut out = Vec::new();
        for i in 0..parts.saturating_sub(1) {
            let random = spread(&draw, &[b'x', i], seed.len());
            if xor.push(&random).is_err() {
                return;
            }
            out.push(random);
        }
        if xor.push(seed).is_err() {
            return;
        }
        let Ok(last) = xor.entropy() else {
            return;
        };
        out.push(Zeroizing::new(last.as_bytes().to_vec()));
        let mut lines = Vec::new();
        for e in &out {
            let Ok(part) = Mnemonic::from_entropy(lang, e) else {
                return;
            };
            let mut line = crate::secret_text::room();
            for (k, &w) in part.indices().iter().enumerate() {
                if k > 0 {
                    line.push(' ');
                }
                line.push_str(lang.word(w));
            }
            lines.push(line);
        }
        if let Some(b) = self.backup.as_mut() {
            b.paper = Some(PaperForm::Xor(lines));
        }
    }

    /// codex32 shares of the key's BIP-32 seed, `k` of `n`.
    pub(crate) fn paper_codex32(&mut self, k: u8, n: u8) {
        let c = self.compute();
        if k >= 2 && !self.makes_secrets() {
            return;
        }
        let Some((m, passphrase, fp)) = self.paper_key(&c) else {
            return self.toast(EN.paper_this_key_has);
        };
        let Some(seed) = faraday_keys::identity::seed_of(&c, &m, &passphrase) else {
            return;
        };
        let Some(draw) = self.fresh(b"codex32") else {
            return self.toast(EN.paper_no_randomness);
        };
        let need = usize::from(k.saturating_sub(1)) * codex32::symbols_for(seed.expose().len());
        let fresh = spread(&draw, b"codex32 shares", need);
        let shares = Codex32::from_seed(k, codex32::identifier_for(fp), seed.expose())
            .and_then(|secret| secret.split(k, n, &fresh));
        match shares {
            Ok(shares) => {
                let strings = shares
                    .iter()
                    .map(|s| Zeroizing::new(s.encode().as_str().to_string()))
                    .collect();
                if let Some(b) = self.backup.as_mut() {
                    b.paper = Some(PaperForm::Codex32 { k, strings });
                }
            }
            Err(e) => self.toast(&fill(EN.paper_codex32, &[&e.to_string()])),
        }
    }
}
