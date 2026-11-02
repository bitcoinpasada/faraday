//! Where the board's sticks are in the flow: a wallet's map read as the
//! flow's [`Target`]s, and the plan's places edited.

use super::{Plan, Target};

impl Plan {
    /// The vaults the `k`th vault stick of place `p` (the `k`th of
    /// `places[p]`) carries, in order: its first vault, then the vaults
    /// joined to it.
    pub fn stick_vaults(&self, p: usize, k: usize) -> Vec<usize> {
        let mut out: Vec<usize> = self
            .places
            .get(p)
            .and_then(|vs| vs.get(k))
            .copied()
            .into_iter()
            .collect();
        out.extend(
            self.joined
                .iter()
                .filter(|&&(q, s, _)| q == p && s == k)
                .map(|&(_, _, v)| v),
        );
        out
    }

    /// The vaults of the `n`th (from 0) vault stick of place `p` whose
    /// first vault is `v`; `v` alone where the plan has no such stick.
    pub fn nth_stick_vaults(&self, p: usize, v: usize, n: usize) -> Vec<usize> {
        let vs = self.places.get(p).map_or(&[][..], Vec::as_slice);
        let k = vs.iter().enumerate().filter(|&(_, &u)| u == v).nth(n);
        k.map_or_else(|| vec![v], |(k, _)| self.stick_vaults(p, k))
    }

    /// Place `p` drops the sticks whose first vault is `v`, or keeps one
    /// more; false where it would keep none. The vaults joined to a stick
    /// that stays go with it.
    pub(crate) fn toggle_place_vault(&mut self, p: usize, v: usize) -> bool {
        let mut now = self.places[p].clone();
        if now.contains(&v) {
            if now.len() < 2 {
                return false;
            }
            now.retain(|&x| x != v);
        } else {
            now.push(v);
        }
        let old = std::mem::take(&mut self.places[p]);
        let mut used = vec![false; now.len()];
        // Each old stick is matched to the first unmatched new stick with
        // its first vault; the ones left over are gone.
        let moved: Vec<Option<usize>> = old
            .iter()
            .map(|&v| {
                let k = (0..now.len()).find(|&k| !used[k] && now[k] == v)?;
                used[k] = true;
                Some(k)
            })
            .collect();
        self.joined = std::mem::take(&mut self.joined)
            .into_iter()
            .filter_map(|(q, s, v)| {
                if q != p {
                    return Some((q, s, v));
                }
                moved.get(s).copied().flatten().map(|k| (q, k, v))
            })
            .collect();
        self.places[p] = now;
        true
    }

    /// The plan with place `p` taken out: the places after it move up.
    pub(crate) fn remove_place(&mut self, p: usize) {
        self.places.remove(p);
        self.together.retain(|&q| q != p);
        self.joined.retain(|j| j.0 != p);
        self.place_files.retain(|&q| q != p);
        let up = |q: &mut usize| *q -= usize::from(*q > p);
        self.together.iter_mut().for_each(up);
        self.place_files.iter_mut().for_each(up);
        self.joined.iter_mut().for_each(|j| up(&mut j.0));
    }
}

/// The flow's stick for stick `s` at place `p` of a wallet's map
/// `sticks` (`docs/BACKUP-FLOW.md` §17.4), as [`plan_of`] names it: the
/// place's only stick with a vault, [`Target::Place`]; else its vault's
/// first stick there, [`Target::PlaceVault`], and a later one
/// [`Target::Another`]; a stick with no vault, the place's first stick of
/// files [`Target::PlaceFiles`] and a later one [`Target::Another`].
pub fn stick_target(sticks: &[crate::plan::Spot], p: usize, s: usize) -> Option<Target> {
    use crate::plan::{stick_vault, sticks_at};
    let n = sticks_at(sticks, p);
    if s >= n {
        return None;
    }
    let vault = stick_vault(sticks, p, s);
    let before = (0..s)
        .filter(|&k| stick_vault(sticks, p, k) == vault)
        .count();
    // A place whose one vault stick carries several vaults is still the
    // place's one stick.
    Some(match vault {
        Some(v) => {
            let vault_sticks = (0..n)
                .filter(|&k| stick_vault(sticks, p, k).is_some())
                .count();
            if vault_sticks <= 1 {
                Target::Place(p)
            } else if before == 0 {
                Target::PlaceVault(p, v)
            } else {
                Target::Another(p, Some(v), before)
            }
        }
        None if before == 0 => Target::PlaceFiles(p),
        None => Target::Another(p, None, before),
    })
}

/// The stick of a wallet's map `sticks` that the flow's stick `t` is
/// ([`stick_target`] read back); `None` for a stick no place keeps.
pub fn target_stick(sticks: &[crate::plan::Spot], t: Target) -> Option<crate::plan::At> {
    let p = match t {
        Target::Place(p) | Target::PlaceVault(p, _) | Target::PlaceFiles(p) => p,
        Target::Another(p, _, _) => p,
        Target::Files | Target::Public => return None,
    };
    (0..crate::plan::sticks_at(sticks, p))
        .find(|&s| stick_target(sticks, p, s) == Some(t))
        .map(|s| crate::plan::At::Stick(p, s))
}
