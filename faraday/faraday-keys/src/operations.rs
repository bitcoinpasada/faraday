//! What Faraday does with a held key's secret, as methods of
//! [`Keys`](crate::Keys): each takes the key's place and the plain values
//! it needs, and gives back a public result, or a new secret that the
//! caller holds under `Zeroizing` (`docs/KEYS.md` §3).

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::bip32::{DerivationPath, Xpub};
use faraday_bip::codex32::{self, Codex32};
use faraday_bip::keys::{Fingerprint, MasterKey, Network};
use faraday_bip::slip39::{self, Share};
use faraday_codec::qr::QrMatrix;
use faraday_codec::seedqr;
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use faraday_entropy::SeedXor;
use subtle::ConstantTimeEq;
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
/// [`Keys::slip39_one_of_one`], [`Keys::codex32_converted`],
/// [`Keys::seed_xor_converted`]).
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

/// A way to write a key's words as text ([`Keys::words_written`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordsForm {
    /// The words themselves, as the list writes them, single spaces between.
    Words,
    /// The entropy they carry, in lower-case hex.
    HexEntropy,
    /// Each word's number on the list, one a line: 1 to 2048, then 0 to
    /// 2047 in brackets, `1 (0)`.
    Numbers,
    /// Each word's first four letters, single spaces between: the English
    /// list only, which is unique in them.
    Starts,
}

/// The fresh bytes Seed XOR into `parts` parts takes at most (a lookup):
/// a random part of the longest words, 32 bytes, for every part but the
/// last ([`Keys::seed_xor`]).
pub fn seed_xor_random_len(parts: u8) -> usize {
    usize::from(parts.saturating_sub(1)) * 32
}

/// The fresh bytes a codex32 set any `k` of which recover the seed takes
/// at most (a lookup): a symbol for each of the longest seed's, 64 bytes,
/// in each of the `k − 1` random shares, then, for a set (`k` above 0),
/// four for its identifier ([`Keys::codex32_shares`],
/// [`Keys::codex32_new_set`], which leaves the last four unused).
pub fn codex32_random_len(k: u8) -> usize {
    match k {
        0 => 0,
        k => usize::from(k - 1) * codex32::symbols_for(64) + IDENTIFIER_LEN,
    }
}

/// The characters of a codex32 identifier.
const IDENTIFIER_LEN: usize = 4;

/// A codex32 identifier drawn from `random`'s first four bytes, five bits
/// of each, that is none of `avoid`: a draw that lands on one is stepped
/// through the whole 20-bit value (wrapping) until it does not. `None`
/// where `avoid` holds every identifier, or `random` has fewer than four
/// bytes.
fn drawn_identifier(random: &[u8], avoid: &[[u8; 4]]) -> Option<[u8; 4]> {
    let bits = random.get(..IDENTIFIER_LEN)?;
    let start = bits.iter().fold(0u32, |v, b| (v << 5) | u32::from(b & 31));
    (0..1u32 << 20).find_map(|step| {
        let v = (start + step) & ((1 << 20) - 1);
        let id: [u8; 4] =
            std::array::from_fn(|k| codex32::CHARSET[((v >> (15 - 5 * k)) & 31) as usize]);
        (!avoid.contains(&id)).then_some(id)
    })
}

/// The fresh bytes a SLIP-39 set of these groups takes for a secret of
/// `seed_len` bytes (a lookup): two for the identifier, then what the
/// split asks for ([`slip39::random_len`]).
pub fn slip39_random_len(
    seed_len: usize,
    group_threshold: u8,
    groups: &[slip39::GroupSpec],
) -> usize {
    2 + slip39::random_len(seed_len, group_threshold, groups)
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
        reason = "a vanity search's key index, dial, kind, prefix and the cursor and budget of one slice"
    )]
    pub fn vanity_grind(
        &self,
        c: &Compute,
        i: usize,
        dial: Dial,
        kind: faraday_bip::vanity::Kind,
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
        vanity::grind(c, dial, kind, prefix, self.network(), cursor, budget)
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
    /// symbol, and for a set the four bytes after them draw its
    /// identifier, never `identifier_for` the key's fingerprint nor its own
    /// set's (a new set from a seed is not the same set: shares of two
    /// would otherwise interpolate to another seed with no error); fewer
    /// than they need are refused ([`codex32_random_len`] is always
    /// enough). The secret alone, `k` 0, keeps that identifier.
    pub fn codex32_shares(
        &self,
        i: usize,
        k: u8,
        n: u8,
        random: &[u8],
        why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Paper> {
        self.codex32_shares_avoiding(i, k, n, random, &[], why)
    }

    /// [`Keys::codex32_shares`] where the set's drawn identifier is also
    /// none of `taken`, the identifiers other sets on the device carry
    /// (lower case): a draw that lands on one is stepped on as for the
    /// key's own.
    pub fn codex32_shares_avoiding(
        &self,
        i: usize,
        k: u8,
        n: u8,
        random: &[u8],
        taken: &[[u8; 4]],
        _why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Paper> {
        let s = self.source_secrets(i).ok_or(Paper::NoWords)?;
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
        let drawn = random
            .get(need..need + IDENTIFIER_LEN)
            .ok_or_else(|| one(codex32::Error::NotEnoughRandomness))?;
        let own = self
            .source_info(i)
            .and_then(|k| k.codex32.as_ref())
            .map(|f| f.identifier);
        let avoid: Vec<[u8; 4]> = std::iter::once(id)
            .chain(own)
            .chain(taken.iter().map(|t| t.map(|b| b.to_ascii_lowercase())))
            .collect();
        let id = drawn_identifier(drawn, &avoid)
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

    /// Costs two parses of the words for [`WordsForm::Words`], one otherwise,
    /// and a scan of the list per word for [`WordsForm::Starts`]. Key `i`'s
    /// words written in `form`. A key with no words is
    /// [`Convert::NotGiven`], as are the starts of a key on another list.
    /// Read back: the text read as that form gives the same entropy as the
    /// key's words, on the same list, so the same words. The seed is made
    /// from those words and the key's passphrase, so the same words are the
    /// same fingerprint: no PBKDF2 is run to compare it.
    pub fn words_written(
        &self,
        c: &Compute,
        i: usize,
        form: WordsForm,
        why: Reveal,
    ) -> Result<Held<Zeroizing<String>>, Convert> {
        let s = self.convert_source(i, why)?;
        let words = s.words.ok_or(Convert::NotGiven)?;
        let lang = s.language;
        if form == WordsForm::Starts && lang != Language::English {
            return Err(Convert::NotGiven);
        }
        let m = Mnemonic::parse(c, lang, words).map_err(|_| Convert::NotGiven)?;
        let text = match form {
            WordsForm::HexEntropy => {
                faraday_crypto::hex::encode_secret(m.entropy().expose().as_bytes())
            }
            _ => written_by_word(&m, form),
        };
        let read = match form {
            WordsForm::Words => Mnemonic::parse(c, lang, &text).ok(),
            WordsForm::HexEntropy => faraday_crypto::hex::decode_vec(&text)
                .map(Zeroizing::new)
                .and_then(|b| crate::words::from_entropy(lang, &b).ok()),
            WordsForm::Numbers => {
                numbers_read(&text).and_then(|n| crate::words::from_indices(lang, &n).ok())
            }
            WordsForm::Starts => {
                starts_read(&text).and_then(|n| crate::words::from_indices(lang, &n).ok())
            }
        };
        same_words(&m, read.as_ref(), text)
    }

    /// Costs a parse of the words and a QR encoding. Key `i`'s SeedQR, or
    /// CompactSeedQR if `compact`, with its word count: for a key on the
    /// English list of 12 or 24 words only, any other being
    /// [`Convert::NotGiven`]. Read back: the digits or the entropy bytes the
    /// code is made of, read by SeedQR's decoder, give the same entropy as
    /// the key's words, so the same fingerprint, as [`Keys::words_written`].
    /// The modules themselves are not scanned here, as this crate holds no
    /// QR reader: the caller scans the matrix and checks the payload with
    /// [`Keys::seedqr_reads_as`].
    pub fn seedqr_written(
        &self,
        c: &Compute,
        i: usize,
        compact: bool,
        why: Reveal,
    ) -> Result<(Held<Box<QrMatrix>>, usize), Convert> {
        let s = self.convert_source(i, why)?;
        let words = s.words.ok_or(Convert::NotGiven)?;
        if s.language != Language::English {
            return Err(Convert::NotGiven);
        }
        let m = Mnemonic::parse(c, s.language, words).map_err(|_| Convert::NotGiven)?;
        if !matches!(m.word_count(), 12 | 24) {
            return Err(Convert::NotGiven);
        }
        let (matrix, read) = if compact {
            let bytes = seedqr::to_entropy(&m);
            (
                seedqr::encode_compact(c, &m),
                crate::words::from_compact_seedqr(bytes.expose().as_bytes(), s.language),
            )
        } else {
            let digits = seedqr::to_digits(&m);
            (
                seedqr::encode_seedqr(c, &m),
                crate::words::from_seedqr_digits(digits.expose().as_bytes(), s.language),
            )
        };
        // Boxed after the encoder returned it by value, so that return's
        // copy of the modules may stay on the stack; boxing only stops the
        // later moves by value, each of which would leave another.
        let matrix = Box::new(matrix.map_err(|_| Convert::NotGiven)?);
        let held = same_words(&m, read.ok().as_ref(), matrix)?;
        Ok((held, m.word_count()))
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

    /// Costs a master key, a PBKDF2 for the split and two more for the
    /// read back. Key `i`'s seed bytes as a SLIP-39 set (`group_threshold`
    /// of the `groups`, each a threshold of its members), given only for
    /// `Reveal::PaperForm`; a key with words is refused
    /// ([`Convert::Words`]), as is a length other than 16 or 32 bytes
    /// ([`Convert::Length`]), and a shape the split refuses, or too
    /// little `random` ([`slip39_random_len`], the secret's own length
    /// needing less), [`Convert::Refused`]. The passphrase is empty, the
    /// set extendable, the iteration exponent 1; the first two bytes of
    /// `random` are the identifier, the rest the split's. Read back: the
    /// first threshold's worth of members of each of the first
    /// `group_threshold` groups and the last threshold's worth of each of
    /// the last, each recovered with an empty passphrase, give the key's
    /// fingerprint, after every share has been parsed (a share that does
    /// not parse is [`Convert::NotSameWallet`]); anything else is
    /// [`Convert::NotSameWallet`].
    /// The shares are in group, then member order.
    pub fn slip39_converted(
        &self,
        c: &Compute,
        i: usize,
        group_threshold: u8,
        groups: &[slip39::GroupSpec],
        random: &[u8],
        why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Convert> {
        let s = self.convert_source(i, why)?;
        let seed = match (s.words, s.seed) {
            (Some(_), _) => return Err(Convert::Words),
            (None, Some(bytes)) => bytes,
            (None, None) => return Err(Convert::NotGiven),
        };
        if seed.len() != 16 && seed.len() != 32 {
            return Err(Convert::Length);
        }
        let need = slip39_random_len(seed.len(), group_threshold, groups);
        let random = random.get(..need).ok_or(Convert::Refused)?;
        let (id, mut rest) = random.split_at(2);
        let identifier = u16::from_le_bytes([id[0], id[1]]) & 0x7fff;
        let mut fill = |out: &mut [u8]| {
            let (now, after) = rest.split_at(out.len());
            out.copy_from_slice(now);
            rest = after;
        };
        let total: usize = groups.iter().map(|g| usize::from(g.count)).sum();
        let mut shares = Vec::new();
        shares.resize_with(total, Share::default);
        let made = slip39::split(
            c,
            seed,
            b"",
            identifier,
            true,
            1,
            group_threshold,
            groups,
            &mut fill,
            &mut shares,
        )
        .map_err(|_| Convert::Refused)?;
        if made != total {
            return Err(Convert::NotSameWallet);
        }
        let lines: Vec<SecretLine> = shares.iter().map(share_line).collect();
        // Every share parses before any recovery is paid for.
        let parsed: Option<Vec<Share>> = lines.iter().map(|l| Share::parse(l).ok()).collect();
        let parsed = parsed.ok_or(Convert::NotSameWallet)?;
        let recovers = |chosen: &[usize]| {
            let own: Vec<Share> = chosen.iter().map(|&n| parsed[n].duplicate()).collect();
            slip39_seed(c, &own, b"")
                .ok()
                .and_then(|secret| fingerprint_of_seed(c, &secret))
        };
        let (first, last) = first_and_last_quorum(group_threshold, groups);
        let (a, b) = (recovers(&first), recovers(&last));
        if a != b {
            return Err(Convert::NotSameWallet);
        }
        checked(s.master.fingerprint(), a, lines)
    }

    /// Costs what [`Keys::seed_xor`] costs and a parse of each part.
    /// [`Keys::seed_xor`] for key `i`, given only for `Reveal::PaperForm`,
    /// for a key on the English list (another is [`Convert::NotGiven`]),
    /// and only when the parts read back: each part parsed as BIP-39 words
    /// on the key's own list, and their entropies XORed give the key's
    /// entropy on that list, so the same words. Anything else is
    /// [`Convert::NotSameWallet`]; a key with no words is
    /// [`Convert::NotGiven`], and too little randomness is
    /// [`Convert::Refused`].
    pub fn seed_xor_converted(
        &self,
        c: &Compute,
        i: usize,
        parts: u8,
        random: &[u8],
        why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Convert> {
        let s = self.convert_source(i, why)?;
        if !(2..=4).contains(&parts) {
            return Err(Convert::Refused);
        }
        // Seed XOR's parts are restored on the English list only, so a key
        // on another list is not given: the parts would not be the same
        // wallet by what the app says restores them.
        if s.language != Language::English {
            return Err(Convert::NotGiven);
        }
        let m = Mnemonic::parse(c, s.language, s.words.ok_or(Convert::NotGiven)?)
            .map_err(|_| Convert::NotGiven)?;
        let lines = self
            .seed_xor(c, i, parts, random, why)
            .map_err(|e| match e {
                Paper::NoWords | Paper::Words => Convert::NotGiven,
                Paper::Refused(_) | Paper::Failed => Convert::Refused,
            })?
            .release();
        let mut xor = SeedXor::new();
        let mut read_all = lines.len() == usize::from(parts);
        for line in lines.iter() {
            match Mnemonic::parse(c, m.language(), line) {
                Ok(part) => read_all &= xor.push(part.entropy().expose().as_bytes()).is_ok(),
                Err(_) => read_all = false,
            }
        }
        let read = if read_all {
            xor.entropy()
                .ok()
                .and_then(|e| Mnemonic::from_entropy(m.language(), e.as_bytes()).ok())
        } else {
            None
        };
        same_words(&m, read.as_ref(), lines)
    }

    /// Costs what [`Keys::codex32_shares`] costs and a master key.
    /// [`Keys::codex32_shares_avoiding`] for key `i` (`k` 0 and `n` 1 is the secret
    /// itself; the pair is the randomness and the identifiers taken), given only for `Reveal::PaperForm` and only when the set
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
        (random, taken): (&[u8], &[[u8; 4]]),
        why: Reveal,
    ) -> Result<Held<Vec<Zeroizing<String>>>, Convert> {
        let s = self.convert_source(i, why)?;
        let shares = self
            .codex32_shares_avoiding(i, k, n, random, taken, why)
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
        self.source_secrets(i).ok_or(Convert::NotGiven)
    }

    /// A conversion's source: key `i`, or the aezeed held at
    /// [`Keys::AEZEED_SOURCE`], which nothing but a conversion reaches.
    fn source_secrets(&self, i: usize) -> Option<crate::session::Secrets<'_>> {
        if i == Self::AEZEED_SOURCE {
            return self.aezeed.as_ref().map(|a| a.secrets());
        }
        self.secrets(i)
    }
}

/// One line of secret text: a share, written by hand.
pub type SecretLine = Zeroizing<String>;

/// A SLIP-39 share's words, single spaces between.
pub(crate) fn share_line(share: &Share) -> SecretLine {
    let mut text = faraday_intake::secret_buf::room();
    for n in 0..share.word_count() {
        if n > 0 {
            text.push(' ');
        }
        text.push_str(share.word(n));
    }
    text
}

/// Among a set's shares in group, then member order, a first and a last
/// qualifying subset: the first threshold's worth of members of each of
/// the first `group_threshold` groups, and the last threshold's worth of
/// each of the last. For a shape the split took.
pub(crate) fn first_and_last_quorum(
    group_threshold: u8,
    groups: &[slip39::GroupSpec],
) -> (Vec<usize>, Vec<usize>) {
    let g = usize::from(group_threshold);
    // Where each group's members start among the shares.
    let mut starts = Vec::with_capacity(groups.len());
    let mut at = 0;
    for spec in groups {
        starts.push(at);
        at += usize::from(spec.count);
    }
    let first = (0..g)
        .flat_map(|gi| starts[gi]..starts[gi] + usize::from(groups[gi].threshold))
        .collect();
    let last = (groups.len() - g..groups.len())
        .flat_map(|gi| {
            let end = starts[gi] + usize::from(groups[gi].count);
            end - usize::from(groups[gi].threshold)..end
        })
        .collect();
    (first, last)
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

/// `result`, held, when what it read back as is `m`: the same list and the
/// same entropy. Anything else gives [`Convert::NotSameWallet`], as
/// [`checked`].
fn same_words<T>(m: &Mnemonic, read: Option<&Mnemonic>, result: T) -> Result<Held<T>, Convert> {
    let same = read.is_some_and(|r| {
        // Lengths are not secret; the bytes are compared in constant time.
        let (a, b) = (r.entropy(), m.entropy());
        let (a, b) = (a.expose().as_bytes(), b.expose().as_bytes());
        r.language() == m.language() && a.len() == b.len() && bool::from(a.ct_eq(b))
    });
    if same {
        Ok(Held::of(result))
    } else {
        Err(Convert::NotSameWallet)
    }
}

/// `m` written word by word in `form`: [`WordsForm::Numbers`] a line each,
/// [`WordsForm::Words`] and [`WordsForm::Starts`] a space between.
fn written_by_word(m: &Mnemonic, form: WordsForm) -> Zeroizing<String> {
    use core::fmt::Write as _;
    // The room is a phrase of the longest list's words (Japanese or Korean
    // go over it): the buffer is sized to what is written, so it never moves.
    let words: usize = m
        .indices()
        .iter()
        .map(|&n| m.language().word_display(n).len() + 4)
        .sum();
    let mut text = Zeroizing::new(String::with_capacity(words.max(faraday_intake::ROOM)));
    for (place, &n) in m.indices().iter().enumerate() {
        if place > 0 {
            text.push(if form == WordsForm::Numbers {
                '\n'
            } else {
                ' '
            });
        }
        let word = m.language().word_display(n);
        match form {
            WordsForm::Numbers => {
                let _ = write!(text, "{} ({n})", n + 1);
            }
            WordsForm::Starts => text.extend(word.chars().take(4)),
            _ => text.push_str(word),
        }
    }
    text
}

/// [`WordsForm::Numbers`] read back: the list's indices, when each line's
/// two numbers agree.
fn numbers_read(text: &str) -> Option<Zeroizing<Vec<u16>>> {
    let mut out = Zeroizing::new(Vec::with_capacity(24));
    for line in text.lines() {
        let (one, zero) = line.split_once(' ')?;
        let one: u16 = one.parse().ok()?;
        let zero: u16 = zero.strip_prefix('(')?.strip_suffix(')')?.parse().ok()?;
        (one.checked_sub(1)? == zero).then_some(())?;
        out.push(zero);
    }
    Some(out)
}

/// [`WordsForm::Starts`] read back: for each start, the index of the one
/// word on the English list that it is, or that begins with it if it has
/// its full four letters.
fn starts_read(text: &str) -> Option<Zeroizing<Vec<u16>>> {
    let mut out = Zeroizing::new(Vec::with_capacity(24));
    for start in text.split(' ') {
        let mut found = (0..2048u16).filter(|&n| {
            let word = Language::English.word(n);
            // Lengths are not secret; the bytes are compared in constant
            // time, the whole word or its four-letter start.
            let whole =
                word.len() == start.len() && bool::from(word.as_bytes().ct_eq(start.as_bytes()));
            let prefix = start.len() == 4
                && word
                    .as_bytes()
                    .get(..4)
                    .is_some_and(|w| bool::from(w.ct_eq(start.as_bytes())));
            whole | prefix
        });
        let n = found.next()?;
        found.next().is_none().then_some(())?;
        out.push(n);
    }
    Some(out)
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
