//! **Look further** on the address check (`docs/QR.md` §2): an address
//! that is none of the first hundred receive and change addresses of any
//! loaded wallet is looked for to a thousand, then ten thousand, in every
//! loaded wallet (single-key, multisig and miniscript), a tick at a time,
//! with how far it has got and **Stop**.

use faraday_bip::bitcoin::ScriptBuf;
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;

use super::{Account, DEPTHS, Pace};

/// A press on the check's foot while it looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookWay {
    /// Look to the depth at this place in [`DEPTHS`].
    To(u8),
    /// Stop the look under way.
    Stop,
}

/// Where a look is: an address not found to `looked` indices of each
/// chain, and the look beyond it under way.
pub(crate) struct Look {
    /// What is looked for.
    target: ScriptBuf,
    /// How many wallets are looked in.
    pub(crate) wallets: usize,
    /// The standard accounts of loaded keys with no wallet, looked in
    /// beside the wallets: a find's place counts on from the wallets'.
    accounts: Vec<Account>,
    /// Indices of each chain every wallet is known not to hold it in.
    pub(crate) looked: u32,
    /// The look under way: its depth, the next index and its pace.
    under_way: Option<(u32, u32, Pace)>,
    /// Where it was found, once it was: the wallet's place, the chain
    /// and the index.
    pub(crate) found: Option<(usize, bool, u32)>,
}

impl Look {
    /// A miss at the first [`DEPTHS`]' depth in `wallets` wallets and the
    /// `accounts`.
    pub(crate) fn new(target: ScriptBuf, wallets: usize, accounts: Vec<Account>) -> Self {
        Look {
            target,
            wallets,
            accounts,
            looked: DEPTHS[0],
            under_way: None,
            found: None,
        }
    }

    /// How many accounts are looked in beside the wallets.
    pub(crate) fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// The account a find's place is, when it is past the wallets.
    pub(crate) fn account(&self, place: usize) -> Option<&Account> {
        self.accounts.get(place.checked_sub(self.wallets)?)
    }

    /// Whether a look is under way.
    pub(crate) fn running(&self) -> bool {
        self.under_way.is_some()
    }

    /// How far the look under way is, and its depth.
    pub(crate) fn progress(&self) -> Option<(u32, u32)> {
        self.under_way.map(|(depth, next, _)| (next, depth))
    }

    /// The depths still offered: those past what was looked at.
    pub(crate) fn offered(&self) -> impl Iterator<Item = (u8, u32)> + '_ {
        DEPTHS
            .iter()
            .enumerate()
            .filter(|&(_, &d)| d > self.looked)
            .map(|(i, &d)| (i as u8, d))
    }

    /// A press: a look to a depth starts at once, **Stop** ends the one
    /// under way where it is, the indices it did look at kept as looked.
    pub(crate) fn press(&mut self, way: LookWay, now_ms: u64) {
        match way {
            LookWay::To(i) => {
                let Some(&depth) = DEPTHS.get(usize::from(i)) else {
                    return;
                };
                if self.under_way.is_none() && depth > self.looked && self.found.is_none() {
                    self.under_way = Some((depth, self.looked, Pace::new(now_ms)));
                }
            }
            LookWay::Stop => {
                if let Some((_, next, _)) = self.under_way.take() {
                    self.looked = self.looked.max(next);
                }
            }
        }
    }

    /// Costs an address derivation for each index of the tick, on both
    /// chains of every wallet in `policies` and every account. One tick of the look under
    /// way. Returns whether anything changed to draw.
    pub(crate) fn tick(&mut self, c: &Compute, policies: &[&WalletPolicy], now_ms: u64) -> bool {
        let Some((depth, start, mut pace)) = self.under_way else {
            return false;
        };
        let secp = Secp256k1::verification_only();
        // The wallets as they were when the look began, by place: one
        // loaded or taken away since does not move an account's place.
        let policies: Vec<Option<&WalletPolicy>> = (0..self.wallets)
            .map(|w| policies.get(w).copied())
            .chain(self.accounts.iter().map(|a| Some(&a.policy)))
            .collect();
        let end = start.saturating_add(pace.budget()).min(depth);
        let mut next = start;
        'indices: for i in start..end {
            for (w, policy) in policies.iter().enumerate() {
                let Some(policy) = policy else { continue };
                for change in [false, true] {
                    if super::script(c, &secp, policy, change, i).as_ref() == Some(&self.target) {
                        self.found = Some((w, change, i));
                        break 'indices;
                    }
                }
            }
            next = i + 1;
        }
        pace.ticked(next.saturating_sub(start), now_ms);
        if self.found.is_some() {
            self.under_way = None;
        } else if next >= depth {
            self.looked = depth;
            self.under_way = None;
        } else {
            self.under_way = Some((depth, next, pace));
        }
        true
    }
}
