//! The name of the person's wallet an output of a spend is verified as
//! an address of (F254): the Review and the spend panel say which wallet
//! a change or a transfer goes to.

use faraday_psbt::OwnWallet;
use faraday_text::{Text, text};

use super::{Session, fp_text, key_line};
use crate::strings::EN;

impl Session {
    /// The name `own` goes by: a registered wallet's name; a loaded
    /// key's account, the loaded single-key wallet at that account when
    /// there is one, otherwise the key's label and the account's kind.
    /// `None` where `own` is no wallet loaded. A lookup.
    pub(crate) fn own_wallet_name(&self, own: &OwnWallet) -> Option<Text> {
        match own {
            OwnWallet::Registered(i) => self.wallets.get(*i).map(|w| Text::of(&w.name)),
            OwnWallet::Account { key, path } => {
                // The wallet the core names that account by (the same rule).
                let at_account = self
                    .wallets
                    .iter()
                    .find(|w| OwnWallet::account_of_wallet(w.policy()).as_ref() == Some(own));
                if let Some(w) = at_account {
                    return Some(Text::of(&w.name));
                }
                let holder = match self.keys.key_label(*key) {
                    Some(label) => key_line(*key, label),
                    None => fp_text(*key),
                };
                // The purpose of the account's path, hardened.
                const H: u32 = 0x8000_0000;
                let kind = match path.as_ref().first().map(|n| u32::from(*n)) {
                    Some(n) if n == H | 44 => EN.explore_legacy,
                    Some(n) if n == H | 49 => EN.explore_nested_segwit,
                    Some(n) if n == H | 84 => EN.explore_native_segwit,
                    Some(n) if n == H | 86 => EN.explore_taproot,
                    _ => return Some(holder),
                };
                Some(text!("{} · {kind}", &*holder))
            }
        }
    }
}
