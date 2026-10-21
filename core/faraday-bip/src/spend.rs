//! The ways a wallet's script can be satisfied.
//!
//! A miniscript says, in one expression, every combination of signatures
//! and timelocks that can spend the coins. A person reading a wallet
//! needs that as a list: one line per way, each naming the keys it wants
//! and the wait it imposes. `miniscript`'s own semantic policy
//! ([`miniscript::policy::Semantic`], what `lift()` returns) is that
//! expression with the script's encoding taken off, and this module
//! turns it into the list.
//!
//! Nothing here is words. A [`SpendPath`] is key indices and locks; the
//! sentence a screen shows is built from
//! `opensigner-core`'s strings.

use alloc::vec::Vec;

use miniscript::MiniscriptKey;
use miniscript::policy::Semantic;

/// A wait a spend path imposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lock {
    /// `older(n)`: n blocks after the coins were received.
    Blocks(u32),
    /// `older(n)` with the time flag: n units of 512 seconds after the
    /// coins were received.
    Intervals(u32),
    /// `after(n)` below 500 000 000: a block height the chain must
    /// reach.
    Height(u32),
    /// `after(n)` at or above 500 000 000: a Unix time the chain's
    /// median time must pass.
    Time(u32),
    /// A hash whose preimage the spender must produce.
    Preimage,
}

/// One way a wallet's script can be satisfied: the keys that must sign
/// and the waits that must have passed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SpendPath {
    /// Indices into [`crate::policy::WalletPolicy::keys`], ascending.
    pub keys: Vec<usize>,
    /// The locks, in the order [`Lock`] orders them.
    pub locks: Vec<Lock>,
}

impl SpendPath {
    /// The two paths taken together: both sets of keys, both waits.
    fn and(&self, other: &SpendPath) -> SpendPath {
        let mut keys = self.keys.clone();
        keys.extend_from_slice(&other.keys);
        keys.sort_unstable();
        keys.dedup();
        let mut locks = self.locks.clone();
        locks.extend_from_slice(&other.locks);
        locks.sort_unstable();
        locks.dedup();
        SpendPath { keys, locks }
    }
}

/// How many paths are listed before the list stops being something a
/// person reads. A wallet whose script has more ways to spend it than
/// this states none of them rather than a truncated few.
const MAX_PATHS: usize = 64;

/// The ways `policy` can be satisfied, with each key named by its
/// position in `keys`. A key the list does not hold makes the whole
/// answer `None`, since a path naming no key states nothing.
///
/// `None` when the script has more than [`MAX_PATHS`] of them.
pub fn spend_paths<Pk: MiniscriptKey>(
    policy: &Semantic<Pk>,
    keys: &[Pk],
) -> Option<Vec<SpendPath>> {
    let mut paths = walk(policy, keys)?;
    paths.sort();
    paths.dedup();
    Some(paths)
}

/// One path, for the leaves that are a single condition.
fn one(path: SpendPath) -> Option<Vec<SpendPath>> {
    Some(alloc::vec![path])
}

fn walk<Pk: MiniscriptKey>(policy: &Semantic<Pk>, keys: &[Pk]) -> Option<Vec<SpendPath>> {
    match policy {
        // Nothing satisfies it, so there is no way to list.
        Semantic::Unsatisfiable => Some(Vec::new()),
        Semantic::Trivial => one(SpendPath::default()),
        Semantic::Key(pk) => {
            let at = keys.iter().position(|k| k == pk)?;
            one(SpendPath {
                keys: alloc::vec![at],
                locks: Vec::new(),
            })
        }
        Semantic::Older(rel) => one(lock_path(if rel.is_time_locked() {
            // The low sixteen bits are the count; the flag above them
            // says the unit.
            Lock::Intervals(rel.to_consensus_u32() & 0xffff)
        } else {
            Lock::Blocks(rel.to_consensus_u32() & 0xffff)
        })),
        Semantic::After(abs) => one(lock_path(if abs.is_block_time() {
            Lock::Time(abs.to_consensus_u32())
        } else {
            Lock::Height(abs.to_consensus_u32())
        })),
        Semantic::Sha256(_)
        | Semantic::Hash256(_)
        | Semantic::Ripemd160(_)
        | Semantic::Hash160(_) => one(lock_path(Lock::Preimage)),
        Semantic::Thresh(thresh) => {
            let children: Vec<Vec<SpendPath>> = thresh
                .iter()
                .map(|child| walk(child, keys))
                .collect::<Option<_>>()?;
            combine(thresh.k(), &children)
        }
    }
}

fn lock_path(lock: Lock) -> SpendPath {
    SpendPath {
        keys: Vec::new(),
        locks: alloc::vec![lock],
    }
}

/// Every way to satisfy `k` of `children`: each choice of `k` of them,
/// with each of those children's own ways combined.
///
/// `k` equal to the number of children is an `and`, `k` of one is an
/// `or`, and both fall out of the same walk.
fn combine(k: usize, children: &[Vec<SpendPath>]) -> Option<Vec<SpendPath>> {
    if k > children.len() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    let mut chosen = Vec::with_capacity(k);
    choose(k, children, 0, &mut chosen, &mut out)?;
    Some(out)
}

fn choose(
    k: usize,
    children: &[Vec<SpendPath>],
    from: usize,
    chosen: &mut Vec<usize>,
    out: &mut Vec<SpendPath>,
) -> Option<()> {
    if chosen.len() == k {
        let mut combos = alloc::vec![SpendPath::default()];
        for child in chosen.iter() {
            let mut next = Vec::new();
            for a in &combos {
                for b in &children[*child] {
                    next.push(a.and(b));
                    if out.len() + next.len() > MAX_PATHS {
                        return None;
                    }
                }
            }
            combos = next;
        }
        out.extend(combos);
        return Some(());
    }
    // Not enough children left to finish a choice of k.
    if children.len() - from < k - chosen.len() {
        return Some(());
    }
    for i in from..children.len() {
        chosen.push(i);
        choose(k, children, i + 1, chosen, out)?;
        chosen.pop();
    }
    Some(())
}
