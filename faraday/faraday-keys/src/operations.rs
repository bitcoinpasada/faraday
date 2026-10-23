//! What Faraday does with a held key's secret, as methods of
//! [`Keys`](crate::Keys): each takes the key's place and the plain values
//! it needs, and gives back a public result, or a new secret that the
//! caller holds under `Zeroizing` (`docs/KEYS.md` §3).

use faraday_bip::bip39::Mnemonic;
use faraday_bip::bitcoin::bip32::{DerivationPath, Xpub};
use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use faraday_bip::codex32::{self, Codex32};
use faraday_bip::keys::ScriptType;
use faraday_bip::slip39;
use faraday_crypto::work::Compute;
use faraday_entropy::SeedXor;
use zeroize::Zeroizing;

use crate::bip85;
use crate::frost;
use crate::lightning;
use crate::vanity;
use crate::{Held, Keys, Reveal};

/// Which dial a vanity search turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dial {
    /// The account number of the key's first address.
    Account,
    /// A suffix of the BIP-39 passphrase.
    Passphrase,
}

/// Why a paper form was not made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Paper {
    /// The key has no words, or they do not read.
    NoWords,
    /// The split was refused, with the reason's text.
    Refused(String),
    /// Something else went wrong; there is nothing to say.
    Failed,
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

impl Keys {
    /// Costs a parse of the words, a PBKDF2, key derivations and an
    /// elliptic-curve multiplication. The Lightning node key ldk-node
    /// derives from key `i`'s words and passphrase, on the store's
    /// network. `None` for a key with no words.
    pub fn lightning_node(&self, c: &Compute, i: usize) -> Option<Result<lightning::Node, String>> {
        let s = self.secrets(i)?;
        let words = s.words?;
        Some(lightning::node_from_words(
            c,
            s.language,
            words,
            s.passphrase.unwrap_or(""),
            self.network(),
        ))
    }

    /// Costs what `vanity::grind` costs. A budget of a vanity search on key
    /// `i`: its first address made to begin with `prefix`, on `dial`.
    /// `None` where the passphrase dial has no words to turn.
    #[allow(clippy::too_many_arguments)]
    pub fn vanity_grind(
        &self,
        c: &Compute,
        i: usize,
        dial: Dial,
        script: ScriptType,
        prefix: &str,
        cursor: u64,
        budget: u64,
    ) -> Option<vanity::Outcome> {
        let s = self.secrets(i)?;
        let dial = match dial {
            Dial::Account => vanity::Dial::Account(s.master),
            Dial::Passphrase => vanity::Dial::Passphrase {
                language: s.language,
                words: s.words,
                base: s.passphrase.map_or(&[], |p| p.as_bytes()),
            },
        };
        vanity::grind(c, dial, script, prefix, self.network(), cursor, budget)
    }

    /// Costs what `make_group` costs. Deals a threshold wallet from the
    /// keys at `places`, in slot order: the other shares and the group key
    /// (`n` shares, `m` needed), each key's words read as a share. The new
    /// shares stay in the [`frost::Dealt`] until [`Keys::add_dealt`].
    pub fn deal(
        &self,
        c: &Compute,
        places: &[usize],
        n: usize,
        m: usize,
    ) -> Result<frost::Dealt, frost::DealError> {
        let dealers: Vec<frost::Dealer<'_>> = places
            .iter()
            .map(|&i| match self.secrets(i) {
                Some(s) => frost::Dealer {
                    language: s.language,
                    words: s.words,
                },
                None => frost::Dealer {
                    language: faraday_bip::bip39::Language::English,
                    words: None,
                },
            })
            .collect();
        frost::make_group(c, &dealers, n, m, self.network())
    }

    /// Costs what `add_words` costs per share. Loads the new shares of
    /// `dealt` as keys, each labelled `label(id)` by its identifier
    /// (counting from 0), so that their words can be written down in the
    /// backup like any key's.
    pub fn add_dealt(&mut self, c: &Compute, dealt: frost::Dealt, label: impl Fn(usize) -> String) {
        for (id, words) in &dealt.words {
            let _ = self.add_words(c, words, &label(*id), None);
        }
    }

    /// Costs what `bip85::child` costs. The BIP-85 child of `app` at
    /// `index` below key `i`.
    pub fn bip85_child(
        &self,
        c: &Compute,
        i: usize,
        app: bip85::App,
        index: u32,
        _why: Reveal,
    ) -> Option<Held<Zeroizing<String>>> {
        bip85::child(c, self.secrets(i)?.master, app, index).map(Held::of)
    }

    /// Costs a key derivation and an elliptic-curve multiplication. The
    /// extended public key at `path` below key `i`.
    pub fn xpub_at(&self, c: &Compute, i: usize, path: &DerivationPath) -> Option<Xpub> {
        Some(self.secrets(i)?.master.derive(c, path).to_xpub(c))
    }

    /// Costs two key derivations. The silent payments scan and spend keys
    /// of key `i` at `account`.
    pub fn silent_receiver(
        &self,
        c: &Compute,
        i: usize,
        account: u32,
    ) -> Option<crate::silent::Receiver> {
        Some(crate::silent::receiver(c, self.secrets(i)?.master, account))
    }

    /// Costs a parse of the words and a split. Key `i`'s words split into
    /// `parts` Seed XOR parts, each a line of words, every one needed.
    /// `draw` is fresh randomness the random parts are spread from.
    pub fn seed_xor(
        &self,
        c: &Compute,
        i: usize,
        parts: u8,
        draw: &[u8; 32],
        _why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Paper> {
        let s = self.secrets(i).ok_or(Paper::NoWords)?;
        let m = Mnemonic::parse(c, s.language, s.words.ok_or(Paper::NoWords)?)
            .map_err(|_| Paper::NoWords)?;
        let lang = m.language();
        let entropy = m.entropy();
        let seed = entropy.expose().as_bytes();
        let mut xor = SeedXor::new();
        let mut out = Vec::new();
        for k in 0..parts.saturating_sub(1) {
            let random = spread(draw, &[b'x', k], seed.len());
            if xor.push(&random).is_err() {
                return Err(Paper::Failed);
            }
            out.push(random);
        }
        if xor.push(seed).is_err() {
            return Err(Paper::Failed);
        }
        let Ok(last) = xor.entropy() else {
            return Err(Paper::Failed);
        };
        out.push(Zeroizing::new(last.as_bytes().to_vec()));
        let mut lines = Vec::new();
        for e in &out {
            let Ok(part) = Mnemonic::from_entropy(lang, e) else {
                return Err(Paper::Failed);
            };
            let mut line = faraday_intake::secret_buf::room();
            for (k, &w) in part.indices().iter().enumerate() {
                if k > 0 {
                    line.push(' ');
                }
                line.push_str(lang.word(w));
            }
            lines.push(line);
        }
        Ok(Held::of(lines))
    }

    /// Costs a parse of the words and a PBKDF2 for a key with words;
    /// a split of the seed in either case. Key `i`'s BIP-32 seed as
    /// codex32 strings (BIP 93): `n` shares, any `k` enough (`k` from 2
    /// to 9, `n` from `k` to 31), or, with `k` 0 and `n` 1, the secret
    /// itself. A key with words gives its 64-byte BIP-39 seed (127
    /// characters); a key loaded from a master seed (`add_seed`) gives
    /// the seed it was loaded from, at that seed's own length. `draw` is
    /// fresh randomness the other shares are spread from.
    pub fn codex32_shares(
        &self,
        c: &Compute,
        i: usize,
        k: u8,
        n: u8,
        draw: &[u8; 32],
        _why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Paper> {
        let s = self.secrets(i).ok_or(Paper::NoWords)?;
        let fp = s.master.fingerprint().0;
        let made;
        let seed: &[u8] = match (s.words, s.seed) {
            (Some(words), _) => {
                let m = Mnemonic::parse(c, s.language, words).map_err(|_| Paper::NoWords)?;
                made = crate::identity::seed_of(c, &m, s.passphrase.unwrap_or(""))
                    .ok_or(Paper::Failed)?;
                made.expose()
            }
            (None, Some(bytes)) => bytes,
            (None, None) => return Err(Paper::NoWords),
        };
        let one = |e: codex32::Error| Paper::Refused(e.to_string());
        let id = codex32::identifier_for(fp);
        if k == 0 {
            if n != 1 {
                return Err(Paper::Refused(codex32::Error::BadSplit.to_string()));
            }
            let secret = Codex32::from_seed(0, id, seed).map_err(one)?;
            return Ok(Held::of(vec![Zeroizing::new(
                secret.encode().as_str().to_string(),
            )]));
        }
        let need = usize::from(k.saturating_sub(1)) * codex32::symbols_for(seed.len());
        let fresh = spread(draw, b"codex32 shares", need);
        Codex32::from_seed(k, id, seed)
            .and_then(|secret| secret.split(k, n, &fresh))
            .map(|shares| {
                shares
                    .iter()
                    .map(|s| Zeroizing::new(s.encode().as_str().to_string()))
                    .collect()
            })
            .map(Held::of)
            .map_err(one)
    }
}

/// The master seed `parts` make: the one that holds the secret itself, or
/// the recovery of the shares.
pub fn codex32_seed(parts: &[Codex32]) -> Result<Zeroizing<Vec<u8>>, codex32::Error> {
    let seed = match parts.iter().find(|c| c.is_secret()) {
        Some(s) => s.to_seed(),
        None => Codex32::recover(parts).and_then(|s| s.to_seed()),
    }?;
    Ok(Zeroizing::new(seed.expose().bytes().to_vec()))
}

/// Costs a share recovery (SLIP-39). The master secret `shares` make with
/// `passphrase`.
pub fn slip39_seed(
    c: &Compute,
    shares: &[slip39::Share],
    passphrase: &[u8],
) -> Result<Zeroizing<Vec<u8>>, slip39::Error> {
    let secret = slip39::recover(c, shares, passphrase)?;
    Ok(Zeroizing::new(secret.expose().as_bytes().to_vec()))
}
