//! A Coldcard export holding a single-sig account and a silent payments
//! account (`bip352`, `docs/INTEROP.md` §5 I9): which of the two loads is
//! the person's choice (owner, 2026-10-29). The export waits here, both
//! wallets read, while the account dialog asks; the choice is kept for
//! the session, so the press that asked runs again and loads the one
//! chosen.

use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;

use super::{Origin, Refusal, Session, same_wallet};
use crate::Action;

/// Which of an export's two accounts loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Account {
    /// Its single-sig account: BIP-84's, else 86's, 49's, 44's.
    SingleSig,
    /// Its silent payments account, BIP-352's.
    Silent,
}

/// An export waiting for its account to be chosen: both wallets, read
/// once, and what the dialog names them by.
pub struct AccountAsk {
    /// The name it is added under.
    pub name: String,
    /// The single-sig account's wallet.
    pub single: WalletPolicy,
    /// The single-sig account's BIP, "BIP-84", as the dialog names it.
    pub single_bip: String,
    /// The silent payments account's wallet.
    pub silent: WalletPolicy,
    /// Where it came from, as [`super::Wallet::source`] has it.
    pub source: String,
    /// What that source is.
    pub origin: Origin,
    /// The press that asked for it, run again once an account is chosen,
    /// with the Inbox's names and sizes as they were then
    /// ([`crate::Faraday::inbox_print`]); `None` for one that came in
    /// without such a press, which the choice adds itself.
    pub replay: Option<(Action, u64)>,
}

/// The export waiting, and the accounts chosen in this session, by the
/// single-sig wallet's descriptor ([`same_wallet`]).
#[derive(Default)]
pub struct Accounts {
    /// The export waiting for its account, if one does.
    asked: Option<Box<AccountAsk>>,
    /// The choices made, by the single-sig account's descriptor.
    chosen: Vec<(String, Account)>,
}

impl Session {
    /// Costs a JSON parse of a Coldcard export and, for one with a
    /// silent payments account beside a single-sig one, an
    /// elliptic-curve multiplication and the export read again. The
    /// wallet `read` from `text` stands for, unless `text` is a Coldcard
    /// export with both accounts: then the one chosen in this session,
    /// or, with none chosen, the export waits for the account dialog and
    /// is refused for now.
    pub(super) fn account_of(
        &mut self,
        c: &Compute,
        text: &str,
        read: WalletPolicy,
        (name, source, origin): (&str, &str, Origin),
    ) -> Result<WalletPolicy, Refusal> {
        if read.silent().is_some() || !faraday_bip::coldcard::looks_like_export(text) {
            return Ok(read);
        }
        // A silent payments account refused beside a single-sig one
        // leaves the single-sig one, as before it was read.
        let Some(Ok(silent)) = faraday_bip::coldcard::silent(c, text) else {
            return Ok(read);
        };
        let key = same_wallet(&read);
        match self.accounts.chosen.iter().find(|(k, _)| *k == key) {
            Some((_, Account::SingleSig)) => return Ok(read),
            Some((_, Account::Silent)) => return Ok(WalletPolicy::of_silent(silent)),
            None => {}
        }
        if self.accounts.asked.is_none() {
            let field = faraday_bip::coldcard::read(c, text).map_or("", |(_, f)| f);
            self.accounts.asked = Some(Box::new(AccountAsk {
                name: name.to_string(),
                single: read,
                single_bip: format!("BIP-{}", field.trim_start_matches("bip")),
                silent: WalletPolicy::of_silent(silent),
                source: source.to_string(),
                origin,
                replay: None,
            }));
        }
        Err(Refusal::AccountUnchosen)
    }

    /// The export waiting for its account to be chosen, if one is.
    pub fn account_asked(&self) -> Option<&AccountAsk> {
        self.accounts.asked.as_deref()
    }

    /// The press `action` asked for the waiting export, if one waits and
    /// no press is noted for it yet.
    pub fn account_replay(&mut self, action: Action, inbox: u64) {
        if let Some(ask) = self.accounts.asked.as_mut() {
            ask.replay.get_or_insert((action, inbox));
        }
    }

    /// The person chose `account` for the waiting export: kept for the
    /// session, and the export handed back. `None` when none waits.
    pub fn account_choose(&mut self, account: Account) -> Option<AccountAsk> {
        let ask = self.accounts.asked.take()?;
        self.accounts
            .chosen
            .push((same_wallet(&ask.single), account));
        Some(*ask)
    }

    /// The waiting export is not added.
    pub fn account_drop(&mut self) {
        self.accounts.asked = None;
    }
}
