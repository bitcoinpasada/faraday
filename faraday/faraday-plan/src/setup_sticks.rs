//! The board's first placement for a wallet whose vaults were set up
//! before its keys (`docs/STICK-SETUP.md` §6.1): The standard way's
//! places with the setup's sticks placed on them.

use super::{Answers, At, Shape, What, map, max_places};
use faraday_crypto::work::Compute;

impl Answers {
    /// Costs what `map` costs, when sticks share a vault. Places the sticks
    /// the setup read, `sticks` listing the vaults on each in stick order:
    /// vault `n` holds key `n` (a vault past the last key holds a copy of
    /// the key before it; keys left over stay on paper alone); each stick
    /// goes to the place of its first vault's key, a stick whose first
    /// vault holds a copy, or whose place another stick has, to a place
    /// added after the others; vaults on one stick stay on it.
    pub fn place_setup_sticks(&mut self, c: &Compute, shape: &Shape, sticks: &[Vec<usize>]) {
        let here = shape.here();
        let key_of = |v: usize| here.get(v.min(here.len().saturating_sub(1))).copied();
        let rows = self.vaults.len().max(shape.vault_rows());
        self.vaults = (0..rows)
            .map(|v| {
                let mut row = vec![false; shape.seeds.len()];
                if let Some(i) = key_of(v) {
                    row[i] = true;
                }
                row
            })
            .collect();
        self.sticks = vec![vec![false; self.places]; rows];
        self.seeds[super::seeds::VAULT] = !shape.watch_only();
        self.wallet[super::wallet::VAULT] = true;
        // The place each stick goes to, and the places added.
        let mut taken: Vec<usize> = Vec::new();
        let mut at: Vec<usize> = Vec::new();
        let mut total = self.places;
        for on in sticks {
            let own = on
                .first()
                .filter(|&&f| f < here.len())
                .map(|&f| if shape.multi() { here[f] } else { 0 })
                .filter(|&p| p < self.places && !taken.contains(&p));
            let p = own.unwrap_or_else(|| {
                total += 1;
                total - 1
            });
            taken.push(p);
            at.push(p);
        }
        if total > self.places {
            let first = self.places;
            self.set_places(shape, total);
            // An added place keeps no paper copy.
            self.bare.extend(first..total);
        }
        for (on, &p) in sticks.iter().zip(&at) {
            for &v in on {
                if let Some(row) = self.sticks.get_mut(v).and_then(|r| r.get_mut(p)) {
                    *row = true;
                }
            }
        }
        // Vaults on one stick are one stick on the map.
        if sticks.iter().any(|on| on.len() > 1) {
            let mut m = map(c, shape, self);
            for (on, &p) in sticks.iter().zip(&at) {
                let Some((&first, rest)) = on.split_first() else {
                    continue;
                };
                let find = |m: &[super::Spot], v: usize| {
                    m.iter().position(|s| {
                        matches!(s.at, At::Stick(q, _) if q == p)
                            && s.holds.iter().any(|(w, _)| *w == What::VaultStick(v))
                    })
                };
                for &v in rest {
                    let (Some(from), Some(to)) = (find(&m, v), find(&m, first)) else {
                        continue;
                    };
                    if from == to {
                        continue;
                    }
                    if let Some(n) = m[from]
                        .holds
                        .iter()
                        .position(|(w, _)| *w == What::VaultStick(v))
                    {
                        let held = m[from].holds.remove(n);
                        m[to].holds.push(held);
                    }
                }
            }
            self.set_map(m);
        }
        // The passphrase's place keeps no copy of its seed, the vault's
        // stick included.
        for row in self.pass.iter_mut().filter(|r| !r.is_empty()) {
            row.iter_mut().for_each(|b| *b = false);
        }
        let most = if self.places > max_places(shape) {
            usize::MAX
        } else {
            max_places(shape)
        };
        self.place_passphrases_up_to(shape, most);
    }
}
