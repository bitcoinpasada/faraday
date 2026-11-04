//! What Faraday does with a held key's secret, as methods of
//! [`Keys`](crate::Keys): each takes the key's place and the plain values
//! it needs, and gives back a public result, or a new secret that the
//! caller holds under `Zeroizing` (`docs/KEYS.md` §3).

use faraday_bip::bip39::Mnemonic;
use faraday_bip::bitcoin::bip32::{DerivationPath, Xpub};
use faraday_bip::codex32::{self, Codex32};
use faraday_bip::keys::{Fingerprint, MasterKey, Network, ScriptType};
use faraday_bip::slip39::{self, Share};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use faraday_entropy::SeedXor;
use zeroize::Zeroizing;

use crate::bip85;
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
    /// The key has words: its seed is never split into codex32 strings,
    /// which would hold its 64-byte seed with the passphrase merged in
    /// and give back neither the words nor the passphrase.
    Words,
    /// The split was refused, with the reason's text.
    Refused(String),
    /// Something else went wrong; there is nothing to say.
    Failed,
}

/// Why a conversion of a key's master seed was not made
/// ([`Keys::master_seed_hex`], [`Keys::master_xprv`],
/// [`Keys::slip39_one_of_one`], [`Keys::codex32_converted`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Convert {
    /// No such key, a reason but `Reveal::PaperForm`, or no words or seed
    /// bytes to convert.
    NotGiven,
    /// The key has words: their seed is the words' and passphrase's, which
    /// SLIP-39 and codex32 do not carry as the words' wallet.
    Words,
    /// The seed's length is not one the standard holds (SLIP-39: 16 or
    /// 32 bytes).
    Length,
    /// The standard refused the shape asked for (codex32's threshold,
    /// count or fresh bytes).
    Refused,
    /// The result did not read back, through its standard's decoder, as
    /// the same wallet (the same master fingerprint): nothing is returned.
    NotSameWallet,
}

/// The fresh bytes Seed XOR into `parts` parts takes at most (a lookup):
/// a random part of the longest words, 32 bytes, for every part but the
/// last ([`Keys::seed_xor`]).
pub fn seed_xor_random_len(parts: u8) -> usize {
    usize::from(parts.saturating_sub(1)) * 32
}

/// The fresh bytes a codex32 set any `k` of which recover the seed takes
/// at most (a lookup): a symbol for each of the longest seed's, 64 bytes,
/// in each of the `k − 1` random shares ([`Keys::codex32_shares`],
/// [`Keys::codex32_new_set`]).
pub fn codex32_random_len(k: u8) -> usize {
    usize::from(k.saturating_sub(1)) * codex32::symbols_for(64)
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
    #[allow(
        clippy::too_many_arguments,
        reason = "a vanity search's key index, dial, script, prefix and the cursor and budget of one slice"
    )]
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

    /// Costs a key derivation of at most
    /// [`faraday_bip::keys::MAX_PATH_STEPS`] steps and an elliptic-curve
    /// multiplication. The extended public key at `path` below key `i`;
    /// `None` for a longer path, which no wallet has.
    pub fn xpub_at(&self, c: &Compute, i: usize, path: &DerivationPath) -> Option<Xpub> {
        if path.len() > faraday_bip::keys::MAX_PATH_STEPS {
            return None;
        }
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
    /// `random` is fresh randomness, taken as it is: the random parts are
    /// its first bytes in turn, each as long as the words' entropy; fewer
    /// bytes than that are refused ([`seed_xor_random_len`] is always
    /// enough).
    pub fn seed_xor(
        &self,
        c: &Compute,
        i: usize,
        parts: u8,
        random: &[u8],
        _why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Paper> {
        let s = self.secrets(i).ok_or(Paper::NoWords)?;
        let m = Mnemonic::parse(c, s.language, s.words.ok_or(Paper::NoWords)?)
            .map_err(|_| Paper::NoWords)?;
        let lang = m.language();
        let entropy = m.entropy();
        let seed = entropy.expose().as_bytes();
        let randoms = usize::from(parts.saturating_sub(1));
        if random.len() < randoms * seed.len() {
            return Err(Paper::Failed);
        }
        let mut xor = SeedXor::new();
        let mut out = Vec::new();
        for part in random.chunks_exact(seed.len()).take(randoms) {
            let part = Zeroizing::new(part.to_vec());
            if xor.push(&part).is_err() {
                return Err(Paper::Failed);
            }
            out.push(part);
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

    /// Key `i`'s master seed as codex32
    /// strings (BIP 93): `n` shares, any `k` enough (`k` from 2 to 9, `n`
    /// from `k` to 31), or, with `k` 0 and `n` 1, the secret itself. A key
    /// loaded from a master seed (`add_seed`) gives the seed it was loaded
    /// from, at that seed's own length; a key with words is refused
    /// ([`Paper::Words`]). `random` is fresh randomness, taken as it is:
    /// the other shares' symbols are its first bytes in turn, one byte a
    /// symbol; fewer than they need are refused ([`codex32_random_len`]
    /// is always enough).
    pub fn codex32_shares(
        &self,
        i: usize,
        k: u8,
        n: u8,
        random: &[u8],
        _why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Paper> {
        let s = self.secrets(i).ok_or(Paper::NoWords)?;
        let fp = s.master.fingerprint().0;
        let seed: &[u8] = match (s.words, s.seed) {
            (Some(_), _) => return Err(Paper::Words),
            (None, Some(bytes)) => bytes,
            (None, None) => return Err(Paper::NoWords),
        };
        let one = |e: codex32::Error| Paper::Refused(faraday_strings::bip::codex32_error(&e));
        let id = codex32::identifier_for(fp);
        if k == 0 {
            if n != 1 {
                return Err(Paper::Refused(faraday_strings::bip::codex32_error(
                    &codex32::Error::BadSplit,
                )));
            }
            let secret = Codex32::from_seed(0, id, seed).map_err(one)?;
            return Ok(Held::of(vec![Zeroizing::new(
                secret.encode().as_str().to_string(),
            )]));
        }
        let need = usize::from(k.saturating_sub(1)) * codex32::symbols_for(seed.len());
        let fresh = random
            .get(..need)
            .ok_or_else(|| one(codex32::Error::NotEnoughRandomness))?;
        Codex32::from_seed(k, id, seed)
            .and_then(|secret| secret.split(k, n, fresh))
            .map(|shares| {
                shares
                    .iter()
                    .map(|s| Zeroizing::new(s.encode().as_str().to_string()))
                    .collect()
            })
            .map(Held::of)
            .map_err(one)
    }

    /// Costs a parse of the words and a PBKDF2 for a key with words, and a
    /// master key from the seed read back. Key `i`'s master seed in
    /// lower-case hex: the 64-byte seed of a key's words and passphrase,
    /// or the seed bytes of a key loaded by `add_seed`. Read back: the hex
    /// decoded and its master key's fingerprint is the key's.
    pub fn master_seed_hex(
        &self,
        c: &Compute,
        i: usize,
        why: Reveal,
    ) -> Result<Held<Zeroizing<String>>, Convert> {
        let s = self.convert_source(i, why)?;
        let full;
        let seed: &[u8] = match (s.words, s.seed) {
            (Some(words), _) => {
                let m = Mnemonic::parse(c, s.language, words).map_err(|_| Convert::NotGiven)?;
                full = m
                    .to_seed(c, s.passphrase.unwrap_or("").as_bytes())
                    .map_err(|_| Convert::NotGiven)?;
                &full.expose()[..]
            }
            (None, Some(bytes)) => bytes,
            (None, None) => return Err(Convert::NotGiven),
        };
        let text = faraday_crypto::hex::encode_secret(seed);
        let back = faraday_crypto::hex::decode_vec(&text).map(Zeroizing::new);
        let read = back.as_deref().and_then(|b| fingerprint_of_seed(c, b));
        checked(s.master.fingerprint(), read, text)
    }

    /// Costs a parse of the master key's text. Key `i`'s master extended
    /// private key as text: `xprv…` on mainnet, `tprv…` on a test network.
    /// Read back: the text parsed by the strict importer has the key's
    /// fingerprint.
    pub fn master_xprv(
        &self,
        c: &Compute,
        i: usize,
        why: Reveal,
    ) -> Result<Held<Zeroizing<String>>, Convert> {
        let s = self.convert_source(i, why)?;
        let mut text = faraday_intake::secret_buf::room();
        text.push_str(s.master.xpriv_ascii().as_str());
        let read = MasterKey::decode(c, &text).ok().map(|k| k.fingerprint());
        checked(s.master.fingerprint(), read, text)
    }

    /// Costs a master key and what `slip39::one_of_one` and
    /// `slip39::recover` cost (a PBKDF2 each). Key `i`'s seed bytes as the
    /// SLIP-39 1 of 1, as words: a key loaded by `add_seed` of 16 or 32
    /// bytes; any other length is refused ([`Convert::Length`]), and a key
    /// with words ([`Convert::Words`]), since the words' entropy as
    /// SLIP-39 is another wallet and SLIP-39 holds no 64-byte seed. Read
    /// back: the words parsed and recovered with an empty passphrase give
    /// the key's fingerprint.
    pub fn slip39_one_of_one(
        &self,
        c: &Compute,
        i: usize,
        why: Reveal,
    ) -> Result<Held<Zeroizing<String>>, Convert> {
        let s = self.convert_source(i, why)?;
        let seed = match (s.words, s.seed) {
            (Some(_), _) => return Err(Convert::Words),
            (None, Some(bytes)) => bytes,
            (None, None) => return Err(Convert::NotGiven),
        };
        if seed.len() != 16 && seed.len() != 32 {
            return Err(Convert::Length);
        }
        let share = slip39::one_of_one(c, seed).map_err(|_| Convert::Refused)?;
        let mut text = faraday_intake::secret_buf::room();
        for n in 0..share.word_count() {
            if n > 0 {
                text.push(' ');
            }
            text.push_str(share.word(n));
        }
        let read = Share::parse(&text)
            .ok()
            .and_then(|share| slip39_seed(c, &[share], b"").ok())
            .and_then(|secret| fingerprint_of_seed(c, &secret));
        checked(s.master.fingerprint(), read, text)
    }

    /// Costs what [`Keys::codex32_shares`] costs and a master key.
    /// [`Keys::codex32_shares`] for key `i` (`k` 0 and `n` 1 is the secret
    /// itself), given only for `Reveal::PaperForm` and only when the set
    /// read back through codex32's decoder (every share parsed, and the
    /// first `k` and the last `k` each recovering the key, or the secret)
    /// gives the key's fingerprint. A key with words is refused
    /// ([`Convert::Words`]).
    pub fn codex32_converted(
        &self,
        c: &Compute,
        i: usize,
        k: u8,
        n: u8,
        random: &[u8],
        why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Convert> {
        let s = self.convert_source(i, why)?;
        let shares = self
            .codex32_shares(i, k, n, random, why)
            .map_err(|e| match e {
                Paper::Words => Convert::Words,
                Paper::NoWords => Convert::NotGiven,
                Paper::Refused(_) | Paper::Failed => Convert::Refused,
            })?
            .release();
        let take = usize::from(k.max(1));
        let mut parsed = Vec::with_capacity(shares.len());
        for t in shares.iter() {
            match Codex32::parse(t) {
                Ok(p) => parsed.push(p),
                Err(_) => return Err(Convert::NotSameWallet),
            }
        }
        // The first `take` shares and the last `take` each recover the key.
        let of = |part: &[Codex32]| {
            codex32_seed(part)
                .ok()
                .and_then(|seed| fingerprint_of_seed(c, &seed))
        };
        let first = of(&parsed[..take.min(parsed.len())]);
        let last = of(&parsed[parsed.len().saturating_sub(take)..]);
        let read = if first == last { first } else { None };
        checked(s.master.fingerprint(), read, shares)
    }

    /// The key `i`'s secrets for a conversion: only for `Reveal::PaperForm`
    /// (each result is written by hand), and only for a key that is held.
    fn convert_source(
        &self,
        i: usize,
        why: Reveal,
    ) -> Result<crate::session::Secrets<'_>, Convert> {
        if why != Reveal::PaperForm {
            return Err(Convert::NotGiven);
        }
        self.secrets(i).ok_or(Convert::NotGiven)
    }
}

/// Costs an HMAC-SHA512 and an elliptic-curve multiplication. The master
/// fingerprint of a BIP-32 seed read back from a result (the network does
/// not change it); `None` for one that is too long.
fn fingerprint_of_seed(c: &Compute, seed: &[u8]) -> Option<Fingerprint> {
    let bytes = SeedBytes::new(seed)?;
    Some(MasterKey::from_seed_bytes(c, &Secret::new(bytes), Network::Mainnet).fingerprint())
}

/// `result`, held, when what it read back as is the key's wallet: the same
/// master fingerprint. Anything else, including a result that did not
/// read at all (`None`), gives [`Convert::NotSameWallet`] and no result.
fn checked<T>(key: Fingerprint, read: Option<Fingerprint>, result: T) -> Result<Held<T>, Convert> {
    if read == Some(key) {
        Ok(Held::of(result))
    } else {
        Err(Convert::NotSameWallet)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A result that reads back as another wallet is not returned.
    #[test]
    fn a_wrong_fingerprint_gives_the_error_and_no_result() {
        let key = Fingerprint([0x73, 0xc5, 0xda, 0x0a]);
        assert!(checked(key, Some(key), "x").is_ok());
        assert_eq!(
            checked(key, Some(Fingerprint([0, 0, 0, 1])), "x").err(),
            Some(Convert::NotSameWallet)
        );
        assert_eq!(checked(key, None, "x").err(), Some(Convert::NotSameWallet));
    }
}
