//! The groups of places a hand-out must rebuild for (`hand_out`): all of
//! them listed up to 16 places; above that, the groups of up to `M`
//! places checked within a step budget, and a plan not checked in full
//! is not called rebuilding.

use std::cell::Cell;

/// The most places whose groups are listed in full.
const LISTED: usize = 16;

/// The steps a check above [`LISTED`] places may take, all hand-out
/// picks together.
const STEPS: u32 = 200_000;

/// The groups of one plan's places, by the keys each place holds as seeds.
pub(super) struct Groups<'a> {
    seeds: &'a [u64],
    m: usize,
    all: u64,
    /// The groups that must rebuild, by bit mask of places: empty above
    /// [`LISTED`] places.
    listed: Vec<u64>,
    steps: Cell<u32>,
}

impl<'a> Groups<'a> {
    /// The groups of the places holding `seeds` for an `m`-of plan whose
    /// keys are the bits of `all`: every group of `m` places, and every
    /// group holding `m` seeds. Costs a pass over the groups, up to 2^16.
    pub(super) fn new(seeds: &'a [u64], m: usize, all: u64) -> Self {
        let places = seeds.len();
        let listed = if places <= LISTED {
            (1u64..(1 << places))
                .filter(|&g| {
                    let keys = (0..places)
                        .filter(|p| g & (1 << p) != 0)
                        .fold(0u64, |k, p| k | seeds[p]);
                    g.count_ones() as usize >= m || keys.count_ones() as usize >= m
                })
                .collect()
        } else {
            Vec::new()
        };
        Groups {
            seeds,
            m,
            all,
            listed,
            steps: Cell::new(STEPS),
        }
    }

    /// Whether every group has every key between its seeds and the
    /// shares `pick` hands its places (an index into `shares`). Above
    /// [`LISTED`] places: false unless every group of at most `m` places
    /// that can rebuild was checked within the steps left. Costs up to a
    /// pass over the groups.
    pub(super) fn rebuild(&self, shares: &[u64], pick: &[usize]) -> bool {
        let places = self.seeds.len();
        if places > LISTED {
            let held: Vec<u64> = (0..places)
                .map(|p| self.seeds[p] | shares[pick[p]])
                .collect();
            return self.walk(&held, 0, 0, 0, 0);
        }
        self.listed.iter().all(|&g| {
            (0..places)
                .filter(|p| g & (1 << p) != 0)
                .fold(0u64, |k, p| k | self.seeds[p] | shares[pick[p]])
                == self.all
        })
    }

    /// Spends a step; false when none are left.
    fn spend(&self) -> bool {
        let left = self.steps.get();
        self.steps.set(left.saturating_sub(1));
        left > 0
    }

    /// The groups that start with the places chosen so far (`size` of
    /// them, holding `seed_keys` as seeds and `keys` with shares) and go
    /// on from place `from`: larger groups hold what a group of `m`
    /// holds, so a group is checked where it first can rebuild.
    fn walk(&self, held: &[u64], from: usize, size: usize, seed_keys: u64, keys: u64) -> bool {
        if size > 0 && (size >= self.m || seed_keys.count_ones() as usize >= self.m) {
            return self.spend() && keys == self.all;
        }
        (from..held.len()).all(|p| {
            self.spend()
                && self.walk(
                    held,
                    p + 1,
                    size + 1,
                    seed_keys | self.seeds[p],
                    keys | held[p],
                )
        })
    }
}
