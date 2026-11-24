//! **Look further** on the address check (`docs/QR.md` §2): an address
//! that is none of the first hundred receive and change addresses of any
//! loaded wallet is looked for to a thousand, then ten thousand, in every
//! loaded wallet (single-key, multisig and miniscript), a tick at a time,
//! with how far it has got and **Stop**.

use faraday_bip::bitcoin::ScriptBuf;
use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_bip::keys::{Fingerprint, ScriptType};
use faraday_bip::policy::{WalletId, WalletPolicy};
use faraday_crypto::work::Compute;
use faraday_text::{Text, text};

use crate::wallet::{Session, Wallet};

use super::{Account, DEPTHS, Pace};

/// A press on the check's foot while it looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookWay {
    /// Look to the depth at this place in [`DEPTHS`].
    To(u8),
    /// Stop the look under way.
    Stop,
}

/// What a find is in, worked out once at the find from the wallets then
/// loaded, so the card is drawn from it and never from the wallets.
#[derive(Clone)]
pub(crate) enum FoundIn {
    /// A loaded wallet: its name, its shape and a line for each of its
    /// keys, the key's fingerprint and its full path to the address.
    Wallet {
        /// Its place among the loaded wallets, which Open the wallet takes.
        place: usize,
        name: Text,
        shape: Text,
        keys: Vec<Text>,
        /// What Sign a message takes of the address: a single-key wallet
        /// whose key is loaded.
        sign: Option<SignAt>,
    },
    /// A loaded key's standard account: the key's name, the account's
    /// script, number and where a wallet of it is made, and the one line
    /// of the key and its full path to the address.
    Account {
        label: Text,
        script: ScriptType,
        number: u32,
        line: Text,
        at: (Fingerprint, DerivationPath),
        /// What Sign a message takes of the address: the key is loaded.
        sign: Option<SignAt>,
    },
}

/// What Sign a message takes of a find.
#[derive(Clone)]
pub(crate) enum SignAt {
    /// Receive address `index` of the single-key wallet at this place:
    /// Sign a message opens on that wallet, with -/+ still stepping.
    Wallet(usize, u32),
    /// Any other address of a loaded key: Sign a message on its path.
    Key(crate::wallet::MessageAt),
}

impl FoundIn {
    /// What Sign a message takes of the find, if it offers it.
    pub(crate) fn sign(&self) -> Option<&SignAt> {
        match self {
            FoundIn::Wallet { sign, .. } | FoundIn::Account { sign, .. } => sign.as_ref(),
        }
    }
}

/// The path of address `index` on a chain below `base`.
fn leaf(base: &DerivationPath, change: bool, index: u32) -> Option<DerivationPath> {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    Some(
        base.child(ChildNumber::from_normal_idx(u32::from(change)).ok()?)
            .child(ChildNumber::from_normal_idx(index).ok()?),
    )
}

/// "m/84h/0h/0h/0/12": an account's path, the chain and the index.
fn full_path(account: &DerivationPath, change: bool, index: u32) -> Text {
    Text::replaced(&text!("m/{account}/{}/{index}", u8::from(change)), "'", "h")
}

/// Where a look is: an address not found to `looked` indices of each
/// chain, and the look beyond it under way.
pub(crate) struct Look {
    /// What is looked for.
    target: ScriptBuf,
    /// The address looked for, as it is shown.
    pub(crate) address: Text,
    /// The one wallet's name, when exactly one wallet is looked in.
    pub(crate) only: Option<Text>,
    /// How many wallets are looked in.
    pub(crate) wallets: usize,
    /// The ids of those wallets' policies, by place: what tells
    /// that the loaded wallets are still the ones looked in.
    loaded: Vec<WalletId>,
    /// Whether the loaded wallets changed since the look began: it looks
    /// no further, since a place may hold another wallet.
    ended: bool,
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
    /// What it was found in, as it was at the find.
    pub(crate) found_in: Option<FoundIn>,
    /// The place of the wallet made from a key's account it was found in.
    pub(crate) made: Option<usize>,
    /// Whether the address is one of a file's several, whose finds offer
    /// no wallet to make.
    pub(crate) listed: bool,
}

impl Look {
    /// A miss at the first [`DEPTHS`]' depth in the `wallets` and the
    /// `accounts`.
    pub(crate) fn new(
        (address, target): (Text, ScriptBuf),
        wallets: &[Wallet],
        accounts: Vec<Account>,
    ) -> Self {
        Look {
            target,
            address,
            only: (wallets.len() == 1).then(|| wallets[0].name.clone()),
            wallets: wallets.len(),
            loaded: wallets.iter().map(Wallet::id).collect(),
            ended: false,
            accounts,
            looked: DEPTHS[0],
            under_way: None,
            found: None,
            found_in: None,
            made: None,
            listed: false,
        }
    }

    /// Ends the look when `wallets` are no longer the ones it began in,
    /// the indices it did look at kept as looked of those. A find stays.
    pub(crate) fn sync(&mut self, wallets: &[Wallet]) {
        if !self.ended
            && (wallets.len() != self.loaded.len()
                || wallets
                    .iter()
                    .zip(&self.loaded)
                    .any(|(w, id)| w.id() != *id))
        {
            self.ended = true;
            if let Some((_, next, _)) = self.under_way.take() {
                self.looked = self.looked.max(next);
            }
        }
    }

    /// Whether the wallets changed and the look ended with them.
    pub(crate) fn ended(&self) -> bool {
        self.ended && self.found.is_none()
    }

    /// Keeps what a find is in, from `session` as it is at the find.
    pub(crate) fn settle(&mut self, session: &Session) {
        let (Some((w, change, index)), None) = (self.found, &self.found_in) else {
            return;
        };
        self.found_in = if let Some(a) = self.account(w) {
            let line = Text::fill(
                crate::strings::EN.address_check_key_path,
                &[
                    &crate::wallet::fp_text(a.fingerprint),
                    &full_path(&a.account, change, index),
                ],
            );
            let sign = session
                .keys
                .position(a.fingerprint)
                .and_then(|_| leaf(&a.account, change, index))
                .map(|path| {
                    SignAt::Key(crate::wallet::MessageAt {
                        fingerprint: a.fingerprint,
                        path,
                        script: a.script_type,
                        change,
                        index,
                        address: self.address.clone(),
                        line: line.clone(),
                    })
                });
            Some(FoundIn::Account {
                label: a.label.clone(),
                script: a.script_type,
                number: a.number,
                line,
                at: (a.fingerprint, a.account.clone()),
                sign,
            })
        } else {
            session.wallets.get(w).map(|wl| {
                let keys: Vec<Text> = wl
                    .policy()
                    .keys()
                    .iter()
                    .filter_map(|k| {
                        Some(Text::fill(
                            crate::strings::EN.address_check_key_path,
                            &[
                                &crate::wallet::fp_text(k.fingerprint()?),
                                &full_path(k.path()?, change, index),
                            ],
                        ))
                    })
                    .collect();
                let sign = if !session.message_wallets().contains(&w) {
                    None
                } else if !change {
                    Some(SignAt::Wallet(w, index))
                } else {
                    let single = wl.policy().keys().first();
                    match (single, crate::wallet::Kind::of(wl.policy()), keys.first()) {
                        (Some(k), crate::wallet::Kind::Single(script), Some(line)) => k
                            .fingerprint()
                            .zip(k.path())
                            .and_then(|(fingerprint, origin)| {
                                Some(SignAt::Key(crate::wallet::MessageAt {
                                    fingerprint,
                                    path: leaf(origin, true, index)?,
                                    script,
                                    change: true,
                                    index,
                                    address: self.address.clone(),
                                    line: line.clone(),
                                }))
                            }),
                        _ => None,
                    }
                };
                FoundIn::Wallet {
                    place: w,
                    name: wl.name.clone(),
                    shape: Session::shape(wl),
                    sign,
                    keys,
                }
            })
        };
    }

    /// Whether the wallet at `place` is still the one the look began in.
    pub(crate) fn same_wallet(&self, session: &Session, place: usize) -> bool {
        matches!((self.loaded.get(place), session.wallets.get(place)),
            (Some(id), Some(w)) if w.id() == *id)
    }

    /// Costs an address derivation for each index of each chain of each
    /// account, up to `depth`. The place and the chain and index of
    /// `addr` in the first account that holds it, as a find's place
    /// counts on from the wallets'.
    pub(crate) fn accounts_find(
        &self,
        c: &Compute,
        net: faraday_bip::keys::Network,
        addr: &faraday_bip::bitcoin::Address,
        depth: u32,
    ) -> Option<(usize, (bool, u32))> {
        self.accounts.iter().enumerate().find_map(|(i, a)| {
            Some((
                self.wallets + i,
                a.policy.find_address(c, net, addr, depth)?,
            ))
        })
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
                if self.under_way.is_none()
                    && depth > self.looked
                    && self.found.is_none()
                    && !self.ended
                {
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
