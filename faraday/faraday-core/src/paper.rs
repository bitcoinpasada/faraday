//! Other paper forms of a key held here, beside its words:
//! a Seed XOR split of its words
//! into parts that are each BIP-39 words, every part needed; and codex32
//! shares of its BIP-32 seed (BIP 93), any `k` of `n` enough. The
//! arithmetic is `faraday-keys`' (`Keys::seed_xor`, `Keys::codex32_shares`);
//! the randomness is fresh from the system. The parts are shown to copy by
//! hand and never written to a file; only the seed itself goes into a
//! vault or, past the secret sheet, a file (`crate::secrets`).

use crate::strings::EN;
use crate::strings::fill;
use faraday_keys::Paper;
use zeroize::Zeroizing;

use crate::Faraday;

/// A key's other paper form, on screen.
pub enum PaperForm {
    /// Seed XOR parts, each a line of words; every one is needed.
    Xor(faraday_keys::Held<Vec<Zeroizing<String>>>),
    /// codex32 shares; any `k` of them make the seed.
    Codex32 {
        /// How many make the seed.
        k: u8,
        /// The strings.
        strings: faraday_keys::Held<Vec<Zeroizing<String>>>,
    },
}

impl Faraday {
    /// Whether the key the backup shows has words to make a paper form of.
    fn paper_key(&self) -> bool {
        self.backup
            .as_ref()
            .and_then(|b| self.session.keys.info(b.key))
            .is_some_and(|k| k.has_words())
    }

    /// Splits the key's words into `parts` Seed XOR parts.
    pub(crate) fn paper_xor(&mut self, parts: u8) {
        let c = self.compute();
        // Fresh parts are a new secret: none in the demo.
        if !self.makes_secrets() {
            return;
        }
        if !self.paper_key() {
            return self.toast(EN.paper_no_words_to_split);
        }
        let Some(draw) = self.fresh(b"seed xor") else {
            return self.toast(EN.paper_no_randomness);
        };
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let Ok(lines) =
            self.session
                .keys
                .seed_xor(&c, b.key, parts, &draw, faraday_keys::Reveal::PaperForm)
        else {
            return;
        };
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
        if !self.paper_key() {
            return self.toast(EN.paper_this_key_has);
        }
        let Some(draw) = self.fresh(b"codex32") else {
            return self.toast(EN.paper_no_randomness);
        };
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        match self.session.keys.codex32_shares(
            &c,
            b.key,
            k,
            n,
            &draw,
            faraday_keys::Reveal::PaperForm,
        ) {
            Ok(strings) => {
                if let Some(b) = self.backup.as_mut() {
                    b.paper = Some(PaperForm::Codex32 { k, strings });
                }
            }
            Err(Paper::Refused(e)) => self.toast(&fill(EN.paper_codex32, &[&e])),
            Err(_) => {}
        }
    }
}
