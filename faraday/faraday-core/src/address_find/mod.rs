//! What Explore's Find an address (F76) and the address check's **Look
//! further** (`docs/QR.md` §2) share: the depths a search goes to, the
//! pace it keeps so a frame is still a frame, and the derivation of the
//! address at one index of a wallet. A search is not a thread: every tick
//! derives a budget of indices and comes back, as a vanity grind runs
//! (`crate::grind`).

mod accounts;
mod card;
mod look;
pub(crate) mod screen;

pub(crate) use accounts::{Account, unmade};
pub(crate) use card::{Answer, answer};
pub use look::LookWay;
pub(crate) use look::{FoundIn, Look, SignAt};

use crate::strings::EN;
use crate::{Faraday, address_find, wallet};
use faraday_bip::bitcoin::secp256k1::{Secp256k1, VerifyOnly};
use faraday_bip::bitcoin::{Address, Network as BtcNetwork, ScriptBuf};
use faraday_bip::keys::Network;
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;

/// How deep a search goes, in indices of each chain: the first hundred,
/// the depth "Is this address mine?" answers at once for a loaded wallet
/// (`docs/QR.md` §2); a thousand, a wallet used for years; ten thousand,
/// a shop's or a service's: 1.9 s on the build machine for one key, about
/// twenty on a Pi 3 (F76), and a multisig wallet's several keys an index
/// more.
pub const DEPTHS: [u32; 3] = [100, 1_000, 10_000];

/// How long one tick of a search aims to take, ms.
const TICK_MS: u64 = 50;

/// The first tick's indices, small enough to come back at once on the
/// slowest device, since it is the tick the rate is first measured over.
const FIRST_BUDGET: u32 = 4;

/// The most indices one tick takes, whatever the rate measured.
const MAX_BUDGET: u32 = 4_096;

/// How many indices a search's next tick takes, resized from how long the
/// last one took.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pace {
    /// Indices the next tick takes.
    budget: u32,
    /// When the last tick came, ms.
    last_ms: u64,
}

impl Pace {
    /// The pace of a search that starts at `now_ms`.
    pub(crate) fn new(now_ms: u64) -> Self {
        Pace {
            budget: FIRST_BUDGET,
            last_ms: now_ms,
        }
    }

    /// Indices the next tick takes.
    pub(crate) fn budget(&self) -> u32 {
        self.budget
    }

    /// The budget for the next tick, from `tested` indices taken since
    /// the last one.
    pub(crate) fn ticked(&mut self, tested: u32, now_ms: u64) {
        let span = now_ms.saturating_sub(self.last_ms).max(1);
        self.last_ms = now_ms;
        let next = u64::from(tested.max(1)) * TICK_MS / span;
        self.budget = (next.min(u64::from(MAX_BUDGET)) as u32).max(FIRST_BUDGET);
    }
}

/// Costs an address derivation. The script of `policy` at `change`/`index`,
/// or `None` where there is none (an index whose key is invalid, one
/// chance in 2¹²⁷).
pub(crate) fn script(
    c: &Compute,
    secp: &Secp256k1<VerifyOnly>,
    policy: &WalletPolicy,
    change: bool,
    index: u32,
) -> Option<ScriptBuf> {
    policy.script_at_with(c, secp, change, index).ok()
}

/// Costs an address derivation. The address of `policy` at
/// `change`/`index` on `net`, or `None` where there is none.
pub(crate) fn address(
    c: &Compute,
    secp: &Secp256k1<VerifyOnly>,
    policy: &WalletPolicy,
    net: Network,
    change: bool,
    index: u32,
) -> Option<Address> {
    let s = script(c, secp, policy, change, index)?;
    Address::from_script(&s, BtcNetwork::from(net)).ok()
}

impl Faraday {
    /// What `text` is as an address (`docs/QR.md` §2): `None` when it is
    /// not one; the line it is refused with (a payment link that needs
    /// something this device does not read, a checksum that fails, another
    /// network's address); or the address, from a `bitcoin:` link as well.
    pub(crate) fn address_read(
        &self,
        text: &str,
    ) -> Option<Result<faraday_bip::bitcoin::Address, faraday_text::Text>> {
        let refused = |line: &'static str| Some(Err(faraday_text::Text::of(line)));
        let t = text.trim();
        let mut is_uri = false;
        let t = match faraday_codec::bip21::parse(t) {
            Ok(uri) => {
                is_uri = true;
                uri.address
            }
            Err(faraday_codec::bip21::Error::NotUri) => t,
            Err(faraday_codec::bip21::Error::Required) => {
                return refused(EN.address_note_uri_required);
            }
            Err(faraday_codec::bip21::Error::Malformed) => {
                return refused(EN.address_note_uri_malformed);
            }
        };
        let net = self.session.network();
        let Ok(addr) =
            t.parse::<faraday_bip::bitcoin::Address<faraday_bip::bitcoin::address::NetworkUnchecked>>()
        else {
            // An address's characters, as many as one has, that fail its
            // checksum: one of them is wrong.
            let whole = t.len() >= 26
                && t.bytes().all(|b| b.is_ascii_alphanumeric())
                && faraday_bip::keys::Network::ALL
                    .iter()
                    .any(|&n| faraday_bip::address::is_address_prefix(t, n));
            if whole {
                return refused(EN.address_note_that_address_fails);
            }
            // A link whose address is anything else (not empty: BIP 321
            // links carry none) is not well formed.
            return (is_uri && !t.is_empty())
                .then(|| Err(faraday_text::Text::of(EN.address_note_uri_malformed)));
        };
        if addr.is_valid_for_network(net.into()) {
            return Some(Ok(addr.assume_checked()));
        }
        // Testnet and signet write their addresses alike: the first
        // network that takes it names it.
        let of = faraday_bip::keys::Network::ALL
            .iter()
            .copied()
            .find(|&n| addr.is_valid_for_network(n.into()))
            .unwrap_or(net);
        Some(Err(faraday_text::Text::fill(
            EN.address_note_wrong_network,
            &[
                &faraday_text::Text::ascii_lowercase(wallet::network_name(of)),
                &faraday_text::Text::ascii_lowercase(wallet::network_name(net)),
            ],
        )))
    }

    /// Costs an address derivation (through `find_address`), and a key
    /// derivation for each standard account of a loaded key that has no
    /// wallet. What an address is to the loaded wallets and those
    /// accounts: one of a wallet's first hundred on either chain, or none
    /// of them (`docs/QR.md` §2: "Is this address mine?"), and for none of
    /// them the look further that is offered. `None` when no wallet and no
    /// key is loaded.
    pub(crate) fn address_check(
        &self,
        c: &Compute,
        addr: &faraday_bip::bitcoin::Address,
    ) -> Option<address_find::Look> {
        let net = self.session.network();
        let accounts = address_find::unmade(c, &self.session);
        let wallets = &self.session.wallets;
        if wallets.len() + accounts.len() == 0 {
            return None;
        }
        let mut shown = faraday_text::Text::new();
        let _ = std::fmt::Write::write_fmt(&mut shown, format_args!("{addr}"));
        let mut look = address_find::Look::new((shown, addr.script_pubkey()), wallets, accounts);
        let wallet_find = wallets
            .iter()
            .enumerate()
            .find_map(|(w, wl)| Some((w, wl.policy().find_address(c, net, addr, 100)?)));
        let found = wallet_find.or_else(|| look.accounts_find(c, net, addr, 100));
        if let Some((place, (change, index))) = found {
            look.found = Some((place, change, index));
            look.settle(&self.session);
        }
        Some(look)
    }
}
