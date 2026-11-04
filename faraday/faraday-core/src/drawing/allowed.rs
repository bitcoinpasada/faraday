//! The owner's exceptions, on [`View`]: costly work drawing does outside
//! the memo and the state, each a named method with why it is not kept
//! (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4). One entry today,
//! [`Allowed::SecretShown`], held only for [`View::reveal_record`], which
//! lends a field and works nothing out. The secrets a person opens to
//! see or copy are not exceptions: each is worked out once when shown
//! and held while shown, then dropped and wiped when hidden, on leaving
//! the page and at a lock (F195, `docs/SPEED.md` §2 rule 1), and the
//! methods below that name them (`bip85_held`, `record_words_held`,
//! `paperkey_held`, `entry_code_held`) borrow what is held. A new
//! exception is a new method here, which the owner decides. Each is
//! handed the view's token (`View::allowed`).

use faraday_bip::bip39::Mnemonic;
use faraday_codec::qr::QrMatrix;
use faraday_keys::vault::RecordView;
use zeroize::Zeroizing;

use super::View;

/// Costly work drawing does outside the memo and the state, each named
/// with why it is not kept: the owner's list, each entry a method of
/// [`View`] below. The owner decides each entry (`docs/SPEED.md` §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Allowed {
    /// A secret field of a vault record lent to be drawn while its eye
    /// shows it: a password, a TOTP secret, notes or a note's text. It
    /// is the store's own wiped text, so nothing is worked out; the token
    /// is there because the store's reveal asks for one for a key's
    /// words, which are held while shown and never asked for here (F195,
    /// `docs/SPEED.md` §2 rule 1).
    SecretShown,
}

impl View<'_> {
    /// A key's words or passphrase, lent for this frame to be drawn
    /// ([`faraday_keys::Keys::reveal`]; `docs/KEYS.md` §3). A borrow of
    /// the store's own text: nothing is copied, and nothing costs.
    pub fn reveal(&self, i: usize, why: faraday_keys::Reveal) -> faraday_keys::Revealed<'_> {
        self.app.session.keys.reveal(i, why)
    }

    /// The BIP-85 child shown, held since the eye opened it (F195): a
    /// borrow, nothing costs.
    pub fn bip85_held(&self) -> Option<&faraday_keys::Held<Zeroizing<String>>> {
        self.app.bip85.held()
    }

    /// Convert a seed's value, held since its press: a borrow, nothing
    /// costs.
    pub fn convert_held(&self) -> Option<&crate::convert::Value> {
        self.app.tools.convert().held()
    }

    /// The text Convert's open eye shows, held since the press that
    /// opened it: a borrow, nothing costs.
    pub fn convert_shown_text(&self) -> Option<&str> {
        self.app.tools.convert().shown_text()
    }

    /// The selected key record's words, held since its eye opened them
    /// (F195): a borrow, nothing costs.
    pub fn record_words_held(&self) -> Option<&Mnemonic> {
        self.app.vaults.words_shown(self.app.vault_on_page())
    }

    /// The selected GPG key's paperkey, held since its eye opened it
    /// (F195): a borrow, nothing costs.
    pub fn paperkey_held(&self) -> Option<&str> {
        self.app.vaults.paperkey_shown(self.app.vault_on_page())
    }

    /// What the entry on the page gives for its TOTP secret, held for the
    /// page and made once for each period (F195,
    /// `Faraday::vault_code_refresh`): a borrow, nothing costs.
    pub(crate) fn entry_code_held(&self) -> Option<&crate::vault::CodeHeld> {
        self.app.vaults.code_held(self.app.vault_on_page())
    }

    /// A secret field of a vault record, lent for this frame to be drawn
    /// ([`faraday_keys::vault::OpenVaults::reveal`];
    /// [`Allowed::SecretShown`]): an entry's password, TOTP secret or
    /// notes, a note's text. Costs nothing: a key's words, which the store
    /// would parse, are held while shown and not asked for here.
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

    /// The SeedQR number of the seed shown on the backup's copy page,
    /// word `i`, held since the eye opened it (F195): a read, nothing
    /// costs.
    pub fn backup_word_number(&self, i: usize) -> Option<u16> {
        self.app.backup.shown_number(i)
    }

    /// The seed shown's SeedQR grid on the backup's copy page, Compact or
    /// Standard, held since the eye opened it (F195): a borrow, nothing
    /// costs.
    pub fn backup_seedqr(&self, compact: bool) -> Option<&QrMatrix> {
        self.app.backup.shown_qr(compact)
    }
}
