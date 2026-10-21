//! The owner's exceptions, on [`View`]: costly work drawing does outside
//! the memo and the state, each a named method with why it is not kept
//! (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4). One entry today,
//! [`Allowed::SecretShown`]: a secret the person opened to see or copy,
//! drawn from its source each frame because nothing secret is kept, not
//! even for a frame. A new exception is a new method here, which the
//! owner decides. Each is handed the view's token (`View::allowed`).

use faraday_vault::Record;
use osk_bip::bip39::{Language, Mnemonic};
use osk_codec::qr::QrMatrix;
use zeroize::Zeroizing;

use super::View;

/// Costly work drawing does outside the memo and the state, each named
/// with why it is not kept: the owner's list, each entry a method of
/// [`View`] below. The owner decides each entry (`docs/SPEED.md` §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Allowed {
    /// A secret the person opened to see or to copy by hand, drawn from
    /// its source on each frame: a seed's words parsed for its SeedQR
    /// grid on the backup's copy page, a vault key record's words and
    /// a BIP-85 child shown. Nothing secret is kept, on the state or in
    /// the memo for a frame (§2 rule 1), so there is nothing to keep it
    /// in; it is drawn only while that page or row is open.
    SecretShown,
}

impl View<'_> {
    /// The BIP-85 child shown, derived for this frame and wiped with it
    /// ([`crate::Faraday::bip85_value`]; [`Allowed::SecretShown`]). Costs a
    /// key derivation.
    pub fn bip85_shown(&self) -> Option<Zeroizing<String>> {
        self.allowed(Allowed::SecretShown, |c| self.app.bip85_value(c))
    }

    /// A vault key record's words, read for this frame to be shown
    /// ([`crate::wallet::record_words`]; [`Allowed::SecretShown`]). Costs
    /// a parse of the words.
    pub fn record_words_shown(&self, r: &Record) -> Option<Mnemonic> {
        self.allowed(Allowed::SecretShown, |c| crate::wallet::record_words(c, r))
    }

    /// A seed's SeedQR grid on the backup's copy page, Compact or
    /// Standard, from its words for this frame
    /// ([`Allowed::SecretShown`]). Costs a parse of the words and a QR
    /// encoding.
    pub fn seedqr_shown(&self, lang: Language, words: &str, compact: bool) -> Option<QrMatrix> {
        self.allowed(Allowed::SecretShown, |c| {
            let mn = crate::wallet::parse_words(c, lang, words).ok()?;
            if compact {
                osk_codec::seedqr::encode_compact(c, &mn).ok()
            } else {
                osk_codec::seedqr::encode_seedqr(c, &mn).ok()
            }
        })
    }
}
