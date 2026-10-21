//! The owner's exceptions, on [`View`]: costly work drawing does outside
//! the memo and the state, each a named method with why it is not kept
//! (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4). One entry today,
//! [`Allowed::SecretShown`]: a secret the person opened to see or copy,
//! drawn from its source each frame because nothing secret is kept, not
//! even for a frame. A new exception is a new method here, which the
//! owner decides.

use faraday_vault::Record;
use osk_bip::bip39::{Language, Mnemonic};
use osk_codec::qr::QrMatrix;
use zeroize::Zeroizing;

use super::View;
use crate::work::{self, Allowed};

impl View<'_> {
    /// The BIP-85 child shown, derived for this frame and wiped with it
    /// ([`crate::Faraday::bip85_value`]; [`Allowed::SecretShown`]). Costs a
    /// key derivation.
    pub fn bip85_shown(&self) -> Option<Zeroizing<String>> {
        work::allowed(Allowed::SecretShown, || self.app.bip85_value())
    }

    /// A vault key record's words, read for this frame to be shown
    /// ([`crate::wallet::record_words`]; [`Allowed::SecretShown`]). Costs
    /// a parse of the words.
    pub fn record_words_shown(&self, r: &Record) -> Option<Mnemonic> {
        work::allowed(Allowed::SecretShown, || crate::wallet::record_words(r))
    }

    /// A seed's SeedQR grid on the backup's copy page, Compact or
    /// Standard, from its words for this frame
    /// ([`Allowed::SecretShown`]). Costs a parse of the words and a QR
    /// encoding.
    pub fn seedqr_shown(&self, lang: Language, words: &str, compact: bool) -> Option<QrMatrix> {
        work::allowed(Allowed::SecretShown, || {
            let mn = crate::wallet::parse_words(lang, words).ok()?;
            work::count(work::Work::QrEncode);
            if compact {
                osk_codec::seedqr::encode_compact(&mn).ok()
            } else {
                osk_codec::seedqr::encode_seedqr(&mn).ok()
            }
        })
    }
}
