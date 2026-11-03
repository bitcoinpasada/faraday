//! The seed shown on the backup's copy page, held while it is shown
//! (F195).

use super::WalletBackupArea;
use faraday_bip::bip39::Language;
use faraday_crypto::work::Compute;
use faraday_keys::Keys;

/// The seed shown on the copy page, worked out once when the eye opened
/// it (F195): each word's SeedQR number and the SeedQR grid. Wiped on
/// drop (the numbers in a `Zeroizing`, the grid zeroized by its type),
/// dropped the moment the seed is hidden, the page is left or the
/// session locks; drawing only reads it.
pub(super) struct SeedShown {
    /// The key shown, by index into the session.
    key: usize,
    /// Each word's number, `u16::MAX` where the list has no such word.
    numbers: zeroize::Zeroizing<Vec<u16>>,
    /// The SeedQR grid and whether it is Compact, when the page draws one.
    qr: Option<(bool, faraday_codec::qr::QrMatrix)>,
}

impl WalletBackupArea {
    /// The SeedQR number of shown word `i`; none when no seed is shown.
    pub(crate) fn shown_number(&self, i: usize) -> Option<u16> {
        let n = *self.shown.as_ref()?.numbers.get(i)?;
        (n != u16::MAX).then_some(n)
    }

    /// The shown seed's SeedQR grid, when it is held and is `compact`.
    pub(crate) fn shown_qr(&self, compact: bool) -> Option<&faraday_codec::qr::QrMatrix> {
        match &self.shown.as_ref()?.qr {
            Some((c, m)) if *c == compact => Some(m),
            _ => None,
        }
    }

    /// Whether a seed is held for drawing.
    #[cfg(test)]
    pub(crate) fn shown_held(&self) -> bool {
        self.shown.is_some()
    }

    /// Drops the held seed, wiped.
    pub(super) fn drop_shown(&mut self) {
        self.shown = None;
    }

    /// Costs a scan of the word list for each word when `k`'s numbers are
    /// not held yet, and a parse of the words and a QR encoding when the
    /// grid (`seedqr`) is not held in this `compact`ness. The seed `k`
    /// has just been shown; nothing when it is held already.
    pub(super) fn hold(
        &mut self,
        c: &Compute,
        keys: &Keys,
        k: usize,
        lang: Language,
        compact: bool,
        seedqr: bool,
    ) {
        let held = self.shown.as_ref().filter(|h| h.key == k);
        let has_qr =
            held.is_some_and(|h| h.qr.as_ref().map(|(m, _)| *m) == seedqr.then_some(compact));
        if has_qr {
            return;
        }
        let numbers = match self.shown.take().filter(|h| h.key == k) {
            Some(h) => h.numbers,
            None => {
                let words = keys.reveal(k, faraday_keys::Reveal::CopyByHand);
                zeroize::Zeroizing::new(
                    words
                        .split_whitespace()
                        .map(|w| lang.index_of(w).unwrap_or(u16::MAX))
                        .collect(),
                )
            }
        };
        let qr = seedqr
            .then(|| {
                let words = keys.reveal(k, faraday_keys::Reveal::SeedQr);
                let mn = crate::wallet::parse_words(c, lang, &words).ok()?;
                let m = if compact {
                    faraday_codec::seedqr::encode_compact(c, &mn)
                } else {
                    faraday_codec::seedqr::encode_seedqr(c, &mn)
                };
                m.ok().map(|m| (compact, m))
            })
            .flatten();
        self.shown = Some(SeedShown {
            key: k,
            numbers,
            qr,
        });
    }
}
