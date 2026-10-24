//! FROST shares and dealing. A 24-word key is also a FROST share: its 32
//! bytes of entropy read as the share's scalar. Dealing is compiled out
//! of the app with FROST (`unreleased`, F109).

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::secp256k1::{PublicKey, Secp256k1};
use faraday_bip::frost::SecShare;
#[cfg(feature = "unreleased")]
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use zeroize::Zeroize;
#[cfg(feature = "unreleased")]
use zeroize::Zeroizing;

/// The FROST share 24 words are, when they are 24 words.
pub(crate) fn secret_share(m: &Mnemonic) -> Option<SecShare> {
    if m.word_count() != 24 {
        return None;
    }
    let entropy = m.entropy();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(entropy.expose().as_bytes());
    let share = SecShare::from_bytes(&bytes).ok();
    bytes.zeroize();
    share
}

/// Costs an elliptic-curve multiplication. The public share 24 words
/// are, when they are 24 words.
pub fn public_share(m: &Mnemonic) -> Option<PublicKey> {
    let secp = Secp256k1::new();
    secret_share(m).map(|s| s.public_share(&secp))
}

/// Costs a parse of the words. Runs `take` on the secret share `words`
/// (in `language`) are, rebuilt for the call and wiped after it; it runs
/// on nothing when there are no words or they are not 24 words.
pub(crate) fn with_secret_share(
    c: &Compute,
    language: Language,
    words: Option<&str>,
    take: &mut dyn FnMut(&SecShare),
) {
    if let Some(share) = words
        .and_then(|w| Mnemonic::parse(c, language, w).ok())
        .and_then(|m| secret_share(&m))
    {
        take(&share);
    }
}

#[cfg(feature = "unreleased")]
/// A key to deal from: its words and their list.
pub(crate) struct Dealer<'a> {
    /// The BIP-39 list the words are from.
    pub language: Language,
    /// The words, when the key has them.
    pub words: Option<&'a str>,
}

#[cfg(feature = "unreleased")]
/// Why a deal was not made.
#[derive(Debug, PartialEq, Eq)]
pub enum DealError {
    /// A chosen key has no share to deal from.
    NoShare,
    /// The deal refused the shares, with its own reason.
    Refused(String),
}

#[cfg(feature = "unreleased")]
/// What a deal made: the public record, and the new shares, which stay
/// inside this crate until [`Keys::add_dealt`](crate::Keys::add_dealt)
/// loads them as keys. Neither `Clone` nor `Debug`.
pub struct Dealt {
    /// The threshold record, as text: the group key and every share's
    /// public part, for the network dealt on.
    pub record: String,
    /// The new shares, as 24 words each, by identifier (counting from 0):
    /// the ones past the chosen keys', which the person writes down.
    pub(crate) words: Vec<(usize, Zeroizing<String>)>,
}

#[cfg(feature = "unreleased")]
impl Dealt {
    /// The identifiers (counting from 0) of the new shares.
    pub fn new_shares(&self) -> Vec<usize> {
        self.words.iter().map(|(id, _)| *id).collect()
    }
}

#[cfg(feature = "unreleased")]
/// Costs a parse of the words per dealer, and the deal's elliptic-curve
/// work. Deals an `m`-of-`n` threshold wallet from the shares `dealers`
/// are, in order as identifiers 0, 1…: the other shares and the group
/// key are computed.
pub(crate) fn make_group(
    c: &Compute,
    dealers: &[Dealer<'_>],
    n: usize,
    m: usize,
    network: Network,
) -> Result<Dealt, DealError> {
    let secp = Secp256k1::new();
    let mut chosen = Vec::new();
    for (id, dealer) in dealers.iter().enumerate() {
        let mut got = None;
        with_secret_share(c, dealer.language, dealer.words, &mut |s| {
            let mut b = s.secret_bytes();
            got = SecShare::from_bytes(&b).ok();
            b.zeroize();
        });
        chosen.push((id as u32, got.ok_or(DealError::NoShare)?));
    }
    let dealt = faraday_bip::frost::deal(&secp, n, m, &chosen)
        .map_err(|e| DealError::Refused(format!("{e:?}")))?;
    let record = faraday_bip::threshold::ThresholdRecord::new(dealt.info.clone(), network.kind());
    let mut words = Vec::new();
    for (id, share) in dealt.shares.iter().enumerate().skip(m) {
        let mut bytes = share.secret_bytes();
        let made = Mnemonic::from_entropy(Language::English, &bytes).map(|mn| {
            Zeroizing::new(
                mn.indices()
                    .iter()
                    .map(|w| Language::English.word(*w))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        });
        bytes.zeroize();
        if let Ok(w) = made {
            words.push((id, w));
        }
    }
    Ok(Dealt {
        record: record.to_text(),
        words,
    })
}
