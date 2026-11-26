//! A Coldcard export holding a single-sig account and a silent payments
//! account (`bip352`, `docs/INTEROP.md` §5, BIP-392 sp()): which of the two loads is
//! the person's choice (owner, 2026-10-29). The export waits here, both
//! wallets read, while the account dialog asks; the choice is kept for
//! the session, so the press that asked runs again and loads the one
//! chosen. Add to vault asks each time (owner, 2026-11-25): its choice
//! is for that press run again and is not kept.

use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;

use super::{Origin, Refusal, Session, same_wallet};
use crate::Action;
use crate::vault::VaultAction;

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

impl AccountAsk {
    /// Costs what [`super::read_wallet`] does, an elliptic-curve
    /// multiplication and the export read again. Whether `text` is the
    /// export asked about: both its wallets.
    pub(crate) fn is_export(&self, c: &Compute, text: &str) -> bool {
        super::read_wallet(c, text).is_ok_and(|read| read == self.single)
            && matches!(
                faraday_bip::coldcard::silent(c, text),
                Some(Ok(silent)) if WalletPolicy::of_silent(silent.clone()) == self.silent
            )
    }
}

/// The export waiting, and the accounts chosen for a load in this
/// session, by the single-sig wallet's descriptor ([`same_wallet`]).
#[derive(Default)]
pub struct Accounts {
    /// The export waiting for its account, if one does.
    asked: Option<Box<AccountAsk>>,
    /// The choices made, by the single-sig account's descriptor.
    chosen: Vec<(faraday_text::Text, Account)>,
    /// The account just chosen for Add to vault, held only while that
    /// press runs again.
    adding: Option<Account>,
}

impl Session {
    /// Costs a JSON parse of a Coldcard export and, for one with a
    /// silent payments account beside a single-sig one, an
    /// elliptic-curve multiplication and the export read again. The
    /// wallet `read` from `text` stands for, unless `text` is a Coldcard
    /// export with both accounts: then the one chosen, or, with none
    /// chosen, the export waits for the account dialog and is refused
    /// for now. The choice is the session's for a load; `add` is `Some`
    /// for Add to vault, and holds the one chosen for this very press.
    pub(super) fn account_of(
        &mut self,
        c: &Compute,
        text: &str,
        read: WalletPolicy,
        (name, source, origin): (&str, &str, Origin),
        add: Option<Option<Account>>,
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
        let chosen = add.unwrap_or_else(|| {
            let kept = self.accounts.chosen.iter().find(|(k, _)| *k == key);
            kept.map(|(_, account)| *account)
        });
        match chosen {
            Some(Account::SingleSig) => return Ok(read),
            Some(Account::Silent) => return Ok(WalletPolicy::of_silent(silent)),
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

    /// Costs what [`super::read_wallet`] and [`Self::account_of`] do:
    /// the wallet of file text `text`, for a press that keeps it without
    /// loading it (Add to vault): an export of two accounts waits for
    /// the dialog each time, whatever was chosen before. `name` and
    /// `source` are as in [`Self::add_wallet`].
    pub(crate) fn read_wallet_to_add(
        &mut self,
        c: &Compute,
        text: &str,
        (name, source): (&str, &str),
    ) -> Result<WalletPolicy, Refusal> {
        let read = super::read_wallet(c, text)?;
        let name = super::safe_name(c, name);
        let add = Some(self.accounts.adding.take());
        self.account_of(c, text, read, (&name, source, Origin::Named), add)
    }

    /// Whether `text` is a Coldcard export of a single-sig and a silent
    /// payments account whose account is not settled as the single-sig
    /// one: the wallet taken from it may be either. Costs what
    /// [`Self::account_of`] does.
    pub(crate) fn account_may_differ(&self, c: &Compute, text: &str) -> bool {
        if !faraday_bip::coldcard::looks_like_export(text) {
            return false;
        }
        let Some(Ok(_)) = faraday_bip::coldcard::silent(c, text) else {
            return false;
        };
        let Ok(read) = super::read_wallet(c, text) else {
            return true;
        };
        !matches!(
            self.accounts
                .chosen
                .iter()
                .find(|(k, _)| *k == same_wallet(&read)),
            Some((_, Account::SingleSig))
        )
    }

    /// The export waiting for its account to be chosen, if one is.
    pub(crate) fn account_asked(&self) -> Option<&AccountAsk> {
        self.accounts.asked.as_deref()
    }

    /// The press `action` asked for the waiting export, if one waits and
    /// no press is noted for it yet.
    pub(crate) fn account_replay(&mut self, action: Action, inbox: u64) {
        if let Some(ask) = self.accounts.asked.as_mut() {
            ask.replay.get_or_insert((action, inbox));
        }
    }

    /// Holds `account` for the Add to vault press about to run again,
    /// and `None` once it has run.
    pub(crate) fn account_for_add(&mut self, account: Option<Account>) {
        self.accounts.adding = account;
    }

    /// The person chose `account` for the waiting export, which is
    /// handed back: a load's choice is kept for the session, Add to
    /// vault's is not. `None` when none waits.
    pub(crate) fn account_choose(&mut self, account: Account) -> Option<AccountAsk> {
        let ask = self.accounts.asked.take()?;
        if !matches!(
            ask.replay,
            Some((Action::Vault(VaultAction::AddFile(_)), _))
        ) {
            self.accounts
                .chosen
                .push((same_wallet(&ask.single), account));
        }
        Some(*ask)
    }

    /// The waiting export is not added.
    pub(crate) fn account_drop(&mut self) {
        self.accounts.asked = None;
    }
}
