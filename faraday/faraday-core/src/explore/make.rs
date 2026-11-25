//! **Make a wallet** at the explored path (`docs/FLOWS.md`, Explore,
//! Make a wallet at the path): the wallet Create would make for a loaded
//! key at a standard single-key account, named after the key and the
//! account, its first address checked against the key's own before it is
//! kept; a wallet already made for that key, account and script is
//! opened instead.

use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use faraday_bip::keys::ScriptType;
use faraday_bip::policy::Template;
use faraday_text::Text;

use super::ExploreArea;
use crate::create::NewKind;
use crate::wallet::Session;

/// What the account at the explored path is offered as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Offer {
    /// **Make a wallet**: none is loaded for it.
    Make,
    /// **Open the wallet**: the loaded wallet `.0` is it.
    Open(usize),
}

/// The kind of single-key wallet whose script is `script`.
pub(super) fn kind_of(script: ScriptType) -> NewKind {
    match script {
        ScriptType::NativeSegwit => NewKind::NativeSegwit,
        ScriptType::NestedSegwit => NewKind::NestedSegwit,
        ScriptType::Legacy => NewKind::Legacy,
        ScriptType::Taproot => NewKind::Taproot,
    }
}

/// Whether `account` is a standard account of `script` on the network
/// whose coin type is `coin`: its purpose, the coin type and an account
/// number, each hardened.
fn standard(account: &DerivationPath, script: ScriptType, coin: u32) -> bool {
    matches!(
        account.as_ref(),
        [
            ChildNumber::Hardened { index: p },
            ChildNumber::Hardened { index: c },
            ChildNumber::Hardened { .. },
        ] if *p == script.purpose() && *c == coin
    )
}

/// The loaded wallet that is the single key of fingerprint `fp` at
/// `account` under `script`, whatever its name and source.
fn loaded_wallet(
    session: &Session,
    fp: [u8; 4],
    account: &DerivationPath,
    script: ScriptType,
) -> Option<usize> {
    session.wallets.iter().position(|w| {
        let p = w.policy();
        let [key] = p.keys() else {
            return false;
        };
        matches!(p.template(), Template::Single { .. })
            && p.script_type() == script
            && key.fingerprint().is_some_and(|f| f.0 == fp)
            && key.path() == Some(account)
    })
}

impl ExploreArea {
    /// What the explored path's account is offered as, or `None` where it
    /// is not a standard single-key account of the network's coin type
    /// under a loaded key. Comparisons only.
    pub(crate) fn make_offer(&self, session: &Session) -> Option<Offer> {
        let (Some(fp), account, script, _, true) = self.account(session)? else {
            return None;
        };
        if !standard(&account, script, session.network().coin_type()) {
            return None;
        }
        Some(match loaded_wallet(session, fp, &account, script) {
            Some(i) => Offer::Open(i),
            None => Offer::Make,
        })
    }

    /// Notes the key of the wallet just made when it is a BIP-85 child
    /// seed.
    pub(super) fn set_child(&mut self, child: Option<[u8; 4]>) {
        self.child = child;
    }

    /// The child seed the notice is about, until it is shown.
    pub(crate) fn child(&self) -> Option<[u8; 4]> {
        self.child
    }

    /// Takes the child seed the notice is about: shown once.
    pub(super) fn take_child(&mut self) -> Option<[u8; 4]> {
        self.child.take()
    }

    /// Clears the line a refused press left.
    pub(crate) fn clear_make_error(&mut self) {
        if let Some(e) = self.state.as_mut() {
            e.make_error = None;
        }
    }

    /// Sets the wallet made, whose dialog is open, or the line a refused
    /// press leaves.
    pub(super) fn set_made(&mut self, made: Result<usize, Text>) {
        if let Some(e) = self.state.as_mut() {
            match made {
                Ok(i) => e.made = Some(i),
                Err(t) => e.make_error = Some(t),
            }
        }
    }

    /// Wallet `gone` left the session list: the wallet made moves up with
    /// it, and is none when it is the one gone.
    pub(crate) fn wallet_left(&mut self, gone: usize) {
        self.held_for = self
            .held_for
            .and_then(|h| crate::wallet::place_after_removal(h, gone));
        if let Some(e) = self.state.as_mut() {
            e.made = e
                .made
                .and_then(|m| crate::wallet::place_after_removal(m, gone));
        }
    }
}
