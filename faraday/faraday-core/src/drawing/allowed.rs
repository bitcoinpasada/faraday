//! The owner's exceptions, on [`View`]: costly work drawing does outside
//! the memo and the state, each a named method with why it is not kept
//! (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4). One entry today,
//! [`Allowed::SecretShown`]: a secret the person opened to see or copy,
//! drawn from its source each frame because nothing secret is kept, not
//! even for a frame. A new exception is a new method here, which the
//! owner decides. Each is handed the view's token (`View::allowed`).

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_codec::qr::QrMatrix;
use faraday_keys::Held;
use faraday_keys::vault::RecordView;
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
    /// A key's words or passphrase, lent for this frame to be drawn
    /// ([`faraday_keys::Keys::reveal`]; `docs/KEYS.md` §3). A borrow of
    /// the store's own text: nothing is copied, and nothing costs.
    pub fn reveal(&self, i: usize, why: faraday_keys::Reveal) -> faraday_keys::Revealed<'_> {
        self.app.session.keys.reveal(i, why)
    }

    /// The BIP-85 child shown, derived for this frame and wiped with it
    /// ([`crate::Faraday::bip85_value`]; [`Allowed::SecretShown`]). Costs a
    /// key derivation.
    pub fn bip85_shown(&self) -> Option<faraday_keys::Held<Zeroizing<String>>> {
        self.allowed(Allowed::SecretShown, |c| self.app.bip85_value(c))
    }

    /// A vault key record's words, read for this frame to be shown
    /// ([`faraday_keys::vault::OpenVaults::words_of`];
    /// [`Allowed::SecretShown`]). Costs a parse of the words.
    pub fn record_words_shown(&self, r: RecordView<'_>) -> Option<Held<Mnemonic>> {
        self.allowed(Allowed::SecretShown, |c| {
            self.app.vaults.store().words_of(
                c,
                r,
                faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Words),
            )
        })
    }

    /// A secret field of a vault record, lent for this frame to be drawn
    /// ([`faraday_keys::vault::OpenVaults::reveal`];
    /// [`Allowed::SecretShown`]): an entry's password, TOTP secret or
    /// notes, a note's text. Costs nothing but, for a key's words, a parse
    /// of them.
    pub fn reveal_record(
        &self,
        r: RecordView<'_>,
        number: u8,
        why: faraday_keys::Reveal,
    ) -> faraday_keys::Revealed<'_> {
        let Some((id, i)) = r.home() else {
            return faraday_keys::Revealed::empty();
        };
        self.allowed(Allowed::SecretShown, |c| {
            self.app.vaults.store().reveal(c, id, i, number, why)
        })
    }

    /// The code vault entry `r` gives at `now`, and the account its
    /// secret names ([`faraday_keys::vault::OpenVaults::totp`];
    /// [`Allowed::SecretShown`]): drawn from the secret each frame, as it
    /// changes with the clock. Cheap: one HMAC.
    pub fn entry_code(
        &self,
        r: RecordView<'_>,
        now: Option<u64>,
    ) -> Result<Option<faraday_keys::totp::Shown>, faraday_keys::totp::NotTotp> {
        let Some((id, i)) = r.home() else {
            return Err(faraday_keys::totp::NotTotp::Unreadable);
        };
        self.allowed(Allowed::SecretShown, |_| {
            match self.app.vaults.store().totp(
                id,
                i,
                now,
                faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Totp),
            ) {
                Some(t) => t.code,
                None => Err(faraday_keys::totp::NotTotp::Unreadable),
            }
        })
    }

    /// The paperkey text of GPG record `r`, made for this frame
    /// ([`faraday_keys::vault::OpenVaults::gpg_paperkey`];
    /// [`Allowed::SecretShown`]), and wiped with it.
    pub fn paperkey_shown(&self, r: RecordView<'_>) -> Option<Held<Zeroizing<String>>> {
        self.allowed(Allowed::SecretShown, |_| {
            self.app
                .vaults
                .store()
                .gpg_paperkey(r, faraday_keys::Reveal::GpgPaperkey)
        })
    }

    /// A seed's SeedQR grid on the backup's copy page, Compact or
    /// Standard, from its words for this frame
    /// ([`Allowed::SecretShown`]). Costs a parse of the words and a QR
    /// encoding.
    pub fn seedqr_shown(&self, lang: Language, words: &str, compact: bool) -> Option<QrMatrix> {
        self.allowed(Allowed::SecretShown, |c| {
            let mn = crate::wallet::parse_words(c, lang, words).ok()?;
            if compact {
                faraday_codec::seedqr::encode_compact(c, &mn).ok()
            } else {
                faraday_codec::seedqr::encode_seedqr(c, &mn).ok()
            }
        })
    }
}
