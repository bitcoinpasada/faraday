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
pub(crate) use card::{Answer, refresh};
pub use look::LookWay;
pub(crate) use look::{FoundIn, Look};

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
